use axum::{extract::FromRequestParts, http::request::Parts, http::StatusCode};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use uuid::Uuid;

use crate::http::AppState;

/// Only the claims we actually need. Supabase JWTs carry many more (role,
/// app_metadata, etc.) — serde ignores fields we don't declare, so there's
/// no need to model the full shape.
#[derive(Debug, Deserialize)]
struct Claims {
    sub: Uuid,
    email: Option<String>,
}

/// Extractor for any handler that needs to know who's calling. Verifies
/// the `Authorization: Bearer <jwt>` header against `state.jwt_secret`
/// (Supabase's JWT secret — see docs/dashboard.md for where that comes
/// from) and, on success, provisions/updates the local `accounts` row for
/// this Supabase user id. Ownership checks against a specific `agent_id`
/// are each handler's own job (Supabase only proves *who* is calling, not
/// what they're allowed to touch) — see e.g. `require_owned_agent` in
/// `http.rs`.
#[derive(Debug, Clone)]
pub struct AuthenticatedAccount {
    pub id: Uuid,
    pub email: Option<String>,
}

fn verify_jwt(token: &str, secret: &[u8]) -> Result<Claims, jsonwebtoken::errors::Error> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_audience(&["authenticated"]); // Supabase's convention for user-facing tokens
    let data = decode::<Claims>(token, &DecodingKey::from_secret(secret), &validation)?;
    Ok(data.claims)
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

        let claims = verify_jwt(token, state.jwt_secret.as_bytes()).map_err(|err| {
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

    fn make_token(secret: &[u8], claims: serde_json::Value) -> String {
        encode(&Header::new(Algorithm::HS256), &claims, &EncodingKey::from_secret(secret)).unwrap()
    }

    #[test]
    fn accepts_a_well_formed_token() {
        let secret = b"test-secret";
        let sub = Uuid::new_v4();
        let token = make_token(
            secret,
            json!({ "sub": sub, "email": "user@example.test", "aud": "authenticated", "exp": 9_999_999_999u64 }),
        );

        let claims = verify_jwt(&token, secret).unwrap();
        assert_eq!(claims.sub, sub);
        assert_eq!(claims.email.as_deref(), Some("user@example.test"));
    }

    #[test]
    fn rejects_wrong_secret() {
        let sub = Uuid::new_v4();
        let token = make_token(b"real-secret", json!({ "sub": sub, "aud": "authenticated", "exp": 9_999_999_999u64 }));
        assert!(verify_jwt(&token, b"wrong-secret").is_err());
    }

    #[test]
    fn rejects_expired_token() {
        let secret = b"test-secret";
        let sub = Uuid::new_v4();
        let token = make_token(secret, json!({ "sub": sub, "aud": "authenticated", "exp": 1u64 }));
        assert!(verify_jwt(&token, secret).is_err());
    }

    #[test]
    fn rejects_wrong_audience() {
        let secret = b"test-secret";
        let sub = Uuid::new_v4();
        let token = make_token(secret, json!({ "sub": sub, "aud": "some-other-audience", "exp": 9_999_999_999u64 }));
        assert!(verify_jwt(&token, secret).is_err());
    }
}
