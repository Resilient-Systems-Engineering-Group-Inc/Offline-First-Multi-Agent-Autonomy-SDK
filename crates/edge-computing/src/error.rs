//! Error types for edge computing.

use thiserror::Error;

/// Top‑level error for edge computing.
#[derive(Error, Debug)]
pub enum Error {
    /// Hardware detection error.
    #[error("Hardware detection error: {0}")]
    Hardware(String),

    /// Resource constraint violation.
    #[error("Resource constraint violation: {0}")]
    ResourceConstraint(String),

    /// Platform unsupported.
    #[error("Platform unsupported: {0}")]
    UnsupportedPlatform(String),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Internal error.
    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}