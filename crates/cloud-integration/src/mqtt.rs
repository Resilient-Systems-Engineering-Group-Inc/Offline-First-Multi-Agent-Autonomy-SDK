//! Generic MQTT broker integration.

use super::{CloudAdapter, CloudError, CloudConfig};
use common::types::{AgentId, MeshMessage};
use mesh_transport::TransportEvent;
use async_trait::async_trait;

/// Generic MQTT adapter (using rumqttc).
#[cfg(feature = "mqtt")]
pub struct MqttAdapter {
    config: CloudConfig,
    // rumqttc client would go here.
}

#[cfg(feature = "mqtt")]
#[async_trait]
impl CloudAdapter for MqttAdapter {
    async fn connect(&mut self) -> Result<(), CloudError> {
        tracing::info!("Connecting to MQTT broker at {}", self.config.endpoint);
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), CloudError> {
        Ok(())
    }

    async fn send(&mut self, message: MeshMessage) -> Result<(), CloudError> {
        tracing::debug!("Publishing to MQTT: {:?}", message);
        Ok(())
    }

    async fn recv(&mut self) -> Option<TransportEvent> {
        // Poll MQTT messages.
        None
    }

    async fn subscribe(&mut self, topic: &str) -> Result<(), CloudError> {
        tracing::info!("Subscribing to MQTT topic: {}", topic);
        Ok(())
    }

    async fn unsubscribe(&mut self, topic: &str) -> Result<(), CloudError> {
        tracing::info!("Unsubscribing from MQTT topic: {}", topic);
        Ok(())
    }
}

#[cfg(not(feature = "mqtt"))]
pub struct MqttAdapter;

#[cfg(not(feature = "mqtt"))]
#[async_trait]
impl CloudAdapter for MqttAdapter {
    async fn connect(&mut self) -> Result<(), CloudError> {
        Err(CloudError::Config("MQTT feature not enabled".into()))
    }

    async fn disconnect(&mut self) -> Result<(), CloudError> {
        Err(CloudError::Config("MQTT feature not enabled".into()))
    }

    async fn send(&mut self, _message: MeshMessage) -> Result<(), CloudError> {
        Err(CloudError::Config("MQTT feature not enabled".into()))
    }

    async fn recv(&mut self) -> Option<TransportEvent> {
        None
    }

    async fn subscribe(&mut self, _topic: &str) -> Result<(), CloudError> {
        Err(CloudError::Config("MQTT feature not enabled".into()))
    }

    async fn unsubscribe(&mut self, _topic: &str) -> Result<(), CloudError> {
        Err(CloudError::Config("MQTT feature not enabled".into()))
    }
}