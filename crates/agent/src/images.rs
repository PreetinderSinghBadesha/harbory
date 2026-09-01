use bollard::image::{ListImagesOptions, RemoveImageOptions};
use bollard::Docker;
use harbory_protocol::v1::ImageInfo;

use crate::docker_inspect::in_use_image_ids;

/// Read-only Docker image inspection plus explicit removal, served to the
/// control plane on demand over the log-request pattern (request/response
/// over the persistent stream). Removal is deliberately non-forced: an
/// image backing any container (running or stopped) fails with Docker's
/// own conflict error rather than silently yanking it out from under a
/// deployed stack — the UI marks in-use images so this rarely triggers.
pub struct ImagesManager {
    docker: Docker,
}

impl ImagesManager {
    pub fn connect() -> Result<Self, bollard::errors::Error> {
        Ok(Self { docker: Docker::connect_with_local_defaults()? })
    }

    /// Every image on this host with an `in_use` flag computed from the
    /// containers currently known to Docker (any state).
    pub async fn list(&self) -> Result<Vec<ImageInfo>, bollard::errors::Error> {
        let in_use_ids = in_use_image_ids(&self.docker).await?;

        let images = self
            .docker
            .list_images(Some(ListImagesOptions::<String> { all: false, ..Default::default() }))
            .await?;

        Ok(images
            .into_iter()
            .map(|img| {
                let id = img.id;
                ImageInfo {
                    in_use: in_use_ids.contains(&id),
                    id,
                    repo_tags: img.repo_tags,
                    size_bytes: img.size,
                    created_at: img.created,
                }
            })
            .collect())
    }

    /// Removes one image by ID. Non-forced: an image still backing a
    /// container returns Docker's conflict error, which the control plane
    /// surfaces verbatim.
    pub async fn remove(&self, id: &str) -> Result<(), bollard::errors::Error> {
        self.docker
            .remove_image(id, Some(RemoveImageOptions { force: false, noprune: false }), None)
            .await?;
        Ok(())
    }
}
