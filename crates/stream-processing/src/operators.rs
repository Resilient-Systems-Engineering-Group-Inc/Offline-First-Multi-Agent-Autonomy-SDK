//! Stream operators.

use crate::stream::{StreamEvent, StreamOperator};
use anyhow::Result;

/// Map operator - transforms events.
pub struct MapOperator<F>
where
    F: Fn(StreamEvent) -> Result<Option<StreamEvent>> + Send + Sync,
{
    name: String,
    func: F,
}

impl<F> MapOperator<F>
where
    F: Fn(StreamEvent) -> Result<Option<StreamEvent>> + Send + Sync,
{
    pub fn new(name: &str, func: F) -> Self {
        Self {
            name: name.to_string(),
            func,
        }
    }
}

#[async_trait::async_trait]
impl<F> StreamOperator for MapOperator<F>
where
    F: Fn(StreamEvent) -> Result<Option<StreamEvent>> + Send + Sync,
{
    fn name(&self) -> &str {
        &self.name
    }

    async fn process(&self, event: StreamEvent) -> Result<Option<StreamEvent>> {
        (self.func)(event)
    }
}

/// Filter operator - filters events.
pub struct FilterOperator<F>
where
    F: Fn(&StreamEvent) -> bool + Send + Sync,
{
    name: String,
    predicate: F,
}

impl<F> FilterOperator<F>
where
    F: Fn(&StreamEvent) -> bool + Send + Sync,
{
    pub fn new(name: &str, predicate: F) -> Self {
        Self {
            name: name.to_string(),
            predicate,
        }
    }
}

#[async_trait::async_trait]
impl<F> StreamOperator for FilterOperator<F>
where
    F: Fn(&StreamEvent) -> bool + Send + Sync,
{
    fn name(&self) -> &str {
        &self.name
    }

    async fn process(&self, event: StreamEvent) -> Result<Option<StreamEvent>> {
        if (self.predicate)(&event) {
            Ok(Some(event))
        } else {
            Ok(None)
        }
    }
}

/// Window operator - groups events by time window.
pub struct WindowOperator {
    name: String,
    window_size_secs: u64,
    slide_size_secs: u64,
}

impl WindowOperator {
    pub fn new(name: &str, window_size_secs: u64, slide_size_secs: u64) -> Self {
        Self {
            name: name.to_string(),
            window_size_secs,
            slide_size_secs,
        }
    }
}

#[async_trait::async_trait]
impl StreamOperator for WindowOperator {
    fn name(&self) -> &str {
        &self.name
    }

    async fn process(&self, event: StreamEvent) -> Result<Option<StreamEvent>> {
        // Add window metadata to event
        let window_start = event.event_time.timestamp() as u64 / self.window_size_secs * self.window_size_secs;
        let window_end = window_start + self.window_size_secs;

        let mut data = event.data.clone();
        if let Some(obj) = data.as_object_mut() {
            obj.insert("window_start".to_string(), serde_json::json!(window_start));
            obj.insert("window_end".to_string(), serde_json::json!(window_end));
        }

        Ok(Some(StreamEvent {
            id: event.id,
            data,
            timestamp: event.timestamp,
            event_time: event.event_time,
        }))
    }
}

/// Reduce operator - aggregates events.
pub struct ReduceOperator<F, T>
where
    F: Fn(T, &StreamEvent) -> T + Send + Sync,
    T: Clone + Send + Sync + Default,
{
    name: String,
    func: F,
    state: std::sync::Mutex<T>,
    _phantom: std::marker::PhantomData<T>,
}

impl<F, T> ReduceOperator<F, T>
where
    F: Fn(T, &StreamEvent) -> T + Send + Sync,
    T: Clone + Send + Sync + Default,
{
    pub fn new(name: &str, func: F) -> Self {
        Self {
            name: name.to_string(),
            func,
            state: std::sync::Mutex::new(T::default()),
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn get_state(&self) -> T {
        self.state.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl<F, T> StreamOperator for ReduceOperator<F, T>
where
    F: Fn(T, &StreamEvent) -> T + Send + Sync,
    T: Clone + Send + Sync + Default,
{
    fn name(&self) -> &str {
        &self.name
    }

    async fn process(&self, event: StreamEvent) -> Result<Option<StreamEvent>> {
        let mut state = self.state.lock().unwrap();
        *state = (self.func)(state.clone(), &event);

        Ok(Some(event))
    }
}

/// KeyBy operator - partitions events by key.
pub struct KeyByOperator<F>
where
    F: Fn(&StreamEvent) -> String + Send + Sync,
{
    name: String,
    key_selector: F,
}

impl<F> KeyByOperator<F>
where
    F: Fn(&StreamEvent) -> String + Send + Sync,
{
    pub fn new(name: &str, key_selector: F) -> Self {
        Self {
            name: name.to_string(),
            key_selector,
        }
    }
}

#[async_trait::async_trait]
impl<F> StreamOperator for KeyByOperator<F>
where
    F: Fn(&StreamEvent) -> String + Send + Sync,
{
    fn name(&self) -> &str {
        &self.name
    }

    async fn process(&self, event: StreamEvent) -> Result<Option<StreamEvent>> {
        let key = (self.key_selector)(&event);
        
        let mut data = event.data.clone();
        if let Some(obj) = data.as_object_mut() {
            obj.insert("_key".to_string(), serde_json::json!(key));
        }

        Ok(Some(StreamEvent {
            id: event.id,
            data,
            timestamp: event.timestamp,
            event_time: event.event_time,
        }))
    }
}

/// FlatMap operator - one-to-many transformation.
pub struct FlatMapOperator<F>
where
    F: Fn(StreamEvent) -> Result<Vec<StreamEvent>> + Send + Sync,
{
    name: String,
    func: F,
    pending: std::sync::Mutex<Vec<StreamEvent>>,
}

impl<F> FlatMapOperator<F>
where
    F: Fn(StreamEvent) -> Result<Vec<StreamEvent>> + Send + Sync,
{
    pub fn new(name: &str, func: F) -> Self {
        Self {
            name: name.to_string(),
            func,
            pending: std::sync::Mutex::new(Vec::new()),
        }
    }
}

#[async_trait::async_trait]
impl<F> StreamOperator for FlatMapOperator<F>
where
    F: Fn(StreamEvent) -> Result<Vec<StreamEvent>> + Send + Sync,
{
    fn name(&self) -> &str {
        &self.name
    }

    async fn process(&self, event: StreamEvent) -> Result<Option<StreamEvent>> {
        // Return first event, store rest in pending
        let mut pending = self.pending.lock().unwrap();
        
        if !pending.is_empty() {
            return Ok(Some(pending.remove(0)));
        }

        let events = (self.func)(event)?;
        let mut events = events.into_iter();
        
        Ok(events.next())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_filter_operator() {
        let filter = FilterOperator::new("test-filter", |e| {
            e.data.get("priority").and_then(|v| v.as_i64()).unwrap_or(0) > 50
        });

        let high_priority = StreamEvent {
            id: "1".to_string(),
            data: serde_json::json!({"priority": 100}),
            timestamp: chrono::Utc::now(),
            event_time: chrono::Utc::now(),
        };

        let low_priority = StreamEvent {
            id: "2".to_string(),
            data: serde_json::json!({"priority": 10}),
            timestamp: chrono::Utc::now(),
            event_time: chrono::Utc::now(),
        };

        assert!(filter.process(high_priority).await.unwrap().is_some());
        assert!(filter.process(low_priority).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_map_operator() {
        let map = MapOperator::new("test-map", |mut e| {
            if let Some(obj) = e.data.as_object_mut() {
                obj.insert("processed".to_string(), serde_json::json!(true));
            }
            Ok(Some(e))
        });

        let event = StreamEvent {
            id: "1".to_string(),
            data: serde_json::json!({"value": 42}),
            timestamp: chrono::Utc::now(),
            event_time: chrono::Utc::now(),
        };

        let result = map.process(event).await.unwrap().unwrap();
        assert!(result.data.get("processed").and_then(|v| v.as_bool()).unwrap());
    }
}
