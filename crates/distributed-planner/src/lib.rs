//! Distributed task planning for offline‑first multi‑agent systems.
//!
//! This module provides a planner that coordinates tasks across multiple agents
//! using consensus and shared state.

pub mod algorithms;
pub mod sync;

use common::types::{AgentId, VectorClock, Capability};
use mesh_transport::{MeshTransport, MeshTransportConfig};
use bounded_consensus::{BoundedConsensus, TwoPhaseBoundedConsensus, BoundedConsensusConfig, Proposal};
use state_sync::crdt_map::CrdtMap;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tokio::sync::RwLock;
use anyhow::Result;

/// A task that can be assigned to an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub description: String,
    /// Resource requirements (e.g., "cpu", "memory", "gpu").
    pub required_resources: Vec<String>,
    /// Capability requirements (e.g., "camera", "gripper", "navigation").
    pub required_capabilities: Vec<Capability>,
    pub estimated_duration_secs: u64,
    /// Deadline as Unix timestamp (seconds). If None, no deadline.
    #[serde(default)]
    pub deadline: Option<u64>,
    /// Priority from 0 (lowest) to 255 (highest).
    #[serde(default)]
    pub priority: u8,
    /// IDs of tasks that must be completed before this task can start.
    #[serde(default)]
    pub dependencies: Vec<String>,
}

/// Assignment of a task to an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assignment {
    pub task_id: String,
    pub agent_id: AgentId,
    pub start_time: Option<u64>,
    pub status: AssignmentStatus,
    /// Deadline copied from the task (optional).
    #[serde(default)]
    pub deadline: Option<u64>,
    /// Priority copied from the task.
    #[serde(default)]
    pub priority: u8,
    /// Whether dependencies are satisfied.
    #[serde(default)]
    pub dependencies_satisfied: bool,
    /// Estimated finish time (start_time + estimated_duration_secs).
    #[serde(default)]
    pub estimated_finish_time: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AssignmentStatus {
    Pending,
    Assigned,
    InProgress,
    Completed,
    Failed,
}

impl Task {
    /// Create a new assignment for this task, assigned to the given agent.
    pub fn create_assignment(&self, agent_id: AgentId) -> Assignment {
        Assignment {
            task_id: self.id.clone(),
            agent_id,
            start_time: None,
            status: AssignmentStatus::Pending,
            deadline: self.deadline,
            priority: self.priority,
            dependencies_satisfied: self.dependencies.is_empty(),
            estimated_finish_time: None,
        }
    }
}

/// Configuration for the distributed planner.
#[derive(Debug, Clone)]
pub struct DistributedPlannerConfig {
    pub local_agent_id: AgentId,
    pub participant_agents: HashSet<AgentId>,
    pub consensus_config: BoundedConsensusConfig,
    pub transport_config: MeshTransportConfig,
}

/// The distributed planner.
pub struct DistributedPlanner<T: BoundedConsensus> {
    config: DistributedPlannerConfig,
    consensus: T,
    transport: MeshTransport,
    tasks: RwLock<HashMap<String, Task>>,
    assignments: RwLock<HashMap<String, Assignment>>,
    crdt_map: CrdtMap,
    task_sync: sync::TaskSync,
}

impl DistributedPlanner<TwoPhaseBoundedConsensus<Assignment>> {
    /// Create a new distributed planner with default two‑phase consensus.
    pub async fn new(config: DistributedPlannerConfig) -> Result<Self> {
        let consensus = TwoPhaseBoundedConsensus::new(config.consensus_config.clone());
        let transport = MeshTransport::new(config.transport_config.clone()).await?;
        let crdt_map = CrdtMap::new();
        let task_sync = sync::TaskSync::new(crdt_map, config.local_agent_id);
        Ok(Self {
            config,
            consensus,
            transport,
            tasks: RwLock::new(HashMap::new()),
            assignments: RwLock::new(HashMap::new()),
            crdt_map,
            task_sync,
        })
    }

    /// Start the planner (start transport and consensus).
    pub async fn start(&mut self) -> Result<()> {
        self.transport.start().await?;
        // No explicit start for consensus yet.
        Ok(())
    }

    /// Stop the planner.
    pub async fn stop(&mut self) -> Result<()> {
        self.transport.stop().await?;
        Ok(())
    }

    /// Add a new task to the pool (local only).
    pub async fn add_task(&self, task: Task) -> Result<()> {
        let mut tasks = self.tasks.write().await;
        tasks.insert(task.id.clone(), task);
        Ok(())
    }

    /// Publish a task to the shared CRDT map.
    pub async fn publish_task(&mut self, task: &Task) {
        self.task_sync.publish_task(task);
    }

    /// Propose a task assignment for consensus.
    pub async fn propose_assignment(&mut self, assignment: Assignment) -> Result<()> {
        let proposal = Proposal {
            id: rand::random(),
            value: assignment.clone(),
            proposer: self.config.local_agent_id,
        };
        let mut rx = self.consensus.propose(proposal).await?;
        // Wait for outcome (simplified)
        tokio::spawn(async move {
            while let Some(outcome) = rx.recv().await {
                match outcome {
                    bounded_consensus::ConsensusOutcome::Decided(ass) => {
                        tracing::info!("Assignment decided: {:?}", ass);
                        // In a real implementation, store the assignment.
                    }
                    bounded_consensus::ConsensusOutcome::Timeout => {
                        tracing::warn!("Consensus timeout for assignment");
                    }
                    bounded_consensus::ConsensusOutcome::Aborted => {
                        tracing::warn!("Consensus aborted for assignment");
                    }
                }
            }
        });
        Ok(())
    }

    /// Get all tasks known to the local planner.
    pub async fn get_tasks(&self) -> Vec<Task> {
        let tasks = self.tasks.read().await;
        tasks.values().cloned().collect()
    }

    /// Get all assignments (local view).
    pub async fn get_assignments(&self) -> Vec<Assignment> {
        let assignments = self.assignments.read().await;
        assignments.values().cloned().collect()
    }

    /// Update assignment status (local).
    pub async fn update_assignment_status(&self, task_id: &str, status: AssignmentStatus) -> Result<()> {
        let mut assignments = self.assignments.write().await;
        if let Some(assignment) = assignments.get_mut(task_id) {
            assignment.status = status;
        }
        Ok(())
    }

    /// Run a planning algorithm to produce new assignments.
    pub async fn run_planning_algorithm<A: algorithms::PlanningAlgorithm>(
        &self,
        algorithm: &A,
    ) -> Result<Vec<Assignment>> {
        let tasks = self.get_tasks().await;
        let agents = self.config.participant_agents.clone();
        let current_assignments = self.get_assignments().await;
        algorithm.plan(tasks, agents, current_assignments).await
    }

    /// Synchronize tasks and assignments from the CRDT map.
    pub async fn sync_from_crdt(&mut self) -> Result<()> {
        let tasks = self.task_sync.get_all_tasks();
        let assignments = self.task_sync.get_all_assignments();

        // Update local tasks
        let mut local_tasks = self.tasks.write().await;
        for task in tasks {
            local_tasks.insert(task.id.clone(), task);
        }

        // Update local assignments
        let mut local_assignments = self.assignments.write().await;
        for assignment in assignments {
            local_assignments.insert(assignment.task_id.clone(), assignment);
        }

        Ok(())
    }

    /// Generate a delta of changes since the given vector clock.
    pub fn delta_since(&self, since: &VectorClock) -> Option<state_sync::delta::Delta> {
        self.task_sync.delta_since(since)
    }

    /// Apply a delta from another agent.
    pub fn apply_delta(&mut self, delta: state_sync::delta::Delta) {
        self.task_sync.apply_delta(delta);
    }
}
// Re‑export planning algorithms for convenience.
pub use algorithms::{
    PlanningAlgorithm,
    RoundRobinPlanner,
    AuctionPlanner,
    ResourceAwarePlanner,
    CapabilityAwarePlanner,
    DeadlineAwarePlanner,
    DependencyAwarePlanner,
    ConsensusPlanner,
};