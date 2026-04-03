//! Error types for ABAC integration.

use thiserror::Error;

/// ABAC integration error.
#[derive(Error, Debug)]
pub enum AbacError {
    /// Policy not found.
    #[error("policy not found: {0}")]
    PolicyNotFound(String),

    /// Invalid policy definition.
    #[error("invalid policy: {0}")]
    InvalidPolicy(String),

    /// Evaluation error.
    #[error("evaluation error: {0}")]
    Evaluation(String),

    /// Attribute missing.
    #[error("missing attribute: {0}")]
    MissingAttribute(String),

    /// Serialization/deserialization error.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Other error.
    #[error("other error: {0}")]
    Other(String),
}

/// Alias for `Result<T, AbacError>`.
pub type Result<T> = std::result::Result<T, AbacError>;