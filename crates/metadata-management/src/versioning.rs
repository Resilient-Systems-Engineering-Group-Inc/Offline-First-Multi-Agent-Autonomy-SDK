//! Metadata versioning and history.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::{MetadataError, Result};
use crate::model::{Metadata, MetadataId};

/// Versioned metadata entry.
#[derive(Debug, Clone)]
pub struct VersionedMetadata {
    /// Current metadata.
    pub current: Metadata,
    /// Previous versions (oldest first).
    pub history: Vec<Metadata>,
}

/// Metadata versioning store.
pub struct MetadataVersioning {
    /// Map from metadata ID to versioned entry.
    store: Arc<RwLock<HashMap<MetadataId, VersionedMetadata>>>,
    /// Maximum number of historical versions to keep.
    max_history: usize,
}

impl MetadataVersioning {
    /// Create a new versioning store with a given history limit.
    pub fn new(max_history: usize) -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
            max_history,
        }
    }

    /// Insert or update metadata, preserving previous version.
    pub async fn put(&self, metadata: Metadata) -> Result<()> {
        let mut store = self.store.write().await;
        let id = metadata.id;
        let entry = store.entry(id).or_insert_with(|| VersionedMetadata {
            current: metadata.clone(),
            history: Vec::new(),
        });

        // If the metadata already exists and is different, move current to history
        if entry.current.version != metadata.version {
            entry.history.push(entry.current.clone());
            entry.current = metadata;

            // Trim history if exceeds limit
            if entry.history.len() > self.max_history {
                entry.history.drain(0..entry.history.len() - self.max_history);
            }
        }

        Ok(())
    }

    /// Get current version of metadata.
    pub async fn get(&self, id: MetadataId) -> Result<Option<Metadata>> {
        let store = self.store.read().await;
        Ok(store.get(&id).map(|entry| entry.current.clone()))
    }

    /// Get a specific version of metadata.
    pub async fn get_version(&self, id: MetadataId, version: u64) -> Result<Option<Metadata>> {
        let store = self.store.read().await;
        if let Some(entry) = store.get(&id) {
            if entry.current.version == version {
                return Ok(Some(entry.current.clone()));
            }
            for historical in &entry.history {
                if historical.version == version {
                    return Ok(Some(historical.clone()));
                }
            }
        }
        Ok(None)
    }

    /// Get all versions of a metadata entry (oldest first).
    pub async fn get_all_versions(&self, id: MetadataId) -> Result<Vec<Metadata>> {
        let store = self.store.read().await;
        if let Some(entry) = store.get(&id) {
            let mut all = entry.history.clone();
            all.push(entry.current.clone());
            Ok(all)
        } else {
            Ok(Vec::new())
        }
    }

    /// Rollback to a previous version.
    pub async fn rollback(&self, id: MetadataId, target_version: u64) -> Result<()> {
        let mut store = self.store.write().await;
        let entry = store.get_mut(&id).ok_or_else(|| MetadataError::NotFound(format!("metadata {}", id)))?;

        // Find the target version in history
        let target_index = entry.history.iter().position(|m| m.version == target_version);
        if let Some(idx) = target_index {
            // Remove all newer history entries (after idx) and set current to target
            let target = entry.history[idx].clone();
            entry.history.truncate(idx);
            entry.current = target;
        } else if entry.current.version == target_version {
            // Already current, nothing to do
        } else {
            return Err(MetadataError::Versioning(format!("version {} not found", target_version)));
        }

        Ok(())
    }

    /// Delete metadata and its history.
    pub async fn delete(&self, id: MetadataId) -> Result<()> {
        let mut store = self.store.write().await;
        store.remove(&id);
        Ok(())
    }
}

impl Default for MetadataVersioning {
    fn default() -> Self {
        Self::new(10)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Metadata, MetadataType};

    #[tokio::test]
    async fn test_versioning_put_and_get() {
        let versioning = MetadataVersioning::new(5);
        let metadata = Metadata::new(
            MetadataType::Agent,
            "agent1",
            serde_json::json!({}),
        );
        versioning.put(metadata.clone()).await.unwrap();
        let retrieved = versioning.get(metadata.id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().version, 1);
    }

    #[tokio::test]
    async fn test_versioning_multiple_versions() {
        let versioning = MetadataVersioning::new(5);
        let mut metadata = Metadata::new(
            MetadataType::Agent,
            "agent1",
            serde_json::json!({}),
        );
        versioning.put(metadata.clone()).await.unwrap();

        metadata.update_content(serde_json::json!({ "updated": true }));
        versioning.put(metadata.clone()).await.unwrap();

        let all = versioning.get_all_versions(metadata.id).await.unwrap();
        assert_eq!(all.len(), 2);
    }
}