//! Power‑aware task scheduling.

use crate::error::{Result, Error};
use crate::monitor::PowerMetrics;
use crate::policy::{PowerAction, PowerPolicyManager};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{self, Duration};

/// Task with power‑aware metadata.
#[derive(Debug, Clone)]
pub struct PowerAwareTask {
    /// Unique task identifier.
    pub id: String,
    /// Task description.
    pub description: String,
    /// Estimated energy consumption in joules (if known).
    pub estimated_energy_joules: Option<f64>,
    /// Estimated execution time in seconds.
    pub estimated_duration_secs: f64,
    /// Priority (higher = more important).
    pub priority: i32,
    /// Whether the task can be delayed when power is low.
    pub deferrable: bool,
    /// Whether the task requires hardware acceleration.
    pub requires_acceleration: bool,
    /// Task payload (opaque).
    pub payload: Vec<u8>,
}

/// Scheduling decision.
#[derive(Debug, Clone)]
pub enum SchedulingDecision {
    /// Execute now.
    ExecuteNow,
    /// Defer until later (specify suggested delay in seconds).
    Defer(f64),
    /// Reject (cannot be executed under current power constraints).
    Reject,
    /// Execute with reduced capabilities (e.g., without hardware acceleration).
    ExecuteReduced,
}

/// Power‑aware scheduler.
pub struct PowerAwareScheduler {
    /// Policy manager.
    policy_manager: Arc<RwLock<PowerPolicyManager>>,
    /// Task queue.
    task_queue: Mutex<VecDeque<PowerAwareTask>>,
    /// Current power metrics (cached).
    current_metrics: RwLock<Option<PowerMetrics>>,
    /// Scheduler configuration.
    config: SchedulerConfig,
}

/// Scheduler configuration.
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// Update interval for power metrics (seconds).
    pub metrics_update_interval_secs: u64,
    /// Whether to enable automatic policy application.
    pub auto_apply_policies: bool,
    /// Maximum deferred task queue length.
    pub max_deferred_tasks: usize,
    /// Energy threshold (joules) below which deferrable tasks are deferred.
    pub low_energy_threshold_joules: Option<f64>,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            metrics_update_interval_secs: 10,
            auto_apply_policies: true,
            max_deferred_tasks: 100,
            low_energy_threshold_joules: None,
        }
    }
}

impl PowerAwareScheduler {
    /// Creates a new scheduler with default configuration.
    pub fn new() -> Self {
        Self::with_config(SchedulerConfig::default())
    }

    /// Creates a new scheduler with custom configuration.
    pub fn with_config(config: SchedulerConfig) -> Self {
        Self {
            policy_manager: Arc::new(RwLock::new(PowerPolicyManager::new())),
            task_queue: Mutex::new(VecDeque::new()),
            current_metrics: RwLock::new(None),
            config,
        }
    }

    /// Updates the current power metrics.
    pub async fn update_metrics(&self, metrics: PowerMetrics) {
        let mut current = self.current_metrics.write().await;
        *current = Some(metrics);
    }

    /// Submits a task for scheduling.
    pub async fn submit(&self, task: PowerAwareTask) -> Result<()> {
        let mut queue = self.task_queue.lock().await;
        if queue.len() >= self.config.max_deferred_tasks {
            return Err(Error::SchedulingError("Task queue full".to_string()));
        }
        queue.push_back(task);
        Ok(())
    }

    /// Makes a scheduling decision for the next task.
    pub async fn decide(&self) -> Result<Option<(PowerAwareTask, SchedulingDecision)>> {
        let metrics = self.current_metrics.read().await.clone();
        let metrics = match metrics {
            Some(m) => m,
            None => return Ok(None), // no metrics yet, cannot decide
        };

        let mut queue = self.task_queue.lock().await;
        if queue.is_empty() {
            return Ok(None);
        }

        // For simplicity, we examine the first task.
        let task = queue.pop_front().unwrap();
        let decision = self.evaluate_task(&task, &metrics).await;
        Ok(Some((task, decision)))
    }

    /// Evaluates a single task against current power metrics.
    async fn evaluate_task(&self, task: &PowerAwareTask, metrics: &PowerMetrics) -> SchedulingDecision {
        // Apply policy actions if auto‑apply is enabled.
        if self.config.auto_apply_policies {
            let mut manager = self.policy_manager.write().await;
            let _ = manager.apply_best_policy(metrics);
        }

        // Check battery level.
        if let Some(battery) = metrics.battery_percent {
            if battery < 10.0 && task.deferrable {
                return SchedulingDecision::Defer(300.0); // defer 5 minutes
            }
            if battery < 5.0 && !task.deferrable {
                return SchedulingDecision::Reject;
            }
        }

        // Check if hardware acceleration is required but not available.
        if task.requires_acceleration {
            // For now, assume acceleration is available.
            // In a real implementation, we would check hardware status.
        }

        // Check energy threshold.
        if let Some(threshold) = self.config.low_energy_threshold_joules {
            if let Some(energy) = task.estimated_energy_joules {
                if energy > threshold && task.deferrable {
                    return SchedulingDecision::Defer(60.0);
                }
            }
        }

        SchedulingDecision::ExecuteNow
    }

    /// Starts the scheduler background loop.
    pub async fn run(&self) -> Result<()> {
        let mut interval = time::interval(Duration::from_secs(self.config.metrics_update_interval_secs));
        loop {
            interval.tick().await;
            // In a real implementation, we would collect metrics here.
            // For now, we just process any pending decisions.
            if let Some((task, decision)) = self.decide().await? {
                self.handle_decision(task, decision).await?;
            }
        }
    }

    /// Handles a scheduling decision (e.g., executes or defers the task).
    async fn handle_decision(&self, task: PowerAwareTask, decision: SchedulingDecision) -> Result<()> {
        match decision {
            SchedulingDecision::ExecuteNow => {
                tracing::info!("Executing task {} now", task.id);
                // TODO: actually execute the task
            }
            SchedulingDecision::Defer(delay_secs) => {
                tracing::info!("Deferring task {} by {} seconds", task.id, delay_secs);
                // Re‑enqueue the task after delay.
                let self_clone = self.clone();
                let task_clone = task.clone();
                tokio::spawn(async move {
                    time::sleep(Duration::from_secs_f64(delay_secs)).await;
                    let _ = self_clone.submit(task_clone).await;
                });
            }
            SchedulingDecision::Reject => {
                tracing::warn!("Rejecting task {} due to power constraints", task.id);
            }
            SchedulingDecision::ExecuteReduced => {
                tracing::info!("Executing task {} with reduced capabilities", task.id);
                // TODO: execute with reduced capabilities
            }
        }
        Ok(())
    }

    /// Returns the number of pending tasks.
    pub async fn pending_tasks(&self) -> usize {
        let queue = self.task_queue.lock().await;
        queue.len()
    }
}

// Implement Clone for PowerAwareScheduler (needed for spawning).
impl Clone for PowerAwareScheduler {
    fn clone(&self) -> Self {
        Self {
            policy_manager: self.policy_manager.clone(),
            task_queue: Mutex::new(VecDeque::new()),
            current_metrics: RwLock::new(None),
            config: self.config.clone(),
        }
    }
}

/// Utility function to create a simple power‑aware task.
pub fn simple_task(id: &str, duration_secs: f64, deferrable: bool) -> PowerAwareTask {
    PowerAwareTask {
        id: id.to_string(),
        description: format!("Task {}", id),
        estimated_energy_joules: None,
        estimated_duration_secs: duration_secs,
        priority: 0,
        deferrable,
        requires_acceleration: false,
        payload: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitor::{PowerMetrics, PowerSource, BatteryStatus};

    fn sample_metrics(battery_percent: Option<f32>) -> PowerMetrics {
        PowerMetrics {
            source: PowerSource::Battery,
            battery_percent,
            battery_status: BatteryStatus::Discharging,
            battery_remaining_secs: None,
            cpu_frequency_mhz: None,
            cpu_power_watts: None,
            system_power_watts: None,
            timestamp: std::time::SystemTime::now(),
        }
    }

    #[tokio::test]
    async fn test_scheduler_submit() {
        let scheduler = PowerAwareScheduler::new();
        let task = simple_task("test1", 5.0, true);
        assert!(scheduler.submit(task).await.is_ok());
        assert_eq!(scheduler.pending_tasks().await, 1);
    }

    #[tokio::test]
    async fn test_scheduler_decide_with_low_battery() {
        let scheduler = PowerAwareScheduler::new();
        scheduler.update_metrics(sample_metrics(Some(8.0))).await;
        let task = simple_task("test2", 5.0, true);
        scheduler.submit(task).await.unwrap();
        let decision = scheduler.decide().await.unwrap();
        assert!(decision.is_some());
        let (_, decision) = decision.unwrap();
        match decision {
            SchedulingDecision::Defer(_) => {} // expected
            _ => panic!("Expected Defer decision"),
        }
    }

    #[tokio::test]
    async fn test_scheduler_decide_with_high_battery() {
        let scheduler = PowerAwareScheduler::new();
        scheduler.update_metrics(sample_metrics(Some(80.0))).await;
        let task = simple_task("test3", 5.0, false);
        scheduler.submit(task).await.unwrap();
        let decision = scheduler.decide().await.unwrap();
        assert!(decision.is_some());
        let (_, decision) = decision.unwrap();
        match decision {
            SchedulingDecision::ExecuteNow => {} // expected
            _ => panic!("Expected ExecuteNow decision"),
        }
    }
}