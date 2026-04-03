//! Device abstraction for hardware accelerators.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Type of hardware device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeviceType {
    /// Central Processing Unit (general‑purpose).
    Cpu,
    /// Graphics Processing Unit.
    Gpu,
    /// Tensor Processing Unit.
    Tpu,
    /// Field‑Programmable Gate Array.
    Fpga,
    /// Neural Processing Unit.
    Npu,
    /// Vision Processing Unit.
    Vpu,
    /// Unknown or custom device.
    Other,
}

impl std::fmt::Display for DeviceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeviceType::Cpu => write!(f, "CPU"),
            DeviceType::Gpu => write!(f, "GPU"),
            DeviceType::Tpu => write!(f, "TPU"),
            DeviceType::Fpga => write!(f, "FPGA"),
            DeviceType::Npu => write!(f, "NPU"),
            DeviceType::Vpu => write!(f, "VPU"),
            DeviceType::Other => write!(f, "Other"),
        }
    }
}

/// Backend used to communicate with the device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AccelerationBackend {
    /// OpenCL (cross‑platform GPU/CPU/FPGA).
    OpenCl,
    /// CUDA (NVIDIA GPUs).
    Cuda,
    /// Vulkan (cross‑platform GPU).
    Vulkan,
    /// Metal (Apple GPU).
    Metal,
    /// WebGPU (cross‑platform web‑native).
    Wgpu,
    /// TensorFlow Lite (for TPU/Edge).
    TfLite,
    /// PyTorch (via libtorch).
    Torch,
    /// ONNX Runtime.
    Onnx,
    /// Custom or unknown backend.
    Custom,
}

impl std::fmt::Display for AccelerationBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AccelerationBackend::OpenCl => write!(f, "OpenCL"),
            AccelerationBackend::Cuda => write!(f, "CUDA"),
            AccelerationBackend::Vulkan => write!(f, "Vulkan"),
            AccelerationBackend::Metal => write!(f, "Metal"),
            AccelerationBackend::Wgpu => write!(f, "WebGPU"),
            AccelerationBackend::TfLite => write!(f, "TensorFlow Lite"),
            AccelerationBackend::Torch => write!(f, "PyTorch"),
            AccelerationBackend::Onnx => write!(f, "ONNX Runtime"),
            AccelerationBackend::Custom => write!(f, "Custom"),
        }
    }
}

/// Represents a physical or logical hardware device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    /// Unique identifier (backend‑specific).
    pub id: String,
    /// Human‑readable name.
    pub name: String,
    /// Device type.
    pub device_type: DeviceType,
    /// Backend used to access this device.
    pub backend: AccelerationBackend,
    /// Total memory in bytes (if known).
    pub total_memory: Option<u64>,
    /// Compute capability (e.g., CUDA capability, OpenCL version).
    pub capability: String,
    /// Whether the device is currently available.
    pub available: bool,
    /// Additional vendor‑specific properties.
    pub properties: HashMap<String, String>,
}

impl Device {
    /// Creates a new device description.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        device_type: DeviceType,
        backend: AccelerationBackend,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            device_type,
            backend,
            total_memory: None,
            capability: String::new(),
            available: true,
            properties: HashMap::new(),
        }
    }

    /// Sets the total memory.
    pub fn with_total_memory(mut self, bytes: u64) -> Self {
        self.total_memory = Some(bytes);
        self
    }

    /// Sets the compute capability.
    pub fn with_capability(mut self, capability: impl Into<String>) -> Self {
        self.capability = capability.into();
        self
    }

    /// Adds a custom property.
    pub fn with_property(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.properties.insert(key.into(), value.into());
        self
    }

    /// Returns a short description of the device.
    pub fn description(&self) -> String {
        format!("{} ({}) via {}", self.name, self.device_type, self.backend)
    }

    /// Checks whether the device supports a certain feature (by property).
    pub fn supports(&self, feature: &str) -> bool {
        self.properties
            .get(feature)
            .map(|v| v == "true" || v == "1" || v == "yes")
            .unwrap_or(false)
    }
}

/// A list of devices discovered on the system.
#[derive(Debug, Clone, Default)]
pub struct DeviceList {
    devices: Vec<Device>,
}

impl DeviceList {
    /// Creates an empty device list.
    pub fn new() -> Self {
        Self { devices: Vec::new() }
    }

    /// Adds a device to the list.
    pub fn add(&mut self, device: Device) {
        self.devices.push(device);
    }

    /// Returns all devices.
    pub fn all(&self) -> &[Device] {
        &self.devices
    }

    /// Returns devices of a specific type.
    pub fn by_type(&self, device_type: DeviceType) -> Vec<&Device> {
        self.devices
            .iter()
            .filter(|d| d.device_type == device_type)
            .collect()
    }

    /// Returns devices using a specific backend.
    pub fn by_backend(&self, backend: AccelerationBackend) -> Vec<&Device> {
        self.devices
            .iter()
            .filter(|d| d.backend == backend)
            .collect()
    }

    /// Returns the first available device of the given type, if any.
    pub fn first_available(&self, device_type: DeviceType) -> Option<&Device> {
        self.devices
            .iter()
            .find(|d| d.device_type == device_type && d.available)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_creation() {
        let device = Device::new("gpu0", "NVIDIA GeForce RTX 4090", DeviceType::Gpu, AccelerationBackend::Cuda)
            .with_total_memory(24 * 1024 * 1024 * 1024)
            .with_capability("8.9")
            .with_property("cuda_cores", "16384");
        assert_eq!(device.device_type, DeviceType::Gpu);
        assert_eq!(device.backend, AccelerationBackend::Cuda);
        assert!(device.total_memory.unwrap() > 0);
        assert!(device.supports("cuda_cores"));
    }

    #[test]
    fn test_device_list() {
        let mut list = DeviceList::new();
        list.add(Device::new("cpu0", "Intel Xeon", DeviceType::Cpu, AccelerationBackend::OpenCl));
        list.add(Device::new("gpu0", "AMD Radeon", DeviceType::Gpu, AccelerationBackend::OpenCl));
        assert_eq!(list.all().len(), 2);
        let gpus = list.by_type(DeviceType::Gpu);
        assert_eq!(gpus.len(), 1);
        assert_eq!(gpus[0].name, "AMD Radeon");
    }
}