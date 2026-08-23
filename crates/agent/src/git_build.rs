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
        version: bollard::image::BuilderVersion::BuilderBuildKit,
        ..Default::default()
    };

    let mut stream = docker.build_image(options, None, None);
    let mut build_logs = String::new();

    while let Some(chunk_res) = stream.next().await {
        match chunk_res {
            Ok(info) => {
                if let Some(s) = info.stream {
                    build_logs.push_str(&s);
                }
                if let Some(s) = info.status {
                    build_logs.push_str(&s);
                    build_logs.push('\n');
                }
                if let Some(error) = info.error {
                    if !build_logs.is_empty() {
                        return Err(BuildError::Build(format!("{build_logs}\nError: {error}")));
                    }
                    return Err(BuildError::Build(error));
                }
            }
            Err(err) => {
                let err_msg = match &err {
                    bollard::errors::Error::DockerStreamError { error } => error.clone(),
                    other => other.to_string(),
                };
                if !build_logs.is_empty() {
                    return Err(BuildError::Build(format!("{build_logs}\nDocker stream error: {err_msg}")));
                }
                return Err(BuildError::Build(format!("Docker build failed: {err_msg}")));
            }
        }
    }

    Ok(tag)
}
