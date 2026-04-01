//! Core types for event-driven architecture.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Unique identifier for an event.
pub type EventId = Uuid;

/// Event type/category.
pub type EventType = String;

/// Event priority levels.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventPriority {
    /// Low priority (background tasks).
    Low = 0,
    /// Normal priority (default).
    Normal = 1,
    /// High priority (urgent but not critical).
    High = 2,
    /// Critical priority (must be processed immediately).
    Critical = 3,
}

impl Default for EventPriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// Core event structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Unique event identifier.
    pub id: EventId,
    /// Event type/category.
    pub event_type: EventType,
    /// Event source (who produced it).
    pub source: String,
    /// Event timestamp (UTC).
    pub timestamp: DateTime<Utc>,
    /// Event priority.
    pub priority: EventPriority,
    /// Event payload (serialized data).
    pub payload: serde_json::Value,
    /// Correlation ID for tracing related events.
    pub correlation_id: Option<EventId>,
    /// Causation ID (ID of the event that caused this one).
    pub causation_id: Option<EventId>,
    /// Metadata (key-value pairs).
    pub metadata: HashMap<String, String>,
    /// TTL (time-to-live) in seconds (optional).
    pub ttl_seconds: Option<u64>,
}

impl Event {
    /// Create a new event.
    pub fn new(
        event_type: EventType,
        source: String,
        payload: serde_json::Value,
        priority: EventPriority,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type,
            source,
            timestamp: Utc::now(),
            priority,
            payload,
            correlation_id: None,
            causation_id: None,
            metadata: HashMap::new(),
            ttl_seconds: None,
        }
    }

    /// Create a new event with default priority.
    pub fn new_normal(event_type: EventType, source: String, payload: serde_json::Value) -> Self {
        Self::new(event_type, source, payload, EventPriority::Normal)
    }

    /// Set correlation ID.
    pub fn with_correlation_id(mut self, correlation_id: EventId) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }

    /// Set causation ID.
    pub fn with_causation_id(mut self, causation_id: EventId) -> Self {
        self.causation_id = Some(causation_id);
        self
    }

    /// Set metadata.
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Set TTL.
    pub fn with_ttl(mut self, ttl_seconds: u64) -> Self {
        self.ttl_seconds = Some(ttl_seconds);
        self
    }

    /// Check if event is expired.
    pub fn is_expired(&self) -> bool {
        match self.ttl_seconds {
            Some(ttl) => {
                let expiry = self.timestamp + chrono::Duration::seconds(ttl as i64);
                Utc::now() > expiry
            }
            None => false,
        }
    }

    /// Serialize event to bytes (JSON).
    pub fn to_bytes(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    /// Deserialize event from bytes (JSON).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}

/// Event envelope for routing.
#[derive(Debug, Clone)]
pub struct EventEnvelope {
    /// The event itself.
    pub event: Event,
    /// Routing key/topic.
    pub topic: String,
    /// Partition key (for ordered processing).
    pub partition_key: Option<String>,
    /// Delivery attempt count.
    pub delivery_attempt: u32,
}

/// Event handler result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HandlerResult {
    /// Event processed successfully.
    Success,
    /// Event processing failed (should be retried).
    Retry { delay_seconds: u64, reason: String },
    /// Event processing failed permanently (should be dead-lettered).
    Failure { reason: String },
    /// Event should be ignored.
    Ignored,
}

/// Subscription filter for selecting events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionFilter {
    /// Event types to include (wildcards supported).
    pub event_types: Vec<String>,
    /// Sources to include (wildcards supported).
    pub sources: Option<Vec<String>>,
    /// Priority range.
    pub priority_min: Option<EventPriority>,
    /// Priority range.
    pub priority_max: Option<EventPriority>,
    /// Metadata filters (key must match value).
    pub metadata_filters: HashMap<String, String>,
    /// Payload filter (JSON path expression).
    pub payload_filter: Option<String>,
}

impl SubscriptionFilter {
    /// Create a filter that matches all events.
    pub fn all() -> Self {
        Self {
            event_types: vec!["*".to_string()],
            sources: None,
            priority_min: None,
            priority_max: None,
            metadata_filters: HashMap::new(),
            payload_filter: None,
        }
    }

    /// Create a filter that matches specific event types.
    pub fn for_event_types(event_types: Vec<String>) -> Self {
        Self {
            event_types,
            sources: None,
            priority_min: None,
            priority_max: None,
            metadata_filters: HashMap::new(),
            payload_filter: None,
        }
    }

    /// Check if an event matches this filter.
    pub fn matches(&self, event: &Event) -> bool {
        // Check event type
        let type_matches = self.event_types.iter().any(|pattern| {
            if pattern == "*" {
                true
            } else if pattern.ends_with('*') {
                let prefix = &pattern[..pattern.len() - 1];
                event.event_type.starts_with(prefix)
            } else {
                pattern == &event.event_type
            }
        });

        if !type_matches {
            return false;
        }

        // Check source
        if let Some(sources) = &self.sources {
            let source_matches = sources.iter().any(|pattern| {
                if pattern == "*" {
                    true
                } else if pattern.ends_with('*') {
                    let prefix = &pattern[..pattern.len() - 1];
                    event.source.starts_with(prefix)
                } else {
                    pattern == &event.source
                }
            });
            if !source_matches {
                return false;
            }
        }

        // Check priority range
        if let Some(min) = self.priority_min {
            if event.priority < min {
                return false;
            }
        }
        if let Some(max) = self.priority_max {
            if event.priority > max {
                return false;
            }
        }

        // Check metadata filters
        for (key, value) in &self.metadata_filters {
            match event.metadata.get(key) {
                Some(v) if v == value => continue,
                _ => return false,
            }
        }

        // TODO: Check payload filter (would require JSON path evaluation)

        true
    }
}

/// Event handler trait.
#[async_trait::async_trait]
pub trait EventHandler: Send + Sync {
    /// Handle an event.
    async fn handle(&self, event: EventEnvelope) -> HandlerResult;
}

/// Event bus statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventBusStats {
    /// Total events published.
    pub total_published: u64,
    /// Total events delivered.
    pub total_delivered: u64,
    /// Total events failed.
    pub total_failed: u64,
    /// Current active subscriptions.
    pub active_subscriptions: usize,
    /// Events by type.
    pub events_by_type: HashMap<String, u64>,
    /// Average processing time in milliseconds.
    pub avg_processing_time_ms: f64,
    /// Timestamp of last event.
    pub last_event_at: Option<DateTime<Utc>>,
}

/// Common event types for the multi-agent system.
pub mod event_types {
    /// Agent lifecycle events.
    pub mod agent {
        pub const AGENT_CREATED: &str = "agent.created";
        pub const AGENT_STARTED: &str = "agent.started";
        pub const AGENT_STOPPED: &str = "agent.stopped";
        pub const AGENT_FAILED: &str = "agent.failed";
        pub const AGENT_HEALTH_CHANGED: &str = "agent.health_changed";
    }

    /// Task events.
    pub mod task {
        pub const TASK_CREATED: &str = "task.created";
        pub const TASK_ASSIGNED: &str = "task.assigned";
        pub const TASK_STARTED: &str = "task.started";
        pub const TASK_COMPLETED: &str = "task.completed";
        pub const TASK_FAILED: &str = "task.failed";
        pub const TASK_CANCELLED: &str = "task.cancelled";
    }

    /// Resource events.
    pub mod resource {
        pub const RESOURCE_LOW: &str = "resource.low";
        pub const RESOURCE_CRITICAL: &str = "resource.critical";
        pub const RESOURCE_RECOVERED: &str = "resource.recovered";
    }

    /// Network events.
    pub mod network {
        pub const PEER_DISCOVERED: &str = "network.peer_discovered";
        pub const PEER_LOST: &str = "network.peer_lost";
        pub const MESSAGE_RECEIVED: &str = "network.message_received";
        pub const MESSAGE_SENT: &str = "network.message_sent";
    }

    /// System events.
    pub mod system {
        pub const SYSTEM_STARTUP: &str = "system.startup";
        pub const SYSTEM_SHUTDOWN: &str = "system.shutdown";
        pub const CONFIG_CHANGED: &str = "system.config_changed";
        pub const ALERT_TRIGGERED: &str = "system.alert_triggered";
    }

    /// ML events.
    pub mod ml {
        pub const MODEL_TRAINED: &str = "ml.model_trained";
        pub const MODEL_PUBLISHED: &str = "ml.model_published";
        pub const MODEL_DEPLOYED: &str = "ml.model_deployed";
        pub const TRAINING_STARTED: &str = "ml.training_started";
        pub const TRAINING_COMPLETED: &str = "ml.training_completed";
    }
}