//! LoRa (Long Range) backend for mesh transport.
//!
//! Provides a simulated LoRa radio backend for long-range, low-bandwidth
//! peer-to-peer communication. LoRa is inherently broadcast-only, so
//! `send_to()` is implemented as a filtered broadcast. In production,
//! this would interface with actual LoRa hardware via SPI/UART.
//!
//! Key characteristics:
//! - Low bandwidth (typically 0.3-50 kbps)
//! - Long range (up to 15 km line-of-sight)
//! - Broadcast-only medium (no native addressing)
//! - High latency (100-1000ms per packet)

use crate::backend::Backend;
use crate::message::TransportEvent;
use common::types::{AgentId, PeerInfo};
use common::error::Result;
use async_trait::async_trait;
use futures::stream::{BoxStream, StreamExt};
use tokio::sync::{broadcast, RwLock};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

/// LoRa backend configuration.
#[derive(Debug, Clone)]
pub struct LoRaConfig {
    pub frequency: u64, // Hz
    pub bandwidth: u64, // Hz
    pub spreading_factor: u8,
    pub coding_rate: u8,
    pub tx_power: i8, // dBm
    /// Simulated packet transmission time in milliseconds.
    pub simulated_tx_time_ms: u64,
    /// Simulated packet error rate (0.0 to 1.0).
    pub simulated_packet_error_rate: f64,
}

impl Default for LoRaConfig {
    fn default() -> Self {
        Self {
            frequency: 868_000_000, // 868 MHz (EU)
            bandwidth: 125_000,
            spreading_factor: 7,
            coding_rate: 5,
            tx_power: 14,
            simulated_tx_time_ms: 200,
            simulated_packet_error_rate: 0.05, // 5% packet loss simulation
        }
    }
}

/// LoRa backend with proper event streaming and peer tracking.
pub struct LoRaBackend {
    config: LoRaConfig,
    local_agent_id: AgentId,
    /// Broadcast channel for events — allows multiple subscribers without data loss.
    event_tx: broadcast::Sender<TransportEvent>,
    /// Known peers discovered via LoRa (agent_id -> PeerInfo).
    peers: Arc<RwLock<HashMap<AgentId, PeerInfo>>>,
    /// Background task handle for simulated event generation.
    _task_handle: Option<tokio::task::JoinHandle<()>>,
    /// Shutdown signal.
    shutdown: Arc<std::sync::atomic::AtomicBool>,
}

impl LoRaBackend {
    /// Create a new LoRa backend.
    pub fn new(config: LoRaConfig, local_agent_id: AgentId) -> Self {
        let (event_tx, _) = broadcast::channel(256);
        Self {
            config,
            local_agent_id,
            event_tx,
            peers: Arc::new(RwLock::new(HashMap::new())),
            _task_handle: None,
            shutdown: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Register a peer discovered via LoRa.
    pub async fn add_peer(&self, agent_id: AgentId, peer_info: PeerInfo) {
        let mut peers = self.peers.write().await;
        peers.insert(agent_id, peer_info);
        let _ = self.event_tx.send(TransportEvent::PeerDiscovered(agent_id));
    }

    /// Remove a peer that is no longer in range.
    pub async fn remove_peer(&self, agent_id: AgentId) {
        let mut peers = self.peers.write().await;
        peers.remove(&agent_id);
        let _ = self.event_tx.send(TransportEvent::PeerLost(agent_id));
    }

    /// Simulate packet loss based on the configured error rate.
    fn should_drop_packet(&self) -> bool {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        rng.gen::<f64>() < self.config.simulated_packet_error_rate
    }

    /// Calculate the approximate time-on-air for a given payload size.
    fn calculate_tx_time(&self, payload_len: usize) -> Duration {
        // LoRa time-on-air depends on spreading factor, bandwidth, coding rate, and payload length.
        // Simplified calculation: base time + payload overhead.
        let base_time = self.config.simulated_tx_time_ms;
        let overhead = (payload_len as u64) * 10; // ~10ms per byte overhead
        Duration::from_millis(base_time + overhead)
    }
}

#[async_trait]
impl Backend for LoRaBackend {
    async fn start(&mut self) -> Result<()> {
        tracing::info!(
            "LoRa backend starting (simulated) on {} Hz, SF{}, BW{}",
            self.config.frequency,
            self.config.spreading_factor,
            self.config.bandwidth,
        );
        // In production: initialize LoRa radio module (e.g., via SPI).
        // For simulation: spawn a background task for periodic beaconing.
        let event_tx = self.event_tx.clone();
        let peers = self.peers.clone();
        let shutdown = self.shutdown.clone();
        let local_id = self.local_agent_id;

        let handle = tokio::spawn(async move {
            // Emit self-discovery event
            let _ = event_tx.send(TransportEvent::PeerDiscovered(local_id));

            // Periodic beacon loop (simulates LoRa beacon frames)
            loop {
                tokio::time::sleep(Duration::from_secs(10)).await;
                if shutdown.load(std::sync::atomic::Ordering::SeqCst) {
                    break;
                }
                // Simulate periodic beacon transmission
                let peer_count = peers.read().await.len();
                tracing::trace!(
                    "LoRa beacon: {} peers in range, freq={} Hz",
                    peer_count,
                    self.config.frequency
                );
            }
            tracing::info!("LoRa backend background task stopped");
        });

        self._task_handle = Some(handle);
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        tracing::info!("LoRa backend stopping");
        self.shutdown.store(true, std::sync::atomic::Ordering::SeqCst);

        // Wait for background task to finish
        if let Some(handle) = self._task_handle.take() {
            let _ = tokio::time::timeout(Duration::from_secs(5), handle).await;
        }

        // Clear peers
        self.peers.write().await.clear();
        Ok(())
    }

    async fn send_to(&mut self, peer_id: AgentId, payload: Vec<u8>) -> Result<()> {
        // LoRa is broadcast-only; we cannot address a specific peer.
        // Simulate by checking if the peer is known and sending.
        let peers = self.peers.read().await;
        if peers.contains_key(&peer_id) {
            // Simulate packet loss
            if self.should_drop_packet() {
                tracing::warn!("LoRa: packet to {} dropped (simulated loss)", peer_id);
                return Ok(());
            }
            // Simulate transmission time
            let tx_time = self.calculate_tx_time(payload.len());
            tokio::time::sleep(tx_time).await;
            tracing::debug!(
                "LoRa: sent {} bytes to {} in {:?}",
                payload.len(),
                peer_id,
                tx_time
            );
            let _ = self.event_tx.send(TransportEvent::MessageSent(peer_id));
        } else {
            tracing::warn!("LoRa: peer {} not in range", peer_id);
        }
        Ok(())
    }

    async fn broadcast(&mut self, payload: Vec<u8>) -> Result<()> {
        // LoRa is inherently broadcast — transmit to all peers in range.
        let peer_ids: Vec<AgentId> = self.peers.read().await.keys().cloned().collect();
        tracing::debug!(
            "LoRa: broadcasting {} bytes to {} peers in range",
            payload.len(),
            peer_ids.len()
        );

        // Simulate transmission time (broadcast is a single transmission on LoRa)
        let tx_time = self.calculate_tx_time(payload.len());
        tokio::time::sleep(tx_time).await;

        for peer_id in peer_ids {
            // Simulate packet loss per peer
            if self.should_drop_packet() {
                tracing::warn!("LoRa: broadcast packet lost for peer {}", peer_id);
                continue;
            }
            let _ = self.event_tx.send(TransportEvent::MessageSent(peer_id));
        }
        Ok(())
    }

    fn peers(&self) -> Vec<PeerInfo> {
        // LoRa does not have explicit peer discovery; return known peers.
        let rt = tokio::runtime::Handle::try_current();
        match rt {
            Ok(handle) => handle.block_on(async {
                self.peers.read().await.values().cloned().collect()
            }),
            Err(_) => vec![],
        }
    }

    fn events(&mut self) -> BoxStream<'static, TransportEvent> {
        // Create a new receiver by subscribing to the broadcast channel.
        // This avoids discarding the old receiver and losing events.
        let rx = self.event_tx.subscribe();
        Box::pin(
            tokio_stream::wrappers::BroadcastStream::new(rx)
                .filter_map(|result| futures::future::ready(result.ok()))
        )
    }

    fn local_agent_id(&self) -> AgentId {
        self.local_agent_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_lora_backend_creation() {
        let config = LoRaConfig::default();
        let backend = LoRaBackend::new(config, AgentId(1));
        assert_eq!(backend.local_agent_id(), AgentId(1));
        assert!(backend.peers().is_empty());
    }

    #[tokio::test]
    async fn test_lora_backend_start_stop() {
        let config = LoRaConfig::default();
        let mut backend = LoRaBackend::new(config, AgentId(1));
        backend.start().await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        backend.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_lora_add_remove_peer() {
        let config = LoRaConfig::default();
        let backend = LoRaBackend::new(config, AgentId(1));
        let peer_info = PeerInfo {
            agent_id: AgentId(2),
            addresses: vec![],
            metadata: std::collections::HashMap::new(),
        };
        backend.add_peer(AgentId(2), peer_info).await;
        assert_eq!(backend.peers().len(), 1);
        backend.remove_peer(AgentId(2)).await;
        assert!(backend.peers().is_empty());
    }

    #[tokio::test]
    async fn test_lora_send_to_unknown_peer() {
        let config = LoRaConfig {
            simulated_packet_error_rate: 0.0, // Disable packet loss for deterministic test
            ..Default::default()
        };
        let mut backend = LoRaBackend::new(config, AgentId(1));
        // Sending to unknown peer should not error
        backend.send_to(AgentId(99), vec![1, 2, 3]).await.unwrap();
    }

    #[tokio::test]
    async fn test_lora_broadcast() {
        let config = LoRaConfig {
            simulated_packet_error_rate: 0.0,
            simulated_tx_time_ms: 10, // Fast for testing
            ..Default::default()
        };
        let mut backend = LoRaBackend::new(config, AgentId(1));
        let peer_info = PeerInfo {
            agent_id: AgentId(2),
            addresses: vec![],
            metadata: std::collections::HashMap::new(),
        };
        backend.add_peer(AgentId(2), peer_info).await;
        backend.broadcast(vec![1, 2, 3]).await.unwrap();
    }

    #[tokio::test]
    async fn test_lora_events_stream() {
        let config = LoRaConfig::default();
        let mut backend = LoRaBackend::new(config, AgentId(1));
        let mut stream = backend.events();
        // Add a peer — should generate a PeerDiscovered event
        let peer_info = PeerInfo {
            agent_id: AgentId(2),
            addresses: vec![],
            metadata: std::collections::HashMap::new(),
        };
        backend.add_peer(AgentId(2), peer_info).await;
        // The event should be in the stream
        if let Some(event) = tokio::time::timeout(Duration::from_millis(100), stream.next()).await {
            let event = event.unwrap();
            match event {
                TransportEvent::PeerDiscovered(id) => assert_eq!(id, AgentId(2)),
                _ => panic!("Expected PeerDiscovered event"),
            }
        }
    }
}