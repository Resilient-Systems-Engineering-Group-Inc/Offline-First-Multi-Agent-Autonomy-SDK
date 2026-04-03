//! Workflow monitoring and observability.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::Utc;

use crate::error::{WorkflowError, Result};
use crate::model::{Workflow, WorkflowId, Task, TaskId, TaskStatus, WorkflowStatus};

/// Metrics for workflow monitoring.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkflowMetrics {
    /// Workflow ID.
    pub workflow_id: WorkflowId,
    /// Number of tasks.
    pub total_tasks: usize,
    /// Number of completed tasks.
    pub completed_tasks: usize,
    /// Number of failed tasks.
    pub failed_tasks: usize,
    /// Number of running tasks.
    pub running_tasks: usize,
    /// Number of pending tasks.
    pub pending_tasks: usize,
    /// Overall progress (0‑100).
    pub progress_percent: f64,
    /// Estimated time remaining (seconds).
    pub estimated_remaining_secs: f64,
    /// Start time.
    pub start_time: Option<chrono::DateTime<Utc>>,
    /// Finish time (if completed).
    pub finish_time: Option<chrono::DateTime<Utc>>,
}

/// Monitor that tracks workflow execution and provides metrics.
pub struct WorkflowMonitor {
    workflows: Arc<RwLock<HashMap<WorkflowId, Workflow>>>,
    metrics_cache: Arc<RwLock<HashMap<WorkflowId, WorkflowMetrics>>>,
}

impl WorkflowMonitor {
    /// Create a new monitor.
    pub fn new() -> Self {
        Self {
            workflows: Arc::new(RwLock::new(HashMap::new())),
            metrics_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a workflow for monitoring.
    pub async fn register_workflow(&self, workflow: Workflow) {
        let workflow_id = workflow.id;
        let mut workflows = self.workflows.write().await;
        workflows.insert(workflow_id, workflow);
        self.update_metrics(workflow_id).await;
    }

    /// Update a workflow.
    pub async fn update_workflow(&self, workflow: Workflow) {
        let workflow_id = workflow.id;
        let mut workflows = self.workflows.write().await;
        workflows.insert(workflow_id, workflow);
        self.update_metrics(workflow_id).await;
    }

    /// Remove a workflow from monitoring.
    pub async fn remove_workflow(&self, workflow_id: WorkflowId) {
        let mut workflows = self.workflows.write().await;
        workflows.remove(&workflow_id);
        let mut cache = self.metrics_cache.write().await;
        cache.remove(&workflow_id);
    }

    /// Update metrics for a workflow.
    async fn update_metrics(&self, workflow_id: WorkflowId) {
        let workflows = self.workflows.read().await;
        let Some(workflow) = workflows.get(&workflow_id) else {
            return;
        };

        let total = workflow.tasks.len();
        let completed = workflow.tasks.iter().filter(|t| t.status == TaskStatus::Completed).count();
        let failed = workflow.tasks.iter().filter(|t| t.status == TaskStatus::Failed).count();
        let running = workflow.tasks.iter().filter(|t| t.status == TaskStatus::Running).count();
        let pending = workflow.tasks.iter().filter(|t| t.status == TaskStatus::Pending || t.status == TaskStatus::Waiting).count();
        let progress = if total > 0 {
            (completed as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        // Estimate remaining time based on average duration of completed tasks
        let avg_duration = workflow.tasks.iter()
            .filter(|t| t.status == TaskStatus::Completed)
            .filter_map(|t| {
                t.started_at.and_then(|start| t.finished_at.map(|finish| (finish - start).num_seconds() as f64))
            })
            .collect::<Vec<_>>();
        let avg_secs = if !avg_duration.is_empty() {
            avg_duration.iter().sum::<f64>() / avg_duration.len() as f64
        } else {
            0.0
        };
        let remaining_secs = avg_secs * (total - completed) as f64;

        let metrics = WorkflowMetrics {
            workflow_id,
            total_tasks: total,
            completed_tasks: completed,
            failed_tasks: failed,
            running_tasks: running,
            pending_tasks: pending,
            progress_percent: progress,
            estimated_remaining_secs: remaining_secs,
            start_time: workflow.started_at,
            finish_time: workflow.finished_at,
        };

        let mut cache = self.metrics_cache.write().await;
        cache.insert(workflow_id, metrics);
    }

    /// Get metrics for a workflow.
    pub async fn get_metrics(&self, workflow_id: WorkflowId) -> Option<WorkflowMetrics> {
        let cache = self.metrics_cache.read().await;
        cache.get(&workflow_id).cloned()
    }

    /// Get metrics for all monitored workflows.
    pub async fn get_all_metrics(&self) -> Vec<WorkflowMetrics> {
        let cache = self.metrics_cache.read().await;
        cache.values().cloned().collect()
    }

    /// Generate alerts for workflows that are stuck, failing, etc.
    pub async fn generate_alerts(&self) -> Vec<WorkflowAlert> {
        let mut alerts = Vec::new();
        let workflows = self.workflows.read().await;
        for workflow in workflows.values() {
            // Check for stuck workflows (no progress for a long time)
            if workflow.status == WorkflowStatus::Running {
                let now = Utc::now();
                if let Some(started) = workflow.started_at {
                    let duration = now - started;
                    if duration.num_minutes() > 30 {
                        alerts.push(WorkflowAlert {
                            workflow_id: workflow.id,
                            severity: AlertSeverity::Warning,
                            message: format!("Workflow '{}' has been running for over 30 minutes", workflow.name),
                            timestamp: now,
                        });
                    }
                }
            }

            // Check for failed tasks
            let failed_count = workflow.tasks.iter().filter(|t| t.status == TaskStatus::Failed).count();
            if failed_count > 0 {
                alerts.push(WorkflowAlert {
                    workflow_id: workflow.id,
                    severity: AlertSeverity::Error,
                    message: format!("Workflow '{}' has {} failed tasks", workflow.name, failed_count),
                    timestamp: Utc::now(),
                });
            }
        }
        alerts
    }
}

impl Default for WorkflowMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Alert severity.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Workflow alert.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkflowAlert {
    /// Workflow ID.
    pub workflow_id: WorkflowId,
    /// Severity.
    pub severity: AlertSeverity,
    /// Alert message.
    pub message: String,
    /// Timestamp.
    pub timestamp: chrono::DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_monitor_register() {
        let monitor = WorkflowMonitor::new();
        let workflow = Workflow::new("test", "");
        monitor.register_workflow(workflow).await;
        let metrics = monitor.get_all_metrics().await;
        assert_eq!(metrics.len(), 1);
    }
}