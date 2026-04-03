//! Image management utilities.

use crate::error::{ContainerError, Result};
use crate::types::*;
use std::path::Path;
use tracing::{debug, info};

/// Image manager for container images.
pub struct ImageManager {
    /// Local image cache directory.
    cache_dir: std::path::PathBuf,
}

impl ImageManager {
    /// Creates a new image manager.
    pub fn new<P: AsRef<Path>>(cache_dir: P) -> Result<Self> {
        let cache_dir = cache_dir.as_ref().to_path_buf();
        
        // Create cache directory if it doesn't exist
        if !cache_dir.exists() {
            std::fs::create_dir_all(&cache_dir).map_err(|e| {
                ContainerError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to create image cache directory: {}", e),
                ))
            })?;
        }
        
        Ok(Self { cache_dir })
    }
    
    /// Gets the image cache directory.
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }
    
    /// Validates an image reference.
    pub fn validate_image_ref(&self, image_ref: &str) -> Result<()> {
        // Basic validation: should have format [registry/]repository[:tag|@digest]
        if image_ref.is_empty() {
            return Err(ContainerError::InvalidArgument("Image reference cannot be empty".to_string()));
        }
        
        // Check for invalid characters
        if image_ref.contains("  ") || image_ref.starts_with('.') || image_ref.ends_with('.') {
            return Err(ContainerError::InvalidArgument(
                format!("Invalid image reference: {}", image_ref)
            ));
        }
        
        Ok(())
    }
    
    /// Parses an image reference into repository and tag/digest.
    pub fn parse_image_ref(&self, image_ref: &str) -> Result<(String, Option<String>)> {
        self.validate_image_ref(image_ref)?;
        
        // Split by @ for digest
        if let Some(at_pos) = image_ref.find('@') {
            let repository = image_ref[..at_pos].to_string();
            let digest = image_ref[at_pos + 1..].to_string();
            return Ok((repository, Some(digest)));
        }
        
        // Split by : for tag
        if let Some(colon_pos) = image_ref.rfind(':') {
            // Check if it's a port number (registry with port)
            let before_colon = &image_ref[..colon_pos];
            if before_colon.contains('/') {
                let after_slash = before_colon.split('/').last().unwrap_or("");
                if after_slash.contains('.') || after_slash.contains(':') {
                    // Could be a registry port, not a tag
                    return Ok((image_ref.to_string(), None));
                }
            }
            
            let repository = image_ref[..colon_pos].to_string();
            let tag = image_ref[colon_pos + 1..].to_string();
            return Ok((repository, Some(tag)));
        }
        
        // No tag or digest
        Ok((image_ref.to_string(), None))
    }
    
    /// Gets the default tag for an image.
    pub fn default_tag(&self, repository: &str) -> String {
        // Check if it's a known base image
        if repository.contains("alpine") {
            "latest".to_string()
        } else if repository.contains("ubuntu") {
            "latest".to_string()
        } else if repository.contains("debian") {
            "latest".to_string()
        } else {
            "latest".to_string()
        }
    }
    
    /// Creates a mock image info for testing.
    pub fn create_mock_image(&self, repository: &str, tag: &str) -> ImageInfo {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let id: u64 = rng.gen();
        
        ImageInfo {
            id: format!("sha256:{:x}", id),
            repository: repository.to_string(),
            tag: tag.to_string(),
            digest: None,
            created: chrono::Utc::now(),
            size: 100 * 1024 * 1024, // 100 MB
            labels: std::collections::HashMap::new(),
        }
    }
    
    /// Calculates the estimated download size for an image.
    pub fn estimate_download_size(&self, image_info: &ImageInfo) -> u64 {
        // Simple estimation: image size + 10% overhead
        (image_info.size as f64 * 1.1) as u64
    }
    
    /// Checks if an image is compatible with the current platform.
    pub fn check_platform_compatibility(&self, image_info: &ImageInfo) -> Result<()> {
        // In a real implementation, this would check architecture and OS
        // For now, we'll assume all images are compatible
        debug!("Platform compatibility check for {}:{} (simulated)", 
            image_info.repository, image_info.tag);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_validate_image_ref() {
        let temp_dir = tempdir().unwrap();
        let manager = ImageManager::new(temp_dir.path()).unwrap();
        
        // Valid references
        assert!(manager.validate_image_ref("alpine").is_ok());
        assert!(manager.validate_image_ref("alpine:latest").is_ok());
        assert!(manager.validate_image_ref("docker.io/library/alpine:latest").is_ok());
        assert!(manager.validate_image_ref("myregistry.com/myimage@sha256:abc123").is_ok());
        
        // Invalid references
        assert!(manager.validate_image_ref("").is_err());
        assert!(manager.validate_image_ref("  ").is_err());
    }
    
    #[test]
    fn test_parse_image_ref() {
        let temp_dir = tempdir().unwrap();
        let manager = ImageManager::new(temp_dir.path()).unwrap();
        
        // Test without tag
        let (repo, tag) = manager.parse_image_ref("alpine").unwrap();
        assert_eq!(repo, "alpine");
        assert_eq!(tag, None);
        
        // Test with tag
        let (repo, tag) = manager.parse_image_ref("alpine:latest").unwrap();
        assert_eq!(repo, "alpine");
        assert_eq!(tag, Some("latest".to_string()));
        
        // Test with digest
        let (repo, tag) = manager.parse_image_ref("alpine@sha256:abc123").unwrap();
        assert_eq!(repo, "alpine");
        assert_eq!(tag, Some("sha256:abc123".to_string()));
        
        // Test with registry and tag
        let (repo, tag) = manager.parse_image_ref("docker.io/library/alpine:latest").unwrap();
        assert_eq!(repo, "docker.io/library/alpine");
        assert_eq!(tag, Some("latest".to_string()));
    }
    
    #[test]
    fn test_default_tag() {
        let temp_dir = tempdir().unwrap();
        let manager = ImageManager::new(temp_dir.path()).unwrap();
        
        assert_eq!(manager.default_tag("alpine"), "latest");
        assert_eq!(manager.default_tag("ubuntu"), "latest");
        assert_eq!(manager.default_tag("my-custom-image"), "latest");
    }
}