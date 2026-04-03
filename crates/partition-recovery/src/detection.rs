//! Network partition detection based on heartbeat and connectivity.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use common::types::AgentId;
use mesh_transport::TransportEvent;

/// Configuration for partition detection.
#[derive(Clone, Debug)]
pub struct PartitionDetectionConfig {
    /// Heartbeat interval (how often to send ping messages).
    pub heartbeat_interval: Duration,
    /// Timeout after which a peer is considered disconnected.
    pub heartbeat_timeout: Duration,
    /// Minimum number of peers that must be reachable to consider partition.
    pub min_reachable_peers: usize,
    /// Enable adaptive timeout based on network conditions.
    pub adaptive_timeout: bool,
}

impl Default for PartitionDetectionConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval: Duration::from_secs(5),
            heartbeat_timeout: Duration::from_secs(15),
            min_reachable_peers: 1,
            adaptive_timeout: false,
        }
    }
}

/// Tracks the last seen timestamp of each peer.
struct PeerStatus {
    last_seen: Instant,
    consecutive_misses: u32,
}

/// Detects network partitions by monitoring peer connectivity.
pub struct PartitionDetector {
    config: PartitionDetectionConfig,
    peers: RwLock<HashMap<AgentId, PeerStatus>>,
    local_agent: AgentId,
    known_peers: HashSet<AgentId>,
}

impl PartitionDetector {
    /// Create a new detector.
    pub fn new(
        local_agent: AgentId,
        known_peers: HashSet<AgentId>,
        config: PartitionDetectionConfig,
    ) -> Self {
        let mut peers = HashMap::new();
        let now = Instant::now();
        for &peer in &known_peers {
            peers.insert(peer, PeerStatus {
                last_seen: now,
                consecutive_misses: 0,
            });
        }
        Self {
            config,
            peers: RwLock::new(peers),
            local_agent,
            known_peers,
        }
    }

    /// Update peer status based on a received heartbeat or any message.
    pub async fn on_message_received(&self, from: AgentId) {
        let mut peers = self.peers.write().await;
        if let Some(status) = peers.get_mut(&from) {
            status.last_seen = Instant::now();
            status.consecutive_misses = 0;
        } else {
            // New peer discovered
            peers.insert(from, PeerStatus {
                last_seen: Instant::now(),
                consecutive_misses: 0,
            });
        }
    }

    /// Mark that a heartbeat was sent (optional).
    pub async fn on_heartbeat_sent(&self, to: AgentId) {
        // Could track sent time for RTT estimation
    }

    /// Check for partitions.
    /// Returns a set of agents that are considered disconnected.
    pub async fn detect_partitions(&self) -> HashSet<AgentId> {
        let mut disconnected = HashSet::new();
        let now = Instant::now();
        let peers = self.peers.read().await;
        for (&agent, status) in peers.iter() {
            if agent == self.local_agent {
                continue;
            }
            let elapsed = now.duration_since(status.last_seen);
            if elapsed > self.config.heartbeat_timeout {
                disconnected.insert(agent);
            }
        }
        disconnected
    }

    /// Determine if we are in a minority partition.
    /// Returns true if the number of reachable peers is less than min_reachable_peers.
    pub async fn is_minority_partition(&self) -> bool {
        let disconnected = self.detect_partitions().await;
        let reachable = self.known_peers.len() - disconnected.len();
        reachable < self.config.min_reachable_peers
    }

    /// Process transport events to update connectivity.
    pub async fn on_transport_event(&self, event: &TransportEvent) {
        match event {
            TransportEvent::MessageReceived { from, .. } => {
                self.on_message_received(*from).await;
            }
            TransportEvent::PeerConnected(peer) => {
                self.on_message_received(*peer).await;
            }
            TransportEvent::PeerDisconnected(peer) => {
                // Mark as disconnected immediately
                let mut peers = self.peers.write().await;
                if let Some(status) = peers.get_mut(peer) {
                    status.consecutive_misses += 1;
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_detection() {
        let local = 1;
        let peers: HashSet<AgentId> = [2, 3].iter().cloned().collect();
        let config = PartitionDetectionConfig {
            heartbeat_timeout: Duration::from_millis(100),
            ..Default::default()
        };
        let detector = PartitionDetector::new(local, peers, config);
        // Initially no partitions
        let disconnected = detector.detect_partitions().await;
        assert!(disconnected.is_empty());
        // Simulate time passing
        tokio::time::sleep(Duration::from_millis(150)).await;
        let disconnected = detector.detect_partitions().await;
        assert_eq!(disconnected.len(), 2);
        // Receive a message from peer 2
        detector.on_message_received(2).await;
        let disconnected = detector.detect_partitions().await;
        assert_eq!(disconnected.len(), 1);
        assert!(disconnected.contains(&3));
    }
}