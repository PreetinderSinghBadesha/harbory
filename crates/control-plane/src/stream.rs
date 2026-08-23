use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use harbory_common::keypair::{verify, Keypair};
use harbory_protocol::{
    proxy_hash,
    v1::{
        agent_message::Payload as AgentPayload, agent_stream_service_server::AgentStreamService,
        container_command::Action as ContainerAction, control_plane_message::Payload as ControlPlanePayload,
        AgentMessage, Challenge, ContainerCommand, ContainerStatus, ControlPlaneMessage, GitSource as ProtoGitSource,
        Heartbeat as HeartbeatMsg, HeartbeatAck, LogsResponse, PortMapping as ProtoPortMapping, ProxyConfig, Welcome,
    },
};
use rand::RngCore;
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio_stream::{wrappers::ReceiverStream, Stream, StreamExt};
use tonic::{Request, Response, Status, Streaming};
use uuid::Uuid;

use crate::metrics::ConnectedAgentGuard;
use crate::reconcile::{self, Action, ObservedContainer, ObservedStatus};
use crate::store::Store;

const CHALLENGE_RESPONSE_TIMEOUT: Duration = Duration::from_secs(10);

pub struct AgentStreamServiceImpl {
    pub store: Store,
    pub signer: Keypair,
    pub heartbeat_interval_seconds: u32,
    pub missed_heartbeat_threshold: u32,
    pub registry: ConnectionRegistry,
}

/// Tracks all currently-connected agents so the HTTP layer can forward
/// one-off requests (e.g. log fetches) into the live stream without
/// needing a separate connection or a new gRPC RPC.
///
/// Structure per agent:
///   outbound   — send messages down the stream to the agent
///   pending_logs — map of request_id → oneshot::Sender waiting for
///                  the agent's LogsResponse
#[derive(Clone, Default)]
pub struct ConnectionRegistry {
    inner: Arc<Mutex<HashMap<Uuid, ConnectedAgent>>>,
}

struct ConnectedAgent {
    outbound: mpsc::Sender<Result<ControlPlaneMessage, Status>>,
    pending_logs: HashMap<String, oneshot::Sender<LogsResponse>>,
}

impl ConnectionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    async fn register(&self, agent_id: Uuid, outbound: mpsc::Sender<Result<ControlPlaneMessage, Status>>) {
        self.inner.lock().await.insert(agent_id, ConnectedAgent { outbound, pending_logs: HashMap::new() });
    }

    async fn deregister(&self, agent_id: Uuid) {
        self.inner.lock().await.remove(&agent_id);
    }

    /// Send a `LogsRequest` to a connected agent and return a `Receiver`
    /// that will be resolved when the agent sends its `LogsResponse`.
    /// Returns `None` if the agent has no live connection.
    pub async fn request_logs(
        &self,
        agent_id: Uuid,
        request_id: String,
        container_name: String,
        tail_lines: u32,
    ) -> Option<oneshot::Receiver<LogsResponse>> {
        let mut guard = self.inner.lock().await;
        let conn = guard.get_mut(&agent_id)?;
        let (tx, rx) = oneshot::channel();
        conn.pending_logs.insert(request_id.clone(), tx);
        let msg = ControlPlaneMessage {
            payload: Some(ControlPlanePayload::LogsRequest(harbory_protocol::v1::LogsRequest {
                request_id: request_id.clone(),
                container_name,
                tail_lines,
            })),
        };
        if conn.outbound.send(Ok(msg)).await.is_err() {
            // Stream is dead — remove the pending entry we just inserted.
            conn.pending_logs.remove(&request_id);
            return None;
        }
        Some(rx)
    }

    /// Resolve a pending log request. Called by `drive_connection` when
    /// the agent sends a `LogsResponse`.
    async fn resolve_logs(&self, agent_id: Uuid, response: LogsResponse) {
        let mut guard = self.inner.lock().await;
        if let Some(conn) = guard.get_mut(&agent_id) {
            if let Some(tx) = conn.pending_logs.remove(&response.request_id) {
                let _ = tx.send(response);
            }
        }
    }
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
    let error = if state.error.is_empty() { None } else { Some(state.error) };
    ObservedContainer { name: state.name, image: state.image, status, error }
}

/// Embeds the account's GitHub credential into the repo URL for this one
/// wire message only — `desired_containers` (and the `GitSource` passed
/// in here) always holds the plain URL; nothing credential-bearing is
/// ever persisted. Falls back to a plain (unauthenticated) clone URL if
/// the account never connected GitHub, which works fine for a public
/// repo and fails with a clear error from Docker for a private one.
async fn resolve_git_source(store: &Store, agent_id: uuid::Uuid, source: reconcile::GitSource) -> ProtoGitSource {
    let account_id = match store.get_agent(agent_id).await {
        Ok(Some(agent)) => Some(agent.account_id),
        Ok(None) => None,
        Err(err) => {
            tracing::warn!(%agent_id, ?err, "failed to look up agent's account for github credential embedding");
            None
        }
    };
    let connection = match account_id {
        Some(id) => store.get_github_connection(id).await.ok().flatten(),
        None => None,
    };

    let repo_url = match connection {
        Some(conn) => embed_credential(&source.repo_url, &conn.access_token),
        None => source.repo_url,
    };
    ProtoGitSource { repo_url, git_ref: source.git_ref, dockerfile_path: source.dockerfile_path }
}

fn embed_credential(repo_url: &str, token: &str) -> String {
    match repo_url.strip_prefix("https://") {
        Some(rest) => format!("https://x-access-token:{token}@{rest}"),
        // Not an https URL this knows how to credential — leave it as-is
        // rather than guessing at a format.
        None => repo_url.to_string(),
    }
}

async fn action_to_command(store: &Store, agent_id: uuid::Uuid, action: Action) -> ContainerCommand {
    match action {
        Action::Deploy(d) => {
            let git_source = match d.git_source {
                Some(source) => Some(resolve_git_source(store, agent_id, source).await),
                None => None,
            };
            ContainerCommand {
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
                    git_source,
                })),
            }
        }
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

    if let Err(err) = store.cleanup_converged_absent_containers(agent_id).await {
        tracing::warn!(%agent_id, ?err, "failed to cleanup converged absent containers");
    }

    let desired = match store.get_desired_containers(agent_id).await {
        Ok(desired) => desired,
        Err(err) => {
            tracing::warn!(%agent_id, ?err, "failed to load desired container state");
            return true;
        }
    };

    for action in reconcile::diff(&desired, &observed) {
        let action_label = match &action {
            Action::Deploy(_) => "deploy",
            Action::Remove(_) => "remove",
        };
        let command = action_to_command(store, agent_id, action).await;
        if tx.send(frame(ControlPlanePayload::Command(command))).await.is_err() {
            return false;
        }
        metrics::counter!("harbory_container_commands_dispatched_total", "action" => action_label).increment(1);
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

    let sent = tx.send(frame(ControlPlanePayload::ProxyConfig(ProxyConfig { routes: desired }))).await.is_ok();
    if sent {
        metrics::counter!("harbory_proxy_configs_dispatched_total").increment(1);
    }
    sent
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
    registry: ConnectionRegistry,
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
            metrics::counter!("harbory_agent_connections_total", "outcome" => "invalid_credential").increment(1);
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
        metrics::counter!("harbory_agent_connections_total", "outcome" => "invalid_challenge_signature").increment(1);
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
    metrics::counter!("harbory_agent_connections_total", "outcome" => "success").increment(1);
    // Decrements the gauge on every exit from this function — RAII so the
    // several `break`s in the loop below don't each need their own
    // decrement call.
    let _connected_guard = ConnectedAgentGuard::new();

    let agent_id = agent.id;

    // Register this connection so the HTTP layer can send log requests.
    registry.register(agent_id, tx.clone()).await;

    // Step 4: heartbeats and container state reports, for as long as the
    // connection stays open. Commands are dispatched only in response to
    // a state report (reconcile_and_dispatch), not pushed the instant
    // desired state changes elsewhere — see docs/reconciliation.md for why
    // that's an acceptable simplification rather than an oversight.
    loop {
        match inbound.next().await {
            Some(Ok(AgentMessage { payload: Some(AgentPayload::Heartbeat(HeartbeatMsg { .. })) })) => {
                metrics::counter!("harbory_heartbeats_received_total").increment(1);
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
            Some(Ok(AgentMessage { payload: Some(AgentPayload::LogsResponse(resp)) })) => {
                registry.resolve_logs(agent_id, resp).await;
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

    registry.deregister(agent_id).await;
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
            self.registry.clone(),
        ));

        Ok(Response::new(Box::pin(ReceiverStream::new(rx)) as Self::StreamStream))
    }
}
