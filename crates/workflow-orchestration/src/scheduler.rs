//! Workflow scheduling algorithms.

use std::collections::{HashMap, HashSet};
use dashmap::DashMap;
use petgraph::algo::toposort;
use petgraph::graph::DiGraph;

use crate::error::{WorkflowError, Result};
use crate::model::{Workflow, Task, TaskId, TaskStatus};

/// Scheduler that decides which tasks to run and on which agents.
pub struct WorkflowScheduler {
    /// Mapping from task ID to candidate agents.
    task_assignments: DashMap<TaskId, Vec<crate::common::types::AgentId>>,
    /// Mapping from agent ID to its capabilities.
    agent_capabilities: DashMap<crate::common::types::AgentId, HashSet<String>>,
}

impl WorkflowScheduler {
    /// Create a new scheduler.
    pub fn new() -> Self {
        Self {
            task_assignments: DashMap::new(),
            agent_capabilities: DashMap::new(),
        }
    }

    /// Register an agent with its capabilities.
    pub fn register_agent(
        &self,
        agent_id: crate::common::types::AgentId,
        capabilities: HashSet<String>,
    ) {
        self.agent_capabilities.insert(agent_id, capabilities);
    }

    /// Unregister an agent.
    pub fn unregister_agent(&self, agent_id: crate::common::types::AgentId) {
        self.agent_capabilities.remove(&agent_id);
    }

    /// Schedule tasks of a workflow.
    /// Returns a mapping from task ID to selected agent ID (if any).
    pub fn schedule(&self, workflow: &Workflow) -> HashMap<TaskId, Option<crate::common::types::AgentId>> {
        let mut assignments = HashMap::new();
        let graph = workflow.dependency_graph();

        // Topological order ensures dependencies are respected.
        let order = match toposort(&graph, None) {
            Ok(order) => order,
            Err(_) => return assignments, // cycle, cannot schedule
        };

        for node_idx in order {
            let task = &graph[node_idx];
            if task.status != TaskStatus::Pending && task.status != TaskStatus::Waiting {
                assignments.insert(task.id, None);
                continue;
            }

            // Find agents that have the required capabilities.
            let required: HashSet<String> = task.required_capabilities.iter().cloned().collect();
            let mut candidates = Vec::new();
            for entry in self.agent_capabilities.iter() {
                let agent_id = *entry.key();
                let capabilities = entry.value();
                if required.is_subset(capabilities) {
                    candidates.push(agent_id);
                }
            }

            // Simple round‑robin selection (could be replaced with more sophisticated algorithm).
            let selected = candidates.first().copied();
            assignments.insert(task.id, selected);
            if let Some(agent) = selected {
                self.task_assignments.insert(task.id, vec![agent]);
            }
        }

        assignments
    }

    /// Get the assigned agents for a task.
    pub fn get_assigned_agents(&self, task_id: TaskId) -> Option<Vec<crate::common::types::AgentId>> {
        self.task_assignments.get(&task_id).map(|v| v.clone())
    }

    /// Update task status and free resources if needed.
    pub fn update_task_status(
        &self,
        task_id: TaskId,
        status: TaskStatus,
    ) {
        if status == TaskStatus::Completed || status == TaskStatus::Failed || status == TaskStatus::Cancelled {
            // Task finished, remove assignment
            self.task_assignments.remove(&task_id);
        }
    }
}

impl Default for WorkflowScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Task;

    #[test]
    fn test_scheduler_register() {
        let scheduler = WorkflowScheduler::new();
        let mut caps = HashSet::new();
        caps.insert("compute".to_string());
        scheduler.register_agent(1, caps);
        assert_eq!(scheduler.agent_capabilities.len(), 1);
    }

    #[test]
    fn test_schedule_simple() {
        let scheduler = WorkflowScheduler::new();
        let mut caps = HashSet::new();
        caps.insert("compute".to_string());
        scheduler.register_agent(42, caps);

        let mut workflow = Workflow::new("test", "");
        let task = Task::new("t1", "compute", serde_json::json!({}));
        task.required_capabilities.push("compute".to_string());
        let task_id = workflow.add_task(task);

        let assignments = scheduler.schedule(&workflow);
        assert_eq!(assignments.get(&task_id).unwrap().unwrap(), 42);
    }
}