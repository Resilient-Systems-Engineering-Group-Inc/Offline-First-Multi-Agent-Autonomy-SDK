//! Main transport implementation.

use crate::backend::Backend;
use crate::libp2p_backend::Libp2pBackend;
use crate::in_memory_backend::InMemoryBackend;
use crate::webrtc_backend::{WebRtcBackend, WebRtcConfig};
use crate::lora_backend::{LoRaBackend, LoRaConfig};
use common::types::{AgentId, PeerInfo};
use common::error::Result;
use common::metrics;
use async_trait::async_trait;
use futures::stream::BoxStream;
use tokio::sync::mpsc;

/// Type of backend to use for the mesh transport.
#[derive(Debug, Clone, PartialEq)]
pub enum BackendType {
    /// libp2p‑based backend (TCP, WebSocket, mDNS).
    Libp2p,
    /// In‑memory backend for testing/simulation.
    InMemory,
    /// WebRTC data channels (for browser/Web environments).
    WebRtc,
    /// LoRa radio (long‑range, low‑bandwidth).
    LoRa,
}

impl Default for BackendType {
    fn default() -> Self {
        BackendType::Libp2p
    }
}

/// Configuration for the mesh transport.
#[derive(Debug, Clone)]
pub struct MeshTransportConfig {
    /// Local agent ID.
    pub local_agent_id: AgentId,
    /// Static peer list (optional).
    pub static_peers: Vec<PeerInfo>,
    /// Use mDNS discovery (only for Libp2p backend).
    pub use_mdns: bool,
    /// Listening address (for Libp2p backend).
    pub listen_addr: String,
    /// Backend type.
    pub backend_type: BackendType,
    /// WebRTC‑specific configuration (if backend_type is WebRtc).
    pub webrtc_config: Option<WebRtcConfig>,
    /// LoRa‑specific configuration (if backend_type is LoRa).
    pub lora_config: Option<LoRaConfig>,
}

impl Default for MeshTransportConfig {
    fn default() -> Self {
        Self {
            local_agent_id: AgentId(0),
            static_peers: Vec::new(),
            use_mdns: true,
            listen_addr: "/ip4/0.0.0.0/tcp/0".to_string(),
            backend_type: BackendType::Libp2p,
            webrtc_config: None,
            lora_config: None,
        }
    }
}

impl MeshTransportConfig {
    /// Create a configuration that uses the in‑memory backend.
    pub fn in_memory() -> Self {
        Self {
            backend_type: BackendType::InMemory,
            ..Default::default()
        }
    }

    /// Create a configuration that uses the WebRTC backend.
    pub fn webrtc(config: WebRtcConfig) -> Self {
        Self {
            backend_type: BackendType::WebRtc,
            webrtc_config: Some(config),
            ..Default::default()
        }
    }

    /// Create a configuration that uses the LoRa backend.
    pub fn lora(config: LoRaConfig) -> Self {
        Self {
            backend_type: BackendType::LoRa,
            lora_config: Some(config),
            ..Default::default()
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

        let mut backend: Box<dyn Backend> = match config.backend_type {
            BackendType::InMemory => {
                let backend = InMemoryBackend::new(config.clone()).await?;
                Box::new(backend)
            }
            BackendType::Libp2p => {
                let backend = Libp2pBackend::new(config).await
                    .map_err(|e| common::error::SdkError::Network(e.to_string()))?;
                Box::new(backend)
            }
            BackendType::WebRtc => {
                let webrtc_config = config.webrtc_config.unwrap_or_default();
                let backend = WebRtcBackend::new(webrtc_config, config.local_agent_id);
                Box::new(backend)
            }
            BackendType::LoRa => {
                let lora_config = config.lora_config.unwrap_or_default();
                let backend = LoRaBackend::new(lora_config, config.local_agent_id);
                Box::new(backend)
            }
        };
        // Start the backend (listening etc.)
        backend.start().await?;

        Ok(Self {
            backend,
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
        metrics::inc_messages_sent();
        self.backend.broadcast(payload).await
    }

    /// Send a message to a specific peer.
    pub async fn send_to(&mut self, peer_id: AgentId, payload: Vec<u8>) -> Result<()> {
        metrics::inc_messages_sent();
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