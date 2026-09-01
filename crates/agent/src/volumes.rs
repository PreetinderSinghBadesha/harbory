use bollard::Docker;
use chrono::{DateTime};
use harbory_protocol::v1::VolumeInfo;

use crate::docker_inspect::in_use_volume_names;

/// Read-only Docker volume inspection plus explicit removal, served to the
/// control plane on demand — same shape as `NetworksManager`. Volumes
/// currently mounted by any container (running or stopped) are marked
/// `in_use = true` so the UI can disable removal instead of surfacing
/// Docker's conflict error.
pub struct VolumesManager {
    docker: Docker,
}

impl VolumesManager {
    pub fn connect() -> Result<Self, bollard::errors::Error> {
        Ok(Self { docker: Docker::connect_with_local_defaults()? })
    }

    /// Every volume on this host with an `in_use` flag computed from the
    /// containers currently known to Docker (any state).
    pub async fn list(&self) -> Result<Vec<VolumeInfo>, bollard::errors::Error> {
        let in_use_names = in_use_volume_names(&self.docker).await?;

        let volumes = self.docker.list_volumes::<String>(None).await?;

        Ok(volumes
            .volumes
            .unwrap_or_default()
            .into_iter()
            .map(|v| {
                let name = v.name;
                let created_at = v.created_at.as_deref().and_then(parse_rfc3339_to_unix).unwrap_or(0);
                VolumeInfo {
                    name: name.clone(),
                    driver: v.driver,
                    mountpoint: v.mountpoint,
                    created_at,
                    in_use: in_use_names.contains(&name),
                }
            })
            .collect())
    }

    /// Removes one volume by name. Fails if the volume is in use by any
    /// container (Docker returns a conflict error).
    pub async fn remove(&self, name: &str) -> Result<(), bollard::errors::Error> {
        self.docker.remove_volume(name, None).await?;
        Ok(())
    }
}

/// Parse RFC3339 timestamp string to unix seconds.
fn parse_rfc3339_to_unix(s: &str) -> Option<i64> {
    DateTime::parse_from_rfc3339(s).ok().map(|dt| dt.timestamp())
}