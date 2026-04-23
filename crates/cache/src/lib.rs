//! Advanced caching with Redis and in-memory support.
//!
//! Provides:
//! - Multi-level caching (L1 in-memory, L2 Redis)
//! - Cache invalidation strategies
//! - Distributed locking
//! - Cache metrics
//! - TTL management

pub mod store;
pub mod invalidation;
pub mod lock;
pub mod metrics;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::info;

pub use store::*;
pub use invalidation::*;
pub use lock::*;
pub use metrics::*;

/// Cache configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub cache_type: CacheType,
    pub redis_url: String,
    pub key_prefix: String,
    pub default_ttl_secs: u64,
    pub max_memory_mb: usize,
    pub enable_metrics: bool,
    pub enable_local_cache: bool,
    pub local_cache_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CacheType {
    Redis,
    RedisCluster,
    InMemory,
    MultiLevel,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            cache_type: CacheType::MultiLevel,
            redis_url: "redis://localhost:6379".to_string(),
            key_prefix: "sdk:".to_string(),
            default_ttl_secs: 3600,
            max_memory_mb: 512,
            enable_metrics: true,
            enable_local_cache: true,
            local_cache_size: 10000,
        }
    }
}

/// Cache manager.
pub struct CacheManager {
    config: CacheConfig,
    store: CacheStore,
    invalidation: InvalidationStrategy,
    metrics: RwLock<CacheMetrics>,
}

impl CacheManager {
    /// Create new cache manager.
    pub fn new(config: CacheConfig) -> Self {
        let store = CacheStore::new(&config);
        let invalidation = InvalidationStrategy::TTL;

        Self {
            config,
            store,
            invalidation,
            metrics: RwLock::new(CacheMetrics::default()),
        }
    }

    /// Initialize cache.
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing cache with type: {:?}", self.config.cache_type);
        self.store.initialize().await?;
        info!("Cache initialized");
        Ok(())
    }

    /// Get value from cache.
    pub async fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>> {
        let full_key = self.make_key(key);
        let start = std::time::Instant::now();

        let value = self.store.get(&full_key).await?;
        
        let duration = start.elapsed().as_secs_f64() * 1000.0;
        self.update_metrics("get", duration, value.is_some()).await;

        Ok(value)
    }

    /// Set value in cache.
    pub async fn set<T: Serialize>(&self, key: &str, value: &T, ttl_secs: Option<u64>) -> Result<()> {
        let full_key = self.make_key(key);
        let ttl = ttl_secs.unwrap_or(self.config.default_ttl_secs);
        let start = std::time::Instant::now();

        self.store.set(&full_key, value, ttl).await?;
        
        let duration = start.elapsed().as_secs_f64() * 1000.0;
        self.update_metrics("set", duration, true).await;

        info!("Cache set: {} (TTL: {}s)", full_key, ttl);
        Ok(())
    }

    /// Delete value from cache.
    pub async fn delete(&self, key: &str) -> Result<()> {
        let full_key = self.make_key(key);
        let start = std::time::Instant::now();

        self.store.delete(&full_key).await?;
        
        let duration = start.elapsed().as_secs_f64() * 1000.0;
        self.update_metrics("delete", duration, true).await;

        info!("Cache delete: {}", full_key);
        Ok(())
    }

    /// Delete multiple keys by pattern.
    pub async fn delete_pattern(&self, pattern: &str) -> Result<usize> {
        let full_pattern = format!("{}{}", self.config.key_prefix, pattern);
        let count = self.store.delete_pattern(&full_pattern).await?;
        info!("Cache delete pattern: {} ({} keys)", full_pattern, count);
        Ok(count)
    }

    /// Check if key exists.
    pub async fn exists(&self, key: &str) -> Result<bool> {
        let full_key = self.make_key(key);
        self.store.exists(&full_key).await
    }

    /// Get TTL for key.
    pub async fn get_ttl(&self, key: &str) -> Result<Option<i64>> {
        let full_key = self.make_key(key);
        self.store.get_ttl(&full_key).await
    }

    /// Refresh TTL.
    pub async fn refresh_ttl(&self, key: &str, ttl_secs: u64) -> Result<()> {
        let full_key = self.make_key(key);
        self.store.refresh_ttl(&full_key, ttl_secs).await
    }

    /// Increment counter.
    pub async fn increment(&self, key: &str, delta: i64) -> Result<i64> {
        let full_key = self.make_key(key);
        let value = self.store.increment(&full_key, delta).await?;
        info!("Cache increment: {} = {}", full_key, value);
        Ok(value)
    }

    /// Decrement counter.
    pub async fn decrement(&self, key: &str, delta: i64) -> Result<i64> {
        self.increment(key, -delta).await
    }

    /// Get or set with loader function.
    pub async fn get_or_set<T, F>(&self, key: &str, loader: F, ttl_secs: Option<u64>) -> Result<T>
    where
        T: Clone + Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static,
        F: std::future::Future<Output = Result<T>> + Send,
    {
        // Try to get from cache
        if let Some(value) = self.get::<T>(key).await? {
            return Ok(value);
        }

        // Load value
        let value = loader.await?;

        // Set in cache
        self.set(key, &value, ttl_secs).await?;

        Ok(value)
    }

    /// Invalidate cache by strategy.
    pub async fn invalidate(&self, keys: &[&str]) -> Result<()> {
        match self.invalidation {
            InvalidationStrategy::TTL => {
                // TTL-based invalidation is automatic
                info!("TTL-based invalidation for {} keys", keys.len());
            }
            InvalidationStrategy::Manual => {
                for key in keys {
                    self.delete(key).await?;
                }
            }
            InvalidationStrategy::Pattern => {
                for key in keys {
                    self.delete_pattern(key).await?;
                }
            }
        }

        Ok(())
    }

    /// Acquire distributed lock.
    pub async fn acquire_lock(&self, key: &str, ttl_secs: u64) -> Result<DistributedLock> {
        let full_key = self.make_key(&format!("lock:{}", key));
        DistributedLock::acquire(&self.store, &full_key, ttl_secs).await
    }

    /// Get cache statistics.
    pub async fn get_stats(&self) -> Result<CacheStats> {
        let metrics = self.metrics.read().await;
        let store_stats = self.store.get_stats().await?;

        Ok(CacheStats {
            cache_type: format!("{:?}", self.config.cache_type),
            hits: metrics.hits,
            misses: metrics.misses,
            hit_rate: if metrics.hits + metrics.misses > 0 {
                metrics.hits as f64 / (metrics.hits + metrics.misses) as f64
            } else {
                0.0
            },
            avg_get_latency_ms: metrics.avg_get_latency_ms,
            avg_set_latency_ms: metrics.avg_set_latency_ms,
            total_keys: store_stats.total_keys,
            memory_used_mb: store_stats.memory_used_mb,
        })
    }

    /// Clear all cache.
    pub async fn clear(&self) -> Result<()> {
        self.store.clear().await?;
        info!("Cache cleared");
        Ok(())
    }

    fn make_key(&self, key: &str) -> String {
        format!("{}{}", self.config.key_prefix, key)
    }

    async fn update_metrics(&self, operation: &str, latency_ms: f64, hit: bool) {
        let mut metrics = self.metrics.write().await;
        
        match operation {
            "get" => {
                if hit {
                    metrics.hits += 1;
                } else {
                    metrics.misses += 1;
                }
                metrics.update_get_latency(latency_ms);
            }
            "set" => {
                metrics.sets += 1;
                metrics.update_set_latency(latency_ms);
            }
            "delete" => {
                metrics.deletes += 1;
            }
            _ => {}
        }
    }
}

/// Cache statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub cache_type: String,
    pub hits: i64,
    pub misses: i64,
    pub hit_rate: f64,
    pub avg_get_latency_ms: f64,
    pub avg_set_latency_ms: f64,
    pub total_keys: i64,
    pub memory_used_mb: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_manager() {
        let config = CacheConfig {
            cache_type: CacheType::InMemory,
            ..Default::default()
        };
        let manager = CacheManager::new(config);

        // Initialize
        manager.initialize().await.unwrap();

        // Set value
        manager.set("test-key", &"test-value", Some(60)).await.unwrap();

        // Get value
        let value: Option<String> = manager.get("test-key").await.unwrap();
        assert_eq!(value, Some("test-value".to_string()));

        // Check exists
        assert!(manager.exists("test-key").await.unwrap());

        // Get TTL
        let ttl = manager.get_ttl("test-key").await.unwrap();
        assert!(ttl.unwrap() > 0);

        // Delete
        manager.delete("test-key").await.unwrap();
        assert!(!manager.exists("test-key").await.unwrap());

        // Get stats
        let stats = manager.get_stats().await.unwrap();
        assert!(stats.hits >= 1);
    }

    #[tokio::test]
    async fn test_counter() {
        let config = CacheConfig {
            cache_type: CacheType::InMemory,
            ..Default::default()
        };
        let manager = CacheManager::new(config);
        manager.initialize().await.unwrap();

        // Increment
        let val1 = manager.increment("counter", 5).await.unwrap();
        assert_eq!(val1, 5);

        let val2 = manager.increment("counter", 3).await.unwrap();
        assert_eq!(val2, 8);

        // Decrement
        let val3 = manager.decrement("counter", 2).await.unwrap();
        assert_eq!(val3, 6);
    }
}
