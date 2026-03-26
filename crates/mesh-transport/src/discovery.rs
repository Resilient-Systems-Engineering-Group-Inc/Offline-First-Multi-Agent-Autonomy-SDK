//! Peer discovery mechanisms.

use async_trait::async_trait;
use common::types::{AgentId, PeerInfo};
use std::net::SocketAddr;

/// Trait for peer discovery.
#[async_trait]
pub trait Discovery: Send + Sync {
    /// Discover peers currently reachable.
    async fn discover_peers(&self) -> Vec<PeerInfo>;

    /// Register a peer manually (e.g., from configuration).
    async fn add_peer(&mut self, peer: PeerInfo);

    /// Remove a peer from the discovery list.
    async fn remove_peer(&mut self, agent_id: AgentId);

    /// Start the discovery process (e.g., start listening for advertisements).
    async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>>;

    /// Stop discovery.
    async fn stop(&mut self);
}

/// mDNS‑based discovery using libp2p.
pub struct MdnsDiscovery {
    // Libp2p swarm handle etc.
    // Placeholder for now.
}

impl MdnsDiscovery {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl Discovery for MdnsDiscovery {
    async fn discover_peers(&self) -> Vec<PeerInfo> {
        // TODO: integrate with libp2p mdns
        vec![]
    }

    async fn add_peer(&mut self, _peer: PeerInfo) {
        // Not applicable for mDNS
    }

    async fn remove_peer(&mut self, _agent_id: AgentId) {
        // Not applicable
    }

    async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        tracing::info!("mDNS discovery started");
        Ok(())
    }

    async fn stop(&mut self) {
        tracing::info!("mDNS discovery stopped");
    }
}

/// Static discovery from a predefined list.
pub struct StaticDiscovery {
    peers: Vec<PeerInfo>,
}

impl StaticDiscovery {
    pub fn new(peers: Vec<PeerInfo>) -> Self {
        Self { peers }
    }
}

#[async_trait]
impl Discovery for StaticDiscovery {
    async fn discover_peers(&self) -> Vec<PeerInfo> {
        self.peers.clone()
    }

    async fn add_peer(&mut self, peer: PeerInfo) {
        self.peers.push(peer);
    }

    async fn remove_peer(&mut self, agent_id: AgentId) {
        self.peers.retain(|p| p.agent_id != agent_id);
    }

    async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    async fn stop(&mut self) {}
}