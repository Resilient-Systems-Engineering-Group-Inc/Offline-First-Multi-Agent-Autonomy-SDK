//! Connection management between peers.

use common::types::{AgentId, PeerInfo};
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::sync::mpsc;

/// Represents an active connection to a peer.
pub struct Connection {
    pub peer_id: AgentId,
    pub address: SocketAddr,
    // In a real implementation, this would hold a stream/sink.
    pub sender: mpsc::UnboundedSender<Vec<u8>>,
}

impl Connection {
    pub fn new(peer_id: AgentId, address: SocketAddr, sender: mpsc::UnboundedSender<Vec<u8>>) -> Self {
        Self {
            peer_id,
            address,
            sender,
        }
    }

    /// Send data to the peer.
    pub async fn send(&self, data: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
        self.sender.send(data).map_err(|e| e.into())
    }
}

/// Manages all active connections.
pub struct ConnectionManager {
    connections: HashMap<AgentId, Connection>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
        }
    }

    /// Add a new connection.
    pub fn add_connection(&mut self, conn: Connection) {
        self.connections.insert(conn.peer_id, conn);
    }

    /// Remove a connection.
    pub fn remove_connection(&mut self, peer_id: &AgentId) {
        self.connections.remove(peer_id);
    }

    /// Get a connection by peer ID.
    pub fn get_connection(&self, peer_id: &AgentId) -> Option<&Connection> {
        self.connections.get(peer_id)
    }

    /// List all connected peers.
    pub fn connected_peers(&self) -> Vec<PeerInfo> {
        self.connections
            .values()
            .map(|conn| PeerInfo {
                agent_id: conn.peer_id,
                addresses: vec![conn.address],
                metadata: Default::default(),
            })
            .collect()
    }

    /// Broadcast data to all connected peers.
    pub async fn broadcast(&self, data: Vec<u8>) {
        for conn in self.connections.values() {
            let _ = conn.send(data.clone()).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    #[tokio::test]
    async fn test_connection_manager() {
        let mut manager = ConnectionManager::new();
        let (tx1, mut rx1) = mpsc::unbounded_channel();
        let (tx2, mut rx2) = mpsc::unbounded_channel();

        let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8081);

        let conn1 = Connection::new(AgentId(1), addr1, tx1);
        let conn2 = Connection::new(AgentId(2), addr2, tx2);

        manager.add_connection(conn1);
        manager.add_connection(conn2);

        assert_eq!(manager.connected_peers().len(), 2);

        // Test get_connection
        assert!(manager.get_connection(&AgentId(1)).is_some());
        assert!(manager.get_connection(&AgentId(3)).is_none());

        // Test broadcast
        manager.broadcast(b"hello".to_vec()).await;

        let msg1 = rx1.recv().await.unwrap();
        let msg2 = rx2.recv().await.unwrap();
        assert_eq!(msg1, b"hello");
        assert_eq!(msg2, b"hello");

        // Test remove
        manager.remove_connection(&AgentId(1));
        assert_eq!(manager.connected_peers().len(), 1);
    }
}