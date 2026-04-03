//! Error types for agent package management.

use thiserror::Error;

/// Main error type for package management operations.
#[derive(Error, Debug)]
pub enum PackageError {
    /// Package not found.
    #[error("Package '{0}' not found")]
    PackageNotFound(String),

    /// Version not found.
    #[error("Version '{0}' of package '{1}' not found")]
    VersionNotFound(String, String),

    /// Invalid package format.
    #[error("Invalid package format: {0}")]
    InvalidFormat(String),

    /// Dependency resolution failed.
    #[error("Dependency resolution failed: {0}")]
    DependencyResolution(String),

    /// Installation failed.
    #[error("Installation failed: {0}")]
    InstallationFailed(String),

    /// Verification failed.
    #[error("Verification failed: {0}")]
    VerificationFailed(String),

    /// Checksum mismatch.
    #[error("Checksum mismatch: expected {0}, got {1}")]
    ChecksumMismatch(String, String),

    /// Conflict error.
    #[error("Conflict: {0}")]
    Conflict(String),

    /// Network error.
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Storage error.
    #[error("Storage error: {0}")]
    StorageError(String),

    /// Serialization/deserialization error.
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// URL parsing error.
    #[error("URL error: {0}")]
    Url(#[from] url::ParseError),

    /// Semver error.
    #[error("Semver error: {0}")]
    Semver(#[from] semver::Error),
}

/// Result type for package management operations.
pub type Result<T> = std::result::Result<T, PackageError>;