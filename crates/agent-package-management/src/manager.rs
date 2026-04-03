//! Main package manager implementation.

use crate::cache::PackageCache;
use crate::error::{PackageError, Result};
use crate::installer::PackageInstaller;
use crate::repository::{LocalRepository, RemoteRepository, Repository};
use crate::resolver::DependencyResolver;
use crate::types::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Main package manager coordinating repositories, cache, resolver, and installer.
pub struct PackageManager {
    /// Local package cache.
    cache: PackageCache,
    /// Local repository (installed packages).
    local_repo: LocalRepository,
    /// Remote repositories.
    remote_repos: Vec<RemoteRepository>,
    /// Dependency resolver.
    resolver: DependencyResolver,
    /// Package installer.
    installer: PackageInstaller,
    /// Configuration.
    config: ManagerConfig,
}

/// Package manager configuration.
#[derive(Debug, Clone)]
pub struct ManagerConfig {
    /// Default installation directory.
    pub install_dir: PathBuf,
    /// Whether to verify checksums.
    pub verify_checksums: bool,
    /// Whether to verify signatures.
    pub verify_signatures: bool,
    /// Whether to install dependencies automatically.
    pub auto_install_deps: bool,
    /// Maximum concurrent downloads.
    pub max_concurrent_downloads: usize,
    /// Timeout for network operations in seconds.
    pub network_timeout_secs: u64,
}

impl Default for ManagerConfig {
    fn default() -> Self {
        Self {
            install_dir: PathBuf::from("/opt/agent-packages"),
            verify_checksums: true,
            verify_signatures: false,
            auto_install_deps: true,
            max_concurrent_downloads: 4,
            network_timeout_secs: 30,
        }
    }
}

impl PackageManager {
    /// Creates a new package manager with the given cache directory.
    pub fn new<P: AsRef<Path>>(cache_dir: P) -> Result<Self> {
        let cache_dir = cache_dir.as_ref();
        let config = ManagerConfig::default();
        
        let cache = PackageCache::new(cache_dir.join("cache"))?;
        let local_repo = LocalRepository::new(cache_dir.join("local"))?;
        let resolver = DependencyResolver::new();
        let installer = PackageInstaller::new(config.install_dir.clone());
        
        Ok(Self {
            cache,
            local_repo,
            remote_repos: Vec::new(),
            resolver,
            installer,
            config,
        })
    }
    
    /// Creates a new package manager with custom configuration.
    pub fn with_config<P: AsRef<Path>>(cache_dir: P, config: ManagerConfig) -> Result<Self> {
        let cache_dir = cache_dir.as_ref();
        
        let cache = PackageCache::new(cache_dir.join("cache"))?;
        let local_repo = LocalRepository::new(cache_dir.join("local"))?;
        let resolver = DependencyResolver::new();
        let installer = PackageInstaller::new(config.install_dir.clone());
        
        Ok(Self {
            cache,
            local_repo,
            remote_repos: Vec::new(),
            resolver,
            installer,
            config,
        })
    }
    
    /// Adds a remote registry to the package manager.
    pub async fn add_registry(&mut self, config: RegistryConfig) -> Result<()> {
        let repo = RemoteRepository::new(config)?;
        self.remote_repos.push(repo);
        info!("Added remote registry: {}", self.remote_repos.len());
        Ok(())
    }
    
    /// Searches for packages matching the query.
    pub async fn search(&self, query: PackageQuery) -> Result<Vec<PackageMetadata>> {
        let mut results = Vec::new();
        
        // Search local repository
        let local_results = self.local_repo.search(&query).await?;
        results.extend(local_results);
        
        // Search remote repositories
        for repo in &self.remote_repos {
            match repo.search(&query).await {
                Ok(remote_results) => results.extend(remote_results),
                Err(e) => warn!("Failed to search remote repository: {}", e),
            }
        }
        
        // Deduplicate by package ID
        let mut seen = HashMap::new();
        results.retain(|pkg| seen.insert(pkg.id.clone(), ()).is_none());
        
        // Apply limit and offset
        if let Some(limit) = query.limit {
            let offset = query.offset.unwrap_or(0);
            if offset < results.len() {
                let end = (offset + limit).min(results.len());
                results = results[offset..end].to_vec();
            } else {
                results.clear();
            }
        } else if let Some(offset) = query.offset {
            if offset < results.len() {
                results = results[offset..].to_vec();
            } else {
                results.clear();
            }
        }
        
        Ok(results)
    }
    
    /// Resolves dependencies for a package.
    pub async fn resolve(
        &self,
        package_id: &str,
        version_req: &VersionReq,
    ) -> Result<ResolutionGraph> {
        info!("Resolving dependencies for {} {}", package_id, version_req);
        
        // Find the package version
        let package_version = self.find_package_version(package_id, version_req).await?;
        
        // Resolve dependencies
        let graph = self.resolver.resolve(&package_version, |dep_id, dep_req| {
            Box::pin(self.find_package_version(dep_id, dep_req))
        }).await?;
        
        Ok(graph)
    }
    
    /// Installs a package and its dependencies.
    pub async fn install(&mut self, package_id: &str, version_req: &VersionReq) -> Result<InstallResult> {
        info!("Installing package {} {}", package_id, version_req);
        
        // Resolve dependencies
        let graph = self.resolve(package_id, version_req).await?;
        
        // Check for conflicts
        if !graph.conflicts.is_empty() {
            return Err(PackageError::DependencyResolution(
                format!("Dependency conflicts: {:?}", graph.conflicts)
            ));
        }
        
        // Download packages to cache
        let mut downloaded = Vec::new();
        for (pkg_id, pkg_version) in &graph.packages {
            if self.cache.has(pkg_id, &pkg_version.version).await? {
                debug!("Package {} {} already in cache", pkg_id, pkg_version.version);
                continue;
            }
            
            // Download from remote repository
            let archive_data = self.download_package(pkg_id, &pkg_version).await?;
            self.cache.put(pkg_id, &pkg_version, archive_data).await?;
            downloaded.push((pkg_id.clone(), pkg_version.version.clone()));
        }
        
        // Install packages
        let mut installed = Vec::new();
        for (pkg_id, pkg_version) in &graph.packages {
            // Skip if already installed
            if self.local_repo.is_installed(pkg_id, &pkg_version.version).await? {
                debug!("Package {} {} already installed", pkg_id, pkg_version.version);
                continue;
            }
            
            // Get from cache
            let archive_data = self.cache.get(pkg_id, &pkg_version.version).await?;
            
            // Install
            let install_result = self.installer.install(pkg_version, archive_data).await?;
            self.local_repo.register_installation(&install_result).await?;
            installed.push(install_result.clone());
            
            if pkg_id == package_id {
                // This is the requested package
                info!("Successfully installed {} {}", pkg_id, pkg_version.version);
                return Ok(install_result);
            }
        }
        
        Err(PackageError::PackageNotFound(package_id.to_string()))
    }
    
    /// Uninstalls a package.
    pub async fn uninstall(&mut self, package_id: &str, version: &str) -> Result<()> {
        info!("Uninstalling package {} {}", package_id, version);
        
        // Check if installed
        if !self.local_repo.is_installed(package_id, version).await? {
            return Err(PackageError::PackageNotFound(format!("{} {}", package_id, version)));
        }
        
        // Get installation info
        let install_info = self.local_repo.get_installation(package_id, version).await?;
        
        // Uninstall
        self.installer.uninstall(&install_info).await?;
        self.local_repo.unregister_installation(package_id, version).await?;
        
        info!("Successfully uninstalled {} {}", package_id, version);
        Ok(())
    }
    
    /// Updates a package to the latest version.
    pub async fn update(&mut self, package_id: &str) -> Result<InstallResult> {
        info!("Updating package {}", package_id);
        
        // Get currently installed version
        let installed = self.local_repo.get_installed_version(package_id).await?;
        
        // Find latest version
        let latest_req = VersionReq::parse("*").map_err(|e| PackageError::Semver(e))?;
        let latest = self.find_package_version(package_id, &latest_req).await?;
        
        if latest.version == installed {
            info!("Package {} is already at latest version", package_id);
            return self.local_repo.get_installation(package_id, &installed).await
                .map(|info| InstallResult {
                    package: latest,
                    install_path: info.install_path,
                    installed_deps: Vec::new(),
                    total_size_bytes: 0,
                    installed_at: chrono::Utc::now(),
                });
        }
        
        // Uninstall old version
        self.uninstall(package_id, &installed).await?;
        
        // Install new version
        let req = VersionReq::parse(&format!("={}", latest.version))
            .map_err(|e| PackageError::Semver(e))?;
        self.install(package_id, &req).await
    }
    
    /// Lists installed packages.
    pub async fn list_installed(&self) -> Result<Vec<InstallResult>> {
        self.local_repo.list_installations().await
    }
    
    /// Gets package statistics.
    pub async fn stats(&self) -> Result<PackageStats> {
        let local_stats = self.local_repo.stats().await?;
        let cache_stats = self.cache.stats().await?;
        
        Ok(PackageStats {
            total_packages: local_stats.total_packages,
            total_versions: local_stats.total_versions,
            cache_size_bytes: cache_stats.cache_size_bytes,
            packages_by_type: local_stats.packages_by_type,
            popular_packages: local_stats.popular_packages,
            last_updated: chrono::Utc::now(),
        })
    }
    
    /// Cleans the package cache.
    pub async fn clean_cache(&mut self) -> Result<()> {
        info!("Cleaning package cache");
        self.cache.clean().await
    }
    
    /// Finds a package version that matches the version requirement.
    async fn find_package_version(
        &self,
        package_id: &str,
        version_req: &VersionReq,
    ) -> Result<PackageVersion> {
        // Check local repository first
        if let Ok(version) = self.local_repo.find_version(package_id, version_req).await {
            return Ok(version);
        }
        
        // Check remote repositories
        for repo in &self.remote_repos {
            match repo.find_version(package_id, version_req).await {
                Ok(version) => return Ok(version),
                Err(_) => continue,
            }
        }
        
        Err(PackageError::PackageNotFound(package_id.to_string()))
    }
    
    /// Downloads a package from a remote repository.
    async fn download_package(
        &self,
        package_id: &str,
        version: &PackageVersion,
    ) -> Result<Vec<u8>> {
        for repo in &self.remote_repos {
            match repo.download(package_id, &version.version).await {
                Ok(data) => {
                    info!("Downloaded {} {} from remote", package_id, version.version);
                    return Ok(data);
                }
                Err(e) => {
                    debug!("Failed to download from {}: {}", repo.name(), e);
                    continue;
                }
            }
        }
        
        Err(PackageError::NetworkError(format!(
            "Failed to download package {} {} from any repository",
            package_id, version.version
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_package_manager_creation() {
        let temp_dir = tempdir().unwrap();
        let manager = PackageManager::new(temp_dir.path()).unwrap();
        
        assert!(manager.remote_repos.is_empty());
    }
    
    #[tokio::test]
    async fn test_add_registry() {
        let temp_dir = tempdir().unwrap();
        let mut manager = PackageManager::new(temp_dir.path()).unwrap();
        
        let config = RegistryConfig {
            url: "https://example.com".to_string(),
            auth_token: None,
            timeout_secs: 30,
            verify_ssl: true,
        };
        
        let result = manager.add_registry(config).await;
        assert!(result.is_ok());
        assert_eq!(manager.remote_repos.len(), 1);
    }
}