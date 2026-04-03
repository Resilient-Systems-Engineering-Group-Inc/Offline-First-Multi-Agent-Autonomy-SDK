//! Local package cache for offline operation.

use crate::error::{PackageError, Result};
use crate::types::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Local package cache for storing downloaded packages.
pub struct PackageCache {
    /// Path to the cache directory.
    path: PathBuf,
    /// In-memory cache index.
    index: Arc<RwLock<HashMap<String, CacheEntry>>>,
    /// Maximum cache size in bytes.
    max_size_bytes: u64,
    /// Current cache size in bytes.
    current_size_bytes: Arc<RwLock<u64>>,
}

impl PackageCache {
    /// Creates a new package cache at the given path.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        
        // Create directory if it doesn't exist
        if !path.exists() {
            std::fs::create_dir_all(&path).map_err(|e| {
                PackageError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to create cache directory: {}", e),
                ))
            })?;
        }
        
        Ok(Self {
            path,
            index: Arc::new(RwLock::new(HashMap::new())),
            max_size_bytes: 10 * 1024 * 1024 * 1024, // 10 GB default
            current_size_bytes: Arc::new(RwLock::new(0)),
        })
    }
    
    /// Creates a new package cache with custom maximum size.
    pub fn with_max_size<P: AsRef<Path>>(path: P, max_size_bytes: u64) -> Result<Self> {
        let mut cache = Self::new(path)?;
        cache.max_size_bytes = max_size_bytes;
        Ok(cache)
    }
    
    /// Loads the cache index from disk.
    pub async fn load(&self) -> Result<()> {
        let index_path = self.path.join("index.json");
        if !index_path.exists() {
            return Ok(());
        }
        
        let data = fs::read(&index_path).await.map_err(|e| {
            PackageError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to read cache index: {}", e),
            ))
        })?;
        
        let index: HashMap<String, CacheEntry> = serde_json::from_slice(&data)
            .map_err(|e| PackageError::Json(e))?;
        
        // Calculate total size
        let mut total_size = 0;
        for entry in index.values() {
            total_size += entry.package_version.size_bytes;
        }
        
        let mut cache_index = self.index.write().await;
        *cache_index = index;
        
        let mut current_size = self.current_size_bytes.write().await;
        *current_size = total_size;
        
        info!("Loaded cache with {} entries, total size: {} bytes", 
            cache_index.len(), total_size);
        
        Ok(())
    }
    
    /// Saves the cache index to disk.
    pub async fn save(&self) -> Result<()> {
        let index_path = self.path.join("index.json");
        let index = self.index.read().await;
        
        let data = serde_json::to_vec_pretty(&*index)
            .map_err(|e| PackageError::Json(e))?;
        
        fs::write(&index_path, &data).await.map_err(|e| {
            PackageError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to write cache index: {}", e),
            ))
        })?;
        
        Ok(())
    }
    
    /// Checks if a package is in the cache.
    pub async fn has(&self, package_id: &str, version: &str) -> Result<bool> {
        let key = Self::cache_key(package_id, version);
        let index = self.index.read().await;
        Ok(index.contains_key(&key))
    }
    
    /// Gets a package from the cache.
    pub async fn get(&self, package_id: &str, version: &str) -> Result<Vec<u8>> {
        let key = Self::cache_key(package_id, version);
        let index = self.index.read().await;
        
        let entry = index.get(&key)
            .ok_or_else(|| PackageError::PackageNotFound(format!("{} {}", package_id, version)))?;
        
        // Update last accessed time
        let mut entry = entry.clone();
        entry.last_accessed = chrono::Utc::now();
        entry.access_count += 1;
        
        // Read the cached file
        let data = fs::read(&entry.cache_path).await.map_err(|e| {
            PackageError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to read cached file: {}", e),
            ))
        })?;
        
        // Update index with new access time
        drop(index); // Release read lock
        let mut index = self.index.write().await;
        index.insert(key, entry);
        
        debug!("Retrieved {} {} from cache", package_id, version);
        Ok(data)
    }
    
    /// Puts a package into the cache.
    pub async fn put(
        &self,
        package_id: &str,
        package_version: &PackageVersion,
        data: Vec<u8>,
    ) -> Result<()> {
        let key = Self::cache_key(package_id, &package_version.version);
        
        // Check if already in cache
        if self.has(package_id, &package_version.version).await? {
            debug!("Package {} {} already in cache, skipping", package_id, package_version.version);
            return Ok(());
        }
        
        // Check cache size and evict if necessary
        self.ensure_space(data.len() as u64).await?;
        
        // Generate cache filename
        let filename = format!("{}-{}.tar.gz", package_id, package_version.version);
        let cache_path = self.path.join(&filename);
        
        // Write data to file
        fs::write(&cache_path, &data).await.map_err(|e| {
            PackageError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to write cache file: {}", e),
            ))
        })?;
        
        // Create cache entry
        let entry = CacheEntry {
            package_version: package_version.clone(),
            cache_path: cache_path.to_string_lossy().to_string(),
            cached_at: chrono::Utc::now(),
            last_accessed: chrono::Utc::now(),
            access_count: 0,
        };
        
        // Update index
        let mut index = self.index.write().await;
        index.insert(key, entry);
        
        // Update size
        let mut current_size = self.current_size_bytes.write().await;
        *current_size += data.len() as u64;
        
        // Save index
        drop(index);
        self.save().await?;
        
        info!("Cached {} {} ({} bytes)", package_id, package_version.version, data.len());
        Ok(())
    }
    
    /// Removes a package from the cache.
    pub async fn remove(&self, package_id: &str, version: &str) -> Result<()> {
        let key = Self::cache_key(package_id, version);
        let mut index = self.index.write().await;
        
        if let Some(entry) = index.remove(&key) {
            // Remove file
            if let Err(e) = fs::remove_file(&entry.cache_path).await {
                warn!("Failed to remove cache file {}: {}", entry.cache_path, e);
            }
            
            // Update size
            let mut current_size = self.current_size_bytes.write().await;
            *current_size = current_size.saturating_sub(entry.package_version.size_bytes);
            
            info!("Removed {} {} from cache", package_id, version);
        }
        
        Ok(())
    }
    
    /// Cleans the cache by removing old or least accessed packages.
    pub async fn clean(&mut self) -> Result<()> {
        info!("Cleaning package cache");
        
        let index = self.index.read().await;
        let mut entries: Vec<CacheEntry> = index.values().cloned().collect();
        drop(index);
        
        // Sort by last accessed time (oldest first)
        entries.sort_by_key(|e| e.last_accessed);
        
        let mut removed = 0;
        let mut freed_bytes = 0;
        
        // Remove entries until we're under 80% of max size
        let target_size = (self.max_size_bytes as f64 * 0.8) as u64;
        let mut current_size = *self.current_size_bytes.read().await;
        
        for entry in entries {
            if current_size <= target_size {
                break;
            }
            
            let package_id = &entry.package_version.package_id;
            let version = &entry.package_version.version;
            
            if let Err(e) = self.remove(package_id, version).await {
                warn!("Failed to remove {} {} during cleanup: {}", package_id, version, e);
                continue;
            }
            
            removed += 1;
            freed_bytes += entry.package_version.size_bytes;
            current_size = current_size.saturating_sub(entry.package_version.size_bytes);
        }
        
        info!("Cache cleanup removed {} packages, freed {} bytes", removed, freed_bytes);
        Ok(())
    }
    
    /// Gets cache statistics.
    pub async fn stats(&self) -> Result<CacheStats> {
        let index = self.index.read().await;
        let current_size = *self.current_size_bytes.read().await;
        
        let mut packages_by_type = HashMap::new();
        let mut total_access_count = 0;
        
        for entry in index.values() {
            let type_str = match entry.package_version.package_type {
                PackageType::Agent => "agent",
                PackageType::Capability => "capability",
                PackageType::Library => "library",
                PackageType::Plugin => "plugin",
                PackageType::Tool => "tool",
            };
            *packages_by_type.entry(type_str.to_string()).or_insert(0) += 1;
            
            total_access_count += entry.access_count;
        }
        
        Ok(CacheStats {
            total_packages: index.len(),
            total_size_bytes: current_size,
            max_size_bytes: self.max_size_bytes,
            packages_by_type,
            total_access_count,
            last_cleaned: chrono::Utc::now(), // Would be stored in real implementation
        })
    }
    
    /// Ensures there's enough space in the cache for new data.
    async fn ensure_space(&self, needed_bytes: u64) -> Result<()> {
        let current_size = *self.current_size_bytes.read().await;
        
        if current_size + needed_bytes <= self.max_size_bytes {
            return Ok(());
        }
        
        // Need to free space
        let target_free = needed_bytes + (self.max_size_bytes - current_size);
        self.free_space(target_free).await
    }
    
    /// Frees space in the cache by removing least recently used packages.
    async fn free_space(&self, target_bytes: u64) -> Result<()> {
        let index = self.index.read().await;
        let mut entries: Vec<CacheEntry> = index.values().cloned().collect();
        drop(index);
        
        // Sort by last accessed time (oldest first) and access count (least accessed first)
        entries.sort_by(|a, b| {
            a.last_accessed.cmp(&b.last_accessed)
                .then(a.access_count.cmp(&b.access_count))
        });
        
        let mut freed_bytes = 0;
        
        for entry in entries {
            if freed_bytes >= target_bytes {
                break;
            }
            
            let package_id = &entry.package_version.package_id;
            let version = &entry.package_version.version;
            
            if let Err(e) = self.remove(package_id, version).await {
                warn!("Failed to remove {} {} during space freeing: {}", package_id, version, e);
                continue;
            }
            
            freed_bytes += entry.package_version.size_bytes;
        }
        
        if freed_bytes < target_bytes {
            warn!("Could only free {} of {} requested bytes", freed_bytes, target_bytes);
        }
        
        Ok(())
    }
    
    /// Generates a cache key for a package.
    fn cache_key(package_id: &str, version: &str) -> String {
        format!("{}@{}", package_id, version)
    }
}

/// Cache statistics.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CacheStats {
    /// Total number of packages in cache.
    pub total_packages: usize,
    /// Total size of cache in bytes.
    pub total_size_bytes: u64,
    /// Maximum cache size in bytes.
    pub max_size_bytes: u64,
    /// Packages grouped by type.
    pub packages_by_type: HashMap<String, usize>,
    /// Total number of cache accesses.
    pub total_access_count: u64,
    /// When the cache was last cleaned.
    pub last_cleaned: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    fn create_test_package_version() -> PackageVersion {
        PackageVersion {
            package_id: "test-package".to_string(),
            version: "1.0.0".to_string(),
            semver: semver::Version::parse("1.0.0").unwrap(),
            changelog: "".to_string(),
            checksum: "".to_string(),
            size_bytes: 1024,
            dependencies: Vec::new(),
            platforms: Vec::new(),
            install_instructions: None,
            is_default: true,
            is_deprecated: false,
            created_at: chrono::Utc::now(),
            author: "test".to_string(),
        }
    }
    
    #[tokio::test]
    async fn test_package_cache_creation() {
        let temp_dir = tempdir().unwrap();
        let cache = PackageCache::new(temp_dir.path()).unwrap();
        
        assert!(cache.path.exists());
    }
    
    #[tokio::test]
    async fn test_cache_put_and_get() {
        let temp_dir = tempdir().unwrap();
        let cache = PackageCache::new(temp_dir.path()).unwrap();
        
        let package_version = create_test_package_version();
        let test_data = b"test package data".to_vec();
        
        // Put package in cache
        let result = cache.put("test-package", &package_version, test_data.clone()).await;
        assert!(result.is_ok());
        
        // Check if it's in cache
        let has = cache.has("test-package", "1.0.0").await.unwrap();
        assert!(has);
        
        // Get from cache
        let retrieved = cache.get("test-package", "1.0.0").await.unwrap();
        assert_eq!(retrieved, test_data);
    }
    
    #[tokio::test]
    async fn test_cache_remove() {
        let temp_dir = tempdir().unwrap();
        let cache = PackageCache::new(temp_dir.path()).unwrap();
        
        let package_version = create_test_package_version();
        let test_data = b"test data".to_vec();
        
        cache.put("test-package", &package_version, test_data).await.unwrap();
        
        // Remove from cache
        let result = cache.remove("test-package", "1.0.0").await;
        assert!(result.is_ok());
        
        // Should not be in cache anymore
        let has = cache.has("test-package", "1.0.0").await.unwrap();
        assert!(!has);
    }
    
    #[tokio::test]
    async fn test_cache_stats() {
        let temp_dir = tempdir().unwrap();
        let cache = PackageCache::new(temp_dir.path()).unwrap();
        
        let stats = cache.stats().await.unwrap();
        assert_eq!(stats.total_packages, 0);
        assert_eq!(stats.total_size_bytes, 0);
    }
}