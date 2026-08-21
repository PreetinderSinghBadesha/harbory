use std::collections::HashMap;

use bollard::container::{
    Config, CreateContainerOptions, ListContainersOptions, RemoveContainerOptions, StartContainerOptions,
    StopContainerOptions,
};
use bollard::image::CreateImageOptions;
use bollard::models::{HostConfig, PortBinding};
use bollard::Docker;
use harbory_protocol::v1::{ContainerSpec, ContainerState, ContainerStatus};
use tokio::sync::Mutex;
use tokio_stream::StreamExt;

/// Only containers carrying this label are ever listed, reported, or
/// touched by `remove`/`deploy`'s cleanup step — critical on a host that
/// also runs containers Harbory doesn't own. Never do an unfiltered
/// `list_containers`/`remove_container` sweep in this module.
const MANAGED_LABEL: &str = "harbory.managed";
const NAME_LABEL: &str = "harbory.name";

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

fn docker_name(logical_name: &str) -> String {
    format!("harbory-{logical_name}")
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
    pub async fn deploy(&self, spec: &ContainerSpec) -> Result<(), bollard::errors::Error> {
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

    async fn try_deploy(&self, spec: &ContainerSpec) -> Result<(), bollard::errors::Error> {
        let name = docker_name(&spec.name);
        self.force_remove(&name).await;

        // Unlike `docker run`, bollard's create_container doesn't pull a
        // missing image on its own — found the hard way against a real
        // daemon (create_container 404'd with "No such image"). Pull
        // errors are deliberately swallowed rather than propagated: the
        // image might already exist locally under this exact tag (offline
        // dev, a custom-built image never pushed anywhere), in which case
        // create_container below still succeeds; if it doesn't exist
        // either way, create_container's own error is the one that matters.
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

        let mut labels = HashMap::new();
        labels.insert(MANAGED_LABEL.to_string(), "true".to_string());
        labels.insert(NAME_LABEL.to_string(), spec.name.clone());

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
            image: Some(spec.image.clone()),
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
                let image = c.image.unwrap_or_default();
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
}
