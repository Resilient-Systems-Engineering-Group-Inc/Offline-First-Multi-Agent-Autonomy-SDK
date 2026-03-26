//! Integration adapter between mesh transport and state sync.

use common::types::{AgentId, MeshMessage};
use common::error::Result;
use mesh_transport::{MeshTransport, TransportEvent};
use state_sync::{StateSync, Delta};
use async_trait::async_trait;
use futures::StreamExt;
use tokio::sync::mpsc;
use serde_json::Value;

/// Adapter that forwards messages between transport and state sync.
pub struct IntegrationAdapter {
    transport: MeshTransport,
    state_sync: Box<dyn StateSync>,
    event_rx: mpsc::UnboundedReceiver<TransportEvent>,
}

impl IntegrationAdapter {
    /// Create a new integration adapter.
    pub fn new(transport: MeshTransport, state_sync: Box<dyn StateSync>) -> Self {
        let event_rx = transport.events();
        // Convert stream to channel for simplicity (in reality we'd keep the stream)
        let (tx, rx) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            let mut stream = event_rx;
            while let Some(event) = stream.next().await {
                let _ = tx.send(event);
            }
        });

        Self {
            transport,
            state_sync,
            event_rx: rx,
        }
    }

    /// Start the integration loop.
    pub async fn run(&mut self) -> Result<()> {
        self.transport.start().await?;
        tracing::info!("Integration adapter started");

        while let Some(event) = self.event_rx.recv().await {
            match event {
                TransportEvent::MessageReceived { from, payload } => {
                    self.handle_incoming_message(from, payload).await?;
                }
                TransportEvent::PeerDiscovered(peer) => {
                    tracing::info!("Peer discovered: {:?}", peer);
                }
                _ => {}
            }
        }

        Ok(())
    }

    async fn handle_incoming_message(&mut self, from: AgentId, payload: Vec<u8>) -> Result<()> {
        // Deserialize delta
        let delta: Delta = common::utils::from_cbor(&payload)?;
        self.state_sync.apply_delta(delta).await?;
        tracing::debug!("Applied delta from {}", from.0);
        Ok(())
    }

    /// Set a key‑value pair in the local CRDT map.
    pub fn set_value<V: serde::Serialize>(&mut self, key: &str, value: V) -> Result<()> {
        let map = self.state_sync.map_mut();
        map.set(key, value, self.transport.local_agent_id());
        Ok(())
    }

    /// Get a value from the local CRDT map.
    pub fn get_value<V: for<'de> serde::Deserialize<'de>>(&self, key: &str) -> Option<V> {
        self.state_sync.map().get(key)
    }

    /// Broadcast local changes to all connected peers.
    pub async fn broadcast_changes(&mut self) -> Result<()> {
        // Generate a delta containing all changes since the beginning.
        // In a production implementation we would track per‑peer vector clocks.
        let map = self.state_sync.map();
        let empty_vclock = common::types::VectorClock::default();
        if let Some(delta) = map.delta_since(&empty_vclock) {
            let payload = common::utils::to_cbor(&delta)?;
            self.transport.broadcast(payload).await?;
            tracing::info!("Broadcast delta with {} operations", delta.ops.len());
        } else {
            tracing::debug!("No changes to broadcast");
        }
        Ok(())
    }
}