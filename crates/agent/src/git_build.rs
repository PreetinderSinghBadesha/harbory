//! Builds a container image from a git repo by shelling out to the host's
//! `git` and `docker` CLIs: clone into a temp dir, then `docker build` it.
//! Both binaries are therefore hard requirements for git-sourced deploys —
//! missing ones surface as a named, actionable error (see spawn_error)
//! rather than a bare ENOENT.

use std::path::PathBuf;
use std::time::Duration;

use bollard::Docker;
use harbory_protocol::v1::GitSource;
use tokio::process::Command;

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("docker build request failed: {0}")]
    Docker(#[from] bollard::errors::Error),
    #[error("build failed: {0}")]
    Build(String),
}

/// Both cloning and building are network/CPU-bound and can hang (a stalled
/// connection, a Dockerfile step waiting on something that never answers).
/// Without a bound, a hung build wedges more than just this one deploy —
/// it blocks the agent's single per-connection message loop, so
/// heartbeats and every other command on that stream stall too until the
/// process is killed. These are generous on purpose (clone: most repos in
/// seconds, allow minutes for a slow host; build: real Dockerfiles can
/// legitimately take several minutes) — the point is a bound exists at
/// all, not that it's tight.
const CLONE_TIMEOUT: Duration = Duration::from_secs(180);
const BUILD_TIMEOUT: Duration = Duration::from_secs(900);

/// Turns a spawn failure into something an operator can act on. The
/// common case is the binary simply not being installed on this host
/// (`ErrorKind::NotFound` = ENOENT) — a raw "os error 2" gives no hint
/// which program or what to do about it.
fn spawn_error(binary: &str, err: std::io::Error) -> BuildError {
    if err.kind() == std::io::ErrorKind::NotFound {
        BuildError::Build(format!(
            "'{binary}' executable not found on this host — git-sourced builds shell out to it. \
             Install it (e.g. 'sudo apt-get install -y {binary}') and restart harbory-agent."
        ))
    } else {
        BuildError::Build(format!("failed to execute '{binary}': {err}"))
    }
}

/// Runs `cmd`, bounded by `timeout`, and maps every failure mode (spawn
/// failure, timeout, nonzero exit is left to the caller) into a
/// `BuildError` — the one place that knows how to name the binary in the
/// error either way.
async fn run(mut cmd: Command, binary: &str, timeout: Duration) -> Result<std::process::Output, BuildError> {
    match tokio::time::timeout(timeout, cmd.output()).await {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(err)) => Err(spawn_error(binary, err)),
        Err(_elapsed) => Err(BuildError::Build(format!("'{binary}' timed out after {}s", timeout.as_secs()))),
    }
}

fn sanitize_docker_identifier(s: &str) -> String {
    let sanitized: String = s
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '-' })
        .collect();
    let trimmed = sanitized.trim_matches('-');
    if trimmed.is_empty() {
        "container".to_string()
    } else {
        trimmed.to_string()
    }
}

/// One fixed tag per logical container name — no need for per-build
/// uniqueness since `ContainerManager::try_deploy` already force-removes
/// any existing container by this name before recreating, and v1
/// deliberately does a full rebuild on every deploy rather than caching
/// across them.
fn tag_for(logical_name: &str) -> String {
    let slug = sanitize_docker_identifier(logical_name).to_ascii_lowercase();
    format!("harbory-build-{slug}:latest")
}

pub async fn clone_repo(repo_url: &str, git_ref: &str, work_dir: &PathBuf) -> Result<(), BuildError> {
    let work_dir_str = work_dir.to_str().ok_or_else(|| BuildError::Build("work dir is not valid UTF-8".into()))?;

    // Shallow first — this is how the established PaaS projects clone
    // (Coolify: `git clone --depth=1 -b <branch> <token-url> <dir>`), and
    // how Docker's own builder clones for remote contexts. A full-history
    // clone of a large repo is the difference between seconds and minutes
    // per deploy. Commit-SHA refs can't be fetched shallow (GitHub
    // doesn't allow `--depth 1 --branch <sha>` for an arbitrary sha), so
    // on failure fall back to a full clone + explicit checkout.
    let mut shallow = Command::new("git");
    shallow
        .args(["clone", "--depth", "1", "--recurse-submodules", "--shallow-submodules"])
        .env("GIT_TERMINAL_PROMPT", "0");
    if !git_ref.is_empty() {
        shallow.args(["--branch", git_ref]);
    }
    shallow.args([repo_url, work_dir_str]);
    let shallow_output = run(shallow, "git", CLONE_TIMEOUT).await?;

    if shallow_output.status.success() {
        return Ok(());
    }

    // A failed clone can leave a partial directory behind; git refuses to
    // clone into a non-empty dir, so clear it before retrying.
    let _ = tokio::fs::remove_dir_all(work_dir).await;

    let mut clone_cmd = Command::new("git");
    clone_cmd.arg("clone").arg("--recurse-submodules").arg(repo_url).arg(work_dir).env("GIT_TERMINAL_PROMPT", "0");
    let clone_output = run(clone_cmd, "git", CLONE_TIMEOUT).await?;
    if !clone_output.status.success() {
        let stderr = String::from_utf8_lossy(&clone_output.stderr);
        return Err(BuildError::Build(format!("git clone failed:\n{stderr}")));
    }

    if !git_ref.is_empty() {
        let mut checkout_cmd = Command::new("git");
        checkout_cmd.current_dir(work_dir).args(["checkout", git_ref]);
        let checkout_output = run(checkout_cmd, "git", CLONE_TIMEOUT).await?;
        if !checkout_output.status.success() {
            let stderr = String::from_utf8_lossy(&checkout_output.stderr);
            return Err(BuildError::Build(format!("git checkout failed:\n{stderr}")));
        }
    }

    Ok(())
}

/// Builds `source` and returns the local image tag it was built as. The
/// credential (if any) for a private repo is expected to already be
/// embedded in `source.repo_url` by the caller (the control plane does
/// this only in the wire message it sends, never in what it persists) —
/// this function doesn't know or care whether the repo is public or
/// private.
pub async fn build(_docker: &Docker, logical_name: &str, source: &GitSource) -> Result<String, BuildError> {
    let tag = tag_for(logical_name);
    let dockerfile = if source.dockerfile_path.is_empty() { "Dockerfile" } else { source.dockerfile_path.as_str() };

    let temp_dir = tempfile::tempdir_in(".").map_err(|e| BuildError::Build(format!("failed to create temp dir: {e}")))?;
    let repo_dir = temp_dir.path().join("repo");

    clone_repo(&source.repo_url, &source.git_ref, &repo_dir).await?;

    // `.git` (history + submodule checkouts) has no purpose inside the
    // build context and would otherwise get uploaded to the Docker
    // daemon in full on every deploy — best-effort, not fatal if it
    // fails, since it's a size optimization, not a correctness concern.
    let _ = tokio::fs::remove_dir_all(repo_dir.join(".git")).await;

    // No forced DOCKER_BUILDKIT=1: if it's set but the `buildx` CLI
    // plugin isn't installed (common on distro-packaged Docker, e.g.
    // Ubuntu's `docker.io` apt package rather than Docker's own
    // `docker-ce` repo), `docker build` hard-fails with "BuildKit is
    // enabled but the buildx component is missing" instead of falling
    // back — every single git-sourced deploy would fail on such a host.
    // Nothing here needs a BuildKit-only feature (no --mount=type=secret,
    // no multi-platform), so there's no reason to force it and every
    // reason not to.
    let mut build_cmd = Command::new("docker");
    build_cmd.current_dir(&repo_dir).args(["build", "-t", &tag, "-f", dockerfile, "."]);
    let output = run(build_cmd, "docker", BUILD_TIMEOUT).await?;

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(BuildError::Build(format!(
            "docker build failed (exit {}):\n\nSTDOUT:\n{stdout}\n\nSTDERR:\n{stderr}",
            output.status.code().unwrap_or(-1),
        )));
    }

    Ok(tag)
}
