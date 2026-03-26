//! Abstract backend for mesh transport.

use crate::message::TransportEvent;
use common::types::{AgentId, PeerInfo};
use common::error::Result;
use async_trait::async_trait;
use futures::stream::BoxStream;

/// A backend that provides networking capabilities.
#[async_trait]
pub trait Backend: Send + Sync {
    /// Start the backend.
    async fn start(&mut self) -> Result<()>;
    /// Stop the backend.
    async fn stop(&mut self) -> Result<()>;
    /// Send a message to a specific peer.
    async fn send_to(&mut self, peer_id: AgentId, payload: Vec<u8>) -> Result<()>;
    /// Broadcast a message to all connected peers.
    async fn broadcast(&mut self, payload: Vec<u8>) -> Result<()>;
    /// Get a list of currently known peers.
    fn peers(&self) -> Vec<PeerInfo>;
    /// Return a stream of transport events.
    fn events(&mut self) -> BoxStream<'static, TransportEvent>;
    /// Get the local agent ID.
    fn local_agent_id(&self) -> AgentId;
}