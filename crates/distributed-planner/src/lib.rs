//! Distributed task planning for offline‑first multi‑agent systems.
//!
//! This module provides a planner that coordinates tasks across multiple agents
//! using consensus and shared state.

use common::types::AgentId;
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
    pub required_resources: Vec<String>,
    pub estimated_duration_secs: u64,
}

/// Assignment of a task to an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assignment {
    pub task_id: String,
    pub agent_id: AgentId,
    pub start_time: Option<u64>,
    pub status: AssignmentStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AssignmentStatus {
    Pending,
    Assigned,
    InProgress,
    Completed,
    Failed,
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
}

impl DistributedPlanner<TwoPhaseBoundedConsensus<Assignment>> {
    /// Create a new distributed planner with default two‑phase consensus.
    pub async fn new(config: DistributedPlannerConfig) -> Result<Self> {
        let consensus = TwoPhaseBoundedConsensus::new(config.consensus_config.clone());
        let transport = MeshTransport::new(config.transport_config.clone()).await?;
        Ok(Self {
            config,
            consensus,
            transport,
            tasks: RwLock::new(HashMap::new()),
            assignments: RwLock::new(HashMap::new()),
            crdt_map: CrdtMap::new(),
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
}