//! Message types for mesh communication.

use common::types::{AgentId, MeshMessage};
use serde::{Deserialize, Serialize};

/// Internal event used by the transport layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransportEvent {
    /// A new peer discovered.
    PeerDiscovered(AgentId),
    /// A peer is no longer reachable.
    PeerLost(AgentId),
    /// A message received from a peer.
    MessageReceived {
        from: AgentId,
        payload: Vec<u8>,
    },
    /// Connection established with a peer.
    ConnectionEstablished(AgentId),
    /// Connection closed with a peer.
    ConnectionClosed(AgentId),
}

/// Protocol message exchanged between peers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProtocolMessage {
    /// Ping request.
    Ping,
    /// Ping response.
    Pong,
    /// User data.
    Data(Vec<u8>),
    /// Request to sync peer list.
    SyncPeers,
    /// Response with peer list.
    PeerList(Vec<AgentId>),
}

impl From<MeshMessage> for ProtocolMessage {
    fn from(msg: MeshMessage) -> Self {
        ProtocolMessage::Data(msg.payload)
    }
}