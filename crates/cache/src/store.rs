//! Cache store implementations.

use crate::CacheConfig;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Cache store trait.
#[async_trait::async_trait]
pub trait CacheStoreTrait: Send + Sync {
    async fn initialize(&self) -> Result<()>;
    async fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>>;
    async fn set<T: Serialize>(&self, key: &str, value: &T, ttl_secs: u64) -> Result<()>;
    async fn delete(&self, key: &str) -> Result<()>;
    async fn delete_pattern(&self, pattern: &str) -> Result<usize>;
    async fn exists(&self, key: &str) -> Result<bool>;
    async fn get_ttl(&self, key: &str) -> Result<Option<i64>>;
    async fn refresh_ttl(&self, key: &str, ttl_secs: u64) -> Result<()>;
    async fn increment(&self, key: &str, delta: i64) -> Result<i64>;
    async fn get_stats(&self) -> Result<CacheStoreStats>;
    async fn clear(&self) -> Result<()>;
}

/// Cache store enum.
pub enum CacheStore {
    InMemory(Arc<InMemoryStore>),
    Redis(Arc<RedisStore>),
    MultiLevel(Arc<MultiLevelStore>),
}

impl CacheStore {
    pub fn new(config: &CacheConfig) -> Self {
        match config.cache_type {
            crate::CacheType::InMemory => {
                Self::InMemory(Arc::new(InMemoryStore::new(config.local_cache_size)))
            }
            crate::CacheType::Redis => {
                Self::Redis(Arc::new(RedisStore::new(&config.redis_url)))
            }
            crate::CacheType::MultiLevel => {
                Self::MultiLevel(Arc::new(MultiLevelStore::new(config)))
            }
            crate::CacheType::RedisCluster => {
                Self::Redis(Arc::new(RedisStore::new(&config.redis_url)))
            }
        }
    }

    pub async fn initialize(&self) -> Result<()> {
        match self {
            Self::InMemory(store) => store.initialize().await,
            Self::Redis(store) => store.initialize().await,
            Self::MultiLevel(store) => store.initialize().await,
        }
    }

    pub async fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>> {
        match self {
            Self::InMemory(store) => store.get(key).await,
            Self::Redis(store) => store.get(key).await,
            Self::MultiLevel(store) => store.get(key).await,
        }
    }

    pub async fn set<T: Serialize>(&self, key: &str, value: &T, ttl_secs: u64) -> Result<()> {
        match self {
            Self::InMemory(store) => store.set(key, value, ttl_secs).await,
            Self::Redis(store) => store.set(key, value, ttl_secs).await,
            Self::MultiLevel(store) => store.set(key, value, ttl_secs).await,
        }
    }

    pub async fn delete(&self, key: &str) -> Result<()> {
        match self {
            Self::InMemory(store) => store.delete(key).await,
            Self::Redis(store) => store.delete(key).await,
            Self::MultiLevel(store) => store.delete(key).await,
        }
    }

    pub async fn delete_pattern(&self, pattern: &str) -> Result<usize> {
        match self {
            Self::InMemory(store) => store.delete_pattern(pattern).await,
            Self::Redis(store) => store.delete_pattern(pattern).await,
            Self::MultiLevel(store) => store.delete_pattern(pattern).await,
        }
    }

    pub async fn exists(&self, key: &str) -> Result<bool> {
        match self {
            Self::InMemory(store) => store.exists(key).await,
            Self::Redis(store) => store.exists(key).await,
            Self::MultiLevel(store) => store.exists(key).await,
        }
    }

    pub async fn get_ttl(&self, key: &str) -> Result<Option<i64>> {
        match self {
            Self::InMemory(store) => store.get_ttl(key).await,
            Self::Redis(store) => store.get_ttl(key).await,
            Self::MultiLevel(store) => store.get_ttl(key).await,
        }
    }

    pub async fn refresh_ttl(&self, key: &str, ttl_secs: u64) -> Result<()> {
        match self {
            Self::InMemory(store) => store.refresh_ttl(key, ttl_secs).await,
            Self::Redis(store) => store.refresh_ttl(key, ttl_secs).await,
            Self::MultiLevel(store) => store.refresh_ttl(key, ttl_secs).await,
        }
    }

    pub async fn increment(&self, key: &str, delta: i64) -> Result<i64> {
        match self {
            Self::InMemory(store) => store.increment(key, delta).await,
            Self::Redis(store) => store.increment(key, delta).await,
            Self::MultiLevel(store) => store.increment(key, delta).await,
        }
    }

    pub async fn get_stats(&self) -> Result<CacheStoreStats> {
        match self {
            Self::InMemory(store) => store.get_stats().await,
            Self::Redis(store) => store.get_stats().await,
            Self::MultiLevel(store) => store.get_stats().await,
        }
    }

    pub async fn clear(&self) -> Result<()> {
        match self {
            Self::InMemory(store) => store.clear().await,
            Self::Redis(store) => store.clear().await,
            Self::MultiLevel(store) => store.clear().await,
        }
    }
}

/// In-memory cache store.
pub struct InMemoryStore {
    data: RwLock<lru::LruCache<String, CacheEntry>>,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    value: serde_json::Value,
    expires_at: Option<i64>,
}

impl InMemoryStore {
    pub fn new(size: usize) -> Self {
        Self {
            data: RwLock::new(lru::LruCache::new(
                std::num::NonZeroUsize::new(size).unwrap()
            )),
        }
    }
}

#[async_trait::async_trait]
impl CacheStoreTrait for InMemoryStore {
    async fn initialize(&self) -> Result<()> {
        Ok(())
    }

    async fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>> {
        let mut cache = self.data.write().await;
        
        if let Some(entry) = cache.get(key) {
            // Check expiration
            if let Some(expires) = entry.expires_at {
                if chrono::Utc::now().timestamp() > expires {
                    cache.pop(key);
                    return Ok(None);
                }
            }
            return Ok(serde_json::from_value(entry.value.clone())?);
        }

        Ok(None)
    }

    async fn set<T: Serialize>(&self, key: &str, value: &T, ttl_secs: u64) -> Result<()> {
        let expires_at = if ttl_secs > 0 {
            Some(chrono::Utc::now().timestamp() + ttl_secs as i64)
        } else {
            None
        };

        let entry = CacheEntry {
            value: serde_json::to_value(value)?,
            expires_at,
        };

        let mut cache = self.data.write().await;
        cache.put(key.to_string(), entry);

        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let mut cache = self.data.write().await;
        cache.pop(key);
        Ok(())
    }

    async fn delete_pattern(&self, pattern: &str) -> Result<usize> {
        let mut cache = self.data.write().await;
        let keys_to_delete: Vec<_> = cache.iter()
            .filter(|(k, _)| k.contains(pattern))
            .map(|(k, _)| k.clone())
            .collect();

        for key in &keys_to_delete {
            cache.pop(key);
        }

        Ok(keys_to_delete.len())
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        let cache = self.data.read().await;
        Ok(cache.contains(key))
    }

    async fn get_ttl(&self, key: &str) -> Result<Option<i64>> {
        let cache = self.data.read().await;
        
        if let Some(entry) = cache.peek(key) {
            if let Some(expires) = entry.expires_at {
                let ttl = expires - chrono::Utc::now().timestamp();
                return Ok(Some(ttl.max(0)));
            }
        }

        Ok(None)
    }

    async fn refresh_ttl(&self, key: &str, ttl_secs: u64) -> Result<()> {
        let mut cache = self.data.write().await;
        
        if let Some(entry) = cache.get_mut(key) {
            entry.expires_at = Some(chrono::Utc::now().timestamp() + ttl_secs as i64);
        }

        Ok(())
    }

    async fn increment(&self, key: &str, delta: i64) -> Result<i64> {
        let mut cache = self.data.write().await;
        
        let value = if let Some(entry) = cache.get_mut(key) {
            let current = entry.value.as_i64().unwrap_or(0);
            let new_value = current + delta;
            entry.value = serde_json::json!(new_value);
            new_value
        } else {
            let new_value = delta;
            cache.put(key.to_string(), CacheEntry {
                value: serde_json::json!(new_value),
                expires_at: None,
            });
            new_value
        };

        Ok(value)
    }

    async fn get_stats(&self) -> Result<CacheStoreStats> {
        let cache = self.data.read().await;
        Ok(CacheStoreStats {
            total_keys: cache.len() as i64,
            memory_used_mb: 0.0, // Would calculate actual memory
        })
    }

    async fn clear(&self) -> Result<()> {
        let mut cache = self.data.write().await;
        cache.clear();
        Ok(())
    }
}

/// Redis cache store.
pub struct RedisStore {
    url: String,
    client: RwLock<Option<redis::Client>>,
}

impl RedisStore {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            client: RwLock::new(None),
        }
    }
}

#[async_trait::async_trait]
impl CacheStoreTrait for RedisStore {
    async fn initialize(&self) -> Result<()> {
        let client = redis::Client::open(self.url.as_str())?;
        *self.client.write().await = Some(client);
        Ok(())
    }

    async fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>> {
        let client = self.client.read().await;
        let client = client.as_ref().ok_or_else(|| anyhow::anyhow!("Redis not initialized"))?;

        let mut conn = client.get_async_connection().await?;
        let value: Option<String> = redis::cmd("GET").arg(key).query_async(&mut conn).await?;
        
        match value {
            Some(v) => Ok(Some(serde_json::from_str(&v)?)),
            None => Ok(None),
        }
    }

    async fn set<T: Serialize>(&self, key: &str, value: &T, ttl_secs: u64) -> Result<()> {
        let client = self.client.read().await;
        let client = client.as_ref().ok_or_else(|| anyhow::anyhow!("Redis not initialized"))?;

        let mut conn = client.get_async_connection().await?;
        let json = serde_json::to_string(value)?;

        if ttl_secs > 0 {
            redis::cmd("SETEX").arg(key).arg(ttl_secs).arg(json).query_async(&mut conn).await?;
        } else {
            redis::cmd("SET").arg(key).arg(json).query_async(&mut conn).await?;
        }

        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let client = self.client.read().await;
        let client = client.as_ref().ok_or_else(|| anyhow::anyhow!("Redis not initialized"))?;

        let mut conn = client.get_async_connection().await?;
        redis::cmd("DEL").arg(key).query_async(&mut conn).await?;

        Ok(())
    }

    async fn delete_pattern(&self, pattern: &str) -> Result<usize> {
        let client = self.client.read().await;
        let client = client.as_ref().ok_or_else(|| anyhow::anyhow!("Redis not initialized"))?;

        let mut conn = client.get_async_connection().await?;
        let keys: Vec<String> = redis::cmd("KEYS").arg(pattern).query_async(&mut conn).await?;

        if keys.is_empty() {
            return Ok(0);
        }

        let count = keys.len();
        redis::cmd("DEL").arg(&keys).query_async(&mut conn).await?;

        Ok(count)
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        let client = self.client.read().await;
        let client = client.as_ref().ok_or_else(|| anyhow::anyhow!("Redis not initialized"))?;

        let mut conn = client.get_async_connection().await?;
        let exists: i32 = redis::cmd("EXISTS").arg(key).query_async(&mut conn).await?;
        
        Ok(exists > 0)
    }

    async fn get_ttl(&self, key: &str) -> Result<Option<i64>> {
        let client = self.client.read().await;
        let client = client.as_ref().ok_or_else(|| anyhow::anyhow!("Redis not initialized"))?;

        let mut conn = client.get_async_connection().await?;
        let ttl: i64 = redis::cmd("TTL").arg(key).query_async(&mut conn).await?;
        
        if ttl < 0 {
            Ok(None)
        } else {
            Ok(Some(ttl))
        }
    }

    async fn refresh_ttl(&self, key: &str, ttl_secs: u64) -> Result<()> {
        let client = self.client.read().await;
        let client = client.as_ref().ok_or_else(|| anyhow::anyhow!("Redis not initialized"))?;

        let mut conn = client.get_async_connection().await?;
        redis::cmd("EXPIRE").arg(key).arg(ttl_secs as i64).query_async(&mut conn).await?;

        Ok(())
    }

    async fn increment(&self, key: &str, delta: i64) -> Result<i64> {
        let client = self.client.read().await;
        let client = client.as_ref().ok_or_else(|| anyhow::anyhow!("Redis not initialized"))?;

        let mut conn = client.get_async_connection().await?;
        let value: i64 = redis::cmd("INCRBY").arg(key).arg(delta).query_async(&mut conn).await?;
        
        Ok(value)
    }

    async fn get_stats(&self) -> Result<CacheStoreStats> {
        let client = self.client.read().await;
        let client = client.as_ref().ok_or_else(|| anyhow::anyhow!("Redis not initialized"))?;

        let mut conn = client.get_async_connection().await?;
        let info: String = redis::cmd("INFO").arg("memory").query_async(&mut conn).await?;
        
        // Parse memory usage (simplified)
        let memory_used_mb = 0.0;

        Ok(CacheStoreStats {
            total_keys: 0, // Would need to scan keyspace
            memory_used_mb,
        })
    }

    async fn clear(&self) -> Result<()> {
        let client = self.client.read().await;
        let client = client.as_ref().ok_or_else(|| anyhow::anyhow!("Redis not initialized"))?;

        let mut conn = client.get_async_connection().await?;
        redis::cmd("FLUSHDB").query_async(&mut conn).await?;

        Ok(())
    }
}

/// Multi-level cache store (L1 + L2).
pub struct MultiLevelStore {
    l1: Arc<InMemoryStore>,
    l2: Arc<RedisStore>,
}

impl MultiLevelStore {
    pub fn new(config: &CacheConfig) -> Self {
        let l1_config = CacheConfig {
            cache_type: crate::CacheType::InMemory,
            local_cache_size: config.local_cache_size,
            ..config.clone()
        };

        let l2_config = CacheConfig {
            cache_type: crate::CacheType::Redis,
            ..config.clone()
        };

        Self {
            l1: Arc::new(InMemoryStore::new(config.local_cache_size)),
            l2: Arc::new(RedisStore::new(&config.redis_url)),
        }
    }
}

#[async_trait::async_trait]
impl CacheStoreTrait for MultiLevelStore {
    async fn initialize(&self) -> Result<()> {
        self.l1.initialize().await?;
        self.l2.initialize().await?;
        Ok(())
    }

    async fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>> {
        // Try L1 first
        if let Some(value) = self.l1.get::<T>(key).await? {
            return Ok(Some(value));
        }

        // Try L2
        if let Some(value) = self.l2.get::<T>(key).await? {
            // Populate L1
            self.l1.set(key, &value, 300).await?; // Short TTL for L1
            return Ok(Some(value));
        }

        Ok(None)
    }

    async fn set<T: Serialize>(&self, key: &str, value: &T, ttl_secs: u64) -> Result<()> {
        // Set in both L1 and L2
        self.l1.set(key, value, ttl_secs.min(300)).await?; // Shorter TTL for L1
        self.l2.set(key, value, ttl_secs).await?;
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        self.l1.delete(key).await?;
        self.l2.delete(key).await?;
        Ok(())
    }

    async fn delete_pattern(&self, pattern: &str) -> Result<usize> {
        let l1_count = self.l1.delete_pattern(pattern).await?;
        let l2_count = self.l2.delete_pattern(pattern).await?;
        Ok(l1_count + l2_count)
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        Ok(self.l1.exists(key).await? || self.l2.exists(key).await?)
    }

    async fn get_ttl(&self, key: &str) -> Result<Option<i64>> {
        self.l2.get_ttl(key).await
    }

    async fn refresh_ttl(&self, key: &str, ttl_secs: u64) -> Result<()> {
        self.l1.refresh_ttl(key, ttl_secs.min(300)).await?;
        self.l2.refresh_ttl(key, ttl_secs).await?;
        Ok(())
    }

    async fn increment(&self, key: &str, delta: i64) -> Result<i64> {
        let value = self.l2.increment(key, delta).await?;
        self.l1.set(key, &value, 300).await?;
        Ok(value)
    }

    async fn get_stats(&self) -> Result<CacheStoreStats> {
        self.l2.get_stats().await
    }

    async fn clear(&self) -> Result<()> {
        self.l1.clear().await?;
        self.l2.clear().await?;
        Ok(())
    }
}

/// Cache store statistics.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CacheStoreStats {
    pub total_keys: i64,
    pub memory_used_mb: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_store() {
        let store = InMemoryStore::new(1000);
        store.initialize().await.unwrap();

        // Set
        store.set("key1", &"value1", 60).await.unwrap();

        // Get
        let value: Option<String> = store.get("key1").await.unwrap();
        assert_eq!(value, Some("value1".to_string()));

        // Exists
        assert!(store.exists("key1").await.unwrap());

        // Delete
        store.delete("key1").await.unwrap();
        assert!(!store.exists("key1").await.unwrap());
    }
}
