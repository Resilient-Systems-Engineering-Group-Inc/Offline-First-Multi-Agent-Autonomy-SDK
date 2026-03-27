//! Persistent storage for the KV store.

use crate::error::{Error, Result};
use serde_json::Value;
use std::path::Path;
use tokio::fs;

/// Persistent store backed by the filesystem (using sled optionally).
pub struct PersistentStore {
    /// Directory where data is stored.
    path: String,
    /// In‑memory cache of recently written keys.
    cache: std::collections::HashMap<String, Value>,
}

impl PersistentStore {
    /// Open or create a persistent store at the given path.
    pub async fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        fs::create_dir_all(&path_str).await?;
        Ok(Self {
            path: path_str,
            cache: HashMap::new(),
        })
    }

    /// Store a key‑value pair.
    pub async fn put<V: serde::Serialize>(&mut self, key: &str, value: &V) -> Result<()> {
        let serialized = serde_json::to_string(value).map_err(Error::Serialization)?;
        let file_path = format!("{}/{}.json", self.path, key);
        fs::write(&file_path, serialized).await?;
        self.cache.insert(key.to_string(), serde_json::to_value(value).unwrap());
        Ok(())
    }

    /// Retrieve a value by key.
    pub async fn get<V: for<'de> serde::Deserialize<'de>>(&self, key: &str) -> Result<Option<V>> {
        // Check cache first
        if let Some(val) = self.cache.get(key) {
            let v = serde_json::from_value(val.clone()).map_err(Error::Serialization)?;
            return Ok(Some(v));
        }

        let file_path = format!("{}/{}.json", self.path, key);
        match fs::read(&file_path).await {
            Ok(data) => {
                let v: V = serde_json::from_slice(&data).map_err(Error::Serialization)?;
                Ok(Some(v))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(Error::Io(e)),
        }
    }

    /// Delete a key.
    pub async fn delete(&mut self, key: &str) -> Result<()> {
        let file_path = format!("{}/{}.json", self.path, key);
        let _ = fs::remove_file(&file_path).await;
        self.cache.remove(key);
        Ok(())
    }

    /// Create a snapshot of the entire store.
    pub async fn snapshot(&self) -> Result<()> {
        let snapshot_dir = format!("{}/snapshots/{}", self.path, chrono::Utc::now().timestamp());
        fs::create_dir_all(&snapshot_dir).await?;

        // Copy all .json files
        let mut entries = fs::read_dir(&self.path).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let dest = format!("{}/{}", snapshot_dir, path.file_name().unwrap().to_string_lossy());
                fs::copy(&path, &dest).await?;
            }
        }
        Ok(())
    }

    /// List all keys.
    pub async fn list_keys(&self) -> Result<Vec<String>> {
        let mut keys = Vec::new();
        let mut entries = fs::read_dir(&self.path).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem() {
                    keys.push(stem.to_string_lossy().to_string());
                }
            }
        }
        Ok(keys)
    }
}

/// A snapshot of the store at a point in time.
pub struct Snapshot {
    /// Timestamp of the snapshot.
    pub timestamp: i64,
    /// Path to the snapshot directory.
    pub path: String,
}

use std::collections::HashMap;