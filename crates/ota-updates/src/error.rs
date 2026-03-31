//! Error types for OTA updates.

use thiserror::Error;

/// Top‑level error for OTA operations.
#[derive(Error, Debug)]
pub enum Error {
    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Network error.
    #[error("Network error: {0}")]
    Network(String),

    /// Signature verification failed.
    #[error("Signature verification failed: {0}")]
    Signature(String),

    /// Package validation failed.
    #[error("Package validation failed: {0}")]
    Validation(String),

    /// Delta application failed.
    #[error("Delta application failed: {0}")]
    Delta(String),

    /// Version conflict.
    #[error("Version conflict: {0}")]
    VersionConflict(String),

    /// Internal error.
    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}