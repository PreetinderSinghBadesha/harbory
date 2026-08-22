use axum::{extract::FromRequestParts, http::request::Parts, http::StatusCode};
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use uuid::Uuid;

use crate::http::AppState;
use crate::jwks::JwkVerifier;

/// Only the claims we actually need. Supabase JWTs carry many more (role,
/// app_metadata, etc.) — serde ignores fields we don't declare, so there's
/// no need to model the full shape.
#[derive(Debug, Deserialize)]
struct Claims {
    sub: Uuid,
    email: Option<String>,
}

/// Extractor for any handler that needs to know who's calling. Verifies
/// the `Authorization: Bearer <jwt>` header (see `verify_jwt` below for
/// which of Supabase's two signing schemes that means) and, on success,
/// provisions/updates the local `accounts` row for this Supabase user id.
/// Ownership checks against a specific `agent_id` are each handler's own
/// job (Supabase only proves *who* is calling, not what they're allowed
/// to touch) — see e.g. `require_owned_agent` in `http.rs`.
#[derive(Debug, Clone)]
pub struct AuthenticatedAccount {
    pub id: Uuid,
    pub email: Option<String>,
}

/// Verifies a token signed with the legacy shared HS256 secret.
fn verify_hs256(token: &str, secret: &[u8]) -> Result<Claims, jsonwebtoken::errors::Error> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_audience(&["authenticated"]); // Supabase's convention for user-facing tokens
    let data = decode::<Claims>(token, &DecodingKey::from_secret(secret), &validation)?;
    Ok(data.claims)
}

/// Verifies a token signed with Supabase's newer asymmetric ES256 signing
/// key, already resolved by the caller to the key matching the token's
/// `kid`.
fn verify_es256(token: &str, key: &DecodingKey) -> Result<Claims, jsonwebtoken::errors::Error> {
    let mut validation = Validation::new(Algorithm::ES256);
    validation.set_audience(&["authenticated"]);
    let data = decode::<Claims>(token, key, &validation)?;
    Ok(data.claims)
}

/// Supabase signs a session JWT with either the legacy shared HS256
/// secret or, for newer projects, an asymmetric ES256 key advertised via
/// its JWKS endpoint (`jwks.rs`) — the token's own header says which.
/// That header is read only to pick a verification path; the actual
/// signature check against the matching secret/key still happens either
/// way, so a caller can't just claim "trust me, HS256" to bypass
/// anything. See docs/dashboard.md for why both paths exist.
fn verify_jwt(token: &str, jwt_secret: Option<&str>, jwks: &JwkVerifier) -> Result<Claims, jsonwebtoken::errors::Error> {
    use jsonwebtoken::errors::ErrorKind;

    let header = decode_header(token)?;
    match header.alg {
        Algorithm::HS256 => {
            let secret = jwt_secret.ok_or(ErrorKind::InvalidToken)?;
            verify_hs256(token, secret.as_bytes())
        }
        Algorithm::ES256 => {
            let kid = header.kid.ok_or(ErrorKind::InvalidToken)?;
            let key = jwks.get(&kid).ok_or(ErrorKind::InvalidToken)?;
            verify_es256(token, key)
        }
        _ => Err(ErrorKind::InvalidAlgorithm.into()),
    }
}

#[axum::async_trait]
impl FromRequestParts<AppState> for AuthenticatedAccount {
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or(StatusCode::UNAUTHORIZED)?;
        let token = header.strip_prefix("Bearer ").ok_or(StatusCode::UNAUTHORIZED)?;

        let claims = verify_jwt(token, state.jwt_secret.as_deref(), &state.jwks).map_err(|err| {
            tracing::debug!(?err, "rejected request with invalid/expired token");
            StatusCode::UNAUTHORIZED
        })?;

        let email = claims.email.clone().unwrap_or_default();
        state.store.get_or_create_account_by_id(claims.sub, &email).await.map_err(|err| {
            tracing::error!(?err, "failed to provision account from verified JWT");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        Ok(AuthenticatedAccount { id: claims.sub, email: claims.email })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde_json::json;

    fn make_hs256_token(secret: &[u8], claims: serde_json::Value) -> String {
        encode(&Header::new(Algorithm::HS256), &claims, &EncodingKey::from_secret(secret)).unwrap()
    }

    #[test]
    fn accepts_a_well_formed_hs256_token() {
        let secret = b"test-secret";
        let sub = Uuid::new_v4();
        let token = make_hs256_token(
            secret,
            json!({ "sub": sub, "email": "user@example.test", "aud": "authenticated", "exp": 9_999_999_999u64 }),
        );

        let claims = verify_jwt(&token, Some("test-secret"), &JwkVerifier::empty()).unwrap();
        assert_eq!(claims.sub, sub);
        assert_eq!(claims.email.as_deref(), Some("user@example.test"));
    }

    #[test]
    fn rejects_wrong_hs256_secret() {
        let sub = Uuid::new_v4();
        let token = make_hs256_token(b"real-secret", json!({ "sub": sub, "aud": "authenticated", "exp": 9_999_999_999u64 }));
        assert!(verify_jwt(&token, Some("wrong-secret"), &JwkVerifier::empty()).is_err());
    }

    #[test]
    fn rejects_expired_hs256_token() {
        let secret = b"test-secret";
        let sub = Uuid::new_v4();
        let token = make_hs256_token(secret, json!({ "sub": sub, "aud": "authenticated", "exp": 1u64 }));
        assert!(verify_jwt(&token, Some("test-secret"), &JwkVerifier::empty()).is_err());
    }

    #[test]
    fn rejects_wrong_hs256_audience() {
        let secret = b"test-secret";
        let sub = Uuid::new_v4();
        let token = make_hs256_token(secret, json!({ "sub": sub, "aud": "some-other-audience", "exp": 9_999_999_999u64 }));
        assert!(verify_jwt(&token, Some("test-secret"), &JwkVerifier::empty()).is_err());
    }

    #[test]
    fn rejects_hs256_token_when_no_secret_configured() {
        let sub = Uuid::new_v4();
        let token = make_hs256_token(b"any-secret", json!({ "sub": sub, "aud": "authenticated", "exp": 9_999_999_999u64 }));
        assert!(verify_jwt(&token, None, &JwkVerifier::empty()).is_err());
    }

    // Fixed P-256 test keypair, generated once for these tests only — not
    // a secret used anywhere real. Public coordinates match the private
    // key below; `from_components` mirrors what `JwkVerifier::fetch`
    // would build from a real JWKS response.
    const TEST_EC_PRIVATE_KEY_PEM: &str = "-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgRVRY4EqELnkAAMyl
STQwb46V0kZXrvNz/UcLgSvuD+ehRANCAAS1zOF7ew3W4SPw8FEPv5o+A7jWuZ/W
u+7gb4ASyFmqxTB1Y4VM6XRtiD9kUr+o2mMCqQy9aH+zbkG8PbvsWKcS
-----END PRIVATE KEY-----";
    const TEST_EC_X: &str = "tczhe3sN1uEj8PBRD7-aPgO41rmf1rvu4G-AEshZqsU";
    const TEST_EC_Y: &str = "MHVjhUzpdG2IP2RSv6jaYwKpDL1of7NuQbw9u-xYpxI";
    const TEST_KID: &str = "test-kid";

    fn make_es256_token(kid: &str, claims: serde_json::Value) -> String {
        let mut header = Header::new(Algorithm::ES256);
        header.kid = Some(kid.to_string());
        let key = EncodingKey::from_ec_pem(TEST_EC_PRIVATE_KEY_PEM.as_bytes()).unwrap();
        encode(&header, &claims, &key).unwrap()
    }

    #[test]
    fn accepts_a_well_formed_es256_token() {
        let sub = Uuid::new_v4();
        let token = make_es256_token(
            TEST_KID,
            json!({ "sub": sub, "email": "user@example.test", "aud": "authenticated", "exp": 9_999_999_999u64 }),
        );
        let jwks = JwkVerifier::from_components(TEST_KID, TEST_EC_X, TEST_EC_Y).unwrap();

        let claims = verify_jwt(&token, None, &jwks).unwrap();
        assert_eq!(claims.sub, sub);
    }

    #[test]
    fn rejects_es256_token_with_unknown_kid() {
        let sub = Uuid::new_v4();
        let token = make_es256_token("some-other-kid", json!({ "sub": sub, "aud": "authenticated", "exp": 9_999_999_999u64 }));
        let jwks = JwkVerifier::from_components(TEST_KID, TEST_EC_X, TEST_EC_Y).unwrap();

        assert!(verify_jwt(&token, None, &jwks).is_err());
    }

    #[test]
    fn rejects_es256_token_when_no_jwks_configured() {
        let sub = Uuid::new_v4();
        let token = make_es256_token(TEST_KID, json!({ "sub": sub, "aud": "authenticated", "exp": 9_999_999_999u64 }));

        assert!(verify_jwt(&token, None, &JwkVerifier::empty()).is_err());
    }
}
