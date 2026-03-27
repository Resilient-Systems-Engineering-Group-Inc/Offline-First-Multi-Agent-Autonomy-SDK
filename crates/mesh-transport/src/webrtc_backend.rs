//! WebRTC backend for mesh transport.

use crate::backend::Backend;
use crate::message::TransportEvent;
use common::types::{AgentId, PeerInfo};
use common::error::Result;
use async_trait::async_trait;
use futures::stream::BoxStream;
use futures::stream::StreamExt;
use tokio::sync::mpsc;

/// WebRTC backend configuration.
pub struct WebRtcConfig {
    pub stun_servers: Vec<String>,
    pub turn_servers: Vec<(String, String, String)>, // (url, username, credential)
    pub data_channel_label: String,
}

impl Default for WebRtcConfig {
    fn default() -> Self {
        Self {
            stun_servers: vec!["stun:stun.l.google.com:19302".to_string()],
            turn_servers: vec![],
            data_channel_label: "mesh".to_string(),
        }
    }
}

/// WebRTC backend (stub implementation).
pub struct WebRtcBackend {
    config: WebRtcConfig,
    local_agent_id: AgentId,
    event_tx: mpsc::Sender<TransportEvent>,
    event_rx: mpsc::Receiver<TransportEvent>,
}

impl WebRtcBackend {
    /// Create a new WebRTC backend.
    pub fn new(config: WebRtcConfig, local_agent_id: AgentId) -> Self {
        let (event_tx, event_rx) = mpsc::channel(100);
        Self {
            config,
            local_agent_id,
            event_tx,
            event_rx,
        }
    }
}

#[async_trait]
impl Backend for WebRtcBackend {
    async fn start(&mut self) -> Result<()> {
        // TODO: implement WebRTC connection establishment
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        Ok(())
    }

    async fn send_to(&mut self, _peer_id: AgentId, _payload: Vec<u8>) -> Result<()> {
        // TODO: send via data channel
        Ok(())
    }

    async fn broadcast(&mut self, _payload: Vec<u8>) -> Result<()> {
        // WebRTC does not natively support broadcast; simulate by sending to each peer.
        Ok(())
    }

    fn peers(&self) -> Vec<PeerInfo> {
        vec![]
    }

    fn events(&mut self) -> BoxStream<'static, TransportEvent> {
        let rx = std::mem::replace(&mut self.event_rx, mpsc::channel(100).1);
        Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx))
    }

    fn local_agent_id(&self) -> AgentId {
        self.local_agent_id
    }
}