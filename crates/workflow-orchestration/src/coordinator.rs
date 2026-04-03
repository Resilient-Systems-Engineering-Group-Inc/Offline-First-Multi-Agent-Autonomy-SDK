//! Distributed coordination of workflows across agents.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use dashmap::DashMap;

use crate::error::{WorkflowError, Result};
use crate::model::{Workflow, WorkflowId, Task, TaskId, TaskStatus, WorkflowStatus};
use crate::scheduler::WorkflowScheduler;
use crate::executor::ExecutionEngine;

/// Distributed coordinator that manages workflows across multiple agents.
pub struct DistributedCoordinator {
    /// Local agent ID.
    local_agent_id: crate::common::types::AgentId,
    /// Scheduler for task assignment.
    scheduler: Arc<WorkflowScheduler>,
    /// Execution engine.
    engine: Arc<ExecutionEngine>,
    /// Known workflows.
    workflows: Arc<RwLock<HashMap<WorkflowId, Workflow>>>,
    /// Mapping from task ID to workflow ID.
    task_to_workflow: Arc<DashMap<TaskId, WorkflowId>>,
    /// Mapping from workflow ID to its status.
    workflow_status: Arc<DashMap<WorkflowId, WorkflowStatus>>,
    /// Peers (other coordinators).
    peers: Arc<Mutex<HashSet<crate::common::types::AgentId>>>,
}

impl DistributedCoordinator {
    /// Create a new coordinator.
    pub fn new(
        local_agent_id: crate::common::types::AgentId,
        scheduler: Arc<WorkflowScheduler>,
        engine: Arc<ExecutionEngine>,
    ) -> Self {
        Self {
            local_agent_id,
            scheduler,
            engine,
            workflows: Arc::new(RwLock::new(HashMap::new())),
            task_to_workflow: Arc::new(DashMap::new()),
            workflow_status: Arc::new(DashMap::new()),
            peers: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Add a peer coordinator.
    pub async fn add_peer(&self, agent_id: crate::common::types::AgentId) {
        let mut peers = self.peers.lock().await;
        peers.insert(agent_id);
    }

    /// Remove a peer.
    pub async fn remove_peer(&self, agent_id: crate::common::types::AgentId) {
        let mut peers = self.peers.lock().await;
        peers.remove(&agent_id);
    }

    /// Submit a new workflow for execution.
    pub async fn submit_workflow(&self, mut workflow: Workflow) -> Result<WorkflowId> {
        workflow.status = WorkflowStatus::Scheduled;
        workflow.started_at = Some(chrono::Utc::now());
        let workflow_id = workflow.id;

        // Store workflow
        {
            let mut workflows = self.workflows.write().await;
            workflows.insert(workflow_id, workflow);
        }

        // Schedule tasks
        self.schedule_workflow(workflow_id).await?;

        Ok(workflow_id)
    }

    /// Schedule tasks of a workflow.
    async fn schedule_workflow(&self, workflow_id: WorkflowId) -> Result<()> {
        let workflow = {
            let workflows = self.workflows.read().await;
            workflows.get(&workflow_id).cloned().ok_or_else(|| WorkflowError::NotFound(format!("workflow {}", workflow_id)))?
        };

        let assignments = self.scheduler.schedule(&workflow);
        for (task_id, maybe_agent) in assignments {
            if let Some(agent_id) = maybe_agent {
                if agent_id == self.local_agent_id {
                    // This agent should execute the task
                    self.execute_local_task(task_id).await?;
                } else {
                    // Forward task to peer (in a real implementation, send message)
                    // For now, we just log
                    tracing::info!("Task {} assigned to peer {}", task_id, agent_id);
                }
            } else {
                tracing::warn!("No agent found for task {}", task_id);
            }
        }

        Ok(())
    }

    /// Execute a task locally.
    async fn execute_local_task(&self, task_id: TaskId) -> Result<()> {
        // Find the workflow containing this task
        let workflow_id = self.task_to_workflow.get(&task_id).map(|e| *e.value()).ok_or_else(|| WorkflowError::NotFound(format!("task {}", task_id)))?;
        let task = {
            let workflows = self.workflows.read().await;
            let workflow = workflows.get(&workflow_id).ok_or_else(|| WorkflowError::NotFound(format!("workflow {}", workflow_id)))?;
            workflow.tasks.iter().find(|t| t.id == task_id).cloned().ok_or_else(|| WorkflowError::NotFound(format!("task {}", task_id)))?
        };

        // Update task status to running
        self.update_task_status(task_id, TaskStatus::Running).await?;

        // Execute
        let engine = self.engine.clone();
        let task_to_workflow = self.task_to_workflow.clone();
        let workflows = self.workflows.clone();
        tokio::spawn(async move {
            match engine.start_task(task).await {
                Ok(_) => {
                    // Task completed successfully (status updated by engine)
                }
                Err(e) => {
                    tracing::error!("Task execution failed: {}", e);
                    // In a real implementation, we'd update task status to failed
                }
            }
        });

        Ok(())
    }

    /// Update task status and propagate.
    async fn update_task_status(&self, task_id: TaskId, status: TaskStatus) -> Result<()> {
        let workflow_id = self.task_to_workflow.get(&task_id).map(|e| *e.value()).ok_or_else(|| WorkflowError::NotFound(format!("task {}", task_id)))?;
        {
            let mut workflows = self.workflows.write().await;
            let workflow = workflows.get_mut(&workflow_id).ok_or_else(|| WorkflowError::NotFound(format!("workflow {}", workflow_id)))?;
            for task in &mut workflow.tasks {
                if task.id == task_id {
                    task.status = status;
                    task.started_at = Some(chrono::Utc::now());
                    break;
                }
            }
        }
        self.scheduler.update_task_status(task_id, status);
        Ok(())
    }

    /// Get workflow status.
    pub async fn get_workflow_status(&self, workflow_id: WorkflowId) -> Option<WorkflowStatus> {
        self.workflow_status.get(&workflow_id).map(|e| *e.value())
    }

    /// Get all workflows.
    pub async fn list_workflows(&self) -> Vec<Workflow> {
        let workflows = self.workflows.read().await;
        workflows.values().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::DummyExecutor;

    #[tokio::test]
    async fn test_coordinator_submit() {
        let scheduler = Arc::new(WorkflowScheduler::new());
        let executor = Arc::new(DummyExecutor::new());
        let engine = Arc::new(ExecutionEngine::new(executor));
        let coordinator = DistributedCoordinator::new(1, scheduler, engine);
        let workflow = Workflow::new("test", "");
        let workflow_id = coordinator.submit_workflow(workflow).await.unwrap();
        assert!(coordinator.get_workflow_status(workflow_id).await.is_some());
    }
}