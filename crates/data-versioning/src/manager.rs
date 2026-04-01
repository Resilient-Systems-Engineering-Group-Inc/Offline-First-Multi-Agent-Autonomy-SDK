//! Snapshot manager for creating, storing, and retrieving versions.

use crate::error::{Result, VersioningError};
use crate::version::{Snapshot, Version, Delta};
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::RwLock;

/// Storage backend for snapshots and deltas.
#[async_trait]
pub trait VersionStorage: Send + Sync {
    /// Store a snapshot.
    async fn store_snapshot(&self, snapshot: &Snapshot) -> Result<()>;

    /// Retrieve a snapshot by version.
    async fn get_snapshot(&self, version: &Version) -> Result<Snapshot>;

    /// List all snapshots.
    async fn list_snapshots(&self) -> Result<Vec<Snapshot>>;

    /// Store a delta.
    async fn store_delta(&self, delta: &Delta) -> Result<()>;

    /// Retrieve a delta between two versions.
    async fn get_delta(&self, from: &Version, to: &Version) -> Result<Delta>;

    /// Delete a snapshot (and associated deltas).
    async fn delete_snapshot(&self, version: &Version) -> Result<()>;
}

/// In‑memory storage (for testing).
pub struct InMemoryStorage {
    snapshots: RwLock<HashMap<String, Snapshot>>,
    deltas: RwLock<HashMap<String, Delta>>,
}

impl InMemoryStorage {
    /// Create a new in‑memory storage.
    pub fn new() -> Self {
        Self {
            snapshots: RwLock::new(HashMap::new()),
            deltas: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl VersionStorage for InMemoryStorage {
    async fn store_snapshot(&self, snapshot: &Snapshot) -> Result<()> {
        let key = snapshot.version.to_string();
        self.snapshots.write().await.insert(key, snapshot.clone());
        Ok(())
    }

    async fn get_snapshot(&self, version: &Version) -> Result<Snapshot> {
        let key = version.to_string();
        self.snapshots
            .read()
            .await
            .get(&key)
            .cloned()
            .ok_or_else(|| VersioningError::SnapshotNotFound(key))
    }

    async fn list_snapshots(&self) -> Result<Vec<Snapshot>> {
        Ok(self.snapshots.read().await.values().cloned().collect())
    }

    async fn store_delta(&self, delta: &Delta) -> Result<()> {
        let key = format!("{}->{}", delta.from.to_string(), delta.to.to_string());
        self.deltas.write().await.insert(key, delta.clone());
        Ok(())
    }

    async fn get_delta(&self, from: &Version, to: &Version) -> Result<Delta> {
        let key = format!("{}->{}", from.to_string(), to.to_string());
        self.deltas
            .read()
            .await
            .get(&key)
            .cloned()
            .ok_or_else(|| VersioningError::SnapshotNotFound(key))
    }

    async fn delete_snapshot(&self, version: &Version) -> Result<()> {
        let key = version.to_string();
        self.snapshots.write().await.remove(&key);
        // Also delete related deltas (simplistic).
        let mut deltas = self.deltas.write().await;
        deltas.retain(|k, _| !k.contains(&key));
        Ok(())
    }
}

/// File‑based storage (persistent).
pub struct FileStorage {
    base_dir: PathBuf,
}

impl FileStorage {
    /// Create a new file storage at the given directory.
    pub fn new<P: Into<PathBuf>>(base_dir: P) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }
}

#[async_trait]
impl VersionStorage for FileStorage {
    async fn store_snapshot(&self, snapshot: &Snapshot) -> Result<()> {
        let path = self.base_dir.join("snapshots").join(snapshot.version.to_string());
        tokio::fs::create_dir_all(path.parent().unwrap()).await?;
        let data = serde_json::to_vec(snapshot)?;
        tokio::fs::write(path, data).await?;
        Ok(())
    }

    async fn get_snapshot(&self, version: &Version) -> Result<Snapshot> {
        let path = self.base_dir.join("snapshots").join(version.to_string());
        let data = tokio::fs::read(path).await?;
        let snapshot: Snapshot = serde_json::from_slice(&data)?;
        Ok(snapshot)
    }

    async fn list_snapshots(&self) -> Result<Vec<Snapshot>> {
        let dir = self.base_dir.join("snapshots");
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut entries = tokio::fs::read_dir(dir).await?;
        let mut snapshots = Vec::new();
        while let Some(entry) = entries.next_entry().await? {
            let data = tokio::fs::read(entry.path()).await?;
            let snapshot: Snapshot = serde_json::from_slice(&data)?;
            snapshots.push(snapshot);
        }
        Ok(snapshots)
    }

    async fn store_delta(&self, delta: &Delta) -> Result<()> {
        let path = self
            .base_dir
            .join("deltas")
            .join(format!("{}->{}", delta.from.to_string(), delta.to.to_string()));
        tokio::fs::create_dir_all(path.parent().unwrap()).await?;
        let data = serde_json::to_vec(delta)?;
        tokio::fs::write(path, data).await?;
        Ok(())
    }

    async fn get_delta(&self, from: &Version, to: &Version) -> Result<Delta> {
        let path = self
            .base_dir
            .join("deltas")
            .join(format!("{}->{}", from.to_string(), to.to_string()));
        let data = tokio::fs::read(path).await?;
        let delta: Delta = serde_json::from_slice(&data)?;
        Ok(delta)
    }

    async fn delete_snapshot(&self, version: &Version) -> Result<()> {
        let snapshot_path = self.base_dir.join("snapshots").join(version.to_string());
        if snapshot_path.exists() {
            tokio::fs::remove_file(snapshot_path).await?;
        }
        // Delete related deltas (simplistic).
        let delta_dir = self.base_dir.join("deltas");
        if delta_dir.exists() {
            let mut entries = tokio::fs::read_dir(delta_dir).await?;
            while let Some(entry) = entries.next_entry().await? {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.contains(&version.to_string()) {
                    tokio::fs::remove_file(entry.path()).await?;
                }
            }
        }
        Ok(())
    }
}

/// Main version manager.
pub struct VersionManager<S: VersionStorage> {
    storage: S,
    current_version: RwLock<Option<Version>>,
}

impl<S: VersionStorage> VersionManager<S> {
    /// Create a new version manager with given storage.
    pub fn new(storage: S) -> Self {
        Self {
            storage,
            current_version: RwLock::new(None),
        }
    }

    /// Create a snapshot of the current state.
    pub async fn create_snapshot(
        &self,
        description: String,
        data: Vec<u8>,
        metadata: HashMap<String, String>,
    ) -> Result<Version> {
        let current = self.current_version.read().await;
        let seq = current.as_ref().map(|v| v.seq + 1).unwrap_or(0);
        let author = 0; // In real usage, would be the agent ID.
        let version = Version::new(seq, author);
        let snapshot = Snapshot::new(version.clone(), description, data, metadata);
        self.storage.store_snapshot(&snapshot).await?;
        *self.current_version.write().await = Some(version.clone());
        Ok(version)
    }

    /// Restore state to a given version.
    pub async fn restore_snapshot(&self, version: &Version) -> Result<Snapshot> {
        let snapshot = self.storage.get_snapshot(version).await?;
        *self.current_version.write().await = Some(version.clone());
        Ok(snapshot)
    }

    /// List all available snapshots.
    pub async fn list_snapshots(&self) -> Result<Vec<Snapshot>> {
        self.storage.list_snapshots().await
    }

    /// Get the current version.
    pub async fn current_version(&self) -> Option<Version> {
        self.current_version.read().await.clone()
    }

    /// Create a delta between two versions (if stored).
    pub async fn get_delta(&self, from: &Version, to: &Version) -> Result<Delta> {
        self.storage.get_delta(from, to).await
    }

    /// Delete a snapshot.
    pub async fn delete_snapshot(&self, version: &Version) -> Result<()> {
        self.storage.delete_snapshot(version).await
    }
}