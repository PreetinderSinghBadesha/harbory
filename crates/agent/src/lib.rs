//! Exposes just enough for the dev-only examples under `examples/` to
//! reuse real implementations when smoke-testing against a live
//! nginx/Docker install — the binary target (`main.rs`) is the actual
//! agent.
pub mod container;
pub mod docker_inspect;
pub mod git_build;
pub mod proxy;
pub mod volumes;
