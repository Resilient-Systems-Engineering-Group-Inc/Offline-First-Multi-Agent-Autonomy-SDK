//! Planning algorithms for distributed task assignment.

use crate::{Task, Assignment, AgentId};
use common::types::Capability;
use std::collections::{HashMap, HashSet};
use anyhow::Result;

/// Trait for a planning algorithm that decides task assignments.
#[async_trait::async_trait]
pub trait PlanningAlgorithm: Send + Sync {
    /// Given a set of tasks and a set of available agents, produce assignments.
    async fn plan(
        &self,
        tasks: Vec<Task>,
        agents: HashSet<AgentId>,
        current_assignments: Vec<Assignment>,
    ) -> Result<Vec<Assignment>>;

    /// Name of the algorithm for logging and debugging.
    fn name(&self) -> &'static str;
}

/// Simple round‑robin assignment.
pub struct RoundRobinPlanner;

#[async_trait::async_trait]
impl PlanningAlgorithm for RoundRobinPlanner {
    async fn plan(
        &self,
        tasks: Vec<Task>,
        agents: HashSet<AgentId>,
        _current_assignments: Vec<Assignment>,
    ) -> Result<Vec<Assignment>> {
        let agents: Vec<AgentId> = agents.into_iter().collect();
        if agents.is_empty() {
            return Ok(Vec::new());
        }

        let mut assignments = Vec::new();
        for (i, task) in tasks.into_iter().enumerate() {
            let agent_idx = i % agents.len();
            let assignment = Assignment {
                task_id: task.id,
                agent_id: agents[agent_idx],
                start_time: None,
                status: crate::AssignmentStatus::Pending,
            };
            assignments.push(assignment);
        }
        Ok(assignments)
    }

    fn name(&self) -> &'static str {
        "round_robin"
    }
}

/// Auction‑based planner: each agent bids on tasks, lowest cost wins.
pub struct AuctionPlanner {
    /// Function to compute cost of a task for an agent.
    /// In a real implementation this would consider resource availability, distance, etc.
    cost_fn: Box<dyn Fn(&Task, AgentId) -> u64 + Send + Sync>,
}

impl AuctionPlanner {
    pub fn new<F>(cost_fn: F) -> Self
    where
        F: Fn(&Task, AgentId) -> u64 + Send + Sync + 'static,
    {
        Self {
            cost_fn: Box::new(cost_fn),
        }
    }
}

#[async_trait::async_trait]
impl PlanningAlgorithm for AuctionPlanner {
    async fn plan(
        &self,
        tasks: Vec<Task>,
        agents: HashSet<AgentId>,
        _current_assignments: Vec<Assignment>,
    ) -> Result<Vec<Assignment>> {
        let mut assignments = Vec::new();
        for task in tasks {
            let mut best_agent = None;
            let mut best_cost = u64::MAX;
            for &agent in &agents {
                let cost = (self.cost_fn)(&task, agent);
                if cost < best_cost {
                    best_cost = cost;
                    best_agent = Some(agent);
                }
            }
            if let Some(agent) = best_agent {
                assignments.push(Assignment {
                    task_id: task.id,
                    agent_id: agent,
                    start_time: None,
                    status: crate::AssignmentStatus::Pending,
                });
            }
        }
        Ok(assignments)
    }

    fn name(&self) -> &'static str {
        "auction"
    }
}

/// Resource‑aware planner that only assigns tasks if the agent has the required resources and capabilities.
pub struct ResourceAwarePlanner {
    /// Map from agent ID to its available resources.
    agent_resources: HashMap<AgentId, HashSet<String>>,
    /// Map from agent ID to its available capabilities.
    agent_capabilities: HashMap<AgentId, HashSet<Capability>>,
}

impl ResourceAwarePlanner {
    pub fn new(
        agent_resources: HashMap<AgentId, HashSet<String>>,
        agent_capabilities: HashMap<AgentId, HashSet<Capability>>,
    ) -> Self {
        Self {
            agent_resources,
            agent_capabilities,
        }
    }
}

#[async_trait::async_trait]
impl PlanningAlgorithm for ResourceAwarePlanner {
    async fn plan(
        &self,
        tasks: Vec<Task>,
        agents: HashSet<AgentId>,
        _current_assignments: Vec<Assignment>,
    ) -> Result<Vec<Assignment>> {
        let mut assignments = Vec::new();
        for task in tasks {
            let mut best_agent = None;
            // Find an agent that has all required resources AND capabilities.
            for &agent in &agents {
                let has_resources = self.agent_resources.get(&agent)
                    .map(|resources| task.required_resources.iter().all(|r| resources.contains(r)))
                    .unwrap_or(false);
                let has_capabilities = self.agent_capabilities.get(&agent)
                    .map(|caps| task.required_capabilities.iter().all(|c| caps.contains(c)))
                    .unwrap_or(false);
                if has_resources && has_capabilities {
                    best_agent = Some(agent);
                    break;
                }
            }
            if let Some(agent) = best_agent {
                assignments.push(Assignment {
                    task_id: task.id,
                    agent_id: agent,
                    start_time: None,
                    status: crate::AssignmentStatus::Pending,
                });
            }
        }
        Ok(assignments)
    }

    fn name(&self) -> &'static str {
        "resource_aware"
    }
}

/// Capability‑aware planner that only checks capabilities (ignores resources).
pub struct CapabilityAwarePlanner {
    /// Map from agent ID to its available capabilities.
    agent_capabilities: HashMap<AgentId, HashSet<Capability>>,
}

impl CapabilityAwarePlanner {
    pub fn new(agent_capabilities: HashMap<AgentId, HashSet<Capability>>) -> Self {
        Self { agent_capabilities }
    }
}

#[async_trait::async_trait]
impl PlanningAlgorithm for CapabilityAwarePlanner {
    async fn plan(
        &self,
        tasks: Vec<Task>,
        agents: HashSet<AgentId>,
        _current_assignments: Vec<Assignment>,
    ) -> Result<Vec<Assignment>> {
        let mut assignments = Vec::new();
        for task in tasks {
            let mut best_agent = None;
            for &agent in &agents {
                if let Some(caps) = self.agent_capabilities.get(&agent) {
                    if task.required_capabilities.iter().all(|c| caps.contains(c)) {
                        best_agent = Some(agent);
                        break;
                    }
                }
            }
            if let Some(agent) = best_agent {
                assignments.push(Assignment {
                    task_id: task.id,
                    agent_id: agent,
                    start_time: None,
                    status: crate::AssignmentStatus::Pending,
                });
            }
        }
        Ok(assignments)
    }

    fn name(&self) -> &'static str {
        "capability_aware"
    }
}

/// Consensus‑based planner that uses bounded consensus to agree on assignments.
/// This is a wrapper around the existing consensus mechanism.
pub struct ConsensusPlanner;

#[async_trait::async_trait]
impl PlanningAlgorithm for ConsensusPlanner {
    async fn plan(
        &self,
        _tasks: Vec<Task>,
        _agents: HashSet<AgentId>,
        _current_assignments: Vec<Assignment>,
    ) -> Result<Vec<Assignment>> {
        // This planner does not produce assignments directly; it relies on the consensus
        // process that runs across agents. Therefore we return empty list.
        // In a real implementation, this would interact with the consensus module.
        Ok(Vec::new())
    }

    fn name(&self) -> &'static str {
        "consensus"
    }
}