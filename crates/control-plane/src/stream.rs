use std::pin::Pin;
use std::time::Duration;

use harbory_common::keypair::{verify, Keypair};
use harbory_protocol::{
    proxy_hash,
    v1::{
        agent_message::Payload as AgentPayload, agent_stream_service_server::AgentStreamService,
        container_command::Action as ContainerAction, control_plane_message::Payload as ControlPlanePayload,
        AgentMessage, Challenge, ContainerCommand, ContainerStatus, ControlPlaneMessage, Heartbeat as HeartbeatMsg,
        HeartbeatAck, PortMapping as ProtoPortMapping, ProxyConfig, Welcome,
    },
};
use rand::RngCore;
use tokio::sync::mpsc;
use tokio_stream::{wrappers::ReceiverStream, Stream, StreamExt};
use tonic::{Request, Response, Status, Streaming};

use crate::reconcile::{self, Action, ObservedContainer, ObservedStatus};
use crate::store::Store;

const CHALLENGE_RESPONSE_TIMEOUT: Duration = Duration::from_secs(10);

pub struct AgentStreamServiceImpl {
    pub store: Store,
    pub signer: Keypair,
    pub heartbeat_interval_seconds: u32,
    pub missed_heartbeat_threshold: u32,
}

type AgentStream = Pin<Box<dyn Stream<Item = Result<ControlPlaneMessage, Status>> + Send>>;
type Outbound = mpsc::Sender<Result<ControlPlaneMessage, Status>>;

async fn read_next(inbound: &mut Streaming<AgentMessage>) -> Result<AgentMessage, Status> {
    match inbound.next().await {
        Some(Ok(msg)) => Ok(msg),
        Some(Err(status)) => Err(status),
        None => Err(Status::invalid_argument("stream ended before handshake completed")),
    }
}

fn frame(payload: ControlPlanePayload) -> Result<ControlPlaneMessage, Status> {
    Ok(ControlPlaneMessage { payload: Some(payload) })
}

fn observed_from_proto(state: harbory_protocol::v1::ContainerState) -> ObservedContainer {
    let status = match ContainerStatus::try_from(state.status).unwrap_or(ContainerStatus::Unspecified) {
        ContainerStatus::Running => ObservedStatus::Running,
        ContainerStatus::Stopped => ObservedStatus::Stopped,
        ContainerStatus::Removed => ObservedStatus::Removed,
        ContainerStatus::Error | ContainerStatus::Unspecified => ObservedStatus::Error,
    };
    ObservedContainer { name: state.name, image: state.image, status }
}

fn action_to_command(action: Action) -> ContainerCommand {
    match action {
        Action::Deploy(d) => ContainerCommand {
            action: Some(ContainerAction::Deploy(harbory_protocol::v1::ContainerSpec {
                name: d.name,
                image: d.image,
                env: d.env,
                ports: d
                    .ports
                    .into_iter()
                    .map(|p| ProtoPortMapping { host_port: p.host_port as u32, container_port: p.container_port as u32 })
                    .collect(),
                command: d.command,
            })),
        },
        Action::Remove(name) => ContainerCommand { action: Some(ContainerAction::Remove(name)) },
    }
}

/// Persists a freshly-reported observed snapshot, diffs it against desired
/// state, and sends back whatever commands are needed to converge. Returns
/// `false` if the connection died mid-send (caller should stop looping).
async fn reconcile_and_dispatch(
    store: &Store,
    agent_id: uuid::Uuid,
    observed: Vec<ObservedContainer>,
    tx: &Outbound,
) -> bool {
    if let Err(err) = store.replace_observed_containers(agent_id, &observed).await {
        tracing::warn!(%agent_id, ?err, "failed to persist observed container state");
        return true;
    }

    let desired = match store.get_desired_containers(agent_id).await {
        Ok(desired) => desired,
        Err(err) => {
            tracing::warn!(%agent_id, ?err, "failed to load desired container state");
            return true;
        }
    };

    for action in reconcile::diff(&desired, &observed) {
        let command = action_to_command(action);
        if tx.send(frame(ControlPlanePayload::Command(command))).await.is_err() {
            return false;
        }
    }
    true
}

/// Persists what the agent reports it has applied, and — if that doesn't
/// match the hash of current desired state — sends the full desired route
/// set back down. Same "converge on report, not on instant push" pattern
/// as containers; see docs/proxy-management.md. Returns `false` if the
/// connection died mid-send.
async fn reconcile_proxy_and_dispatch(
    store: &Store,
    agent_id: uuid::Uuid,
    applied_hash: Vec<u8>,
    error: Option<String>,
    tx: &Outbound,
) -> bool {
    if let Err(err) = store.record_proxy_state(agent_id, &applied_hash, error.as_deref()).await {
        tracing::warn!(%agent_id, ?err, "failed to persist proxy state");
        return true;
    }

    let desired = match store.get_desired_proxy_routes(agent_id).await {
        Ok(desired) => desired,
        Err(err) => {
            tracing::warn!(%agent_id, ?err, "failed to load desired proxy routes");
            return true;
        }
    };

    if proxy_hash::hash_routes(&desired).as_slice() == applied_hash.as_slice() {
        return true; // already converged
    }

    tx.send(frame(ControlPlanePayload::ProxyConfig(ProxyConfig { routes: desired }))).await.is_ok()
}

/// Everything that happens on one connection, after the response stream
/// has already been handed back to the client. Response headers (and so
/// the client's read side) don't actually flush until the handler returns
/// `Response::new(...)` — trying to send/receive handshake messages before
/// that point deadlocks both sides waiting on each other. So `stream()`
/// below returns immediately and all of this runs in a spawned task
/// instead, reporting failures as an `Err(Status)` item on `tx` rather
/// than as the RPC's own return value.
async fn drive_connection(
    store: Store,
    signer: Keypair,
    heartbeat_interval_seconds: u32,
    missed_heartbeat_threshold: u32,
    mut inbound: Streaming<AgentMessage>,
    tx: Outbound,
) {
    macro_rules! fail {
        ($status:expr) => {{
            let _ = tx.send(Err($status)).await;
            return;
        }};
    }

    // Step 1: Hello — presents the credential issued at pairing time.
    let hello = match read_next(&mut inbound).await {
        Ok(msg) => msg,
        Err(status) => fail!(status),
    };
    let credential = match hello.payload {
        Some(AgentPayload::Hello(h)) => h.credential,
        _ => fail!(Status::invalid_argument("expected Hello as the first message")),
    };

    let agent = match store.verify_agent_credential(&signer.public_key_bytes(), &credential).await {
        Ok(agent) => agent,
        Err(err) => {
            tracing::warn!(?err, "stream connect rejected: invalid credential");
            fail!(Status::unauthenticated("invalid credential"));
        }
    };

    let public_key: [u8; 32] = match agent.public_key.as_slice().try_into() {
        Ok(pk) => pk,
        Err(_) => fail!(Status::internal("corrupt stored public key")),
    };

    // Step 2/3: Challenge / ChallengeResponse — proves the caller holds
    // the private key matching the credential's fingerprint, which the
    // credential alone (a bearer token) does not. See docs/security.md.
    let mut nonce = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut nonce);

    if tx.send(frame(ControlPlanePayload::Challenge(Challenge { nonce: nonce.to_vec() }))).await.is_err() {
        return;
    }

    let response = match tokio::time::timeout(CHALLENGE_RESPONSE_TIMEOUT, read_next(&mut inbound)).await {
        Ok(Ok(msg)) => msg,
        Ok(Err(status)) => fail!(status),
        Err(_) => fail!(Status::deadline_exceeded("timed out waiting for challenge response")),
    };

    let signature_bytes = match response.payload {
        Some(AgentPayload::ChallengeResponse(r)) => r.signature,
        _ => fail!(Status::invalid_argument("expected ChallengeResponse")),
    };
    let signature: [u8; 64] = match signature_bytes.as_slice().try_into() {
        Ok(sig) => sig,
        Err(_) => fail!(Status::invalid_argument("signature must be 64 bytes")),
    };

    if !verify(&public_key, &nonce, &signature) {
        tracing::warn!(agent_id = %agent.id, "stream connect rejected: challenge signature invalid");
        fail!(Status::unauthenticated("challenge response signature invalid"));
    }

    if tx
        .send(frame(ControlPlanePayload::Welcome(Welcome {
            agent_id: agent.id.to_string(),
            heartbeat_interval_seconds,
            missed_heartbeat_threshold,
        })))
        .await
        .is_err()
    {
        return;
    }

    tracing::info!(agent_id = %agent.id, "agent stream authenticated");

    // Step 4: heartbeats and container state reports, for as long as the
    // connection stays open. Commands are dispatched only in response to
    // a state report (reconcile_and_dispatch), not pushed the instant
    // desired state changes elsewhere — see docs/reconciliation.md for why
    // that's an acceptable simplification (no shared connection registry
    // needed) rather than an oversight.
    let agent_id = agent.id;
    loop {
        match inbound.next().await {
            Some(Ok(AgentMessage { payload: Some(AgentPayload::Heartbeat(HeartbeatMsg { .. })) })) => {
                if let Err(err) = store.record_heartbeat(agent_id).await {
                    tracing::warn!(%agent_id, ?err, "failed to record heartbeat");
                }
                if tx.send(frame(ControlPlanePayload::HeartbeatAck(HeartbeatAck {}))).await.is_err() {
                    break;
                }
            }
            Some(Ok(AgentMessage { payload: Some(AgentPayload::StateReport(report)) })) => {
                let observed: Vec<ObservedContainer> = report.containers.into_iter().map(observed_from_proto).collect();
                if !reconcile_and_dispatch(&store, agent_id, observed, &tx).await {
                    break;
                }
            }
            Some(Ok(AgentMessage { payload: Some(AgentPayload::ProxyState(state)) })) => {
                let error = if state.error.is_empty() { None } else { Some(state.error) };
                if !reconcile_proxy_and_dispatch(&store, agent_id, state.applied_hash, error, &tx).await {
                    break;
                }
            }
            Some(Ok(_)) => {
                tracing::debug!(%agent_id, "ignoring unexpected message after handshake");
            }
            Some(Err(err)) => {
                tracing::info!(%agent_id, %err, "agent stream closed with error");
                break;
            }
            None => {
                tracing::info!(%agent_id, "agent stream closed");
                break;
            }
        }
    }
}

#[tonic::async_trait]
impl AgentStreamService for AgentStreamServiceImpl {
    type StreamStream = AgentStream;

    async fn stream(
        &self,
        request: Request<Streaming<AgentMessage>>,
    ) -> Result<Response<Self::StreamStream>, Status> {
        let inbound = request.into_inner();
        let (tx, rx) = mpsc::channel::<Result<ControlPlaneMessage, Status>>(16);

        tokio::spawn(drive_connection(
            self.store.clone(),
            self.signer.clone(),
            self.heartbeat_interval_seconds,
            self.missed_heartbeat_threshold,
            inbound,
            tx,
        ));

        Ok(Response::new(Box::pin(ReceiverStream::new(rx)) as Self::StreamStream))
    }
}
