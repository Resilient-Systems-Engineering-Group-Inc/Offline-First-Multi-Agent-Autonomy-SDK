//! Task execution engine.

use std::sync::Arc;
use tokio::sync::Mutex;
use chrono::Utc;

use crate::error::{WorkflowError, Result};
use crate::model::{Task, TaskStatus, TaskId};

/// Trait for task executors.
#[async_trait::async_trait]
pub trait TaskExecutor: Send + Sync {
    /// Execute a task.
    async fn execute(&self, task: &Task) -> Result<serde_json::Value>;

    /// Cancel a running task.
    async fn cancel(&self, task_id: TaskId) -> Result<()>;

    /// Get status of a task.
    async fn status(&self, task_id: TaskId) -> Result<TaskStatus>;
}

/// Simple in‑memory executor for testing.
pub struct DummyExecutor {
    tasks: Arc<Mutex<std::collections::HashMap<TaskId, TaskStatus>>>,
}

impl DummyExecutor {
    /// Create a new dummy executor.
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl TaskExecutor for DummyExecutor {
    async fn execute(&self, task: &Task) -> Result<serde_json::Value> {
        let mut tasks = self.tasks.lock().await;
        tasks.insert(task.id, TaskStatus::Running);
        // Simulate work
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        tasks.insert(task.id, TaskStatus::Completed);
        Ok(serde_json::json!({ "success": true, "task_id": task.id }))
    }

    async fn cancel(&self, task_id: TaskId) -> Result<()> {
        let mut tasks = self.tasks.lock().await;
        if let Some(status) = tasks.get_mut(&task_id) {
            *status = TaskStatus::Cancelled;
        }
        Ok(())
    }

    async fn status(&self, task_id: TaskId) -> Result<TaskStatus> {
        let tasks = self.tasks.lock().await;
        Ok(tasks.get(&task_id).copied().unwrap_or(TaskStatus::Pending))
    }
}

/// Execution engine that manages multiple executors.
pub struct ExecutionEngine {
    executor: Arc<dyn TaskExecutor>,
    running_tasks: Arc<Mutex<std::collections::HashSet<TaskId>>>,
}

impl ExecutionEngine {
    /// Create a new execution engine with a given executor.
    pub fn new(executor: Arc<dyn TaskExecutor>) -> Self {
        Self {
            executor,
            running_tasks: Arc::new(Mutex::new(std::collections::HashSet::new())),
        }
    }

    /// Start executing a task asynchronously.
    pub async fn start_task(&self, task: Task) -> Result<TaskId> {
        let task_id = task.id;
        let executor = self.executor.clone();
        let running_tasks = self.running_tasks.clone();

        // Mark as running
        {
            let mut rt = running_tasks.lock().await;
            rt.insert(task_id);
        }

        // Spawn background execution
        tokio::spawn(async move {
            let _ = executor.execute(&task).await;
            let mut rt = running_tasks.lock().await;
            rt.remove(&task_id);
        });

        Ok(task_id)
    }

    /// Cancel a running task.
    pub async fn cancel_task(&self, task_id: TaskId) -> Result<()> {
        self.executor.cancel(task_id).await
    }

    /// Get status of a task.
    pub async fn task_status(&self, task_id: TaskId) -> Result<TaskStatus> {
        self.executor.status(task_id).await
    }

    /// Check if a task is currently running.
    pub async fn is_running(&self, task_id: TaskId) -> bool {
        let running = self.running_tasks.lock().await;
        running.contains(&task_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dummy_executor() {
        let executor = DummyExecutor::new();
        let task = Task::new("test", "dummy", serde_json::json!({}));
        let result = executor.execute(&task).await.unwrap();
        assert_eq!(result["success"], true);
    }

    #[tokio::test]
    async fn test_execution_engine() {
        let executor = Arc::new(DummyExecutor::new());
        let engine = ExecutionEngine::new(executor);
        let task = Task::new("test", "dummy", serde_json::json!({}));
        let task_id = engine.start_task(task).await.unwrap();
        assert!(engine.is_running(task_id).await);
        // Wait a bit for task to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
        assert!(!engine.is_running(task_id).await);
    }
}