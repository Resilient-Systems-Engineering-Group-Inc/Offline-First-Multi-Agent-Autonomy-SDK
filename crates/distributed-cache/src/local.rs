//! Local in‑memory cache.

use crate::error::{CacheError, Result};
use crate::item::{CacheItem, CacheStats};
use crate::policy::EvictionPolicy;
use dashmap::DashMap;
use std::hash::Hash;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Local cache with configurable eviction policy.
pub struct LocalCache<K, V, P>
where
    K: Eq + Hash + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
    P: EvictionPolicy<K> + 'static,
{
    store: DashMap<K, CacheItem<V>>,
    policy: RwLock<P>,
    stats: Arc<CacheStatsInternal>,
    max_items: usize,
    max_size_bytes: usize,
}

struct CacheStatsInternal {
    hits: AtomicU64,
    misses: AtomicU64,
    evictions: AtomicU64,
    expired_removals: AtomicU64,
    total_size_bytes: AtomicU64,
}

impl<K, V, P> LocalCache<K, V, P>
where
    K: Eq + Hash + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
    P: EvictionPolicy<K> + 'static,
{
    /// Create a new local cache with the given policy and limits.
    pub fn new(policy: P, max_items: usize, max_size_bytes: usize) -> Self {
        Self {
            store: DashMap::with_capacity(max_items),
            policy: RwLock::new(policy),
            stats: Arc::new(CacheStatsInternal {
                hits: AtomicU64::new(0),
                misses: AtomicU64::new(0),
                evictions: AtomicU64::new(0),
                expired_removals: AtomicU64::new(0),
                total_size_bytes: AtomicU64::new(0),
            }),
            max_items,
            max_size_bytes,
        }
    }

    /// Get a value from the cache.
    pub async fn get(&self, key: &K) -> Result<Option<V>> {
        let mut policy = self.policy.write().await;
        match self.store.get_mut(key) {
            Some(mut entry) => {
                let item = entry.value_mut();
                if item.is_expired() {
                    // Remove expired item
                    drop(entry);
                    self.store.remove(key);
                    policy.on_remove(key);
                    self.stats.expired_removals.fetch_add(1, Ordering::Relaxed);
                    self.stats.misses.fetch_add(1, Ordering::Relaxed);
                    return Ok(None);
                }
                item.record_access();
                policy.on_access(key);
                self.stats.hits.fetch_add(1, Ordering::Relaxed);
                Ok(Some(item.value.clone()))
            }
            None => {
                self.stats.misses.fetch_add(1, Ordering::Relaxed);
                Ok(None)
            }
        }
    }

    /// Insert a value into the cache.
    pub async fn insert(&self, key: K, value: V, ttl_secs: u64, size_bytes: usize) -> Result<()> {
        // Evict if needed
        self.evict_if_needed(size_bytes).await?;

        let item = CacheItem::new(value, ttl_secs, size_bytes);
        let old = self.store.insert(key.clone(), item);
        let mut policy = self.policy.write().await;
        if old.is_none() {
            policy.on_insert(key.clone());
            self.stats
                .total_size_bytes
                .fetch_add(size_bytes as u64, Ordering::Relaxed);
        } else {
            // Replacement: size change? (simplified)
        }
        Ok(())
    }

    /// Remove a key from the cache.
    pub async fn remove(&self, key: &K) -> Result<Option<V>> {
        let mut policy = self.policy.write().await;
        if let Some((_, item)) = self.store.remove(key) {
            policy.on_remove(key);
            self.stats
                .total_size_bytes
                .fetch_sub(item.size_bytes as u64, Ordering::Relaxed);
            Ok(Some(item.value))
        } else {
            Ok(None)
        }
    }

    /// Check if key exists (and not expired).
    pub async fn contains_key(&self, key: &K) -> bool {
        match self.store.get(key) {
            Some(item) => !item.is_expired(),
            None => false,
        }
    }

    /// Clear the cache.
    pub async fn clear(&self) {
        self.store.clear();
        self.policy.write().await.reset();
        self.stats.hits.store(0, Ordering::Relaxed);
        self.stats.misses.store(0, Ordering::Relaxed);
        self.stats.evictions.store(0, Ordering::Relaxed);
        self.stats.expired_removals.store(0, Ordering::Relaxed);
        self.stats.total_size_bytes.store(0, Ordering::Relaxed);
    }

    /// Get cache statistics.
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            items_count: self.store.len(),
            total_size_bytes: self.stats.total_size_bytes.load(Ordering::Relaxed) as usize,
            hits: self.stats.hits.load(Ordering::Relaxed),
            misses: self.stats.misses.load(Ordering::Relaxed),
            evictions: self.stats.evictions.load(Ordering::Relaxed),
            expired_removals: self.stats.expired_removals.load(Ordering::Relaxed),
        }
    }

    /// Evict items if capacity limits are exceeded.
    async fn evict_if_needed(&self, incoming_size: usize) -> Result<()> {
        let current_items = self.store.len();
        let current_size = self.stats.total_size_bytes.load(Ordering::Relaxed) as usize;

        if current_items >= self.max_items || current_size + incoming_size > self.max_size_bytes {
            let mut policy = self.policy.write().await;
            while current_items >= self.max_items
                || current_size + incoming_size > self.max_size_bytes
            {
                if let Some(key) = policy.choose_for_eviction() {
                    if let Some((_, item)) = self.store.remove(&key) {
                        self.stats
                            .total_size_bytes
                            .fetch_sub(item.size_bytes as u64, Ordering::Relaxed);
                        self.stats.evictions.fetch_add(1, Ordering::Relaxed);
                    }
                } else {
                    return Err(CacheError::Full);
                }
            }
        }
        Ok(())
    }
}