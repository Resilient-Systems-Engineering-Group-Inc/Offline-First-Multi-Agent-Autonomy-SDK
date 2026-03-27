//! State snapshot and comparison utilities.

use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use common::types::AgentId;
use anyhow::Result;

/// A snapshot of an agent's local state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSnapshot {
    pub agent_id: AgentId,
    pub timestamp: u64,
    /// Key‑value pairs representing the state (e.g., CRDT map entries).
    pub state: HashMap<String, serde_json::Value>,
    /// Metrics at the time of snapshot.
    pub metrics: HashMap<String, f64>,
}

impl AgentSnapshot {
    pub fn new(agent_id: AgentId, state: HashMap<String, serde_json::Value>) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Self {
            agent_id,
            timestamp,
            state,
            metrics: HashMap::new(),
        }
    }

    /// Add a metric to the snapshot.
    pub fn with_metric(mut self, key: &str, value: f64) -> Self {
        self.metrics.insert(key.to_string(), value);
        self
    }

    /// Serialize to JSON.
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Compare two snapshots and return differences.
    pub fn diff(&self, other: &Self) -> SnapshotDiff {
        let mut added = HashMap::new();
        let mut removed = HashMap::new();
        let mut changed = HashMap::new();

        for (key, val) in &self.state {
            match other.state.get(key) {
                None => removed.insert(key.clone(), val.clone()),
                Some(other_val) if other_val != val => changed.insert(key.clone(), (val.clone(), other_val.clone())),
                _ => continue,
            };
        }
        for (key, val) in &other.state {
            if !self.state.contains_key(key) {
                added.insert(key.clone(), val.clone());
            }
        }

        SnapshotDiff { added, removed, changed }
    }
}

/// Difference between two snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotDiff {
    pub added: HashMap<String, serde_json::Value>,
    pub removed: HashMap<String, serde_json::Value>,
    pub changed: HashMap<String, (serde_json::Value, serde_json::Value)>,
}

impl SnapshotDiff {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.changed.is_empty()
    }
}

/// Collect snapshots from multiple agents (simulated).
pub async fn collect_snapshots(
    agents: &[AgentId],
    state_fetcher: impl Fn(AgentId) -> HashMap<String, serde_json::Value>,
) -> Vec<AgentSnapshot> {
    let mut snapshots = Vec::new();
    for &agent in agents {
        let state = state_fetcher(agent);
        snapshots.push(AgentSnapshot::new(agent, state));
    }
    snapshots
}

/// Write snapshots to a file.
pub fn write_snapshots_to_file(snapshots: &[AgentSnapshot], path: &str) -> Result<()> {
    let json = serde_json::to_string_pretty(snapshots)?;
    std::fs::write(path, json)?;
    Ok(())
}