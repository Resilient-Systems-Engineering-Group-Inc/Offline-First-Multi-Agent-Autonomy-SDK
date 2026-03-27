//! CRDT‑based synchronization of tasks and assignments.

use crate::{Task, Assignment, AgentId};
use state_sync::crdt_map::CrdtMap;
use serde_json::{Value, json};
use anyhow::Result;

/// Keys used in the CRDT map.
pub const TASKS_KEY: &str = "distributed_planner/tasks";
pub const ASSIGNMENTS_KEY: &str = "distributed_planner/assignments";

/// Synchronize tasks to/from the CRDT map.
pub struct TaskSync {
    crdt_map: CrdtMap,
    local_agent: AgentId,
}

impl TaskSync {
    pub fn new(crdt_map: CrdtMap, local_agent: AgentId) -> Self {
        Self { crdt_map, local_agent }
    }

    /// Publish a local task to the shared map.
    pub fn publish_task(&mut self, task: &Task) {
        let key = format!("{}/{}", TASKS_KEY, task.id);
        self.crdt_map.set(&key, json!(task), self.local_agent);
    }

    /// Remove a task from the shared map (e.g., when completed).
    pub fn remove_task(&mut self, task_id: &str) {
        let key = format!("{}/{}", TASKS_KEY, task_id);
        self.crdt_map.delete(&key, self.local_agent);
    }

    /// Retrieve all tasks from the shared map.
    pub fn get_all_tasks(&self) -> Vec<Task> {
        let hashmap: std::collections::HashMap<String, Value> = self.crdt_map.to_hashmap();
        let mut tasks = Vec::new();
        for (key, value) in hashmap {
            if key.starts_with(TASKS_KEY) {
                if let Ok(task) = serde_json::from_value(value) {
                    tasks.push(task);
                }
            }
        }
        tasks
    }

    /// Publish an assignment to the shared map.
    pub fn publish_assignment(&mut self, assignment: &Assignment) {
        let key = format!("{}/{}", ASSIGNMENTS_KEY, assignment.task_id);
        self.crdt_map.set(&key, json!(assignment), self.local_agent);
    }

    /// Remove an assignment from the shared map.
    pub fn remove_assignment(&mut self, task_id: &str) {
        let key = format!("{}/{}", ASSIGNMENTS_KEY, task_id);
        self.crdt_map.delete(&key, self.local_agent);
    }

    /// Retrieve all assignments from the shared map.
    pub fn get_all_assignments(&self) -> Vec<Assignment> {
        let hashmap: std::collections::HashMap<String, Value> = self.crdt_map.to_hashmap();
        let mut assignments = Vec::new();
        for (key, value) in hashmap {
            if key.starts_with(ASSIGNMENTS_KEY) {
                if let Ok(assignment) = serde_json::from_value(value) {
                    assignments.push(assignment);
                }
            }
        }
        assignments
    }

    /// Merge a delta from another agent into the local map.
    pub fn apply_delta(&mut self, delta: state_sync::delta::Delta) {
        self.crdt_map.apply_delta(delta);
    }

    /// Generate a delta representing changes since the given vector clock.
    pub fn delta_since(&self, since: &common::types::VectorClock) -> Option<state_sync::delta::Delta> {
        self.crdt_map.delta_since(since)
    }
}