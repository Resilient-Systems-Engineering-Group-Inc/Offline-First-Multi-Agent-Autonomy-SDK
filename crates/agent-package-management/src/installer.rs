//! Package installation and extraction.

use crate::error::{PackageError, Result};
use crate::types::*;
use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tar::Archive;
use tempfile::TempDir;
use tokio::fs as tokio_fs;
use tracing::{debug, error, info, warn};

/// Package installer that extracts archives and places files.
pub struct PackageInstaller {
    /// Default installation directory.
    install_dir: PathBuf,
}

impl PackageInstaller {
    /// Creates a new package installer.
    pub fn new<P: AsRef<Path>>(install_dir: P) -> Self {
        Self {
            install_dir: install_dir.as_ref().to_path_buf(),
        }
    }
    
    /// Installs a package from archive data.
    pub async fn install(
        &self,
        package: &PackageVersion,
        archive_data: Vec<u8>,
    ) -> Result<InstallResult> {
        info!("Installing package {} {}", package.package_id, package.version);
        
        // Verify checksum
        self.verify_checksum(package, &archive_data).await?;
        
        // Create temporary directory for extraction
        let temp_dir = TempDir::new().map_err(|e| {
            PackageError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to create temp directory: {}", e),
            ))
        })?;
        
        // Extract archive
        let extracted_path = self.extract_archive(&archive_data, temp_dir.path()).await?;
        
        // Determine installation path
        let install_path = self.determine_install_path(package);
        
        // Create installation directory
        tokio_fs::create_dir_all(&install_path).await.map_err(|e| {
            PackageError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to create installation directory: {}", e),
            ))
        })?;
        
        // Copy files
        self.copy_files(&extracted_path, &install_path).await?;
        
        // Create metadata file
        self.create_metadata_file(package, &install_path).await?;
        
        // Clean up temp directory (automatically done by TempDir drop)
        
        info!("Successfully installed to {}", install_path.display());
        
        Ok(InstallResult {
            package: package.clone(),
            install_path: install_path.to_string_lossy().to_string(),
            installed_deps: Vec::new(), // Will be filled by caller
            total_size_bytes: self.calculate_directory_size(&install_path).await?,
            installed_at: chrono::Utc::now(),
        })
    }
    
    /// Uninstalls a package.
    pub async fn uninstall(&self, install_info: &InstallResult) -> Result<()> {
        let install_path = Path::new(&install_info.install_path);
        
        if !install_path.exists() {
            warn!("Installation path does not exist: {}", install_path.display());
            return Ok(());
        }
        
        info!("Uninstalling package from {}", install_path.display());
        
        // Remove directory
        tokio_fs::remove_dir_all(install_path).await.map_err(|e| {
            PackageError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to remove installation directory: {}", e),
            ))
        })?;
        
        info!("Successfully uninstalled package");
        Ok(())
    }
    
    /// Verifies package checksum.
    async fn verify_checksum(&self, package: &PackageVersion, data: &[u8]) -> Result<()> {
        if package.checksum.is_empty() {
            warn!("Package has no checksum, skipping verification");
            return Ok(());
        }
        
        let mut hasher = Sha256::new();
        hasher.update(data);
        let computed = hex::encode(hasher.finalize());
        
        if computed != package.checksum {
            return Err(PackageError::ChecksumMismatch(
                package.checksum.clone(),
                computed,
            ));
        }
        
        debug!("Checksum verified successfully");
        Ok(())
    }
    
    /// Extracts an archive to a temporary directory.
    async fn extract_archive(&self, data: &[u8], dest: &Path) -> Result<PathBuf> {
        // Determine archive format (simplified - assume tar.gz)
        let decoder = GzDecoder::new(data);
        let mut archive = Archive::new(decoder);
        
        // Extract all files
        archive.unpack(dest).map_err(|e| {
            PackageError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to extract archive: {}", e),
            ))
        })?;
        
        // Find the root directory (usually the first directory in the archive)
        let entries = fs::read_dir(dest).map_err(|e| {
            PackageError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to read extracted directory: {}", e),
            ))
        })?;
        
        for entry in entries {
            let entry = entry.map_err(|e| {
                PackageError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to read directory entry: {}", e),
                ))
            })?;
            
            let path = entry.path();
            if path.is_dir() {
                return Ok(path);
            }
        }
        
        // If no directory found, return the dest itself
        Ok(dest.to_path_buf())
    }
    
    /// Determines the installation path for a package.
    fn determine_install_path(&self, package: &PackageVersion) -> PathBuf {
        // Format: <install_dir>/<package_id>/<version>
        self.install_dir
            .join(&package.package_id)
            .join(&package.version)
    }
    
    /// Copies files from source to destination.
    async fn copy_files(&self, source: &Path, dest: &Path) -> Result<()> {
        debug!("Copying files from {} to {}", source.display(), dest.display());
        
        // Create destination directory if it doesn't exist
        tokio_fs::create_dir_all(dest).await.map_err(|e| {
            PackageError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to create destination directory: {}", e),
            ))
        })?;
        
        // Walk through source directory
        let mut entries = tokio_fs::read_dir(source).await.map_err(|e| {
            PackageError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to read source directory: {}", e),
            ))
        })?;
        
        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            PackageError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to read directory entry: {}", e),
            ))
        })? {
            let source_path = entry.path();
            let file_name = entry.file_name();
            let dest_path = dest.join(file_name);
            
            if source_path.is_dir() {
                // Recursively copy directory
                self.copy_files(&source_path, &dest_path).await?;
            } else {
                // Copy file
                tokio_fs::copy(&source_path, &dest_path).await.map_err(|e| {
                    PackageError::Io(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Failed to copy file {}: {}", source_path.display(), e),
                    ))
                })?;
            }
        }
        
        Ok(())
    }
    
    /// Creates a metadata file in the installation directory.
    async fn create_metadata_file(
        &self,
        package: &PackageVersion,
        install_path: &Path,
    ) -> Result<()> {
        let metadata_path = install_path.join("package.json");
        
        let metadata = serde_json::json!({
            "package_id": package.package_id,
            "version": package.version,
            "semver": package.semver.to_string(),
            "changelog": package.changelog,
            "checksum": package.checksum,
            "size_bytes": package.size_bytes,
            "dependencies": package.dependencies,
            "platforms": package.platforms,
            "install_instructions": package.install_instructions,
            "is_default": package.is_default,
            "is_deprecated": package.is_deprecated,
            "created_at": package.created_at.to_rfc3339(),
            "author": package.author,
            "installed_at": chrono::Utc::now().to_rfc3339(),
        });
        
        let content = serde_json::to_vec_pretty(&metadata)
            .map_err(|e| PackageError::Json(e))?;
        
        tokio_fs::write(&metadata_path, &content).await.map_err(|e| {
            PackageError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to write metadata file: {}", e),
            ))
        })?;
        
        Ok(())
    }
    
    /// Calculates the total size of a directory.
    async fn calculate_directory_size(&self, path: &Path) -> Result<u64> {
        let mut total_size = 0;
        
        let mut stack = vec![path.to_path_buf()];
        
        while let Some(current_path) = stack.pop() {
            let mut entries = tokio_fs::read_dir(&current_path).await.map_err(|e| {
                PackageError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to read directory {}: {}", current_path.display(), e),
                ))
            })?;
            
            while let Some(entry) = entries.next_entry().await.map_err(|e| {
                PackageError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to read directory entry: {}", e),
                ))
            })? {
                let entry_path = entry.path();
                
                if entry_path.is_dir() {
                    stack.push(entry_path);
                } else {
                    let metadata = entry.metadata().await.map_err(|e| {
                        PackageError::Io(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("Failed to get file metadata: {}", e),
                        ))
                    })?;
                    total_size += metadata.len();
                }
            }
        }
        
        Ok(total_size)
    }
    
    /// Validates that a package can be installed on the current platform.
    pub fn validate_platform(&self, package: &PackageVersion) -> Result<()> {
        // Get current platform
        let current_os = std::env::consts::OS;
        let current_arch = std::env::consts::ARCH;
        
        // If package has no platform constraints, it's compatible
        if package.platforms.is_empty() {
            return Ok(());
        }
        
        // Check if current platform matches any of the supported platforms
        for platform in &package.platforms {
            if platform.os == current_os && platform.arch == current_arch {
                return Ok(());
            }
        }
        
        Err(PackageError::InstallationFailed(format!(
            "Package not compatible with platform {}/{}",
            current_os, current_arch
        )))
    }
    
    /// Creates a symbolic link for easy access to the package.
    pub async fn create_symlink(
        &self,
        package: &PackageVersion,
        install_path: &Path,
    ) -> Result<()> {
        let link_path = self.install_dir
            .join(&package.package_id)
            .join("current");
        
        // Remove existing symlink if it exists
        if link_path.exists() {
            tokio_fs::remove_file(&link_path).await.map_err(|e| {
                PackageError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to remove existing symlink: {}", e),
                ))
            })?;
        }
        
        // Create symlink
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            symlink(install_path, &link_path).map_err(|e| {
                PackageError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to create symlink: {}", e),
                ))
            })?;
        }
        
        #[cfg(windows)]
        {
            use std::os::windows::fs::symlink_dir;
            symlink_dir(install_path, &link_path).map_err(|e| {
                PackageError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to create symlink: {}", e),
                ))
            })?;
        }
        
        debug!("Created symlink: {} -> {}", link_path.display(), install_path.display());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    fn create_test_package() -> PackageVersion {
        PackageVersion {
            package_id: "test-package".to_string(),
            version: "1.0.0".to_string(),
            semver: semver::Version::parse("1.0.0").unwrap(),
            changelog: "Test package".to_string(),
            checksum: "".to_string(), // Empty for tests
            size_bytes: 0,
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
    async fn test_package_installer_creation() {
        let temp_dir = tempdir().unwrap();
        let installer = PackageInstaller::new(temp_dir.path());
        
        assert_eq!(installer.install_dir, temp_dir.path());
    }
    
    #[tokio::test]
    async fn test_determine_install_path() {
        let temp_dir = tempdir().unwrap();
        let installer = PackageInstaller::new(temp_dir.path());
        
        let package = create_test_package();
        let install_path = installer.determine_install_path(&package);
        
        let expected_path = temp_dir.path()
            .join("test-package")
            .join("1.0.0");
        
        assert_eq!(install_path, expected_path);
    }
    
    #[tokio::test]
    async fn test_validate_platform() {
        let temp_dir = tempdir().unwrap();
        let installer = PackageInstaller::new(temp_dir.path());
        
        let mut package = create_test_package();
        
        // Empty platforms should pass
        let result = installer.validate_platform(&package);
        assert!(result.is_ok());
        
        // Add platform constraint matching current platform
        package.platforms.push(Platform {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            constraints: HashMap::new(),
        });
        
        let result = installer.validate_platform(&package);
        assert!(result.is_ok());
        
        // Add non-matching platform
        package.platforms.push(Platform {
            os: "unknown".to_string(),
            arch: "unknown".to_string(),
            constraints: HashMap::new(),
        });
        
        // Should still pass because at least one matches
        let result = installer.validate_platform(&package);
        assert!(result.is_ok());
        
        // Test with only non-matching platforms
        let mut package2 = create_test_package();
        package2.platforms = vec![Platform {
            os: "unknown".to_string(),
            arch: "unknown".to_string(),
            constraints: HashMap::new(),
        }];
        
        let result = installer.validate_platform(&package2);
        assert!(result.is_err());
    }
}