//! Network partition detection and automatic recovery.
//!
//! This crate provides mechanisms to detect network partitions (split‑brain)
//! in a mesh of agents and automatically recover consistency using
//! consensus‑based reconciliation.

pub mod detection;
pub mod recovery;
pub mod error;
pub mod manager;

pub use detection::PartitionDetector;
pub use recovery::PartitionRecovery;
pub use manager::PartitionRecoveryManager;
pub use error::PartitionRecoveryError;

/// Re‑export of common types.
pub use common::types::{AgentId, VectorClock};
pub use mesh_transport::TransportEvent;
pub use state_sync::{StateSync, DefaultStateSync};