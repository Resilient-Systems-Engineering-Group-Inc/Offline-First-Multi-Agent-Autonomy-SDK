//! Error types for IoT interfaces.

use thiserror::Error;

/// Main error type.
#[derive(Error, Debug)]
pub enum Error {
    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization/deserialization error.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Protocol‑specific error (MQTT, CoAP, Modbus, etc.)
    #[error("Protocol error: {0}")]
    Protocol(String),

    /// Device not found.
    #[error("Device not found: {0}")]
    NotFound(String),

    /// Device busy or locked.
    #[error("Device busy: {0}")]
    Busy(String),

    /// Invalid configuration.
    #[error("Invalid configuration: {0}")]
    Config(String),

    /// Timeout.
    #[error("Timeout: {0}")]
    Timeout(String),

    /// Other error.
    #[error("Other: {0}")]
    Other(String),
}

/// Convenience result type.
pub type Result<T> = std::result::Result<T, Error>;