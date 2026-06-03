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

    /// Internal error (e.g., invariant violation, unexpected state).
    #[error("Internal error: {0}")]
    Internal(String),

    /// Not found.
    #[error("Not found: {0}")]
    NotFound(String),

    /// Invalid argument.
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    /// Unsupported operation.
    #[error("Unsupported operation: {0}")]
    Unsupported(String),

    /// Other errors wrapped in `anyhow`.
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl SdkError {
    /// Create an internal error.
    pub fn internal(msg: impl Into<String>) -> Self {
        SdkError::Internal(msg.into())
    }

    /// Create a not-found error.
    pub fn not_found(msg: impl Into<String>) -> Self {
        SdkError::NotFound(msg.into())
    }

    /// Create an invalid argument error.
    pub fn invalid_argument(msg: impl Into<String>) -> Self {
        SdkError::InvalidArgument(msg.into())
    }

    /// Create a configuration error.
    pub fn config(msg: impl Into<String>) -> Self {
        SdkError::Config(msg.into())
    }

    /// Returns `true` if this error is a timeout.
    pub fn is_timeout(&self) -> bool {
        matches!(self, SdkError::Timeout)
    }

    /// Returns `true` if this error is an internal error.
    pub fn is_internal(&self) -> bool {
        matches!(self, SdkError::Internal(_))
    }

    /// Returns `true` if this error is a not-found error.
    pub fn is_not_found(&self) -> bool {
        matches!(self, SdkError::NotFound(_))
    }

    /// Returns `true` if this error is an invalid argument error.
    pub fn is_invalid_argument(&self) -> bool {
        matches!(self, SdkError::InvalidArgument(_))
    }

    /// Returns `true` if this error is a configuration error.
    pub fn is_config(&self) -> bool {
        matches!(self, SdkError::Config(_))
    }

    /// Returns `true` if this error is a network error.
    pub fn is_network(&self) -> bool {
        matches!(self, SdkError::Network(_))
    }

    /// Returns `true` if this error is a CRDT error.
    pub fn is_crdt(&self) -> bool {
        matches!(self, SdkError::Crdt(_))
    }

    /// Returns `true` if this error is an unsupported operation error.
    pub fn is_unsupported(&self) -> bool {
        matches!(self, SdkError::Unsupported(_))
    }
}

/// Alias for `Result<T, SdkError>`.
pub type Result<T> = std::result::Result<T, SdkError>;