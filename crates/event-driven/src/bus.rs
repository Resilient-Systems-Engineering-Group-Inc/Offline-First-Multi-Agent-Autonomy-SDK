//! Event bus implementation.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use futures::Stream;
use tokio::sync::{mpsc, RwLock};
use tokio::time::timeout;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;

use crate::error::{EventError, Result};
use crate::types::{Event, EventEnvelope, EventHandler, EventPriority, HandlerResult, SubscriptionFilter};

/// Event bus configuration.
#[derive(Debug, Clone)]
pub struct EventBusConfig {
    /// Maximum queue size per topic.
    pub max_queue_size: usize,
    /// Maximum number of subscribers per topic.
    pub max_subscribers_per_topic: usize,
    /// Default timeout for event delivery (seconds).
    pub default_delivery_timeout_secs: u64,
    /// Enable dead-letter queue for failed events.
    pub enable_dead_letter_queue: bool,
    /// Maximum retry attempts for failed events.
    pub max_retry_attempts: u32,
    /// Retry delay base (seconds).
    pub retry_delay_base_secs: u64,
}

impl Default for EventBusConfig {
    fn default() -> Self {
        Self {
            max_queue_size: 10_000,
            max_subscribers_per_topic: 100,
            default_delivery_timeout_secs: 30,
            enable_dead_letter_queue: true,
            max_retry_attempts: 3,
            retry_delay_base_secs: 1,
        }
    }
}

/// Event bus core implementation.
pub struct EventBus {
    /// Configuration.
    config: EventBusConfig,
    /// Subscriptions by topic.
    subscriptions: DashMap<String, Vec<Subscription>>,
    /// Event queues by topic.
    queues: DashMap<String, mpsc::Sender<EventEnvelope>>,
    /// Statistics.
    stats: Arc<RwLock<BusStats>>,
    /// Dead letter queue (if enabled).
    dead_letter_queue: Option<mpsc::Sender<DeadLetterEvent>>,
}

/// Subscription entry.
struct Subscription {
    /// Subscription ID.
    id: String,
    /// Filter for events.
    filter: SubscriptionFilter,
    /// Handler for events.
    handler: Arc<dyn EventHandler>,
    /// Whether subscription is active.
    active: bool,
}

/// Bus statistics.
#[derive(Debug, Clone, Default)]
struct BusStats {
    /// Total events published.
    total_published: u64,
    /// Total events delivered.
    total_delivered: u64,
    /// Total events failed.
    total_failed: u64,
    /// Events by type.
    events_by_type: HashMap<String, u64>,
    /// Active subscriptions count.
    active_subscriptions: usize,
}

/// Dead letter event.
#[derive(Debug, Clone)]
struct DeadLetterEvent {
    /// Original event.
    event: EventEnvelope,
    /// Failure reason.
    reason: String,
    /// Number of attempts.
    attempts: u32,
    /// Timestamp.
    timestamp: chrono::DateTime<chrono::Utc>,
}

impl EventBus {
    /// Create a new event bus with the given configuration.
    pub fn new(config: EventBusConfig) -> Self {
        let dead_letter_queue = if config.enable_dead_letter_queue {
            let (tx, _) = mpsc::channel(1000);
            Some(tx)
        } else {
            None
        };

        Self {
            config,
            subscriptions: DashMap::new(),
            queues: DashMap::new(),
            stats: Arc::new(RwLock::new(BusStats::default())),
            dead_letter_queue,
        }
    }

    /// Publish an event to a topic.
    pub async fn publish(&self, topic: &str, event: Event) -> Result<()> {
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_published += 1;
            *stats.events_by_type.entry(event.event_type.clone()).or_insert(0) += 1;
        }

        // Create envelope
        let envelope = EventEnvelope {
            event,
            topic: topic.to_string(),
            partition_key: None,
            delivery_attempt: 0,
        };

        // Get or create queue for topic
        let queue = self
            .queues
            .entry(topic.to_string())
            .or_insert_with(|| {
                let (tx, _) = mpsc::channel(self.config.max_queue_size);
                tx
            })
            .clone();

        // Send to queue
        queue
            .send(envelope)
            .await
            .map_err(|e| EventError::PublishError(e.to_string()))?;

        Ok(())
    }

    /// Subscribe to events on a topic with a filter.
    pub async fn subscribe(
        &self,
        topic: &str,
        filter: SubscriptionFilter,
        handler: Arc<dyn EventHandler>,
    ) -> Result<String> {
        let subscription_id = uuid::Uuid::new_v4().to_string();

        let mut subscriptions = self.subscriptions.entry(topic.to_string()).or_default();
        
        // Check subscriber limit
        if subscriptions.len() >= self.config.max_subscribers_per_topic {
            return Err(EventError::SubscriptionError(
                "Maximum subscribers reached for topic".to_string(),
            ));
        }

        subscriptions.push(Subscription {
            id: subscription_id.clone(),
            filter,
            handler,
            active: true,
        });

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.active_subscriptions += 1;
        }

        Ok(subscription_id)
    }

    /// Unsubscribe from a topic.
    pub async fn unsubscribe(&self, topic: &str, subscription_id: &str) -> Result<()> {
        let mut removed = false;
        
        if let Some(mut subscriptions) = self.subscriptions.get_mut(topic) {
            subscriptions.retain(|sub| {
                if sub.id == subscription_id {
                    removed = true;
                    false
                } else {
                    true
                }
            });
        }

        if removed {
            // Update stats
            let mut stats = self.stats.write().await;
            stats.active_subscriptions = stats.active_subscriptions.saturating_sub(1);
            Ok(())
        } else {
            Err(EventError::SubscriptionError(
                "Subscription not found".to_string(),
            ))
        }
    }

    /// Start processing events for a topic.
    pub async fn start_processing(&self, topic: &str) -> Result<()> {
        // Get or create queue
        let queue = self
            .queues
            .entry(topic.to_string())
            .or_insert_with(|| {
                let (tx, _) = mpsc::channel(self.config.max_queue_size);
                tx
            })
            .clone();

        // Get subscriptions for this topic
        let subscriptions = self
            .subscriptions
            .entry(topic.to_string())
            .or_default()
            .clone();

        // Spawn processing task
        let stats = self.stats.clone();
        let dead_letter_queue = self.dead_letter_queue.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut receiver = queue;
            while let Some(envelope) = receiver.recv().await {
                Self::process_envelope(
                    &envelope,
                    &subscriptions,
                    &stats,
                    &dead_letter_queue,
                    &config,
                ).await;
            }
        });

        Ok(())
    }

    /// Process a single event envelope.
    async fn process_envelope(
        envelope: &EventEnvelope,
        subscriptions: &[Subscription],
        stats: &Arc<RwLock<BusStats>>,
        dead_letter_queue: &Option<mpsc::Sender<DeadLetterEvent>>,
        config: &EventBusConfig,
    ) {
        let mut delivered = false;
        let mut failed = false;

        for subscription in subscriptions {
            if !subscription.active {
                continue;
            }

            if !subscription.filter.matches(&envelope.event) {
                continue;
            }

            delivered = true;

            // Clone handler
            let handler = subscription.handler.clone();
            let envelope_clone = envelope.clone();

            // Spawn handler task with timeout
            let task = tokio::spawn(async move {
                handler.handle(envelope_clone).await
            });

            let result = match timeout(
                Duration::from_secs(config.default_delivery_timeout_secs),
                task,
            )
            .await
            {
                Ok(Ok(result)) => result,
                Ok(Err(join_err)) => HandlerResult::Failure {
                    reason: format!("Handler panicked: {}", join_err),
                },
                Err(_) => HandlerResult::Failure {
                    reason: "Handler timeout".to_string(),
                },
            };

            match result {
                HandlerResult::Success => {
                    // Success - continue to next subscription
                }
                HandlerResult::Retry { delay_seconds, reason } => {
                    // Retry logic would go here
                    tracing::warn!(
                        "Event {} requires retry: {} (delay {}s)",
                        envelope.event.id,
                        reason,
                        delay_seconds
                    );
                    // For simplicity, we treat retry as failure for now
                    failed = true;
                }
                HandlerResult::Failure { reason } => {
                    tracing::error!("Event {} failed: {}", envelope.event.id, reason);
                    failed = true;
                    
                    // Send to dead letter queue if enabled
                    if let Some(dlq) = dead_letter_queue {
                        let dead_event = DeadLetterEvent {
                            event: envelope.clone(),
                            reason,
                            attempts: envelope.delivery_attempt,
                            timestamp: chrono::Utc::now(),
                        };
                        let _ = dlq.send(dead_event).await;
                    }
                }
                HandlerResult::Ignored => {
                    // Event ignored - continue
                }
            }
        }

        // Update statistics
        let mut stats = stats.write().await;
        if delivered {
            stats.total_delivered += 1;
        }
        if failed {
            stats.total_failed += 1;
        }
    }

    /// Get bus statistics.
    pub async fn get_stats(&self) -> Result<crate::types::EventBusStats> {
        let stats = self.stats.read().await;
        
        Ok(crate::types::EventBusStats {
            total_published: stats.total_published,
            total_delivered: stats.total_delivered,
            total_failed: stats.total_failed,
            active_subscriptions: stats.active_subscriptions,
            events_by_type: stats.events_by_type.clone(),
            avg_processing_time_ms: 0.0, // Not tracked in this simple implementation
            last_event_at: None,
        })
    }

    /// Create a stream of events for a topic (for reactive programming).
    pub fn stream(&self, topic: &str, filter: SubscriptionFilter) -> impl Stream<Item = Event> {
        let (tx, rx) = mpsc::channel(100);
        
        // Create a simple handler that forwards events to the stream
        struct StreamHandler {
            sender: mpsc::Sender<Event>,
        }
        
        #[async_trait::async_trait]
        impl EventHandler for StreamHandler {
            async fn handle(&self, envelope: EventEnvelope) -> HandlerResult {
                let _ = self.sender.send(envelope.event).await;
                HandlerResult::Success
            }
        }
        
        let handler = Arc::new(StreamHandler { sender: tx });
        
        // Subscribe (ignore result for simplicity)
        let _ = self.subscribe(topic, filter, handler);
        
        ReceiverStream::new(rx)
    }
}

/// Simple in-memory event bus for testing.
pub struct InMemoryEventBus {
    bus: EventBus,
}

impl InMemoryEventBus {
    /// Create a new in-memory event bus.
    pub fn new() -> Self {
        Self {
            bus: EventBus::new(EventBusConfig::default()),
        }
    }

    /// Get a reference to the underlying bus.
    pub fn bus(&self) -> &EventBus {
        &self.bus
    }
}

impl Default for InMemoryEventBus {
    fn default() -> Self {
        Self::new()
    }
}