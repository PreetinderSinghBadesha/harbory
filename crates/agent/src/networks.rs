use bollard::Docker;
use harbory_protocol::v1::NetworkInfo;

/// Read-only Docker network inspection plus explicit removal, served to the
/// control plane on demand — same shape as `ImagesManager`. Docker's own
/// built-in networks (bridge/host/none) are marked non-removable so the UI
/// can disable the action instead of surfacing Docker's refusal as an error.
pub struct NetworksManager {
    docker: Docker,
}

const BUILTIN_NETWORKS: [&str; 3] = ["bridge", "host", "none"];

impl NetworksManager {
    pub fn connect() -> Result<Self, bollard::errors::Error> {
        Ok(Self { docker: Docker::connect_with_local_defaults()? })
    }

    pub async fn list(&self) -> Result<Vec<NetworkInfo>, bollard::errors::Error> {
        let networks = self.docker.list_networks::<String>(None).await?;

        Ok(networks
            .into_iter()
            .map(|n| {
                let name = n.name.unwrap_or_default();
                NetworkInfo {
                    id: n.id.unwrap_or_default(),
                    removable: !BUILTIN_NETWORKS.contains(&name.as_str()),
                    name,
                    driver: n.driver.unwrap_or_default(),
                    scope: n.scope.unwrap_or_default(),
                }
            })
            .collect())
    }

    pub async fn remove(&self, id: &str) -> Result<(), bollard::errors::Error> {
        self.docker.remove_network(id).await
    }
}
