//! Error types for distributed logging.

use thiserror::Error;

/// Errors that can occur while logging or managing logs.
#[derive(Error, Debug)]
pub enum LogError {
    /// I/O error (file, network, etc.)
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization/deserialization error.
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Compression/decompression error.
    #[cfg(feature = "compression")]
    #[error("Compression error: {0}")]
    Compression(String),

    /// Network transport error.
    #[cfg(feature = "mesh")]
    #[error("Transport error: {0}")]
    Transport(String),

    /// State synchronization error.
    #[cfg(feature = "sync")]
    #[error("Sync error: {0}")]
    Sync(String),

    /// Invalid log level.
    #[error("Invalid log level: {0}")]
    InvalidLevel(String),

    /// Log sink error (e.g., cannot write to sink).
    #[error("Sink error: {0}")]
    Sink(String),

    /// Aggregation error.
    #[error("Aggregation error: {0}")]
    Aggregation(String),

    /// Other errors wrapped in `anyhow`.
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Alias for `Result<T, LogError>`.
pub type Result<T> = std::result::Result<T, LogError>;