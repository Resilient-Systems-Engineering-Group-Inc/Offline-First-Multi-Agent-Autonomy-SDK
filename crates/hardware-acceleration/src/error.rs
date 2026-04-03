//! Error types for hardware acceleration.

use thiserror::Error;

/// Errors that can occur while working with hardware accelerators.
#[derive(Error, Debug)]
pub enum AccelerationError {
    /// No compatible hardware found.
    #[error("No compatible hardware found: {0}")]
    NoHardware(String),

    /// Device initialization failed.
    #[error("Device initialization failed: {0}")]
    DeviceInit(String),

    /// Out of memory on the device.
    #[error("Out of device memory: {0}")]
    OutOfMemory(String),

    /// Kernel compilation or loading failed.
    #[error("Kernel error: {0}")]
    Kernel(String),

    /// Memory transfer failed.
    #[error("Memory transfer error: {0}")]
    MemoryTransfer(String),

    /// Unsupported operation for the current backend.
    #[error("Unsupported operation: {0}")]
    Unsupported(String),

    /// Backend‑specific error.
    #[cfg(feature = "gpu")]
    #[error("OpenCL error: {0}")]
    OpenCl(String),

    /// CUDA‑specific error.
    #[cfg(feature = "cuda")]
    #[error("CUDA error: {0}")]
    Cuda(String),

    /// TPU‑specific error.
    #[cfg(feature = "tpu")]
    #[error("TPU error: {0}")]
    Tpu(String),

    /// ML framework error.
    #[cfg(feature = "ml")]
    #[error("ML framework error: {0}")]
    Ml(String),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization/deserialization error.
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Other errors wrapped in `anyhow`.
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Alias for `Result<T, AccelerationError>`.
pub type Result<T> = std::result::Result<T, AccelerationError>;