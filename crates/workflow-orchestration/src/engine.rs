//! Workflow execution engine.
//!
//! Handles workflow lifecycle:
//! - Parsing and validation
//! - Task scheduling and execution
//! - Dependency resolution
//! - Error handling and recovery
//! - Rollback support

use crate::error::{WorkflowError, Result};
use crate::model::{
    Workflow, WorkflowInstance, WorkflowStatus, WorkflowResult, WorkflowFailureStrategy,
    Task, TaskStatus,
};
use crate::scheduler::TaskScheduler;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::{Mutex, RwLock, Notify};
use tokio::time::{Duration, sleep};
use tracing::{info, error, debug};

/// Workflow execution engine.
pub struct WorkflowEngine {
    workflows: RwLock<HashMap<String, Workflow>>,
    instances: RwLock<HashMap<String, WorkflowInstance>>,
    scheduler: Arc<Mutex<TaskScheduler>>,
    max_concurrent: usize,
    /// Notifier for workflow instance state changes (wakes up polling loops).
    instance_notifier: Arc<Notify>,
    /// Global shutdown flag.
    shutdown: Arc<AtomicBool>,
}

impl WorkflowEngine {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            workflows: RwLock::new(HashMap::new()),
            instances: RwLock::new(HashMap::new()),
            scheduler: Arc::new(Mutex::new(TaskScheduler::new())),
            max_concurrent,
            instance_notifier: Arc::new(Notify::new()),
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Register a workflow definition.
    pub async fn register_workflow(&self, workflow: Workflow) -> Result<String, WorkflowError> {
        workflow.validate()?;
        
        let id = workflow.id.to_string();
        info!("Registering workflow: {} ({})", workflow.name, id);
        
        let mut workflows = self.workflows.write().await;
        workflows.insert(id.clone(), workflow);
        
        Ok(id)
    }

    /// Get a workflow definition.
    pub async fn get_workflow(&self, id: &str) -> Option<Workflow> {
        let workflows = self.workflows.read().await;
        workflows.get(id).cloned()
    }

    /// List all registered workflows.
    pub async fn list_workflows(&self) -> Vec<Workflow> {
        let workflows = self.workflows.read().await;
        workflows.values().cloned().collect()
    }

    /// Unregister a workflow definition.
    pub async fn unregister_workflow(&self, id: &str) -> bool {
        let mut workflows = self.workflows.write().await;
        workflows.remove(id).is_some()
    }

    /// Start a new workflow instance.
    pub async fn start_workflow(
        &self,
        workflow_id: &str,
        parameters: HashMap<String, String>,
    ) -> Result<WorkflowInstanceHandle, WorkflowError> {
        let workflow = self.get_workflow(workflow_id)
            .ok_or_else(|| WorkflowError::NotFound(workflow_id.to_string()))?;

        let instance = WorkflowInstance::new(&workflow, parameters);
        let instance_id = instance.instance_id.clone();

        info!("Starting workflow instance: {} (workflow: {})", instance_id, workflow_id);

        let mut instances = self.instances.write().await;
        instances.insert(instance_id.clone(), instance);
        drop(instances);

        // Spawn execution task
        let engine = self.clone();
        tokio::spawn(async move {
            if let Err(e) = engine.execute_instance(&instance_id).await {
                error!("Workflow execution failed: {}", e);
            }
        });

        Ok(WorkflowInstanceHandle {
            instance_id,
            engine: self.clone(),
        })
    }

    /// Execute a workflow instance.
    async fn execute_instance(&self, instance_id: &str) -> Result<(), WorkflowError> {
        let workflow_id = {
            let instances = self.instances.read().await;
            let instance = instances.get(instance_id)
                .ok_or_else(|| WorkflowError::NotFound(instance_id.to_string()))?;
            instance.workflow_id
        };

        let workflow = self.get_workflow(&workflow_id.to_string())
            .ok_or_else(|| WorkflowError::NotFound(workflow_id.to_string()))?;

        loop {
            // Check for global shutdown
            if self.shutdown.load(Ordering::SeqCst) {
                info!("Workflow engine shutting down, cancelling instance {}", instance_id);
                let mut instances = self.instances.write().await;
                if let Some(instance) = instances.get_mut(instance_id) {
                    instance.status = WorkflowStatus::Cancelled;
                }
                return Ok(());
            }

            let should_continue = {
                let mut instances = self.instances.write().await;
                let instance = instances.get_mut(instance_id)
                    .ok_or_else(|| WorkflowError::NotFound(instance_id.to_string()))?;

                if instance.is_complete() {
                    false
                } else {
                    // Get ready tasks
                    let completed_tasks = instance.get_completed_tasks();
                    let ready_tasks = workflow.get_next_tasks(&completed_tasks);

                    // Execute ready tasks in parallel
                    let mut task_handles = Vec::new();
                    for task in ready_tasks {
                        if task_handles.len() >= self.max_concurrent {
                            break;
                        }

                        // Mark task as running
                        instance.mark_task_started(&task.id)?;

                        let task_clone = task.clone();
                        let instance_id_clone = instance_id.to_string();
                        let handle = tokio::spawn(async move {
                            Self::execute_task(&task_clone, &instance_id_clone).await
                        });
                        task_handles.push((task.id, handle));
                    }

                    // Wait for all spawned tasks
                    for (task_id, handle) in task_handles {
                        match handle.await {
                            Ok(Ok(output)) => {
                                let mut instances = self.instances.write().await;
                                if let Some(instance) = instances.get_mut(instance_id) {
                                    let _ = instance.mark_task_completed(&task_id, output);
                                    // Notify any waiters (e.g., await_completion)
                                    self.instance_notifier.notify_waiters();
                                }
                            }
                            Ok(Err(e)) => {
                                let mut instances = self.instances.write().await;
                                if let Some(instance) = instances.get_mut(instance_id) {
                                    let _ = instance.mark_task_failed(&task_id, &e.to_string());

                                    // Handle failure based on strategy
                                    match workflow.on_failure {
                                        WorkflowFailureStrategy::Fail => {
                                            instance.status = WorkflowStatus::Failed;
                                            instance.error = Some(e.to_string());
                                            self.instance_notifier.notify_waiters();
                                            return Err(WorkflowError::TaskExecution(e.to_string()));
                                        }
                                        WorkflowFailureStrategy::Continue => {
                                            // Continue with other tasks
                                        }
                                        WorkflowFailureStrategy::Rollback => {
                                            self.rollback_instance(instance_id).await?;
                                            self.instance_notifier.notify_waiters();
                                            return Ok(());
                                        }
                                        WorkflowFailureStrategy::Pause => {
                                            instance.status = WorkflowStatus::Paused;
                                            self.instance_notifier.notify_waiters();
                                            return Ok(());
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Task execution panicked: {}", e);
                            }
                        }
                    }

                    // Check if all tasks are completed
                    let all_completed = workflow.tasks.iter()
                        .all(|t| {
                            instance.task_states.get(&t.id)
                                .map(|s| s.status == TaskStatus::Completed)
                                .unwrap_or(false)
                        });

                    if all_completed {
                        instance.status = WorkflowStatus::Completed;
                        instance.completed_at = Some(
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs(),
                        );
                        self.instance_notifier.notify_waiters();
                        false
                    } else {
                        true
                    }
                }
            };

            if !should_continue {
                break;
            }

            // Wait for notification instead of busy-loop polling
            // Use a timeout to periodically check for shutdown signals
            tokio::select! {
                _ = self.instance_notifier.notified() => {
                    // A task completed or state changed, re-check
                }
                _ = sleep(Duration::from_millis(500)) => {
                    // Timeout to prevent starvation if notification is missed
                }
            }
        }

        Ok(())
    }

    /// Execute a single task.
    async fn execute_task(
        task: &Task,
        instance_id: &str,
    ) -> Result<HashMap<String, String>, WorkflowError> {
        debug!("Executing task: {} (instance: {})", task.name, instance_id);

        // Simulate task execution (in real impl, would call actual task handler)
        let duration = Duration::from_secs(task.estimated_duration_secs);
        sleep(duration).await;

        // Generate output based on task type
        let output = match task.task_type.as_str() {
            "setup" => {
                HashMap::from([("status".to_string(), "initialized".to_string())])
            }
            "action" => {
                HashMap::from([("result".to_string(), "success".to_string())])
            }
            "teardown" => {
                HashMap::from([("status".to_string(), "cleaned".to_string())])
            }
            _ => {
                HashMap::new()
            }
        };

        debug!("Task completed: {}", task.name);
        Ok(output)
    }

    /// Rollback a workflow instance.
    async fn rollback_instance(&self, instance_id: &str) -> Result<(), WorkflowError> {
        info!("Rolling back workflow instance: {}", instance_id);

        let mut instances = self.instances.write().await;
        let instance = instances.get_mut(instance_id)
            .ok_or_else(|| WorkflowError::NotFound(instance_id.to_string()))?;

        instance.status = WorkflowStatus::Rollback;

        // Rollback tasks in reverse order
        let completed_tasks: Vec<_> = instance.task_states
            .iter()
            .filter(|(_, state)| state.status == TaskStatus::Completed)
            .map(|(id, _)| *id)
            .collect();

        for task_id in completed_tasks.into_iter().rev() {
            if let Some(state) = instance.task_states.get_mut(&task_id) {
                state.status = TaskStatus::RolledBack;
                info!("Rolled back task: {}", task_id);
            }
        }

        instance.status = WorkflowStatus::Failed;
        instance.error = Some("Rollback completed".to_string());

        Ok(())
    }

    /// Pause a workflow instance.
    pub async fn pause_workflow(&self, instance_id: &str) -> Result<(), WorkflowError> {
        let mut instances = self.instances.write().await;
        let instance = instances.get_mut(instance_id)
            .ok_or_else(|| WorkflowError::NotFound(instance_id.to_string()))?;

        if instance.status == WorkflowStatus::Running {
            instance.status = WorkflowStatus::Paused;
            self.instance_notifier.notify_waiters();
            Ok(())
        } else {
            Err(WorkflowError::Other(format!(
                "Cannot pause workflow in state {:?}", instance.status
            )))
        }
    }

    /// Resume a paused workflow instance.
    pub async fn resume_workflow(&self, instance_id: &str) -> Result<(), WorkflowError> {
        let mut instances = self.instances.write().await;
        let instance = instances.get_mut(instance_id)
            .ok_or_else(|| WorkflowError::NotFound(instance_id.to_string()))?;

        if instance.status == WorkflowStatus::Paused {
            instance.status = WorkflowStatus::Running;
            self.instance_notifier.notify_waiters();
            Ok(())
        } else {
            Err(WorkflowError::Other(format!(
                "Cannot resume workflow in state {:?}", instance.status
            )))
        }
    }

    /// Cancel a workflow instance.
    pub async fn cancel_workflow(&self, instance_id: &str) -> Result<(), WorkflowError> {
        let mut instances = self.instances.write().await;
        let instance = instances.get_mut(instance_id)
            .ok_or_else(|| WorkflowError::NotFound(instance_id.to_string()))?;

        if instance.status == WorkflowStatus::Running || instance.status == WorkflowStatus::Paused {
            instance.status = WorkflowStatus::Cancelled;
            self.instance_notifier.notify_waiters();
            Ok(())
        } else {
            Err(WorkflowError::Other(format!(
                "Cannot cancel workflow in state {:?}", instance.status
            )))
        }
    }

    /// Get workflow instance status.
    pub async fn get_instance(&self, instance_id: &str) -> Option<WorkflowInstance> {
        let instances = self.instances.read().await;
        instances.get(instance_id).cloned()
    }

    /// Get workflow result.
    pub async fn get_result(&self, instance_id: &str) -> Option<WorkflowResult> {
        let instances = self.instances.read().await;
        instances.get(instance_id).map(WorkflowResult::from_instance)
    }

    /// List all workflow instances.
    pub async fn list_instances(&self) -> Vec<WorkflowInstance> {
        let instances = self.instances.read().await;
        instances.values().cloned().collect()
    }

    /// Delete a completed workflow instance.
    pub async fn delete_instance(&self, instance_id: &str) -> bool {
        let mut instances = self.instances.write().await;
        instances.remove(instance_id).is_some()
    }

    /// Shutdown the engine gracefully, cancelling all running instances.
    pub async fn shutdown(&self) {
        info!("Shutting down workflow engine");
        self.shutdown.store(true, Ordering::SeqCst);
        self.instance_notifier.notify_waiters();
    }
}

impl Clone for WorkflowEngine {
    fn clone(&self) -> Self {
        Self {
            workflows: self.workflows.clone(),
            instances: self.instances.clone(),
            scheduler: self.scheduler.clone(),
            max_concurrent: self.max_concurrent,
            instance_notifier: self.instance_notifier.clone(),
            shutdown: self.shutdown.clone(),
        }
    }
}

/// Handle for a running workflow instance.
pub struct WorkflowInstanceHandle {
    instance_id: String,
    engine: WorkflowEngine,
}

impl WorkflowInstanceHandle {
    /// Get the instance ID.
    pub fn instance_id(&self) -> &str {
        &self.instance_id
    }

    /// Wait for workflow completion using notification-based waiting.
    pub async fn await_completion(&self) -> Result<WorkflowResult, WorkflowError> {
        loop {
            let instance = self.engine.get_instance(&self.instance_id).await
                .ok_or_else(|| WorkflowError::NotFound(self.instance_id.clone()))?;

            if instance.is_complete() {
                let result = WorkflowResult::from_instance(&instance);
                return Ok(result);
            }

            // Wait for notification instead of busy-loop polling
            // Use a notified() future that we can select with a timeout
            tokio::select! {
                _ = self.engine.instance_notifier.notified() => {
                    // State changed, re-check
                }
                _ = sleep(Duration::from_millis(500)) => {
                    // Timeout to prevent starvation
                }
            }
        }
    }

    /// Get current status.
    pub async fn status(&self) -> Option<WorkflowStatus> {
        let instance = self.engine.get_instance(&self.instance_id).await?;
        Some(instance.status)
    }

    /// Get progress percentage.
    pub async fn progress(&self) -> f64 {
        let instance = self.engine.get_instance(&self.instance_id).await
            .unwrap_or_else(|| WorkflowInstance::new(
                &Workflow::new("unknown", "unknown"),
                HashMap::new()
            ));
        instance.progress()
    }

    /// Pause the workflow.
    pub async fn pause(&self) -> Result<(), WorkflowError> {
        self.engine.pause_workflow(&self.instance_id).await
    }

    /// Resume the workflow.
    pub async fn resume(&self) -> Result<(), WorkflowError> {
        self.engine.resume_workflow(&self.instance_id).await
    }

    /// Cancel the workflow.
    pub async fn cancel(&self) -> Result<(), WorkflowError> {
        self.engine.cancel_workflow(&self.instance_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_workflow_execution() {
        let engine = WorkflowEngine::new(4);

        let mut workflow = Workflow::new("Test Workflow", "A test workflow");
        workflow.add_task(Task::new("Task 1", "setup", serde_json::json!({})));
        workflow.add_task(Task::new("Task 2", "action", serde_json::json!({})));
        workflow.on_failure = WorkflowFailureStrategy::Fail;

        let wf_id = workflow.id.to_string();
        engine.register_workflow(workflow).await.unwrap();

        let handle = engine.start_workflow(&wf_id, HashMap::new()).await.unwrap();
        let result = handle.await_completion().await.unwrap();

        assert_eq!(result.status, WorkflowStatus::Completed);
        assert_eq!(result.completed_tasks, 2);
        assert_eq!(result.failed_tasks, 0);
    }

    #[tokio::test]
    async fn test_workflow_pause_resume() {
        let engine = WorkflowEngine::new(4);

        let mut workflow = Workflow::new("Pause Test", "Testing pause/resume");
        workflow.add_task(Task::new("long-task", "action", serde_json::json!({})));
        workflow.tasks[0].estimated_duration_secs = 5;

        let wf_id = workflow.id.to_string();
        engine.register_workflow(workflow).await.unwrap();

        let handle = engine.start_workflow(&wf_id, HashMap::new()).await.unwrap();

        // Pause the workflow
        handle.pause().await.unwrap();
        let status = handle.status().await.unwrap();
        assert_eq!(status, WorkflowStatus::Paused);

        // Resume
        handle.resume().await.unwrap();
        let status = handle.status().await.unwrap();
        assert_eq!(status, WorkflowStatus::Running);

        // Wait for completion
        let result = handle.await_completion().await.unwrap();
        assert_eq!(result.status, WorkflowStatus::Completed);
    }

    #[tokio::test]
    async fn test_workflow_cancel() {
        let engine = WorkflowEngine::new(4);

        let mut workflow = Workflow::new("Cancel Test", "Testing cancellation");
        workflow.add_task(Task::new("long-task", "action", serde_json::json!({})));
        workflow.tasks[0].estimated_duration_secs = 10;

        let wf_id = workflow.id.to_string();
        engine.register_workflow(workflow).await.unwrap();

        let handle = engine.start_workflow(&wf_id, HashMap::new()).await.unwrap();

        // Cancel the workflow
        handle.cancel().await.unwrap();
        let status = handle.status().await.unwrap();
        assert_eq!(status, WorkflowStatus::Cancelled);
    }

    #[tokio::test]
    async fn test_workflow_not_found() {
        let engine = WorkflowEngine::new(4);
        let result = engine.start_workflow("nonexistent", HashMap::new()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_workflow_progress() {
        let engine = WorkflowEngine::new(4);

        let mut workflow = Workflow::new("Progress Test", "Testing progress tracking");
        workflow.add_task(Task::new("t1", "setup", serde_json::json!({})));
        workflow.add_task(Task::new("t2", "action", serde_json::json!({})));

        let wf_id = workflow.id.to_string();
        engine.register_workflow(workflow).await.unwrap();

        let handle = engine.start_workflow(&wf_id, HashMap::new()).await.unwrap();

        // Wait for completion
        let result = handle.await_completion().await.unwrap();
        assert_eq!(result.status, WorkflowStatus::Completed);
        assert_eq!(result.completed_tasks, 2);
        assert_eq!(result.total_tasks, 2);
    }

    #[tokio::test]
    async fn test_workflow_engine_shutdown() {
        let engine = WorkflowEngine::new(4);

        let mut workflow = Workflow::new("Shutdown Test", "Testing shutdown");
        workflow.add_task(Task::new("long-task", "action", serde_json::json!({})));
        workflow.tasks[0].estimated_duration_secs = 30;

        let wf_id = workflow.id.to_string();
        engine.register_workflow(workflow).await.unwrap();

        let handle = engine.start_workflow(&wf_id, HashMap::new()).await.unwrap();

        // Shutdown the engine
        engine.shutdown().await;

        // The instance should be cancelled
        let result = handle.await_completion().await.unwrap();
        assert_eq!(result.status, WorkflowStatus::Cancelled);
    }
}
