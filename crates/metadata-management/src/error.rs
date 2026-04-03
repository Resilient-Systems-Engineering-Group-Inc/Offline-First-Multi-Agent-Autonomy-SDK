//! Error types for metadata management.

use thiserror::Error;

/// Metadata management error.
#[derive(Error, Debug)]
pub enum MetadataError {
    /// Metadata not found.
    #[error("metadata not found: {0}")]
    NotFound(String),

    /// Invalid metadata schema.
    #[error("invalid schema: {0}")]
    InvalidSchema(String),

    /// Validation error.
    #[error("validation error: {0}")]
    Validation(String),

    /// Index error.
    #[error("index error: {0}")]
    Index(String),

    /// Query error.
    #[error("query error: {0}")]
    Query(String),

    /// Versioning error.
    #[error("versioning error: {0}")]
    Versioning(String),

    /// Serialization/deserialization error.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Other error.
    #[error("other error: {0}")]
    Other(String),
}

/// Alias for `Result<T, MetadataError>`.
pub type Result<T> = std::result::Result<T, MetadataError>;