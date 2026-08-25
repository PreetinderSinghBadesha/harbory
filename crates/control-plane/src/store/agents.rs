use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::{audit::AuditEventType, Store};

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
    /// existed. Audit-logged (Phase 6) so revocations show up in the
    /// account's activity feed alongside pairing/credential misuse events.
    pub async fn revoke_agent(&self, agent_id: Uuid) -> Result<bool, sqlx::Error> {
        let account_id = sqlx::query_scalar::<_, Uuid>(
            "UPDATE agents SET status = 'revoked', revoked_at = now()
             WHERE id = $1 AND status = 'active'
             RETURNING account_id",
        )
        .bind(agent_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(account_id) = account_id else {
            return Ok(false);
        };

        let _ = self
            .record_audit_event(AuditEventType::AgentRevoked, Some(account_id), Some(agent_id), serde_json::json!({}))
            .await;
        Ok(true)
    }

    /// Permanently removes an agent and everything scoped to it. Nothing
    /// in the schema cascades, so every referencing table is cleaned up
    /// explicitly in one transaction: desired/observed containers, proxy
    /// routes + state, compose stacks, and — for the two nullable
    /// references worth keeping history for — audit_log and pairing
    /// tokens get their agent_id NULLed rather than deleted, so the
    /// security trail survives the agent itself. Returns the owning
    /// account id (for the post-delete audit event, which references the
    /// deleted agent only through its detail JSON since the row is gone),
    /// or None if no agent by that id existed.
    pub async fn delete_agent(&self, agent_id: Uuid) -> Result<Option<Uuid>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("UPDATE audit_log SET agent_id = NULL WHERE agent_id = $1")
            .bind(agent_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE pairing_tokens SET consumed_by_agent_id = NULL WHERE consumed_by_agent_id = $1")
            .bind(agent_id)
            .execute(&mut *tx)
            .await?;
        for table in [
            "desired_containers",
            "observed_containers",
            "desired_proxy_routes",
            "proxy_state",
            "desired_compose_stacks",
            "observed_compose_stacks",
        ] {
            sqlx::query(&format!("DELETE FROM {table} WHERE agent_id = $1"))
                .bind(agent_id)
                .execute(&mut *tx)
                .await?;
        }

        let account_id = sqlx::query_scalar::<_, Uuid>(
            "DELETE FROM agents WHERE id = $1 RETURNING account_id",
        )
        .bind(agent_id)
        .fetch_optional(&mut *tx)
        .await?;

        tx.commit().await?;

        if let Some(account_id) = account_id {
            let _ = self
                .record_audit_event(
                    AuditEventType::AgentDeleted,
                    Some(account_id),
                    None,
                    serde_json::json!({ "deleted_agent_id": agent_id.to_string() }),
                )
                .await;
        }
        Ok(account_id)
    }
}
