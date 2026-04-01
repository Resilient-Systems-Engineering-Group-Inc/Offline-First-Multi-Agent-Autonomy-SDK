//! Integration with existing SDK components.

use std::sync::Arc;

use async_trait::async_trait;

use crate::bus::EventBus;
use crate::error::Result;
use crate::types::{Event, EventEnvelope, EventHandler, EventPriority, HandlerResult, SubscriptionFilter};

/// Integration with mesh transport.
#[cfg(feature = "mesh")]
pub mod mesh_integration {
    use super::*;
    use crate::mesh_transport::{MeshTransport, TransportEvent};

    /// Bridge between mesh transport and event bus.
    pub struct MeshEventBridge {
        event_bus: Arc<EventBus>,
        topic: String,
    }

    impl MeshEventBridge {
        /// Create a new bridge.
        pub fn new(event_bus: Arc<EventBus>, topic: &str) -> Self {
            Self {
                event_bus,
                topic: topic.to_string(),
            }
        }

        /// Start bridging mesh events to the event bus.
        pub async fn start(&self, mut transport: MeshTransport) -> Result<()> {
            let event_bus = self.event_bus.clone();
            let topic = self.topic.clone();

            tokio::spawn(async move {
                while let Some(event) = transport.events().next().await {
                    match event {
                        TransportEvent::MessageReceived { from, payload } => {
                            let event = Event::new_normal(
                                "mesh.message_received".to_string(),
                                format!("agent_{}", from),
                                serde_json::json!({
                                    "from": from,
                                    "payload_size": payload.len(),
                                }),
                            );

                            let _ = event_bus.publish(&topic, event).await;
                        }
                        TransportEvent::PeerConnected(peer_id) => {
                            let event = Event::new_normal(
                                "mesh.peer_connected".to_string(),
                                "mesh".to_string(),
                                serde_json::json!({
                                    "peer_id": peer_id,
                                }),
                            );

                            let _ = event_bus.publish(&topic, event).await;
                        }
                        TransportEvent::PeerDisconnected(peer_id) => {
                            let event = Event::new_normal(
                                "mesh.peer_disconnected".to_string(),
                                "mesh".to_string(),
                                serde_json::json!({
                                    "peer_id": peer_id,
                                }),
                            );

                            let _ = event_bus.publish(&topic, event).await;
                        }
                        _ => {}
                    }
                }
            });

            Ok(())
        }
    }
}

/// Integration with agent core.
#[cfg(feature = "agent")]
pub mod agent_integration {
    use super::*;
    use crate::agent_core::{Agent, AgentState};

    /// Event handler for agent lifecycle events.
    pub struct AgentLifecycleEventHandler {
        event_bus: Arc<EventBus>,
        topic: String,
    }

    impl AgentLifecycleEventHandler {
        /// Create a new handler.
        pub fn new(event_bus: Arc<EventBus>, topic: &str) -> Self {
            Self {
                event_bus,
                topic: topic.to_string(),
            }
        }
    }

    #[async_trait]
    impl EventHandler for AgentLifecycleEventHandler {
        async fn handle(&self, envelope: EventEnvelope) -> HandlerResult {
            // In a real implementation, you would update agent state based on events
            // For now, just log and forward
            tracing::debug!(
                "Agent lifecycle event: {} from {}",
                envelope.event.event_type,
                envelope.event.source
            );

            HandlerResult::Success
        }
    }

    /// Instrument an agent to publish events.
    pub struct InstrumentedAgent {
        inner: Agent,
        event_bus: Arc<EventBus>,
        topic: String,
    }

    impl InstrumentedAgent {
        /// Create a new instrumented agent.
        pub fn new(agent: Agent, event_bus: Arc<EventBus>, topic: &str) -> Self {
            Self {
                inner: agent,
                event_bus,
                topic: topic.to_string(),
            }
        }

        /// Start the agent and publish events.
        pub async fn start(&mut self) -> Result<()> {
            // Publish agent started event
            let event = Event::new_normal(
                "agent.started".to_string(),
                self.inner.id().to_string(),
                serde_json::json!({
                    "agent_id": self.inner.id(),
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                }),
            );

            self.event_bus.publish(&self.topic, event).await?;

            // Start the inner agent
            self.inner.start().await?;

            Ok(())
        }

        /// Stop the agent and publish events.
        pub async fn stop(&mut self) -> Result<()> {
            // Publish agent stopping event
            let event = Event::new_normal(
                "agent.stopping".to_string(),
                self.inner.id().to_string(),
                serde_json::json!({
                    "agent_id": self.inner.id(),
                }),
            );

            self.event_bus.publish(&self.topic, event).await?;

            // Stop the inner agent
            self.inner.stop().await?;

            // Publish agent stopped event
            let event = Event::new_normal(
                "agent.stopped".to_string(),
                self.inner.id().to_string(),
                serde_json::json!({
                    "agent_id": self.inner.id(),
                }),
            );

            self.event_bus.publish(&self.topic, event).await?;

            Ok(())
        }
    }
}

/// Integration with distributed planner.
pub mod planner_integration {
    use super::*;

    /// Event types for planning.
    pub mod event_types {
        pub const TASK_CREATED: &str = "planner.task_created";
        pub const TASK_ASSIGNED: &str = "planner.task_assigned";
        pub const TASK_COMPLETED: &str = "planner.task_completed";
        pub const PLANNING_ROUND_STARTED: &str = "planner.round_started";
        pub const PLANNING_ROUND_COMPLETED: &str = "planner.round_completed";
    }

    /// Event handler for planning events.
    pub struct PlanningEventHandler {
        /// Callback for task assignment events.
        pub on_task_assigned: Option<Box<dyn Fn(Event) + Send + Sync>>,
    }

    #[async_trait]
    impl EventHandler for PlanningEventHandler {
        async fn handle(&self, envelope: EventEnvelope) -> HandlerResult {
            match envelope.event.event_type.as_str() {
                event_types::TASK_ASSIGNED => {
                    if let Some(callback) = &self.on_task_assigned {
                        callback(envelope.event);
                    }
                }
                _ => {}
            }
            HandlerResult::Success
        }
    }
}

/// Integration with monitoring.
pub mod monitoring_integration {
    use super::*;

    /// Event handler that forwards events to monitoring system.
    pub struct MonitoringForwarder {
        /// Monitoring endpoint (e.g., Prometheus push gateway).
        endpoint: String,
    }

    impl MonitoringForwarder {
        /// Create a new forwarder.
        pub fn new(endpoint: &str) -> Self {
            Self {
                endpoint: endpoint.to_string(),
            }
        }
    }

    #[async_trait]
    impl EventHandler for MonitoringForwarder {
        async fn handle(&self, envelope: EventEnvelope) -> HandlerResult {
            // In a real implementation, you would forward the event to monitoring
            tracing::debug!(
                "Forwarding event {} to monitoring endpoint {}",
                envelope.event.id,
                self.endpoint
            );
            HandlerResult::Success
        }
    }
}

/// Event-driven configuration manager.
pub struct EventDrivenConfigManager {
    event_bus: Arc<EventBus>,
    config_topic: String,
}

impl EventDrivenConfigManager {
    /// Create a new event-driven config manager.
    pub fn new(event_bus: Arc<EventBus>, config_topic: &str) -> Self {
        Self {
            event_bus,
            config_topic: config_topic.to_string(),
        }
    }

    /// Publish a configuration change event.
    pub async fn publish_config_change(
        &self,
        config_key: &str,
        old_value: serde_json::Value,
        new_value: serde_json::Value,
        source: &str,
    ) -> Result<()> {
        let event = Event::new_normal(
            "config.changed".to_string(),
            source.to_string(),
            serde_json::json!({
                "key": config_key,
                "old_value": old_value,
                "new_value": new_value,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }),
        );

        self.event_bus.publish(&self.config_topic, event).await
    }

    /// Subscribe to configuration changes.
    pub async fn subscribe_to_config_changes(
        &self,
        handler: Arc<dyn EventHandler>,
    ) -> Result<String> {
        let filter = SubscriptionFilter::for_event_types(vec!["config.changed".to_string()]);
        self.event_bus
            .subscribe(&self.config_topic, filter, handler)
            .await
    }
}

/// Event-driven task scheduler.
pub struct EventDrivenTaskScheduler {
    event_bus: Arc<EventBus>,
    task_topic: String,
}

impl EventDrivenTaskScheduler {
    /// Create a new event-driven task scheduler.
    pub fn new(event_bus: Arc<EventBus>, task_topic: &str) -> Self {
        Self {
            event_bus,
            task_topic: task_topic.to_string(),
        }
    }

    /// Schedule a task by publishing an event.
    pub async fn schedule_task(
        &self,
        task_id: &str,
        task_type: &str,
        parameters: serde_json::Value,
        priority: EventPriority,
    ) -> Result<()> {
        let event = Event::new(
            "task.scheduled".to_string(),
            "scheduler".to_string(),
            serde_json::json!({
                "task_id": task_id,
                "task_type": task_type,
                "parameters": parameters,
                "scheduled_at": chrono::Utc::now().to_rfc3339(),
            }),
            priority,
        );

        self.event_bus.publish(&self.task_topic, event).await
    }

    /// Subscribe to task completion events.
    pub async fn subscribe_to_task_completions(
        &self,
        handler: Arc<dyn EventHandler>,
    ) -> Result<String> {
        let filter = SubscriptionFilter::for_event_types(vec![
            "task.completed".to_string(),
            "task.failed".to_string(),
        ]);
        self.event_bus.subscribe(&self.task_topic, filter, handler).await
    }
}