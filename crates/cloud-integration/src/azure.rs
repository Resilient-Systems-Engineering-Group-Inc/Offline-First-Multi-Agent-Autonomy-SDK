//! Azure IoT Hub integration.

use super::{CloudAdapter, CloudError, CloudConfig};
use common::types::{AgentId, MeshMessage};
use mesh_transport::TransportEvent;
use async_trait::async_trait;

/// Azure IoT Hub adapter.
#[cfg(feature = "azure")]
pub struct AzureIotHubAdapter {
    config: CloudConfig,
    // Azure SDK clients would go here.
}

#[cfg(feature = "azure")]
#[async_trait]
impl CloudAdapter for AzureIotHubAdapter {
    async fn connect(&mut self) -> Result<(), CloudError> {
        tracing::info!("Connecting to Azure IoT Hub");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), CloudError> {
        Ok(())
    }

    async fn send(&mut self, message: MeshMessage) -> Result<(), CloudError> {
        tracing::debug!("Sending message to Azure IoT Hub: {:?}", message);
        Ok(())
    }

    async fn recv(&mut self) -> Option<TransportEvent> {
        // Poll Azure IoT Hub events.
        None
    }

    async fn subscribe(&mut self, topic: &str) -> Result<(), CloudError> {
        tracing::info!("Subscribing to Azure IoT Hub topic: {}", topic);
        Ok(())
    }

    async fn unsubscribe(&mut self, topic: &str) -> Result<(), CloudError> {
        tracing::info!("Unsubscribing from Azure IoT Hub topic: {}", topic);
        Ok(())
    }
}

#[cfg(not(feature = "azure"))]
pub struct AzureIotHubAdapter;

#[cfg(not(feature = "azure"))]
#[async_trait]
impl CloudAdapter for AzureIotHubAdapter {
    async fn connect(&mut self) -> Result<(), CloudError> {
        Err(CloudError::Config("Azure feature not enabled".into()))
    }

    async fn disconnect(&mut self) -> Result<(), CloudError> {
        Err(CloudError::Config("Azure feature not enabled".into()))
    }

    async fn send(&mut self, _message: MeshMessage) -> Result<(), CloudError> {
        Err(CloudError::Config("Azure feature not enabled".into()))
    }

    async fn recv(&mut self) -> Option<TransportEvent> {
        None
    }

    async fn subscribe(&mut self, _topic: &str) -> Result<(), CloudError> {
        Err(CloudError::Config("Azure feature not enabled".into()))
    }

    async fn unsubscribe(&mut self, _topic: &str) -> Result<(), CloudError> {
        Err(CloudError::Config("Azure feature not enabled".into()))
    }
}