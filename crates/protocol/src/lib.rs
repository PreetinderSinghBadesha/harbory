//! Shared gRPC service definitions and message types for the control plane
//! <-> agent wire protocol. Single source of truth: both sides depend on
//! this crate rather than hand-maintaining parallel structs.

pub mod v1 {
    tonic::include_proto!("harbory.v1");
}
