//! Backend storage for secrets.

use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::{SecretsError, Result};
use crate::model::{Secret, SecretQuery, SecretVersion};
use crate::crypto::{KeyManager, EncryptionKey};

/// Trait for secret storage backends.
#[async_trait]
pub trait Backend: Send + Sync {
    /// Store a secret.
    async fn put(&self, secret: Secret) -> Result<()>;
    
    /// Retrieve a secret by ID.
    async fn get(&self, id: &str) -> Result<Secret>;
    
    /// Delete a secret.
    async fn delete(&self, id: &str) -> Result<()>;
    
    /// List secrets matching a query.
    async fn list(&self, query: &SecretQuery) -> Result<Vec<Secret>>;
    
    /// Update a secret.
    async fn update(&self, secret: Secret) -> Result<()>;
    
    /// Check if a secret exists.
    async fn exists(&self, id: &str) -> Result<bool>;
    
    /// Get version history of a secret.
    async fn versions(&self, id: &str) -> Result<Vec<SecretVersion>>;
    
    /// Backup the backend.
    async fn backup(&self, destination: &str) -> Result<()>;
    
    /// Restore from backup.
    async fn restore(&self, source: &str) -> Result<()>;
}

/// In‑memory backend (for testing and development).
#[derive(Debug, Default)]
pub struct InMemoryBackend {
    secrets: RwLock<HashMap<String, Secret>>,
    versions: RwLock<HashMap<String, Vec<SecretVersion>>>,
}

impl InMemoryBackend {
    /// Create a new in‑memory backend.
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl Backend for InMemoryBackend {
    async fn put(&self, secret: Secret) -> Result<()> {
        let mut secrets = self.secrets.write().await;
        if secrets.contains_key(&secret.id) {
            return Err(SecretsError::AlreadyExists(secret.id.clone()));
        }
        secrets.insert(secret.id.clone(), secret);
        Ok(())
    }
    
    async fn get(&self, id: &str) -> Result<Secret> {
        let secrets = self.secrets.read().await;
        secrets.get(id)
            .cloned()
            .ok_or_else(|| SecretsError::NotFound(id.to_string()))
    }
    
    async fn delete(&self, id: &str) -> Result<()> {
        let mut secrets = self.secrets.write().await;
        secrets.remove(id)
            .ok_or_else(|| SecretsError::NotFound(id.to_string()))?;
        Ok(())
    }
    
    async fn list(&self, query: &SecretQuery) -> Result<Vec<Secret>> {
        let secrets = self.secrets.read().await;
        let mut results = Vec::new();
        
        for secret in secrets.values() {
            // Filter by ID prefix
            if let Some(prefix) = &query.id_prefix {
                if !secret.id.starts_with(prefix) {
                    continue;
                }
            }
            
            // Filter by tags
            if !query.tags.is_empty() {
                let mut matches = false;
                for tag in &query.tags {
                    if secret.tags.contains(tag) {
                        matches = true;
                        break;
                    }
                }
                if !matches {
                    continue;
                }
            }
            
            // Filter by expiration
            if !query.include_expired && secret.is_expired() {
                continue;
            }
            
            results.push(secret.clone());
        }
        
        // Apply pagination
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(usize::MAX);
        
        Ok(results.into_iter()
            .skip(offset)
            .take(limit)
            .collect())
    }
    
    async fn update(&self, secret: Secret) -> Result<()> {
        let mut secrets = self.secrets.write().await;
        if !secrets.contains_key(&secret.id) {
            return Err(SecretsError::NotFound(secret.id.clone()));
        }
        secrets.insert(secret.id.clone(), secret);
        Ok(())
    }
    
    async fn exists(&self, id: &str) -> Result<bool> {
        let secrets = self.secrets.read().await;
        Ok(secrets.contains_key(id))
    }
    
    async fn versions(&self, id: &str) -> Result<Vec<SecretVersion>> {
        let versions = self.versions.read().await;
        Ok(versions.get(id)
            .cloned()
            .unwrap_or_default())
    }
    
    async fn backup(&self, _destination: &str) -> Result<()> {
        // In‑memory backup is a no‑op
        Ok(())
    }
    
    async fn restore(&self, _source: &str) -> Result<()> {
        // In‑memory restore is a no‑op
        Ok(())
    }
}

/// Encrypted file backend (stores secrets as encrypted JSON files).
#[derive(Debug)]
pub struct EncryptedFileBackend {
    directory: PathBuf,
    key_manager: Arc<KeyManager>,
}

impl EncryptedFileBackend {
    /// Create a new encrypted file backend.
    pub fn new(directory: impl Into<PathBuf>, key_manager: Arc<KeyManager>) -> Result<Self> {
        let dir = directory.into();
        std::fs::create_dir_all(&dir)
            .map_err(|e| SecretsError::Io(e))?;
        
        Ok(Self {
            directory: dir,
            key_manager,
        })
    }
    
    /// Get file path for a secret.
    fn secret_path(&self, id: &str) -> PathBuf {
        // Sanitize ID for filename
        let sanitized = id.replace(|c: char| !c.is_alphanumeric(), "_");
        self.directory.join(format!("{}.json.enc", sanitized))
    }
    
    /// Get versions directory for a secret.
    fn versions_dir(&self, id: &str) -> PathBuf {
        let sanitized = id.replace(|c: char| !c.is_alphanumeric(), "_");
        self.directory.join("versions").join(sanitized)
    }
}

#[async_trait]
impl Backend for EncryptedFileBackend {
    async fn put(&self, secret: Secret) -> Result<()> {
        let path = self.secret_path(&secret.id);
        if path.exists() {
            return Err(SecretsError::AlreadyExists(secret.id.clone()));
        }
        
        // Serialize and encrypt
        let json = serde_json::to_vec(&secret)
            .map_err(|e| SecretsError::Serialization(e))?;
        
        #[cfg(feature = "encryption")]
        let encrypted = self.key_manager.encrypt(&json).await?;
        
        #[cfg(not(feature = "encryption"))]
        let encrypted = json; // No encryption
        
        tokio::fs::write(&path, encrypted).await
            .map_err(|e| SecretsError::Io(e))?;
        
        Ok(())
    }
    
    async fn get(&self, id: &str) -> Result<Secret> {
        let path = self.secret_path(id);
        if !path.exists() {
            return Err(SecretsError::NotFound(id.to_string()));
        }
        
        let encrypted = tokio::fs::read(&path).await
            .map_err(|e| SecretsError::Io(e))?;
        
        #[cfg(feature = "encryption")]
        let json = self.key_manager.decrypt(&encrypted, "default").await?;
        
        #[cfg(not(feature = "encryption"))]
        let json = encrypted;
        
        let secret: Secret = serde_json::from_slice(&json)
            .map_err(|e| SecretsError::Serialization(e))?;
        
        Ok(secret)
    }
    
    async fn delete(&self, id: &str) -> Result<()> {
        let path = self.secret_path(id);
        if !path.exists() {
            return Err(SecretsError::NotFound(id.to_string()));
        }
        
        tokio::fs::remove_file(&path).await
            .map_err(|e| SecretsError::Io(e))?;
        
        // Also delete versions
        let versions_dir = self.versions_dir(id);
        if versions_dir.exists() {
            tokio::fs::remove_dir_all(versions_dir).await
                .map_err(|e| SecretsError::Io(e))?;
        }
        
        Ok(())
    }
    
    async fn list(&self, query: &SecretQuery) -> Result<Vec<Secret>> {
        let mut results = Vec::new();
        let mut entries = tokio::fs::read_dir(&self.directory).await
            .map_err(|e| SecretsError::Io(e))?;
        
        while let Some(entry) = entries.next_entry().await
            .map_err(|e| SecretsError::Io(e))? {
            
            let path = entry.path();
            if path.is_file() && path.extension().map(|e| e == "enc").unwrap_or(false) {
                let filename = path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");
                
                // Remove .json from filename
                let id = filename.trim_end_matches(".json");
                
                // Basic filtering by ID prefix
                if let Some(prefix) = &query.id_prefix {
                    if !id.starts_with(prefix) {
                        continue;
                    }
                }
                
                // Try to load the secret
                match self.get(id).await {
                    Ok(secret) => {
                        // Filter by tags
                        if !query.tags.is_empty() {
                            let mut matches = false;
                            for tag in &query.tags {
                                if secret.tags.contains(tag) {
                                    matches = true;
                                    break;
                                }
                            }
                            if !matches {
                                continue;
                            }
                        }
                        
                        // Filter by expiration
                        if !query.include_expired && secret.is_expired() {
                            continue;
                        }
                        
                        results.push(secret);
                    }
                    Err(_) => continue, // Skip corrupted files
                }
            }
        }
        
        // Apply pagination
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(usize::MAX);
        
        Ok(results.into_iter()
            .skip(offset)
            .take(limit)
            .collect())
    }
    
    async fn update(&self, secret: Secret) -> Result<()> {
        let path = self.secret_path(&secret.id);
        if !path.exists() {
            return Err(SecretsError::NotFound(secret.id.clone()));
        }
        
        // Create version backup
        let versions_dir = self.versions_dir(&secret.id);
        tokio::fs::create_dir_all(&versions_dir).await
            .map_err(|e| SecretsError::Io(e))?;
        
        let old_content = tokio::fs::read(&path).await
            .map_err(|e| SecretsError::Io(e))?;
        
        let version_path = versions_dir.join(format!("v{}.enc", secret.metadata.version));
        tokio::fs::write(&version_path, old_content).await
            .map_err(|e| SecretsError::Io(e))?;
        
        // Write new secret
        let json = serde_json::to_vec(&secret)
            .map_err(|e| SecretsError::Serialization(e))?;
        
        #[cfg(feature = "encryption")]
        let encrypted = self.key_manager.encrypt(&json).await?;
        
        #[cfg(not(feature = "encryption"))]
        let encrypted = json;
        
        tokio::fs::write(&path, encrypted).await
            .map_err(|e| SecretsError::Io(e))?;
        
        Ok(())
    }
    
    async fn exists(&self, id: &str) -> Result<bool> {
        let path = self.secret_path(id);
        Ok(path.exists())
    }
    
    async fn versions(&self, id: &str) -> Result<Vec<SecretVersion>> {
        let versions_dir = self.versions_dir(id);
        if !versions_dir.exists() {
            return Ok(Vec::new());
        }
        
        let mut versions = Vec::new();
        let mut entries = tokio::fs::read_dir(&versions_dir).await
            .map_err(|e| SecretsError::Io(e))?;
        
        while let Some(entry) = entries.next_entry().await
            .map_err(|e| SecretsError::Io(e))? {
            
            let path = entry.path();
            if path.is_file() && path.extension().map(|e| e == "enc").unwrap_or(false) {
                // Try to load version
                let encrypted = tokio::fs::read(&path).await
                    .map_err(|e| SecretsError::Io(e))?;
                
                #[cfg(feature = "encryption")]
                let json = self.key_manager.decrypt(&encrypted, "default").await?;
                
                #[cfg(not(feature = "encryption"))]
                let json = encrypted;
                
                let version: SecretVersion = serde_json::from_slice(&json)
                    .map_err(|e| SecretsError::Serialization(e))?;
                
                versions.push(version);
            }
        }
        
        Ok(versions)
    }
    
    async fn backup(&self, destination: &str) -> Result<()> {
        use std::process::Command;
        
        // Simple copy using OS commands
        #[cfg(target_family = "unix")]
        let status = Command::new("cp")
            .arg("-r")
            .arg(&self.directory)
            .arg(destination)
            .status()
            .map_err(|e| SecretsError::Io(e))?;
        
        #[cfg(target_family = "windows")]
        let status = Command::new("xcopy")
            .arg(&self.directory)
            .arg(destination)
            .arg("/E")
            .arg("/I")
            .status()
            .map_err(|e| SecretsError::Io(e))?;
        
        if !status.success() {
            return Err(SecretsError::Backend("backup command failed".into()));
        }
        
        Ok(())
    }
    
    async fn restore(&self, source: &str) -> Result<()> {
        use std::process::Command;
        
        // Remove existing directory
        if self.directory.exists() {
            tokio::fs::remove_dir_all(&self.directory).await
                .map_err(|e| SecretsError::Io(e))?;
        }
        
        // Copy backup
        #[cfg(target_family = "unix")]
        let status = Command::new("cp")
            .arg("-r")
            .arg(source)
            .arg(&self.directory)
            .status()
            .map_err(|e| SecretsError::Io(e))?;
        
        #[cfg(target_family = "windows")]
        let status = Command::new("xcopy")
            .arg(source)
            .arg(&self.directory)
            .arg("/E")
            .arg("/I")
            .status()
            .map_err(|e| SecretsError::Io(e))?;
        
        if !status.success() {
            return Err(SecretsError::Backend("restore command failed".into()));
        }
        
        Ok(())
    }
}