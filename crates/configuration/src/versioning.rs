//! Configuration versioning with history, diffs, and rollback capabilities.
//!
//! This module provides functionality to track changes to configuration over time,
//! compute diffs between versions, and roll back to previous versions if needed.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::error::Error;
use crate::schema::Configuration;

/// A single version of a configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigurationVersion {
    /// Unique version identifier (e.g., timestamp or hash).
    pub id: String,
    /// Human‑readable label for this version.
    pub label: Option<String>,
    /// The configuration data at this version.
    pub config: Configuration,
    /// When this version was created.
    pub created_at: DateTime<Utc>,
    /// Who created this version (agent ID, user, etc.).
    pub author: Option<String>,
    /// Optional metadata (tags, comments, etc.).
    pub metadata: HashMap<String, String>,
}

impl ConfigurationVersion {
    /// Create a new version.
    pub fn new(
        id: impl Into<String>,
        config: Configuration,
        author: Option<String>,
    ) -> Self {
        Self {
            id: id.into(),
            label: None,
            config,
            created_at: Utc::now(),
            author,
            metadata: HashMap::new(),
        }
    }

    /// Add a label to this version.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Add metadata key‑value pair.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Difference between two configuration versions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigurationDiff {
    /// ID of the "from" version.
    pub from_id: String,
    /// ID of the "to" version.
    pub to_id: String,
    /// JSON Patch (RFC 6902) representing the changes.
    pub patch: serde_json::Value,
    /// Summary of changes (added, removed, modified keys).
    pub summary: DiffSummary,
}

/// Summary of changes in a diff.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffSummary {
    /// Number of added configuration keys.
    pub added: usize,
    /// Number of removed configuration keys.
    pub removed: usize,
    /// Number of modified configuration keys.
    pub modified: usize,
}

impl DiffSummary {
    /// Create an empty summary.
    pub fn empty() -> Self {
        Self {
            added: 0,
            removed: 0,
            modified: 0,
        }
    }

    /// Check if there are any changes.
    pub fn is_empty(&self) -> bool {
        self.added == 0 && self.removed == 0 && self.modified == 0
    }
}

/// Storage backend for configuration versions.
#[async_trait::async_trait]
pub trait VersionStorage: Send + Sync {
    /// Store a new version.
    async fn store(&self, version: ConfigurationVersion) -> Result<(), Error>;

    /// Retrieve a version by ID.
    async fn retrieve(&self, id: &str) -> Result<Option<ConfigurationVersion>, Error>;

    /// List all versions, optionally filtered.
    async fn list(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<ConfigurationVersion>, Error>;

    /// Compute diff between two versions.
    async fn diff(&self, from_id: &str, to_id: &str) -> Result<Option<ConfigurationDiff>, Error>;

    /// Delete a version (if allowed).
    async fn delete(&self, id: &str) -> Result<(), Error>;
}

/// In‑memory storage for configuration versions (useful for testing).
pub struct InMemoryVersionStorage {
    versions: Arc<RwLock<HashMap<String, ConfigurationVersion>>>,
}

impl InMemoryVersionStorage {
    /// Create a new in‑memory storage.
    pub fn new() -> Self {
        Self {
            versions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl VersionStorage for InMemoryVersionStorage {
    async fn store(&self, version: ConfigurationVersion) -> Result<(), Error> {
        let mut versions = self.versions.write().await;
        versions.insert(version.id.clone(), version);
        Ok(())
    }

    async fn retrieve(&self, id: &str) -> Result<Option<ConfigurationVersion>, Error> {
        let versions = self.versions.read().await;
        Ok(versions.get(id).cloned())
    }

    async fn list(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<ConfigurationVersion>, Error> {
        let versions = self.versions.read().await;
        let mut sorted: Vec<_> = versions.values().cloned().collect();
        // Sort by creation date (newest first)
        sorted.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        let offset = offset.unwrap_or(0);
        let limit = limit.unwrap_or(usize::MAX);
        
        Ok(sorted.into_iter().skip(offset).take(limit).collect())
    }

    async fn diff(&self, from_id: &str, to_id: &str) -> Result<Option<ConfigurationDiff>, Error> {
        let versions = self.versions.read().await;
        let from = versions.get(from_id);
        let to = versions.get(to_id);
        
        match (from, to) {
            (Some(from_version), Some(to_version)) => {
                // Simplified diff: just indicate that they're different
                // In a real implementation, you'd use a library like `json_patch`
                let patch = serde_json::json!([]);
                let summary = DiffSummary {
                    added: 1,  // placeholder
                    removed: 0,
                    modified: 0,
                };
                Ok(Some(ConfigurationDiff {
                    from_id: from_id.to_string(),
                    to_id: to_id.to_string(),
                    patch,
                    summary,
                }))
            }
            _ => Ok(None),
        }
    }

    async fn delete(&self, id: &str) -> Result<(), Error> {
        let mut versions = self.versions.write().await;
        versions.remove(id);
        Ok(())
    }
}

/// File‑based storage for configuration versions.
pub struct FileVersionStorage {
    base_dir: PathBuf,
}

impl FileVersionStorage {
    /// Create a new file‑based storage at the given directory.
    pub fn new<P: AsRef<Path>>(base_dir: P) -> Result<Self, Error> {
        let base_dir = base_dir.as_ref().to_path_buf();
        if !base_dir.exists() {
            std::fs::create_dir_all(&base_dir)?;
        }
        Ok(Self { base_dir })
    }

    fn version_path(&self, id: &str) -> PathBuf {
        self.base_dir.join(format!("{}.json", id))
    }
}

#[async_trait::async_trait]
impl VersionStorage for FileVersionStorage {
    async fn store(&self, version: ConfigurationVersion) -> Result<(), Error> {
        let path = self.version_path(&version.id);
        let content = serde_json::to_vec_pretty(&version)?;
        tokio::fs::write(path, content).await?;
        Ok(())
    }

    async fn retrieve(&self, id: &str) -> Result<Option<ConfigurationVersion>, Error> {
        let path = self.version_path(id);
        if !path.exists() {
            return Ok(None);
        }
        let content = tokio::fs::read(path).await?;
        let version = serde_json::from_slice(&content)?;
        Ok(Some(version))
    }

    async fn list(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<ConfigurationVersion>, Error> {
        let mut entries = tokio::fs::read_dir(&self.base_dir).await?;
        let mut versions = Vec::new();
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
                let content = tokio::fs::read(&path).await?;
                if let Ok(version) = serde_json::from_slice(&content) {
                    versions.push(version);
                }
            }
        }
        
        // Sort by creation date (newest first)
        versions.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        let offset = offset.unwrap_or(0);
        let limit = limit.unwrap_or(usize::MAX);
        
        Ok(versions.into_iter().skip(offset).take(limit).collect())
    }

    async fn diff(&self, from_id: &str, to_id: &str) -> Result<Option<ConfigurationDiff>, Error> {
        let from_version = self.retrieve(from_id).await?;
        let to_version = self.retrieve(to_id).await?;
        
        match (from_version, to_version) {
            (Some(from), Some(to)) => {
                // Placeholder diff implementation
                let patch = serde_json::json!([]);
                let summary = DiffSummary::empty();
                Ok(Some(ConfigurationDiff {
                    from_id: from_id.to_string(),
                    to_id: to_id.to_string(),
                    patch,
                    summary,
                }))
            }
            _ => Ok(None),
        }
    }

    async fn delete(&self, id: &str) -> Result<(), Error> {
        let path = self.version_path(id);
        if path.exists() {
            tokio::fs::remove_file(path).await?;
        }
        Ok(())
    }
}

/// Manager for configuration versioning.
pub struct ConfigurationVersionManager {
    storage: Arc<dyn VersionStorage>,
}

impl ConfigurationVersionManager {
    /// Create a new version manager with the given storage backend.
    pub fn new(storage: Arc<dyn VersionStorage>) -> Self {
        Self { storage }
    }

    /// Create a new version from a configuration.
    pub async fn create_version(
        &self,
        config: Configuration,
        label: Option<String>,
        author: Option<String>,
    ) -> Result<String, Error> {
        // Generate a version ID (timestamp + random suffix)
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
        let random_suffix: u32 = rand::random();
        let id = format!("{}_{:06x}", timestamp, random_suffix);
        
        let mut version = ConfigurationVersion::new(&id, config, author);
        if let Some(label) = label {
            version = version.with_label(label);
        }
        
        self.storage.store(version).await?;
        Ok(id)
    }

    /// Get a specific version by ID.
    pub async fn get_version(&self, id: &str) -> Result<Option<ConfigurationVersion>, Error> {
        self.storage.retrieve(id).await
    }

    /// List versions, newest first.
    pub async fn list_versions(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<ConfigurationVersion>, Error> {
        self.storage.list(limit, offset).await
    }

    /// Roll back to a previous version.
    pub async fn rollback(&self, target_id: &str) -> Result<Configuration, Error> {
        let version = self.storage.retrieve(target_id).await?
            .ok_or_else(|| Error::Other(format!("version {} not found", target_id)))?;
        
        // Create a new version representing the rollback
        let rollback_id = self.create_version(
            version.config.clone(),
            Some(format!("Rollback to {}", target_id)),
            Some("system".to_string()),
        ).await?;
        
        Ok(version.config)
    }

    /// Compare two versions.
    pub async fn compare(&self, from_id: &str, to_id: &str) -> Result<Option<ConfigurationDiff>, Error> {
        self.storage.diff(from_id, to_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::Configuration;

    fn sample_config() -> Configuration {
        let mut config = Configuration::new();
        config.insert("key".to_string(), serde_json::json!("value"));
        config
    }

    #[tokio::test]
    async fn test_in_memory_storage() {
        let storage = Arc::new(InMemoryVersionStorage::new());
        let manager = ConfigurationVersionManager::new(storage);
        
        let config = sample_config();
        let id = manager.create_version(config.clone(), None, None).await.unwrap();
        
        let retrieved = manager.get_version(&id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().config, config);
    }

    #[tokio::test]
    async fn test_list_versions() {
        let storage = Arc::new(InMemoryVersionStorage::new());
        let manager = ConfigurationVersionManager::new(storage);
        
        let config = sample_config();
        manager.create_version(config.clone(), None, None).await.unwrap();
        manager.create_version(config.clone(), None, None).await.unwrap();
        
        let versions = manager.list_versions(None, None).await.unwrap();
        assert_eq!(versions.len(), 2);
    }
}