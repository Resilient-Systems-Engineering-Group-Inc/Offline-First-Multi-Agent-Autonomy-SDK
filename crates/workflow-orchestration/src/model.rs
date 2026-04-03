//! Workflow and task data models.

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use petgraph::graph::{DiGraph, NodeIndex};

/// Unique identifier for a workflow.
pub type WorkflowId = Uuid;

/// Unique identifier for a task.
pub type TaskId = Uuid;

/// Status of a task.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
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
}

/// Status of a workflow.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
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
    pub owner_agent_id: Option<crate::common::types::AgentId>,
    /// Metadata (JSON).
    pub metadata: serde_json::Value,
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
        let mut node_indices = std::collections::HashMap::new();
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
}