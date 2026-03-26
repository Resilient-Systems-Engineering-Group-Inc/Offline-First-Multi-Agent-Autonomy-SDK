//! State synchronization engine.

use crate::crdt_map::CrdtMap;
use crate::delta::Delta;
use common::types::{AgentId, VectorClock};
use common::error::Result;
use async_trait::async_trait;
use std::collections::HashMap;

/// Trait for state synchronization.
#[async_trait]
pub trait StateSync: Send + Sync {
    /// Get the current CRDT map.
    fn map(&self) -> &CrdtMap;

    /// Get a mutable reference to the map.
    fn map_mut(&mut self) -> &mut CrdtMap;

    /// Apply a delta from a remote peer.
    async fn apply_delta(&mut self, delta: Delta) -> Result<()>;

    /// Generate a delta for a remote peer based on its known vector clock.
    async fn delta_for_peer(&self, peer: AgentId, known_vclock: &VectorClock) -> Option<Delta>;

    /// Broadcast local changes to all peers (via transport).
    async fn broadcast_changes(&mut self) -> Result<()>;
}

/// Default implementation of StateSync.
pub struct DefaultStateSync {
    map: CrdtMap,
    local_agent: AgentId,
    // Per‑peer vector clocks (the latest state we know they have).
    peer_clocks: HashMap<AgentId, VectorClock>,
}

impl DefaultStateSync {
    pub fn new(local_agent: AgentId) -> Self {
        Self {
            map: CrdtMap::new(),
            local_agent,
            peer_clocks: HashMap::new(),
        }
    }

    /// Update the vector clock for a peer (after they acknowledge a delta).
    pub fn update_peer_clock(&mut self, peer: AgentId, vclock: &VectorClock) {
        let entry = self.peer_clocks.entry(peer).or_insert_with(VectorClock::default);
        for (agent, &count) in &vclock.entries {
            let e = entry.entries.entry(*agent).or_insert(0);
            *e = (*e).max(count);
        }
    }
}

#[async_trait]
impl StateSync for DefaultStateSync {
    fn map(&self) -> &CrdtMap {
        &self.map
    }

    fn map_mut(&mut self) -> &mut CrdtMap {
        &mut self.map
    }

    async fn apply_delta(&mut self, delta: Delta) -> Result<()> {
        self.map.apply_delta(delta.clone());
        // Update peer clock for the sender
        self.update_peer_clock(delta.author, &delta.vclock);
        Ok(())
    }

    async fn delta_for_peer(&self, peer: AgentId, known_vclock: &VectorClock) -> Option<Delta> {
        // Generate delta based on what the peer hasn't seen yet.
        let delta = self.map.delta_since(known_vclock)?;
        // Set the author to local agent (the sender)
        Some(Delta::new(self.local_agent, delta.ops, delta.vclock))
    }

    async fn broadcast_changes(&mut self) -> Result<()> {
        // For each peer, generate a delta and send via transport.
        // This is a placeholder; in reality we would have a transport reference.
        // For now, just log.
        tracing::info!("Broadcasting changes (not implemented)");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_default_state_sync_new() {
        let sync = DefaultStateSync::new(AgentId(42));
        assert_eq!(sync.local_agent, AgentId(42));
        assert!(sync.peer_clocks.is_empty());
    }

    #[tokio::test]
    async fn test_apply_delta() {
        let mut sync = DefaultStateSync::new(AgentId(1));
        let delta = Delta::new(
            AgentId(2),
            vec![
                crate::delta::Op::Set {
                    key: "foo".to_string(),
                    value: json!("bar"),
                    author: AgentId(2),
                    seq: 1,
                },
            ],
            VectorClock::from_entries(vec![(AgentId(2), 1)]),
        );

        let result = sync.apply_delta(delta).await;
        assert!(result.is_ok());
        // Check that map contains the key
        let value: serde_json::Value = sync.map.get("foo").unwrap();
        assert_eq!(value, json!("bar"));
        // Check that peer clock updated
        let peer_clock = sync.peer_clocks.get(&AgentId(2)).unwrap();
        assert_eq!(peer_clock.entries.get(&AgentId(2)), Some(&1));
    }

    #[tokio::test]
    async fn test_delta_for_peer() {
        let mut sync = DefaultStateSync::new(AgentId(1));
        sync.map.set("key1", json!("value1"), AgentId(1));
        sync.map.set("key2", json!("value2"), AgentId(1));

        // Peer knows nothing (empty vector clock)
        let delta = sync.delta_for_peer(AgentId(99), &VectorClock::default()).await;
        assert!(delta.is_some());
        let delta = delta.unwrap();
        assert_eq!(delta.author, AgentId(1));
        assert_eq!(delta.ops.len(), 2);

        // Peer knows up to seq 1 for agent 1
        let known = VectorClock::from_entries(vec![(AgentId(1), 1)]);
        let delta = sync.delta_for_peer(AgentId(99), &known).await;
        assert!(delta.is_some());
        let delta = delta.unwrap();
        // Should contain only the second operation (seq 2)
        assert_eq!(delta.ops.len(), 1);
        match &delta.ops[0] {
            crate::delta::Op::Set { key, .. } => assert_eq!(key, "key2"),
            _ => panic!("Expected Set op"),
        }
    }

    #[tokio::test]
    async fn test_broadcast_changes_does_not_panic() {
        let mut sync = DefaultStateSync::new(AgentId(1));
        let result = sync.broadcast_changes().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_peer_clock() {
        let mut sync = DefaultStateSync::new(AgentId(1));
        let vclock = VectorClock::from_entries(vec![(AgentId(2), 5), (AgentId(3), 7)]);
        sync.update_peer_clock(AgentId(9), &vclock);
        let stored = sync.peer_clocks.get(&AgentId(9)).unwrap();
        assert_eq!(stored.entries.get(&AgentId(2)), Some(&5));
        assert_eq!(stored.entries.get(&AgentId(3)), Some(&7));

        // Update with higher seq
        let vclock2 = VectorClock::from_entries(vec![(AgentId(2), 10)]);
        sync.update_peer_clock(AgentId(9), &vclock2);
        let stored = sync.peer_clocks.get(&AgentId(9)).unwrap();
        assert_eq!(stored.entries.get(&AgentId(2)), Some(&10)); // max
        assert_eq!(stored.entries.get(&AgentId(3)), Some(&7)); // unchanged
    }
}