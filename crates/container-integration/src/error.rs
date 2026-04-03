//! Error types for container integration.

use thiserror::Error;

/// Main error type for container operations.
#[derive(Error, Debug)]
pub enum ContainerError {
    /// Container not found.
    #[error("Container '{0}' not found")]
    ContainerNotFound(String),

    /// Image not found.
    #[error("Image '{0}' not found")]
    ImageNotFound(String),

    /// Container runtime error.
    #[error("Container runtime error: {0}")]
    RuntimeError(String),

    /// Docker API error.
    #[cfg(feature = "docker")]
    #[error("Docker API error: {0}")]
    DockerError(String),

    /// Containerd API error.
    #[cfg(feature = "containerd")]
    #[error("Containerd API error: {0}")]
    ContainerdError(String),

    /// Image build error.
    #[error("Image build error: {0}")]
    BuildError(String),

    /// Image pull error.
    #[error("Image pull error: {0}")]
    PullError(String),

    /// Image push error.
    #[error("Image push error: {0}")]
    PushError(String),

    /// Container start error.
    #[error("Container start error: {0}")]
    StartError(String),

    /// Container stop error.
    #[error("Container stop error: {0}")]
    StopError(String),

    /// Container remove error.
    #[error("Container remove error: {0}")]
    RemoveError(String),

    /// Resource constraint error.
    #[error("Resource constraint error: {0}")]
    ResourceError(String),

    /// Networking error.
    #[error("Networking error: {0}")]
    NetworkError(String),

    /// Volume error.
    #[error("Volume error: {0}")]
    VolumeError(String),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// URL parsing error.
    #[error("URL error: {0}")]
    Url(#[from] url::ParseError),

    /// Invalid argument error.
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    /// Timeout error.
    #[error("Operation timed out: {0}")]
    Timeout(String),

    /// Unknown error.
    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Result type for container operations.
pub type Result<T> = std::result::Result<T, ContainerError>;