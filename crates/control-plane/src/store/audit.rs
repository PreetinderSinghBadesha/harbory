use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::Store;

#[derive(Debug, Clone, Copy)]
pub enum AuditEventType {
    PairingSuccess,
    PairingTokenReuse,
    PairingTokenExpired,
    CredentialFingerprintMismatch,
    AgentRevoked,
}

impl AuditEventType {
    fn as_str(self) -> &'static str {
        match self {
            Self::PairingSuccess => "pairing_success",
            Self::PairingTokenReuse => "pairing_token_reuse",
            Self::PairingTokenExpired => "pairing_token_expired",
            Self::CredentialFingerprintMismatch => "credential_fingerprint_mismatch",
            Self::AgentRevoked => "agent_revoked",
        }
    }
}

/// Misuse signals worth calling out in the dashboard's activity feed
/// distinctly from routine events (successful pairing, an operator's own
/// revoke action) — see docs/observability.md. Matches on the stored
/// string rather than `AuditEventType` since `list_audit_events_for_account`
/// reads rows back from the DB, not typed events.
fn is_misuse_signal(event_type: &str) -> bool {
    matches!(event_type, "pairing_token_reuse" | "credential_fingerprint_mismatch")
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AuditEventRecord {
    pub event_type: String,
    pub agent_id: Option<Uuid>,
    pub detail: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub is_misuse_signal: bool,
}

impl Store {
    pub async fn record_audit_event(
        &self,
        event: AuditEventType,
        account_id: Option<Uuid>,
        agent_id: Option<Uuid>,
        detail: serde_json::Value,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO audit_log (event_type, account_id, agent_id, detail)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(event.as_str())
        .bind(account_id)
        .bind(agent_id)
        .bind(detail)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Most recent security-relevant activity for the dashboard's activity
    /// feed — see docs/observability.md for why this is in-dashboard
    /// rather than email (no external email service is wired up yet).
    pub async fn list_audit_events_for_account(
        &self,
        account_id: Uuid,
        limit: i64,
    ) -> Result<Vec<AuditEventRecord>, sqlx::Error> {
        #[derive(sqlx::FromRow)]
        struct Row {
            event_type: String,
            agent_id: Option<Uuid>,
            detail: serde_json::Value,
            created_at: DateTime<Utc>,
        }

        let rows = sqlx::query_as::<_, Row>(
            "SELECT event_type, agent_id, detail, created_at
             FROM audit_log WHERE account_id = $1
             ORDER BY created_at DESC LIMIT $2",
        )
        .bind(account_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| AuditEventRecord {
                is_misuse_signal: is_misuse_signal(&r.event_type),
                event_type: r.event_type,
                agent_id: r.agent_id,
                detail: r.detail,
                created_at: r.created_at,
            })
            .collect())
    }
}
