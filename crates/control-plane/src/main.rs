use std::path::PathBuf;

use harbory_common::keypair::Keypair;
use harbory_control_plane::{
    grpc::PairingServiceImpl,
    store::Store,
};
use harbory_protocol::v1::pairing_service_server::PairingServiceServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://harbory:harbory_dev_password@localhost:55433/harbory".into());
    let signing_key_path = std::env::var("CONTROL_PLANE_SIGNING_KEY_PATH")
        .unwrap_or_else(|_| "./control-plane-signing-key".into());
    let addr = std::env::var("LISTEN_ADDR").unwrap_or_else(|_| "127.0.0.1:50051".into());

    let store = Store::connect(&database_url).await?;
    let signer = Keypair::load_or_generate(&PathBuf::from(signing_key_path))?;

    tracing::info!(%addr, "starting harbory control plane");

    // NOTE: plaintext h2c for now. TLS (rustls) termination is added when
    // the persistent bi-directional stream is wired up in Phase 2 — see
    // /docs/security.md.
    tonic::transport::Server::builder()
        .add_service(PairingServiceServer::new(PairingServiceImpl { store, signer }))
        .serve(addr.parse()?)
        .await?;

    Ok(())
}
