use std::collections::HashMap;

use bollard::container::{
    Config, CreateContainerOptions, ListContainersOptions, LogOutput, LogsOptions, RemoveContainerOptions,
    StartContainerOptions, StopContainerOptions,
};
use bollard::image::CreateImageOptions;
use bollard::models::{HostConfig, PortBinding};
use bollard::Docker;
use harbory_protocol::v1::{ContainerSpec, ContainerState, ContainerStatus};
use tokio::sync::Mutex;
use tokio_stream::StreamExt;

use crate::git_build;

/// Only containers carrying this label are ever listed, reported, or
/// touched by `remove`/`deploy`'s cleanup step — critical on a host that
/// also runs containers Harbory doesn't own. Never do an unfiltered
/// `list_containers`/`remove_container` sweep in this module.
const MANAGED_LABEL: &str = "harbory.managed";
const NAME_LABEL: &str = "harbory.name";
/// Echoes back the exact `spec.image` string a container was deployed
/// with (real pull ref, or the synthetic "git+..." identity string for a
/// git-sourced deploy — see ContainerSpec.image in harbory.proto).
/// `list_state` reads this instead of Docker's own reported image so
/// reconciliation compares against what was actually *desired*, not
/// whatever Docker/the daemon happens to report for the real underlying
/// image — which for a git-sourced container is a local build tag that
/// would otherwise never match the control plane's synthetic identity
/// string, permanently failing to converge.
const IMAGE_LABEL: &str = "harbory.image";

#[derive(Debug, thiserror::Error)]
pub enum DeployError {
    #[error(transparent)]
    Docker(#[from] bollard::errors::Error),
    #[error(transparent)]
    Build(#[from] git_build::BuildError),
}

pub struct ContainerManager {
    docker: Docker,
    /// Deploy failures for containers that therefore don't exist in Docker
    /// at all (e.g. bad image reference) — `list_state` has nothing to
    /// report for those from `list_containers` alone, so this is what
    /// surfaces them as `ContainerStatus::Error` instead of them silently
    /// looking "absent". Cleared on the next successful deploy or on
    /// remove.
    errors: Mutex<HashMap<String, String>>,
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

fn docker_name(logical_name: &str) -> String {
    let slug = sanitize_docker_identifier(logical_name);
    format!("harbory-{slug}")
}

impl ContainerManager {
    pub fn connect() -> Result<Self, bollard::errors::Error> {
        Ok(Self { docker: Docker::connect_with_local_defaults()?, errors: Mutex::new(HashMap::new()) })
    }

    /// Create-or-replace, idempotent: removes any existing container by
    /// this name first (whatever its current image/state), then creates
    /// fresh. Simpler and more robust than diffing/patching in place, and
    /// it's what makes `reconcile::Action::Deploy` correct for "wrong
    /// image running" and "crashed" cases, not just "doesn't exist yet".
    pub async fn deploy(&self, spec: &ContainerSpec) -> Result<(), DeployError> {
        match self.try_deploy(spec).await {
            Ok(()) => {
                self.errors.lock().await.remove(&spec.name);
                Ok(())
            }
            Err(err) => {
                self.errors.lock().await.insert(spec.name.clone(), err.to_string());
                Err(err)
            }
        }
    }

    async fn try_deploy(&self, spec: &ContainerSpec) -> Result<(), DeployError> {
        let name = docker_name(&spec.name);
        self.force_remove(&name).await;

        // A git-sourced deploy builds a real local tag first and runs
        // that — `spec.image` for that case is only the synthetic
        // reconciliation-comparison identity (see IMAGE_LABEL above), not
        // something that exists in any registry to pull.
        let image_to_run = if let Some(source) = &spec.git_source {
            git_build::build(&self.docker, &spec.name, source).await?
        } else {
            // Unlike `docker run`, bollard's create_container doesn't pull
            // a missing image on its own — found the hard way against a
            // real daemon (create_container 404'd with "No such image").
            // Pull errors are deliberately swallowed rather than
            // propagated: the image might already exist locally under
            // this exact tag (offline dev, a custom-built image never
            // pushed anywhere), in which case create_container below
            // still succeeds; if it doesn't exist either way,
            // create_container's own error is the one that matters.
            let mut pull = self.docker.create_image(
                Some(CreateImageOptions { from_image: spec.image.as_str(), ..Default::default() }),
                None,
                None,
            );
            while let Some(result) = pull.next().await {
                if let Err(err) = result {
                    tracing::debug!(image = %spec.image, %err, "image pull failed, will still try create_container");
                    break;
                }
            }
            spec.image.clone()
        };

        let mut labels = HashMap::new();
        labels.insert(MANAGED_LABEL.to_string(), "true".to_string());
        labels.insert(NAME_LABEL.to_string(), spec.name.clone());
        labels.insert(IMAGE_LABEL.to_string(), spec.image.clone());

        let mut exposed_ports: HashMap<String, HashMap<(), ()>> = HashMap::new();
        let mut port_bindings: HashMap<String, Option<Vec<PortBinding>>> = HashMap::new();
        for p in &spec.ports {
            let key = format!("{}/tcp", p.container_port);
            exposed_ports.insert(key.clone(), HashMap::new());
            port_bindings.insert(
                key,
                Some(vec![PortBinding { host_ip: None, host_port: Some(p.host_port.to_string()) }]),
            );
        }

        let config = Config {
            image: Some(image_to_run),
            env: Some(spec.env.clone()),
            cmd: if spec.command.is_empty() { None } else { Some(spec.command.clone()) },
            labels: Some(labels),
            exposed_ports: Some(exposed_ports),
            host_config: Some(HostConfig { port_bindings: Some(port_bindings), ..Default::default() }),
            ..Default::default()
        };

        self.docker.create_container(Some(CreateContainerOptions { name: name.clone(), platform: None }), config).await?;
        self.docker.start_container(&name, None::<StartContainerOptions<String>>).await?;
        Ok(())
    }

    pub async fn remove(&self, logical_name: &str) {
        self.force_remove(&docker_name(logical_name)).await;
        self.errors.lock().await.remove(logical_name);
    }

    /// The reconciler never emits this today (v1 desired state is only
    /// "running" or "absent" — see reconcile.rs), but the wire protocol
    /// supports it for a future manual/dashboard-triggered stop, so the
    /// agent handles it rather than silently ignoring the variant.
    pub async fn stop(&self, logical_name: &str) -> Result<(), bollard::errors::Error> {
        self.docker.stop_container(&docker_name(logical_name), None::<StopContainerOptions>).await
    }

    /// Best-effort: "doesn't exist" is a success, not an error, both here
    /// and as the redeploy cleanup step in `deploy`.
    async fn force_remove(&self, docker_container_name: &str) {
        let _ = self
            .docker
            .remove_container(docker_container_name, Some(RemoveContainerOptions { force: true, ..Default::default() }))
            .await;
    }

    /// Every Harbory-managed container on this host, regardless of the
    /// `harbory.name` desired state on file — this is the observed side
    /// of reconciliation, not filtered by what's currently desired. Also
    /// synthesizes entries for names that failed to deploy at all (see
    /// `errors`), so a bad image reference shows up as an error rather
    /// than looking like nothing was ever attempted.
    pub async fn list_state(&self) -> Result<Vec<ContainerState>, bollard::errors::Error> {
        let mut filters = HashMap::new();
        filters.insert("label".to_string(), vec![format!("{MANAGED_LABEL}=true")]);

        let containers = self
            .docker
            .list_containers(Some(ListContainersOptions { all: true, filters, ..Default::default() }))
            .await?;

        let mut states: Vec<ContainerState> = containers
            .into_iter()
            .map(|c| {
                let name = c.labels.as_ref().and_then(|l| l.get(NAME_LABEL)).cloned().unwrap_or_default();
                // The label, not `c.image` — see IMAGE_LABEL's doc comment
                // for why (git-sourced containers need this to be the
                // synthetic identity string, not Docker's real local tag,
                // for reconciliation to ever converge).
                let image = c.labels.as_ref().and_then(|l| l.get(IMAGE_LABEL)).cloned().unwrap_or_default();
                let status = match c.state.as_deref() {
                    Some("running") => ContainerStatus::Running,
                    Some("exited") | Some("dead") | Some("created") => ContainerStatus::Stopped,
                    _ => ContainerStatus::Error,
                };
                ContainerState { name, image, status: status as i32, error: String::new() }
            })
            .collect();

        let errors = self.errors.lock().await;
        for (name, error) in errors.iter() {
            if !states.iter().any(|s| &s.name == name) {
                states.push(ContainerState {
                    name: name.clone(),
                    image: String::new(),
                    status: ContainerStatus::Error as i32,
                    error: error.clone(),
                });
            }
        }

        Ok(states)
    }
    /// Fetches the last `tail` lines of stdout+stderr from the named container.
    /// `tail == 0` uses a sensible default (100 lines).
    /// Returns `Err` only on Docker API failures; a missing container is
    /// represented as an Ok with an error string rather than a hard error so
    /// the agent can surface it cleanly to the control plane.
    pub async fn logs(&self, logical_name: &str, tail: u32) -> Result<String, bollard::errors::Error> {
        let name = docker_name(logical_name);
        let tail_str = if tail == 0 { "100".to_string() } else { tail.to_string() };
        let options = LogsOptions::<String> {
            stdout: true,
            stderr: true,
            follow: false,
            tail: tail_str,
            ..Default::default()
        };
        let mut stream = self.docker.logs(&name, Some(options));
        let mut out = String::new();
        let mut had_chunks = false;
        while let Some(chunk_res) = stream.next().await {
            match chunk_res {
                Ok(chunk) => {
                    had_chunks = true;
                    match chunk {
                        LogOutput::StdOut { message } | LogOutput::StdErr { message } => {
                            out.push_str(&String::from_utf8_lossy(&message));
                        }
                        LogOutput::Console { message } => {
                            out.push_str(&String::from_utf8_lossy(&message));
                        }
                        LogOutput::StdIn { .. } => {}
                    }
                }
                Err(err) => {
                    if !had_chunks {
                        if let Some(deploy_err) = self.errors.lock().await.get(logical_name) {
                            return Ok(format!("Build / Deploy Error:\n{deploy_err}"));
                        }
                    }
                    return Err(err);
                }
            }
        }
        if out.is_empty() {
            if let Some(deploy_err) = self.errors.lock().await.get(logical_name) {
                return Ok(format!("Build / Deploy Error:\n{deploy_err}"));
            }
        }
        Ok(out)
    }
}
