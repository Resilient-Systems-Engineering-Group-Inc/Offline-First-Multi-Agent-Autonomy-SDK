//! High‑level agent abstraction.
//!
//! Provides a full‑fledged agent that combines mesh transport, state synchronization,
//! fault tolerance, and application logic into a single cohesive unit.
//!
//! # Architecture
//!
//! The `Agent` struct orchestrates the following subsystems:
//! - **Mesh Transport**: Peer‑to‑peer communication over ad‑hoc networks
//! - **State Sync**: CRDT‑based eventually‑consistent state synchronization
//! - **Fault Tolerance**: Heartbeat monitoring, failure detection, and task reallocation
//! - **IoT Integration** (optional): Sensor and actuator management
//!
//! # Example
//!
//! ```no_run
//! use agent_core::Agent;
//! use mesh_transport::MeshTransportConfig;
//! use common::types::AgentId;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut agent = Agent::new(1, MeshTransportConfig::in_memory())?;
//! agent.start()?;
//! agent.set_value("status", "active")?;
//! agent.stop().await?;
//! # Ok(())
//! # }
//! ```

use crate::integration::IntegrationAdapter;
use crate::fault_tolerance::FaultToleranceManager;
use common::types::AgentId;
use common::error::{Result, SdkError};
use mesh_transport::{MeshTransport, MeshTransportConfig};
use state_sync::{DefaultStateSync, StateSync};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{info, warn, error};

#[cfg(feature = "iot")]
use iot_interface::{DeviceRegistry, SensorConfig, ActuatorConfig};

/// A full‑fledged agent combining transport, state sync, and application logic.
///
/// The agent lifecycle follows a strict state machine:
/// 1. `new()` — Creates the agent and initializes subsystems
/// 2. `start()` — Spawns the main event loop (non‑blocking)
/// 3. `stop()` — Gracefully shuts down all subsystems
///
/// # Panics
///
/// This struct will never panic due to internal state management. All dangerous
/// operations use `Option`-based take patterns instead of `unreachable!()`.
pub struct Agent {
    id: AgentId,
    integration: Option<IntegrationAdapter>,
    task_handle: Option<JoinHandle<Result<()>>>,
    fault_handle: Option<JoinHandle<()>>,
    started: bool,
    #[cfg(feature = "iot")]
    device_registry: DeviceRegistry,
}

impl Agent {
    /// Create a new agent with the given configuration.
    ///
    /// This initializes the mesh transport, state synchronization, and fault tolerance
    /// subsystems. The agent is not yet started — call [`start()`](Self::start) to begin
    /// processing events.
    pub async fn new(id: AgentId, config: MeshTransportConfig) -> Result<Self> {
        let transport = MeshTransport::new(config).await?;
        let state_sync = Box::new(DefaultStateSync::new(id));

        // Create channel for fault tolerance events
        let (fault_tx, fault_rx) = mpsc::unbounded_channel();
        let integration = IntegrationAdapter::new(transport, state_sync, Some(fault_tx));

        // Start fault tolerance manager in background
        let fault_manager = FaultToleranceManager::new(fault_rx);
        let fault_handle = tokio::spawn(async move {
            fault_manager.run().await;
        });

        info!(agent = ?id, "Agent created successfully");

        Ok(Self {
            id,
            integration: Some(integration),
            task_handle: None,
            fault_handle: Some(fault_handle),
            started: false,
            #[cfg(feature = "iot")]
            device_registry: DeviceRegistry::new(),
        })
    }

    /// Start the agent (non‑blocking).
    ///
    /// Spawns the main integration event loop as a background task. Returns an error
    /// if the agent has already been started or if the integration has been consumed.
    pub fn start(&mut self) -> Result<()> {
        if self.started {
            warn!(agent = ?self.id, "Agent already started, ignoring duplicate start");
            return Ok(());
        }

        let integration = self.integration.take().ok_or_else(|| {
            SdkError::Internal("Integration already consumed".to_string())
        })?;

        let handle = tokio::spawn(async move {
            let result = integration.run().await;
            if let Err(e) = &result {
                error!("Agent integration loop failed: {}", e);
            }
            result
        });

        self.task_handle = Some(handle);
        self.started = true;
        info!(agent = ?self.id, "Agent started");
        Ok(())
    }

    /// Stop the agent gracefully.
    ///
    /// Aborts the main event loop and fault tolerance manager, then waits for
    /// both tasks to complete. This method is idempotent — calling it multiple
    /// times has no additional effect.
    pub async fn stop(&mut self) -> Result<()> {
        if !self.started && self.task_handle.is_none() && self.fault_handle.is_none() {
            warn!(agent = ?self.id, "Agent already stopped, ignoring duplicate stop");
            return Ok(());
        }

        // Abort and await the main integration task
        if let Some(handle) = self.task_handle.take() {
            if !handle.is_finished() {
                handle.abort();
            }
            let _ = handle.await;
            info!(agent = ?self.id, "Integration task stopped");
        }

        // Abort and await the fault tolerance task
        if let Some(fault_handle) = self.fault_handle.take() {
            if !fault_handle.is_finished() {
                fault_handle.abort();
            }
            let _ = fault_handle.await;
            info!(agent = ?self.id, "Fault tolerance task stopped");
        }

        self.started = false;
        info!(agent = ?self.id, "Agent stopped gracefully");
        Ok(())
    }

    /// Check if the agent is currently running.
    pub fn is_running(&self) -> bool {
        self.started
    }

    /// Get the agent's ID.
    pub fn id(&self) -> AgentId {
        self.id
    }

    /// Set a key‑value pair in the agent's CRDT map.
    ///
    /// The value is serialized and stored locally. Call [`broadcast_changes()`](Self::broadcast_changes)
    /// to propagate the change to peers.
    pub fn set_value<V: serde::Serialize>(&mut self, key: &str, value: V) -> Result<()> {
        self.integration.as_mut()
            .ok_or_else(|| SdkError::Internal("Agent not initialized".to_string()))?
            .set_value(key, value)
    }

    /// Get a value from the agent's CRDT map.
    pub fn get_value<V: for<'de> serde::Deserialize<'de>>(&self, key: &str) -> Option<V> {
        self.integration.as_ref()?.get_value(key)
    }

    /// Broadcast local changes to all connected peers.
    pub async fn broadcast_changes(&mut self) -> Result<()> {
        self.integration.as_mut()
            .ok_or_else(|| SdkError::Internal("Agent not initialized".to_string()))?
            .broadcast_changes().await
    }

    /// Get a reference to the integration adapter (if available).
    pub fn integration(&self) -> Option<&IntegrationAdapter> {
        self.integration.as_ref()
    }

    /// Get a mutable reference to the integration adapter (if available).
    pub fn integration_mut(&mut self) -> Option<&mut IntegrationAdapter> {
        self.integration.as_mut()
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

impl Drop for Agent {
    fn drop(&mut self) {
        if self.started || self.task_handle.is_some() || self.fault_handle.is_some() {
            warn!(agent = ?self.id, "Agent dropped without calling stop() — tasks may leak");
            // Attempt to abort tasks synchronously (best-effort cleanup)
            if let Some(handle) = self.task_handle.take() {
                if !handle.is_finished() {
                    handle.abort();
                }
            }
            if let Some(fault_handle) = self.fault_handle.take() {
                if !fault_handle.is_finished() {
                    fault_handle.abort();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mesh_transport::MeshTransportConfig;

    #[tokio::test]
    async fn test_agent_creation() {
        let agent = Agent::new(1, MeshTransportConfig::in_memory()).await.unwrap();
        assert_eq!(agent.id(), 1);
        assert!(!agent.is_running());
    }

    #[tokio::test]
    async fn test_agent_start_stop() {
        let mut agent = Agent::new(1, MeshTransportConfig::in_memory()).await.unwrap();
        agent.start().unwrap();
        assert!(agent.is_running());
        agent.stop().await.unwrap();
        assert!(!agent.is_running());
    }

    #[tokio::test]
    async fn test_agent_duplicate_start() {
        let mut agent = Agent::new(1, MeshTransportConfig::in_memory()).await.unwrap();
        agent.start().unwrap();
        // Second start should be a no-op, not an error
        agent.start().unwrap();
        agent.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_agent_duplicate_stop() {
        let mut agent = Agent::new(1, MeshTransportConfig::in_memory()).await.unwrap();
        agent.start().unwrap();
        agent.stop().await.unwrap();
        // Second stop should be a no-op, not an error
        agent.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_agent_set_get_value() {
        let mut agent = Agent::new(1, MeshTransportConfig::in_memory()).await.unwrap();
        agent.set_value("test_key", "test_value").unwrap();
        let value: Option<String> = agent.get_value("test_key");
        assert_eq!(value, Some("test_value".to_string()));
    }

    #[tokio::test]
    async fn test_agent_stop_before_start() {
        let mut agent = Agent::new(1, MeshTransportConfig::in_memory()).await.unwrap();
        // Stopping before starting should be a no-op
        agent.stop().await.unwrap();
        assert!(!agent.is_running());
    }

    #[tokio::test]
    async fn test_agent_multiple_stops() {
        let mut agent = Agent::new(1, MeshTransportConfig::in_memory()).await.unwrap();
        agent.start().unwrap();
        agent.stop().await.unwrap();
        agent.stop().await.unwrap();
        agent.stop().await.unwrap();
        assert!(!agent.is_running());
    }

    #[tokio::test]
    async fn test_agent_get_value_before_set() {
        let agent = Agent::new(1, MeshTransportConfig::in_memory()).await.unwrap();
        let value: Option<String> = agent.get_value("nonexistent");
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_agent_overwrite_value() {
        let mut agent = Agent::new(1, MeshTransportConfig::in_memory()).await.unwrap();
        agent.set_value("key", "value1").unwrap();
        agent.set_value("key", "value2").unwrap();
        let value: Option<String> = agent.get_value("key");
        assert_eq!(value, Some("value2".to_string()));
    }
}