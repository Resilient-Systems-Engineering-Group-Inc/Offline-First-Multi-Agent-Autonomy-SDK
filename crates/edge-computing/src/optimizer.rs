//! Edge‑aware configuration optimizer.

use crate::hardware::Capabilities;
use crate::error::Error;
use configuration::Configuration;

/// Edge optimizer adjusts SDK configuration for edge devices.
pub struct EdgeOptimizer {
    capabilities: Capabilities,
}

impl EdgeOptimizer {
    /// Create a new optimizer with detected hardware.
    pub fn new() -> Result<Self, Error> {
        let capabilities = Capabilities::detect()?;
        Ok(Self { capabilities })
    }

    /// Optimize a configuration for the current edge device.
    pub fn optimize(&self, config: &mut Configuration) {
        // Reduce memory usage on low‑memory devices.
        if self.capabilities.total_memory_mb < 2048 {
            config.agent.max_concurrent_tasks = 5;
            config.state_sync.sync_interval_secs = 5;
            config.resource_monitor.collection_interval_secs = 30;
        }

        // Adjust mesh backend based on platform.
        match self.capabilities.platform {
            crate::hardware::Platform::RaspberryPi => {
                // Prefer in‑memory or lightweight backend.
                if config.mesh.backend == "libp2p" {
                    config.mesh.backend = "in_memory".to_string();
                }
            }
            crate::hardware::Platform::NvidiaJetson => {
                // Jetson can handle more; enable GPU‑accelerated tasks.
                config.planning.planner_type = "distributed".to_string();
            }
            _ => {}
        }

        // Disable heavy features on low‑end hardware.
        if self.capabilities.cpu_cores <= 2 {
            config.state_sync.delta_compression = false;
            config.mesh.enable_encryption = false;
        }

        // Log the optimizations.
        tracing::info!(
            "Edge optimization applied for platform: {:?}",
            self.capabilities.platform
        );
    }
}