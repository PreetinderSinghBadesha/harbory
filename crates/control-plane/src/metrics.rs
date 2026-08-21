//! Process-global counters/gauges for agent and control-plane health,
//! exposed as Prometheus text at `GET /metrics` (unauthenticated — see
//! docs/observability.md for why that's a deliberate choice, not an
//! oversight). Instrumentation calls (`metrics::counter!`/`gauge!`) live
//! at the call sites in `stream.rs` and `grpc.rs`, not here — this module
//! only owns installing the recorder and rendering it for the endpoint.

use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

pub fn install() -> PrometheusHandle {
    PrometheusBuilder::new().install_recorder().expect("failed to install Prometheus metrics recorder")
}

/// RAII guard for the "agents currently connected" gauge: increments on
/// creation (right after a stream authenticates), decrements on drop —
/// so every one of `drive_connection`'s several `break`/return points
/// decrements it correctly without repeating the call at each one.
pub struct ConnectedAgentGuard;

impl ConnectedAgentGuard {
    pub fn new() -> Self {
        ::metrics::gauge!("harbory_agents_connected").increment(1.0);
        Self
    }
}

impl Drop for ConnectedAgentGuard {
    fn drop(&mut self) {
        ::metrics::gauge!("harbory_agents_connected").decrement(1.0);
    }
}
