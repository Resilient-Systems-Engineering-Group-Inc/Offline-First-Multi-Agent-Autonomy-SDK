//! Error types for ML model versioning.

use thiserror::Error;

/// Main error type for the ML model versioning system.
#[derive(Error, Debug)]
pub enum ModelVersioningError {
    /// Invalid model version format.
    #[error("Invalid version format: {0}")]
    InvalidVersion(String),

    /// Model not found.
    #[error("Model '{0}' not found")]
    ModelNotFound(String),

    /// Version not found.
    #[error("Version '{0}' of model '{1}' not found")]
    VersionNotFound(String, String),

    /// Storage error.
    #[error("Storage error: {0}")]
    StorageError(String),

    /// Serialization/deserialization error.
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Checksum mismatch.
    #[error("Checksum mismatch: expected {0}, got {1}")]
    ChecksumMismatch(String, String),

    /// Dependency error.
    #[error("Dependency error: {0}")]
    DependencyError(String),

    /// Invalid metadata.
    #[error("Invalid metadata: {0}")]
    InvalidMetadata(String),

    /// Conflict error (e.g., version already exists).
    #[error("Conflict: {0}")]
    Conflict(String),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Semver error.
    #[error("Semver error: {0}")]
    Semver(#[from] semver::Error),
}

/// Result type for ML model versioning operations.
pub type Result<T> = std::result::Result<T, ModelVersioningError>;