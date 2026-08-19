use uuid::Uuid;

use super::Store;

#[derive(Debug, Clone, Copy)]
pub enum AuditEventType {
    PairingSuccess,
    PairingTokenReuse,
    PairingTokenExpired,
    CredentialFingerprintMismatch,
}

impl AuditEventType {
    fn as_str(self) -> &'static str {
        match self {
            Self::PairingSuccess => "pairing_success",
            Self::PairingTokenReuse => "pairing_token_reuse",
            Self::PairingTokenExpired => "pairing_token_expired",
            Self::CredentialFingerprintMismatch => "credential_fingerprint_mismatch",
        }
    }
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
}
