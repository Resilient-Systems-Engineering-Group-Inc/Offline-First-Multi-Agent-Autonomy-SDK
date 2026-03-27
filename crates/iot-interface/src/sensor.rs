//! Sensor abstraction.

use crate::error::{Error, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// A reading from a sensor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorReading {
    /// Timestamp (Unix epoch in milliseconds).
    pub timestamp: u64,
    /// Value as a JSON‑serializable type.
    pub value: serde_json::Value,
    /// Optional metadata (unit, accuracy, etc.)
    pub metadata: Option<serde_json::Value>,
}

/// Configuration for a sensor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorConfig {
    /// Unique identifier for the sensor.
    pub id: String,
    /// Polling interval (if applicable).
    pub poll_interval_ms: Option<u64>,
    /// Protocol‑specific configuration (JSON).
    pub protocol_config: serde_json::Value,
}

/// Trait for any sensor.
#[async_trait]
pub trait Sensor: Send + Sync {
    /// Read the current value from the sensor.
    async fn read(&self) -> Result<SensorReading>;

    /// Get the sensor's configuration.
    fn config(&self) -> &SensorConfig;

    /// Start continuous polling (if supported).
    async fn start_polling<F>(&self, callback: F) -> Result<()>
    where
        F: Fn(SensorReading) + Send + Sync + 'static;

    /// Stop polling.
    async fn stop_polling(&self) -> Result<()>;
}

/// A dummy sensor for testing.
pub struct DummySensor {
    config: SensorConfig,
    value: f64,
}

impl DummySensor {
    pub fn new(id: String, initial_value: f64) -> Self {
        Self {
            config: SensorConfig {
                id,
                poll_interval_ms: Some(1000),
                protocol_config: serde_json::json!({}),
            },
            value: initial_value,
        }
    }
}

#[async_trait]
impl Sensor for DummySensor {
    async fn read(&self) -> Result<SensorReading> {
        Ok(SensorReading {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            value: serde_json::json!(self.value),
            metadata: Some(serde_json::json!({"unit": "test"})),
        })
    }

    fn config(&self) -> &SensorConfig {
        &self.config
    }

    async fn start_polling<F>(&self, _callback: F) -> Result<()>
    where
        F: Fn(SensorReading) + Send + Sync + 'static,
    {
        // Not implemented for dummy
        Ok(())
    }

    async fn stop_polling(&self) -> Result<()> {
        Ok(())
    }
}