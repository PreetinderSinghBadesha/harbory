//! Builds a container image from a git repo by shelling out to the host's
//! `git` and `docker` CLIs: clone into a temp dir, then `docker build` it.
//! Both binaries are therefore hard requirements for git-sourced deploys —
//! missing ones surface as a named, actionable error (see spawn_error)
//! rather than a bare ENOENT.

use bollard::Docker;
use harbory_protocol::v1::GitSource;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("docker build request failed: {0}")]
    Docker(#[from] bollard::errors::Error),
    #[error("build failed: {0}")]
    Build(String),
}

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

pub async fn clone_repo(
    repo_url: &str,
    git_ref: &str,
    work_dir: &PathBuf,
) -> Result<(), BuildError> {
    // 1. Git clone
    let mut clone_cmd = tokio::process::Command::new("git");
    clone_cmd.arg("clone").arg("--recurse-submodules").arg(repo_url).arg(work_dir);

    // Prevent git from asking for interactive credentials
    clone_cmd.env("GIT_TERMINAL_PROMPT", "0");

    let clone_output =
        clone_cmd.output().await.map_err(|e| spawn_error("git", e))?;
    if !clone_output.status.success() {
        let stderr = String::from_utf8_lossy(&clone_output.stderr);
        return Err(BuildError::Build(format!("Git clone failed:\n{}", stderr)));
    }

    // 2. Git checkout ref (if specified)
    if !git_ref.is_empty() {
        let checkout_output = tokio::process::Command::new("git")
            .current_dir(work_dir)
            .args(["checkout", git_ref])
            .output()
            .await
            .map_err(|e| spawn_error("git", e))?;
            
        if !checkout_output.status.success() {
            let stderr = String::from_utf8_lossy(&checkout_output.stderr);
            return Err(BuildError::Build(format!("Git checkout failed:\n{}", stderr)));
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
    
    let temp_dir = tempfile::tempdir_in(".").map_err(|e| BuildError::Build(format!("Failed to create temp dir: {}", e)))?;
    let repo_dir = temp_dir.path().join("repo");

    clone_repo(&source.repo_url, &source.git_ref, &repo_dir).await?;

    // 3. Docker build
    let output = tokio::process::Command::new("docker")
        .current_dir(&repo_dir)
        .args(["build", "-t", &tag, "-f", dockerfile, "."])
        .env("DOCKER_BUILDKIT", "1")
        .output()
        .await
        .map_err(|e| spawn_error("docker", e))?;

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(BuildError::Build(format!(
            "Docker build failed with exit code: {}\n\nSTDOUT:\n{}\n\nSTDERR:\n{}",
            output.status.code().unwrap_or(-1),
            stdout,
            stderr
        )));
    }

    Ok(tag)
}
