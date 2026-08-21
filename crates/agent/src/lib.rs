//! Exposes just enough for `examples/render_proxy_config.rs` to reuse the
//! real `render` implementation when verifying its output against a real
//! nginx parser — the binary target (`main.rs`) is the actual agent.
pub mod proxy;
