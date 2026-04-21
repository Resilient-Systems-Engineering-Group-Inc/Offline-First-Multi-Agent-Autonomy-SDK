//! Task lifecycle management for distributed planner.
//!
//! Provides state machine for task transitions and lifecycle operations.

use crate::{Task, Assignment, AssignmentStatus};
use common::types::AgentId;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{info, warn, error};

/// Task state machine states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskState {
    Pending,
    Ready,
    Assigned,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

/// Lifecycle event for a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LifecycleEvent {
    TaskCreated { task_id: String },
    TaskAssigned { task_id: String, agent_id: AgentId },
    TaskStarted { task_id: String, agent_id: AgentId },
    TaskCompleted { task_id: String, agent_id: AgentId, duration_secs: u64 },
    TaskFailed { task_id: String, agent_id: AgentId, reason: String },
    TaskCancelled { task_id: String, reason: String },
    TaskRetried { task_id: String, attempt: u32 },
}

/// Task lifecycle manager.
pub struct TaskLifecycleManager {
    task_states: Arc<RwLock<HashMap<String, TaskState>>>,
    assignment_states: Arc<RwLock<HashMap<String, AssignmentStatus>>>,
    retry_counts: Arc<RwLock<HashMap<String, u32>>>,
    max_retries: u32,
    event_callbacks: Vec<Box<dyn Fn(LifecycleEvent) + Send + Sync>>,
}

impl Default for TaskLifecycleManager {
    fn default() -> Self {
        Self::new(3) // Default max 3 retries
    }
}

impl TaskLifecycleManager {
    pub fn new(max_retries: u32) -> Self {
        Self {
            task_states: Arc::new(RwLock::new(HashMap::new())),
            assignment_states: Arc::new(RwLock::new(HashMap::new())),
            retry_counts: Arc::new(RwLock::new(HashMap::new())),
            max_retries,
            event_callbacks: Vec::new(),
        }
    }

    /// Register a callback for lifecycle events.
    pub fn on_event<F>(&mut self, callback: F)
    where
        F: Fn(LifecycleEvent) + Send + Sync + 'static,
    {
        self.event_callbacks.push(Box::new(callback));
    }

    fn emit_event(&self, event: LifecycleEvent) {
        for callback in &self.event_callbacks {
            callback(event.clone());
        }
    }

    /// Register a new task.
    pub async fn register_task(&self, task_id: &str) {
        let mut states = self.task_states.write().await;
        states.insert(task_id.to_string(), TaskState::Pending);
        info!("Task {} registered in Pending state", task_id);
    }

    /// Check if task is ready to be assigned (dependencies satisfied).
    pub async fn is_task_ready(&self, task: &Task, completed_tasks: &HashSet<String>) -> bool {
        task.dependencies.iter().all(|dep| completed_tasks.contains(dep))
    }

    /// Transition task to Assigned state.
    pub async fn assign_task(&self, task_id: &str, agent_id: AgentId) -> Result<(), LifecycleError> {
        let mut states = self.task_states.write().await;
        
        if let Some(state) = states.get_mut(task_id) {
            match state {
                TaskState::Pending | TaskState::Ready => {
                    *state = TaskState::Assigned;
                    let mut assignments = self.assignment_states.write().await;
                    assignments.insert(task_id.to_string(), AssignmentStatus::Assigned);
                    self.emit_event(LifecycleEvent::TaskAssigned {
                        task_id: task_id.to_string(),
                        agent_id,
                    });
                    info!("Task {} assigned to agent {}", task_id, agent_id);
                    Ok(())
                }
                _ => Err(LifecycleError::InvalidTransition {
                    task_id: task_id.to_string(),
                    from: *state,
                    to: TaskState::Assigned,
                }),
            }
        } else {
            Err(LifecycleError::TaskNotFound(task_id.to_string()))
        }
    }

    /// Transition task to InProgress state.
    pub async fn start_task(&self, task_id: &str, agent_id: AgentId) -> Result<(), LifecycleError> {
        let mut states = self.task_states.write().await;
        
        if let Some(state) = states.get_mut(task_id) {
            match state {
                TaskState::Assigned => {
                    *state = TaskState::InProgress;
                    let mut assignments = self.assignment_states.write().await;
                    assignments.insert(task_id.to_string(), AssignmentStatus::InProgress);
                    self.emit_event(LifecycleEvent::TaskStarted {
                        task_id: task_id.to_string(),
                        agent_id,
                    });
                    info!("Task {} started by agent {}", task_id, agent_id);
                    Ok(())
                }
                _ => Err(LifecycleError::InvalidTransition {
                    task_id: task_id.to_string(),
                    from: *state,
                    to: TaskState::InProgress,
                }),
            }
        } else {
            Err(LifecycleError::TaskNotFound(task_id.to_string()))
        }
    }

    /// Transition task to Completed state.
    pub async fn complete_task(&self, task_id: &str, agent_id: AgentId, duration_secs: u64) -> Result<(), LifecycleError> {
        let mut states = self.task_states.write().await;
        
        if let Some(state) = states.get_mut(task_id) {
            match state {
                TaskState::InProgress => {
                    *state = TaskState::Completed;
                    let mut assignments = self.assignment_states.write().await;
                    assignments.insert(task_id.to_string(), AssignmentStatus::Completed);
                    
                    // Reset retry count
                    let mut retries = self.retry_counts.write().await;
                    retries.remove(task_id);
                    
                    self.emit_event(LifecycleEvent::TaskCompleted {
                        task_id: task_id.to_string(),
                        agent_id,
                        duration_secs,
                    });
                    info!("Task {} completed by agent {} in {}s", task_id, agent_id, duration_secs);
                    Ok(())
                }
                _ => Err(LifecycleError::InvalidTransition {
                    task_id: task_id.to_string(),
                    from: *state,
                    to: TaskState::Completed,
                }),
            }
        } else {
            Err(LifecycleError::TaskNotFound(task_id.to_string()))
        }
    }

    /// Transition task to Failed state with retry logic.
    pub async fn fail_task(&self, task_id: &str, agent_id: AgentId, reason: String) -> Result<RetryDecision, LifecycleError> {
        let mut states = self.task_states.write().await;
        
        if let Some(state) = states.get_mut(task_id) {
            match state {
                TaskState::InProgress => {
                    // Increment retry count
                    let mut retries = self.retry_counts.write().await;
                    let count = retries.entry(task_id.to_string()).or_insert(0);
                    *count += 1;
                    let attempt = *count;

                    if attempt <= self.max_retries {
                        // Retry
                        *state = TaskState::Pending;
                        self.emit_event(LifecycleEvent::TaskRetried {
                            task_id: task_id.to_string(),
                            attempt,
                        });
                        warn!("Task {} failed (attempt {}/{}): {}", task_id, attempt, self.max_retries, reason);
                        Ok(RetryDecision::Retry(attempt))
                    } else {
                        // Max retries exceeded
                        *state = TaskState::Failed;
                        let mut assignments = self.assignment_states.write().await;
                        assignments.insert(task_id.to_string(), AssignmentStatus::Failed);
                        self.emit_event(LifecycleEvent::TaskFailed {
                            task_id: task_id.to_string(),
                            agent_id,
                            reason,
                        });
                        error!("Task {} failed after {} attempts: {}", task_id, attempt, reason);
                        Ok(RetryDecision::MaxRetriesExceeded)
                    }
                }
                _ => Err(LifecycleError::InvalidTransition {
                    task_id: task_id.to_string(),
                    from: *state,
                    to: TaskState::Failed,
                }),
            }
        } else {
            Err(LifecycleError::TaskNotFound(task_id.to_string()))
        }
    }

    /// Cancel a task.
    pub async fn cancel_task(&self, task_id: &str, reason: String) -> Result<(), LifecycleError> {
        let mut states = self.task_states.write().await;
        
        if let Some(state) = states.get_mut(task_id) {
            match state {
                TaskState::Pending | TaskState::Ready | TaskState::Assigned | TaskState::InProgress => {
                    *state = TaskState::Cancelled;
                    let mut assignments = self.assignment_states.write().await;
                    assignments.insert(task_id.to_string(), AssignmentStatus::Failed);
                    self.emit_event(LifecycleEvent::TaskCancelled {
                        task_id: task_id.to_string(),
                        reason,
                    });
                    info!("Task {} cancelled: {}", task_id, reason);
                    Ok(())
                }
                _ => Err(LifecycleError::InvalidTransition {
                    task_id: task_id.to_string(),
                    from: *state,
                    to: TaskState::Cancelled,
                }),
            }
        } else {
            Err(LifecycleError::TaskNotFound(task_id.to_string()))
        }
    }

    /// Get current state of a task.
    pub async fn get_task_state(&self, task_id: &str) -> Option<TaskState> {
        let states = self.task_states.read().await;
        states.get(task_id).copied()
    }

    /// Get all completed task IDs.
    pub async fn get_completed_tasks(&self) -> HashSet<String> {
        let states = self.task_states.read().await;
        states.iter()
            .filter(|(_, state)| **state == TaskState::Completed)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Get retry count for a task.
    pub async fn get_retry_count(&self, task_id: &str) -> u32 {
        let retries = self.retry_counts.read().await;
        *retries.get(task_id).unwrap_or(&0)
    }
}

/// Decision about task retry.
#[derive(Debug, Clone)]
pub enum RetryDecision {
    Retry(u32), // Attempt number
    MaxRetriesExceeded,
}

/// Lifecycle error types.
#[derive(Debug, Clone)]
pub enum LifecycleError {
    TaskNotFound(String),
    InvalidTransition {
        task_id: String,
        from: TaskState,
        to: TaskState,
    },
}

impl std::fmt::Display for LifecycleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LifecycleError::TaskNotFound(id) => write!(f, "Task not found: {}", id),
            LifecycleError::InvalidTransition { task_id, from, to } => {
                write!(f, "Invalid transition for task {}: {:?} -> {:?}", task_id, from, to)
            }
        }
    }
}

impl std::error::Error for LifecycleError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_task_lifecycle() {
        let manager = TaskLifecycleManager::new(3);
        let task_id = "task-1";
        let agent_id = "agent-1";

        // Register task
        manager.register_task(task_id).await;
        assert_eq!(manager.get_task_state(task_id).await, Some(TaskState::Pending));

        // Assign task
        manager.assign_task(task_id, agent_id).await.unwrap();
        assert_eq!(manager.get_task_state(task_id).await, Some(TaskState::Assigned));

        // Start task
        manager.start_task(task_id, agent_id).await.unwrap();
        assert_eq!(manager.get_task_state(task_id).await, Some(TaskState::InProgress));

        // Complete task
        manager.complete_task(task_id, agent_id, 10).await.unwrap();
        assert_eq!(manager.get_task_state(task_id).await, Some(TaskState::Completed));
    }

    #[tokio::test]
    async fn test_task_retry() {
        let manager = TaskLifecycleManager::new(2);
        let task_id = "task-2";
        let agent_id = "agent-1";

        manager.register_task(task_id).await;
        manager.assign_task(task_id, agent_id).await.unwrap();
        manager.start_task(task_id, agent_id).await.unwrap();

        // First failure - should retry
        let decision = manager.fail_task(task_id, agent_id, "test error".to_string()).await.unwrap();
        assert!(matches!(decision, RetryDecision::Retry(1)));

        // Second failure - should retry
        let decision = manager.fail_task(task_id, agent_id, "test error".to_string()).await.unwrap();
        assert!(matches!(decision, RetryDecision::Retry(2)));

        // Third failure - max retries exceeded
        let decision = manager.fail_task(task_id, agent_id, "test error".to_string()).await.unwrap();
        assert!(matches!(decision, RetryDecision::MaxRetriesExceeded));
    }
}
