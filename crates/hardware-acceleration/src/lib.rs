//! Hardware acceleration support (GPU, TPU, FPGA) for agent systems.
//!
//! This crate provides abstractions and implementations for leveraging
//! specialized hardware accelerators in multi‑agent autonomous systems.
//!
//! Supported hardware:
//! - **GPU** – via OpenCL, CUDA, or wgpu (WebGPU)
//! - **TPU** – Tensor Processing Units (via TensorFlow Lite, PyTorch, or custom drivers)
//! - **FPGA** – Field‑Programmable Gate Arrays (via OpenCL or vendor SDKs)
//!
//! # Example
//! ```
//! use hardware_acceleration::{Accelerator, DeviceType, AccelerationBackend};
//!
//! let accelerator = Accelerator::new(DeviceType::GPU, AccelerationBackend::OpenCL);
//! if accelerator.is_available() {
//!     println!("GPU acceleration is available");
//! }
//! ```

pub mod accelerator;
pub mod backend;
pub mod device;
pub mod error;
pub mod kernel;
pub mod manager;
pub mod memory;
pub mod task;

#[cfg(feature = "gpu")]
pub mod gpu;

#[cfg(feature = "cuda")]
pub mod cuda;

#[cfg(feature = "tpu")]
pub mod tpu;

#[cfg(feature = "ml")]
pub mod ml;

pub use accelerator::*;
pub use backend::*;
pub use device::*;
pub use error::*;
pub use kernel::*;
pub use manager::*;
pub use memory::*;
pub use task::*;

/// Re‑export of common types for convenience.
pub mod prelude {
    pub use super::{
        Accelerator, Device, DeviceType, AccelerationBackend, Kernel, MemoryBuffer,
        AccelerationTask, AccelerationManager,
    };
    #[cfg(feature = "gpu")]
    pub use super::gpu::*;
    #[cfg(feature = "cuda")]
    pub use super::cuda::*;
    #[cfg(feature = "tpu")]
    pub use super::tpu::*;
    #[cfg(feature = "ml")]
    pub use super::ml::*;
}

/// Initializes the hardware acceleration subsystem.
pub fn init() {
    tracing::info!("Hardware acceleration subsystem initialized");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init() {
        init();
    }
}