//! LoRa (Long Range) backend for mesh transport.

use crate::backend::Backend;
use crate::message::TransportEvent;
use common::types::{AgentId, PeerInfo};
use common::error::Result;
use async_trait::async_trait;
use futures::stream::BoxStream;
use tokio::sync::mpsc;

/// LoRa backend configuration.
pub struct LoRaConfig {
    pub frequency: u64, // Hz
    pub bandwidth: u64, // Hz
    pub spreading_factor: u8,
    pub coding_rate: u8,
    pub tx_power: i8, // dBm
}

impl Default for LoRaConfig {
    fn default() -> Self {
        Self {
            frequency: 868_000_000, // 868 MHz (EU)
            bandwidth: 125_000,
            spreading_factor: 7,
            coding_rate: 5,
            tx_power: 14,
        }
    }
}

/// LoRa backend (stub implementation).
pub struct LoRaBackend {
    config: LoRaConfig,
    local_agent_id: AgentId,
    event_tx: mpsc::Sender<TransportEvent>,
    event_rx: mpsc::Receiver<TransportEvent>,
}

impl LoRaBackend {
    /// Create a new LoRa backend.
    pub fn new(config: LoRaConfig, local_agent_id: AgentId) -> Self {
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
impl Backend for LoRaBackend {
    async fn start(&mut self) -> Result<()> {
        // TODO: initialize LoRa radio
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        // TODO: shutdown radio
        Ok(())
    }

    async fn send_to(&mut self, _peer_id: AgentId, _payload: Vec<u8>) -> Result<()> {
        // LoRa is broadcast‑only; we cannot address a specific peer.
        // Simulate by broadcasting.
        Ok(())
    }

    async fn broadcast(&mut self, _payload: Vec<u8>) -> Result<()> {
        // TODO: transmit over LoRa
        Ok(())
    }

    fn peers(&self) -> Vec<PeerInfo> {
        // LoRa does not have peer discovery; assume all agents in range are peers.
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