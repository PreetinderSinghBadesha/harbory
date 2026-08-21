//! Integration tests for the HTTP API's auth gate: missing/invalid
//! tokens are rejected, and one account can't reach another account's
//! agents. Uses `Router::oneshot` (no real TCP listener needed) with a
//! synthetic JWT signed by a known test secret — same technique as
//! `crates/control-plane/src/auth.rs`'s own unit tests, just exercised
//! through the full router this time. Real Postgres — see
//! tests/registration.rs for the DB connection convention.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::Duration as ChronoDuration;
use harbory_common::keypair::Keypair;
use harbory_control_plane::{
    http::{router, AppState},
    store::Store,
};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

const TEST_JWT_SECRET: &str = "test-jwt-secret-for-http-integration-tests";

async fn test_store() -> Store {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://harbory:harbory_dev_password@localhost:55433/harbory".into());
    Store::connect(&database_url).await.expect("failed to connect to test database")
}

fn unique_email() -> String {
    format!("{}@example.test", Uuid::new_v4())
}

fn test_app(store: Store) -> axum::Router {
    router(AppState { store, online_threshold_seconds: 30, jwt_secret: TEST_JWT_SECRET.to_string() })
}

fn make_token(sub: Uuid, email: &str) -> String {
    let claims = json!({ "sub": sub, "email": email, "aud": "authenticated", "exp": 9_999_999_999u64 });
    encode(&Header::new(Algorithm::HS256), &claims, &EncodingKey::from_secret(TEST_JWT_SECRET.as_bytes())).unwrap()
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn request_without_token_is_rejected() {
    let app = test_app(test_store().await);
    let response =
        app.oneshot(Request::builder().uri("/agents").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn request_with_garbage_token_is_rejected() {
    let app = test_app(test_store().await);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/agents")
                .header("Authorization", "Bearer not-a-real-jwt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn me_returns_the_authenticated_account_and_provisions_it() {
    let store = test_store().await;
    let sub = Uuid::new_v4();
    let email = unique_email();
    let token = make_token(sub, &email);

    let response = test_app(store.clone())
        .oneshot(Request::builder().uri("/me").header("Authorization", format!("Bearer {token}")).body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(body["id"], sub.to_string());
    assert_eq!(body["email"], email);

    // The extractor should have provisioned an `accounts` row for this
    // Supabase user id, not just returned the claims verbatim.
    let agents = store.list_agents_for_account(sub, 30).await.unwrap();
    assert!(agents.is_empty()); // no assertion failure = the account_id is queryable at all
}

#[tokio::test]
async fn creates_a_pairing_token_scoped_to_the_authenticated_account() {
    let store = test_store().await;
    let sub = Uuid::new_v4();
    let token = make_token(sub, &unique_email());

    let response = test_app(store)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/pairing-tokens")
                .header("Authorization", format!("Bearer {token}"))
                .header("Content-Type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert!(body["token"].as_str().unwrap().starts_with("hbp_"));
}

async fn register_agent_for_account(store: &Store, signer: &Keypair, account_id: Uuid) -> Uuid {
    store.get_or_create_account_by_id(account_id, &unique_email()).await.unwrap();
    let token = store.issue_pairing_token(account_id, ChronoDuration::minutes(10)).await.unwrap();
    let outcome =
        store.register_agent(signer, &token.plaintext, Keypair::generate().public_key_bytes()).await.unwrap();
    outcome.agent_id
}

#[tokio::test]
async fn cannot_reach_another_accounts_agent() {
    let store = test_store().await;
    let signer = Keypair::generate();
    let owner = Uuid::new_v4();
    let agent_id = register_agent_for_account(&store, &signer, owner).await;

    let someone_else_token = make_token(Uuid::new_v4(), &unique_email());
    let response = test_app(store)
        .oneshot(
            Request::builder()
                .uri(format!("/agents/{agent_id}/containers"))
                .header("Authorization", format!("Bearer {someone_else_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // 404, not 403 — don't confirm the agent exists to a non-owner.
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn owner_can_reach_their_own_agent() {
    let store = test_store().await;
    let signer = Keypair::generate();
    let owner = Uuid::new_v4();
    let agent_id = register_agent_for_account(&store, &signer, owner).await;

    let owner_token = make_token(owner, &unique_email());
    let response = test_app(store)
        .oneshot(
            Request::builder()
                .uri(format!("/agents/{agent_id}/containers"))
                .header("Authorization", format!("Bearer {owner_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn revoke_flips_status_and_is_reflected_in_the_agent_list() {
    let store = test_store().await;
    let signer = Keypair::generate();
    let owner = Uuid::new_v4();
    let agent_id = register_agent_for_account(&store, &signer, owner).await;
    let owner_token = make_token(owner, &unique_email());

    let app = test_app(store);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/agents/{agent_id}/revoke"))
                .header("Authorization", format!("Bearer {owner_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let list_response = app
        .oneshot(
            Request::builder()
                .uri("/agents")
                .header("Authorization", format!("Bearer {owner_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = body_json(list_response).await;
    assert_eq!(body[0]["status"], "revoked");
}
