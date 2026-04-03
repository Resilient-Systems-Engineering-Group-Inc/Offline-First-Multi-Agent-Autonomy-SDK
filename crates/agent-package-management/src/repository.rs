//! Package repository implementations (local and remote).

use crate::error::{PackageError, Result};
use crate::types::*;
use async_trait::async_trait;
use futures::future::BoxFuture;
use semver::{Version, VersionReq};
use serde_json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Trait for package repositories.
#[async_trait]
pub trait Repository: Send + Sync {
    /// Returns the repository name.
    fn name(&self) -> &str;
    
    /// Searches for packages matching the query.
    async fn search(&self, query: &PackageQuery) -> Result<Vec<PackageMetadata>>;
    
    /// Finds a package version that matches the version requirement.
    async fn find_version(&self, package_id: &str, version_req: &VersionReq) -> Result<PackageVersion>;
    
    /// Gets package metadata.
    async fn get_metadata(&self, package_id: &str) -> Result<PackageMetadata>;
    
    /// Lists all versions of a package.
    async fn list_versions(&self, package_id: &str) -> Result<Vec<PackageVersion>>;
}

/// Local repository for installed packages.
pub struct LocalRepository {
    /// Path to the local repository database.
    path: PathBuf,
    /// In-memory cache of installed packages.
    cache: Arc<RwLock<HashMap<String, InstallResult>>>,
}

impl LocalRepository {
    /// Creates a new local repository at the given path.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        
        // Create directory if it doesn't exist
        if !path.exists() {
            std::fs::create_dir_all(&path).map_err(|e| {
                PackageError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to create local repository directory: {}", e),
                ))
            })?;
        }
        
        Ok(Self {
            path,
            cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }
    
    /// Loads the repository from disk.
    pub async fn load(&self) -> Result<()> {
        let index_path = self.path.join("index.json");
        if !index_path.exists() {
            return Ok(());
        }
        
        let data = fs::read(&index_path).await.map_err(|e| {
            PackageError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to read repository index: {}", e),
            ))
        })?;
        
        let index: HashMap<String, InstallResult> = serde_json::from_slice(&data)
            .map_err(|e| PackageError::Json(e))?;
        
        let mut cache = self.cache.write().await;
        *cache = index;
        
        info!("Loaded local repository with {} packages", cache.len());
        Ok(())
    }
    
    /// Saves the repository to disk.
    pub async fn save(&self) -> Result<()> {
        let index_path = self.path.join("index.json");
        let cache = self.cache.read().await;
        
        let data = serde_json::to_vec_pretty(&*cache)
            .map_err(|e| PackageError::Json(e))?;
        
        fs::write(&index_path, &data).await.map_err(|e| {
            PackageError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to write repository index: {}", e),
            ))
        })?;
        
        Ok(())
    }
    
    /// Registers a package installation.
    pub async fn register_installation(&self, install_result: &InstallResult) -> Result<()> {
        let key = format!("{}@{}", install_result.package.package_id, install_result.package.version);
        
        let mut cache = self.cache.write().await;
        cache.insert(key, install_result.clone());
        
        // Save to disk
        self.save().await?;
        
        info!("Registered installation: {}", install_result.package.package_id);
        Ok(())
    }
    
    /// Unregisters a package installation.
    pub async fn unregister_installation(&self, package_id: &str, version: &str) -> Result<()> {
        let key = format!("{}@{}", package_id, version);
        
        let mut cache = self.cache.write().await;
        if cache.remove(&key).is_none() {
            return Err(PackageError::PackageNotFound(format!("{} {}", package_id, version)));
        }
        
        // Save to disk
        self.save().await?;
        
        info!("Unregistered installation: {} {}", package_id, version);
        Ok(())
    }
    
    /// Checks if a package is installed.
    pub async fn is_installed(&self, package_id: &str, version: &str) -> Result<bool> {
        let key = format!("{}@{}", package_id, version);
        let cache = self.cache.read().await;
        Ok(cache.contains_key(&key))
    }
    
    /// Gets installation information.
    pub async fn get_installation(&self, package_id: &str, version: &str) -> Result<InstallResult> {
        let key = format!("{}@{}", package_id, version);
        let cache = self.cache.read().await;
        cache.get(&key)
            .cloned()
            .ok_or_else(|| PackageError::PackageNotFound(format!("{} {}", package_id, version)))
    }
    
    /// Gets the installed version of a package.
    pub async fn get_installed_version(&self, package_id: &str) -> Result<String> {
        let cache = self.cache.read().await;
        
        // Find the latest installed version
        let mut versions = Vec::new();
        for key in cache.keys() {
            if key.starts_with(&format!("{}@", package_id)) {
                let version = key.split('@').nth(1).unwrap_or("");
                versions.push(version.to_string());
            }
        }
        
        if versions.is_empty() {
            return Err(PackageError::PackageNotFound(package_id.to_string()));
        }
        
        // Sort by semantic version (simplified)
        versions.sort();
        Ok(versions.last().unwrap().clone())
    }
    
    /// Lists all installations.
    pub async fn list_installations(&self) -> Result<Vec<InstallResult>> {
        let cache = self.cache.read().await;
        Ok(cache.values().cloned().collect())
    }
    
    /// Gets repository statistics.
    pub async fn stats(&self) -> Result<PackageStats> {
        let cache = self.cache.read().await;
        
        let mut packages_by_type = HashMap::new();
        let mut popular_packages = HashMap::new();
        
        for install in cache.values() {
            // Count by package type
            let type_str = match install.package.package_type {
                PackageType::Agent => "agent",
                PackageType::Capability => "capability",
                PackageType::Library => "library",
                PackageType::Plugin => "plugin",
                PackageType::Tool => "tool",
            };
            *packages_by_type.entry(type_str.to_string()).or_insert(0) += 1;
            
            // Track popularity (simplified)
            *popular_packages.entry(install.package.package_id.clone()).or_insert(0) += 1;
        }
        
        let popular_packages_vec: Vec<(String, u64)> = popular_packages
            .into_iter()
            .map(|(id, count)| (id, count))
            .collect();
        
        Ok(PackageStats {
            total_packages: cache.len(),
            total_versions: cache.len(),
            cache_size_bytes: 0, // Not applicable for local repo
            packages_by_type,
            popular_packages: popular_packages_vec,
            last_updated: chrono::Utc::now(),
        })
    }
}

#[async_trait]
impl Repository for LocalRepository {
    fn name(&self) -> &str {
        "local"
    }
    
    async fn search(&self, query: &PackageQuery) -> Result<Vec<PackageMetadata>> {
        let cache = self.cache.read().await;
        let mut results = Vec::new();
        
        for install in cache.values() {
            let metadata = PackageMetadata {
                id: install.package.package_id.clone(),
                name: install.package.package_id.clone(), // Simplified
                description: String::new(),
                package_type: install.package.package_type.clone(),
                author: install.package.author.clone(),
                license: String::new(),
                repository: None,
                tags: Vec::new(),
                custom_metadata: HashMap::new(),
                created_at: install.package.created_at,
                updated_at: install.package.created_at,
            };
            
            // Apply filters
            if let Some(query_type) = &query.package_type {
                if &metadata.package_type != query_type {
                    continue;
                }
            }
            
            if let Some(search_term) = &query.search {
                if !metadata.id.contains(search_term) && !metadata.name.contains(search_term) {
                    continue;
                }
            }
            
            results.push(metadata);
        }
        
        Ok(results)
    }
    
    async fn find_version(&self, package_id: &str, version_req: &VersionReq) -> Result<PackageVersion> {
        let cache = self.cache.read().await;
        
        // Find all installed versions of this package
        let mut matching_versions = Vec::new();
        for key in cache.keys() {
            if key.starts_with(&format!("{}@", package_id)) {
                let version_str = key.split('@').nth(1).unwrap_or("");
                let version = Version::parse(version_str)
                    .map_err(|e| PackageError::Semver(e))?;
                
                if version_req.matches(&version) {
                    let install = cache.get(key).unwrap();
                    matching_versions.push((version, install.package.clone()));
                }
            }
        }
        
        if matching_versions.is_empty() {
            return Err(PackageError::PackageNotFound(package_id.to_string()));
        }
        
        // Return the highest version
        matching_versions.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(matching_versions.last().unwrap().1.clone())
    }
    
    async fn get_metadata(&self, package_id: &str) -> Result<PackageMetadata> {
        let cache = self.cache.read().await;
        
        // Find any installation of this package
        for key in cache.keys() {
            if key.starts_with(&format!("{}@", package_id)) {
                let install = cache.get(key).unwrap();
                return Ok(PackageMetadata {
                    id: install.package.package_id.clone(),
                    name: install.package.package_id.clone(),
                    description: String::new(),
                    package_type: install.package.package_type.clone(),
                    author: install.package.author.clone(),
                    license: String::new(),
                    repository: None,
                    tags: Vec::new(),
                    custom_metadata: HashMap::new(),
                    created_at: install.package.created_at,
                    updated_at: install.package.created_at,
                });
            }
        }
        
        Err(PackageError::PackageNotFound(package_id.to_string()))
    }
    
    async fn list_versions(&self, package_id: &str) -> Result<Vec<PackageVersion>> {
        let cache = self.cache.read().await;
        let mut versions = Vec::new();
        
        for key in cache.keys() {
            if key.starts_with(&format!("{}@", package_id)) {
                let install = cache.get(key).unwrap();
                versions.push(install.package.clone());
            }
        }
        
        Ok(versions)
    }
}

/// Remote repository for package distribution.
pub struct RemoteRepository {
    /// Repository configuration.
    config: RegistryConfig,
    /// HTTP client (if network feature is enabled).
    #[cfg(feature = "network")]
    client: reqwest::Client,
}

impl RemoteRepository {
    /// Creates a new remote repository.
    pub fn new(config: RegistryConfig) -> Result<Self> {
        #[cfg(feature = "network")]
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| PackageError::NetworkError(e.to_string()))?;
        
        Ok(Self {
            config,
            #[cfg(feature = "network")]
            client,
        })
    }
    
    /// Gets the repository URL.
    pub fn url(&self) -> &str {
        &self.config.url
    }
    
    /// Downloads a package from the remote repository.
    #[cfg(feature = "network")]
    pub async fn download(&self, package_id: &str, version: &str) -> Result<Vec<u8>> {
        let url = format!("{}/packages/{}/{}.tar.gz", self.config.url, package_id, version);
        
        let mut request = self.client.get(&url);
        
        if let Some(token) = &self.config.auth_token {
            request = request.bearer_auth(token);
        }
        
        let response = request.send().await
            .map_err(|e| PackageError::NetworkError(e.to_string()))?;
        
        if !response.status().is_success() {
            return Err(PackageError::NetworkError(format!(
                "HTTP {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )));
        }
        
        let data = response.bytes().await
            .map_err(|e| PackageError::NetworkError(e.to_string()))?
            .to_vec();
        
        Ok(data)
    }
    
    /// Downloads a package from the remote repository (stub for when network feature is disabled).
    #[cfg(not(feature = "network"))]
    pub async fn download(&self, package_id: &str, version: &str) -> Result<Vec<u8>> {
        Err(PackageError::NetworkError(
            "Network feature is disabled. Enable with 'network' feature.".to_string()
        ))
    }
}

#[async_trait]
impl Repository for RemoteRepository {
    fn name(&self) -> &str {
        "remote"
    }
    
    async fn search(&self, query: &PackageQuery) -> Result<Vec<PackageMetadata>> {
        // In a real implementation, this would make an HTTP request to the registry API
        // For now, return an empty list
        Ok(Vec::new())
    }
    
    async fn find_version(&self, package_id: &str, version_req: &VersionReq) -> Result<PackageVersion> {
        // In a real implementation, this would query the registry
        // For now, return a mock error
        Err(PackageError::PackageNotFound(package_id.to_string()))
    }
    
    async fn get_metadata(&self, package_id: &str) -> Result<PackageMetadata> {
        Err(PackageError::PackageNotFound(package_id.to_string()))
    }
    
    async fn list_versions(&self, package_id: &str) -> Result<Vec<PackageVersion>> {
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_local_repository_creation() {
        let temp_dir = tempdir().unwrap();
        let repo = LocalRepository::new(temp_dir.path()).unwrap();
        
        assert_eq!(repo.name(), "local");
    }
    
    #[tokio::test]
    async fn test_local_repository_register_installation() {
        let temp_dir = tempdir().unwrap();
        let repo = LocalRepository::new(temp_dir.path()).unwrap();
        
        let package_version = PackageVersion {
            package_id: "test-package".to_string(),
            version: "1.0.0".to_string(),
            semver: Version::parse("1.0.0").unwrap(),
            changelog: "Initial release".to_string(),
            checksum: "abc123".to_string(),
            size_bytes: 1024,
            dependencies: Vec::new(),
            platforms: Vec::new(),
            install_instructions: None,
            is_default: true,
            is_deprecated: false,
            created_at: chrono::Utc::now(),
            author: "test".to_string(),
        };
        
        let install_result = InstallResult {
            package: package_version,
            install_path: "/tmp/test".to_string(),
            installed_deps: Vec::new(),
            total_size_bytes: 1024,
            installed_at: chrono::Utc::now(),
        };
        
        let result = repo.register_installation(&install_result).await;
        assert!(result.is_ok());
        
        let is_installed = repo.is_installed("test-package", "1.0.0").await.unwrap();
        assert!(is_installed);
    }
}