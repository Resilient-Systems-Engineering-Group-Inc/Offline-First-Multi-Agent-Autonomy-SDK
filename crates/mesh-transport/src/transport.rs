//! Main transport implementation.

use crate::backend::Backend;
use crate::libp2p_backend::Libp2pBackend;
use common::types::{AgentId, PeerInfo};
use common::error::Result;
use async_trait::async_trait;
use futures::stream::BoxStream;
use tokio::sync::mpsc;

/// Configuration for the mesh transport.
#[derive(Debug, Clone)]
pub struct MeshTransportConfig {
    /// Local agent ID.
    pub local_agent_id: AgentId,
    /// Static peer list (optional).
    pub static_peers: Vec<PeerInfo>,
    /// Use mDNS discovery.
    pub use_mdns: bool,
    /// Listening address.
    pub listen_addr: String,
}

impl Default for MeshTransportConfig {
    fn default() -> Self {
        Self {
            local_agent_id: AgentId(0),
            static_peers: Vec::new(),
            use_mdns: true,
            listen_addr: "/ip4/0.0.0.0/tcp/0".to_string(),
        }
    }
}

/// The main mesh transport struct.
pub struct MeshTransport {
    backend: Box<dyn Backend>,
    event_tx: mpsc::UnboundedSender<crate::message::TransportEvent>,
    event_rx: mpsc::UnboundedReceiver<crate::message::TransportEvent>,
}

impl MeshTransport {
    /// Create a new mesh transport with the given configuration.
    pub async fn new(config: MeshTransportConfig) -> Result<Self> {
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        // For now, we always use libp2p backend.
        let mut backend = Libp2pBackend::new(config.local_agent_id).await
            .map_err(|e| common::error::SdkError::Network(e.to_string()))?;
        // Start the backend (listening etc.)
        backend.start().await?;

        Ok(Self {
            backend: Box::new(backend),
            event_tx,
            event_rx,
        })
    }

    /// Start the transport (begin discovery, listening, etc.)
    pub async fn start(&mut self) -> Result<()> {
        self.backend.start().await
    }

    /// Stop the transport.
    pub async fn stop(&mut self) -> Result<()> {
        self.backend.stop().await
    }

    /// Broadcast a message to all connected peers.
    pub async fn broadcast(&mut self, payload: Vec<u8>) -> Result<()> {
        self.backend.broadcast(payload).await
    }

    /// Send a message to a specific peer.
    pub async fn send_to(&mut self, peer_id: AgentId, payload: Vec<u8>) -> Result<()> {
        self.backend.send_to(peer_id, payload).await
    }

    /// Get a list of currently known peers.
    pub fn peers(&self) -> Vec<PeerInfo> {
        self.backend.peers()
    }

    /// Return the local agent ID.
    pub fn local_agent_id(&self) -> AgentId {
        self.backend.local_agent_id()
    }

    /// Return a stream of transport events.
    pub fn events(&mut self) -> BoxStream<'static, crate::message::TransportEvent> {
        let rx = std::mem::replace(&mut self.event_rx, mpsc::unbounded_channel().1);
        Box::pin(tokio_stream::wrappers::UnboundedReceiverStream::new(rx))
    }
}

#[async_trait]
pub trait Transport: Send + Sync {
    async fn start(&mut self) -> Result<()>;
    async fn stop(&mut self) -> Result<()>;
    async fn broadcast(&mut self, payload: Vec<u8>) -> Result<()>;
    async fn send_to(&mut self, peer_id: AgentId, payload: Vec<u8>) -> Result<()>;
    fn peers(&self) -> Vec<PeerInfo>;
    fn local_agent_id(&self) -> AgentId;
    fn events(&mut self) -> BoxStream<'static, crate::message::TransportEvent>;
}

#[async_trait]
impl Transport for MeshTransport {
    async fn start(&mut self) -> Result<()> {
        self.start().await
    }

    async fn stop(&mut self) -> Result<()> {
        self.stop().await
    }

    async fn broadcast(&mut self, payload: Vec<u8>) -> Result<()> {
        self.broadcast(payload).await
    }

    async fn send_to(&mut self, peer_id: AgentId, payload: Vec<u8>) -> Result<()> {
        self.send_to(peer_id, payload).await
    }

    fn peers(&self) -> Vec<PeerInfo> {
        self.peers()
    }

    fn local_agent_id(&self) -> AgentId {
        self.local_agent_id()
    }

    fn events(&mut self) -> BoxStream<'static, crate::message::TransportEvent> {
        self.events()
    }
}