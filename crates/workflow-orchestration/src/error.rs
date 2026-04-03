//! Error types for workflow orchestration.

use thiserror::Error;

/// Workflow orchestration error.
#[derive(Error, Debug)]
pub enum WorkflowError {
    /// Invalid workflow definition.
    #[error("invalid workflow definition: {0}")]
    InvalidDefinition(String),

    /// Workflow not found.
    #[error("workflow not found: {0}")]
    NotFound(String),

    /// Task execution failed.
    #[error("task execution failed: {0}")]
    TaskExecution(String),

    /// Scheduling conflict.
    #[error("scheduling conflict: {0}")]
    Scheduling(String),

    /// Coordination error (e.g., consensus failure).
    #[error("coordination error: {0}")]
    Coordination(String),

    /// Timeout.
    #[error("timeout: {0}")]
    Timeout(String),

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

/// Alias for `Result<T, WorkflowError>`.
pub type Result<T> = std::result::Result<T, WorkflowError>;