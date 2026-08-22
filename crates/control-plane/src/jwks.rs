//! Supabase now signs session JWTs with an asymmetric ES256 key by
//! default (rather than the legacy shared HS256 secret) and publishes
//! the public half at `{project_url}/auth/v1/.well-known/jwks.json`.
//! This fetches that set once at startup so `auth.rs` can verify a
//! token's signature against the key matching its `kid`, without ever
//! needing the private key itself. See docs/dashboard.md.

use std::collections::HashMap;
use std::sync::Arc;

use jsonwebtoken::DecodingKey;
use serde::Deserialize;

#[derive(Clone, Default)]
pub struct JwkVerifier {
    keys: Arc<HashMap<String, DecodingKey>>,
}

#[derive(Deserialize)]
struct JwksResponse {
    keys: Vec<Jwk>,
}

#[derive(Deserialize)]
struct Jwk {
    kid: Option<String>,
    kty: String,
    crv: Option<String>,
    x: Option<String>,
    y: Option<String>,
}

impl JwkVerifier {
    /// No keys known — every ES256 token will fail to verify (`kid` never
    /// found), which is the correct behavior when no Supabase project URL
    /// was configured at all rather than a startup error.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Builds a verifier from raw EC point components directly — the
    /// production path is always `fetch`, but constructing one from known
    /// coordinates is also how tests exercise the ES256 path without a
    /// live network call.
    pub fn from_components(kid: &str, x: &str, y: &str) -> Result<Self, jsonwebtoken::errors::Error> {
        let mut keys = HashMap::new();
        keys.insert(kid.to_string(), DecodingKey::from_ec_components(x, y)?);
        Ok(Self { keys: Arc::new(keys) })
    }

    /// Fetches and parses the JWKS once. Only EC P-256 keys (Supabase's
    /// ES256 signing keys) are kept — any other key type present in the
    /// set is silently skipped rather than treated as an error, since an
    /// unrelated future key type shouldn't block startup.
    pub async fn fetch(supabase_url: &str) -> anyhow::Result<Self> {
        let url = format!("{}/auth/v1/.well-known/jwks.json", supabase_url.trim_end_matches('/'));
        let response = reqwest::get(&url).await?.error_for_status()?;
        let jwks: JwksResponse = response.json().await?;

        let mut keys = HashMap::new();
        for key in jwks.keys {
            let (Some(kid), Some(x), Some(y)) = (key.kid, key.x, key.y) else { continue };
            if key.kty != "EC" || key.crv.as_deref() != Some("P-256") {
                continue;
            }
            if let Ok(decoding_key) = DecodingKey::from_ec_components(&x, &y) {
                keys.insert(kid, decoding_key);
            }
        }
        tracing::info!(count = keys.len(), url = %url, "fetched Supabase JWKS");
        Ok(Self { keys: Arc::new(keys) })
    }

    pub fn get(&self, kid: &str) -> Option<&DecodingKey> {
        self.keys.get(kid)
    }
}
