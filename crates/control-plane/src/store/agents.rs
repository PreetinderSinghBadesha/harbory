use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::Store;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AgentRecord {
    pub id: Uuid,
    pub account_id: Uuid,
    pub public_key: Vec<u8>,
    pub public_key_fingerprint: Vec<u8>,
    pub status: String,
}

/// A row for the (Phase 5 will replace this with a real dashboard) basic
/// agent-list endpoint. `online` is computed at query time from
/// `last_heartbeat_at`, not stored — see migration 0002.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AgentSummary {
    pub id: Uuid,
    pub account_id: Uuid,
    pub status: String,
    pub last_heartbeat_at: Option<DateTime<Utc>>,
    pub online: bool,
}

impl Store {
    pub async fn get_agent(&self, agent_id: Uuid) -> Result<Option<AgentRecord>, sqlx::Error> {
        sqlx::query_as::<_, AgentRecord>(
            "SELECT id, account_id, public_key, public_key_fingerprint, status
             FROM agents WHERE id = $1",
        )
        .bind(agent_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn record_heartbeat(&self, agent_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE agents SET last_heartbeat_at = now() WHERE id = $1")
            .bind(agent_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// `online` is true iff a heartbeat has been seen within
    /// `online_threshold_seconds` of now.
    pub async fn list_agents(
        &self,
        online_threshold_seconds: i64,
    ) -> Result<Vec<AgentSummary>, sqlx::Error> {
        sqlx::query_as::<_, AgentSummary>(
            "SELECT id, account_id, status, last_heartbeat_at,
                    (last_heartbeat_at IS NOT NULL
                        AND now() - last_heartbeat_at < make_interval(secs => $1::double precision)) AS online
             FROM agents
             ORDER BY created_at",
        )
        .bind(online_threshold_seconds)
        .fetch_all(&self.pool)
        .await
    }
}
