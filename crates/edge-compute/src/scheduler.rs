//! Intelligent task scheduler for edge devices.

use crate::{EdgeDevice, EdgeTask, ResourceRequirements};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BinaryHeap;
use std::cmp::Ordering;
use tracing::{info, warn};

/// Task scheduler for edge computing.
pub struct EdgeScheduler {
    task_queue: BinaryHeap<ScheduledTask>,
    max_concurrent: usize,
}

impl EdgeScheduler {
    /// Create new scheduler.
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            task_queue: BinaryHeap::new(),
            max_concurrent,
        }
    }

    /// Add task to queue.
    pub fn enqueue(&mut self, task: EdgeTask) {
        let scheduled = ScheduledTask {
            task,
            priority: 0,
            enqueue_time: chrono::Utc::now().timestamp(),
        };
        self.task_queue.push(scheduled);
        info!("Task {} enqueued", scheduled.task.id);
    }

    /// Get next task.
    pub fn dequeue(&mut self) -> Option<EdgeTask> {
        self.task_queue.pop().map(|st| st.task)
    }

    /// Get queue size.
    pub fn queue_size(&self) -> usize {
        self.task_queue.len()
    }

    /// Schedule tasks to edges using bin-packing algorithm.
    pub async fn schedule_to_edges(
        &mut self,
        tasks: Vec<EdgeTask>,
        edges: &[EdgeDevice],
    ) -> Result<Vec<(String, String)>> {
        let mut assignments = Vec::new();

        // Sort tasks by priority (highest first)
        let mut sorted_tasks = tasks;
        sorted_tasks.sort_by(|a, b| b.priority.cmp(&a.priority));

        for task in sorted_tasks {
            if let Some(edge) = self.find_best_edge(&task, edges).await {
                assignments.push((task.id.clone(), edge.id.clone()));
                info!("Task {} scheduled to edge {}", task.id, edge.id);
            } else {
                warn!("No suitable edge found for task {}", task.id);
                self.enqueue(task);
            }
        }

        Ok(assignments)
    }

    /// Find best edge for task.
    async fn find_best_edge(&self, task: &EdgeTask, edges: &[EdgeDevice]) -> Option<EdgeDevice> {
        let mut best_edge: Option<(EdgeDevice, f64)> = None;

        for edge in edges {
            // Check if edge can run task
            if !self.can_edge_run_task(edge, task).await {
                continue;
            }

            // Calculate fitness score
            let score = self.calculate_fitness(edge, task).await;

            if best_edge.as_ref().map_or(true, |(_, best_score)| score > *best_score) {
                best_edge = Some((edge.clone(), score));
            }
        }

        best_edge.map(|(edge, _)| edge)
    }

    /// Check if edge can run task.
    async fn can_edge_run_task(&self, edge: &EdgeDevice, task: &EdgeTask) -> bool {
        // Check capability
        if !edge.has_capability(&task.required_capability) {
            return false;
        }

        // Check resources
        let cpu_ok = edge.resources.cpu_percent + task.resource_requirements.cpu_percent < 95.0;
        let mem_ok = edge.resources.memory_percent + task.resource_requirements.memory_percent < 95.0;

        cpu_ok && mem_ok && edge.is_available()
    }

    /// Calculate fitness score for edge-task pair.
    async fn calculate_fitness(&self, edge: &EdgeDevice, task: &EdgeTask) -> f64 {
        let mut score = 0.0;

        // Prefer edges with more available resources (higher is better)
        score += edge.available_resources() * 0.3;

        // Prefer edges with lower latency
        score += (1.0 - edge.connectivity.latency_ms / 1000.0) * 0.2;

        // Prefer online edges
        if edge.is_online() {
            score += 0.2;
        }

        // Prefer edges with matching capability (exact match)
        if edge.has_capability(&task.required_capability) {
            score += 0.2;
        }

        // Prefer edges with fewer active tasks (load balancing)
        score += (1.0 - edge.active_tasks.len() as f64 / 10.0) * 0.1;

        score
    }

    /// Rebalance tasks across edges.
    pub async fn rebalance(&mut self, edges: &[EdgeDevice]) -> Result<Vec<(String, String, String)>> {
        let mut migrations = Vec::new();

        // Find overloaded edges
        let overloaded: Vec<&EdgeDevice> = edges
            .iter()
            .filter(|e| e.resources.cpu_percent > 80.0 || e.resources.memory_percent > 80.0)
            .collect();

        // Find underloaded edges
        let underloaded: Vec<&EdgeDevice> = edges
            .iter()
            .filter(|e| e.resources.cpu_percent < 30.0 && e.resources.memory_percent < 30.0)
            .collect();

        // Migrate tasks from overloaded to underloaded
        for overloaded_edge in &overloaded {
            for task in &overloaded_edge.active_tasks {
                if let Some(underloaded_edge) = self.find_least_loaded(&underloaded).await {
                    migrations.push((
                        task.id.clone(),
                        overloaded_edge.id.clone(),
                        underloaded_edge.id.clone(),
                    ));
                }
            }
        }

        Ok(migrations)
    }

    /// Find least loaded edge.
    async fn find_least_loaded(&self, edges: &[&EdgeDevice]) -> Option<EdgeDevice> {
        edges
            .iter()
            .min_by(|a, b| {
                let score_a = a.resources.cpu_percent + a.resources.memory_percent;
                let score_b = b.resources.cpu_percent + b.resources.memory_percent;
                score_a.partial_cmp(&score_b).unwrap()
            })
            .cloned()
            .cloned()
    }
}

/// Scheduled task with priority.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ScheduledTask {
    task: EdgeTask,
    priority: i32,
    enqueue_time: i64,
}

impl PartialEq for ScheduledTask {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

impl Eq for ScheduledTask {}

impl PartialOrd for ScheduledTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScheduledTask {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher priority first
        self.priority.cmp(&other.priority)
            .then_with(|| other.enqueue_time.cmp(&self.enqueue_time)) // Earlier tasks first
    }
}

/// Resource requirements for edge task.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourceRequirements {
    pub cpu_percent: f64,
    pub memory_percent: f64,
    pub storage_mb: u64,
    pub network_bandwidth_mbps: f64,
    pub gpu_percent: Option<f64>,
}

impl ResourceRequirements {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_cpu(mut self, cpu_percent: f64) -> Self {
        self.cpu_percent = cpu_percent;
        self
    }

    pub fn with_memory(mut self, memory_percent: f64) -> Self {
        self.memory_percent = memory_percent;
        self
    }

    pub fn with_storage(mut self, storage_mb: u64) -> Self {
        self.storage_mb = storage_mb;
        self
    }

    pub fn with_network(mut self, bandwidth_mbps: f64) -> Self {
        self.network_bandwidth_mbps = bandwidth_mbps;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_scheduler() {
        let mut scheduler = EdgeScheduler::new(10);

        // Create tasks
        let task1 = EdgeTask::new("task-1", "Process data", "compute")
            .with_priority(150);
        let task2 = EdgeTask::new("task-2", "Analyze image", "vision")
            .with_priority(100);

        // Enqueue tasks
        scheduler.enqueue(task1.clone());
        scheduler.enqueue(task2.clone());

        // Dequeue (should get higher priority first)
        let dequeued = scheduler.dequeue().unwrap();
        assert_eq!(dequeued.id, "task-1");

        assert_eq!(scheduler.queue_size(), 1);
    }

    #[tokio::test]
    async fn test_scheduling_to_edges() {
        let mut scheduler = EdgeScheduler::new(10);

        // Create edges
        let mut edge1 = EdgeDevice::new("edge-1");
        edge1.capabilities = vec!["compute".to_string()];
        edge1.resources.cpu_percent = 20.0;
        edge1.resources.memory_percent = 30.0;

        let mut edge2 = EdgeDevice::new("edge-2");
        edge2.capabilities = vec!["compute".to_string(), "vision".to_string()];
        edge2.resources.cpu_percent = 10.0;
        edge2.resources.memory_percent = 20.0;

        let edges = vec![edge1, edge2];

        // Create tasks
        let tasks = vec![
            EdgeTask::new("task-1", "Process data", "compute")
                .with_resources(ResourceRequirements::new().with_cpu(10.0).with_memory(10.0)),
        ];

        // Schedule
        let assignments = scheduler.schedule_to_edges(tasks, &edges).await.unwrap();
        assert!(!assignments.is_empty());
    }
}
