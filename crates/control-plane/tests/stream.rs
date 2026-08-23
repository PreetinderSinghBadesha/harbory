//! Integration tests for the persistent Connect stream: handshake
//! (credential + challenge/response) and heartbeat-driven online/offline
//! computation. Real Postgres, real TCP, real tonic client — see
//! tests/registration.rs for the DB connection convention.

use std::net::SocketAddr;
use std::time::Duration;

use chrono::Duration as ChronoDuration;
use harbory_common::keypair::Keypair;
use harbory_control_plane::{store::Store, stream::AgentStreamServiceImpl};
use harbory_protocol::v1::{
    agent_stream_service_client::AgentStreamServiceClient, agent_stream_service_server::AgentStreamServiceServer,
    control_plane_message::Payload as ControlPlanePayload, AgentMessage, ChallengeResponse, Heartbeat, Hello,
};
use tokio::sync::mpsc;
use tokio_stream::{wrappers::ReceiverStream, StreamExt};
use uuid::Uuid;

async fn test_store() -> Store {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://harbory:harbory_dev_password@localhost:55433/harbory".into());
    Store::connect(&database_url).await.expect("failed to connect to test database")
}

fn unique_email() -> String {
    format!("{}@example.test", Uuid::new_v4())
}

/// Spawns a real server on an OS-assigned port and returns its address.
/// The task is detached (test process teardown reclaims it) — fine for
/// short-lived integration tests.
async fn spawn_server(store: Store, signer: Keypair) -> SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let incoming = tokio_stream::wrappers::TcpListenerStream::new(listener);

    tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(AgentStreamServiceServer::new(AgentStreamServiceImpl {
                store,
                signer,
                heartbeat_interval_seconds: 1,
                missed_heartbeat_threshold: 3,
                registry: harbory_control_plane::stream::ConnectionRegistry::default(),
            }))
            .serve_with_incoming(incoming)
            .await
            .unwrap();
    });

    // Give the listener a moment to actually start accepting.
    tokio::time::sleep(Duration::from_millis(50)).await;
    addr
}

async fn register_test_agent(store: &Store, signer: &Keypair) -> (Uuid, Uuid, Keypair, Vec<u8>) {
    let account_id = store.create_account(&unique_email()).await.unwrap();
    let token = store.issue_pairing_token(account_id, ChronoDuration::minutes(10)).await.unwrap();
    let agent_identity = Keypair::generate();
    let outcome = store
        .register_agent(signer, &token.plaintext, agent_identity.public_key_bytes())
        .await
        .unwrap();
    (outcome.agent_id, account_id, agent_identity, outcome.credential)
}

#[tokio::test]
async fn handshake_succeeds_and_heartbeat_marks_agent_online() {
    let store = test_store().await;
    let signer = Keypair::generate();
    let (agent_id, account_id, agent_identity, credential) = register_test_agent(&store, &signer).await;

    let addr = spawn_server(store.clone(), signer).await;
    let mut client = AgentStreamServiceClient::connect(format!("http://{addr}")).await.unwrap();

    let (tx, rx) = mpsc::channel::<AgentMessage>(16);
    let response = client.stream(ReceiverStream::new(rx)).await.unwrap();
    let mut inbound = response.into_inner();

    tx.send(AgentMessage {
        payload: Some(harbory_protocol::v1::agent_message::Payload::Hello(Hello { credential })),
    })
    .await
    .unwrap();

    let challenge = inbound.next().await.unwrap().unwrap();
    let nonce = match challenge.payload {
        Some(ControlPlanePayload::Challenge(c)) => c.nonce,
        other => panic!("expected Challenge, got {other:?}"),
    };

    let signature = agent_identity.sign(&nonce);
    tx.send(AgentMessage {
        payload: Some(harbory_protocol::v1::agent_message::Payload::ChallengeResponse(
            ChallengeResponse { signature: signature.to_vec() },
        )),
    })
    .await
    .unwrap();

    let welcome = inbound.next().await.unwrap().unwrap();
    match welcome.payload {
        Some(ControlPlanePayload::Welcome(w)) => assert_eq!(w.agent_id, agent_id.to_string()),
        other => panic!("expected Welcome, got {other:?}"),
    }

    // Not online yet: no heartbeat has been sent.
    let before = store.list_agents_for_account(account_id, 30).await.unwrap();
    let summary = before.iter().find(|a| a.id == agent_id).unwrap();
    assert!(!summary.online, "agent should not be online before any heartbeat");

    tx.send(AgentMessage {
        payload: Some(harbory_protocol::v1::agent_message::Payload::Heartbeat(Heartbeat { timestamp: 0 })),
    })
    .await
    .unwrap();

    let ack = inbound.next().await.unwrap().unwrap();
    assert!(matches!(ack.payload, Some(ControlPlanePayload::HeartbeatAck(_))));

    let after = store.list_agents_for_account(account_id, 30).await.unwrap();
    let summary = after.iter().find(|a| a.id == agent_id).unwrap();
    assert!(summary.online, "agent should be online right after a heartbeat");

    // With a threshold shorter than time actually elapsed, the same
    // heartbeat should no longer count as "recent".
    tokio::time::sleep(Duration::from_millis(50)).await;
    let stale = store.list_agents_for_account(account_id, 0).await.unwrap();
    let summary = stale.iter().find(|a| a.id == agent_id).unwrap();
    assert!(!summary.online, "agent should be offline once the heartbeat ages past the threshold");
}

#[tokio::test]
async fn garbage_credential_is_rejected_before_challenge() {
    let store = test_store().await;
    let signer = Keypair::generate();
    let addr = spawn_server(store, signer).await;
    let mut client = AgentStreamServiceClient::connect(format!("http://{addr}")).await.unwrap();

    let (tx, rx) = mpsc::channel::<AgentMessage>(16);
    let response = client.stream(ReceiverStream::new(rx)).await.unwrap();
    let mut inbound = response.into_inner();

    tx.send(AgentMessage {
        payload: Some(harbory_protocol::v1::agent_message::Payload::Hello(Hello {
            credential: vec![1, 2, 3],
        })),
    })
    .await
    .unwrap();

    let result = inbound.next().await.unwrap();
    assert_eq!(result.unwrap_err().code(), tonic::Code::Unauthenticated);
}

#[tokio::test]
async fn signing_challenge_with_wrong_key_is_rejected() {
    let store = test_store().await;
    let signer = Keypair::generate();
    let (_agent_id, _account_id, _real_identity, credential) = register_test_agent(&store, &signer).await;

    let addr = spawn_server(store, signer).await;
    let mut client = AgentStreamServiceClient::connect(format!("http://{addr}")).await.unwrap();

    let (tx, rx) = mpsc::channel::<AgentMessage>(16);
    let response = client.stream(ReceiverStream::new(rx)).await.unwrap();
    let mut inbound = response.into_inner();

    tx.send(AgentMessage {
        payload: Some(harbory_protocol::v1::agent_message::Payload::Hello(Hello { credential })),
    })
    .await
    .unwrap();

    let challenge = inbound.next().await.unwrap().unwrap();
    let nonce = match challenge.payload {
        Some(ControlPlanePayload::Challenge(c)) => c.nonce,
        other => panic!("expected Challenge, got {other:?}"),
    };

    // Sign with an unrelated keypair — simulates presenting a stolen
    // credential without the matching private key.
    let impostor = Keypair::generate();
    let signature = impostor.sign(&nonce);
    tx.send(AgentMessage {
        payload: Some(harbory_protocol::v1::agent_message::Payload::ChallengeResponse(
            ChallengeResponse { signature: signature.to_vec() },
        )),
    })
    .await
    .unwrap();

    let result = inbound.next().await.unwrap();
    assert_eq!(result.unwrap_err().code(), tonic::Code::Unauthenticated);
}
