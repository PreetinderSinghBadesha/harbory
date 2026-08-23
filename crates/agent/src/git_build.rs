//! Builds a container image straight from a git repo, using the Docker
//! daemon's own git-remote build-context support (`BuildImageOptions.remote`
//! as a git URL) rather than the agent cloning and tarring a context
//! itself — no `git` binary, no temp directory, no extra dependency. The
//! daemon (via BuildKit on any reasonably current Docker install) handles
//! the clone internally, exactly like `docker build <git-url>` from the
//! CLI does.

use bollard::image::BuildImageOptions;
use bollard::Docker;
use harbory_protocol::v1::GitSource;
use tokio_stream::StreamExt;

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("docker build request failed: {0}")]
    Docker(#[from] bollard::errors::Error),
    #[error("build failed: {0}")]
    Build(String),
}

/// One fixed tag per logical container name — no need for per-build
/// uniqueness since `ContainerManager::try_deploy` already force-removes
/// any existing container by this name before recreating, and v1
/// deliberately does a full rebuild on every deploy rather than caching
/// across them.
fn tag_for(logical_name: &str) -> String {
    format!("harbory-build-{logical_name}:latest")
}

/// Builds `source` and returns the local image tag it was built as. The
/// credential (if any) for a private repo is expected to already be
/// embedded in `source.repo_url` by the caller (the control plane does
/// this only in the wire message it sends, never in what it persists) —
/// this function doesn't know or care whether the repo is public or
/// private.
pub async fn build(docker: &Docker, logical_name: &str, source: &GitSource) -> Result<String, BuildError> {
    let tag = tag_for(logical_name);

    let mut remote = source.repo_url.clone();
    if !source.git_ref.is_empty() {
        remote.push('#');
        remote.push_str(&source.git_ref);
    }
    let dockerfile = if source.dockerfile_path.is_empty() { "Dockerfile" } else { source.dockerfile_path.as_str() };

    let options = BuildImageOptions::<String> {
        dockerfile: dockerfile.to_string(),
        t: tag.clone(),
        remote,
        rm: true,
        forcerm: true,
        ..Default::default()
    };

    let mut stream = docker.build_image(options, None, None);
    while let Some(chunk) = stream.next().await {
        let info = chunk?;
        // Build failures (bad Dockerfile, git clone failure, missing
        // branch, ...) surface as an `error` field on an otherwise
        // successfully-received stream item, not as a transport `Err` —
        // the stream itself is expected to end shortly after this.
        if let Some(error) = info.error {
            return Err(BuildError::Build(error));
        }
    }

    Ok(tag)
}
