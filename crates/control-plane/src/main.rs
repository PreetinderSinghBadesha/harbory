use std::path::PathBuf;

use harbory_common::keypair::Keypair;
use harbory_control_plane::{
    grpc::PairingServiceImpl,
    http::{self, AppState},
    store::Store,
    stream::AgentStreamServiceImpl,
};
use harbory_protocol::v1::{
    agent_stream_service_server::AgentStreamServiceServer, pairing_service_server::PairingServiceServer,
};

// Defaults resolve the "exact heartbeat interval and missed-heartbeat
// threshold" open question from HARBORY_README.md — see
// docs/connection-lifecycle.md for rationale.
const DEFAULT_HEARTBEAT_INTERVAL_SECONDS: u32 = 10;
const DEFAULT_MISSED_HEARTBEAT_THRESHOLD: u32 = 3;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://harbory:harbory_dev_password@localhost:55433/harbory".into());
    let signing_key_path = std::env::var("CONTROL_PLANE_SIGNING_KEY_PATH")
        .unwrap_or_else(|_| "./control-plane-signing-key".into());
    let grpc_addr = std::env::var("GRPC_LISTEN_ADDR").unwrap_or_else(|_| "127.0.0.1:50051".into());
    let http_addr = std::env::var("HTTP_LISTEN_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".into());
    let heartbeat_interval_seconds: u32 = std::env::var("HEARTBEAT_INTERVAL_SECONDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_HEARTBEAT_INTERVAL_SECONDS);
    let missed_heartbeat_threshold: u32 = std::env::var("MISSED_HEARTBEAT_THRESHOLD")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_MISSED_HEARTBEAT_THRESHOLD);
    // Required, no insecure default: every HTTP endpoint now verifies a
    // Supabase-issued JWT against this secret (see docs/dashboard.md for
    // where to find it in the Supabase dashboard). Failing to set it is a
    // startup error, not a silently-open API.
    let jwt_secret = std::env::var("SUPABASE_JWT_SECRET")
        .map_err(|_| anyhow::anyhow!("SUPABASE_JWT_SECRET must be set — see docs/dashboard.md"))?;

    let store = Store::connect(&database_url).await?;
    let signer = Keypair::load_or_generate(&PathBuf::from(signing_key_path))?;
    let metrics_handle = harbory_control_plane::metrics::install();

    let grpc_store = store.clone();
    let grpc_signer = signer.clone();
    let grpc_addr_parsed = grpc_addr.parse()?;
    let grpc_server = async move {
        tracing::info!(addr = %grpc_addr, "starting harbory control plane (gRPC)");
        // NOTE: plaintext h2c for now — TLS is a deliberately deferred
        // follow-up, not an oversight. See docs/security.md.
        tonic::transport::Server::builder()
            .add_service(PairingServiceServer::new(PairingServiceImpl {
                store: grpc_store.clone(),
                signer: grpc_signer.clone(),
            }))
            .add_service(AgentStreamServiceServer::new(AgentStreamServiceImpl {
                store: grpc_store,
                signer: grpc_signer,
                heartbeat_interval_seconds,
                missed_heartbeat_threshold,
            }))
            .serve(grpc_addr_parsed)
            .await
    };

    let online_threshold_seconds =
        (heartbeat_interval_seconds * missed_heartbeat_threshold) as i64;
    let http_state = AppState { store, online_threshold_seconds, jwt_secret, metrics_handle };
    let http_addr_parsed: std::net::SocketAddr = http_addr.parse()?;
    let http_server = async move {
        tracing::info!(addr = %http_addr, "starting harbory control plane (HTTP)");
        let listener = tokio::net::TcpListener::bind(http_addr_parsed).await?;
        axum::serve(listener, http::router(http_state)).await
    };

    tokio::try_join!(
        async { grpc_server.await.map_err(anyhow::Error::from) },
        async { http_server.await.map_err(anyhow::Error::from) },
    )?;

    Ok(())
}
