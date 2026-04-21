//! Error types for load balancing.

use thiserror::Error;

/// Errors that can occur during load balancing operations.
#[derive(Error, Debug)]
pub enum LoadBalancingError {
    /// No agents available for load balancing.
    #[error("No agents available for load balancing")]
    NoAgentsAvailable,

    /// Agent not found in the load balancer registry.
    #[error("Agent '{0}' not found")]
    AgentNotFound(String),

    /// Invalid configuration provided.
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Metrics collection failed.
    #[error("Failed to collect metrics: {0}")]
    MetricsError(String),

    /// Distributed coordination error.
    #[error("Distributed coordination error: {0}")]
    CoordinationError(String),

    /// Prediction error.
    #[error("Prediction error: {0}")]
    PredictionError(String),

    /// Adaptive algorithm error.
    #[error("Adaptive algorithm error: {0}")]
    AdaptiveError(String),

    /// I/O or system error.
    #[error("System error: {0}")]
    SystemError(#[from] std::io::Error),

    /// Other errors wrapped as strings.
    #[error("{0}")]
    Other(String),
}

impl LoadBalancingError {
    /// Create a new coordination error.
    pub fn coordination(msg: impl Into<String>) -> Self {
        Self::CoordinationError(msg.into())
    }

    /// Create a new prediction error.
    pub fn prediction(msg: impl Into<String>) -> Self {
        Self::PredictionError(msg.into())
    }

    /// Create a new adaptive error.
    pub fn adaptive(msg: impl Into<String>) -> Self {
        Self::AdaptiveError(msg.into())
    }
}

/// Result type for load balancing operations.
pub type Result<T> = std::result::Result<T, LoadBalancingError>;