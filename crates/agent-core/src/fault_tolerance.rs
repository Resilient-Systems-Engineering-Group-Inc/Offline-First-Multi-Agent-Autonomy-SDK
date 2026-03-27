//! Fault tolerance and self‑healing mechanisms.

use crate::agent::Agent;
use common::types::AgentId;
use mesh_transport::TransportEvent;
use std::collections::{HashMap, HashSet};
use tokio::sync::mpsc;
use tracing::{info, warn};

/// Tracks which agents are considered alive.
#[derive(Debug, Default)]
pub struct FaultDetector {
    alive_agents: HashSet<AgentId>,
    // Map from agent ID to timestamp of last heartbeat (not implemented yet)
    last_seen: HashMap<AgentId, u64>,
}

impl FaultDetector {
    pub fn new() -> Self {
        Self {
            alive_agents: HashSet::new(),
            last_seen: HashMap::new(),
        }
    }

    /// Update based on a transport event.
    pub fn on_event(&mut self, event: &TransportEvent) {
        match event {
            TransportEvent::PeerDiscovered(agent) | TransportEvent::ConnectionEstablished(agent) => {
                self.alive_agents.insert(*agent);
                info!(agent = ?agent, "Agent marked as alive");
            }
            TransportEvent::PeerLost(agent) | TransportEvent::ConnectionClosed(agent) => {
                self.alive_agents.remove(agent);
                warn!(agent = ?agent, "Agent marked as dead");
            }
            _ => {}
        }
    }

    /// Check if an agent is considered alive.
    pub fn is_alive(&self, agent: &AgentId) -> bool {
        self.alive_agents.contains(agent)
    }

    /// Get all alive agents.
    pub fn alive_agents(&self) -> &HashSet<AgentId> {
        &self.alive_agents
    }
}

/// Reallocates tasks from failed agents to alive ones.
pub struct TaskReallocator {
    // This could be a reference to a distributed planner, but for simplicity we just log.
}

impl TaskReallocator {
    pub fn new() -> Self {
        Self {}
    }

    /// Called when an agent is detected as dead.
    /// In a real implementation, this would query the task assignments and reassign them.
    pub async fn on_agent_failure(&self, failed_agent: AgentId, alive_agents: &HashSet<AgentId>) {
        warn!(
            ?failed_agent,
            ?alive_agents,
            "Agent failure detected, should reallocate tasks"
        );
        // TODO: integrate with distributed planner to reassign tasks.
    }
}

/// A combined fault‑tolerance manager that runs in the background.
pub struct FaultToleranceManager {
    detector: FaultDetector,
    reallocator: TaskReallocator,
    event_rx: mpsc::UnboundedReceiver<TransportEvent>,
}

impl FaultToleranceManager {
    pub fn new(event_rx: mpsc::UnboundedReceiver<TransportEvent>) -> Self {
        Self {
            detector: FaultDetector::new(),
            reallocator: TaskReallocator::new(),
            event_rx,
        }
    }

    /// Run the manager loop, processing events and triggering recovery.
    pub async fn run(mut self) {
        info!("Fault tolerance manager started");
        while let Some(event) = self.event_rx.recv().await {
            self.detector.on_event(&event);

            // If a peer is lost, trigger reallocation.
            if let TransportEvent::PeerLost(failed_agent) = event {
                let alive = self.detector.alive_agents().clone();
                self.reallocator.on_agent_failure(failed_agent, &alive).await;
            }
        }
    }
}