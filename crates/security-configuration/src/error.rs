//! Error types for security configuration management.

use thiserror::Error;

/// Errors that can occur while managing security configurations.
#[derive(Error, Debug)]
pub enum SecurityConfigError {
    /// Configuration file not found or inaccessible.
    #[error("Configuration file error: {0}")]
    ConfigFile(#[from] std::io::Error),

    /// Invalid YAML/JSON syntax.
    #[error("Invalid configuration syntax: {0}")]
    InvalidSyntax(String),

    /// Validation of a security policy failed.
    #[error("Policy validation failed: {0}")]
    ValidationFailed(String),

    /// A required field is missing.
    #[error("Missing required field: {0}")]
    MissingField(String),

    /// Inconsistent configuration (e.g., conflicting rules).
    #[error("Inconsistent configuration: {0}")]
    Inconsistent(String),

    /// Cryptographic operation failed.
    #[cfg(feature = "crypto")]
    #[error("Crypto error: {0}")]
    Crypto(String),

    /// Integration with another component failed.
    #[error("Integration error: {0}")]
    Integration(String),

    /// Audit logging failure.
    #[error("Audit logging error: {0}")]
    Audit(String),

    /// Serialization/deserialization error.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_yaml::Error),

    /// JSON serialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Other errors wrapped in `anyhow`.
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Alias for `Result<T, SecurityConfigError>`.
pub type Result<T> = std::result::Result<T, SecurityConfigError>;