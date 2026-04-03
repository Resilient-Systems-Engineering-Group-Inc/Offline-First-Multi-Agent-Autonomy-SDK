//! Backend registry and concrete implementations for hardware acceleration.

use crate::accelerator::AcceleratorBackend;
use crate::device::{AccelerationBackend, Device, DeviceType};
use crate::error::{Result, AccelerationError};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

/// Registry of available backends.
#[derive(Default)]
pub struct BackendRegistry {
    backends: HashMap<AccelerationBackend, Arc<dyn AcceleratorBackend>>,
}

impl BackendRegistry {
    /// Creates a new empty registry.
    pub fn new() -> Self {
        Self {
            backends: HashMap::new(),
        }
    }

    /// Registers a backend.
    pub fn register(&mut self, backend: Arc<dyn AcceleratorBackend>) {
        self.backends.insert(backend.backend(), backend);
    }

    /// Returns a backend by its type, if registered.
    pub fn get(&self, backend_type: AccelerationBackend) -> Option<Arc<dyn AcceleratorBackend>> {
        self.backends.get(&backend_type).cloned()
    }

    /// Returns all registered backend types.
    pub fn available_backends(&self) -> Vec<AccelerationBackend> {
        self.backends.keys().cloned().collect()
    }

    /// Automatically discovers and registers all available backends on the system.
    pub async fn discover(&mut self) -> Result<()> {
        // Try to register OpenCL backend if feature enabled
        #[cfg(feature = "gpu")]
        {
            if let Ok(backend) = OpenClBackend::new().await {
                self.register(Arc::new(backend));
            }
        }

        // Try to register CUDA backend if feature enabled
        #[cfg(feature = "cuda")]
        {
            if let Ok(backend) = CudaBackend::new().await {
                self.register(Arc::new(backend));
            }
        }

        // Try to register WebGPU backend if feature enabled
        #[cfg(feature = "gpu")]
        {
            if let Ok(backend) = WgpuBackend::new().await {
                self.register(Arc::new(backend));
            }
        }

        // Try to register TPU backend if feature enabled
        #[cfg(feature = "tpu")]
        {
            if let Ok(backend) = TpuBackend::new().await {
                self.register(Arc::new(backend));
            }
        }

        // Always register dummy backend for fallback
        self.register(Arc::new(DummyBackend));

        Ok(())
    }
}

/// OpenCL backend (requires `gpu` feature).
#[cfg(feature = "gpu")]
pub struct OpenClBackend;

#[cfg(feature = "gpu")]
impl OpenClBackend {
    /// Creates a new OpenCL backend.
    pub async fn new() -> Result<Self> {
        // TODO: actual OpenCL initialization
        tracing::info!("OpenCL backend initialized");
        Ok(Self)
    }
}

#[cfg(feature = "gpu")]
#[async_trait]
impl AcceleratorBackend for OpenClBackend {
    fn backend(&self) -> AccelerationBackend {
        AccelerationBackend::OpenCl
    }

    async fn enumerate_devices(&self) -> Result<Vec<Device>> {
        // TODO: enumerate OpenCL devices
        Ok(vec![])
    }

    async fn create_accelerator(&self, device: &Device) -> Result<Box<dyn crate::accelerator::Accelerator>> {
        Err(AccelerationError::UnsupportedOperation("OpenCL accelerator not yet implemented".to_string()))
    }
}

/// CUDA backend (requires `cuda` feature).
#[cfg(feature = "cuda")]
pub struct CudaBackend;

#[cfg(feature = "cuda")]
impl CudaBackend {
    /// Creates a new CUDA backend.
    pub async fn new() -> Result<Self> {
        // TODO: actual CUDA initialization
        tracing::info!("CUDA backend initialized");
        Ok(Self)
    }
}

#[cfg(feature = "cuda")]
#[async_trait]
impl AcceleratorBackend for CudaBackend {
    fn backend(&self) -> AccelerationBackend {
        AccelerationBackend::Cuda
    }

    async fn enumerate_devices(&self) -> Result<Vec<Device>> {
        // TODO: enumerate CUDA devices
        Ok(vec![])
    }

    async fn create_accelerator(&self, device: &Device) -> Result<Box<dyn crate::accelerator::Accelerator>> {
        Err(AccelerationError::UnsupportedOperation("CUDA accelerator not yet implemented".to_string()))
    }
}

/// WebGPU backend (requires `gpu` feature).
#[cfg(feature = "gpu")]
pub struct WgpuBackend;

#[cfg(feature = "gpu")]
impl WgpuBackend {
    /// Creates a new WebGPU backend.
    pub async fn new() -> Result<Self> {
        // TODO: actual wgpu initialization
        tracing::info!("WebGPU backend initialized");
        Ok(Self)
    }
}

#[cfg(feature = "gpu")]
#[async_trait]
impl AcceleratorBackend for WgpuBackend {
    fn backend(&self) -> AccelerationBackend {
        AccelerationBackend::Wgpu
    }

    async fn enumerate_devices(&self) -> Result<Vec<Device>> {
        // TODO: enumerate wgpu adapters
        Ok(vec![])
    }

    async fn create_accelerator(&self, device: &Device) -> Result<Box<dyn crate::accelerator::Accelerator>> {
        Err(AccelerationError::UnsupportedOperation("WebGPU accelerator not yet implemented".to_string()))
    }
}

/// TPU backend (requires `tpu` feature).
#[cfg(feature = "tpu")]
pub struct TpuBackend;

#[cfg(feature = "tpu")]
impl TpuBackend {
    /// Creates a new TPU backend.
    pub async fn new() -> Result<Self> {
        // TODO: actual TPU initialization
        tracing::info!("TPU backend initialized");
        Ok(Self)
    }
}

#[cfg(feature = "tpu")]
#[async_trait]
impl AcceleratorBackend for TpuBackend {
    fn backend(&self) -> AccelerationBackend {
        AccelerationBackend::TfLite
    }

    async fn enumerate_devices(&self) -> Result<Vec<Device>> {
        // TODO: enumerate TPU devices
        Ok(vec![])
    }

    async fn create_accelerator(&self, device: &Device) -> Result<Box<dyn crate::accelerator::Accelerator>> {
        Err(AccelerationError::UnsupportedOperation("TPU accelerator not yet implemented".to_string()))
    }
}

/// Dummy backend (always available).
pub use crate::accelerator::DummyBackend;