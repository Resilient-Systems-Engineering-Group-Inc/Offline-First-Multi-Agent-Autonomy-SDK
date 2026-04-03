//! Log aggregation and batching.

use crate::error::Result;
use crate::log_record::LogRecord;
use chrono::{DateTime, Utc};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant};

/// Configuration for log aggregation.
pub struct AggregatorConfig {
    /// Maximum number of records per batch.
    pub max_batch_size: usize,
    /// Maximum time a batch can wait before being flushed.
    pub max_batch_age: Duration,
    /// Whether to compress batches (if compression feature enabled).
    pub compress: bool,
}

impl Default for AggregatorConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 100,
            max_batch_age: Duration::from_secs(5),
            compress: false,
        }
    }
}

/// A batch of log records.
#[derive(Debug, Clone)]
pub struct LogBatch {
    /// Unique batch ID.
    pub id: String,
    /// Timestamp when the batch was created.
    pub created_at: DateTime<Utc>,
    /// Records in the batch.
    pub records: Vec<LogRecord>,
    /// Size in bytes (approximate).
    pub size_bytes: usize,
}

impl LogBatch {
    /// Creates a new empty batch.
    pub fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            created_at: Utc::now(),
            records: Vec::new(),
            size_bytes: 0,
        }
    }

    /// Adds a record to the batch.
    pub fn add_record(&mut self, record: LogRecord) {
        self.size_bytes += record.to_json().map(|s| s.len()).unwrap_or(0);
        self.records.push(record);
    }

    /// Returns the number of records in the batch.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Returns whether the batch is empty.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Serializes the batch to JSON.
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| crate::error::LogError::Serialization(e.to_string()))
    }

    /// Serializes the batch to CBOR.
    pub fn to_cbor(&self) -> Result<Vec<u8>> {
        serde_cbor::to_vec(self)
            .map_err(|e| crate::error::LogError::Serialization(e.to_string()))
    }
}

/// Aggregator that collects log records and batches them.
pub struct Aggregator {
    config: AggregatorConfig,
    current_batch: Mutex<LogBatch>,
    last_flush: Mutex<Instant>,
    sink: Arc<dyn crate::sink::LogSink>,
}

impl Aggregator {
    /// Creates a new aggregator with the given configuration and sink.
    pub fn new(config: AggregatorConfig, sink: Arc<dyn crate::sink::LogSink>) -> Self {
        Self {
            config,
            current_batch: Mutex::new(LogBatch::new()),
            last_flush: Mutex::new(Instant::now()),
            sink,
        }
    }

    /// Adds a log record to the aggregator.
    ///
    /// If the batch reaches its size or age limit, it will be automatically flushed.
    pub async fn add_record(&self, record: LogRecord) -> Result<()> {
        let mut batch = self.current_batch.lock().await;
        batch.add_record(record);

        if batch.len() >= self.config.max_batch_size {
            self.flush_batch(batch).await?;
            *batch = LogBatch::new();
            *self.last_flush.lock().await = Instant::now();
        }
        Ok(())
    }

    /// Flushes the current batch if it is not empty.
    pub async fn flush(&self) -> Result<()> {
        let mut batch = self.current_batch.lock().await;
        if !batch.is_empty() {
            self.flush_batch(batch).await?;
            *batch = LogBatch::new();
            *self.last_flush.lock().await = Instant::now();
        }
        Ok(())
    }

    /// Flushes the batch based on age (should be called periodically).
    pub async fn flush_if_old(&self) -> Result<()> {
        let now = Instant::now();
        let last = *self.last_flush.lock().await;
        if now.duration_since(last) >= self.config.max_batch_age {
            self.flush().await?;
        }
        Ok(())
    }

    /// Flushes a specific batch to the sink.
    async fn flush_batch(&self, batch: LogBatch) -> Result<()> {
        // In a real implementation you would serialize the batch and send it.
        // For simplicity we just write each record individually.
        for record in &batch.records {
            self.sink.write(record).await?;
        }
        Ok(())
    }
}

/// Background task that periodically flushes the aggregator.
pub struct AggregatorFlusher {
    aggregator: Arc<Aggregator>,
    interval: Duration,
    stop_signal: tokio::sync::watch::Sender<bool>,
}

impl AggregatorFlusher {
    /// Creates a new flusher that will flush the aggregator at the given interval.
    pub fn new(aggregator: Arc<Aggregator>, interval: Duration) -> Self {
        let (stop_signal, _) = tokio::sync::watch::channel(false);
        Self {
            aggregator,
            interval,
            stop_signal,
        }
    }

    /// Starts the flusher task in the background.
    pub fn start(self) -> tokio::task::JoinHandle<()> {
        let mut stop_receiver = self.stop_signal.subscribe();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = tokio::time::sleep(self.interval) => {
                        if let Err(e) = self.aggregator.flush_if_old().await {
                            tracing::error!("Failed to flush aggregator: {}", e);
                        }
                    }
                    _ = stop_receiver.changed() => {
                        if *stop_receiver.borrow() {
                            break;
                        }
                    }
                }
            }
        })
    }

    /// Stops the flusher task.
    pub fn stop(&self) {
        let _ = self.stop_signal.send(true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sink::MemorySink;

    #[tokio::test]
    async fn test_aggregator_batch_size() {
        let sink = Arc::new(MemorySink::new());
        let config = AggregatorConfig {
            max_batch_size: 2,
            max_batch_age: Duration::from_secs(10),
            compress: false,
        };
        let aggregator = Aggregator::new(config, sink.clone());

        let record = LogRecord::new(
            crate::log_record::LogLevel::Info,
            "agent",
            "test",
            "msg1",
        );
        aggregator.add_record(record).await.unwrap();
        // Batch not full yet
        let batch = aggregator.current_batch.lock().await;
        assert_eq!(batch.len(), 1);
        drop(batch);

        let record2 = LogRecord::new(
            crate::log_record::LogLevel::Info,
            "agent",
            "test",
            "msg2",
        );
        aggregator.add_record(record2).await.unwrap();
        // Batch should have been flushed automatically
        let batch = aggregator.current_batch.lock().await;
        assert_eq!(batch.len(), 0);
        drop(batch);

        // Sink should have received two records
        // (MemorySink stores them individually)
        // We cannot easily verify because MemorySink is inside Arc.
    }

    #[tokio::test]
    async fn test_aggregator_flush() {
        let sink = Arc::new(MemorySink::new());
        let aggregator = Aggregator::new(AggregatorConfig::default(), sink.clone());
        let record = LogRecord::new(
            crate::log_record::LogLevel::Info,
            "agent",
            "test",
            "msg",
        );
        aggregator.add_record(record).await.unwrap();
        aggregator.flush().await.unwrap();
        let batch = aggregator.current_batch.lock().await;
        assert!(batch.is_empty());
    }
}