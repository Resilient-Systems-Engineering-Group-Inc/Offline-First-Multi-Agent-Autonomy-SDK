//! Generic adapter implementations.

use super::{CloudAdapter, CloudError, CloudConfig};
use common::types::{AgentId, MeshMessage};
use mesh_transport::TransportEvent;
use async_trait::async_trait;

/// A dummy adapter for testing.
pub struct DummyCloudAdapter;

#[async_trait]
impl CloudAdapter for DummyCloudAdapter {
    async fn connect(&mut self) -> Result<(), CloudError> {
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), CloudError> {
        Ok(())
    }

    async fn send(&mut self, _message: MeshMessage) -> Result<(), CloudError> {
        Ok(())
    }

    async fn recv(&mut self) -> Option<TransportEvent> {
        None
    }

    async fn subscribe(&mut self, _topic: &str) -> Result<(), CloudError> {
        Ok(())
    }

    async fn unsubscribe(&mut self, _topic: &str) -> Result<(), CloudError> {
        Ok(())
    }
}