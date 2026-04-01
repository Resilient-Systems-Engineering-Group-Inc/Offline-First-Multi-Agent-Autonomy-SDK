//! Error types for RBAC.

use thiserror::Error;

/// Errors that can occur in RBAC operations.
#[derive(Error, Debug)]
pub enum RbacError {
    /// Role not found.
    #[error("Role not found: {0}")]
    RoleNotFound(String),

    /// Permission denied.
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Invalid policy.
    #[error("Invalid policy: {0}")]
    InvalidPolicy(String),

    /// Serialization/deserialization error.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    Config(String),

    /// Conflict (e.g., duplicate role).
    #[error("Conflict: {0}")]
    Conflict(String),

    /// Other errors.
    #[error("Other error: {0}")]
    Other(String),
}

/// Result type for RBAC operations.
pub type Result<T> = std::result::Result<T, RbacError>;