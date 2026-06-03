//! WebRTC backend for mesh transport.
//!
//! Provides a simulated WebRTC data channel backend for peer-to-peer communication.
//! In a production environment, this would use the `webrtc` crate for actual
//! WebRTC data channel connections. This implementation provides the full
//! `Backend` trait with proper event streaming, peer tracking, and simulation.

use crate::backend::Backend;
use crate::message::TransportEvent;
use common::types::{AgentId, PeerInfo};
use common::error::Result;
use async_trait::async_trait;
use futures::stream::{BoxStream, StreamExt};
use tokio::sync::{broadcast, mpsc, RwLock};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

/// WebRTC backend configuration.
#[derive(Debug, Clone)]
pub struct WebRtcConfig {
    pub stun_servers: Vec<String>,
    pub turn_servers: Vec<(String, String, String)>, // (url, username, credential)
    pub data_channel_label: String,
    /// Simulated network latency for testing.
    pub simulated_latency_ms: u64,
}

impl Default for WebRtcConfig {
    fn default() -> Self {
        Self {
            stun_servers: vec!["stun:stun.l.google.com:19302".to_string()],
            turn_servers: vec![],
            data_channel_label: "mesh".to_string(),
            simulated_latency_ms: 10,
        }
    }
}

/// WebRTC backend with proper event streaming and peer tracking.
pub struct WebRtcBackend {
    config: WebRtcConfig,
    local_agent_id: AgentId,
    /// Broadcast channel for events — allows multiple subscribers without data loss.
    event_tx: broadcast::Sender<TransportEvent>,
    /// Known peers (agent_id -> PeerInfo).
    peers: Arc<RwLock<HashMap<AgentId, PeerInfo>>>,
    /// Background task handle for simulated event generation.
    _task_handle: Option<tokio::task::JoinHandle<()>>,
    /// Shutdown signal.
    shutdown: Arc<std::sync::atomic::AtomicBool>,
}

impl WebRtcBackend {
    /// Create a new WebRTC backend.
    pub fn new(config: WebRtcConfig, local_agent_id: AgentId) -> Self {
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

    /// Register a peer that has been discovered via signaling.
    pub async fn add_peer(&self, agent_id: AgentId, peer_info: PeerInfo) {
        let mut peers = self.peers.write().await;
        peers.insert(agent_id, peer_info);
        let _ = self.event_tx.send(TransportEvent::PeerDiscovered(agent_id));
    }

    /// Remove a peer that has disconnected.
    pub async fn remove_peer(&self, agent_id: AgentId) {
        let mut peers = self.peers.write().await;
        peers.remove(&agent_id);
        let _ = self.event_tx.send(TransportEvent::PeerLost(agent_id));
    }
}

#[async_trait]
impl Backend for WebRtcBackend {
    async fn start(&mut self) -> Result<()> {
        tracing::info!(
            "WebRTC backend starting (simulated) with {} STUN servers",
            self.config.stun_servers.len()
        );
        // In production: initialize WebRTC peer connections, ICE agents, etc.
        // For simulation: spawn a background task that generates periodic events.
        let event_tx = self.event_tx.clone();
        let peers = self.peers.clone();
        let shutdown = self.shutdown.clone();
        let local_id = self.local_agent_id;

        let handle = tokio::spawn(async move {
            // Simulate initial self-discovery
            let _ = event_tx.send(TransportEvent::PeerDiscovered(local_id));

            // Periodic heartbeat simulation
            loop {
                tokio::time::sleep(Duration::from_secs(5)).await;
                if shutdown.load(std::sync::atomic::Ordering::SeqCst) {
                    break;
                }
                // Simulate connection health check
                let peer_count = peers.read().await.len();
                tracing::trace!(
                    "WebRTC backend heartbeat: {} connected peers",
                    peer_count
                );
            }
            tracing::info!("WebRTC backend background task stopped");
        });

        self._task_handle = Some(handle);
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        tracing::info!("WebRTC backend stopping");
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
        // In production: send via WebRTC data channel to the specific peer.
        // For simulation: emit a MessageReceived event.
        let peers = self.peers.read().await;
        if peers.contains_key(&peer_id) {
            tracing::debug!("WebRTC sending {} bytes to peer {}", payload.len(), peer_id);
            // Simulate network latency
            if self.config.simulated_latency_ms > 0 {
                tokio::time::sleep(Duration::from_millis(self.config.simulated_latency_ms)).await;
            }
            let _ = self.event_tx.send(TransportEvent::MessageSent(peer_id));
        } else {
            tracing::warn!("WebRTC: peer {} not found, cannot send", peer_id);
        }
        Ok(())
    }

    async fn broadcast(&mut self, payload: Vec<u8>) -> Result<()> {
        // WebRTC does not natively support broadcast; simulate by sending to each peer.
        let peer_ids: Vec<AgentId> = self.peers.read().await.keys().cloned().collect();
        tracing::debug!(
            "WebRTC broadcasting {} bytes to {} peers",
            payload.len(),
            peer_ids.len()
        );
        for peer_id in peer_ids {
            if self.config.simulated_latency_ms > 0 {
                tokio::time::sleep(Duration::from_millis(self.config.simulated_latency_ms)).await;
            }
            let _ = self.event_tx.send(TransportEvent::MessageSent(peer_id));
        }
        Ok(())
    }

    fn peers(&self) -> Vec<PeerInfo> {
        // In a real implementation, this would return connected WebRTC peers.
        // For simulation, return the tracked peers.
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
    async fn test_webrtc_backend_creation() {
        let config = WebRtcConfig::default();
        let backend = WebRtcBackend::new(config, AgentId(1));
        assert_eq!(backend.local_agent_id(), AgentId(1));
        assert!(backend.peers().is_empty());
    }

    #[tokio::test]
    async fn test_webrtc_backend_start_stop() {
        let config = WebRtcConfig::default();
        let mut backend = WebRtcBackend::new(config, AgentId(1));
        backend.start().await.unwrap();
        // Give it a moment to spawn the background task
        tokio::time::sleep(Duration::from_millis(50)).await;
        backend.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_webrtc_add_remove_peer() {
        let config = WebRtcConfig::default();
        let backend = WebRtcBackend::new(config, AgentId(1));
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
    async fn test_webrtc_events_stream() {
        let config = WebRtcConfig::default();
        let mut backend = WebRtcBackend::new(config, AgentId(1));
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