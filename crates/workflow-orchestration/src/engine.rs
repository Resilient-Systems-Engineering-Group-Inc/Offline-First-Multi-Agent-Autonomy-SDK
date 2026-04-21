//! Workflow execution engine.
//!
//! Handles workflow lifecycle:
//! - Parsing and validation
//! - Task scheduling and execution
//! - Dependency resolution
//! - Error handling and recovery
//! - Rollback support

use crate::model::{
    Workflow, WorkflowInstance, WorkflowStatus, WorkflowResult, WorkflowError, WorkflowFailureStrategy,
    Task, TaskStatus, TaskState,
};
use crate::scheduler::TaskScheduler;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{Duration, sleep};
use tracing::{info, warn, error, debug};
use uuid::Uuid;

/// Workflow execution engine.
pub struct WorkflowEngine {
    workflows: RwLock<HashMap<String, Workflow>>,
    instances: RwLock<HashMap<String, WorkflowInstance>>,
    scheduler: Arc<Mutex<TaskScheduler>>,
    max_concurrent: usize,
}

impl WorkflowEngine {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            workflows: RwLock::new(HashMap::new()),
            instances: RwLock::new(HashMap::new()),
            scheduler: Arc::new(Mutex::new(TaskScheduler::new())),
            max_concurrent,
        }
    }

    /// Register a workflow definition.
    pub async fn register_workflow(&self, workflow: Workflow) -> Result<String, WorkflowError> {
        workflow.validate()?;
        
        let id = workflow.id.clone();
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
            .ok_or_else(|| WorkflowError::WorkflowNotFound(workflow_id.to_string()))?;

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
                .ok_or_else(|| WorkflowError::WorkflowNotFound(instance_id.to_string()))?;
            instance.workflow_id.clone()
        };

        let workflow = self.get_workflow(&workflow_id)
            .ok_or_else(|| WorkflowError::WorkflowNotFound(workflow_id.clone()))?;

        loop {
            let should_continue = {
                let mut instances = self.instances.write().await;
                let instance = instances.get_mut(instance_id)
                    .ok_or_else(|| WorkflowError::WorkflowNotFound(instance_id.to_string()))?;

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
                        task_handles.push((task.id.clone(), handle));
                    }

                    // Wait for all spawned tasks
                    for (task_id, handle) in task_handles {
                        match handle.await {
                            Ok(Ok(output)) => {
                                let mut instances = self.instances.write().await;
                                if let Some(instance) = instances.get_mut(instance_id) {
                                    let _ = instance.mark_task_completed(&task_id, output);
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
                                            return Err(WorkflowError::ExecutionFailed(e.to_string()));
                                        }
                                        WorkflowFailureStrategy::Continue => {
                                            // Continue with other tasks
                                        }
                                        WorkflowFailureStrategy::Rollback => {
                                            self.rollback_instance(instance_id).await?;
                                            return Ok(());
                                        }
                                        WorkflowFailureStrategy::Pause => {
                                            instance.status = WorkflowStatus::Paused;
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
                        false
                    } else {
                        true
                    }
                }
            };

            if !should_continue {
                break;
            }

            sleep(Duration::from_millis(100)).await;
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
            .ok_or_else(|| WorkflowError::WorkflowNotFound(instance_id.to_string()))?;

        instance.status = WorkflowStatus::Rollback;

        // Rollback tasks in reverse order
        let completed_tasks: Vec<_> = instance.task_states
            .iter()
            .filter(|(_, state)| state.status == TaskStatus::Completed)
            .map(|(id, _)| id.clone())
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
            .ok_or_else(|| WorkflowError::WorkflowNotFound(instance_id.to_string()))?;

        if instance.status == WorkflowStatus::Running {
            instance.status = WorkflowStatus::Paused;
            Ok(())
        } else {
            Err(WorkflowError::InvalidStateTransition)
        }
    }

    /// Resume a paused workflow instance.
    pub async fn resume_workflow(&self, instance_id: &str) -> Result<(), WorkflowError> {
        let mut instances = self.instances.write().await;
        let instance = instances.get_mut(instance_id)
            .ok_or_else(|| WorkflowError::WorkflowNotFound(instance_id.to_string()))?;

        if instance.status == WorkflowStatus::Paused {
            instance.status = WorkflowStatus::Running;
            Ok(())
        } else {
            Err(WorkflowError::InvalidStateTransition)
        }
    }

    /// Cancel a workflow instance.
    pub async fn cancel_workflow(&self, instance_id: &str) -> Result<(), WorkflowError> {
        let mut instances = self.instances.write().await;
        let instance = instances.get_mut(instance_id)
            .ok_or_else(|| WorkflowError::WorkflowNotFound(instance_id.to_string()))?;

        if instance.status == WorkflowStatus::Running || instance.status == WorkflowStatus::Paused {
            instance.status = WorkflowStatus::Cancelled;
            Ok(())
        } else {
            Err(WorkflowError::InvalidStateTransition)
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
}

impl Clone for WorkflowEngine {
    fn clone(&self) -> Self {
        Self {
            workflows: self.workflows.clone(),
            instances: self.instances.clone(),
            scheduler: self.scheduler.clone(),
            max_concurrent: self.max_concurrent,
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

    /// Wait for workflow completion.
    pub async fn await_completion(&self) -> Result<WorkflowResult, WorkflowError> {
        loop {
            let instance = self.engine.get_instance(&self.instance_id).await
                .ok_or_else(|| WorkflowError::WorkflowNotFound(self.instance_id.clone()))?;

            if instance.is_complete() {
                let result = WorkflowResult::from_instance(&instance);
                return Ok(result);
            }

            sleep(Duration::from_millis(100)).await;
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

        let workflow = Workflow {
            id: "test-wf".to_string(),
            name: "Test Workflow".to_string(),
            description: None,
            version: "1.0".to_string(),
            tasks: vec![
                Task {
                    id: Uuid::new_v4(),
                    name: "Task 1".to_string(),
                    task_type: "setup".to_string(),
                    parameters: serde_json::json!({}),
                    required_capabilities: vec![],
                    estimated_duration_secs: 1,
                    priority: 0,
                    deadline: None,
                    status: TaskStatus::Pending,
                    result: None,
                    error: None,
                    started_at: None,
                    finished_at: None,
                },
                Task {
                    id: Uuid::new_v4(),
                    name: "Task 2".to_string(),
                    task_type: "action".to_string(),
                    parameters: serde_json::json!({}),
                    required_capabilities: vec![],
                    estimated_duration_secs: 1,
                    priority: 0,
                    deadline: None,
                    status: TaskStatus::Pending,
                    result: None,
                    error: None,
                    started_at: None,
                    finished_at: None,
                },
            ],
            dependencies: vec![],
            status: WorkflowStatus::Draft,
            created_at: chrono::Utc::now(),
            started_at: None,
            finished_at: None,
            owner_agent_id: None,
            metadata: serde_json::json!({}),
        };

        engine.register_workflow(workflow).await.unwrap();

        let handle = engine.start_workflow("test-wf", HashMap::new()).await.unwrap();
        let result = handle.await_completion().await.unwrap();

        assert_eq!(result.status, WorkflowStatus::Completed);
        assert_eq!(result.completed_tasks, 2);
        assert_eq!(result.failed_tasks, 0);
    }
}
