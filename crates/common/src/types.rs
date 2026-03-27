//! Common types used across the SDK.

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

/// Unique identifier for an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub u64);

/// Peer information including its network address.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub agent_id: AgentId,
    pub addresses: Vec<SocketAddr>,
    pub metadata: std::collections::HashMap<String, String>,
}

/// A message that can be sent over the mesh.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshMessage {
    pub source: AgentId,
    pub destination: Option<AgentId>, // None for broadcast
    pub payload: Vec<u8>,
    pub timestamp: u64, // logical timestamp
}

/// Vector clock for causal ordering.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VectorClock {
    pub entries: std::collections::HashMap<AgentId, u64>,
}

impl VectorClock {
    /// Increment the clock for a given agent.
    pub fn increment(&mut self, agent: AgentId) {
        let entry = self.entries.entry(agent).or_insert(0);
        *entry += 1;
    }

    /// Compare two vector clocks (happens‑before).
    pub fn precedes(&self, other: &Self) -> bool {
        // Implementation of partial order
        for (agent, &count) in &self.entries {
            if let Some(&other_count) = other.entries.get(agent) {
                if count > other_count {
                    return false;
                }
            } else if count > 0 {
                return false;
            }
        }
        true
    }
}

/// A capability that an agent can possess (e.g., "camera", "gripper", "compute_heavy").
pub type Capability = String;

/// Static capabilities and resource limits of an agent.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentCapabilities {
    /// List of capabilities (strings) that this agent provides.
    pub capabilities: Vec<Capability>,
    /// Maximum CPU cores (if applicable).
    pub max_cpu_cores: Option<u32>,
    /// Maximum memory in bytes.
    pub max_memory_bytes: Option<u64>,
    /// Maximum disk space in bytes.
    pub max_disk_bytes: Option<u64>,
    /// Whether the agent has a battery.
    pub has_battery: bool,
    /// Other custom attributes.
    pub custom_attrs: std::collections::HashMap<String, String>,
}