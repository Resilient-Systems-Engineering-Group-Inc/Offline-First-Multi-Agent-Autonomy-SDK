//! High‑level agent abstraction.

use crate::integration::IntegrationAdapter;
use crate::fault_tolerance::FaultToleranceManager;
use common::types::AgentId;
use common::error::Result;
use mesh_transport::{MeshTransport, MeshTransportConfig};
use state_sync::{DefaultStateSync, StateSync};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

#[cfg(feature = "iot")]
use iot_interface::{DeviceRegistry, SensorConfig, ActuatorConfig};

/// A full‑fledged agent combining transport, state sync, and application logic.
pub struct Agent {
    id: AgentId,
    integration: IntegrationAdapter,
    task_handle: Option<JoinHandle<Result<()>>>,
    fault_handle: Option<JoinHandle<()>>,
    #[cfg(feature = "iot")]
    device_registry: DeviceRegistry,
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
            #[cfg(feature = "iot")]
            device_registry: DeviceRegistry::new(),
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

    /// IoT‑related methods (available only with the `iot` feature).
    #[cfg(feature = "iot")]
    pub async fn add_sensor(&self, config: SensorConfig, protocol: &str) -> Result<()> {
        self.device_registry.add_sensor(config, protocol).await
    }

    #[cfg(feature = "iot")]
    pub async fn add_actuator(&self, config: ActuatorConfig, protocol: &str) -> Result<()> {
        self.device_registry.add_actuator(config, protocol).await
    }

    #[cfg(feature = "iot")]
    pub async fn get_sensor(&self, id: &str) -> Option<std::sync::Arc<dyn iot_interface::Sensor>> {
        self.device_registry.get_sensor(id).await
    }

    #[cfg(feature = "iot")]
    pub async fn get_actuator(&self, id: &str) -> Option<std::sync::Arc<dyn iot_interface::Actuator>> {
        self.device_registry.get_actuator(id).await
    }

    #[cfg(feature = "iot")]
    pub async fn list_sensors(&self) -> Vec<String> {
        self.device_registry.list_sensors().await
    }

    #[cfg(feature = "iot")]
    pub async fn list_actuators(&self) -> Vec<String> {
        self.device_registry.list_actuators().await
    }
}