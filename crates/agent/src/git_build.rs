//! Builds a container image straight from a git repo, using the Docker
//! daemon's own git-remote build-context support (`BuildImageOptions.remote`
//! as a git URL) rather than the agent cloning and tarring a context
//! itself — no `git` binary, no temp directory, no extra dependency. The
//! daemon (via BuildKit on any reasonably current Docker install) handles
//! the clone internally, exactly like `docker build <git-url>` from the
//! CLI does.

use bollard::Docker;
use harbory_protocol::v1::GitSource;

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("docker build request failed: {0}")]
    Docker(#[from] bollard::errors::Error),
    #[error("build failed: {0}")]
    Build(String),
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

/// Builds `source` and returns the local image tag it was built as. The
/// credential (if any) for a private repo is expected to already be
/// embedded in `source.repo_url` by the caller (the control plane does
/// this only in the wire message it sends, never in what it persists) —
/// this function doesn't know or care whether the repo is public or
/// private.
pub async fn build(_docker: &Docker, logical_name: &str, source: &GitSource) -> Result<String, BuildError> {
    let tag = tag_for(logical_name);
    let dockerfile = if source.dockerfile_path.is_empty() { "Dockerfile" } else { source.dockerfile_path.as_str() };
    
    let remote = if source.git_ref.is_empty() {
        source.repo_url.clone()
    } else {
        source.repo_url.clone() // Clone the repo URL, checkout the ref later
    };

    let temp_dir = tempfile::tempdir_in(".").map_err(|e| BuildError::Build(format!("Failed to create temp dir: {}", e)))?;
    let repo_dir = temp_dir.path().join("repo");

    // 1. Git clone
    let mut clone_cmd = tokio::process::Command::new("git");
    clone_cmd.arg("clone").arg("--recurse-submodules").arg(&remote).arg(&repo_dir);
    
    // Prevent git from asking for interactive credentials
    clone_cmd.env("GIT_TERMINAL_PROMPT", "0");
    
    let clone_output = clone_cmd.output().await.map_err(|e| BuildError::Build(format!("Failed to execute git clone: {}", e)))?;
    if !clone_output.status.success() {
        let stderr = String::from_utf8_lossy(&clone_output.stderr);
        return Err(BuildError::Build(format!("Git clone failed:\n{}", stderr)));
    }

    // 2. Git checkout ref (if specified)
    if !source.git_ref.is_empty() {
        let checkout_output = tokio::process::Command::new("git")
            .current_dir(&repo_dir)
            .args(["checkout", &source.git_ref])
            .output()
            .await
            .map_err(|e| BuildError::Build(format!("Failed to execute git checkout: {}", e)))?;
            
        if !checkout_output.status.success() {
            let stderr = String::from_utf8_lossy(&checkout_output.stderr);
            return Err(BuildError::Build(format!("Git checkout failed:\n{}", stderr)));
        }
    }

    // 3. Docker build
    let output = tokio::process::Command::new("docker")
        .current_dir(&repo_dir)
        .args(["build", "-t", &tag, "-f", dockerfile, "."])
        .env("DOCKER_BUILDKIT", "1")
        .output()
        .await
        .map_err(|e| BuildError::Build(format!("Failed to execute docker build: {}", e)))?;

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
