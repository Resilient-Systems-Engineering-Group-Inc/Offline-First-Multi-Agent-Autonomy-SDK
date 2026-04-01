//! Eviction policies for cache.

use crate::item::CacheItem;
use std::collections::HashMap;
use std::hash::Hash;

/// Trait for eviction policies.
pub trait EvictionPolicy<K>: Send + Sync {
    /// Called when a key is accessed.
    fn on_access(&mut self, key: &K);
    /// Called when a key is inserted.
    fn on_insert(&mut self, key: K);
    /// Called when a key is removed.
    fn on_remove(&mut self, key: &K);
    /// Choose a key to evict.
    fn choose_for_eviction(&mut self) -> Option<K>;
    /// Reset the policy.
    fn reset(&mut self);
}

/// Least Recently Used (LRU) policy.
pub struct LruPolicy<K> {
    order: lru::LruCache<K, ()>,
    capacity: usize,
}

impl<K: Eq + Hash + Clone> LruPolicy<K> {
    /// Create a new LRU policy with given capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            order: lru::LruCache::new(capacity),
            capacity,
        }
    }
}

impl<K: Eq + Hash + Clone> EvictionPolicy<K> for LruPolicy<K> {
    fn on_access(&mut self, key: &K) {
        let _ = self.order.get(key);
    }

    fn on_insert(&mut self, key: K) {
        self.order.put(key, ());
    }

    fn on_remove(&mut self, key: &K) {
        self.order.pop(key);
    }

    fn choose_for_eviction(&mut self) -> Option<K> {
        self.order.pop_lru().map(|(k, _)| k)
    }

    fn reset(&mut self) {
        self.order.clear();
    }
}

/// Least Frequently Used (LFU) policy (simplified).
pub struct LfuPolicy<K> {
    frequencies: HashMap<K, u64>,
    capacity: usize,
}

impl<K: Eq + Hash + Clone> LfuPolicy<K> {
    /// Create a new LFU policy.
    pub fn new(capacity: usize) -> Self {
        Self {
            frequencies: HashMap::with_capacity(capacity),
            capacity,
        }
    }
}

impl<K: Eq + Hash + Clone> EvictionPolicy<K> for LfuPolicy<K> {
    fn on_access(&mut self, key: &K) {
        *self.frequencies.entry(key.clone()).or_insert(0) += 1;
    }

    fn on_insert(&mut self, key: K) {
        self.frequencies.insert(key, 1);
    }

    fn on_remove(&mut self, key: &K) {
        self.frequencies.remove(key);
    }

    fn choose_for_eviction(&mut self) -> Option<K> {
        if self.frequencies.len() >= self.capacity {
            let min_key = self
                .frequencies
                .iter()
                .min_by_key(|(_, &count)| count)
                .map(|(k, _)| k.clone());
            if let Some(ref key) = min_key {
                self.frequencies.remove(key);
            }
            min_key
        } else {
            None
        }
    }

    fn reset(&mut self) {
        self.frequencies.clear();
    }
}

/// First‑In‑First‑Out (FIFO) policy.
pub struct FifoPolicy<K> {
    queue: std::collections::VecDeque<K>,
    capacity: usize,
}

impl<K: Eq + Hash + Clone> FifoPolicy<K> {
    /// Create a new FIFO policy.
    pub fn new(capacity: usize) -> Self {
        Self {
            queue: std::collections::VecDeque::with_capacity(capacity),
            capacity,
        }
    }
}

impl<K: Eq + Hash + Clone> EvictionPolicy<K> for FifoPolicy<K> {
    fn on_access(&mut self, _key: &K) {
        // FIFO does not care about accesses.
    }

    fn on_insert(&mut self, key: K) {
        self.queue.push_back(key);
    }

    fn on_remove(&mut self, key: &K) {
        self.queue.retain(|k| k != key);
    }

    fn choose_for_eviction(&mut self) -> Option<K> {
        self.queue.pop_front()
    }

    fn reset(&mut self) {
        self.queue.clear();
    }
}

/// Random replacement policy.
pub struct RandomPolicy<K> {
    keys: Vec<K>,
    capacity: usize,
    rng: rand::rngs::ThreadRng,
}

impl<K: Eq + Hash + Clone> RandomPolicy<K> {
    /// Create a new random policy.
    pub fn new(capacity: usize) -> Self {
        Self {
            keys: Vec::with_capacity(capacity),
            capacity,
            rng: rand::thread_rng(),
        }
    }
}

impl<K: Eq + Hash + Clone> EvictionPolicy<K> for RandomPolicy<K> {
    fn on_access(&mut self, _key: &K) {}

    fn on_insert(&mut self, key: K) {
        self.keys.push(key);
    }

    fn on_remove(&mut self, key: &K) {
        self.keys.retain(|k| k != key);
    }

    fn choose_for_eviction(&mut self) -> Option<K> {
        if self.keys.is_empty() {
            None
        } else {
            use rand::seq::SliceRandom;
            let idx = rand::Rng::gen_range(&mut self.rng, 0..self.keys.len());
            Some(self.keys.swap_remove(idx))
        }
    }

    fn reset(&mut self) {
        self.keys.clear();
    }
}

/// Policy that combines size‑based eviction (largest first).
pub struct SizeAwarePolicy<K> {
    items: HashMap<K, usize>, // key -> size
    capacity: usize,
}

impl<K: Eq + Hash + Clone> SizeAwarePolicy<K> {
    /// Create a new size‑aware policy.
    pub fn new(capacity: usize) -> Self {
        Self {
            items: HashMap::with_capacity(capacity),
            capacity,
        }
    }
}

impl<K: Eq + Hash + Clone> EvictionPolicy<K> for SizeAwarePolicy<K> {
    fn on_access(&mut self, _key: &K) {}

    fn on_insert(&mut self, key: K) {
        // Size must be provided separately; we assume 1 for simplicity.
        self.items.insert(key, 1);
    }

    fn on_remove(&mut self, key: &K) {
        self.items.remove(key);
    }

    fn choose_for_eviction(&mut self) -> Option<K> {
        if self.items.len() >= self.capacity {
            let largest_key = self
                .items
                .iter()
                .max_by_key(|(_, &size)| size)
                .map(|(k, _)| k.clone());
            if let Some(ref key) = largest_key {
                self.items.remove(key);
            }
            largest_key
        } else {
            None
        }
    }

    fn reset(&mut self) {
        self.items.clear();
    }
}