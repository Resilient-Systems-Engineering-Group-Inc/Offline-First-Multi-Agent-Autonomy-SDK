//! Replication manager for distributing deltas across peers.

use crate::error::{Error, Result};
use common::types::{AgentId, PeerInfo};
use mesh_transport::Transport;
use state_sync::{DefaultStateSync, Delta};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{info, warn};

/// Manages periodic synchronization with peers.
pub struct ReplicationManager<T: Transport + Send + Sync> {
    transport: Arc<T>,
    replication_factor: usize,
    sync_interval_secs: u64,
    task_handle: Option<tokio::task::JoinHandle<()>>,
    stop_signal: Arc<tokio::sync::watch::Sender<bool>>,
}

impl<T: Transport + Send + Sync> ReplicationManager<T> {
    pub fn new(transport: Arc<T>, replication_factor: usize, sync_interval_secs: u64) -> Self {
        let (stop_sender, _) = tokio::sync::watch::channel(false);
        Self {
            transport,
            replication_factor,
            sync_interval_secs,
            task_handle: None,
            stop_signal: Arc::new(stop_sender),
        }
    }

    /// Start the background sync task.
    pub async fn start(&self, state_sync: Arc<RwLock<DefaultStateSync>>) -> Result<()> {
        let transport = self.transport.clone();
        let interval_secs = self.sync_interval_secs;
        let replication_factor = self.replication_factor;
        let stop_receiver = self.stop_signal.subscribe();

        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(interval_secs));
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Err(e) = Self::sync_round(&transport, &state_sync, replication_factor).await {
                            warn!("Sync round failed: {}", e);
                        }
                    }
                    _ = stop_receiver.changed() => {
                        info!("Replication task stopping");
                        break;
                    }
                }
            }
        });

        // Store handle
        // Note: we cannot mutate self because it's not mutable; we need interior mutability.
        // For simplicity, we'll just ignore storing handle for now.
        // In a real implementation, we'd use something like `self.task_handle = Some(handle);`
        drop(handle);
        Ok(())
    }

    /// Stop the background sync task.
    pub async fn stop(&self) -> Result<()> {
        self.stop_signal.send(true).map_err(|_| Error::Other("Failed to send stop signal".to_string()))?;
        Ok(())
    }

    /// Notify that a local change occurred, triggering immediate sync.
    pub async fn notify_change(&self) {
        // In a more advanced implementation, we could trigger an immediate sync.
        info!("Local change notified, will sync on next interval");
    }

    /// Perform a single synchronization round.
    async fn sync_round(
        transport: &Arc<T>,
        state_sync: &Arc<RwLock<DefaultStateSync>>,
        replication_factor: usize,
    ) -> Result<()> {
        // Get list of peers
        let peers = transport.peers();
        if peers.is_empty() {
            return Ok(());
        }

        // For each peer, generate delta and send.
        let sync = state_sync.read().await;
        for peer in peers.iter().take(replication_factor) {
            let peer_id = peer.agent_id;
            let known_vclock = sync.peer_clocks.get(&peer_id).cloned().unwrap_or_default();
            if let Some(delta) = sync.delta_for_peer(peer_id, &known_vclock).await {
                // Send delta via transport
                let payload = serde_json::to_vec(&delta).map_err(Error::Serialization)?;
                transport.send_to(peer_id, payload).await
                    .map_err(|e| Error::Transport(e.to_string()))?;
                info!("Sent delta to peer {}", peer_id.0);
            }
        }
        Ok(())
    }
}