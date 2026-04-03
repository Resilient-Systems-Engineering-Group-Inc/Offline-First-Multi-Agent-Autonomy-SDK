//! Error types for power management.

use thiserror::Error;

/// Power management errors.
#[derive(Error, Debug)]
pub enum Error {
    /// Failed to read power information from the system.
    #[error("Failed to read power information: {0}")]
    MonitorError(String),

    /// Invalid power policy configuration.
    #[error("Invalid power policy: {0}")]
    PolicyError(String),

    /// Scheduling error.
    #[error("Scheduling error: {0}")]
    SchedulingError(String),

    /// Platform‑specific error (e.g., syscall failure).
    #[error("Platform error: {0}")]
    PlatformError(String),

    /// Battery not present or not supported.
    #[error("Battery not available: {0}")]
    BatteryNotAvailable(String),

    /// Invalid power state transition.
    #[error("Invalid power state transition: {0}")]
    InvalidStateTransition(String),

    /// External dependency error (e.g., resource‑monitor).
    #[error("External dependency error: {0}")]
    ExternalError(String),

    /// Generic error for unexpected conditions.
    #[error("Power management error: {0}")]
    Generic(String),
}

/// Result alias for power management operations.
pub type Result<T> = std::result::Result<T, Error>;