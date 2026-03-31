//! Error types for autoscaling.

use thiserror::Error;

/// Errors that can occur in autoscaling operations.
#[derive(Error, Debug)]
pub enum AutoscalingError {
    /// Invalid configuration.
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),

    /// Resource monitoring error.
    #[error("resource monitoring error: {0}")]
    ResourceMonitoring(String),

    /// Scaling action failed.
    #[error("scaling action failed: {0}")]
    ScalingActionFailed(String),

    /// Policy evaluation error.
    #[error("policy evaluation error: {0}")]
    PolicyEvaluation(String),

    /// Communication error with agents.
    #[error("communication error: {0}")]
    Communication(String),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Other error.
    #[error("other error: {0}")]
    Other(String),
}