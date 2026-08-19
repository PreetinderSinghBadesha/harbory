use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{DateTime, Duration, Utc};
use rand::RngCore;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::Store;

pub struct IssuedPairingToken {
    /// Shown to the user exactly once; never stored.
    pub plaintext: String,
    pub account_id: Uuid,
    pub expires_at: DateTime<Utc>,
}

fn generate_plaintext_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    format!("hbp_{}", URL_SAFE_NO_PAD.encode(bytes))
}

pub(crate) fn hash_token(plaintext: &str) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(plaintext.as_bytes());
    hasher.finalize().to_vec()
}

impl Store {
    /// Issue a new single-use pairing token for `account_id`, valid for `ttl`.
    pub async fn issue_pairing_token(
        &self,
        account_id: Uuid,
        ttl: Duration,
    ) -> Result<IssuedPairingToken, sqlx::Error> {
        let plaintext = generate_plaintext_token();
        let token_hash = hash_token(&plaintext);
        let expires_at = Utc::now() + ttl;

        sqlx::query(
            "INSERT INTO pairing_tokens (token_hash, account_id, expires_at) VALUES ($1, $2, $3)",
        )
        .bind(&token_hash)
        .bind(account_id)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;

        Ok(IssuedPairingToken {
            plaintext,
            account_id,
            expires_at,
        })
    }
}
