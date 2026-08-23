//! Integration tests for desired proxy route storage and report-triggered
//! reconciliation over the real stream. Real Postgres, real TCP — see
//! tests/registration.rs for the DB connection convention. No real nginx
//! involved: proxy state reports here are hand-crafted protobuf messages.

use std::net::SocketAddr;
use std::time::Duration;

use chrono::Duration as ChronoDuration;
use harbory_common::keypair::Keypair;
use harbory_control_plane::{store::Store, stream::AgentStreamServiceImpl};
use harbory_protocol::{
    proxy_hash,
    v1::{
        agent_stream_service_client::AgentStreamServiceClient, agent_stream_service_server::AgentStreamServiceServer,
        control_plane_message::Payload as ControlPlanePayload, AgentMessage, ChallengeResponse, Hello, ProxyRoute,
        ProxyState,
    },
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

    tokio::time::sleep(Duration::from_millis(50)).await;
    addr
}

async fn register_test_agent(store: &Store, signer: &Keypair) -> (Uuid, Keypair, Vec<u8>) {
    let account_id = store.create_account(&unique_email()).await.unwrap();
    let token = store.issue_pairing_token(account_id, ChronoDuration::minutes(10)).await.unwrap();
    let agent_identity = Keypair::generate();
    let outcome =
        store.register_agent(signer, &token.plaintext, agent_identity.public_key_bytes()).await.unwrap();
    (outcome.agent_id, agent_identity, outcome.credential)
}

async fn connect_and_authenticate(
    addr: SocketAddr,
    identity: &Keypair,
    credential: Vec<u8>,
) -> (mpsc::Sender<AgentMessage>, tonic::Streaming<harbory_protocol::v1::ControlPlaneMessage>) {
    let mut client = AgentStreamServiceClient::connect(format!("http://{addr}")).await.unwrap();
    let (tx, rx) = mpsc::channel::<AgentMessage>(16);
    let response = client.stream(ReceiverStream::new(rx)).await.unwrap();
    let mut inbound = response.into_inner();

    tx.send(AgentMessage { payload: Some(harbory_protocol::v1::agent_message::Payload::Hello(Hello { credential })) })
        .await
        .unwrap();

    let challenge = inbound.next().await.unwrap().unwrap();
    let nonce = match challenge.payload {
        Some(ControlPlanePayload::Challenge(c)) => c.nonce,
        other => panic!("expected Challenge, got {other:?}"),
    };
    let signature = identity.sign(&nonce);
    tx.send(AgentMessage {
        payload: Some(harbory_protocol::v1::agent_message::Payload::ChallengeResponse(ChallengeResponse {
            signature: signature.to_vec(),
        })),
    })
    .await
    .unwrap();

    let welcome = inbound.next().await.unwrap().unwrap();
    assert!(matches!(welcome.payload, Some(ControlPlanePayload::Welcome(_))));

    (tx, inbound)
}

fn sample_route(name: &str) -> ProxyRoute {
    ProxyRoute {
        name: name.into(),
        server_name: "app.example.test".into(),
        listen_port: 80,
        path_prefix: "/".into(),
        upstream_host: "127.0.0.1".into(),
        upstream_port: 8080,
    }
}

async fn send_proxy_state(tx: &mpsc::Sender<AgentMessage>, applied_hash: Vec<u8>, error: &str) {
    tx.send(AgentMessage {
        payload: Some(harbory_protocol::v1::agent_message::Payload::ProxyState(ProxyState {
            applied_hash,
            error: error.to_string(),
        })),
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn store_roundtrips_and_deletes_desired_proxy_routes() {
    let store = test_store().await;
    let account_id = store.create_account(&unique_email()).await.unwrap();
    let signer = Keypair::generate();
    let token = store.issue_pairing_token(account_id, ChronoDuration::minutes(10)).await.unwrap();
    let agent_id = store
        .register_agent(&signer, &token.plaintext, Keypair::generate().public_key_bytes())
        .await
        .unwrap()
        .agent_id;

    let route = sample_route("web");
    store.upsert_desired_proxy_route(agent_id, &route).await.unwrap();

    let fetched = store.get_desired_proxy_routes(agent_id).await.unwrap();
    assert_eq!(fetched, vec![route]);

    assert!(!store.delete_desired_proxy_route(agent_id, "does-not-exist").await.unwrap());
    assert!(store.delete_desired_proxy_route(agent_id, "web").await.unwrap());
    assert!(store.get_desired_proxy_routes(agent_id).await.unwrap().is_empty());
}

#[tokio::test]
async fn mismatched_hash_triggers_proxy_config_command() {
    let store = test_store().await;
    let signer = Keypair::generate();
    let (agent_id, identity, credential) = register_test_agent(&store, &signer).await;

    let route = sample_route("web");
    store.upsert_desired_proxy_route(agent_id, &route).await.unwrap();

    let addr = spawn_server(store, signer).await;
    let (tx, mut inbound) = connect_and_authenticate(addr, &identity, credential).await;

    // Agent reports having applied nothing yet.
    send_proxy_state(&tx, Vec::new(), "").await;

    let msg = inbound.next().await.unwrap().unwrap();
    match msg.payload {
        Some(ControlPlanePayload::ProxyConfig(cfg)) => {
            assert_eq!(cfg.routes, vec![route]);
        }
        other => panic!("expected ProxyConfig, got {other:?}"),
    }
}

#[tokio::test]
async fn matching_hash_produces_no_command() {
    let store = test_store().await;
    let signer = Keypair::generate();
    let (agent_id, identity, credential) = register_test_agent(&store, &signer).await;

    let route = sample_route("web");
    store.upsert_desired_proxy_route(agent_id, &route).await.unwrap();

    let addr = spawn_server(store, signer).await;
    let (tx, mut inbound) = connect_and_authenticate(addr, &identity, credential).await;

    let hash = proxy_hash::hash_routes(&[route]).to_vec();
    send_proxy_state(&tx, hash, "").await;

    let result = tokio::time::timeout(Duration::from_millis(300), inbound.next()).await;
    assert!(result.is_err(), "no command should be sent when already converged, got {result:?}");
}

#[tokio::test]
async fn proxy_state_error_is_persisted_and_visible() {
    let store = test_store().await;
    let signer = Keypair::generate();
    let (agent_id, identity, credential) = register_test_agent(&store, &signer).await;

    let addr = spawn_server(store.clone(), signer).await;
    let (tx, _inbound) = connect_and_authenticate(addr, &identity, credential).await;

    send_proxy_state(&tx, Vec::new(), "nginx config validation failed: test error").await;
    tokio::time::sleep(Duration::from_millis(200)).await;

    let (_, error) = store.get_proxy_state(agent_id).await.unwrap().unwrap();
    assert_eq!(error.as_deref(), Some("nginx config validation failed: test error"));
}
