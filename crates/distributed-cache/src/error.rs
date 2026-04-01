//! Error types for distributed cache.

use thiserror::Error;

/// Errors that can occur in distributed cache operations.
#[derive(Error, Debug)]
pub enum CacheError {
    /// Key not found.
    #[error("Key not found: {0}")]
    NotFound(String),

    /// Cache is full (eviction needed).
    #[error("Cache is full")]
    Full,

    /// Serialization/deserialization error.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Network error (transport).
    #[error("Network error: {0}")]
    Network(String),

    /// Timeout.
    #[error("Operation timeout")]
    Timeout,

    /// Invalid configuration.
    #[error("Invalid configuration: {0}")]
    Config(String),

    /// Replication error.
    #[error("Replication error: {0}")]
    Replication(String),

    /// Other errors.
    #[error("Other error: {0}")]
    Other(String),
}

/// Result type for cache operations.
pub type Result<T> = std::result::Result<T, CacheError>;