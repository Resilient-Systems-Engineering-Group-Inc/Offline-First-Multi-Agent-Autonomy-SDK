//! High‑level manager that orchestrates detection and recovery.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{interval, timeout};
use common::types::AgentId;
use mesh_transport::{MeshTransport, TransportEvent};
use state_sync::{StateSync, DefaultStateSync};
use crate::detection::{PartitionDetector, PartitionDetectionConfig};
use crate::recovery::{PartitionRecovery, RecoveryConfig, CrdtAutoRecovery};
use crate::error::PartitionRecoveryError;

/// Configuration for the partition recovery manager.
#[derive(Clone, Debug)]
pub struct PartitionRecoveryManagerConfig {
    pub detection: PartitionDetectionConfig,
    pub recovery: RecoveryConfig,
    /// How often to run detection (seconds).
    pub detection_interval_secs: u64,
    /// Enable automatic recovery.
    pub auto_recovery: bool,
}

impl Default for PartitionRecoveryManagerConfig {
    fn default() -> Self {
        Self {
            detection: PartitionDetectionConfig::default(),
            recovery: RecoveryConfig::default(),
            detection_interval_secs: 10,
            auto_recovery: true,
        }
    }
}

/// The main manager that ties everything together.
pub struct PartitionRecoveryManager<T: StateSync + Send + Sync + 'static> {
    config: PartitionRecoveryManagerConfig,
    detector: PartitionDetector,
    recovery: PartitionRecovery<T>,
    auto_recovery: CrdtAutoRecovery<T>,
    transport: Arc<MeshTransport>,
    state_sync: Arc<RwLock<T>>,
    local_agent: AgentId,
    known_peers: HashSet<AgentId>,
    event_rx: mpsc::UnboundedReceiver<TransportEvent>,
    event_tx: mpsc::UnboundedSender<TransportEvent>,
}

impl<T: StateSync + Send + Sync + 'static> PartitionRecoveryManager<T> {
    /// Create a new manager.
    pub fn new(
        config: PartitionRecoveryManagerConfig,
        transport: Arc<MeshTransport>,
        state_sync: Arc<RwLock<T>>,
        local_agent: AgentId,
        known_peers: HashSet<AgentId>,
    ) -> Result<Self, PartitionRecoveryError> {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let detector = PartitionDetector::new(
            local_agent,
            known_peers.clone(),
            config.detection.clone(),
        );
        let recovery = PartitionRecovery::new(
            config.recovery.clone(),
            Arc::clone(&state_sync),
            local_agent,
        );
        let auto_recovery = CrdtAutoRecovery::new(Arc::clone(&state_sync));
        Ok(Self {
            config,
            detector,
            recovery,
            auto_recovery,
            transport,
            state_sync,
            local_agent,
            known_peers,
            event_rx,
            event_tx,
        })
    }

    /// Start the manager (spawns background tasks).
    pub async fn start(mut self) -> Result<(), PartitionRecoveryError> {
        let detection_interval = Duration::from_secs(self.config.detection_interval_secs);
        let mut interval = interval(detection_interval);

        // Spawn a task to forward transport events to detector
        let detector = self.detector.clone();
        let event_tx = self.event_tx.clone();
        let mut transport_events = self.transport.events();
        tokio::spawn(async move {
            while let Some(event) = transport_events.next().await {
                detector.on_transport_event(&event).await;
                let _ = event_tx.send(event);
            }
        });

        // Main loop
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = self.run_detection_round().await {
                        tracing::error!("Detection round failed: {}", e);
                    }
                }
                Some(event) = self.event_rx.recv() => {
                    self.handle_event(event).await;
                }
            }
        }
    }

    async fn run_detection_round(&self) -> Result<(), PartitionRecoveryError> {
        let disconnected = self.detector.detect_partitions().await;
        if !disconnected.is_empty() {
            tracing::warn!("Partition detected: disconnected peers {:?}", disconnected);
            if self.config.auto_recovery {
                self.start_recovery(disconnected).await?;
            }
        }
        Ok(())
    }

    async fn start_recovery(&self, disconnected: HashSet<AgentId>) -> Result<(), PartitionRecoveryError> {
        tracing::info!("Starting automatic recovery");
        // First, try simple CRDT sync
        self.auto_recovery.trigger_sync().await?;
        // If still partitioned, run full recovery protocol
        let all_peers = self.known_peers.clone();
        self.recovery.start_recovery(disconnected, all_peers).await
    }

    async fn handle_event(&self, event: TransportEvent) {
        match event {
            TransportEvent::MessageReceived { from, payload } => {
                // Check if it's a recovery message
                if payload.starts_with(b"RECOVERY") {
                    let _ = self.recovery.handle_recovery_message(from, payload).await;
                }
            }
            _ => {}
        }
    }

    /// Get a clone of the event sender (for external integration).
    pub fn event_sender(&self) -> mpsc::UnboundedSender<TransportEvent> {
        self.event_tx.clone()
    }
}

use futures::StreamExt;

#[cfg(test)]
mod tests {
    use super::*;
    use mesh_transport::{MeshTransportConfig, BackendType};
    use state_sync::DefaultStateSync;

    #[tokio::test]
    async fn test_manager_creation() {
        let config = PartitionRecoveryManagerConfig::default();
        let transport = Arc::new(
            MeshTransport::new(MeshTransportConfig {
                backend_type: BackendType::InMemory,
                ..Default::default()
            }).await.unwrap()
        );
        let state_sync = Arc::new(RwLock::new(DefaultStateSync::new(1)));
        let known_peers: HashSet<AgentId> = [2, 3].iter().cloned().collect();
        let manager = PartitionRecoveryManager::new(
            config,
            transport,
            state_sync,
            1,
            known_peers,
        );
        assert!(manager.is_ok());
    }
}