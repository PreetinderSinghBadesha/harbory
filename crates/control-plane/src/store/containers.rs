use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::reconcile::{DesiredContainer, DesiredStatus, GitSource, ObservedContainer, ObservedStatus, PortMapping};

use super::Store;

#[derive(Serialize, Deserialize)]
struct PortMappingJson {
    host_port: u16,
    container_port: u16,
}

fn desired_status_str(status: DesiredStatus) -> &'static str {
    match status {
        DesiredStatus::Running => "running",
        DesiredStatus::Absent => "absent",
    }
}

fn parse_desired_status(s: &str) -> DesiredStatus {
    match s {
        "absent" => DesiredStatus::Absent,
        _ => DesiredStatus::Running,
    }
}

fn observed_status_str(status: ObservedStatus) -> &'static str {
    match status {
        ObservedStatus::Running => "running",
        ObservedStatus::Stopped => "stopped",
        ObservedStatus::Removed => "removed",
        ObservedStatus::Error => "error",
    }
}

fn parse_observed_status(s: &str) -> ObservedStatus {
    match s {
        "running" => ObservedStatus::Running,
        "stopped" => ObservedStatus::Stopped,
        "error" => ObservedStatus::Error,
        _ => ObservedStatus::Removed,
    }
}

#[derive(sqlx::FromRow)]
struct DesiredRow {
    name: String,
    image: String,
    env: Vec<String>,
    port_mappings: serde_json::Value,
    command: Vec<String>,
    desired_status: String,
    git_repo_url: Option<String>,
    git_ref: Option<String>,
    git_dockerfile_path: Option<String>,
}

impl DesiredRow {
    fn into_domain(self) -> DesiredContainer {
        let ports: Vec<PortMappingJson> = serde_json::from_value(self.port_mappings).unwrap_or_default();
        // Presence of git_repo_url is the signal — the other two are
        // optional refinements (empty ref = default branch, empty
        // dockerfile path = "Dockerfile" at root), not independently
        // meaningful.
        let git_source = self.git_repo_url.map(|repo_url| GitSource {
            repo_url,
            git_ref: self.git_ref.unwrap_or_default(),
            dockerfile_path: self.git_dockerfile_path.unwrap_or_default(),
        });
        DesiredContainer {
            name: self.name,
            image: self.image,
            env: self.env,
            ports: ports.into_iter().map(|p| PortMapping { host_port: p.host_port, container_port: p.container_port }).collect(),
            command: self.command,
            status: parse_desired_status(&self.desired_status),
            git_source,
        }
    }
}

#[derive(sqlx::FromRow)]
struct ObservedRow {
    name: String,
    image: String,
    status: String,
}

impl ObservedRow {
    fn into_domain(self) -> ObservedContainer {
        ObservedContainer { name: self.name, image: self.image, status: parse_observed_status(&self.status) }
    }
}

impl Store {
    /// Declares (or updates) the desired state for one container. Running
    /// containers are re-declared with a fresh spec on every call
    /// (upsert), so changing e.g. the image is just calling this again.
    pub async fn upsert_desired_container(
        &self,
        agent_id: Uuid,
        container: &DesiredContainer,
    ) -> Result<(), sqlx::Error> {
        let ports: Vec<PortMappingJson> = container
            .ports
            .iter()
            .map(|p| PortMappingJson { host_port: p.host_port, container_port: p.container_port })
            .collect();
        let ports_json = serde_json::to_value(ports).expect("port mappings are always serializable");
        let (git_repo_url, git_ref, git_dockerfile_path) = match &container.git_source {
            Some(g) => (Some(g.repo_url.as_str()), Some(g.git_ref.as_str()), Some(g.dockerfile_path.as_str())),
            None => (None, None, None),
        };

        sqlx::query(
            "INSERT INTO desired_containers
                (agent_id, name, image, env, port_mappings, command, desired_status,
                 git_repo_url, git_ref, git_dockerfile_path, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, now())
             ON CONFLICT (agent_id, name) DO UPDATE
             SET image = $3, env = $4, port_mappings = $5, command = $6, desired_status = $7,
                 git_repo_url = $8, git_ref = $9, git_dockerfile_path = $10, updated_at = now()",
        )
        .bind(agent_id)
        .bind(&container.name)
        .bind(&container.image)
        .bind(&container.env)
        .bind(ports_json)
        .bind(&container.command)
        .bind(desired_status_str(container.status))
        .bind(git_repo_url)
        .bind(git_ref)
        .bind(git_dockerfile_path)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Marks a previously-declared container as desired-absent. Returns
    /// `false` (no-op) if nothing was ever declared under that name —
    /// there's no "running" declaration to override, and an
    /// undeclared/absent container is already left alone by `reconcile::diff`.
    pub async fn set_desired_absent(&self, agent_id: Uuid, name: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE desired_containers SET desired_status = 'absent', updated_at = now()
             WHERE agent_id = $1 AND name = $2",
        )
        .bind(agent_id)
        .bind(name)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn get_desired_containers(&self, agent_id: Uuid) -> Result<Vec<DesiredContainer>, sqlx::Error> {
        let rows = sqlx::query_as::<_, DesiredRow>(
            "SELECT name, image, env, port_mappings, command, desired_status,
                    git_repo_url, git_ref, git_dockerfile_path
             FROM desired_containers WHERE agent_id = $1 ORDER BY name",
        )
        .bind(agent_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(DesiredRow::into_domain).collect())
    }

    /// Replaces the entire observed snapshot for an agent — matches the
    /// wire format (`ContainerStateReport` is a full snapshot, not a
    /// delta), so storage mirrors that rather than merging incrementally.
    pub async fn replace_observed_containers(
        &self,
        agent_id: Uuid,
        containers: &[ObservedContainer],
    ) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("DELETE FROM observed_containers WHERE agent_id = $1")
            .bind(agent_id)
            .execute(&mut *tx)
            .await?;

        for c in containers {
            sqlx::query(
                "INSERT INTO observed_containers (agent_id, name, image, status, reported_at)
                 VALUES ($1, $2, $3, $4, now())",
            )
            .bind(agent_id)
            .bind(&c.name)
            .bind(&c.image)
            .bind(observed_status_str(c.status))
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await
    }

    pub async fn get_observed_containers(&self, agent_id: Uuid) -> Result<Vec<ObservedContainer>, sqlx::Error> {
        let rows = sqlx::query_as::<_, ObservedRow>(
            "SELECT name, image, status FROM observed_containers WHERE agent_id = $1 ORDER BY name",
        )
        .bind(agent_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(ObservedRow::into_domain).collect())
    }
}
