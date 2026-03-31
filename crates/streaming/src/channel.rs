//! Publish‑subscribe channels.

use crate::error::Error;
use crate::qos::{QoS, QualityOfService};
use bytes::Bytes;
use futures::stream::BoxStream;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::RwLock;

/// A topic identifier.
pub type Topic = String;

/// A message in a stream.
#[derive(Debug, Clone)]
pub struct StreamMessage {
    /// Topic.
    pub topic: Topic,
    /// Payload.
    pub payload: Bytes,
    /// QoS.
    pub qos: QoS,
    /// Timestamp (milliseconds since epoch).
    pub timestamp: u64,
}

/// Publisher for a topic.
pub struct Publisher {
    topic: Topic,
    tx: broadcast::Sender<StreamMessage>,
    qos: QualityOfService,
}

impl Publisher {
    /// Create a new publisher.
    pub fn new(topic: Topic, tx: broadcast::Sender<StreamMessage>, qos: QualityOfService) -> Self {
        Self { topic, tx, qos }
    }

    /// Publish a message.
    pub async fn publish(&self, payload: Bytes) -> Result<(), Error> {
        let msg = StreamMessage {
            topic: self.topic.clone(),
            payload,
            qos: self.qos.level,
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
        };
        self.tx.send(msg).map_err(|_| Error::ChannelClosed)?;
        Ok(())
    }

    /// Get the topic.
    pub fn topic(&self) -> &str {
        &self.topic
    }
}

/// Subscriber for a topic.
pub struct Subscriber {
    topic: Topic,
    rx: broadcast::Receiver<StreamMessage>,
}

impl Subscriber {
    /// Create a new subscriber.
    pub fn new(topic: Topic, rx: broadcast::Receiver<StreamMessage>) -> Self {
        Self { topic, rx }
    }

    /// Receive the next message.
    pub async fn recv(&mut self) -> Result<StreamMessage, Error> {
        self.rx.recv().await.map_err(|_| Error::ChannelClosed)
    }

    /// Convert into a stream.
    pub fn into_stream(self) -> BoxStream<'static, Result<StreamMessage, Error>> {
        Box::pin(tokio_stream::wrappers::BroadcastStream::new(self.rx).filter_map(|res| {
            async move {
                match res {
                    Ok(msg) => Some(Ok(msg)),
                    Err(broadcast::error::RecvError::Closed) => Some(Err(Error::ChannelClosed)),
                    Err(broadcast::error::RecvError::Lagged(_)) => None, // skip lagged
                }
            }
        }))
    }

    /// Get the topic.
    pub fn topic(&self) -> &str {
        &self.topic
    }
}