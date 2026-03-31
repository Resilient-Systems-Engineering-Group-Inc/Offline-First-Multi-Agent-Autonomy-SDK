//! Error types for federated learning.

use thiserror::Error;

/// Top‑level error for federated learning.
#[derive(Error, Debug)]
pub enum Error {
    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Network error.
    #[error("Network error: {0}")]
    Network(String),

    /// Model serialization/deserialization error.
    #[error("Model error: {0}")]
    Model(String),

    /// Aggregation error.
    #[error("Aggregation error: {0}")]
    Aggregation(String),

    /// Privacy error (e.g., differential privacy violation).
    #[error("Privacy error: {0}")]
    Privacy(String),

    /// Internal error.
    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}