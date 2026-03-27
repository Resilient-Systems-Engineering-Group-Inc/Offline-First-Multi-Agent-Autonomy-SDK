//! Swarm simulator for testing offline‑first multi‑agent systems.
//!
//! This crate provides a configurable simulation environment that emulates
//! a network of agents, with configurable network delays, failures, and
//! resource constraints. It can be used to test distributed algorithms
//! (consensus, planning, state sync) in a controlled, reproducible manner.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use agent_core::Agent;
use common::types::{AgentId, Capability, VectorClock};
use distributed_planner::{Task, Assignment, PlanningAlgorithm};
use mesh_transport::{MeshTransport, MeshTransportConfig};
use bounded_consensus::{BoundedConsensus, TwoPhaseBoundedConsensus, BoundedConsensusConfig};
use state_sync::crdt_map::CrdtMap;
use tokio::sync::{RwLock, mpsc};
use tokio::time::{sleep, Instant};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use anyhow::Result;
use tracing::{info, warn, error};

/// Configuration for the swarm simulator.
#[derive(Debug, Clone)]
pub struct SwarmSimulatorConfig {
    /// Number of agents in the swarm.
    pub num_agents: usize,
    /// Seed for deterministic randomness.
    pub rng_seed: u64,
    /// Network latency range (min, max) in milliseconds.
    pub latency_range_ms: (u64, u64),
    /// Probability of message loss (0.0 to 1.0).
    pub message_loss_prob: f64,
    /// Probability of agent failure during simulation (0.0 to 1.0).
    pub agent_failure_prob: f64,
    /// Duration of the simulation in seconds.
    pub simulation_duration_secs: u64,
    /// Whether to enable logging.
    pub enable_logging: bool,
}

impl Default for SwarmSimulatorConfig {
    fn default() -> Self {
        Self {
            num_agents: 5,
            rng_seed: 12345,
            latency_range_ms: (10, 100),
            message_loss_prob: 0.05,
            agent_failure_prob: 0.01,
            simulation_duration_secs: 60,
            enable_logging: true,
        }
    }
}

/// Represents a simulated agent node.
pub struct AgentNode {
    pub id: AgentId,
    pub agent: Agent,
    pub transport: MeshTransport,
    pub consensus: TwoPhaseBoundedConsensus<Assignment>,
    pub crdt_map: CrdtMap,
    pub capabilities: HashSet<Capability>,
    pub resources: HashSet<String>,
    pub is_failed: bool,
}

/// Network model that introduces delays and losses.
pub struct NetworkModel {
    latency_range_ms: (u64, u64),
    loss_prob: f64,
    rng: StdRng,
}

impl NetworkModel {
    pub fn new(latency_range_ms: (u64, u64), loss_prob: f64, seed: u64) -> Self {
        Self {
            latency_range_ms,
            loss_prob,
            rng: StdRng::seed_from_u64(seed),
        }
    }

    /// Simulate sending a message: returns true if the message is delivered,
    /// and the delay in milliseconds.
    pub fn simulate_send(&mut self) -> (bool, u64) {
        let loss = self.rng.gen_bool(self.loss_prob);
        if loss {
            return (false, 0);
        }
        let delay = self.rng.gen_range(self.latency_range_ms.0..=self.latency_range_ms.1);
        (true, delay)
    }
}

/// Failure model that randomly marks agents as failed.
pub struct FailureModel {
    failure_prob: f64,
    rng: StdRng,
}

impl FailureModel {
    pub fn new(failure_prob: f64, seed: u64) -> Self {
        Self {
            failure_prob,
            rng: StdRng::seed_from_u64(seed),
        }
    }

    /// Decide whether an agent fails at this step.
    pub fn should_fail(&mut self) -> bool {
        self.rng.gen_bool(self.failure_prob)
    }
}

/// The main swarm simulator.
pub struct SwarmSimulator {
    config: SwarmSimulatorConfig,
    agents: HashMap<AgentId, Arc<RwLock<AgentNode>>>,
    network_model: RwLock<NetworkModel>,
    failure_model: RwLock<FailureModel>,
    tasks: Vec<Task>,
    assignments: Vec<Assignment>,
    event_tx: mpsc::Sender<SimulationEvent>,
    event_rx: mpsc::Receiver<SimulationEvent>,
}

/// Events that can occur during simulation.
#[derive(Debug, Clone)]
pub enum SimulationEvent {
    AgentFailed(AgentId),
    MessageLost(AgentId, AgentId),
    TaskAssigned(Task, AgentId),
    TaskCompleted(Task, AgentId),
    ConsensusDecided(Assignment),
}

impl SwarmSimulator {
    /// Create a new swarm simulator with the given configuration.
    pub async fn new(config: SwarmSimulatorConfig) -> Result<Self> {
        let (event_tx, event_rx) = mpsc::channel(1000);

        let mut agents = HashMap::new();
        let mut rng = StdRng::seed_from_u64(config.rng_seed);

        for i in 0..config.num_agents {
            let agent_id = AgentId(i as u64);
            // Create a simple mesh transport config (in‑memory backend for simulation)
            let transport_config = MeshTransportConfig {
                local_agent_id: agent_id,
                backend: mesh_transport::BackendKind::InMemory,
                discovery_interval_secs: 5,
                heartbeat_interval_secs: 2,
                ..Default::default()
            };
            let transport = MeshTransport::new(transport_config).await?;
            let consensus_config = BoundedConsensusConfig {
                participant_ids: (0..config.num_agents).map(|i| AgentId(i as u64)).collect(),
                local_id: agent_id,
                timeout_ms: 5000,
                max_rounds: 10,
            };
            let consensus = TwoPhaseBoundedConsensus::new(consensus_config);
            let crdt_map = CrdtMap::new();
            let capabilities = generate_random_capabilities(&mut rng);
            let resources = generate_random_resources(&mut rng);

            let agent = Agent::new(agent_id, capabilities.clone(), resources.clone());

            let node = AgentNode {
                id: agent_id,
                agent,
                transport,
                consensus,
                crdt_map,
                capabilities,
                resources,
                is_failed: false,
            };

            agents.insert(agent_id, Arc::new(RwLock::new(node)));
        }

        let network_model = NetworkModel::new(
            config.latency_range_ms,
            config.message_loss_prob,
            config.rng_seed,
        );
        let failure_model = FailureModel::new(config.agent_failure_prob, config.rng_seed);

        Ok(Self {
            config,
            agents,
            network_model: RwLock::new(network_model),
            failure_model: RwLock::new(failure_model),
            tasks: Vec::new(),
            assignments: Vec::new(),
            event_tx,
            event_rx,
        })
    }

    /// Add a task to the simulation.
    pub fn add_task(&mut self, task: Task) {
        self.tasks.push(task);
    }

    /// Run the simulation for the configured duration.
    pub async fn run(&mut self) -> Result<()> {
        info!("Starting swarm simulation with {} agents", self.config.num_agents);
        let start = Instant::now();
        let duration = Duration::from_secs(self.config.simulation_duration_secs);

        // Start all agent transports
        for (_, agent_lock) in &self.agents {
            let mut agent = agent_lock.write().await;
            agent.transport.start().await?;
        }

        // Simulation loop
        while start.elapsed() < duration {
            // Step 1: apply failures
            self.apply_failures().await;

            // Step 2: simulate network for each agent
            self.simulate_network().await;

            // Step 3: run planning algorithms on a random agent
            self.run_planning_step().await?;

            // Step 4: process events
            self.process_events().await;

            // Wait a short time to avoid busy loop
            sleep(Duration::from_millis(100)).await;
        }

        info!("Simulation finished after {:?}", start.elapsed());
        self.collect_stats().await;
        Ok(())
    }

    async fn apply_failures(&self) {
        let mut failure_model = self.failure_model.write().await;
        for (id, agent_lock) in &self.agents {
            if failure_model.should_fail() {
                let mut agent = agent_lock.write().await;
                if !agent.is_failed {
                    agent.is_failed = true;
                    let _ = self.event_tx.send(SimulationEvent::AgentFailed(*id)).await;
                    warn!("Agent {} failed", id.0);
                }
            }
        }
    }

    async fn simulate_network(&self) {
        // In a real implementation, we would intercept messages between agents
        // and apply delays/losses. For simplicity, we just log.
        let mut network_model = self.network_model.write().await;
        for (src_id, src_lock) in &self.agents {
            for (dst_id, _) in &self.agents {
                if src_id == dst_id {
                    continue;
                }
                let (delivered, delay) = network_model.simulate_send();
                if !delivered {
                    let _ = self.event_tx
                        .send(SimulationEvent::MessageLost(*src_id, *dst_id))
                        .await;
                } else if delay > 0 {
                    // In a real simulation we would delay the message delivery
                    // using a queue. Here we just note it.
                }
            }
        }
    }

    async fn run_planning_step(&self) -> Result<()> {
        // Pick a random non‑failed agent to run planning
        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();
        let agents: Vec<_> = self.agents.values().collect();
        if let Some(agent_lock) = agents.choose(&mut rng) {
            let agent = agent_lock.read().await;
            if agent.is_failed {
                return Ok(());
            }
            // Use a simple round‑robin planner for demonstration
            use distributed_planner::RoundRobinPlanner;
            let planner = RoundRobinPlanner;
            let tasks = self.tasks.clone();
            let agent_ids: HashSet<AgentId> = self.agents
                .keys()
                .filter(|&id| {
                    let guard = futures::executor::block_on(async {
                        self.agents.get(id).unwrap().read().await
                    });
                    !guard.is_failed
                })
                .cloned()
                .collect();
            let assignments = planner.plan(tasks, agent_ids, self.assignments.clone()).await?;
            for assignment in assignments {
                let _ = self.event_tx
                    .send(SimulationEvent::TaskAssigned(
                        self.tasks.iter().find(|t| t.id == assignment.task_id).unwrap().clone(),
                        assignment.agent_id,
                    ))
                    .await;
                info!("Task {} assigned to agent {}", assignment.task_id, assignment.agent_id.0);
            }
        }
        Ok(())
    }

    async fn process_events(&mut self) {
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                SimulationEvent::AgentFailed(id) => {
                    // Mark agent as failed in our local view
                    if let Some(agent_lock) = self.agents.get(&id) {
                        let mut agent = agent_lock.write().await;
                        agent.is_failed = true;
                    }
                }
                SimulationEvent::MessageLost(src, dst) => {
                    if self.config.enable_logging {
                        warn!("Message lost from {} to {}", src.0, dst.0);
                    }
                }
                SimulationEvent::TaskAssigned(task, agent_id) => {
                    // Record assignment
                    let assignment = task.create_assignment(agent_id);
                    self.assignments.push(assignment);
                }
                SimulationEvent::TaskCompleted(task, agent_id) => {
                    info!("Task {} completed by agent {}", task.id, agent_id.0);
                }
                SimulationEvent::ConsensusDecided(assignment) => {
                    info!("Consensus decided assignment: {:?}", assignment);
                }
            }
        }
    }

    async fn collect_stats(&self) {
        let num_failed = self.agents.values()
            .filter(|a| a.read().await.is_failed)
            .count();
        info!("Simulation statistics:");
        info!("  Total agents: {}", self.config.num_agents);
        info!("  Failed agents: {}", num_failed);
        info!("  Tasks submitted: {}", self.tasks.len());
        info!("  Assignments made: {}", self.assignments.len());
    }
}

/// Generate a random set of capabilities for an agent.
fn generate_random_capabilities(rng: &mut StdRng) -> HashSet<Capability> {
    let all_capabilities = vec![
        "navigation".to_string(),
        "manipulation".to_string(),
        "sensing".to_string(),
        "communication".to_string(),
        "computation".to_string(),
    ];
    let count = rng.gen_range(1..=3);
    all_capabilities.into_iter().take(count).collect()
}

/// Generate a random set of resources.
fn generate_random_resources(rng: &mut StdRng) -> HashSet<String> {
    let all_resources = vec!["cpu", "memory", "gpu", "storage", "network"];
    let count = rng.gen_range(1..=3);
    all_resources.into_iter().take(count).map(String::from).collect()
}

/// Utility to create a simple demo simulation.
pub async fn run_demo() -> Result<()> {
    let config = SwarmSimulatorConfig {
        num_agents: 10,
        simulation_duration_secs: 30,
        enable_logging: true,
        ..Default::default()
    };
    let mut simulator = SwarmSimulator::new(config).await?;

    // Add some sample tasks
    for i in 0..5 {
        simulator.add_task(Task {
            id: format!("task_{}", i),
            description: format!("Sample task {}", i),
            required_resources: vec!["cpu".to_string()],
            required_capabilities: vec!["computation".to_string()],
            estimated_duration_secs: 10,
            deadline: None,
            priority: 1,
            dependencies: Vec::new(),
        });
    }

    simulator.run().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simulator_creation() {
        let config = SwarmSimulatorConfig {
            num_agents: 3,
            simulation_duration_secs: 1,
            enable_logging: false,
            ..Default::default()
        };
        let simulator = SwarmSimulator::new(config).await;
        assert!(simulator.is_ok());
    }

    #[tokio::test]
    async fn test_demo() {
        // Quick demo that doesn't run long
        let config = SwarmSimulatorConfig {
            num_agents: 2,
            simulation_duration_secs: 2,
            enable_logging: false,
            ..Default::default()
        };
        let mut simulator = SwarmSimulator::new(config).await.unwrap();
        let result = simulator.run().await;
        assert!(result.is_ok());
    }
}