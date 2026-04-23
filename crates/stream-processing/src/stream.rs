//! Stream abstractions.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

/// Stream event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEvent {
    pub id: String,
    pub data: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub event_time: chrono::DateTime<chrono::Utc>,
}

/// Stream source trait.
#[async_trait::async_trait]
pub trait StreamSource: Send + Sync {
    type Item: Send + Sync;

    async fn next(&self) -> Option<Self::Item>;
}

/// Stream sink trait.
#[async_trait::async_trait]
pub trait StreamSink: Send + Sync {
    async fn write(&self, event: StreamEvent) -> Result<()>;
    async fn flush(&self) -> Result<()>;
}

/// Stream operator trait.
#[async_trait::async_trait]
pub trait StreamOperator: Send + Sync {
    fn name(&self) -> &str;
    async fn process(&self, event: StreamEvent) -> Result<Option<StreamEvent>>;
}

/// Base stream trait.
#[async_trait::async_trait]
pub trait StreamBase: Send + Sync {
    async fn start(&self) -> Result<()>;
    async fn stop(&self) -> Result<()>;
    async fn add_operator(&mut self, operator: Box<dyn StreamOperator>) -> Result<()>;
    async fn get_stats(&self) -> StreamStats;
    async fn create_checkpoint(&self) -> Result<Checkpoint>;
    async fn restore_checkpoint(&self, checkpoint: &Checkpoint) -> Result<()>;
}

/// Stream statistics.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StreamStats {
    pub events_processed: i64,
    pub events_per_second: f64,
    pub avg_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub errors: i64,
    pub watermarks_processed: i64,
    pub late_events: i64,
}

/// Checkpoint for state recovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: String,
    pub stream_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub state: serde_json::Value,
    pub offset: i64,
}

/// Data stream implementation.
pub struct DataStream<S: StreamSource> {
    id: crate::StreamId,
    source: S,
    operators: RwLock<Vec<Box<dyn StreamOperator>>>,
    running: RwLock<bool>,
    stats: RwLock<StreamStats>,
}

impl<S: StreamSource> DataStream<S> {
    pub fn new(id: crate::StreamId, source: S) -> Self {
        Self {
            id,
            source,
            operators: RwLock::new(Vec::new()),
            running: RwLock::new(false),
            stats: RwLock::new(StreamStats::default()),
        }
    }

    pub async fn process_event(&self, event: StreamEvent) -> Result<Option<StreamEvent>> {
        let operators = self.operators.read().await;
        let mut current_event = Some(event);

        for operator in operators.iter() {
            if let Some(evt) = current_event {
                current_event = operator.process(evt).await?;
            } else {
                break;
            }
        }

        Ok(current_event)
    }
}

#[async_trait::async_trait]
impl<S: StreamSource + 'static> StreamBase for DataStream<S> {
    async fn start(&self) -> Result<()> {
        *self.running.write().await = true;
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        *self.running.write().await = false;
        Ok(())
    }

    async fn add_operator(&mut self, operator: Box<dyn StreamOperator>) -> Result<()> {
        self.operators.write().await.push(operator);
        Ok(())
    }

    async fn get_stats(&self) -> StreamStats {
        self.stats.read().await.clone()
    }

    async fn create_checkpoint(&self) -> Result<Checkpoint> {
        let stats = self.stats.read().await;
        let state = serde_json::to_value(&*stats)?;

        Ok(Checkpoint {
            id: uuid::Uuid::new_v4().to_string(),
            stream_id: self.id.name.clone(),
            timestamp: chrono::Utc::now(),
            state,
            offset: stats.events_processed,
        })
    }

    async fn restore_checkpoint(&self, checkpoint: &Checkpoint) -> Result<()> {
        let stats: StreamStats = serde_json::from_value(checkpoint.state.clone())?;
        *self.stats.write().await = stats;
        Ok(())
    }
}

/// Console sink for debugging.
pub struct ConsoleSink;

#[async_trait::async_trait]
impl StreamSink for ConsoleSink {
    async fn write(&self, event: StreamEvent) -> Result<()> {
        println!("Event: {:?}", event);
        Ok(())
    }

    async fn flush(&self) -> Result<()> {
        Ok(())
    }
}

/// Memory sink for testing.
pub struct MemorySink {
    events: RwLock<Vec<StreamEvent>>,
}

impl MemorySink {
    pub fn new() -> Self {
        Self {
            events: RwLock::new(Vec::new()),
        }
    }

    pub async fn get_events(&self) -> Vec<StreamEvent> {
        self.events.read().await.clone()
    }

    pub async fn clear(&self) {
        self.events.write().await.clear();
    }
}

impl Default for MemorySink {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl StreamSink for MemorySink {
    async fn write(&self, event: StreamEvent) -> Result<()> {
        self.events.write().await.push(event);
        Ok(())
    }

    async fn flush(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_sink() {
        let sink = MemorySink::new();
        
        let event = StreamEvent {
            id: "test-1".to_string(),
            data: serde_json::json!({"test": true}),
            timestamp: chrono::Utc::now(),
            event_time: chrono::Utc::now(),
        };

        sink.write(event.clone()).await.unwrap();
        
        let events = sink.get_events().await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, "test-1");
    }
}
