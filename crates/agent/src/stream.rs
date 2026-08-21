use std::time::{Duration, SystemTime, UNIX_EPOCH};

use harbory_common::keypair::Keypair;
use harbory_protocol::v1::{
    agent_stream_service_client::AgentStreamServiceClient, container_command::Action as ContainerAction,
    control_plane_message::Payload as ControlPlanePayload, AgentMessage, ChallengeResponse, ContainerStateReport,
    Heartbeat, Hello,
};
use tokio::sync::mpsc;
use tokio_stream::{wrappers::ReceiverStream, StreamExt};

use crate::container::ContainerManager;

async fn send_state_report(tx: &mpsc::Sender<AgentMessage>, containers: &ContainerManager) -> anyhow::Result<()> {
    let containers = containers.list_state().await?;
    tx.send(AgentMessage {
        payload: Some(harbory_protocol::v1::agent_message::Payload::StateReport(ContainerStateReport { containers })),
    })
    .await?;
    Ok(())
}

/// Connect, run the handshake, then loop sending heartbeats/state reports
/// and executing whatever commands arrive, until the stream breaks.
/// Returns on any disconnect (including a clean server-side close) so the
/// caller can apply reconnect backoff — this function itself has no retry
/// logic, by design, to keep it testable/composable.
pub async fn run_stream(
    control_plane_addr: &str,
    identity: &Keypair,
    credential: &[u8],
    containers: &ContainerManager,
) -> anyhow::Result<()> {
    let mut client = AgentStreamServiceClient::connect(control_plane_addr.to_string()).await?;

    let (tx, rx) = mpsc::channel::<AgentMessage>(16);
    let outbound = ReceiverStream::new(rx);
    let response = client.stream(outbound).await?;
    let mut inbound = response.into_inner();

    tx.send(AgentMessage {
        payload: Some(harbory_protocol::v1::agent_message::Payload::Hello(Hello {
            credential: credential.to_vec(),
        })),
    })
    .await?;

    let challenge = inbound
        .next()
        .await
        .ok_or_else(|| anyhow::anyhow!("stream closed before challenge"))??;
    let nonce = match challenge.payload {
        Some(ControlPlanePayload::Challenge(c)) => c.nonce,
        other => anyhow::bail!("expected Challenge, got {other:?}"),
    };

    let signature = identity.sign(&nonce);
    tx.send(AgentMessage {
        payload: Some(harbory_protocol::v1::agent_message::Payload::ChallengeResponse(
            ChallengeResponse { signature: signature.to_vec() },
        )),
    })
    .await?;

    let welcome = inbound
        .next()
        .await
        .ok_or_else(|| anyhow::anyhow!("stream closed before welcome"))??;
    let (heartbeat_interval_seconds, agent_id) = match welcome.payload {
        Some(ControlPlanePayload::Welcome(w)) => (w.heartbeat_interval_seconds.max(1), w.agent_id),
        other => anyhow::bail!("expected Welcome, got {other:?}"),
    };

    tracing::info!(agent_id, heartbeat_interval_seconds, "stream authenticated, sending heartbeats");

    // Report current state right away so any desired-state changes made
    // while this agent was disconnected converge as soon as possible,
    // rather than waiting for the first periodic tick.
    if let Err(err) = send_state_report(&tx, containers).await {
        tracing::warn!(%err, "failed to send initial container state report");
    }

    let mut ticker = tokio::time::interval(Duration::from_secs(heartbeat_interval_seconds as u64));
    ticker.tick().await; // first tick fires immediately; consume it so the loop below starts after one interval

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64;
                let hb = AgentMessage {
                    payload: Some(harbory_protocol::v1::agent_message::Payload::Heartbeat(Heartbeat { timestamp: now })),
                };
                if tx.send(hb).await.is_err() {
                    anyhow::bail!("outbound channel closed");
                }
                if let Err(err) = send_state_report(&tx, containers).await {
                    tracing::warn!(%err, "failed to send periodic container state report");
                }
            }
            msg = inbound.next() => {
                match msg {
                    Some(Ok(control_plane_msg)) => {
                        if let Some(ControlPlanePayload::Command(cmd)) = control_plane_msg.payload {
                            // Report immediately only on success, for fast
                            // convergence visibility. On failure, don't —
                            // the control plane will just re-send the same
                            // command on the very next report, and with no
                            // delay between "report" and "retry" that's an
                            // unbounded-rate retry loop against Docker for
                            // a persistently broken spec (found this the
                            // hard way against a real daemon). Skipping the
                            // immediate report here means a failure is only
                            // retried at the periodic cadence instead.
                            if execute_command(containers, cmd).await {
                                if let Err(err) = send_state_report(&tx, containers).await {
                                    tracing::warn!(%err, "failed to send post-command container state report");
                                }
                            }
                        }
                        // HeartbeatAck and anything else: nothing to do.
                    }
                    Some(Err(err)) => anyhow::bail!("stream error: {err}"),
                    None => anyhow::bail!("stream closed by control plane"),
                }
            }
        }
    }
}

/// Returns whether the command succeeded — see the caller for why that
/// controls whether a state report is sent immediately afterward.
async fn execute_command(containers: &ContainerManager, cmd: harbory_protocol::v1::ContainerCommand) -> bool {
    match cmd.action {
        Some(ContainerAction::Deploy(spec)) => {
            tracing::info!(name = %spec.name, image = %spec.image, "deploying container");
            match containers.deploy(&spec).await {
                Ok(()) => true,
                Err(err) => {
                    tracing::warn!(name = %spec.name, %err, "failed to deploy container");
                    false
                }
            }
        }
        Some(ContainerAction::Stop(name)) => {
            tracing::info!(%name, "stopping container");
            match containers.stop(&name).await {
                Ok(()) => true,
                Err(err) => {
                    tracing::warn!(%name, %err, "failed to stop container");
                    false
                }
            }
        }
        Some(ContainerAction::Remove(name)) => {
            tracing::info!(%name, "removing container");
            containers.remove(&name).await;
            true
        }
        None => {
            tracing::debug!("received ContainerCommand with no action set");
            false
        }
    }
}
