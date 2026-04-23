//! Stream processing for real-time data processing.
//!
//! Provides:
//! - Stream sources and sinks
//! - Stream operators (map, filter, reduce, window)
//! - Stream processors
//! - Event time processing
//! - Watermarking

pub mod stream;
pub mod operators;
pub mod processor;
pub mod watermark;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::info;

pub use stream::*;
pub use operators::*;
pub use processor::*;
pub use watermark::*;

/// Stream processing configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamProcessingConfig {
    pub buffer_size: usize,
    pub parallelism: usize,
    pub checkpoint_interval_secs: u64,
    pub enable_watermarks: bool,
    pub late_event_handling: LateEventHandling,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LateEventHandling {
    Drop,
    Process,
    SideOutput,
}

impl Default for StreamProcessingConfig {
    fn default() -> Self {
        Self {
            buffer_size: 10000,
            parallelism: 4,
            checkpoint_interval_secs: 60,
            enable_watermarks: true,
            late_event_handling: LateEventHandling::Process,
        }
    }
}

/// Stream processing manager.
pub struct StreamProcessingManager {
    config: StreamProcessingConfig,
    streams: RwLock<HashMap<String, Box<dyn StreamBase>>>,
    processors: RwLock<HashMap<String, StreamProcessor>>,
}

impl StreamProcessingManager {
    /// Create new stream processing manager.
    pub fn new(config: StreamProcessingConfig) -> Self {
        Self {
            config,
            streams: RwLock::new(HashMap::new()),
            processors: RwLock::new(HashMap::new()),
        }
    }

    /// Initialize stream processing.
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing stream processing with parallelism: {}", self.config.parallelism);
        Ok(())
    }

    /// Create stream from source.
    pub async fn create_stream<S>(&self, name: &str, source: S) -> Result<StreamId>
    where
        S: StreamSource + 'static,
    {
        let stream_id = StreamId::new(name);
        let stream = Box::new(DataStream::new(stream_id.clone(), source));

        self.streams.write().await.insert(name.to_string(), stream);
        info!("Stream created: {}", name);

        Ok(stream_id)
    }

    /// Add operator to stream.
    pub async fn add_operator<O>(&self, stream_id: &str, operator: O) -> Result<()>
    where
        O: StreamOperator + 'static,
    {
        let mut streams = self.streams.write().await;
        
        let stream = streams.get_mut(stream_id)
            .ok_or_else(|| anyhow::anyhow!("Stream not found: {}", stream_id))?;

        stream.add_operator(Box::new(operator)).await?;
        info!("Operator added to stream: {}", stream_id);

        Ok(())
    }

    /// Add processor to stream.
    pub async fn add_processor(&self, name: &str, processor: StreamProcessor) -> Result<()> {
        self.processors.write().await.insert(name.to_string(), processor);
        info!("Processor registered: {}", name);
        Ok(())
    }

    /// Start processing.
    pub async fn start_processing(&self, stream_id: &str) -> Result<()> {
        let streams = self.streams.read().await;
        
        let stream = streams.get(stream_id)
            .ok_or_else(|| anyhow::anyhow!("Stream not found: {}", stream_id))?;

        stream.start().await?;
        info!("Processing started for stream: {}", stream_id);

        Ok(())
    }

    /// Stop processing.
    pub async fn stop_processing(&self, stream_id: &str) -> Result<()> {
        let mut streams = self.streams.write().await;
        
        let stream = streams.get_mut(stream_id)
            .ok_or_else(|| anyhow::anyhow!("Stream not found: {}", stream_id))?;

        stream.stop().await?;
        info!("Processing stopped for stream: {}", stream_id);

        Ok(())
    }

    /// Get stream statistics.
    pub async fn get_stats(&self, stream_id: &str) -> Result<StreamStats> {
        let streams = self.streams.read().await;
        
        let stream = streams.get(stream_id)
            .ok_or_else(|| anyhow::anyhow!("Stream not found: {}", stream_id))?;

        Ok(stream.get_stats().await)
    }

    /// Get all stream statistics.
    pub async fn get_all_stats(&self) -> HashMap<String, StreamStats> {
        let streams = self.streams.read().await;
        let mut stats = HashMap::new();

        for (name, stream) in streams.iter() {
            stats.insert(name.clone(), stream.get_stats().await);
        }

        stats
    }

    /// Create checkpoint.
    pub async fn create_checkpoint(&self, stream_id: &str) -> Result<Checkpoint> {
        let streams = self.streams.read().await;
        
        let stream = streams.get(stream_id)
            .ok_or_else(|| anyhow::anyhow!("Stream not found: {}", stream_id))?;

        let checkpoint = stream.create_checkpoint().await?;
        info!("Checkpoint created for stream: {}", stream_id);

        Ok(checkpoint)
    }

    /// Restore from checkpoint.
    pub async fn restore_checkpoint(&self, stream_id: &str, checkpoint: &Checkpoint) -> Result<()> {
        let mut streams = self.streams.write().await;
        
        let stream = streams.get_mut(stream_id)
            .ok_or_else(|| anyhow::anyhow!("Stream not found: {}", stream_id))?;

        stream.restore_checkpoint(checkpoint).await?;
        info!("Checkpoint restored for stream: {}", stream_id);

        Ok(())
    }
}

/// Stream identifier.
#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct StreamId {
    pub name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl StreamId {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            created_at: chrono::Utc::now(),
        }
    }
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

impl Checkpoint {
    pub fn new(stream_id: &str, state: serde_json::Value, offset: i64) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            stream_id: stream_id.to_string(),
            timestamp: chrono::Utc::now(),
            state,
            offset,
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_stream_processing() {
        let config = StreamProcessingConfig::default();
        let manager = StreamProcessingManager::new(config);

        // Initialize
        manager.initialize().await.unwrap();

        // Create stream (mock source)
        let source = MockSource::new();
        let stream_id = manager.create_stream("test-stream", source).await.unwrap();

        // Get stats
        let stats = manager.get_stats(&stream_id.name).await.unwrap();
        assert_eq!(stats.events_processed, 0);
    }
}

/// Mock stream source for testing.
pub struct MockSource {
    count: std::sync::atomic::AtomicI64,
}

impl MockSource {
    pub fn new() -> Self {
        Self {
            count: std::sync::atomic::AtomicI64::new(0),
        }
    }
}

impl Default for MockSource {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl StreamSource for MockSource {
    type Item = StreamEvent;

    async fn next(&self) -> Option<Self::Item> {
        let count = self.count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        
        Some(StreamEvent {
            id: uuid::Uuid::new_v4().to_string(),
            data: serde_json::json!({"count": count}),
            timestamp: chrono::Utc::now(),
            event_time: chrono::Utc::now(),
        })
    }
}
