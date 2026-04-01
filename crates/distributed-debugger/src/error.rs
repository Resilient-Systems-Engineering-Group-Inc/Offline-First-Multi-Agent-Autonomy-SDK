//! Error types for distributed debugger.

use thiserror::Error;

/// Errors that can occur in debugging operations.
#[derive(Error, Debug)]
pub enum DebuggerError {
    /// Agent not found.
    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    /// Debug session not found.
    #[error("Debug session not found: {0}")]
    SessionNotFound(String),

    /// Invalid command.
    #[error("Invalid command: {0}")]
    InvalidCommand(String),

    /// Serialization/deserialization error.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Network error.
    #[error("Network error: {0}")]
    Network(String),

    /// Timeout.
    #[error("Operation timeout")]
    Timeout,

    /// Permission denied.
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Other errors.
    #[error("Other error: {0}")]
    Other(String),
}

/// Result type for debugger operations.
pub type Result<T> = std::result::Result<T, DebuggerError>;