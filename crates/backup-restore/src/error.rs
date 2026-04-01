//! Error types for backup and restore operations.

use thiserror::Error;

/// Main error type for backup and restore operations.
#[derive(Error, Debug)]
pub enum BackupError {
    /// Backup creation failed.
    #[error("Backup creation failed: {0}")]
    BackupFailed(String),

    /// Restore operation failed.
    #[error("Restore failed: {0}")]
    RestoreFailed(String),

    /// Verification failed.
    #[error("Verification failed: {0}")]
    VerificationFailed(String),

    /// Storage error.
    #[error("Storage error: {0}")]
    StorageError(String),

    /// Serialization/deserialization error.
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Checksum mismatch.
    #[error("Checksum mismatch: expected {0}, got {1}")]
    ChecksumMismatch(String, String),

    /// Invalid backup format.
    #[error("Invalid backup format: {0}")]
    InvalidFormat(String),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Compression error.
    #[error("Compression error: {0}")]
    Compression(String),
}

/// Result type for backup and restore operations.
pub type Result<T> = std::result::Result<T, BackupError>;