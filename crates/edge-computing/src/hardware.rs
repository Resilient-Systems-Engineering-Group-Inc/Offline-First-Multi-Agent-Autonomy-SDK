//! Hardware detection and capabilities.

use crate::error::Error;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Hardware platform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Platform {
    /// Raspberry Pi (any model)
    RaspberryPi,
    /// NVIDIA Jetson (any series)
    NvidiaJetson,
    /// Generic x86_64
    X86_64,
    /// Generic ARM (unknown)
    Arm,
    /// Unknown platform
    Unknown,
}

impl Platform {
    /// Detect the current platform.
    pub fn detect() -> Self {
        #[cfg(target_arch = "x86_64")]
        {
            // Check for Raspberry Pi via /proc/device-tree/model
            if let Ok(model) = std::fs::read_to_string("/proc/device-tree/model") {
                if model.contains("Raspberry Pi") {
                    return Self::RaspberryPi;
                }
            }
            // Check for Jetson via /proc/device-tree/compatible
            if let Ok(compatible) = std::fs::read_to_string("/proc/device-tree/compatible") {
                if compatible.contains("nvidia,tegra") {
                    return Self::NvidiaJetson;
                }
            }
            Self::X86_64
        }
        #[cfg(target_arch = "arm")]
        {
            if let Ok(model) = std::fs::read_to_string("/proc/device-tree/model") {
                if model.contains("Raspberry Pi") {
                    return Self::RaspberryPi;
                }
                if model.contains("NVIDIA Jetson") {
                    return Self::NvidiaJetson;
                }
            }
            Self::Arm
        }
        #[cfg(not(any(target_arch = "x86_64", target_arch = "arm")))]
        {
            Self::Unknown
        }
    }
}

/// Hardware capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    /// Platform.
    pub platform: Platform,
    /// CPU cores.
    pub cpu_cores: usize,
    /// Total memory in MB.
    pub total_memory_mb: u64,
    /// Total storage in GB.
    pub total_storage_gb: u64,
    /// GPU present.
    pub has_gpu: bool,
    /// GPIO available.
    pub has_gpio: bool,
}

impl Capabilities {
    /// Detect hardware capabilities.
    pub fn detect() -> Result<Self, Error> {
        let platform = Platform::detect();

        // Simple detection; in a real implementation you would read from /proc/cpuinfo etc.
        let cpu_cores = num_cpus::get();
        let total_memory_mb = sys_info::mem_info()
            .map(|info| info.total)
            .unwrap_or(1024); // fallback 1GB
        let total_storage_gb = 8; // dummy

        let has_gpu = match platform {
            Platform::NvidiaJetson => true,
            _ => false,
        };

        let has_gpio = match platform {
            Platform::RaspberryPi | Platform::NvidiaJetson => true,
            _ => false,
        };

        Ok(Self {
            platform,
            cpu_cores,
            total_memory_mb,
            total_storage_gb,
            has_gpu,
            has_gpio,
        })
    }
}