//! Error types for streaming.

use thiserror::Error;

/// Top‑level error for streaming operations.
#[derive(Error, Debug)]
pub enum Error {
    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Mesh transport error.
    #[error("Mesh transport error: {0}")]
    Transport(#[from] mesh_transport::Error),

    /// Codec error (encoding/decoding).
    #[error("Codec error: {0}")]
    Codec(String),

    /// Channel closed.
    #[error("Channel closed")]
    ChannelClosed,

    /// Timeout.
    #[error("Timeout")]
    Timeout,

    /// Invalid QoS level.
    #[error("Invalid QoS: {0}")]
    InvalidQoS(String),

    /// Subscription error.
    #[error("Subscription error: {0}")]
    Subscription(String),

    /// Internal error.
    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}