//! Error types for agent lifecycle management.

use thiserror::Error;

/// Errors that can occur in agent lifecycle management.
#[derive(Error, Debug)]
pub enum LifecycleError {
    /// Agent is already in the requested state.
    #[error("Agent {0} is already {1}")]
    AlreadyInState(u64, String),

    /// Agent not found.
    #[error("Agent {0} not found")]
    AgentNotFound(u64),

    /// Invalid state transition.
    #[error("Cannot transition from {0} to {1}")]
    InvalidTransition(String, String),

    /// Timeout during operation.
    #[error("Operation timed out: {0}")]
    Timeout(String),

    /// Health check failed.
    #[error("Health check failed: {0}")]
    HealthCheckFailed(String),

    /// Dependency error.
    #[error("Dependency error: {0}")]
    DependencyError(String),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Transport error.
    #[error("Transport error: {0}")]
    TransportError(#[from] mesh_transport::Error),

    /// Resource monitor error.
    #[error("Resource monitor error: {0}")]
    ResourceMonitorError(#[from] resource_monitor::Error),

    /// Internal error.
    #[error("Internal error: {0}")]
    Internal(String),

    /// Serialization/deserialization error.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Result type for lifecycle operations.
pub type Result<T> = std::result::Result<T, LifecycleError>;