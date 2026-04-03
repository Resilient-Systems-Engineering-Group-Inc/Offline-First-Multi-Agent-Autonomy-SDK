//! Metadata storage backend.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use dashmap::DashMap;

use crate::error::{MetadataError, Result};
use crate::model::{Metadata, MetadataId, MetadataType};

/// Trait for metadata storage backends.
#[async_trait::async_trait]
pub trait MetadataStorageBackend: Send + Sync {
    /// Store a metadata entry.
    async fn store(&self, metadata: Metadata) -> Result<MetadataId>;

    /// Retrieve a metadata entry by ID.
    async fn retrieve(&self, id: MetadataId) -> Result<Option<Metadata>>;

    /// Update an existing metadata entry.
    async fn update(&self, metadata: Metadata) -> Result<()>;

    /// Delete a metadata entry.
    async fn delete(&self, id: MetadataId) -> Result<()>;

    /// List metadata entries by type.
    async fn list_by_type(&self, metadata_type: MetadataType) -> Result<Vec<Metadata>>;

    /// List metadata entries by entity ID.
    async fn list_by_entity(&self, entity_id: &str) -> Result<Vec<Metadata>>;
}

/// In‑memory storage backend (for testing).
pub struct InMemoryMetadataStorage {
    storage: Arc<DashMap<MetadataId, Metadata>>,
    index_by_type: Arc<RwLock<HashMap<MetadataType, Vec<MetadataId>>>>,
    index_by_entity: Arc<RwLock<HashMap<String, Vec<MetadataId>>>>,
}

impl InMemoryMetadataStorage {
    /// Create a new in‑memory storage.
    pub fn new() -> Self {
        Self {
            storage: Arc::new(DashMap::new()),
            index_by_type: Arc::new(RwLock::new(HashMap::new())),
            index_by_entity: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn update_indices(&self, metadata: &Metadata) {
        // Update type index
        {
            let mut index = self.index_by_type.write().await;
            let entry = index.entry(metadata.metadata_type.clone()).or_insert_with(Vec::new);
            if !entry.contains(&metadata.id) {
                entry.push(metadata.id);
            }
        }
        // Update entity index
        {
            let mut index = self.index_by_entity.write().await;
            let entry = index.entry(metadata.entity_id.clone()).or_insert_with(Vec::new);
            if !entry.contains(&metadata.id) {
                entry.push(metadata.id);
            }
        }
    }

    async fn remove_from_indices(&self, metadata: &Metadata) {
        // Remove from type index
        {
            let mut index = self.index_by_type.write().await;
            if let Some(entry) = index.get_mut(&metadata.metadata_type) {
                entry.retain(|id| *id != metadata.id);
            }
        }
        // Remove from entity index
        {
            let mut index = self.index_by_entity.write().await;
            if let Some(entry) = index.get_mut(&metadata.entity_id) {
                entry.retain(|id| *id != metadata.id);
            }
        }
    }
}

impl Default for InMemoryMetadataStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl MetadataStorageBackend for InMemoryMetadataStorage {
    async fn store(&self, metadata: Metadata) -> Result<MetadataId> {
        let id = metadata.id;
        self.storage.insert(id, metadata.clone());
        self.update_indices(&metadata).await;
        Ok(id)
    }

    async fn retrieve(&self, id: MetadataId) -> Result<Option<Metadata>> {
        Ok(self.storage.get(&id).map(|entry| entry.clone()))
    }

    async fn update(&self, metadata: Metadata) -> Result<()> {
        let old = self.storage.get(&metadata.id).map(|entry| entry.clone());
        if let Some(old_metadata) = old {
            self.remove_from_indices(&old_metadata).await;
        }
        self.storage.insert(metadata.id, metadata.clone());
        self.update_indices(&metadata).await;
        Ok(())
    }

    async fn delete(&self, id: MetadataId) -> Result<()> {
        if let Some((_, metadata)) = self.storage.remove(&id) {
            self.remove_from_indices(&metadata).await;
        }
        Ok(())
    }

    async fn list_by_type(&self, metadata_type: MetadataType) -> Result<Vec<Metadata>> {
        let index = self.index_by_type.read().await;
        let ids = index.get(&metadata_type).cloned().unwrap_or_default();
        let mut result = Vec::new();
        for id in ids {
            if let Some(metadata) = self.storage.get(&id) {
                result.push(metadata.clone());
            }
        }
        Ok(result)
    }

    async fn list_by_entity(&self, entity_id: &str) -> Result<Vec<Metadata>> {
        let index = self.index_by_entity.read().await;
        let ids = index.get(entity_id).cloned().unwrap_or_default();
        let mut result = Vec::new();
        for id in ids {
            if let Some(metadata) = self.storage.get(&id) {
                result.push(metadata.clone());
            }
        }
        Ok(result)
    }
}

/// Metadata storage with a configurable backend.
pub struct MetadataStorage {
    backend: Arc<dyn MetadataStorageBackend>,
}

impl MetadataStorage {
    /// Create a new metadata storage with a given backend.
    pub fn new(backend: Arc<dyn MetadataStorageBackend>) -> Self {
        Self { backend }
    }

    /// Store metadata.
    pub async fn store(&self, metadata: Metadata) -> Result<MetadataId> {
        self.backend.store(metadata).await
    }

    /// Retrieve metadata by ID.
    pub async fn retrieve(&self, id: MetadataId) -> Result<Option<Metadata>> {
        self.backend.retrieve(id).await
    }

    /// Update metadata.
    pub async fn update(&self, metadata: Metadata) -> Result<()> {
        self.backend.update(metadata).await
    }

    /// Delete metadata.
    pub async fn delete(&self, id: MetadataId) -> Result<()> {
        self.backend.delete(id).await
    }

    /// List metadata by type.
    pub async fn list_by_type(&self, metadata_type: MetadataType) -> Result<Vec<Metadata>> {
        self.backend.list_by_type(metadata_type).await
    }

    /// List metadata by entity ID.
    pub async fn list_by_entity(&self, entity_id: &str) -> Result<Vec<Metadata>> {
        self.backend.list_by_entity(entity_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::MetadataType;

    #[tokio::test]
    async fn test_in_memory_storage() {
        let storage = InMemoryMetadataStorage::new();
        let metadata = crate::model::Metadata::new(
            MetadataType::Agent,
            "agent1",
            serde_json::json!({}),
        );
        let id = storage.store(metadata).await.unwrap();
        let retrieved = storage.retrieve(id).await.unwrap();
        assert!(retrieved.is_some());
    }
}