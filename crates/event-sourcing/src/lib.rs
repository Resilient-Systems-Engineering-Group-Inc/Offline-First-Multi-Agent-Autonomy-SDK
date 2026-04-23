//! Event sourcing and CQRS for the Multi-Agent SDK.
//!
//! Provides:
//! - Event store with append-only log
//! - Command handlers
//! - Query handlers (read models)
//! - Projections
//! - Event replay

pub mod event;
pub mod command;
pub mod query;
pub mod projection;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::info;

pub use event::*;
pub use command::*;
pub use query::*;
pub use projection::*;

/// Event sourcing configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSourcingConfig {
    pub event_store_type: EventStoreType,
    pub connection_string: String,
    pub snapshot_interval: u32,
    pub enable_projections: bool,
    pub max_events_per_batch: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventStoreType {
    Postgres,
    EventStoreDB,
    Redis,
    InMemory,
}

impl Default for EventSourcingConfig {
    fn default() -> Self {
        Self {
            event_store_type: EventStoreType::InMemory,
            connection_string: "postgresql://localhost/events".to_string(),
            snapshot_interval: 100,
            enable_projections: true,
            max_events_per_batch: 1000,
        }
    }
}

/// Event sourcing manager.
pub struct EventSourcingManager {
    config: EventSourcingConfig,
    event_store: RwLock<EventStore>,
    projections: RwLock<HashMap<String, Box<dyn Projection + Send + Sync>>>,
}

impl EventSourcingManager {
    /// Create new event sourcing manager.
    pub fn new(config: EventSourcingConfig) -> Self {
        Self {
            config,
            event_store: RwLock::new(EventStore::new()),
            projections: RwLock::new(HashMap::new()),
        }
    }

    /// Initialize event store.
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing event store with type: {:?}", self.config.event_store_type);

        // Would initialize database connection here
        info!("Event store initialized");
        Ok(())
    }

    /// Append event to stream.
    pub async fn append(&self, stream_id: &str, event: DomainEvent) -> Result<i64> {
        let mut store = self.event_store.write().await;
        let version = store.append(stream_id, event).await?;

        // Update projections
        if self.config.enable_projections {
            self.update_projections(&store.events[stream_id].last().unwrap()).await?;
        }

        info!("Event appended to {} at version {}", stream_id, version);
        Ok(version)
    }

    /// Append multiple events.
    pub async fn append_batch(&self, stream_id: &str, events: Vec<DomainEvent>) -> Result<i64> {
        let mut store = self.event_store.write().await;
        let mut last_version = 0;

        for event in events {
            last_version = store.append(stream_id, event).await?;
        }

        info!("Batch of {} events appended to {} at version {}", 
            last_version, stream_id, last_version);
        Ok(last_version)
    }

    /// Get events from stream.
    pub async fn get_events(&self, stream_id: &str, from_version: i64) -> Result<Vec<DomainEvent>> {
        let store = self.event_store.read().await;
        store.get_events(stream_id, from_version).await
    }

    /// Get all events.
    pub async fn get_all_events(&self, from_position: i64) -> Result<Vec<StoredEvent>> {
        let store = self.event_store.read().await;
        store.get_all_events(from_position).await
    }

    /// Create snapshot.
    pub async fn create_snapshot(&self, stream_id: &str, state: serde_json::Value) -> Result<()> {
        let mut store = self.event_store.write().await;
        let events = store.events.get(stream_id).map(|e| e.len()).unwrap_or(0);

        if events >= self.config.snapshot_interval as usize {
            store.create_snapshot(stream_id, state).await?;
            info!("Snapshot created for {}", stream_id);
        }

        Ok(())
    }

    /// Get snapshot.
    pub async fn get_snapshot(&self, stream_id: &str) -> Result<Option<serde_json::Value>> {
        let store = self.event_store.read().await;
        store.get_snapshot(stream_id).await
    }

    /// Rebuild state from events.
    pub async fn rebuild_state<F, S>(&self, stream_id: &str, mut apply: F) -> Result<S>
    where
        F: FnMut(S, DomainEvent) -> S,
        S: Default,
    {
        let events = self.get_events(stream_id, 0).await?;
        let snapshot = self.get_snapshot(stream_id).await?;

        let mut state = if let Some(snapshot_state) = snapshot {
            serde_json::from_value(snapshot_state)?
        } else {
            S::default()
        };

        for event in events {
            state = apply(state, event);
        }

        Ok(state)
    }

    /// Register projection.
    pub async fn register_projection<P>(&self, projection: P) -> Result<()>
    where
        P: Projection + Send + Sync + 'static,
    {
        let mut projections = self.projections.write().await;
        let name = projection.name().to_string();
        
        projections.insert(name, Box::new(projection));
        info!("Projection registered");
        Ok(())
    }

    /// Update projections.
    async fn update_projections(&self, event: &DomainEvent) -> Result<()> {
        let projections = self.projections.read().await;
        
        for projection in projections.values() {
            if projection.is_interested_in(&event.event_type) {
                projection.handle(event).await?;
            }
        }

        Ok(())
    }

    /// Get event store statistics.
    pub async fn get_stats(&self) -> EventSourcingStats {
        let store = self.event_store.read().await;
        let projections = self.projections.read().await;

        let total_events: usize = store.events.values().map(|e| e.len()).sum();
        let total_streams = store.events.len();

        EventSourcingStats {
            total_events: total_events as i64,
            total_streams: total_streams as i32,
            total_projections: projections.len() as i32,
            snapshot_interval: self.config.snapshot_interval,
        }
    }
}

/// Event store.
struct EventStore {
    events: HashMap<String, Vec<DomainEvent>>,
    snapshots: HashMap<String, Snapshot>,
    positions: HashMap<String, i64>,
}

/// Snapshot of aggregate state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub stream_id: String,
    pub version: i64,
    pub state: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Stored event with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredEvent {
    pub position: i64,
    pub stream_id: String,
    pub version: i64,
    pub event: DomainEvent,
    pub recorded_at: chrono::DateTime<chrono::Utc>,
}

impl EventStore {
    fn new() -> Self {
        Self {
            events: HashMap::new(),
            snapshots: HashMap::new(),
            positions: HashMap::new(),
        }
    }

    async fn append(&mut self, stream_id: &str, event: DomainEvent) -> Result<i64> {
        let stream = self.events.entry(stream_id.to_string()).or_insert_with(Vec::new);
        let version = stream.len() as i64 + 1;

        let position = self.positions.entry(stream_id.to_string()).or_insert(0);
        *position += 1;

        let stored = StoredEvent {
            position: *position,
            stream_id: stream_id.to_string(),
            version,
            event: event.clone(),
            recorded_at: chrono::Utc::now(),
        };

        stream.push(event);

        Ok(version)
    }

    async fn get_events(&self, stream_id: &str, from_version: i64) -> Result<Vec<DomainEvent>> {
        let events = self.events.get(stream_id)
            .ok_or_else(|| anyhow::anyhow!("Stream not found: {}", stream_id))?;

        let filtered: Vec<_> = events.iter()
            .skip(from_version as usize)
            .cloned()
            .collect();

        Ok(filtered)
    }

    async fn get_all_events(&self, from_position: i64) -> Result<Vec<StoredEvent>> {
        let mut all_events = vec![];

        for (stream_id, events) in &self.events {
            for (idx, event) in events.iter().enumerate() {
                let position = (idx + 1) as i64;
                if position > from_position {
                    all_events.push(StoredEvent {
                        position,
                        stream_id: stream_id.clone(),
                        version: position,
                        event: event.clone(),
                        recorded_at: chrono::Utc::now(),
                    });
                }
            }
        }

        all_events.sort_by_key(|e| e.position);
        Ok(all_events)
    }

    async fn create_snapshot(&mut self, stream_id: &str, state: serde_json::Value) -> Result<()> {
        let version = self.events.get(stream_id).map(|e| e.len() as i64).unwrap_or(0);

        let snapshot = Snapshot {
            stream_id: stream_id.to_string(),
            version,
            state,
            created_at: chrono::Utc::now(),
        };

        self.snapshots.insert(stream_id.to_string(), snapshot);
        Ok(())
    }

    async fn get_snapshot(&self, stream_id: &str) -> Result<Option<serde_json::Value>> {
        Ok(self.snapshots.get(stream_id).map(|s| s.state.clone()))
    }
}

/// Event sourcing statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSourcingStats {
    pub total_events: i64,
    pub total_streams: i32,
    pub total_projections: i32,
    pub snapshot_interval: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_sourcing() {
        let config = EventSourcingConfig::default();
        let manager = EventSourcingManager::new(config);

        // Initialize
        manager.initialize().await.unwrap();

        // Append events
        let event1 = DomainEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            event_type: "TaskCreated".to_string(),
            aggregate_id: "task-1".to_string(),
            data: serde_json::json!({"description": "Test task"}),
            metadata: serde_json::json!({}),
            timestamp: chrono::Utc::now(),
        };

        let version = manager.append("task-1", event1).await.unwrap();
        assert_eq!(version, 1);

        // Get events
        let events = manager.get_events("task-1", 0).await.unwrap();
        assert_eq!(events.len(), 1);

        // Get stats
        let stats = manager.get_stats().await;
        assert_eq!(stats.total_events, 1);
        assert_eq!(stats.total_streams, 1);
    }
}
