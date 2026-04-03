//! Power monitoring (battery, CPU frequency, system power draw).

use crate::error::{Result, Error};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time;

/// Power source type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PowerSource {
    /// AC power (wall outlet).
    Ac,
    /// Battery (discharging).
    Battery,
    /// Unknown or other.
    Unknown,
}

/// Battery status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BatteryStatus {
    /// Battery is charging.
    Charging,
    /// Battery is discharging.
    Discharging,
    /// Battery is full.
    Full,
    /// Battery is not present.
    NotPresent,
    /// Unknown status.
    Unknown,
}

/// Power metrics collected from the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerMetrics {
    /// Current power source.
    pub source: PowerSource,
    /// Battery percentage (0‑100), if applicable.
    pub battery_percent: Option<f32>,
    /// Battery status.
    pub battery_status: BatteryStatus,
    /// Estimated remaining battery time in seconds (if discharging).
    pub battery_remaining_secs: Option<u64>,
    /// Current CPU frequency in MHz (average across cores).
    pub cpu_frequency_mhz: Option<u64>,
    /// Current CPU power draw in watts (if available).
    pub cpu_power_watts: Option<f32>,
    /// System total power draw in watts (if available).
    pub system_power_watts: Option<f32>,
    /// Timestamp when metrics were collected.
    pub timestamp: std::time::SystemTime,
}

/// Trait for platform‑specific power monitoring.
pub trait PowerMonitorBackend: Send + Sync {
    /// Collects current power metrics.
    fn collect(&self) -> Result<PowerMetrics>;

    /// Starts continuous monitoring (calls a callback at intervals).
    fn start_monitoring<F>(&self, interval: Duration, callback: F) -> Result<()>
    where
        F: Fn(PowerMetrics) + Send + 'static;
}

/// Default power monitor that uses available platform backends.
pub struct PowerMonitor {
    backend: Box<dyn PowerMonitorBackend>,
}

impl PowerMonitor {
    /// Creates a new power monitor, automatically selecting the best backend for the current platform.
    pub fn new() -> Result<Self> {
        #[cfg(target_os = "linux")]
        let backend = crate::monitor::linux::LinuxPowerMonitor::new()
            .map(|b| Box::new(b) as Box<dyn PowerMonitorBackend>)
            .map_err(|e| Error::PlatformError(e.to_string()))?;

        #[cfg(target_os = "windows")]
        let backend = crate::monitor::windows::WindowsPowerMonitor::new()
            .map(|b| Box::new(b) as Box<dyn PowerMonitorBackend>)
            .map_err(|e| Error::PlatformError(e.to_string()))?;

        #[cfg(target_os = "macos")]
        let backend = crate::monitor::macos::MacPowerMonitor::new()
            .map(|b| Box::new(b) as Box<dyn PowerMonitorBackend>)
            .map_err(|e| Error::PlatformError(e.to_string()))?;

        #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
        let backend = Box::new(DummyPowerMonitor) as Box<dyn PowerMonitorBackend>;

        Ok(Self { backend })
    }

    /// Returns current power metrics.
    pub fn metrics(&self) -> Result<PowerMetrics> {
        self.backend.collect()
    }

    /// Starts background monitoring, spawning a task that calls the provided callback at intervals.
    pub fn start<F>(&self, interval: Duration, callback: F) -> Result<()>
    where
        F: Fn(PowerMetrics) + Send + 'static,
    {
        self.backend.start_monitoring(interval, callback)
    }
}

/// Dummy backend for platforms without power monitoring support.
pub struct DummyPowerMonitor;

impl PowerMonitorBackend for DummyPowerMonitor {
    fn collect(&self) -> Result<PowerMetrics> {
        Ok(PowerMetrics {
            source: PowerSource::Unknown,
            battery_percent: None,
            battery_status: BatteryStatus::NotPresent,
            battery_remaining_secs: None,
            cpu_frequency_mhz: None,
            cpu_power_watts: None,
            system_power_watts: None,
            timestamp: std::time::SystemTime::now(),
        })
    }

    fn start_monitoring<F>(&self, interval: Duration, callback: F) -> Result<()>
    where
        F: Fn(PowerMetrics) + Send + 'static,
    {
        // Spawn a dummy task that periodically calls the callback with dummy metrics.
        tokio::spawn(async move {
            let mut interval = time::interval(interval);
            loop {
                interval.tick().await;
                callback(PowerMetrics {
                    source: PowerSource::Unknown,
                    battery_percent: None,
                    battery_status: BatteryStatus::NotPresent,
                    battery_remaining_secs: None,
                    cpu_frequency_mhz: None,
                    cpu_power_watts: None,
                    system_power_watts: None,
                    timestamp: std::time::SystemTime::now(),
                });
            }
        });
        Ok(())
    }
}

/// Platform‑specific implementations (stubs for now).
#[cfg(target_os = "linux")]
pub mod linux {
    use super::*;

    pub struct LinuxPowerMonitor;

    impl LinuxPowerMonitor {
        pub fn new() -> Result<Self> {
            Ok(Self)
        }
    }

    impl PowerMonitorBackend for LinuxPowerMonitor {
        fn collect(&self) -> Result<PowerMetrics> {
            // TODO: read from /sys/class/power_supply, cpufreq, etc.
            Err(Error::PlatformError("Linux power monitoring not yet implemented".to_string()))
        }

        fn start_monitoring<F>(&self, _interval: Duration, _callback: F) -> Result<()>
        where
            F: Fn(PowerMetrics) + Send + 'static,
        {
            Err(Error::PlatformError("Linux monitoring not implemented".to_string()))
        }
    }
}

#[cfg(target_os = "windows")]
pub mod windows {
    use super::*;

    pub struct WindowsPowerMonitor;

    impl WindowsPowerMonitor {
        pub fn new() -> Result<Self> {
            Ok(Self)
        }
    }

    impl PowerMonitorBackend for WindowsPowerMonitor {
        fn collect(&self) -> Result<PowerMetrics> {
            // TODO: use Win32 API (GetSystemPowerStatus, etc.)
            Err(Error::PlatformError("Windows power monitoring not yet implemented".to_string()))
        }

        fn start_monitoring<F>(&self, _interval: Duration, _callback: F) -> Result<()>
        where
            F: Fn(PowerMetrics) + Send + 'static,
        {
            Err(Error::PlatformError("Windows monitoring not implemented".to_string()))
        }
    }
}

#[cfg(target_os = "macos")]
pub mod macos {
    use super::*;

    pub struct MacPowerMonitor;

    impl MacPowerMonitor {
        pub fn new() -> Result<Self> {
            Ok(Self)
        }
    }

    impl PowerMonitorBackend for MacPowerMonitor {
        fn collect(&self) -> Result<PowerMetrics> {
            // TODO: use IOKit
            Err(Error::PlatformError("macOS power monitoring not yet implemented".to_string()))
        }

        fn start_monitoring<F>(&self, _interval: Duration, _callback: F) -> Result<()>
        where
            F: Fn(PowerMetrics) + Send + 'static,
        {
            Err(Error::PlatformError("macOS monitoring not implemented".to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dummy_monitor() {
        let monitor = DummyPowerMonitor;
        let metrics = monitor.collect().unwrap();
        assert_eq!(metrics.source, PowerSource::Unknown);
        assert!(metrics.battery_percent.is_none());
    }

    #[tokio::test]
    async fn test_power_monitor_new() {
        let monitor = PowerMonitor::new();
        // Should succeed (dummy backend on unsupported platforms)
        assert!(monitor.is_ok());
    }
}