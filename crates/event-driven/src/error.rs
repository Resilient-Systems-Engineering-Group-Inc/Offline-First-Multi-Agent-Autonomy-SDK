//! Error types for event-driven architecture.

use thiserror::Error;

/// Main error type for the event-driven system.
#[derive(Error, Debug)]
pub enum EventError {
    /// Event bus error.
    #[error("Event bus error: {0}")]
    BusError(String),

    /// Subscription error.
    #[error("Subscription error: {0}")]
    SubscriptionError(String),

    /// Publishing error.
    #[error("Publishing error: {0}")]
    PublishError(String),

    /// Serialization/deserialization error.
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Channel error.
    #[error("Channel error: {0}")]
    ChannelError(String),

    /// Timeout error.
    #[error("Timeout waiting for event")]
    Timeout,

    /// Event not found.
    #[error("Event '{0}' not found")]
    EventNotFound(String),

    /// Handler error.
    #[error("Handler error: {0}")]
    HandlerError(String),

    /// Invalid event format.
    #[error("Invalid event format: {0}")]
    InvalidEvent(String),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// UUID error.
    #[error("UUID error: {0}")]
    Uuid(#[from] uuid::Error),
}

/// Result type for event-driven operations.
pub type Result<T> = std::result::Result<T, EventError>;