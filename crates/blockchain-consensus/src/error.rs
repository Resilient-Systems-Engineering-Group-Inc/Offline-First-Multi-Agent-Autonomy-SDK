//! Error types for blockchain consensus.

use thiserror::Error;

/// Main error type.
#[derive(Error, Debug)]
pub enum Error {
    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization/deserialization error.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Invalid block (hash mismatch, invalid signature, etc.)
    #[error("Invalid block: {0}")]
    InvalidBlock(String),

    /// Invalid transaction.
    #[error("Invalid transaction: {0}")]
    InvalidTransaction(String),

    /// Consensus failure (e.g., not enough votes).
    #[error("Consensus failure: {0}")]
    Consensus(String),

    /// Stake‑related error.
    #[error("Stake error: {0}")]
    Stake(String),

    /// Network error.
    #[error("Network error: {0}")]
    Network(String),

    /// Other error.
    #[error("Other: {0}")]
    Other(String),
}

/// Convenience result type.
pub type Result<T> = std::result::Result<T, Error>;