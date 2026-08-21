//! Integration tests for desired/observed container storage and for
//! report-triggered reconciliation over the real stream. Real Postgres,
//! real TCP — see tests/registration.rs for the DB connection convention.
//! No real Docker daemon involved: state reports here are hand-crafted
//! protobuf messages, not actual container state.

use std::net::SocketAddr;
use std::time::Duration;

use chrono::Duration as ChronoDuration;
use harbory_common::keypair::Keypair;
use harbory_control_plane::{
    reconcile::{DesiredContainer, DesiredStatus, PortMapping},
    store::Store,
    stream::AgentStreamServiceImpl,
};
use harbory_protocol::v1::{
    agent_stream_service_client::AgentStreamServiceClient, agent_stream_service_server::AgentStreamServiceServer,
    container_command::Action as ContainerAction, control_plane_message::Payload as ControlPlanePayload,
    AgentMessage, ChallengeResponse, ContainerState, ContainerStateReport, ContainerStatus, Hello,
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

/// Runs the full connect handshake and returns the still-open sender/
/// receiver halves so the test can send a state report and read back
/// whatever command(s) the reconciler decides to send.
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

async fn send_state_report(tx: &mpsc::Sender<AgentMessage>, containers: Vec<ContainerState>) {
    tx.send(AgentMessage {
        payload: Some(harbory_protocol::v1::agent_message::Payload::StateReport(ContainerStateReport {
            containers,
        })),
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn store_roundtrips_desired_and_observed_containers() {
    let store = test_store().await;
    let account_id = store.create_account(&unique_email()).await.unwrap();
    let signer = Keypair::generate();
    let token = store.issue_pairing_token(account_id, ChronoDuration::minutes(10)).await.unwrap();
    let agent_id = store
        .register_agent(&signer, &token.plaintext, Keypair::generate().public_key_bytes())
        .await
        .unwrap()
        .agent_id;

    let desired = DesiredContainer {
        name: "web".into(),
        image: "nginx:alpine".into(),
        env: vec!["FOO=bar".into()],
        ports: vec![PortMapping { host_port: 8080, container_port: 80 }],
        command: vec![],
        status: DesiredStatus::Running,
    };
    store.upsert_desired_container(agent_id, &desired).await.unwrap();

    let fetched = store.get_desired_containers(agent_id).await.unwrap();
    assert_eq!(fetched, vec![desired]);

    // Absent on an unknown name is a no-op, reported as such.
    assert!(!store.set_desired_absent(agent_id, "does-not-exist").await.unwrap());
    // Absent on the one we just declared flips its status.
    assert!(store.set_desired_absent(agent_id, "web").await.unwrap());
    let fetched = store.get_desired_containers(agent_id).await.unwrap();
    assert_eq!(fetched[0].status, DesiredStatus::Absent);
}

#[tokio::test]
async fn missing_desired_container_triggers_deploy_command() {
    let store = test_store().await;
    let signer = Keypair::generate();
    let (agent_id, identity, credential) = register_test_agent(&store, &signer).await;

    store
        .upsert_desired_container(
            agent_id,
            &DesiredContainer {
                name: "web".into(),
                image: "nginx:alpine".into(),
                env: vec![],
                ports: vec![],
                command: vec![],
                status: DesiredStatus::Running,
            },
        )
        .await
        .unwrap();

    let addr = spawn_server(store, signer).await;
    let (tx, mut inbound) = connect_and_authenticate(addr, &identity, credential).await;

    // Agent reports nothing running yet.
    send_state_report(&tx, vec![]).await;

    let msg = inbound.next().await.unwrap().unwrap();
    match msg.payload {
        Some(ControlPlanePayload::Command(cmd)) => match cmd.action {
            Some(ContainerAction::Deploy(spec)) => {
                assert_eq!(spec.name, "web");
                assert_eq!(spec.image, "nginx:alpine");
            }
            other => panic!("expected Deploy action, got {other:?}"),
        },
        other => panic!("expected Command, got {other:?}"),
    }
}

#[tokio::test]
async fn converged_state_produces_no_command() {
    let store = test_store().await;
    let signer = Keypair::generate();
    let (agent_id, identity, credential) = register_test_agent(&store, &signer).await;

    store
        .upsert_desired_container(
            agent_id,
            &DesiredContainer {
                name: "web".into(),
                image: "nginx:alpine".into(),
                env: vec![],
                ports: vec![],
                command: vec![],
                status: DesiredStatus::Running,
            },
        )
        .await
        .unwrap();

    let addr = spawn_server(store, signer).await;
    let (tx, mut inbound) = connect_and_authenticate(addr, &identity, credential).await;

    send_state_report(
        &tx,
        vec![ContainerState {
            name: "web".into(),
            image: "nginx:alpine".into(),
            status: ContainerStatus::Running as i32,
            error: String::new(),
        }],
    )
    .await;

    let result = tokio::time::timeout(Duration::from_millis(300), inbound.next()).await;
    assert!(result.is_err(), "no command should be sent when already converged, got {result:?}");
}

#[tokio::test]
async fn desired_absent_but_observed_running_triggers_remove_command() {
    let store = test_store().await;
    let signer = Keypair::generate();
    let (agent_id, identity, credential) = register_test_agent(&store, &signer).await;

    store
        .upsert_desired_container(
            agent_id,
            &DesiredContainer {
                name: "web".into(),
                image: "nginx:alpine".into(),
                env: vec![],
                ports: vec![],
                command: vec![],
                status: DesiredStatus::Running,
            },
        )
        .await
        .unwrap();
    assert!(store.set_desired_absent(agent_id, "web").await.unwrap());

    let addr = spawn_server(store, signer).await;
    let (tx, mut inbound) = connect_and_authenticate(addr, &identity, credential).await;

    send_state_report(
        &tx,
        vec![ContainerState {
            name: "web".into(),
            image: "nginx:alpine".into(),
            status: ContainerStatus::Running as i32,
            error: String::new(),
        }],
    )
    .await;

    let msg = inbound.next().await.unwrap().unwrap();
    match msg.payload {
        Some(ControlPlanePayload::Command(cmd)) => {
            assert_eq!(cmd.action, Some(ContainerAction::Remove("web".into())));
        }
        other => panic!("expected Command, got {other:?}"),
    }
}
