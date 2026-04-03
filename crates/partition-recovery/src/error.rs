//! Error types for partition recovery.

use thiserror::Error;

/// Errors that can occur during partition detection and recovery.
#[derive(Error, Debug)]
pub enum PartitionRecoveryError {
    #[error("Transport error: {0}")]
    Transport(#[from] anyhow::Error),
    #[error("State sync error: {0}")]
    StateSync(String),
    #[error("Consensus error: {0}")]
    Consensus(String),
    #[error("Invalid configuration: {0}")]
    Config(String),
    #[error("Timeout while waiting for recovery")]
    Timeout,
    #[error("Partition detection failed: {0}")]
    Detection(String),
    #[error("Recovery protocol failed: {0}")]
    Recovery(String),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}