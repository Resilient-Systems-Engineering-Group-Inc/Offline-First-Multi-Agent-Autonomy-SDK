//! Workflow and task data models.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::{HashMap, HashSet};

/// Unique identifier for a workflow.
pub type WorkflowId = Uuid;

/// Unique identifier for a task.
pub type TaskId = Uuid;

/// Status of a task.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TaskStatus {
    /// Task is pending execution.
    Pending,
    /// Task is currently running.
    Running,
    /// Task completed successfully.
    Completed,
    /// Task failed.
    Failed,
    /// Task was cancelled.
    Cancelled,
    /// Task is waiting for dependencies.
    Waiting,
    /// Task was rolled back.
    RolledBack,
}

/// Status of a workflow.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum WorkflowStatus {
    /// Workflow is being defined.
    Draft,
    /// Workflow is scheduled but not yet running.
    Scheduled,
    /// Workflow is currently executing.
    Running,
    /// Workflow completed successfully.
    Completed,
    /// Workflow failed.
    Failed,
    /// Workflow was cancelled.
    Cancelled,
    /// Workflow is paused.
    Paused,
    /// Workflow is being rolled back.
    Rollback,
}

/// Failure strategy for workflow tasks.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum WorkflowFailureStrategy {
    /// Fail the entire workflow immediately.
    Fail,
    /// Continue with remaining tasks.
    Continue,
    /// Rollback all completed tasks.
    Rollback,
    /// Pause the workflow for manual intervention.
    Pause,
}

impl Default for WorkflowFailureStrategy {
    fn default() -> Self {
        Self::Fail
    }
}

/// Runtime state of a task within a workflow instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskState {
    /// Task ID.
    pub id: TaskId,
    /// Current status.
    pub status: TaskStatus,
    /// Task output (if completed).
    pub output: Option<HashMap<String, String>>,
    /// Error message (if failed).
    pub error: Option<String>,
    /// When the task started.
    pub started_at: Option<u64>,
    /// When the task finished.
    pub finished_at: Option<u64>,
}

impl TaskState {
    /// Create a new task state for a given task.
    pub fn new(task: &Task) -> Self {
        Self {
            id: task.id,
            status: TaskStatus::Pending,
            output: None,
            error: None,
            started_at: None,
            finished_at: None,
        }
    }
}

/// A task within a workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique task ID.
    pub id: TaskId,
    /// Human‑readable name.
    pub name: String,
    /// Task type (e.g., "compute", "io", "network").
    pub task_type: String,
    /// Parameters (JSON).
    pub parameters: serde_json::Value,
    /// Required capabilities.
    pub required_capabilities: Vec<String>,
    /// Estimated duration in seconds.
    pub estimated_duration_secs: u64,
    /// Priority (higher = more important).
    pub priority: i32,
    /// Deadline (optional).
    pub deadline: Option<DateTime<Utc>>,
    /// Current status.
    pub status: TaskStatus,
    /// Result (if completed).
    pub result: Option<serde_json::Value>,
    /// Error message (if failed).
    pub error: Option<String>,
    /// Start time.
    pub started_at: Option<DateTime<Utc>>,
    /// Finish time.
    pub finished_at: Option<DateTime<Utc>>,
}

impl Task {
    /// Create a new task.
    pub fn new(
        name: impl Into<String>,
        task_type: impl Into<String>,
        parameters: serde_json::Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            task_type: task_type.into(),
            parameters,
            required_capabilities: Vec::new(),
            estimated_duration_secs: 0,
            priority: 0,
            deadline: None,
            status: TaskStatus::Pending,
            result: None,
            error: None,
            started_at: None,
            finished_at: None,
        }
    }

    /// Check if the task is ready to run (all dependencies satisfied).
    pub fn is_ready(&self) -> bool {
        self.status == TaskStatus::Pending || self.status == TaskStatus::Waiting
    }
}

/// A workflow consisting of tasks and dependencies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    /// Unique workflow ID.
    pub id: WorkflowId,
    /// Human‑readable name.
    pub name: String,
    /// Description.
    pub description: String,
    /// List of tasks.
    pub tasks: Vec<Task>,
    /// Dependencies as (from_task_id, to_task_id) pairs.
    pub dependencies: Vec<(TaskId, TaskId)>,
    /// Current status.
    pub status: WorkflowStatus,
    /// Created timestamp.
    pub created_at: DateTime<Utc>,
    /// Started timestamp.
    pub started_at: Option<DateTime<Utc>>,
    /// Finished timestamp.
    pub finished_at: Option<DateTime<Utc>>,
    /// Owner/creator agent ID.
    pub owner_agent_id: Option<u64>,
    /// Metadata (JSON).
    pub metadata: serde_json::Value,
    /// Failure strategy for task failures.
    #[serde(default)]
    pub on_failure: WorkflowFailureStrategy,
}

impl Workflow {
    /// Create a new workflow.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            description: description.into(),
            tasks: Vec::new(),
            dependencies: Vec::new(),
            status: WorkflowStatus::Draft,
            created_at: Utc::now(),
            started_at: None,
            finished_at: None,
            owner_agent_id: None,
            metadata: serde_json::json!({}),
            on_failure: WorkflowFailureStrategy::Fail,
        }
    }

    /// Add a task to the workflow.
    pub fn add_task(&mut self, task: Task) -> TaskId {
        let id = task.id;
        self.tasks.push(task);
        id
    }

    /// Add a dependency between two tasks.
    pub fn add_dependency(&mut self, from: TaskId, to: TaskId) {
        self.dependencies.push((from, to));
    }

    /// Build a dependency graph.
    pub fn dependency_graph(&self) -> DiGraph<&Task, ()> {
        let mut graph = DiGraph::new();
        let mut node_indices = HashMap::new();
        for task in &self.tasks {
            let idx = graph.add_node(task);
            node_indices.insert(task.id, idx);
        }
        for (from, to) in &self.dependencies {
            if let (Some(&from_idx), Some(&to_idx)) = (node_indices.get(from), node_indices.get(to)) {
                graph.add_edge(from_idx, to_idx, ());
            }
        }
        graph
    }

    /// Get tasks that are ready to run (no pending dependencies).
    pub fn ready_tasks(&self) -> Vec<&Task> {
        let graph = self.dependency_graph();
        let mut ready = Vec::new();
        for task in &self.tasks {
            if task.is_ready() {
                // Check if all predecessors are completed
                let node_idx = graph.node_indices().find(|&idx| graph[idx].id == task.id).unwrap();
                let predecessors = graph.neighbors_directed(node_idx, petgraph::Direction::Incoming);
                let all_predecessors_completed = predecessors
                    .map(|pred_idx| graph[pred_idx].status == TaskStatus::Completed)
                    .all(|x| x);
                if all_predecessors_completed {
                    ready.push(task);
                }
            }
        }
        ready
    }

    /// Get the next set of tasks that can be executed given completed tasks.
    pub fn get_next_tasks(&self, completed_tasks: &HashSet<TaskId>) -> Vec<Task> {
        let graph = self.dependency_graph();
        let mut next = Vec::new();

        for task in &self.tasks {
            if completed_tasks.contains(&task.id) {
                continue;
            }
            if task.status != TaskStatus::Pending && task.status != TaskStatus::Waiting {
                continue;
            }

            // Check if all predecessors are completed
            let node_idx = match graph.node_indices().find(|&idx| graph[idx].id == task.id) {
                Some(idx) => idx,
                None => continue,
            };
            let predecessors = graph.neighbors_directed(node_idx, petgraph::Direction::Incoming);
            let all_predecessors_completed = predecessors
                .all(|pred_idx| completed_tasks.contains(&graph[pred_idx].id));

            if all_predecessors_completed {
                next.push(task.clone());
            }
        }

        next
    }

    /// Validate the workflow definition.
    pub fn validate(&self) -> Result<(), crate::error::WorkflowError> {
        if self.tasks.is_empty() {
            return Err(crate::error::WorkflowError::InvalidDefinition(
                "Workflow must have at least one task".to_string(),
            ));
        }

        // Check for duplicate task IDs
        let mut task_ids = HashSet::new();
        for task in &self.tasks {
            if !task_ids.insert(task.id) {
                return Err(crate::error::WorkflowError::InvalidDefinition(
                    format!("Duplicate task ID: {}", task.id),
                ));
            }
        }

        // Check that all dependencies reference valid tasks
        for (from, to) in &self.dependencies {
            if !task_ids.contains(from) {
                return Err(crate::error::WorkflowError::InvalidDefinition(
                    format!("Dependency references unknown task: {}", from),
                ));
            }
            if !task_ids.contains(to) {
                return Err(crate::error::WorkflowError::InvalidDefinition(
                    format!("Dependency references unknown task: {}", to),
                ));
            }
        }

        // Check for cycles using topological sort
        let graph = self.dependency_graph();
        if petgraph::algo::toposort(&graph, None).is_err() {
            return Err(crate::error::WorkflowError::InvalidDefinition(
                "Workflow contains a cycle in task dependencies".to_string(),
            ));
        }

        Ok(())
    }
}

/// A running instance of a workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowInstance {
    /// Unique instance ID.
    pub instance_id: String,
    /// Workflow ID.
    pub workflow_id: WorkflowId,
    /// Current status.
    pub status: WorkflowStatus,
    /// Task states.
    pub task_states: HashMap<TaskId, TaskState>,
    /// Parameters passed to this instance.
    pub parameters: HashMap<String, String>,
    /// Error message (if failed).
    pub error: Option<String>,
    /// When the instance was created.
    pub created_at: u64,
    /// When the instance started.
    pub started_at: Option<u64>,
    /// When the instance completed.
    pub completed_at: Option<u64>,
}

impl WorkflowInstance {
    /// Create a new workflow instance from a workflow definition.
    pub fn new(workflow: &Workflow, parameters: HashMap<String, String>) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let task_states: HashMap<_, _> = workflow
            .tasks
            .iter()
            .map(|task| (task.id, TaskState::new(task)))
            .collect();

        Self {
            instance_id: Uuid::new_v4().to_string(),
            workflow_id: workflow.id,
            status: WorkflowStatus::Running,
            task_states,
            parameters,
            error: None,
            created_at: now,
            started_at: Some(now),
            completed_at: None,
        }
    }

    /// Check if the workflow instance is complete.
    pub fn is_complete(&self) -> bool {
        matches!(
            self.status,
            WorkflowStatus::Completed
                | WorkflowStatus::Failed
                | WorkflowStatus::Cancelled
                | WorkflowStatus::Rollback
        )
    }

    /// Get the set of completed task IDs.
    pub fn get_completed_tasks(&self) -> HashSet<TaskId> {
        self.task_states
            .iter()
            .filter(|(_, state)| state.status == TaskStatus::Completed)
            .map(|(id, _)| *id)
            .collect()
    }

    /// Mark a task as started.
    pub fn mark_task_started(&mut self, task_id: &TaskId) -> Result<(), crate::error::WorkflowError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let state = self.task_states.get_mut(task_id).ok_or_else(|| {
            crate::error::WorkflowError::NotFound(format!("Task {} not found", task_id))
        })?;

        state.status = TaskStatus::Running;
        state.started_at = Some(now);
        Ok(())
    }

    /// Mark a task as completed with output.
    pub fn mark_task_completed(
        &mut self,
        task_id: &TaskId,
        output: HashMap<String, String>,
    ) -> Result<(), crate::error::WorkflowError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let state = self.task_states.get_mut(task_id).ok_or_else(|| {
            crate::error::WorkflowError::NotFound(format!("Task {} not found", task_id))
        })?;

        state.status = TaskStatus::Completed;
        state.output = Some(output);
        state.finished_at = Some(now);
        Ok(())
    }

    /// Mark a task as failed.
    pub fn mark_task_failed(
        &mut self,
        task_id: &TaskId,
        error: &str,
    ) -> Result<(), crate::error::WorkflowError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let state = self.task_states.get_mut(task_id).ok_or_else(|| {
            crate::error::WorkflowError::NotFound(format!("Task {} not found", task_id))
        })?;

        state.status = TaskStatus::Failed;
        state.error = Some(error.to_string());
        state.finished_at = Some(now);
        Ok(())
    }

    /// Calculate progress as a percentage (0.0 - 100.0).
    pub fn progress(&self) -> f64 {
        if self.task_states.is_empty() {
            return 100.0;
        }

        let completed = self
            .task_states
            .values()
            .filter(|s| s.status == TaskStatus::Completed)
            .count();

        (completed as f64 / self.task_states.len() as f64) * 100.0
    }
}

/// Result of a completed workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowResult {
    /// Instance ID.
    pub instance_id: String,
    /// Workflow ID.
    pub workflow_id: WorkflowId,
    /// Final status.
    pub status: WorkflowStatus,
    /// Number of completed tasks.
    pub completed_tasks: usize,
    /// Number of failed tasks.
    pub failed_tasks: usize,
    /// Total tasks.
    pub total_tasks: usize,
    /// Error message (if failed).
    pub error: Option<String>,
    /// Duration in seconds.
    pub duration_secs: u64,
    /// Task outputs.
    pub task_outputs: HashMap<TaskId, Option<HashMap<String, String>>>,
}

impl WorkflowResult {
    /// Create a workflow result from an instance.
    pub fn from_instance(instance: &WorkflowInstance) -> Self {
        let completed_tasks = instance
            .task_states
            .values()
            .filter(|s| s.status == TaskStatus::Completed)
            .count();
        let failed_tasks = instance
            .task_states
            .values()
            .filter(|s| s.status == TaskStatus::Failed)
            .count();

        let duration_secs = match (instance.started_at, instance.completed_at) {
            (Some(start), Some(end)) => end.saturating_sub(start),
            _ => 0,
        };

        let task_outputs = instance
            .task_states
            .iter()
            .map(|(id, state)| (*id, state.output.clone()))
            .collect();

        Self {
            instance_id: instance.instance_id.clone(),
            workflow_id: instance.workflow_id,
            status: instance.status,
            completed_tasks,
            failed_tasks,
            total_tasks: instance.task_states.len(),
            error: instance.error.clone(),
            duration_secs,
            task_outputs,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task = Task::new("test", "compute", serde_json::json!({}));
        assert_eq!(task.name, "test");
        assert_eq!(task.status, TaskStatus::Pending);
    }

    #[test]
    fn test_workflow_creation() {
        let workflow = Workflow::new("wf", "description");
        assert_eq!(workflow.name, "wf");
        assert_eq!(workflow.status, WorkflowStatus::Draft);
    }

    #[test]
    fn test_workflow_validation_empty_tasks() {
        let workflow = Workflow::new("empty", "");
        assert!(workflow.validate().is_err());
    }

    #[test]
    fn test_workflow_validation_cycle() {
        let mut workflow = Workflow::new("cycle", "");
        let t1 = workflow.add_task(Task::new("t1", "compute", serde_json::json!({})));
        let t2 = workflow.add_task(Task::new("t2", "compute", serde_json::json!({})));
        workflow.add_dependency(t1, t2);
        workflow.add_dependency(t2, t1); // Creates a cycle
        assert!(workflow.validate().is_err());
    }

    #[test]
    fn test_workflow_instance_creation() {
        let mut workflow = Workflow::new("test", "");
        workflow.add_task(Task::new("t1", "compute", serde_json::json!({})));
        workflow.add_task(Task::new("t2", "compute", serde_json::json!({})));

        let instance = WorkflowInstance::new(&workflow, HashMap::new());
        assert_eq!(instance.status, WorkflowStatus::Running);
        assert_eq!(instance.task_states.len(), 2);
        assert!(!instance.is_complete());
    }

    #[test]
    fn test_workflow_instance_progress() {
        let mut workflow = Workflow::new("test", "");
        let t1 = workflow.add_task(Task::new("t1", "compute", serde_json::json!({})));
        let t2 = workflow.add_task(Task::new("t2", "compute", serde_json::json!({})));

        let mut instance = WorkflowInstance::new(&workflow, HashMap::new());
        assert_eq!(instance.progress(), 0.0);

        instance.mark_task_completed(&t1, HashMap::new()).unwrap();
        assert_eq!(instance.progress(), 50.0);

        instance.mark_task_completed(&t2, HashMap::new()).unwrap();
        assert_eq!(instance.progress(), 100.0);
    }

    #[test]
    fn test_get_next_tasks() {
        let mut workflow = Workflow::new("test", "");
        let t1 = workflow.add_task(Task::new("t1", "compute", serde_json::json!({})));
        let t2 = workflow.add_task(Task::new("t2", "compute", serde_json::json!({})));
        workflow.add_dependency(t1, t2);

        // Initially only t1 should be ready
        let completed = HashSet::new();
        let next = workflow.get_next_tasks(&completed);
        assert_eq!(next.len(), 1);
        assert_eq!(next[0].id, t1);

        // After t1 completes, t2 should be ready
        let mut completed = HashSet::new();
        completed.insert(t1);
        let next = workflow.get_next_tasks(&completed);
        assert_eq!(next.len(), 1);
        assert_eq!(next[0].id, t2);
    }
}