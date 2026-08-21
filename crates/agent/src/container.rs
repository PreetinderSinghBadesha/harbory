use std::collections::HashMap;

use bollard::container::{
    Config, CreateContainerOptions, ListContainersOptions, RemoveContainerOptions, StartContainerOptions,
    StopContainerOptions,
};
use bollard::models::{HostConfig, PortBinding};
use bollard::Docker;
use harbory_protocol::v1::{ContainerSpec, ContainerState, ContainerStatus};

/// Only containers carrying this label are ever listed, reported, or
/// touched by `remove`/`deploy`'s cleanup step — critical on a host that
/// also runs containers Harbory doesn't own. Never do an unfiltered
/// `list_containers`/`remove_container` sweep in this module.
const MANAGED_LABEL: &str = "harbory.managed";
const NAME_LABEL: &str = "harbory.name";

pub struct ContainerManager {
    docker: Docker,
}

fn docker_name(logical_name: &str) -> String {
    format!("harbory-{logical_name}")
}

impl ContainerManager {
    pub fn connect() -> Result<Self, bollard::errors::Error> {
        Ok(Self { docker: Docker::connect_with_local_defaults()? })
    }

    /// Create-or-replace, idempotent: removes any existing container by
    /// this name first (whatever its current image/state), then creates
    /// fresh. Simpler and more robust than diffing/patching in place, and
    /// it's what makes `reconcile::Action::Deploy` correct for "wrong
    /// image running" and "crashed" cases, not just "doesn't exist yet".
    pub async fn deploy(&self, spec: &ContainerSpec) -> Result<(), bollard::errors::Error> {
        let name = docker_name(&spec.name);
        self.force_remove(&name).await;

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
    /// of reconciliation, not filtered by what's currently desired.
    pub async fn list_state(&self) -> Result<Vec<ContainerState>, bollard::errors::Error> {
        let mut filters = HashMap::new();
        filters.insert("label".to_string(), vec![format!("{MANAGED_LABEL}=true")]);

        let containers = self
            .docker
            .list_containers(Some(ListContainersOptions { all: true, filters, ..Default::default() }))
            .await?;

        Ok(containers
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
            .collect())
    }
}
