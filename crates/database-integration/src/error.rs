//! Error types for database integration.

use thiserror::Error;

/// Errors that can occur in database operations.
#[derive(Error, Debug)]
pub enum DatabaseError {
    /// SQL error (sqlx).
    #[error("SQL error: {0}")]
    Sql(#[from] sqlx::Error),

    /// Redis error.
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    /// MongoDB error.
    #[cfg(feature = "mongodb")]
    #[error("MongoDB error: {0}")]
    Mongo(#[from] mongodb::error::Error),

    /// Serialization/deserialization error.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Connection error.
    #[error("Connection error: {0}")]
    Connection(String),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    Config(String),

    /// Query error.
    #[error("Query error: {0}")]
    Query(String),

    /// Migration error.
    #[error("Migration error: {0}")]
    Migration(String),

    /// Other errors.
    #[error("Other error: {0}")]
    Other(String),
}

/// Result type for database operations.
pub type Result<T> = std::result::Result<T, DatabaseError>;