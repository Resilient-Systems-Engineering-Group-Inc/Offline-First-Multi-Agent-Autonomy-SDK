//! Stream processors.

use crate::stream::{StreamEvent, StreamSink};
use anyhow::Result;
use std::sync::Arc;

/// Stream processor - processes events with custom logic.
pub struct StreamProcessor {
    name: String,
    logic: Arc<dyn EventLogic>,
    sink: Arc<dyn StreamSink>,
}

/// Event processing logic trait.
#[async_trait::async_trait]
pub trait EventLogic: Send + Sync {
    async fn process(&self, event: StreamEvent) -> Result<Option<StreamEvent>>;
}

impl StreamProcessor {
    pub fn new<L, S>(name: &str, logic: L, sink: S) -> Self
    where
        L: EventLogic + 'static,
        S: StreamSink + 'static,
    {
        Self {
            name: name.to_string(),
            logic: Arc::new(logic),
            sink: Arc::new(sink),
        }
    }

    pub async fn process_event(&self, event: StreamEvent) -> Result<()> {
        if let Some(processed) = self.logic.process(event).await? {
            self.sink.write(processed).await?;
        }
        Ok(())
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Simple processor that applies a function.
pub struct SimpleProcessor<F>
where
    F: Fn(StreamEvent) -> Result<Option<StreamEvent>> + Send + Sync,
{
    func: F,
}

impl<F> SimpleProcessor<F>
where
    F: Fn(StreamEvent) -> Result<Option<StreamEvent>> + Send + Sync,
{
    pub fn new(func: F) -> Self {
        Self { func }
    }
}

#[async_trait::async_trait]
impl<F> EventLogic for SimpleProcessor<F>
where
    F: Fn(StreamEvent) -> Result<Option<StreamEvent>> + Send + Sync,
{
    async fn process(&self, event: StreamEvent) -> Result<Option<StreamEvent>> {
        (self.func)(event)
    }
}

/// Aggregating processor.
pub struct AggregatingProcessor {
    name: String,
    aggregator: Arc<dyn Aggregator>,
}

/// Aggregator trait.
#[async_trait::async_trait]
pub trait Aggregator: Send + Sync {
    async fn add(&self, event: &StreamEvent) -> Result<()>;
    async fn get_result(&self) -> Result<serde_json::Value>;
    async fn reset(&self) -> Result<()>;
}

impl AggregatingProcessor {
    pub fn new(name: &str, aggregator: Arc<dyn Aggregator>) -> Self {
        Self {
            name: name.to_string(),
            aggregator,
        }
    }

    pub async fn get_aggregation(&self) -> Result<serde_json::Value> {
        self.aggregator.get_result().await
    }

    pub async fn reset(&self) -> Result<()> {
        self.aggregator.reset().await
    }
}

#[async_trait::async_trait]
impl EventLogic for AggregatingProcessor {
    async fn process(&self, event: StreamEvent) -> Result<Option<StreamEvent>> {
        self.aggregator.add(&event).await?;
        Ok(Some(event))
    }
}

/// Count aggregator.
pub struct CountAggregator {
    count: tokio::sync::Mutex<i64>,
}

impl CountAggregator {
    pub fn new() -> Self {
        Self {
            count: tokio::sync::Mutex::new(0),
        }
    }
}

impl Default for CountAggregator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl Aggregator for CountAggregator {
    async fn add(&self, _event: &StreamEvent) -> Result<()> {
        let mut count = self.count.lock().await;
        *count += 1;
        Ok(())
    }

    async fn get_result(&self) -> Result<serde_json::Value> {
        let count = *self.count.lock().await;
        Ok(serde_json::json!({"count": count}))
    }

    async fn reset(&self) -> Result<()> {
        *self.count.lock().await = 0;
        Ok(())
    }
}

/// Sum aggregator.
pub struct SumAggregator {
    field: String,
    sum: tokio::sync::Mutex<f64>,
}

impl SumAggregator {
    pub fn new(field: &str) -> Self {
        Self {
            field: field.to_string(),
            sum: tokio::sync::Mutex::new(0.0),
        }
    }
}

#[async_trait::async_trait]
impl Aggregator for SumAggregator {
    async fn add(&self, event: &StreamEvent) -> Result<()> {
        let value = event.data
            .get(&self.field)
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let mut sum = self.sum.lock().await;
        *sum += value;
        Ok(())
    }

    async fn get_result(&self) -> Result<serde_json::Value> {
        let sum = *self.sum.lock().await;
        Ok(serde_json::json!({"sum": sum, "field": self.field}))
    }

    async fn reset(&self) -> Result<()> {
        *self.sum.lock().await = 0.0;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::MemorySink;

    #[tokio::test]
    async fn test_simple_processor() {
        let sink = MemorySink::new();
        
        let processor = StreamProcessor::new(
            "test-processor",
            SimpleProcessor::new(|mut e| {
                if let Some(obj) = e.data.as_object_mut() {
                    obj.insert("processed".to_string(), serde_json::json!(true));
                }
                Ok(Some(e))
            }),
            sink.clone(),
        );

        let event = StreamEvent {
            id: "1".to_string(),
            data: serde_json::json!({"value": 42}),
            timestamp: chrono::Utc::now(),
            event_time: chrono::Utc::now(),
        };

        processor.process_event(event).await.unwrap();

        let events = sink.get_events().await;
        assert_eq!(events.len(), 1);
        assert!(events[0].data.get("processed").and_then(|v| v.as_bool()).unwrap());
    }

    #[tokio::test]
    async fn test_count_aggregator() {
        let aggregator = Arc::new(CountAggregator::new());
        let processor = AggregatingProcessor::new("count", aggregator.clone());

        for i in 0..5 {
            let event = StreamEvent {
                id: i.to_string(),
                data: serde_json::json!({}),
                timestamp: chrono::Utc::now(),
                event_time: chrono::Utc::now(),
            };
            processor.process(event).await.unwrap();
        }

        let result = aggregator.get_result().await.unwrap();
        assert_eq!(result["count"], 5);
    }
}
