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
    /// `online_threshold_seconds` of now. Scoped to one account — every
    /// HTTP caller is authenticated now (Phase 5), so "list every agent on
    /// the control plane regardless of owner" is no longer a thing any
    /// endpoint should do.
    pub async fn list_agents_for_account(
        &self,
        account_id: Uuid,
        online_threshold_seconds: i64,
    ) -> Result<Vec<AgentSummary>, sqlx::Error> {
        sqlx::query_as::<_, AgentSummary>(
            "SELECT id, account_id, status, last_heartbeat_at,
                    (last_heartbeat_at IS NOT NULL
                        AND now() - last_heartbeat_at < make_interval(secs => $2::double precision)) AS online
             FROM agents
             WHERE account_id = $1
             ORDER BY created_at",
        )
        .bind(account_id)
        .bind(online_threshold_seconds)
        .fetch_all(&self.pool)
        .await
    }

    /// Per §3: revoked agents can only rejoin via a brand-new pairing
    /// token. This just flips the status flag — `verify_agent_credential`
    /// (Phase 1) already rejects any agent whose status isn't `'active'`,
    /// so that enforcement point doesn't change; this is what actually
    /// lets an operator trigger it. Returns `false` if nothing by that id
    /// existed.
    pub async fn revoke_agent(&self, agent_id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE agents SET status = 'revoked', revoked_at = now()
             WHERE id = $1 AND status = 'active'",
        )
        .bind(agent_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }
}
