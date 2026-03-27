//! High‑level agent abstraction.

use crate::integration::IntegrationAdapter;
use crate::fault_tolerance::FaultToleranceManager;
use common::types::AgentId;
use common::error::Result;
use mesh_transport::{MeshTransport, MeshTransportConfig};
use state_sync::{DefaultStateSync, StateSync};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

/// A full‑fledged agent combining transport, state sync, and application logic.
pub struct Agent {
    id: AgentId,
    integration: IntegrationAdapter,
    task_handle: Option<JoinHandle<Result<()>>>,
    fault_handle: Option<JoinHandle<()>>,
}

impl Agent {
    /// Create a new agent with the given configuration.
    pub fn new(id: AgentId, config: MeshTransportConfig) -> Result<Self> {
        let transport = MeshTransport::new(config)?;
        let state_sync = Box::new(DefaultStateSync::new(id));

        // Create channel for fault tolerance events
        let (fault_tx, fault_rx) = mpsc::unbounded_channel();
        let integration = IntegrationAdapter::new(transport, state_sync, Some(fault_tx));

        // Start fault tolerance manager in background
        let fault_manager = FaultToleranceManager::new(fault_rx);
        let fault_handle = tokio::spawn(async move {
            fault_manager.run().await;
        });

        Ok(Self {
            id,
            integration,
            task_handle: None,
            fault_handle: Some(fault_handle),
        })
    }

    /// Start the agent (non‑blocking).
    pub fn start(&mut self) -> Result<()> {
        let mut integration = std::mem::replace(&mut self.integration, unreachable!());
        let handle = tokio::spawn(async move {
            integration.run().await
        });
        self.task_handle = Some(handle);
        Ok(())
    }

    /// Stop the agent.
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
            let _ = handle.await;
        }
        if let Some(fault_handle) = self.fault_handle.take() {
            fault_handle.abort();
            let _ = fault_handle.await;
        }
        Ok(())
    }

    /// Get the agent's ID.
    pub fn id(&self) -> AgentId {
        self.id
    }

    /// Set a key‑value pair in the agent's CRDT map.
    pub fn set_value<V: serde::Serialize>(&mut self, key: &str, value: V) -> Result<()> {
        self.integration.set_value(key, value)
    }

    /// Get a value from the agent's CRDT map.
    pub fn get_value<V: for<'de> serde::Deserialize<'de>>(&self, key: &str) -> Option<V> {
        self.integration.get_value(key)
    }

    /// Broadcast local changes.
    pub async fn broadcast_changes(&mut self) -> Result<()> {
        self.integration.broadcast_changes().await
    }
}