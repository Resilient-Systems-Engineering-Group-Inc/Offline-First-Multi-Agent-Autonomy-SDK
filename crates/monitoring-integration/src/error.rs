//! Error types for monitoring integration.

use thiserror::Error;

/// Errors that can occur in monitoring integration.
#[derive(Error, Debug)]
pub enum MonitoringError {
    /// Prometheus-related error.
    #[error("Prometheus error: {0}")]
    Prometheus(#[from] prometheus::Error),

    /// OpenTelemetry-related error.
    #[error("OpenTelemetry error: {0}")]
    OpenTelemetry(#[from] opentelemetry::trace::TraceError),

    /// OTLP export error.
    #[error("OTLP export error: {0}")]
    OtlpExport(String),

    /// HTTP server error.
    #[error("HTTP server error: {0}")]
    HttpServer(#[from] std::io::Error),

    /// Reqwest error (for Grafana API).
    #[error("HTTP client error: {0}")]
    HttpClient(#[from] reqwest::Error),

    /// Invalid configuration.
    #[error("Invalid configuration: {0}")]
    Config(String),

    /// Serialization/deserialization error.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Other errors.
    #[error("Other error: {0}")]
    Other(String),
}

/// Result type for monitoring operations.
pub type Result<T> = std::result::Result<T, MonitoringError>;