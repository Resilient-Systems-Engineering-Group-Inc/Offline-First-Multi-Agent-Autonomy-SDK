//! Watermark handling for event time processing.

use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicI64, Ordering};
use tokio::sync::RwLock;

/// Watermark - indicates progress of event time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Watermark {
    pub timestamp: DateTime<Utc>,
    pub value: i64,
}

impl Watermark {
    pub fn new(timestamp: DateTime<Utc>) -> Self {
        Self {
            timestamp,
            value: timestamp.timestamp_millis(),
        }
    }
}

/// Watermark strategy.
pub trait WatermarkStrategy: Send + Sync {
    fn assign_watermark(&self, event_time: i64) -> i64;
}

/// Bounded out-of-orderness strategy.
pub struct BoundedOutOfOrderness {
    max_out_of_orderness_millis: i64,
}

impl BoundedOutOfOrderness {
    pub fn new(max_out_of_orderness_secs: u64) -> Self {
        Self {
            max_out_of_orderness_millis: (max_out_of_orderness_secs * 1000) as i64,
        }
    }
}

impl WatermarkStrategy for BoundedOutOfOrderness {
    fn assign_watermark(&self, event_time: i64) -> i64 {
        event_time - self.max_out_of_orderness_millis
    }
}

/// Watermark generator.
pub struct WatermarkGenerator<S: WatermarkStrategy> {
    strategy: S,
    current_watermark: AtomicI64,
    idle_timeout_millis: i64,
    last_event_time: AtomicI64,
}

impl<S: WatermarkStrategy> WatermarkGenerator<S> {
    pub fn new(strategy: S, idle_timeout_secs: u64) -> Self {
        Self {
            strategy,
            current_watermark: AtomicI64::new(0),
            idle_timeout_millis: (idle_timeout_secs * 1000) as i64,
            last_event_time: AtomicI64::new(0),
        }
    }

    pub fn observe_timestamp(&self, event_time: i64) -> Option<i64> {
        self.last_event_time.store(event_time, Ordering::SeqCst);
        
        let new_watermark = self.strategy.assign_watermark(event_time);
        let old_watermark = self.current_watermark.load(Ordering::SeqCst);

        if new_watermark > old_watermark {
            self.current_watermark.store(new_watermark, Ordering::SeqCst);
            Some(new_watermark)
        } else {
            None
        }
    }

    pub fn get_current_watermark(&self) -> i64 {
        self.current_watermark.load(Ordering::SeqCst)
    }

    pub fn is_idle(&self, current_time: i64) -> bool {
        let last_event = self.last_event_time.load(Ordering::SeqCst);
        current_time - last_event > self.idle_timeout_millis
    }

    pub fn emit_idle_watermark(&self, current_time: i64) -> Option<i64> {
        if self.is_idle(current_time) {
            let new_watermark = current_time - self.idle_timeout_millis;
            let old_watermark = self.current_watermark.load(Ordering::SeqCst);
            
            if new_watermark > old_watermark {
                self.current_watermark.store(new_watermark, Ordering::SeqCst);
                Some(new_watermark)
            } else {
                None
            }
        } else {
            None
        }
    }
}

/// Late event handler.
pub struct LateEventHandler {
    late_threshold_millis: i64,
    late_events: RwLock<Vec<LateEvent>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LateEvent {
    pub event_id: String,
    pub event_time: i64,
    pub watermark_at_arrival: i64,
    pub lateness: i64,
    pub handled_at: DateTime<Utc>,
}

impl LateEventHandler {
    pub fn new(late_threshold_secs: u64) -> Self {
        Self {
            late_threshold_millis: (late_threshold_secs * 1000) as i64,
            late_events: RwLock::new(Vec::new()),
        }
    }

    pub fn is_late(&self, event_time: i64, watermark: i64) -> bool {
        event_time < watermark - self.late_threshold_millis
    }

    pub fn handle_late_event(&self, event_id: &str, event_time: i64, watermark: i64) -> Result<()> {
        let lateness = watermark - event_time;
        
        let late_event = LateEvent {
            event_id: event_id.to_string(),
            event_time,
            watermark_at_arrival: watermark,
            lateness,
            handled_at: Utc::now(),
        };

        futures::executor::block_on(async {
            self.late_events.write().await.push(late_event);
        });

        Ok(())
    }

    pub async fn get_late_events(&self) -> Vec<LateEvent> {
        self.late_events.read().await.clone()
    }

    pub async fn clear_late_events(&self) {
        self.late_events.write().await.clear();
    }

    pub fn get_late_count(&self) -> usize {
        futures::executor::block_on(async {
            self.late_events.read().await.len()
        })
    }
}

/// Event time handler.
pub struct EventTimeHandler {
    watermark_generator: RwLock<Option<WatermarkGenerator<BoundedOutOfOrderness>>>,
    late_handler: LateEventHandler,
    allowed_lateness_millis: i64,
}

impl EventTimeHandler {
    pub fn new(max_out_of_orderness_secs: u64, allowed_lateness_secs: u64) -> Self {
        let watermark_gen = WatermarkGenerator::new(
            BoundedOutOfOrderness::new(max_out_of_orderness_secs),
            max_out_of_orderness_secs * 2,
        );

        Self {
            watermark_generator: RwLock::new(Some(watermark_gen)),
            late_handler: LateEventHandler::new(allowed_lateness_secs),
            allowed_lateness_millis: (allowed_lateness_secs * 1000) as i64,
        }
    }

    pub fn process_event(&self, event_id: &str, event_time: i64) -> Result<EventTiming> {
        let gen = self.watermark_generator.read().await;
        let gen = gen.as_ref().ok_or_else(|| anyhow::anyhow!("Watermark generator not initialized"))?;

        let current_watermark = gen.get_current_watermark();
        
        // Check if event is late
        if self.late_handler.is_late(event_time, current_watermark) {
            // Check if within allowed lateness
            if event_time >= current_watermark - self.allowed_lateness_millis {
                self.late_handler.handle_late_event(event_id, event_time, current_watermark)?;
                return Ok(EventTiming::Late);
            } else {
                return Ok(EventTiming::Dropped);
            }
        }

        // Update watermark
        if let Some(new_watermark) = gen.observe_timestamp(event_time) {
            Ok(EventTiming::OnTimeWithWatermark(new_watermark))
        } else {
            Ok(EventTiming::OnTime)
        }
    }

    pub fn get_watermark(&self) -> i64 {
        futures::executor::block_on(async {
            let gen = self.watermark_generator.read().await;
            gen.as_ref().map(|g| g.get_current_watermark()).unwrap_or(0)
        })
    }

    pub fn get_late_count(&self) -> usize {
        self.late_handler.get_late_count()
    }
}

/// Event timing result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventTiming {
    OnTime,
    OnTimeWithWatermark(i64),
    Late,
    Dropped,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watermark_generator() {
        let strategy = BoundedOutOfOrderness::new(5); // 5 seconds
        let gen = WatermarkGenerator::new(strategy, 10);

        let event_time = 1000000; // milliseconds
        let new_watermark = gen.observe_timestamp(event_time);
        
        assert!(new_watermark.is_some());
        assert!(new_watermark.unwrap() < event_time);
    }

    #[test]
    fn test_late_event_handler() {
        let handler = LateEventHandler::new(10); // 10 seconds threshold

        let watermark = 1000000;
        let event_time = watermark - 15000; // 15 seconds late

        assert!(handler.is_late(event_time, watermark));
        
        handler.handle_late_event("event-1", event_time, watermark).unwrap();
        assert_eq!(handler.get_late_count(), 1);
    }

    #[tokio::test]
    async fn test_event_time_handler() {
        let handler = EventTimeHandler::new(5, 10);

        let event_time = 1000000;
        let timing = handler.process_event("event-1", event_time).unwrap();
        
        assert!(matches!(timing, EventTiming::OnTimeWithWatermark(_)));

        // Late event
        let late_time = handler.get_watermark() - 8000; // 8 seconds late, but within allowed
        let timing = handler.process_event("event-2", late_time).unwrap();
        
        assert!(matches!(timing, EventTiming::Late));
    }
}
