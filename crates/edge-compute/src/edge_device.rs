//! Edge device management.

use crate::{ConnectivityConfig, EdgeTask, ResourceLimits, ResourceRequirements};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

/// Edge device representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeDevice {
    pub id: String,
    pub name: String,
    pub capabilities: Vec<String>,
    pub resources: DeviceResources,
    pub connectivity: ConnectivityConfig,
    pub active_tasks: Vec<EdgeTask>,
    pub task_history: Vec<TaskResult>,
    pub metadata: HashMap<String, String>,
    pub created_at: u64,
    pub last_heartbeat: u64,
}

/// Device resource usage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceResources {
    pub cpu_percent: f64,
    pub memory_percent: f64,
    pub storage_used_mb: u64,
    pub storage_total_mb: u64,
    pub network_rx_mbps: f64,
    pub network_tx_mbps: f64,
    pub battery_percent: Option<f64>,
}

impl Default for DeviceResources {
    fn default() -> Self {
        Self {
            cpu_percent: 0.0,
            memory_percent: 0.0,
            storage_used_mb: 0,
            storage_total_mb: 1024,
            network_rx_mbps: 0.0,
            network_tx_mbps: 0.0,
            battery_percent: None,
        }
    }
}

/// Task execution result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub status: TaskStatus,
    pub started_at: u64,
    pub completed_at: Option<u64>,
    pub output: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl EdgeDevice {
    /// Create new edge device.
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            name: format!("Edge-{}", id),
            capabilities: vec![],
            resources: DeviceResources::default(),
            connectivity: ConnectivityConfig::default(),
            active_tasks: vec![],
            task_history: vec![],
            metadata: HashMap::new(),
            created_at: chrono::Utc::now().timestamp() as u64,
            last_heartbeat: chrono::Utc::now().timestamp() as u64,
        }
    }

    /// Add capability to device.
    pub fn add_capability(&mut self, capability: &str) {
        if !self.capabilities.contains(&capability.to_string()) {
            self.capabilities.push(capability.to_string());
        }
    }

    /// Check if device has capability.
    pub fn has_capability(&self, capability: &str) -> bool {
        self.capabilities.contains(&capability.to_string())
    }

    /// Check if device is online.
    pub fn is_online(&self) -> bool {
        self.connectivity.is_online
    }

    /// Check if device is available (not at capacity).
    pub fn is_available(&self) -> bool {
        self.resources.cpu_percent < 90.0
            && self.resources.memory_percent < 90.0
            && self.active_tasks.len() < 10
    }

    /// Get available resources score.
    pub fn available_resources(&self) -> f64 {
        let cpu_available = 100.0 - self.resources.cpu_percent;
        let mem_available = 100.0 - self.resources.memory_percent;
        (cpu_available + mem_available) / 2.0
    }

    /// Assign task to device.
    pub fn assign_task(&mut self, task: EdgeTask) {
        // Check resource requirements
        if !self.can_run_task(&task) {
            warn!("Device {} cannot run task {}: insufficient resources", self.id, task.id);
            return;
        }

        let task_with_status = EdgeTaskWithStatus {
            task,
            status: TaskStatus::Running,
            started_at: chrono::Utc::now().timestamp() as u64,
        };

        self.active_tasks.push(task_with_status.task);
        self.update_resources_for_task(&task, true);

        info!("Task {} assigned to device {}", task.id, self.id);
    }

    /// Check if device can run task.
    fn can_run_task(&self, task: &EdgeTask) -> bool {
        // Check if has required capability
        if !self.has_capability(&task.required_capability) {
            return false;
        }

        // Check resource requirements
        let cpu_ok = self.resources.cpu_percent + task.resource_requirements.cpu_percent < 100.0;
        let mem_ok = self.resources.memory_percent + task.resource_requirements.memory_percent < 100.0;

        cpu_ok && mem_ok
    }

    /// Update resources when task starts/completes.
    fn update_resources_for_task(&mut self, task: &EdgeTask, is_start: bool) {
        let multiplier = if is_start { 1.0 } else { -1.0 };

        self.resources.cpu_percent = (self.resources.cpu_percent
            + task.resource_requirements.cpu_percent * multiplier)
            .max(0.0)
            .min(100.0);

        self.resources.memory_percent = (self.resources.memory_percent
            + task.resource_requirements.memory_percent * multiplier)
            .max(0.0)
            .min(100.0);
    }

    /// Complete task.
    pub fn complete_task(&mut self, task_id: &str, output: Option<String>, error: Option<String>) {
        let task_idx = self.active_tasks.iter().position(|t| t.id == task_id);

        if let Some(idx) = task_idx {
            let task = self.active_tasks.remove(idx);
            let result = TaskResult {
                task_id: task.id.clone(),
                status: if error.is_some() {
                    TaskStatus::Failed
                } else {
                    TaskStatus::Completed
                },
                started_at: 0, // Would track this properly
                completed_at: Some(chrono::Utc::now().timestamp() as u64),
                output,
                error,
            };

            self.task_history.push(result);
            self.update_resources_for_task(&task, false);

            info!("Task {} completed on device {}", task_id, self.id);
        }
    }

    /// Update heartbeat.
    pub fn heartbeat(&mut self) {
        self.last_heartbeat = chrono::Utc::now().timestamp() as u64;
    }

    /// Update connectivity.
    pub fn update_connectivity(&mut self, connectivity: ConnectivityConfig) {
        self.connectivity = connectivity;
    }

    /// Update resource usage.
    pub fn update_resources(&mut self, resources: DeviceResources) {
        self.resources = resources;
    }

    /// Sync state with cloud.
    pub async fn sync_state(&self) -> Result<()> {
        // Would sync with cloud service
        info!("Syncing state for device {}", self.id);
        Ok(())
    }

    /// Get device status.
    pub fn get_status(&self) -> DeviceStatus {
        DeviceStatus {
            id: self.id.clone(),
            name: self.name.clone(),
            is_online: self.is_online(),
            is_available: self.is_available(),
            active_tasks: self.active_tasks.len() as i64,
            cpu_usage: self.resources.cpu_percent,
            memory_usage: self.resources.memory_percent,
            battery_level: self.resources.battery_percent,
            capabilities: self.capabilities.clone(),
        }
    }
}

/// Device status for API responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceStatus {
    pub id: String,
    pub name: String,
    pub is_online: bool,
    pub is_available: bool,
    pub active_tasks: i64,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub battery_level: Option<f64>,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EdgeTaskWithStatus {
    task: EdgeTask,
    status: TaskStatus,
    started_at: u64,
}
