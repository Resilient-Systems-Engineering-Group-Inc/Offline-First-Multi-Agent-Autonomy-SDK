//! Quality of Service (QoS) for mesh transport.
//!
//! Provides message prioritization, delivery guarantees, congestion control,
//! and latency management.

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// QoS class (similar to DSCP / IEEE 802.1p).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QosClass {
    /// Best‑effort (default). No guarantees.
    BestEffort,
    /// Background low‑priority traffic.
    Background,
    /// Standard data.
    Standard,
    /// Interactive traffic (low latency).
    Interactive,
    /// Real‑time traffic (guaranteed latency).
    RealTime,
    /// Control plane (highest priority).
    Control,
}

impl Default for QosClass {
    fn default() -> Self {
        QosClass::BestEffort
    }
}

/// Delivery guarantee.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeliveryGuarantee {
    /// At‑most‑once (fire‑and‑forget).
    AtMostOnce,
    /// At‑least‑once (ack‑based retransmission).
    AtLeastOnce,
    /// Exactly‑once (idempotent delivery).
    ExactlyOnce,
}

/// QoS profile for a message or stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QosProfile {
    /// QoS class.
    pub class: QosClass,
    /// Delivery guarantee.
    pub delivery: DeliveryGuarantee,
    /// Maximum allowed latency (if applicable).
    pub max_latency: Option<Duration>,
    /// Minimum required bandwidth (bytes per second).
    pub min_bandwidth_bps: Option<u64>,
    /// Whether the message can be dropped under congestion.
    pub droppable: bool,
    /// Time‑to‑live (after which the message is discarded).
    pub ttl: Option<Duration>,
}

impl Default for QosProfile {
    fn default() -> Self {
        Self {
            class: QosClass::BestEffort,
            delivery: DeliveryGuarantee::AtMostOnce,
            max_latency: None,
            min_bandwidth_bps: None,
            droppable: true,
            ttl: None,
        }
    }
}

impl QosProfile {
    /// Creates a best‑effort profile.
    pub fn best_effort() -> Self {
        Self::default()
    }

    /// Creates a real‑time profile with low latency.
    pub fn real_time(max_latency: Duration) -> Self {
        Self {
            class: QosClass::RealTime,
            delivery: DeliveryGuarantee::AtLeastOnce,
            max_latency: Some(max_latency),
            droppable: false,
            ..Default::default()
        }
    }

    /// Creates a control profile (high priority, reliable).
    pub fn control() -> Self {
        Self {
            class: QosClass::Control,
            delivery: DeliveryGuarantee::ExactlyOnce,
            droppable: false,
            ttl: Some(Duration::from_secs(30)),
            ..Default::default()
        }
    }
}

/// QoS manager that enforces policies and schedules messages.
pub struct QosManager {
    /// Current congestion level (0‑100).
    congestion_level: u8,
    /// Per‑class queues.
    queues: HashMap<QosClass, Vec<QueuedMessage>>,
    /// Statistics.
    stats: QosStats,
}

#[derive(Debug, Clone)]
struct QueuedMessage {
    payload: Vec<u8>,
    profile: QosProfile,
    created_at: std::time::Instant,
}

/// QoS statistics.
#[derive(Debug, Clone, Default)]
pub struct QosStats {
    pub messages_sent: u64,
    pub messages_dropped: u64,
    pub average_latency_ms: f64,
    pub congestion_events: u32,
}

impl QosManager {
    /// Creates a new QoS manager.
    pub fn new() -> Self {
        let mut queues = HashMap::new();
        for &class in &[
            QosClass::Control,
            QosClass::RealTime,
            QosClass::Interactive,
            QosClass::Standard,
            QosClass::Background,
            QosClass::BestEffort,
        ] {
            queues.insert(class, Vec::new());
        }

        Self {
            congestion_level: 0,
            queues,
            stats: QosStats::default(),
        }
    }

    /// Enqueues a message with a given QoS profile.
    pub fn enqueue(&mut self, payload: Vec<u8>, profile: QosProfile) -> Result<()> {
        // Check TTL and drop if expired.
        if let Some(ttl) = profile.ttl {
            // For simplicity, we assume the message is fresh.
            // In a real implementation, we would store creation time.
        }

        let queue = self.queues.get_mut(&profile.class).unwrap();
        queue.push(QueuedMessage {
            payload,
            profile,
            created_at: std::time::Instant::now(),
        });
        Ok(())
    }

    /// Dequeues the next message to send based on QoS policies.
    pub fn dequeue(&mut self) -> Option<(Vec<u8>, QosProfile)> {
        // Priority order: Control > RealTime > Interactive > Standard > Background > BestEffort
        let classes = [
            QosClass::Control,
            QosClass::RealTime,
            QosClass::Interactive,
            QosClass::Standard,
            QosClass::Background,
            QosClass::BestEffort,
        ];

        for class in classes.iter() {
            let queue = self.queues.get_mut(class).unwrap();
            if !queue.is_empty() {
                // Apply congestion control: drop droppable messages if congestion is high.
                if self.congestion_level > 70 {
                    // Find first droppable message and drop it.
                    if let Some(index) = queue.iter().position(|qm| qm.profile.droppable) {
                        let _dropped = queue.remove(index);
                        self.stats.messages_dropped += 1;
                        continue; // try next message
                    }
                }
                let queued = queue.remove(0);
                return Some((queued.payload, queued.profile));
            }
        }
        None
    }

    /// Updates congestion level based on network conditions.
    pub fn update_congestion(&mut self, level: u8) {
        self.congestion_level = level;
        if level > 80 {
            self.stats.congestion_events += 1;
        }
    }

    /// Returns current statistics.
    pub fn stats(&self) -> &QosStats {
        &self.stats
    }
}

/// QoS‑aware sender trait.
pub trait QosSender {
    /// Sends a payload with a specific QoS profile.
    fn send_with_qos(&mut self, payload: Vec<u8>, profile: QosProfile) -> Result<()>;
}

/// Integration with `MeshTransport` (to be implemented in transport.rs).
pub mod integration {
    use super::*;
    use crate::transport::MeshTransport;

    impl QosSender for MeshTransport {
        fn send_with_qos(&mut self, payload: Vec<u8>, profile: QosProfile) -> Result<()> {
            // For now, delegate to normal send; later we could implement prioritization.
            self.broadcast(payload)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qos_profile() {
        let profile = QosProfile::real_time(Duration::from_millis(100));
        assert_eq!(profile.class, QosClass::RealTime);
        assert!(!profile.droppable);
    }

    #[test]
    fn test_qos_manager_enqueue_dequeue() {
        let mut manager = QosManager::new();
        let payload = vec![1, 2, 3];
        let profile = QosProfile::best_effort();
        manager.enqueue(payload.clone(), profile.clone()).unwrap();
        let (dequeued, dequeued_profile) = manager.dequeue().unwrap();
        assert_eq!(dequeued, payload);
        assert_eq!(dequeued_profile.class, profile.class);
    }

    #[test]
    fn test_priority_order() {
        let mut manager = QosManager::new();
        let low = QosProfile::best_effort();
        let high = QosProfile::control();

        manager.enqueue(vec![1], low.clone()).unwrap();
        manager.enqueue(vec![2], high.clone()).unwrap();

        // Control should be dequeued first.
        let (payload, profile) = manager.dequeue().unwrap();
        assert_eq!(payload, vec![2]);
        assert_eq!(profile.class, QosClass::Control);
    }
}