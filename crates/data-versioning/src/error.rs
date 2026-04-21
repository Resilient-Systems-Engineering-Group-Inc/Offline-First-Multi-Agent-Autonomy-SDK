//! Error types for data versioning.

use thiserror::Error;

/// Errors that can occur in data versioning operations.
#[derive(Error, Debug)]
pub enum VersioningError {
    /// Snapshot not found.
    #[error("Snapshot not found: {0}")]
    SnapshotNotFound(String),

    /// Invalid version format.
    #[error("Invalid version format: {0}")]
    InvalidVersion(String),

    /// Serialization/deserialization error.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// State sync error.
    #[error("State sync error: {0}")]
    StateSync(String),

    /// Version conflict (concurrent modification).
    #[error("Version conflict: {0}")]
    Conflict(String),

    /// Storage backend error.
    #[error("Storage backend error: {0}")]
    Storage(String),

    /// Lineage tracking error.
    #[error("Lineage error: {0}")]
    LineageError(String),

    /// Other errors.
    #[error("Other error: {0}")]
    Other(String),
}

/// Result type for versioning operations.
pub type Result<T> = std::result::Result<T, VersioningError>;