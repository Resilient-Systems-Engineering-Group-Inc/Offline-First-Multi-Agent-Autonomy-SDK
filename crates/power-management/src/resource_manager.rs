//! Energy‑aware resource management for edge devices.
//!
//! This module provides a unified manager that coordinates power monitoring,
//! policy application, and resource allocation to optimize energy efficiency
//! while meeting performance requirements.

use crate::error::{Result, Error};
use crate::monitor::{PowerMonitor, PowerMetrics, PowerSource};
use crate::policy::{PowerPolicyManager, PowerAction, PowerMode};
use crate::scheduler::{PowerAwareScheduler, PowerAwareTask, SchedulingDecision};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{self, Duration};

/// Resource type that can be managed for energy efficiency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceType {
    /// CPU cores.
    CpuCores,
    /// CPU frequency.
    CpuFrequency,
    /// GPU (hardware acceleration).
    Gpu,
    /// Memory bandwidth.
    MemoryBandwidth,
    /// Network interface.
    Network,
    /// Storage I/O.
    Storage,
    /// Display/screen.
    Display,
}

/// Current allocation of a resource.
#[derive(Debug, Clone)]
pub struct ResourceAllocation {
    /// Type of resource.
    pub resource_type: ResourceType,
    /// Current usage level (0.0–1.0).
    pub usage: f64,
    /// Maximum available capacity.
    pub capacity: f64,
    /// Power consumption in watts.
    pub power_watts: f64,
    /// Whether this resource can be scaled down.
    pub scalable: bool,
}

/// Configuration for energy‑aware resource management.
#[derive(Debug, Clone)]
pub struct EnergyAwareResourceManagerConfig {
    /// Update interval for power metrics (seconds).
    pub metrics_update_interval_secs: u64,
    /// Whether to enable automatic policy application.
    pub auto_apply_policies: bool,
    /// Whether to enable hardware acceleration by default.
    pub enable_hardware_acceleration: bool,
    /// Maximum CPU frequency in MHz (0 = no limit).
    pub max_cpu_frequency_mhz: u64,
    /// Minimum CPU frequency in MHz.
    pub min_cpu_frequency_mhz: u64,
    /// Target battery life in hours (for automatic tuning).
    pub target_battery_life_hours: Option<f64>,
    /// Power budget in watts (if known).
    pub power_budget_watts: Option<f64>,
}

impl Default for EnergyAwareResourceManagerConfig {
    fn default() -> Self {
        Self {
            metrics_update_interval_secs: 5,
            auto_apply_policies: true,
            enable_hardware_acceleration: true,
            max_cpu_frequency_mhz: 0, // no limit
            min_cpu_frequency_mhz: 800,
            target_battery_life_hours: None,
            power_budget_watts: None,
        }
    }
}

/// Main energy‑aware resource manager.
pub struct EnergyAwareResourceManager {
    /// Power monitor.
    monitor: Arc<PowerMonitor>,
    /// Policy manager.
    policy_manager: Arc<RwLock<PowerPolicyManager>>,
    /// Scheduler.
    scheduler: Arc<PowerAwareScheduler>,
    /// Current resource allocations.
    allocations: RwLock<HashMap<ResourceType, ResourceAllocation>>,
    /// Current power metrics (cached).
    current_metrics: RwLock<Option<PowerMetrics>>,
    /// Configuration.
    config: EnergyAwareResourceManagerConfig,
    /// Current power mode.
    current_mode: RwLock<PowerMode>,
}

impl EnergyAwareResourceManager {
    /// Creates a new energy‑aware resource manager with default configuration.
    pub fn new() -> Result<Self> {
        Self::with_config(EnergyAwareResourceManagerConfig::default())
    }

    /// Creates a new energy‑aware resource manager with custom configuration.
    pub fn with_config(config: EnergyAwareResourceManagerConfig) -> Result<Self> {
        let monitor = Arc::new(PowerMonitor::new()?);
        let policy_manager = Arc::new(RwLock::new(PowerPolicyManager::new()));
        let scheduler = Arc::new(PowerAwareScheduler::new());

        let mut allocations = HashMap::new();
        // Initialize default resource allocations
        allocations.insert(
            ResourceType::CpuCores,
            ResourceAllocation {
                resource_type: ResourceType::CpuCores,
                usage: 0.0,
                capacity: num_cpus::get() as f64,
                power_watts: 0.0,
                scalable: true,
            },
        );
        allocations.insert(
            ResourceType::CpuFrequency,
            ResourceAllocation {
                resource_type: ResourceType::CpuFrequency,
                usage: 0.0,
                capacity: if config.max_cpu_frequency_mhz > 0 {
                    config.max_cpu_frequency_mhz as f64
                } else {
                    4000.0 // assume 4 GHz max
                },
                power_watts: 0.0,
                scalable: true,
            },
        );
        allocations.insert(
            ResourceType::Gpu,
            ResourceAllocation {
                resource_type: ResourceType::Gpu,
                usage: 0.0,
                capacity: if config.enable_hardware_acceleration {
                    1.0 // available
                } else {
                    0.0 // disabled
                },
                power_watts: 0.0,
                scalable: true,
            },
        );

        Ok(Self {
            monitor,
            policy_manager,
            scheduler,
            allocations: RwLock::new(allocations),
            current_metrics: RwLock::new(None),
            config,
            current_mode: RwLock::new(PowerMode::Balanced),
        })
    }

    /// Starts the resource manager background loop.
    pub async fn start(&self) -> Result<()> {
        let monitor_clone = Arc::clone(&self.monitor);
        let metrics_rx = self.current_metrics.clone();
        let policy_manager_clone = Arc::clone(&self.policy_manager);
        let allocations_clone = self.allocations.clone();
        let config = self.config.clone();

        // Start metrics collection
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(config.metrics_update_interval_secs));
            loop {
                interval.tick().await;
                match monitor_clone.metrics() {
                    Ok(metrics) => {
                        let mut current = metrics_rx.write().await;
                        *current = Some(metrics.clone());

                        // Apply policies if enabled
                        if config.auto_apply_policies {
                            let mut manager = policy_manager_clone.write().await;
                            if let Some(policy) = manager.evaluate(&metrics) {
                                // Apply policy actions
                                let _ = manager.apply_best_policy(&metrics);
                                // Update resource allocations based on policy
                                let mut allocations = allocations_clone.write().await;
                                Self::apply_policy_to_allocations(policy, &mut allocations).await;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to collect power metrics: {}", e);
                    }
                }
            }
        });

        // Start the scheduler
        let scheduler_clone = Arc::clone(&self.scheduler);
        tokio::spawn(async move {
            let _ = scheduler_clone.run().await;
        });

        Ok(())
    }

    /// Applies a power policy to resource allocations.
    async fn apply_policy_to_allocations(
        policy: &crate::policy::PowerPolicy,
        allocations: &mut HashMap<ResourceType, ResourceAllocation>,
    ) {
        for action in &policy.actions {
            match action {
                PowerAction::CpuFrequencyLimit(limit_mhz) => {
                    if let Some(allocation) = allocations.get_mut(&ResourceType::CpuFrequency) {
                        if *limit_mhz > 0 {
                            allocation.capacity = *limit_mhz as f64;
                            allocation.usage = allocation.usage.min(allocation.capacity);
                        }
                    }
                }
                PowerAction::CpuCoresLimit(cores) => {
                    if let Some(allocation) = allocations.get_mut(&ResourceType::CpuCores) {
                        allocation.capacity = *cores as f64;
                        allocation.usage = allocation.usage.min(allocation.capacity);
                    }
                }
                PowerAction::HardwareAcceleration(enabled) => {
                    if let Some(allocation) = allocations.get_mut(&ResourceType::Gpu) {
                        allocation.capacity = if *enabled { 1.0 } else { 0.0 };
                        allocation.usage = 0.0;
                    }
                }
                PowerAction::ScreenBrightness(_) => {
                    // Display resource management not yet implemented
                }
                PowerAction::SleepTimeout(_) => {
                    // Sleep management not yet implemented
                }
                PowerAction::NetworkThrottling => {
                    // Network resource management not yet implemented
                }
                PowerAction::None => {}
            }
        }
    }

    /// Submits a task for energy‑aware execution.
    pub async fn submit_task(&self, task: PowerAwareTask) -> Result<()> {
        self.scheduler.submit(task).await
    }

    /// Gets the current power metrics.
    pub async fn current_metrics(&self) -> Option<PowerMetrics> {
        self.current_metrics.read().await.clone()
    }

    /// Gets the current resource allocations.
    pub async fn resource_allocations(&self) -> HashMap<ResourceType, ResourceAllocation> {
        self.allocations.read().await.clone()
    }

    /// Adjusts a resource allocation manually.
    pub async fn adjust_resource(
        &self,
        resource_type: ResourceType,
        new_capacity: f64,
    ) -> Result<()> {
        let mut allocations = self.allocations.write().await;
        if let Some(allocation) = allocations.get_mut(&resource_type) {
            if !allocation.scalable {
                return Err(Error::Generic(format!(
                    "Resource {:?} is not scalable",
                    resource_type
                )));
            }
            allocation.capacity = new_capacity;
            allocation.usage = allocation.usage.min(new_capacity);
            Ok(())
        } else {
            Err(Error::Generic(format!(
                "Resource {:?} not found",
                resource_type
            )))
        }
    }

    /// Estimates the energy consumption of a task.
    pub async fn estimate_task_energy(&self, task: &PowerAwareTask) -> f64 {
        // Simple estimation based on task duration and resource requirements
        let base_power_w = 5.0; // baseline power consumption
        let acceleration_multiplier = if task.requires_acceleration { 2.0 } else { 1.0 };
        
        task.estimated_energy_joules.unwrap_or_else(|| {
            base_power_w * task.estimated_duration_secs * acceleration_multiplier
        })
    }

    /// Checks if there's enough energy budget to execute a task.
    pub async fn can_execute_task(&self, task: &PowerAwareTask) -> bool {
        let metrics = self.current_metrics.read().await;
        let metrics = match metrics.as_ref() {
            Some(m) => m,
            None => return true, // no metrics, assume OK
        };

        // Check battery level
        if let Some(battery) = metrics.battery_percent {
            if battery < 5.0 && !task.deferrable {
                return false;
            }
            if battery < 10.0 && task.deferrable {
                return false;
            }
        }

        // Check power source
        match metrics.source {
            PowerSource::Ac => true, // unlimited power
            PowerSource::Battery => {
                // Estimate remaining energy from battery remaining time and system power
                if let (Some(remaining_secs), Some(power_watts)) = (
                    metrics.battery_remaining_secs,
                    metrics.system_power_watts,
                ) {
                    let remaining_joules = remaining_secs as f64 * power_watts as f64;
                    let task_energy = self.estimate_task_energy(task).await;
                    if task_energy > remaining_joules * 0.1 {
                        // Task would consume more than 10% of remaining energy
                        return false;
                    }
                }
                true
            }
            PowerSource::Unknown => true,
        }
    }

    /// Gets the recommended power mode based on current conditions.
    pub async fn recommended_power_mode(&self) -> PowerMode {
        let metrics = self.current_metrics.read().await;
        let metrics = match metrics.as_ref() {
            Some(m) => m,
            None => return PowerMode::Balanced,
        };

        match metrics.source {
            PowerSource::Ac => PowerMode::Performance,
            PowerSource::Battery => {
                if let Some(battery) = metrics.battery_percent {
                    if battery > 50.0 {
                        PowerMode::Balanced
                    } else if battery > 20.0 {
                        PowerMode::Balanced
                    } else {
                        PowerMode::PowerSaver
                    }
                } else {
                    PowerMode::Balanced
                }
            }
            PowerSource::Unknown => PowerMode::Balanced,
        }
    }

    /// Sets the current power mode.
    pub async fn set_power_mode(&self, mode: PowerMode) -> Result<()> {
        let mut current = self.current_mode.write().await;
        *current = mode;

        // Apply mode-specific adjustments
        let mut allocations = self.allocations.write().await;
        match mode {
            PowerMode::Performance => {
                if let Some(allocation) = allocations.get_mut(&ResourceType::CpuFrequency) {
                    allocation.capacity = 4000.0; // 4 GHz
                }
                if let Some(allocation) = allocations.get_mut(&ResourceType::Gpu) {
                    allocation.capacity = 1.0; // enable GPU
                }
            }
            PowerMode::Balanced => {
                if let Some(allocation) = allocations.get_mut(&ResourceType::CpuFrequency) {
                    allocation.capacity = 2000.0; // 2 GHz
                }
                if let Some(allocation) = allocations.get_mut(&ResourceType::Gpu) {
                    allocation.capacity = 1.0; // enable GPU
                }
            }
            PowerMode::PowerSaver => {
                if let Some(allocation) = allocations.get_mut(&ResourceType::CpuFrequency) {
                    allocation.capacity = 1000.0; // 1 GHz
                }
                if let Some(allocation) = allocations.get_mut(&ResourceType::Gpu) {
                    allocation.capacity = 0.0; // disable GPU
                }
                if let Some(allocation) = allocations.get_mut(&ResourceType::CpuCores) {
                    allocation.capacity = (num_cpus::get() / 2).max(1) as f64;
                }
            }
            PowerMode::Custom => {
                // No automatic adjustments for custom mode
            }
        }

        Ok(())
    }

    /// Gets the current power mode.
    pub async fn current_power_mode(&self) -> PowerMode {
        *self.current_mode.read().await
    }

    /// Calculates the estimated remaining battery life based on current usage.
    pub async fn estimated_battery_life(&self) -> Option<f64> {
        let metrics = self.current_metrics.read().await;
        let metrics = metrics.as_ref()?;

        // Use battery_remaining_secs if available
        if let Some(remaining_secs) = metrics.battery_remaining_secs {
            return Some(remaining_secs as f64 / 3600.0);
        }

        // Otherwise estimate from battery percentage and system power
        if let (Some(battery_percent), Some(system_power_w)) = (
            metrics.battery_percent,
            metrics.system_power_watts,
        ) {
            if system_power_w > 0.0 {
                // Very rough estimate: assume linear discharge
                // This is highly inaccurate but better than nothing
                let remaining_energy_ratio = battery_percent as f64 / 100.0;
                let estimated_total_life_hours = 8.0; // typical laptop battery
                return Some(estimated_total_life_hours * remaining_energy_ratio);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_manager_creation() {
        let manager = EnergyAwareResourceManager::new();
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_resource_allocations() {
        let manager = EnergyAwareResourceManager::new().unwrap();
        let allocations = manager.resource_allocations().await;
        
        assert!(allocations.contains_key(&ResourceType::CpuCores));
        assert!(allocations.contains_key(&ResourceType::CpuFrequency));
        assert!(allocations.contains_key(&ResourceType::Gpu));
    }

    #[tokio::test]
    async fn test_power_mode_transition() {
        let manager = EnergyAwareResourceManager::new().unwrap();
        
        manager.set_power_mode(PowerMode::PowerSaver).await.unwrap();
        assert_eq!(manager.current_power_mode().await, PowerMode::PowerSaver);
        
        manager.set_power_mode(PowerMode::Performance).await.unwrap();
        assert_eq!(manager.current_power_mode().await, PowerMode::Performance);
    }
}