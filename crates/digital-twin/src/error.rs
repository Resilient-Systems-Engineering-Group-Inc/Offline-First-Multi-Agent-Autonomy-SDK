//! Error types for digital twin.

use thiserror::Error;

/// Top‑level error for digital twin.
#[derive(Error, Debug)]
pub enum Error {
    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Physics simulation error.
    #[error("Physics error: {0}")]
    Physics(String),

    /// Visualization error.
    #[error("Visualization error: {0}")]
    Visualization(String),

    /// Model inconsistency.
    #[error("Model error: {0}")]
    Model(String),

    /// Internal error.
    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}