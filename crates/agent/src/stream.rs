use std::time::{Duration, SystemTime, UNIX_EPOCH};

use harbory_common::keypair::Keypair;
use harbory_protocol::v1::{
    agent_stream_service_client::AgentStreamServiceClient, control_plane_message::Payload as ControlPlanePayload,
    AgentMessage, ChallengeResponse, Heartbeat, Hello,
};
use tokio::sync::mpsc;
use tokio_stream::{wrappers::ReceiverStream, StreamExt};

/// Connect, run the handshake, then loop sending heartbeats until the
/// stream breaks. Returns on any disconnect (including a clean server-side
/// close) so the caller can apply reconnect backoff — this function itself
/// has no retry logic, by design, to keep it testable/composable.
pub async fn run_stream(
    control_plane_addr: &str,
    identity: &Keypair,
    credential: &[u8],
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

    let mut ticker = tokio::time::interval(Duration::from_secs(heartbeat_interval_seconds as u64));
    ticker.tick().await; // first tick fires immediately; consume it so heartbeats start after one interval

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
            }
            msg = inbound.next() => {
                match msg {
                    Some(Ok(_ack)) => {}
                    Some(Err(err)) => anyhow::bail!("stream error: {err}"),
                    None => anyhow::bail!("stream closed by control plane"),
                }
            }
        }
    }
}
