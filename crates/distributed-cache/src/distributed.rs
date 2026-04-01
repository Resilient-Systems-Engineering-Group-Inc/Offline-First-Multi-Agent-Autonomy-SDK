//! Distributed cache with replication over mesh transport.

use crate::error::{CacheError, Result};
use crate::local::LocalCache;
use crate::policy::EvictionPolicy;
use async_trait::async_trait;
use common::types::AgentId;
use mesh_transport::Transport;
use std::hash::Hash;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Message types for cache replication.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum CacheMessage<K, V> {
    /// Put a key‑value pair.
    Put {
        key: K,
        value: V,
        ttl_secs: u64,
        size_bytes: usize,
    },
    /// Delete a key.
    Delete(K),
    /// Request a key (get).
    Get(K),
    /// Response to a get request.
    GetResponse {
        key: K,
        value: Option<V>,
        ttl_secs: u64,
    },
    /// Sync entire cache (for new peers).
    Sync(Vec<(K, V, u64)>),
}

/// Configuration for distributed cache.
pub struct DistributedCacheConfig {
    /// Replication factor (how many peers to replicate to).
    pub replication_factor: usize,
    /// Whether to use synchronous replication (wait for acknowledgments).
    pub sync_replication: bool,
    /// Timeout for replication in milliseconds.
    pub replication_timeout_ms: u64,
    /// Enable cache‑aside (fetch from peers on miss).
    pub cache_aside: bool,
}

impl Default for DistributedCacheConfig {
    fn default() -> Self {
        Self {
            replication_factor: 2,
            sync_replication: false,
            replication_timeout_ms: 1000,
            cache_aside: true,
        }
    }
}

/// Distributed cache that replicates across agents.
pub struct DistributedCache<K, V, P, T>
where
    K: Eq + Hash + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
    P: EvictionPolicy<K> + 'static,
    T: Transport + Send + Sync + 'static,
{
    local: Arc<LocalCache<K, V, P>>,
    transport: Arc<T>,
    config: DistributedCacheConfig,
    agent_id: AgentId,
    peers: RwLock<Vec<AgentId>>,
}

impl<K, V, P, T> DistributedCache<K, V, P, T>
where
    K: Eq + Hash + Clone + Send + Sync + serde::Serialize + for<'de> serde::Deserialize<'de> + 'static,
    V: Clone + Send + Sync + serde::Serialize + for<'de> serde::Deserialize<'de> + 'static,
    P: EvictionPolicy<K> + 'static,
    T: Transport + Send + Sync + 'static,
{
    /// Create a new distributed cache.
    pub fn new(
        local: LocalCache<K, V, P>,
        transport: T,
        config: DistributedCacheConfig,
        agent_id: AgentId,
    ) -> Self {
        Self {
            local: Arc::new(local),
            transport: Arc::new(transport),
            config,
            agent_id,
            peers: RwLock::new(Vec::new()),
        }
    }

    /// Update the list of known peers.
    pub async fn update_peers(&self, peers: Vec<AgentId>) {
        *self.peers.write().await = peers;
    }

    /// Put a key‑value pair, replicating to peers.
    pub async fn put(&self, key: K, value: V, ttl_secs: u64, size_bytes: usize) -> Result<()> {
        // Store locally
        self.local.insert(key.clone(), value.clone(), ttl_secs, size_bytes).await?;

        // Replicate to peers
        let peers = self.peers.read().await;
        let target_peers = self.choose_replication_targets(&peers);
        let message = CacheMessage::Put {
            key: key.clone(),
            value,
            ttl_secs,
            size_bytes,
        };
        let bytes = serde_json::to_vec(&message).map_err(CacheError::Serialization)?;

        for peer in target_peers {
            let transport = self.transport.clone();
            let bytes = bytes.clone();
            tokio::spawn(async move {
                let _ = transport.send_to(peer, bytes).await;
            });
        }
        Ok(())
    }

    /// Get a value, possibly fetching from peers.
    pub async fn get(&self, key: &K) -> Result<Option<V>> {
        // Try local cache
        if let Some(value) = self.local.get(key).await? {
            return Ok(Some(value));
        }

        if self.config.cache_aside {
            // Ask peers
            let peers = self.peers.read().await;
            let message = CacheMessage::Get(key.clone());
            let bytes = serde_json::to_vec(&message).map_err(CacheError::Serialization)?;
            for peer in peers.iter() {
                // In a real implementation, we'd send request and wait for response.
                // For simplicity, we just broadcast.
                let _ = self.transport.send_to(*peer, bytes.clone()).await;
            }
        }

        Ok(None)
    }

    /// Delete a key, propagating deletion.
    pub async fn delete(&self, key: &K) -> Result<()> {
        self.local.remove(key).await?;
        let message = CacheMessage::Delete(key.clone());
        let bytes = serde_json::to_vec(&message).map_err(CacheError::Serialization)?;
        let peers = self.peers.read().await;
        for peer in peers.iter() {
            let _ = self.transport.send_to(*peer, bytes.clone()).await;
        }
        Ok(())
    }

    /// Handle an incoming cache message.
    pub async fn handle_message(&self, sender: AgentId, payload: Vec<u8>) -> Result<()> {
        let message: CacheMessage<K, V> =
            serde_json::from_slice(&payload).map_err(CacheError::Serialization)?;
        match message {
            CacheMessage::Put {
                key,
                value,
                ttl_secs,
                size_bytes,
            } => {
                self.local.insert(key, value, ttl_secs, size_bytes).await?;
            }
            CacheMessage::Delete(key) => {
                self.local.remove(&key).await?;
            }
            CacheMessage::Get(key) => {
                // Respond with value if we have it
                if let Some(value) = self.local.get(&key).await? {
                    let message = CacheMessage::GetResponse {
                        key,
                        value: Some(value),
                        ttl_secs: 0, // placeholder
                    };
                    let bytes = serde_json::to_vec(&message).map_err(CacheError::Serialization)?;
                    let _ = self.transport.send_to(sender, bytes).await;
                }
            }
            CacheMessage::GetResponse { key, value, ttl_secs } => {
                // Store the fetched value locally
                if let Some(value) = value {
                    self.local.insert(key, value, ttl_secs, 0).await?;
                }
            }
            CacheMessage::Sync(items) => {
                for (key, value, ttl) in items {
                    let _ = self.local.insert(key, value, ttl, 0).await;
                }
            }
        }
        Ok(())
    }

    /// Choose which peers to replicate to.
    fn choose_replication_targets(&self, peers: &[AgentId]) -> Vec<AgentId> {
        if peers.is_empty() {
            return Vec::new();
        }
        let n = std::cmp::min(self.config.replication_factor, peers.len());
        // Simple deterministic selection based on hash of key? For now pick first n.
        peers.iter().take(n).cloned().collect()
    }
}

/// Trait for cache backends (local + distributed).
#[async_trait]
pub trait CacheBackend<K, V>: Send + Sync {
    /// Get a value.
    async fn get(&self, key: &K) -> Result<Option<V>>;
    /// Put a value.
    async fn put(&self, key: K, value: V, ttl_secs: u64, size_bytes: usize) -> Result<()>;
    /// Delete a value.
    async fn delete(&self, key: &K) -> Result<()>;
    /// Get statistics.
    fn stats(&self) -> crate::item::CacheStats;
}

#[async_trait]
impl<K, V, P, T> CacheBackend<K, V> for DistributedCache<K, V, P, T>
where
    K: Eq + Hash + Clone + Send + Sync + serde::Serialize + for<'de> serde::Deserialize<'de> + 'static,
    V: Clone + Send + Sync + serde::Serialize + for<'de> serde::Deserialize<'de> + 'static,
    P: EvictionPolicy<K> + 'static,
    T: Transport + Send + Sync + 'static,
{
    async fn get(&self, key: &K) -> Result<Option<V>> {
        self.get(key).await
    }

    async fn put(&self, key: K, value: V, ttl_secs: u64, size_bytes: usize) -> Result<()> {
        self.put(key, value, ttl_secs, size_bytes).await
    }

    async fn delete(&self, key: &K) -> Result<()> {
        self.delete(key).await
    }

    fn stats(&self) -> crate::item::CacheStats {
        self.local.stats()
    }
}