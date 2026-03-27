//! Cloud service integration (AWS IoT, Azure IoT) for hybrid deployments.
//!
//! This crate provides adapters that bridge the mesh transport with cloud‑based
//! message brokers, enabling hybrid offline‑first / cloud‑connected scenarios.

pub mod aws;
pub mod azure;
pub mod mqtt;
pub mod adapter;

use async_trait::async_trait;
use common::types::{AgentId, MeshMessage};
use mesh_transport::TransportEvent;
use thiserror::Error;

/// Errors that can occur during cloud integration.
#[derive(Error, Debug)]
pub enum CloudError {
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("Network error: {0}")]
    Network(#[from] std::io::Error),
    #[error("Authentication error: {0}")]
    Auth(String),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Cloud service error: {0}")]
    Service(String),
}

/// A cloud adapter that can send and receive messages to/from a cloud broker.
#[async_trait]
pub trait CloudAdapter: Send + Sync {
    /// Connect to the cloud service.
    async fn connect(&mut self) -> Result<(), CloudError>;

    /// Disconnect from the cloud service.
    async fn disconnect(&mut self) -> Result<(), CloudError>;

    /// Send a mesh message to the cloud.
    async fn send(&mut self, message: MeshMessage) -> Result<(), CloudError>;

    /// Receive events from the cloud (non‑blocking).
    /// Returns `None` if no event is available.
    async fn recv(&mut self) -> Option<TransportEvent>;

    /// Subscribe to a topic (or its cloud equivalent).
    async fn subscribe(&mut self, topic: &str) -> Result<(), CloudError>;

    /// Unsubscribe from a topic.
    async fn unsubscribe(&mut self, topic: &str) -> Result<(), CloudError>;
}

/// Configuration for cloud integration.
#[derive(Debug, Clone)]
pub struct CloudConfig {
    /// Cloud provider (AWS, Azure, generic MQTT).
    pub provider: CloudProvider,
    /// Endpoint URL.
    pub endpoint: String,
    /// Authentication credentials (path to certificate, key, etc.)
    pub credentials: Option<String>,
    /// Client ID (usually the agent ID).
    pub client_id: String,
    /// Quality of service level (0,1,2).
    pub qos: u8,
}

#[derive(Debug, Clone)]
pub enum CloudProvider {
    AwsIot,
    AzureIotHub,
    GenericMqtt,
}

/// A bridge that forwards messages between mesh transport and cloud.
pub struct CloudBridge<C: CloudAdapter> {
    adapter: C,
    /// Local agent ID.
    local_agent: AgentId,
    /// Topics to forward.
    forward_topics: Vec<String>,
}

impl<C: CloudAdapter> CloudBridge<C> {
    pub fn new(adapter: C, local_agent: AgentId) -> Self {
        Self {
            adapter,
            local_agent,
            forward_topics: Vec::new(),
        }
    }

    pub async fn start(&mut self) -> Result<(), CloudError> {
        self.adapter.connect().await?;
        for topic in &self.forward_topics {
            self.adapter.subscribe(topic).await?;
        }
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<(), CloudError> {
        self.adapter.disconnect().await
    }

    /// Forward a local mesh message to the cloud.
    pub async fn forward_to_cloud(&mut self, message: MeshMessage) -> Result<(), CloudError> {
        self.adapter.send(message).await
    }

    /// Poll for incoming cloud messages and convert to transport events.
    pub async fn poll(&mut self) -> Option<TransportEvent> {
        self.adapter.recv().await
    }
}