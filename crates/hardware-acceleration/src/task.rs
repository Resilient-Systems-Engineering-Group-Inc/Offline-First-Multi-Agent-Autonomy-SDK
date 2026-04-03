//! Task abstraction for hardware‑accelerated computation.

use crate::error::{Result, AccelerationError};
use crate::kernel::Kernel;
use crate::memory::MemoryBuffer;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

/// A task that can be executed on an accelerator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccelerationTask {
    /// Unique task ID.
    pub id: String,
    /// Task name (human‑readable).
    pub name: String,
    /// Kernel to execute (if any).
    pub kernel: Option<String>,
    /// Input buffers (by ID).
    pub input_buffers: Vec<String>,
    /// Output buffers (by ID).
    pub output_buffers: Vec<String>,
    /// Work size (global_x, global_y, global_z).
    pub work_size: (usize, usize, usize),
    /// Task priority (higher = more urgent).
    pub priority: u8,
    /// Task status.
    pub status: TaskStatus,
    /// Result metadata (e.g., execution time, error message).
    pub result: Option<TaskResult>,
}

/// Status of a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Task created but not yet submitted.
    Created,
    /// Task submitted to the accelerator queue.
    Submitted,
    /// Task is currently executing.
    Running,
    /// Task completed successfully.
    Completed,
    /// Task failed.
    Failed,
    /// Task cancelled.
    Cancelled,
}

/// Result of a task execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    /// Whether the task succeeded.
    pub success: bool,
    /// Error message (if any).
    pub error: Option<String>,
    /// Execution time in milliseconds.
    pub execution_time_ms: u64,
    /// Additional output data (e.g., output buffer IDs).
    pub output: serde_json::Value,
}

impl AccelerationTask {
    /// Creates a new task.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            kernel: None,
            input_buffers: Vec::new(),
            output_buffers: Vec::new(),
            work_size: (1, 1, 1),
            priority: 5,
            status: TaskStatus::Created,
            result: None,
        }
    }

    /// Sets the kernel name.
    pub fn with_kernel(mut self, kernel: impl Into<String>) -> Self {
        self.kernel = Some(kernel.into());
        self
    }

    /// Adds an input buffer.
    pub fn add_input_buffer(mut self, buffer_id: impl Into<String>) -> Self {
        self.input_buffers.push(buffer_id.into());
        self
    }

    /// Adds an output buffer.
    pub fn add_output_buffer(mut self, buffer_id: impl Into<String>) -> Self {
        self.output_buffers.push(buffer_id.into());
        self
    }

    /// Sets the work size.
    pub fn with_work_size(mut self, x: usize, y: usize, z: usize) -> Self {
        self.work_size = (x, y, z);
        self
    }

    /// Sets the priority.
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Marks the task as submitted.
    pub fn mark_submitted(&mut self) {
        self.status = TaskStatus::Submitted;
    }

    /// Marks the task as running.
    pub fn mark_running(&mut self) {
        self.status = TaskStatus::Running;
    }

    /// Marks the task as completed with a result.
    pub fn mark_completed(&mut self, result: TaskResult) {
        self.status = TaskStatus::Completed;
        self.result = Some(result);
    }

    /// Marks the task as failed.
    pub fn mark_failed(&mut self, error: impl Into<String>) {
        self.status = TaskStatus::Failed;
        self.result = Some(TaskResult {
            success: false,
            error: Some(error.into()),
            execution_time_ms: 0,
            output: serde_json::Value::Null,
        });
    }
}

/// Task scheduler that manages execution of acceleration tasks.
pub struct TaskScheduler {
    pending: Mutex<Vec<AccelerationTask>>,
    running: Mutex<Vec<AccelerationTask>>,
    completed: Mutex<Vec<AccelerationTask>>,
}

impl TaskScheduler {
    /// Creates a new task scheduler.
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(Vec::new()),
            running: Mutex::new(Vec::new()),
            completed: Mutex::new(Vec::new()),
        }
    }

    /// Submits a task for execution.
    pub async fn submit(&self, task: AccelerationTask) -> Result<String> {
        let mut pending = self.pending.lock().await;
        let task_id = task.id.clone();
        pending.push(task);
        Ok(task_id)
    }

    /// Picks the next task to execute (based on priority).
    pub async fn pick_next(&self) -> Option<AccelerationTask> {
        let mut pending = self.pending.lock().await;
        if pending.is_empty() {
            return None;
        }
        // Simple priority sorting: higher priority first.
        pending.sort_by_key(|t| std::cmp::Reverse(t.priority));
        let task = pending.remove(0);
        Some(task)
    }

    /// Moves a task to the running list.
    pub async fn mark_running(&self, task: AccelerationTask) {
        let mut running = self.running.lock().await;
        running.push(task);
    }

    /// Moves a task to the completed list.
    pub async fn mark_completed(&self, task: AccelerationTask) {
        let mut completed = self.completed.lock().await;
        completed.push(task);
    }

    /// Returns all pending tasks.
    pub async fn pending_tasks(&self) -> Vec<AccelerationTask> {
        self.pending.lock().await.clone()
    }

    /// Returns all running tasks.
    pub async fn running_tasks(&self) -> Vec<AccelerationTask> {
        self.running.lock().await.clone()
    }

    /// Returns all completed tasks.
    pub async fn completed_tasks(&self) -> Vec<AccelerationTask> {
        self.completed.lock().await.clone()
    }
}

impl Default for TaskScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task = AccelerationTask::new("test_task")
            .with_kernel("add")
            .add_input_buffer("buf1")
            .with_work_size(256, 1, 1)
            .with_priority(10);
        assert_eq!(task.name, "test_task");
        assert_eq!(task.kernel, Some("add".to_string()));
        assert_eq!(task.input_buffers, vec!["buf1"]);
        assert_eq!(task.priority, 10);
    }

    #[tokio::test]
    async fn test_task_scheduler() {
        let scheduler = TaskScheduler::new();
        let task = AccelerationTask::new("task1");
        let id = scheduler.submit(task).await.unwrap();
        assert!(!id.is_empty());
        let pending = scheduler.pending_tasks().await;
        assert_eq!(pending.len(), 1);
    }
}