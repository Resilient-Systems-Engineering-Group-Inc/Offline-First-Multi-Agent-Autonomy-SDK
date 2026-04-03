//! Error types for incident management.

use thiserror::Error;

/// Incident management error.
#[derive(Error, Debug)]
pub enum IncidentError {
    /// Incident not found.
    #[error("incident not found: {0}")]
    NotFound(String),

    /// Invalid incident data.
    #[error("invalid incident data: {0}")]
    InvalidData(String),

    /// Detection error.
    #[error("detection error: {0}")]
    Detection(String),

    /// Escalation error.
    #[error("escalation error: {0}")]
    Escalation(String),

    /// Resolution error.
    #[error("resolution error: {0}")]
    Resolution(String),

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

/// Alias for `Result<T, IncidentError>`.
pub type Result<T> = std::result::Result<T, IncidentError>;