use tonic::transport::{Channel, ClientTlsConfig, Endpoint};

/// Builds a client channel to the control plane, adding a TLS handshake
/// automatically when `addr` uses `https://` — plain h2c otherwise, same
/// as before. One shared helper since both the one-shot pairing RPC
/// (`main.rs`) and the persistent stream (`stream.rs`) need identical
/// connection logic, and `Client::connect(addr)`'s convenience method
/// doesn't give a place to attach TLS config.
pub async fn connect(addr: &str) -> Result<Channel, tonic::transport::Error> {
    let mut endpoint = Endpoint::from_shared(addr.to_string())?;
    if addr.starts_with("https://") {
        // `ClientTlsConfig::new()` alone has no trust anchors — enabling
        // the `tls-webpki-roots` Cargo feature only makes this builder
        // method available, it doesn't call it. Without this, every real
        // cert (Let's Encrypt included) fails as `UnknownIssuer`.
        endpoint = endpoint.tls_config(ClientTlsConfig::new().with_webpki_roots())?;
    }
    endpoint.connect().await
}
