//! Error types for the distributed KV store.

use thiserror::Error;

/// Main error type.
#[derive(Error, Debug)]
pub enum Error {
    /// CRDT operation failed.
    #[error("CRDT error: {0}")]
    Crdt(String),

    /// Serialization/deserialization error.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// I/O error (e.g., file system).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Persistence error (sled, etc.)
    #[error("Persistence error: {0}")]
    Persistence(String),

    /// Network/transport error.
    #[error("Transport error: {0}")]
    Transport(String),

    /// Invalid query.
    #[error("Invalid query: {0}")]
    Query(String),

    /// Key not found.
    #[error("Key not found: {0}")]
    NotFound(String),

    /// Conflict (e.g., concurrent modification).
    #[error("Conflict: {0}")]
    Conflict(String),

    /// Timeout.
    #[error("Timeout: {0}")]
    Timeout(String),

    /// Other generic error.
    #[error("Other: {0}")]
    Other(String),
}

impl From<common::error::SdkError> for Error {
    fn from(e: common::error::SdkError) -> Self {
        match e {
            common::error::SdkError::Io(io) => Error::Io(io),
            common::error::SdkError::Serialization(ser) => Error::Serialization(ser),
            common::error::SdkError::Crdt(s) => Error::Crdt(s),
            common::error::SdkError::Network(s) => Error::Transport(s),
            common::error::SdkError::Config(s) => Error::Other(s),
            common::error::SdkError::Timeout => Error::Timeout("timeout".to_string()),
            common::error::SdkError::Other(any) => Error::Other(any.to_string()),
        }
    }
}

/// Convenience result type.
pub type Result<T> = std::result::Result<T, Error>;