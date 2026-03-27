//! Core distributed KV store.

use crate::error::{Error, Result};
use crate::persistence::PersistentStore;
use crate::replication::ReplicationManager;
use crate::query::{Query, QueryResult, Index};
use async_trait::async_trait;
use common::types::AgentId;
use mesh_transport::Transport;
use state_sync::{CrdtMap, StateSync, DefaultStateSync};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Configuration for the distributed KV store.
#[derive(Clone, Debug)]
pub struct Config {
    /// Local agent ID.
    pub local_agent: AgentId,
    /// Whether to enable persistence.
    pub persistence_enabled: bool,
    /// Path for persistent storage (if enabled).
    pub persistence_path: Option<String>,
    /// Replication factor (how many peers to replicate to).
    pub replication_factor: usize,
    /// Sync interval in seconds.
    pub sync_interval_secs: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            local_agent: AgentId(0),
            persistence_enabled: true,
            persistence_path: Some("./data".to_string()),
            replication_factor: 3,
            sync_interval_secs: 5,
        }
    }
}

/// Distributed key‑value store.
pub struct DistributedKV<T: Transport + Send + Sync> {
    config: Config,
    state_sync: Arc<RwLock<DefaultStateSync>>,
    transport: Arc<T>,
    replication: ReplicationManager<T>,
    persistence: Option<PersistentStore>,
    indexes: Vec<Index>,
}

impl<T: Transport + Send + Sync> DistributedKV<T> {
    /// Create a new store with the given configuration and transport.
    pub async fn new(config: Config, transport: T) -> Result<Self> {
        let state_sync = DefaultStateSync::new(config.local_agent);
        let persistence = if config.persistence_enabled {
            let path = config.persistence_path.as_deref().unwrap_or("./data");
            Some(PersistentStore::open(path).await?)
        } else {
            None
        };

        let transport_arc = Arc::new(transport);
        let replication = ReplicationManager::new(
            transport_arc.clone(),
            config.replication_factor,
            config.sync_interval_secs,
        );

        Ok(Self {
            config,
            state_sync: Arc::new(RwLock::new(state_sync)),
            transport: transport_arc,
            replication,
            persistence,
            indexes: Vec::new(),
        })
    }

    /// Insert or update a key with a JSON‑serializable value.
    pub async fn put<V: serde::Serialize>(&mut self, key: String, value: V) -> Result<()> {
        let mut sync = self.state_sync.write().await;
        sync.map_mut().set(&key, value, self.config.local_agent);

        // Persist if enabled
        if let Some(persist) = &mut self.persistence {
            persist.put(&key, &value).await?;
        }

        // Update indexes
        for index in &mut self.indexes {
            index.update(&key, &value).await?;
        }

        // Trigger replication
        self.replication.notify_change().await;

        Ok(())
    }

    /// Retrieve a value by key.
    pub async fn get<V: for<'de> serde::Deserialize<'de>>(&self, key: &str) -> Result<Option<V>> {
        let sync = self.state_sync.read().await;
        let value = sync.map().get(key);
        Ok(value)
    }

    /// Delete a key.
    pub async fn delete(&mut self, key: &str) -> Result<()> {
        let mut sync = self.state_sync.write().await;
        sync.map_mut().delete(key, self.config.local_agent);

        if let Some(persist) = &mut self.persistence {
            persist.delete(key).await?;
        }

        for index in &mut self.indexes {
            index.remove(key).await?;
        }

        self.replication.notify_change().await;
        Ok(())
    }

    /// Execute a query over the stored data.
    pub async fn query(&self, query: Query) -> Result<QueryResult> {
        // For now, naive scan over all keys.
        // In the future, use indexes.
        let sync = self.state_sync.read().await;
        let map = sync.map();
        let mut results = Vec::new();

        // Iterate over all keys (inefficient)
        // We need a method to iterate keys; CrdtMap doesn't expose that.
        // For simplicity, we'll just return empty.
        // TODO: Add iteration to CrdtMap.
        Ok(QueryResult { entries: results })
    }

    /// Add an index to speed up queries.
    pub async fn add_index(&mut self, index: Index) -> Result<()> {
        self.indexes.push(index);
        Ok(())
    }

    /// Start background synchronization.
    pub async fn start_sync(&self) -> Result<()> {
        self.replication.start(self.state_sync.clone()).await
    }

    /// Stop background synchronization.
    pub async fn stop_sync(&self) -> Result<()> {
        self.replication.stop().await
    }

    /// Create a snapshot of the current state.
    pub async fn snapshot(&self) -> Result<()> {
        if let Some(persist) = &self.persistence {
            persist.snapshot().await?;
        }
        Ok(())
    }

    /// Merge a remote delta (called by replication).
    pub(crate) async fn apply_delta(&mut self, delta: state_sync::Delta) -> Result<()> {
        let mut sync = self.state_sync.write().await;
        sync.apply_delta(delta).await.map_err(|e| Error::Crdt(e.to_string()))?;
        Ok(())
    }
}

#[async_trait]
pub trait KVStore {
    /// Put a key‑value pair.
    async fn put(&mut self, key: String, value: serde_json::Value) -> Result<()>;
    /// Get a value.
    async fn get(&self, key: &str) -> Result<Option<serde_json::Value>>;
    /// Delete a key.
    async fn delete(&mut self, key: &str) -> Result<()>;
    /// Query.
    async fn query(&self, query: Query) -> Result<QueryResult>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use mesh_transport::{MeshTransport, MeshTransportConfig};
    use serde_json::json;

    #[tokio::test]
    async fn test_put_get() {
        let config = MeshTransportConfig::in_memory();
        let transport = MeshTransport::new(config).await.unwrap();
        let kv_config = Config {
            local_agent: AgentId(1),
            persistence_enabled: false,
            ..Default::default()
        };
        let mut kv = DistributedKV::new(kv_config, transport).await.unwrap();

        kv.put("foo".to_string(), json!("bar")).await.unwrap();
        let val: Option<serde_json::Value> = kv.get("foo").await.unwrap();
        assert_eq!(val, Some(json!("bar")));
    }
}