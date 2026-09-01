use bollard::container::ListContainersOptions;
use bollard::Docker;
use bollard::models::ContainerSummary;
use std::collections::HashSet;

/// Fetches all containers (running + stopped) from Docker.
async fn list_all_containers(docker: &Docker) -> Result<Vec<ContainerSummary>, bollard::errors::Error> {
    docker
        .list_containers(Some(ListContainersOptions::<String> { all: true, ..Default::default() }))
        .await
}

/// Returns a set of image IDs that are currently used by any container on this host.
pub async fn in_use_image_ids(docker: &Docker) -> Result<HashSet<String>, bollard::errors::Error> {
    let containers = list_all_containers(docker).await?;
    let mut in_use = HashSet::new();
    for c in containers {
        if let Some(id) = c.image_id {
            in_use.insert(id);
        }
    }
    Ok(in_use)
}

/// Returns a set of volume names that are currently mounted by any container on this host.
pub async fn in_use_volume_names(docker: &Docker) -> Result<HashSet<String>, bollard::errors::Error> {
    let containers = list_all_containers(docker).await?;
    let mut in_use = HashSet::new();
    for c in containers {
        if let Some(mounts) = c.mounts {
            for m in mounts {
                if let Some(name) = m.name {
                    in_use.insert(name);
                }
            }
        }
    }
    Ok(in_use)
}

/// Returns both in-use image IDs and volume names in a single Docker round-trip.
/// Currently unused but available for potential future optimization (e.g. batch
/// requests).
#[allow(dead_code)]
pub async fn in_use_image_ids_and_volume_names(
    docker: &Docker,
) -> Result<(HashSet<String>, HashSet<String>), bollard::errors::Error> {
    let containers = list_all_containers(docker).await?;
    let mut image_ids = HashSet::new();
    let mut volume_names = HashSet::new();

    for c in containers {
        if let Some(id) = c.image_id {
            image_ids.insert(id);
        }
        if let Some(mounts) = c.mounts {
            for m in mounts {
                if let Some(name) = m.name {
                    volume_names.insert(name);
                }
            }
        }
    }

    Ok((image_ids, volume_names))
}