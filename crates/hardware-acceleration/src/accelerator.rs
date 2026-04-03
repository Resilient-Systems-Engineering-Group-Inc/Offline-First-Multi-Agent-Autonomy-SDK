//! Main accelerator abstraction.

use crate::device::{AccelerationBackend, Device, DeviceType};
use crate::error::{Result, AccelerationError};
use crate::kernel::Kernel;
use crate::memory::MemoryBuffer;
use async_trait::async_trait;
use std::sync::Arc;

/// Trait for hardware acceleration backends.
#[async_trait]
pub trait AcceleratorBackend: Send + Sync {
    /// Returns the backend type.
    fn backend(&self) -> AccelerationBackend;

    /// Returns a list of available devices.
    async fn enumerate_devices(&self) -> Result<Vec<Device>>;

    /// Creates a new accelerator instance for the given device.
    async fn create_accelerator(&self, device: &Device) -> Result<Box<dyn Accelerator>>;
}

/// Trait for an accelerator instance (attached to a specific device).
#[async_trait]
pub trait Accelerator: Send + Sync {
    /// Returns the underlying device.
    fn device(&self) -> &Device;

    /// Allocates a memory buffer on the device.
    async fn allocate_buffer(&self, size: usize) -> Result<MemoryBuffer>;

    /// Copies data from host to device.
    async fn copy_to_device(&self, buffer: &MemoryBuffer, data: &[u8]) -> Result<()>;

    /// Copies data from device to host.
    async fn copy_from_device(&self, buffer: &MemoryBuffer) -> Result<Vec<u8>>;

    /// Compiles a kernel from source (backend‑specific).
    async fn compile_kernel(&self, source: &str, entry_point: &str) -> Result<Box<dyn Kernel>>;

    /// Executes a kernel with given arguments.
    async fn execute_kernel(
        &self,
        kernel: &dyn Kernel,
        work_size: (usize, usize, usize),
        args: &[&dyn std::any::Any],
    ) -> Result<()>;

    /// Synchronizes all operations (blocks until device is idle).
    async fn synchronize(&self) -> Result<()>;
}

/// A generic accelerator wrapper that delegates to a backend.
pub struct GenericAccelerator {
    device: Device,
    backend: Arc<dyn AcceleratorBackend>,
    inner: Box<dyn Accelerator>,
}

impl GenericAccelerator {
    /// Creates a new accelerator by selecting the first available device of the given type.
    pub async fn new(
        backend: Arc<dyn AcceleratorBackend>,
        device_type: DeviceType,
    ) -> Result<Self> {
        let devices = backend.enumerate_devices().await?;
        let device = devices
            .into_iter()
            .find(|d| d.device_type == device_type && d.available)
            .ok_or_else(|| {
                AccelerationError::NoHardware(format!("No available {:?} device", device_type))
            })?;
        let inner = backend.create_accelerator(&device).await?;
        Ok(Self {
            device,
            backend,
            inner,
        })
    }

    /// Returns the device.
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// Returns the backend.
    pub fn backend(&self) -> &dyn AcceleratorBackend {
        self.backend.as_ref()
    }

    /// Returns the inner accelerator (for low‑level operations).
    pub fn inner(&self) -> &dyn Accelerator {
        self.inner.as_ref()
    }
}

#[async_trait]
impl Accelerator for GenericAccelerator {
    fn device(&self) -> &Device {
        self.inner.device()
    }

    async fn allocate_buffer(&self, size: usize) -> Result<MemoryBuffer> {
        self.inner.allocate_buffer(size).await
    }

    async fn copy_to_device(&self, buffer: &MemoryBuffer, data: &[u8]) -> Result<()> {
        self.inner.copy_to_device(buffer, data).await
    }

    async fn copy_from_device(&self, buffer: &MemoryBuffer) -> Result<Vec<u8>> {
        self.inner.copy_from_device(buffer).await
    }

    async fn compile_kernel(&self, source: &str, entry_point: &str) -> Result<Box<dyn Kernel>> {
        self.inner.compile_kernel(source, entry_point).await
    }

    async fn execute_kernel(
        &self,
        kernel: &dyn Kernel,
        work_size: (usize, usize, usize),
        args: &[&dyn std::any::Any],
    ) -> Result<()> {
        self.inner.execute_kernel(kernel, work_size, args).await
    }

    async fn synchronize(&self) -> Result<()> {
        self.inner.synchronize().await
    }
}

/// Dummy backend for testing.
pub struct DummyBackend;

#[async_trait]
impl AcceleratorBackend for DummyBackend {
    fn backend(&self) -> AccelerationBackend {
        AccelerationBackend::Custom
    }

    async fn enumerate_devices(&self) -> Result<Vec<Device>> {
        Ok(vec![Device::new(
            "dummy",
            "Dummy Device",
            DeviceType::Cpu,
            AccelerationBackend::Custom,
        )])
    }

    async fn create_accelerator(&self, device: &Device) -> Result<Box<dyn Accelerator>> {
        Ok(Box::new(DummyAccelerator {
            device: device.clone(),
        }))
    }
}

/// Dummy accelerator that does nothing.
pub struct DummyAccelerator {
    device: Device,
}

#[async_trait]
impl Accelerator for DummyAccelerator {
    fn device(&self) -> &Device {
        &self.device
    }

    async fn allocate_buffer(&self, _size: usize) -> Result<MemoryBuffer> {
        Err(AccelerationError::Unsupported("Dummy accelerator".to_string()))
    }

    async fn copy_to_device(&self, _buffer: &MemoryBuffer, _data: &[u8]) -> Result<()> {
        Ok(())
    }

    async fn copy_from_device(&self, _buffer: &MemoryBuffer) -> Result<Vec<u8>> {
        Ok(vec![])
    }

    async fn compile_kernel(&self, _source: &str, _entry_point: &str) -> Result<Box<dyn Kernel>> {
        Err(AccelerationError::Unsupported("Dummy accelerator".to_string()))
    }

    async fn execute_kernel(
        &self,
        _kernel: &dyn Kernel,
        _work_size: (usize, usize, usize),
        _args: &[&dyn std::any::Any],
    ) -> Result<()> {
        Ok(())
    }

    async fn synchronize(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dummy_backend() {
        let backend = Arc::new(DummyBackend);
        let devices = backend.enumerate_devices().await.unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].name, "Dummy Device");
    }
}