//! Cache item and metadata.

use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

/// A cached value with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheItem<V> {
    /// The actual value.
    pub value: V,
    /// Time‑to‑live in seconds (0 = infinite).
    pub ttl_secs: u64,
    /// When this item was created (Unix timestamp).
    pub created_at: u64,
    /// When this item was last accessed (Unix timestamp).
    pub last_accessed: u64,
    /// Access count.
    pub access_count: u64,
    /// Size estimate in bytes.
    pub size_bytes: usize,
}

impl<V> CacheItem<V> {
    /// Create a new cache item.
    pub fn new(value: V, ttl_secs: u64, size_bytes: usize) -> Self {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            value,
            ttl_secs,
            created_at: now,
            last_accessed: now,
            access_count: 0,
            size_bytes,
        }
    }

    /// Check if the item has expired.
    pub fn is_expired(&self) -> bool {
        if self.ttl_secs == 0 {
            return false;
        }
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now >= self.created_at + self.ttl_secs
    }

    /// Update last accessed timestamp and increment count.
    pub fn record_access(&mut self) {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.last_accessed = now;
        self.access_count += 1;
    }

    /// Get remaining time to live in seconds.
    pub fn remaining_ttl(&self) -> Option<Duration> {
        if self.ttl_secs == 0 {
            return None;
        }
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        if now >= self.created_at + self.ttl_secs {
            Some(Duration::from_secs(0))
        } else {
            Some(Duration::from_secs(self.created_at + self.ttl_secs - now))
        }
    }
}

/// Cache statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheStats {
    /// Total number of items currently in cache.
    pub items_count: usize,
    /// Total size in bytes.
    pub total_size_bytes: usize,
    /// Number of hits.
    pub hits: u64,
    /// Number of misses.
    pub misses: u64,
    /// Number of evictions.
    pub evictions: u64,
    /// Number of expired items removed.
    pub expired_removals: u64,
}

impl CacheStats {
    /// Compute hit rate.
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}