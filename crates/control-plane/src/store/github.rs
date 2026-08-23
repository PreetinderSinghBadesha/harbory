use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{DateTime, Duration, Utc};
use rand::RngCore;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::Store;

pub struct IssuedOAuthState {
    /// Round-tripped through GitHub as the `state` query param; never
    /// stored — only its hash is, same rationale as pairing tokens.
    pub plaintext: String,
    pub expires_at: DateTime<Utc>,
}

fn generate_state_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    format!("ghs_{}", URL_SAFE_NO_PAD.encode(bytes))
}

fn hash_state(plaintext: &str) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(plaintext.as_bytes());
    hasher.finalize().to_vec()
}

#[derive(Debug, thiserror::Error)]
pub enum ConsumeStateError {
    #[error("oauth state not recognized")]
    Invalid,
    #[error("oauth state was already used")]
    AlreadyUsed,
    #[error("oauth state has expired")]
    Expired,
    #[error(transparent)]
    Db(#[from] sqlx::Error),
}

#[derive(sqlx::FromRow)]
struct StateRow {
    account_id: Uuid,
    expires_at: DateTime<Utc>,
    consumed_at: Option<DateTime<Utc>>,
}

#[derive(sqlx::FromRow)]
pub struct GitHubConnectionRecord {
    pub access_token: String,
    pub github_login: String,
    pub connected_at: DateTime<Utc>,
}

impl Store {
    /// Issues a single-use CSRF state for the OAuth redirect round trip —
    /// same shape as `issue_pairing_token`, for the same reason
    /// (short-lived, single-use, only ever compared by hash).
    pub async fn issue_github_oauth_state(
        &self,
        account_id: Uuid,
        ttl: Duration,
    ) -> Result<IssuedOAuthState, sqlx::Error> {
        let plaintext = generate_state_token();
        let expires_at = Utc::now() + ttl;

        sqlx::query("INSERT INTO github_oauth_states (state_hash, account_id, expires_at) VALUES ($1, $2, $3)")
            .bind(hash_state(&plaintext))
            .bind(account_id)
            .bind(expires_at)
            .execute(&self.pool)
            .await?;

        Ok(IssuedOAuthState { plaintext, expires_at })
    }

    /// Validates + consumes the state token, returning the account it was
    /// issued for. Row-locked (`FOR UPDATE`) the same way
    /// `register_agent` consumes a pairing token, so two concurrent
    /// callback hits with the same state can't both succeed.
    pub async fn consume_github_oauth_state(&self, plaintext: &str) -> Result<Uuid, ConsumeStateError> {
        let state_hash = hash_state(plaintext);
        let mut tx = self.pool.begin().await?;

        let row = sqlx::query_as::<_, StateRow>(
            "SELECT account_id, expires_at, consumed_at FROM github_oauth_states WHERE state_hash = $1 FOR UPDATE",
        )
        .bind(&state_hash)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(row) = row else {
            tx.rollback().await.ok();
            return Err(ConsumeStateError::Invalid);
        };
        if row.consumed_at.is_some() {
            tx.rollback().await.ok();
            return Err(ConsumeStateError::AlreadyUsed);
        }
        if row.expires_at < Utc::now() {
            tx.rollback().await.ok();
            return Err(ConsumeStateError::Expired);
        }

        sqlx::query("UPDATE github_oauth_states SET consumed_at = now() WHERE state_hash = $1")
            .bind(&state_hash)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;

        Ok(row.account_id)
    }

    pub async fn upsert_github_connection(
        &self,
        account_id: Uuid,
        access_token: &str,
        github_login: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO github_connections (account_id, access_token, github_login, connected_at)
             VALUES ($1, $2, $3, now())
             ON CONFLICT (account_id) DO UPDATE
             SET access_token = $2, github_login = $3, connected_at = now()",
        )
        .bind(account_id)
        .bind(access_token)
        .bind(github_login)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_github_connection(
        &self,
        account_id: Uuid,
    ) -> Result<Option<GitHubConnectionRecord>, sqlx::Error> {
        sqlx::query_as::<_, GitHubConnectionRecord>(
            "SELECT access_token, github_login, connected_at FROM github_connections WHERE account_id = $1",
        )
        .bind(account_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Returns `false` if there was nothing to delete — same
    /// no-op-vs-error convention as `set_desired_absent`.
    pub async fn delete_github_connection(&self, account_id: Uuid) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM github_connections WHERE account_id = $1")
            .bind(account_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}
