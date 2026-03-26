//! Resource monitoring for agents.

use common::error::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Resource usage metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceMetrics {
    /// CPU usage as a percentage (0‑100).
    pub cpu_usage: f32,
    /// Memory usage in bytes.
    pub memory_used: u64,
    /// Total memory in bytes.
    pub memory_total: u64,
    /// Battery level as a percentage (0‑100), if available.
    pub battery_level: Option<f32>,
    /// Network throughput (bytes/sec) sent.
    pub network_tx: u64,
    /// Network throughput (bytes/sec) received.
    pub network_rx: u64,
    /// Disk usage (bytes) used.
    pub disk_used: u64,
    /// Disk total capacity (bytes).
    pub disk_total: u64,
}

impl Default for ResourceMetrics {
    fn default() -> Self {
        Self {
            cpu_usage: 0.0,
            memory_used: 0,
            memory_total: 1,
            battery_level: None,
            network_tx: 0,
            network_rx: 0,
            disk_used: 0,
            disk_total: 1,
        }
    }
}

/// A resource monitor that can collect metrics.
#[async_trait]
pub trait ResourceMonitor: Send + Sync {
    /// Collect current resource metrics.
    async fn collect(&mut self) -> Result<ResourceMetrics>;

    /// Start continuous monitoring (optional).
    async fn start_monitoring(&mut self, interval: Duration) -> Result<()>;

    /// Stop monitoring.
    async fn stop_monitoring(&mut self) -> Result<()>;
}

/// A monitor that uses the `sysinfo` crate to gather system metrics.
pub struct SysinfoMonitor {
    sys: sysinfo::System,
    refresh_kind: sysinfo::RefreshKind,
    monitoring: bool,
}

impl SysinfoMonitor {
    /// Create a new sysinfo‑based monitor.
    pub fn new() -> Self {
        let mut sys = sysinfo::System::new();
        sys.refresh_all(); // initial refresh
        Self {
            sys,
            refresh_kind: sysinfo::RefreshKind::everything(),
            monitoring: false,
        }
    }

    /// Refresh system information.
    fn refresh(&mut self) {
        self.sys.refresh_specifics(self.refresh_kind);
    }
}

impl Default for SysinfoMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ResourceMonitor for SysinfoMonitor {
    async fn collect(&mut self) -> Result<ResourceMetrics> {
        self.refresh();

        let cpu_usage = self.sys.global_cpu_info().cpu_usage();
        let memory_used = self.sys.used_memory();
        let memory_total = self.sys.total_memory();
        let battery_level = sysinfo::Battery::life_percentage(&self.sys).ok();

        // Network stats: we need to accumulate over time; for simplicity, we return zeros.
        let network_tx = 0;
        let network_rx = 0;

        // Disk stats: first disk
        let disk_used = self.sys.total_swap() - self.sys.free_swap(); // approximate
        let disk_total = self.sys.total_swap();

        Ok(ResourceMetrics {
            cpu_usage,
            memory_used,
            memory_total,
            battery_level,
            network_tx,
            network_rx,
            disk_used,
            disk_total,
        })
    }

    async fn start_monitoring(&mut self, _interval: Duration) -> Result<()> {
        self.monitoring = true;
        // In a real implementation, we would spawn a background task.
        Ok(())
    }

    async fn stop_monitoring(&mut self) -> Result<()> {
        self.monitoring = false;
        Ok(())
    }
}

/// A dummy monitor that returns static values (for testing).
pub struct DummyMonitor;

#[async_trait]
impl ResourceMonitor for DummyMonitor {
    async fn collect(&mut self) -> Result<ResourceMetrics> {
        Ok(ResourceMetrics {
            cpu_usage: 25.0,
            memory_used: 2 * 1024 * 1024 * 1024, // 2 GB
            memory_total: 16 * 1024 * 1024 * 1024, // 16 GB
            battery_level: Some(80.0),
            network_tx: 1000,
            network_rx: 2000,
            disk_used: 50 * 1024 * 1024 * 1024, // 50 GB
            disk_total: 500 * 1024 * 1024 * 1024, // 500 GB
        })
    }

    async fn start_monitoring(&mut self, _interval: Duration) -> Result<()> {
        Ok(())
    }

    async fn stop_monitoring(&mut self) -> Result<()> {
        Ok(())
    }
}