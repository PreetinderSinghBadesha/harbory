use chrono::{DateTime, Utc};
use harbory_common::{
    credential::{sign_credential, verify_credential, CredentialPayload},
    fingerprint::fingerprint,
    keypair::Keypair,
};
use uuid::Uuid;

use super::{audit::AuditEventType, pairing::hash_token, Store};

pub struct RegisterOutcome {
    pub agent_id: Uuid,
    pub account_id: Uuid,
    /// Opaque signed credential bytes to hand back to the agent.
    pub credential: Vec<u8>,
}

#[derive(Debug, thiserror::Error)]
pub enum RegisterError {
    #[error("pairing token not recognized")]
    InvalidToken,
    #[error("pairing token was already used")]
    TokenAlreadyUsed,
    #[error("pairing token has expired")]
    TokenExpired,
    #[error(transparent)]
    Db(#[from] sqlx::Error),
}

#[derive(sqlx::FromRow)]
struct PairingTokenRow {
    account_id: Uuid,
    expires_at: DateTime<Utc>,
    consumed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum VerifyCredentialError {
    #[error("credential is malformed or has an invalid signature")]
    Invalid,
    #[error("agent is not known to the control plane")]
    UnknownAgent,
    #[error("agent has been revoked")]
    Revoked,
    #[error("credential public key fingerprint does not match the agent's registered key")]
    FingerprintMismatch,
}

impl Store {
    /// Atomically: validate + consume the pairing token, create the agent
    /// record, and sign a credential for it. A row lock on the token
    /// (`FOR UPDATE`) prevents two concurrent registrations from both
    /// succeeding with the same token.
    pub async fn register_agent(
        &self,
        signer: &Keypair,
        pairing_token_plaintext: &str,
        public_key: [u8; 32],
    ) -> Result<RegisterOutcome, RegisterError> {
        let token_hash = hash_token(pairing_token_plaintext);

        let mut tx = self.pool.begin().await?;

        let row = sqlx::query_as::<_, PairingTokenRow>(
            "SELECT account_id, expires_at, consumed_at FROM pairing_tokens
             WHERE token_hash = $1 FOR UPDATE",
        )
        .bind(&token_hash)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(row) = row else {
            tx.rollback().await.ok();
            return Err(RegisterError::InvalidToken);
        };

        if let Some(_consumed_at) = row.consumed_at {
            tx.rollback().await.ok();
            let _ = self
                .record_audit_event(
                    AuditEventType::PairingTokenReuse,
                    Some(row.account_id),
                    None,
                    serde_json::json!({ "detail": "reuse of an already-consumed pairing token" }),
                )
                .await;
            return Err(RegisterError::TokenAlreadyUsed);
        }

        if row.expires_at < Utc::now() {
            tx.rollback().await.ok();
            let _ = self
                .record_audit_event(
                    AuditEventType::PairingTokenExpired,
                    Some(row.account_id),
                    None,
                    serde_json::json!({}),
                )
                .await;
            return Err(RegisterError::TokenExpired);
        }

        let agent_id = Uuid::new_v4();
        let fp = fingerprint(&public_key);

        sqlx::query(
            "INSERT INTO agents (id, account_id, public_key, public_key_fingerprint)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(agent_id)
        .bind(row.account_id)
        .bind(public_key.as_slice())
        .bind(fp.as_slice())
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "UPDATE pairing_tokens SET consumed_at = now(), consumed_by_agent_id = $2
             WHERE token_hash = $1",
        )
        .bind(&token_hash)
        .bind(agent_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        let _ = self
            .record_audit_event(
                AuditEventType::PairingSuccess,
                Some(row.account_id),
                Some(agent_id),
                serde_json::json!({}),
            )
            .await;

        let payload = CredentialPayload {
            agent_id,
            account_id: row.account_id,
            public_key_fingerprint: fp,
            issued_at: Utc::now().timestamp(),
        };
        let credential = sign_credential(&payload, signer);

        Ok(RegisterOutcome {
            agent_id,
            account_id: row.account_id,
            credential,
        })
    }

    /// Validate a credential presented by an agent: signature must trace
    /// back to `control_plane_public_key`, the agent must still be known
    /// and active, and the fingerprint it carries must match the key on
    /// file for that agent_id. This is the check to run on stream connect
    /// (wired up when the persistent stream lands in Phase 2); proof of
    /// possession of the private key (signing a connect-time challenge) is
    /// part of that same wiring, not this function.
    pub async fn verify_agent_credential(
        &self,
        control_plane_public_key: &[u8; 32],
        credential_bytes: &[u8],
    ) -> Result<super::AgentRecord, VerifyCredentialError> {
        let payload = verify_credential(credential_bytes, control_plane_public_key)
            .map_err(|_| VerifyCredentialError::Invalid)?;

        let agent = self
            .get_agent(payload.agent_id)
            .await
            .map_err(|_| VerifyCredentialError::Invalid)?
            .ok_or(VerifyCredentialError::UnknownAgent)?;

        if agent.status != "active" {
            return Err(VerifyCredentialError::Revoked);
        }

        if agent.public_key_fingerprint != payload.public_key_fingerprint {
            let _ = self
                .record_audit_event(
                    AuditEventType::CredentialFingerprintMismatch,
                    Some(agent.account_id),
                    Some(agent.id),
                    serde_json::json!({ "detail": "possible compromise: fingerprint mismatch" }),
                )
                .await;
            return Err(VerifyCredentialError::FingerprintMismatch);
        }

        Ok(agent)
    }
}
