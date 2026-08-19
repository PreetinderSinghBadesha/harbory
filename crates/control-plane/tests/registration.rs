//! Integration tests against a real Postgres instance (per project decision
//! to skip an in-memory store for Phase 1 — see HARBORY_README.md Progress
//! Log). Point DATABASE_URL at a scratch database before running.

use chrono::Duration;
use harbory_common::{credential::verify_credential, keypair::Keypair};
use harbory_control_plane::store::{RegisterError, Store, VerifyCredentialError};
use uuid::Uuid;

async fn test_store() -> Store {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://harbory:harbory_dev_password@localhost:55433/harbory".into()
    });
    Store::connect(&database_url)
        .await
        .expect("failed to connect to test database")
}

fn unique_email() -> String {
    format!("{}@example.test", Uuid::new_v4())
}

#[tokio::test]
async fn happy_path_pairing_issues_a_valid_credential() {
    let store = test_store().await;
    let signer = Keypair::generate();

    let account_id = store.create_account(&unique_email()).await.unwrap();
    let token = store
        .issue_pairing_token(account_id, Duration::minutes(10))
        .await
        .unwrap();

    let agent_identity = Keypair::generate();
    let outcome = store
        .register_agent(&signer, &token.plaintext, agent_identity.public_key_bytes())
        .await
        .expect("registration should succeed");

    assert_eq!(outcome.account_id, account_id);

    let payload = verify_credential(&outcome.credential, &signer.public_key_bytes())
        .expect("credential must verify against the control plane's public key");
    assert_eq!(payload.agent_id, outcome.agent_id);
    assert_eq!(payload.account_id, account_id);

    let agent = store.get_agent(outcome.agent_id).await.unwrap().unwrap();
    assert_eq!(agent.status, "active");
}

#[tokio::test]
async fn reused_pairing_token_is_rejected() {
    let store = test_store().await;
    let signer = Keypair::generate();

    let account_id = store.create_account(&unique_email()).await.unwrap();
    let token = store
        .issue_pairing_token(account_id, Duration::minutes(10))
        .await
        .unwrap();

    // First use succeeds.
    store
        .register_agent(&signer, &token.plaintext, Keypair::generate().public_key_bytes())
        .await
        .expect("first registration should succeed");

    // Second use of the same (now-consumed) token must be rejected.
    let result = store
        .register_agent(&signer, &token.plaintext, Keypair::generate().public_key_bytes())
        .await;

    assert!(matches!(result, Err(RegisterError::TokenAlreadyUsed)));
}

#[tokio::test]
async fn unknown_pairing_token_is_rejected() {
    let store = test_store().await;
    let signer = Keypair::generate();

    let result = store
        .register_agent(&signer, "hbp_not-a-real-token", Keypair::generate().public_key_bytes())
        .await;

    assert!(matches!(result, Err(RegisterError::InvalidToken)));
}

#[tokio::test]
async fn concurrent_registration_with_same_token_only_succeeds_once() {
    let store = test_store().await;
    let signer = Keypair::generate();

    let account_id = store.create_account(&unique_email()).await.unwrap();
    let token = store
        .issue_pairing_token(account_id, Duration::minutes(10))
        .await
        .unwrap();

    let (a, b) = tokio::join!(
        store.register_agent(&signer, &token.plaintext, Keypair::generate().public_key_bytes()),
        store.register_agent(&signer, &token.plaintext, Keypair::generate().public_key_bytes()),
    );

    let successes = [&a, &b].iter().filter(|r| r.is_ok()).count();
    assert_eq!(successes, 1, "exactly one concurrent registration should win the race");
}

#[tokio::test]
async fn credential_with_mismatched_fingerprint_is_rejected() {
    let store = test_store().await;
    let signer = Keypair::generate();

    let account_id = store.create_account(&unique_email()).await.unwrap();
    let token = store
        .issue_pairing_token(account_id, Duration::minutes(10))
        .await
        .unwrap();

    let outcome = store
        .register_agent(&signer, &token.plaintext, Keypair::generate().public_key_bytes())
        .await
        .unwrap();

    // Forge a credential for the same agent_id/account_id but a different
    // (attacker-controlled) key's fingerprint — simulates a stolen/replayed
    // credential presented alongside the wrong private key.
    let forged_payload = harbory_common::credential::CredentialPayload {
        agent_id: outcome.agent_id,
        account_id,
        public_key_fingerprint: harbory_common::fingerprint::fingerprint(
            &Keypair::generate().public_key_bytes(),
        ),
        issued_at: chrono::Utc::now().timestamp(),
    };
    let forged_credential = harbory_common::credential::sign_credential(&forged_payload, &signer);

    let result = store
        .verify_agent_credential(&signer.public_key_bytes(), &forged_credential)
        .await;

    assert!(matches!(result, Err(VerifyCredentialError::FingerprintMismatch)));
}

#[tokio::test]
async fn genuine_credential_verifies_successfully() {
    let store = test_store().await;
    let signer = Keypair::generate();

    let account_id = store.create_account(&unique_email()).await.unwrap();
    let token = store
        .issue_pairing_token(account_id, Duration::minutes(10))
        .await
        .unwrap();

    let outcome = store
        .register_agent(&signer, &token.plaintext, Keypair::generate().public_key_bytes())
        .await
        .unwrap();

    let agent = store
        .verify_agent_credential(&signer.public_key_bytes(), &outcome.credential)
        .await
        .expect("genuine credential should verify");

    assert_eq!(agent.id, outcome.agent_id);
}

#[tokio::test]
async fn credential_signed_by_wrong_control_plane_key_is_rejected() {
    let store = test_store().await;
    let real_signer = Keypair::generate();
    let impostor_signer = Keypair::generate();

    let account_id = store.create_account(&unique_email()).await.unwrap();
    let token = store
        .issue_pairing_token(account_id, Duration::minutes(10))
        .await
        .unwrap();

    let outcome = store
        .register_agent(&real_signer, &token.plaintext, Keypair::generate().public_key_bytes())
        .await
        .unwrap();

    // Verifying against the wrong control-plane public key must fail even
    // though the credential itself is well-formed.
    let result = store
        .verify_agent_credential(&impostor_signer.public_key_bytes(), &outcome.credential)
        .await;

    assert!(matches!(result, Err(VerifyCredentialError::Invalid)));
}
