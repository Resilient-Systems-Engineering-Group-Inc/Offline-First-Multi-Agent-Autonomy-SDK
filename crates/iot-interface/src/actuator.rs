//! Actuator abstraction.

use crate::error::{Error, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// A command to be sent to an actuator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActuatorCommand {
    /// Command name (e.g., "set_position", "turn_on").
    pub command: String,
    /// Parameters as JSON.
    pub parameters: serde_json::Value,
    /// Optional priority (higher = more urgent).
    pub priority: Option<u8>,
}

/// Configuration for an actuator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActuatorConfig {
    /// Unique identifier for the actuator.
    pub id: String,
    /// Protocol‑specific configuration (JSON).
    pub protocol_config: serde_json::Value,
    /// Default timeout for commands (milliseconds).
    pub default_timeout_ms: u64,
}

/// Trait for any actuator.
#[async_trait]
pub trait Actuator: Send + Sync {
    /// Execute a command on the actuator.
    async fn execute(&self, command: ActuatorCommand) -> Result<()>;

    /// Get the actuator's configuration.
    fn config(&self) -> &ActuatorConfig;

    /// Get the current status of the actuator (if supported).
    async fn status(&self) -> Result<serde_json::Value>;
}

/// A dummy actuator for testing.
pub struct DummyActuator {
    config: ActuatorConfig,
    state: serde_json::Value,
}

impl DummyActuator {
    pub fn new(id: String) -> Self {
        Self {
            config: ActuatorConfig {
                id,
                protocol_config: serde_json::json!({}),
                default_timeout_ms: 5000,
            },
            state: serde_json::json!("idle"),
        }
    }
}

#[async_trait]
impl Actuator for DummyActuator {
    async fn execute(&self, command: ActuatorCommand) -> Result<()> {
        tracing::info!("Dummy actuator executing command: {:?}", command);
        // Simulate some work
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        Ok(())
    }

    fn config(&self) -> &ActuatorConfig {
        &self.config
    }

    async fn status(&self) -> Result<serde_json::Value> {
        Ok(self.state.clone())
    }
}