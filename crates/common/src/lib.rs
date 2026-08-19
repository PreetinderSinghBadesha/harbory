//! Crypto primitives shared between the control plane and the agent:
//! keypair generation/signing, public key fingerprinting, and the signed
//! credential format. Kept dependency-light and free of any I/O so both
//! sides can use it without pulling in gRPC/DB machinery.

pub mod credential;
pub mod fingerprint;
pub mod keypair;
