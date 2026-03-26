//! Error types for the SDK.

use thiserror::Error;

/// A generic error that can occur in any part of the SDK.
#[derive(Error, Debug)]
pub enum SdkError {
    /// I/O error (file, network, etc.)
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization/deserialization error.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Invalid configuration.
    #[error("Configuration error: {0}")]
    Config(String),

    /// Network-related error.
    #[error("Network error: {0}")]
    Network(String),

    /// CRDT merge conflict or inconsistency.
    #[error("CRDT error: {0}")]
    Crdt(String),

    /// Timeout occurred.
    #[error("Timeout")]
    Timeout,

    /// Other errors wrapped in `anyhow`.
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Alias for `Result<T, SdkError>`.
pub type Result<T> = std::result::Result<T, SdkError>;