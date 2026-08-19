use std::path::PathBuf;

use harbory_common::keypair::Keypair;
use harbory_protocol::v1::{pairing_service_client::PairingServiceClient, RegisterRequest};

/// Phase 1 scope: generate/load the agent's local identity and run the
/// one-shot pairing handshake. The persistent stream (heartbeats, command
/// execution) is built in later phases.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let pairing_token = std::env::args()
        .nth(1)
        .or_else(|| std::env::var("PAIRING_TOKEN").ok())
        .ok_or_else(|| anyhow::anyhow!("usage: harbory-agent <pairing-token> (or set PAIRING_TOKEN)"))?;

    let control_plane_addr =
        std::env::var("CONTROL_PLANE_ADDR").unwrap_or_else(|_| "http://127.0.0.1:50051".into());
    let key_path = PathBuf::from(std::env::var("AGENT_KEY_PATH").unwrap_or_else(|_| "./agent-key".into()));
    let credential_path = PathBuf::from(
        std::env::var("AGENT_CREDENTIAL_PATH").unwrap_or_else(|_| "./agent-credential".into()),
    );

    // Local keypair is generated before any network call, per the security
    // model: identity exists independent of and prior to control plane trust.
    let identity = Keypair::load_or_generate(&key_path)?;

    let mut client = PairingServiceClient::connect(control_plane_addr).await?;
    let response = client
        .register(RegisterRequest {
            pairing_token,
            public_key: identity.public_key_bytes().to_vec(),
        })
        .await?
        .into_inner();

    std::fs::write(&credential_path, &response.credential)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&credential_path, std::fs::Permissions::from_mode(0o600))?;
    }

    tracing::info!(
        agent_id = %response.agent_id,
        account_id = %response.account_id,
        credential_path = %credential_path.display(),
        "paired with control plane"
    );

    Ok(())
}
