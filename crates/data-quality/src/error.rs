//! Error types for data quality management.

use thiserror::Error;

/// Data quality error.
#[derive(Error, Debug)]
pub enum DataQualityError {
    /// Validation failed.
    #[error("validation failed: {0}")]
    ValidationFailed(String),

    /// Invalid rule definition.
    #[error("invalid rule: {0}")]
    InvalidRule(String),

    /// Anomaly detection error.
    #[error("anomaly detection error: {0}")]
    AnomalyDetection(String),

    /// Metric collection error.
    #[error("metric collection error: {0}")]
    MetricCollection(String),

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

/// Alias for `Result<T, DataQualityError>`.
pub type Result<T> = std::result::Result<T, DataQualityError>;