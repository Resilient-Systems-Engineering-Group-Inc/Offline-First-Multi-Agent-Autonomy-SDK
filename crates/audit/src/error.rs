//! Error types for audit logging.

use thiserror::Error;

/// Top‑level error for audit operations.
#[derive(Error, Debug)]
pub enum Error {
    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error.
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Backend error.
    #[error("Backend error: {0}")]
    Backend(String),

    /// Search error.
    #[error("Search error: {0}")]
    Search(String),

    /// Internal error.
    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}