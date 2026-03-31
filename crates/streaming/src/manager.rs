//! Stream manager.

use crate::channel::{Publisher, Subscriber, StreamMessage, Topic};
use crate::error::Error;
use crate::qos::QualityOfService;
use dashmap::DashMap;
use mesh_transport::Transport;
use std::sync::Arc;
use tokio::sync::broadcast;

/// Manages streaming channels and integrates with mesh transport.
pub struct StreamManager {
    /// Local broadcast channels per topic.
    channels: Arc<DashMap<Topic, broadcast::Sender<StreamMessage>>>,
    /// Mesh transport for cross‑agent streaming.
    transport: Arc<dyn Transport + Send + Sync>,
}

impl StreamManager {
    /// Create a new stream manager.
    pub fn new(transport: Arc<dyn Transport + Send + Sync>) -> Self {
        Self {
            channels: Arc::new(DashMap::new()),
            transport,
        }
    }

    /// Create or get a publisher for a topic.
    pub fn publisher(&self, topic: Topic, qos: QualityOfService) -> Publisher {
        let tx = self
            .channels
            .entry(topic.clone())
            .or_insert_with(|| broadcast::channel(1024).0)
            .clone();
        Publisher::new(topic, tx, qos)
    }

    /// Subscribe to a topic.
    pub fn subscriber(&self, topic: Topic) -> Result<Subscriber, Error> {
        let tx = self
            .channels
            .get(&topic)
            .ok_or_else(|| Error::Subscription(format!("Topic '{}' not found", topic)))?;
        let rx = tx.subscribe();
        Ok(Subscriber::new(topic, rx))
    }

    /// Advertise a topic to the mesh (so other agents can discover it).
    pub async fn advertise(&self, topic: &str) -> Result<(), Error> {
        // In a real implementation, we would send a discovery message.
        Ok(())
    }

    /// Forward a message to remote agents via mesh transport.
    pub async fn forward(&self, msg: StreamMessage) -> Result<(), Error> {
        // Encode and send via transport.
        let payload = serde_cbor::to_vec(&msg).map_err(|e| Error::Codec(e.to_string()))?;
        self.transport
            .broadcast(&payload)
            .await
            .map_err(Error::Transport)?;
        Ok(())
    }
}