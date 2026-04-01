//! Event‑driven architecture for Offline‑First Multi‑Agent Autonomy SDK.
//!
//! This crate provides a comprehensive event‑driven architecture for building
//! decoupled, scalable multi‑agent systems. It includes an event bus, publishers,
//! subscribers, and integration with existing SDK components.
//!
//! # Features
//!
//! - **Event bus**: In‑memory event bus with topics, filters, and priorities
//! - **Event types**: Rich event model with metadata, correlation IDs, and TTL
//! - **Subscriptions**: Flexible subscription filters (type, source, metadata, etc.)
//! - **Handlers**: Async event handlers with retry and dead‑letter queue support
//! - **Integration**: Ready‑to‑use integrations with mesh transport, agent core, planning, etc.
//! - **Reactive streams**: Convert events to async streams for reactive programming
//!
//! # Architecture
//!
//! The system is built around three core abstractions:
//!
//! 1. **`Event`**: Immutable event structure with type, payload, metadata, etc.
//! 2. **`EventBus`**: Central event routing and delivery system
//! 3. **`EventHandler`**: Trait for processing events asynchronously
//!
//! # Examples
//!
//! ## Basic usage
//! ```
//! use event_driven::{
//!     EventBus, EventBusConfig,
//!     types::{Event, EventHandler, EventEnvelope, HandlerResult, SubscriptionFilter},
//! };
//! use std::sync::Arc;
//!
//! struct LoggingHandler;
//!
//! #[async_trait::async_trait]
//! impl EventHandler for LoggingHandler {
//!     async fn handle(&self, envelope: EventEnvelope) -> HandlerResult {
//!         println!("Received event: {} from {}", envelope.event.event_type, envelope.event.source);
//!         HandlerResult::Success
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create event bus
//!     let config = EventBusConfig::default();
//!     let bus = EventBus::new(config);
//!
//!     // Subscribe to all events on "system" topic
//!     let handler = Arc::new(LoggingHandler);
//!     let filter = SubscriptionFilter::all();
//!     bus.subscribe("system", filter, handler).await?;
//!
//!     // Start processing
//!     bus.start_processing("system").await?;
//!
//!     // Publish an event
//!     let event = Event::new_normal(
//!         "test.event".to_string(),
//!         "example".to_string(),
//!         serde_json::json!({"message": "Hello, event-driven world!"}),
//!     );
//!
//!     bus.publish("system", event).await?;
//!
//!     // Wait a bit for processing
//!     tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Integration with mesh transport
//! ```ignore
//! use event_driven::integration::mesh_integration::MeshEventBridge;
//! use mesh_transport::MeshTransport;
//!
//! async fn setup_mesh_event_bridge(
//!     bus: Arc<EventBus>,
//!     transport: MeshTransport,
//! ) -> Result<()> {
//!     let bridge = MeshEventBridge::new(bus, "mesh-events");
//!     bridge.start(transport).await?;
//!     Ok(())
//! }
//! ```
//!
//! ## Event-driven configuration
//! ```ignore
//! use event_driven::integration::EventDrivenConfigManager;
//!
//! async fn handle_config_changes(bus: Arc<EventBus>) -> Result<()> {
//!     let config_manager = EventDrivenConfigManager::new(bus, "config");
//!     
//!     // Publish config change
//!     config_manager.publish_config_change(
//!         "max_tasks",
//!         serde_json::json!(10),
//!         serde_json::json!(20),
//!         "admin",
//!     ).await?;
//!     
//!     Ok(())
//! }
//! ```

pub mod bus;
pub mod error;
pub mod integration;
pub mod types;

// Re-export commonly used types
pub use bus::{EventBus, EventBusConfig, InMemoryEventBus};
pub use error::{EventError, Result};
pub use types::{
    Event, EventEnvelope, EventHandler, EventId, EventPriority, EventType, HandlerResult,
    SubscriptionFilter,
};

// Re-export integration modules conditionally
#[cfg(feature = "mesh")]
pub use integration::mesh_integration;
#[cfg(feature = "agent")]
pub use integration::agent_integration;

pub use integration::{
    EventDrivenConfigManager, EventDrivenTaskScheduler,
    planner_integration, monitoring_integration,
};

/// Current version of the event-driven crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize the event-driven system.
pub fn init() {
    tracing::info!("Event-Driven Architecture v{} initialized", VERSION);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::time::Duration;

    struct TestHandler {
        received: Arc<std::sync::Mutex<Vec<String>>>,
    }

    #[async_trait::async_trait]
    impl EventHandler for TestHandler {
        async fn handle(&self, envelope: EventEnvelope) -> HandlerResult {
            self.received
                .lock()
                .unwrap()
                .push(envelope.event.event_type.clone());
            HandlerResult::Success
        }
    }

    #[tokio::test]
    async fn test_basic_publish_subscribe() {
        let bus = EventBus::new(EventBusConfig::default());
        
        let received = Arc::new(std::sync::Mutex::new(Vec::new()));
        let handler = Arc::new(TestHandler {
            received: received.clone(),
        });
        
        let filter = SubscriptionFilter::for_event_types(vec!["test.event".to_string()]);
        bus.subscribe("test-topic", filter, handler).await.unwrap();
        bus.start_processing("test-topic").await.unwrap();
        
        let event = Event::new_normal(
            "test.event".to_string(),
            "test".to_string(),
            serde_json::json!({}),
        );
        
        bus.publish("test-topic", event).await.unwrap();
        
        // Give some time for processing
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        let received = received.lock().unwrap();
        assert_eq!(received.len(), 1);
        assert_eq!(received[0], "test.event");
    }

    #[tokio::test]
    async fn test_filter_matching() {
        let bus = EventBus::new(EventBusConfig::default());
        
        let received = Arc::new(std::sync::Mutex::new(Vec::new()));
        let handler = Arc::new(TestHandler {
            received: received.clone(),
        });
        
        // Subscribe only to "agent.*" events
        let filter = SubscriptionFilter::for_event_types(vec!["agent.*".to_string()]);
        bus.subscribe("agents", filter, handler).await.unwrap();
        bus.start_processing("agents").await.unwrap();
        
        // Publish an agent event (should be received)
        let agent_event = Event::new_normal(
            "agent.created".to_string(),
            "system".to_string(),
            serde_json::json!({}),
        );
        bus.publish("agents", agent_event).await.unwrap();
        
        // Publish a non-agent event (should be filtered out)
        let other_event = Event::new_normal(
            "task.created".to_string(),
            "system".to_string(),
            serde_json::json!({}),
        );
        bus.publish("agents", other_event).await.unwrap();
        
        tokio::time::sleep(Duration::from_millis(50)).await;
        
        let received = received.lock().unwrap();
        assert_eq!(received.len(), 1);
        assert_eq!(received[0], "agent.created");
    }

    #[test]
    fn test_event_creation() {
        let event = Event::new_normal(
            "test".to_string(),
            "source".to_string(),
            serde_json::json!({"key": "value"}),
        );
        
        assert_eq!(event.event_type, "test");
        assert_eq!(event.source, "source");
        assert_eq!(event.priority, EventPriority::Normal);
        assert!(!event.id.is_nil());
    }
}