//! Error types for configuration management.

use thiserror::Error;

/// Top‑level error for configuration operations.
#[derive(Error, Debug)]
pub enum Error {
    /// I/O error while reading/writing configuration files.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Failed to parse configuration (YAML, JSON, TOML, etc.).
    #[error("Parse error: {0}")]
    Parse(String),

    /// Validation error (invalid values, missing required fields).
    #[error("Validation error: {0}")]
    Validation(String),

    /// Configuration file not found.
    #[error("Configuration file not found: {0}")]
    NotFound(String),

    /// Watch error (file system monitoring).
    #[error("Watch error: {0}")]
    Watch(String),

    /// Internal error.
    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}