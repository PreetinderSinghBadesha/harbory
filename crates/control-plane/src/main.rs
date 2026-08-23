use std::path::PathBuf;

use harbory_common::keypair::Keypair;
use harbory_control_plane::{
    grpc::PairingServiceImpl,
    http::{self, AppState},
    jwks::JwkVerifier,
    store::Store,
    stream::{AgentStreamServiceImpl, ConnectionRegistry},
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
    // Every HTTP endpoint verifies a Supabase-issued JWT — but Supabase
    // signs those with *either* a legacy shared HS256 secret *or* (newer
    // projects, by default) an asymmetric ES256 key published via JWKS,
    // and a token's own header says which (see auth.rs). So at least one
    // of these two must be configured, or literally no request could ever
    // authenticate — that combined check is the fail-fast startup error,
    // not silently running with every endpoint open. See docs/dashboard.md.
    let jwt_secret = std::env::var("SUPABASE_JWT_SECRET").ok();
    let supabase_url = std::env::var("SUPABASE_URL").ok();
    if jwt_secret.is_none() && supabase_url.is_none() {
        anyhow::bail!("must set SUPABASE_JWT_SECRET and/or SUPABASE_URL — see docs/dashboard.md");
    }
    // GitHub OAuth App integration — optional, unlike the Supabase check
    // above: a control plane with none of these set still runs fine,
    // just with /github/* routes returning 503 (see github_configured in
    // http.rs) rather than the whole process refusing to start.
    let github_client_id = std::env::var("GITHUB_CLIENT_ID").ok();
    let github_client_secret = std::env::var("GITHUB_CLIENT_SECRET").ok();
    let github_redirect_uri = std::env::var("GITHUB_REDIRECT_URI").ok();
    let frontend_url = std::env::var("FRONTEND_URL").ok();

    let store = Store::connect(&database_url).await?;
    let signer = Keypair::load_or_generate(&PathBuf::from(signing_key_path))?;
    let metrics_handle = harbory_control_plane::metrics::install();
    let jwks = match &supabase_url {
        Some(url) => JwkVerifier::fetch(url).await?,
        None => JwkVerifier::empty(),
    };

    let registry = ConnectionRegistry::new();

    let grpc_store = store.clone();
    let grpc_signer = signer.clone();
    let grpc_registry = registry.clone();
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
                registry: grpc_registry,
            }))
            .serve(grpc_addr_parsed)
            .await
    };

    let online_threshold_seconds =
        (heartbeat_interval_seconds * missed_heartbeat_threshold) as i64;
    let http_state = AppState {
        store,
        online_threshold_seconds,
        jwt_secret,
        jwks,
        metrics_handle,
        registry,
        github_client_id,
        github_client_secret,
        github_redirect_uri,
        frontend_url,
    };
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
