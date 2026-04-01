//! Integration with state‑sync CRDT maps.

use crate::error::{Result, VersioningError};
use crate::manager::VersionManager;
use crate::version::{Snapshot, Version};
use state_sync::crdt_map::CrdtMap;
use std::collections::HashMap;

/// Wrapper that adds versioning to a CRDT map.
pub struct VersionedCrdtMap<S> {
    /// Inner CRDT map.
    pub map: CrdtMap,
    /// Version manager.
    pub version_manager: VersionManager<S>,
}

impl<S: crate::manager::VersionStorage> VersionedCrdtMap<S> {
    /// Create a new versioned CRDT map.
    pub fn new(version_manager: VersionManager<S>) -> Self {
        Self {
            map: CrdtMap::new(),
            version_manager,
        }
    }

    /// Create a snapshot of the current map state.
    pub async fn snapshot(&self, description: String) -> Result<Version> {
        let data = serde_json::to_vec(&self.map).map_err(VersioningError::Serialization)?;
        let metadata = HashMap::new();
        self.version_manager
            .create_snapshot(description, data, metadata)
            .await
    }

    /// Restore the map to a given version.
    pub async fn restore(&mut self, version: &Version) -> Result<()> {
        let snapshot = self.version_manager.restore_snapshot(version).await?;
        let restored_map: CrdtMap =
            serde_json::from_slice(&snapshot.data).map_err(VersioningError::Serialization)?;
        self.map = restored_map;
        Ok(())
    }

    /// Get the current version.
    pub async fn current_version(&self) -> Option<Version> {
        self.version_manager.current_version().await
    }

    /// List all snapshots.
    pub async fn list_snapshots(&self) -> Result<Vec<Snapshot>> {
        self.version_manager.list_snapshots().await
    }
}

/// Trait for types that can be versioned.
pub trait Versionable {
    /// Serialize the state to bytes.
    fn serialize(&self) -> Result<Vec<u8>>;
    /// Deserialize from bytes.
    fn deserialize(data: &[u8]) -> Result<Self>
    where
        Self: Sized;
}

impl Versionable for CrdtMap {
    fn serialize(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(self).map_err(VersioningError::Serialization)
    }

    fn deserialize(data: &[u8]) -> Result<Self> {
        serde_json::from_slice(data).map_err(VersioningError::Serialization)
    }
}

/// Versioned state manager that works with any Versionable type.
pub struct VersionedState<T, S> {
    state: T,
    version_manager: VersionManager<S>,
}

impl<T: Versionable, S: crate::manager::VersionStorage> VersionedState<T, S> {
    /// Create a new versioned state.
    pub fn new(state: T, version_manager: VersionManager<S>) -> Self {
        Self {
            state,
            version_manager,
        }
    }

    /// Get a reference to the inner state.
    pub fn state(&self) -> &T {
        &self.state
    }

    /// Get a mutable reference to the inner state.
    pub fn state_mut(&mut self) -> &mut T {
        &mut self.state
    }

    /// Create a snapshot.
    pub async fn snapshot(&self, description: String) -> Result<Version> {
        let data = self.state.serialize()?;
        let metadata = HashMap::new();
        self.version_manager
            .create_snapshot(description, data, metadata)
            .await
    }

    /// Restore to a version.
    pub async fn restore(&mut self, version: &Version) -> Result<()> {
        let snapshot = self.version_manager.restore_snapshot(version).await?;
        let restored = T::deserialize(&snapshot.data)?;
        self.state = restored;
        Ok(())
    }
}