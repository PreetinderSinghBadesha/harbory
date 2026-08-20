mod backoff;
mod stream;

use std::path::PathBuf;

use harbory_common::keypair::Keypair;
use harbory_protocol::v1::{pairing_service_client::PairingServiceClient, RegisterRequest};

use backoff::Backoff;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let control_plane_addr =
        std::env::var("CONTROL_PLANE_ADDR").unwrap_or_else(|_| "http://127.0.0.1:50051".into());
    let key_path = PathBuf::from(std::env::var("AGENT_KEY_PATH").unwrap_or_else(|_| "./agent-key".into()));
    let credential_path = PathBuf::from(
        std::env::var("AGENT_CREDENTIAL_PATH").unwrap_or_else(|_| "./agent-credential".into()),
    );

    // Local keypair is generated before any network call, per the security
    // model: identity exists independent of and prior to control plane trust.
    let identity = Keypair::load_or_generate(&key_path)?;

    let credential = if credential_path.exists() {
        tracing::info!(path = %credential_path.display(), "using stored credential, skipping pairing");
        std::fs::read(&credential_path)?
    } else {
        let pairing_token = std::env::args()
            .nth(1)
            .or_else(|| std::env::var("PAIRING_TOKEN").ok())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "no stored credential at {} and no pairing token given \
                     (usage: harbory-agent <pairing-token>, or set PAIRING_TOKEN)",
                    credential_path.display()
                )
            })?;
        pair(&control_plane_addr, &identity, &pairing_token, &credential_path).await?
    };

    // Transient disconnects (network blips, control-plane restarts) don't
    // require re-pairing — this loop just keeps reusing the stored
    // credential and identity, per the security model.
    let mut backoff = Backoff::default();
    loop {
        match stream::run_stream(&control_plane_addr, &identity, &credential).await {
            Ok(()) => backoff.reset(),
            Err(err) => {
                let delay = backoff.next_delay();
                tracing::warn!(error = %err, delay = ?delay, "stream disconnected, retrying");
                tokio::time::sleep(delay).await;
                continue;
            }
        }
    }
}

async fn pair(
    control_plane_addr: &str,
    identity: &Keypair,
    pairing_token: &str,
    credential_path: &PathBuf,
) -> anyhow::Result<Vec<u8>> {
    let mut client = PairingServiceClient::connect(control_plane_addr.to_string()).await?;
    let response = client
        .register(RegisterRequest {
            pairing_token: pairing_token.to_string(),
            public_key: identity.public_key_bytes().to_vec(),
        })
        .await?
        .into_inner();

    std::fs::write(credential_path, &response.credential)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(credential_path, std::fs::Permissions::from_mode(0o600))?;
    }

    tracing::info!(
        agent_id = %response.agent_id,
        account_id = %response.account_id,
        credential_path = %credential_path.display(),
        "paired with control plane"
    );

    Ok(response.credential)
}
