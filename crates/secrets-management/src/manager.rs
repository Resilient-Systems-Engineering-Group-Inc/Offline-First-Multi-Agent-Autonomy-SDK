//! High‑level secrets manager.

use async_trait::async_trait;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::backend::{Backend, InMemoryBackend};
use crate::crypto::{KeyManager, KeyRotationStrategy};
use crate::error::{SecretsError, Result};
use crate::model::{Secret, SecretQuery, SecretVersion, AccessPolicy};
use crate::policy::PolicyEngine;
use crate::rotation::RotationScheduler;
use crate::transport::SecretTransport;

/// Main secrets manager.
#[derive(Debug)]
pub struct SecretsManager<B: Backend> {
    backend: Arc<B>,
    key_manager: Arc<KeyManager>,
    policy_engine: Arc<PolicyEngine>,
    rotation_scheduler: Option<Arc<RotationScheduler>>,
    transport: Option<Arc<dyn SecretTransport>>,
}

impl<B: Backend> SecretsManager<B> {
    /// Create a new secrets manager with the given backend.
    pub async fn new(backend: B) -> Result<Self> {
        let key_manager = KeyManager::new().await?;
        let policy_engine = PolicyEngine::new();
        
        Ok(Self {
            backend: Arc::new(backend),
            key_manager: Arc::new(key_manager),
            policy_engine: Arc::new(policy_engine),
            rotation_scheduler: None,
            transport: None,
        })
    }
    
    /// Create a new secrets manager with in‑memory backend (for testing).
    pub async fn in_memory() -> Result<Self> {
        let backend = InMemoryBackend::new();
        Self::new(backend).await
    }
    
    /// Set a rotation scheduler.
    pub fn with_rotation_scheduler(mut self, scheduler: RotationScheduler) -> Self {
        self.rotation_scheduler = Some(Arc::new(scheduler));
        self
    }
    
    /// Set a transport for distributed secret sharing.
    pub fn with_transport<T: SecretTransport + 'static>(mut self, transport: T) -> Self {
        self.transport = Some(Arc::new(transport));
        self
    }
    
    /// Store a secret.
    pub async fn put(&self, secret: Secret) -> Result<()> {
        // Check access policies
        if !self.policy_engine.can_write(&secret.id).await {
            return Err(SecretsError::AccessDenied(
                secret.id.clone(),
                "write permission denied".into()
            ));
        }
        
        // Encrypt the secret value if not already encrypted
        // (In real implementation, we would encrypt here)
        
        // Store in backend
        self.backend.put(secret).await?;
        
        // Notify transport if available
        if let Some(transport) = &self.transport {
            // In a real implementation, we would broadcast the secret
            // to other agents that have read permissions
            transport.secret_updated(&secret.id).await?;
        }
        
        Ok(())
    }
    
    /// Retrieve a secret.
    pub async fn get(&self, id: &str) -> Result<Secret> {
        // Check access policies
        if !self.policy_engine.can_read(id).await {
            return Err(SecretsError::AccessDenied(
                id.to_string(),
                "read permission denied".into()
            ));
        }
        
        let secret = self.backend.get(id).await?;
        
        // Check if secret is expired
        if secret.is_expired() {
            return Err(SecretsError::InvalidFormat(
                format!("secret {} has expired", id)
            ));
        }
        
        // Update access count in policies
        self.policy_engine.record_access(id).await;
        
        Ok(secret)
    }
    
    /// Delete a secret.
    pub async fn delete(&self, id: &str) -> Result<()> {
        // Check access policies
        if !self.policy_engine.can_delete(id).await {
            return Err(SecretsError::AccessDenied(
                id.to_string(),
                "delete permission denied".into()
            ));
        }
        
        self.backend.delete(id).await?;
        
        // Notify transport
        if let Some(transport) = &self.transport {
            transport.secret_deleted(id).await?;
        }
        
        Ok(())
    }
    
    /// List secrets matching a query.
    pub async fn list(&self, query: &SecretQuery) -> Result<Vec<Secret>> {
        // Check if user has list permission
        if !self.policy_engine.can_list().await {
            return Err(SecretsError::AccessDenied(
                "".into(),
                "list permission denied".into()
            ));
        }
        
        let secrets = self.backend.list(query).await?;
        
        // Filter out secrets the user cannot read
        let mut filtered = Vec::new();
        for secret in secrets {
            if self.policy_engine.can_read(&secret.id).await {
                filtered.push(secret);
            }
        }
        
        Ok(filtered)
    }
    
    /// Update a secret.
    pub async fn update(&self, secret: Secret) -> Result<()> {
        // Check access policies
        if !self.policy_engine.can_write(&secret.id).await {
            return Err(SecretsError::AccessDenied(
                secret.id.clone(),
                "write permission denied".into()
            ));
        }
        
        // Update metadata
        let mut secret = secret;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        secret.metadata.updated_at = now;
        secret.metadata.version += 1;
        
        self.backend.update(secret).await?;
        
        // Notify transport
        if let Some(transport) = &self.transport {
            transport.secret_updated(&secret.id).await?;
        }
        
        Ok(())
    }
    
    /// Rotate a secret (change its value).
    pub async fn rotate(&self, id: &str, new_value: impl Into<String>) -> Result<()> {
        // Check access policies
        if !self.policy_engine.can_rotate(id).await {
            return Err(SecretsError::AccessDenied(
                id.to_string(),
                "rotate permission denied".into()
            ));
        }
        
        let mut secret = self.backend.get(id).await?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Create a version entry for the old value
        let old_version = SecretVersion {
            version: secret.metadata.version,
            encrypted_value: secret.encrypted_value.clone(),
            created_at: secret.metadata.created_at,
            active_from: secret.metadata.last_rotated.unwrap_or(secret.metadata.created_at),
            active_to: Some(now),
        };
        
        // Update secret
        secret.encrypted_value = new_value.into();
        secret.metadata.version += 1;
        secret.metadata.updated_at = now;
        secret.metadata.last_rotated = Some(now);
        
        // Store version history (in a real implementation)
        // self.backend.store_version(id, old_version).await?;
        
        // Update the secret
        self.backend.update(secret).await?;
        
        // Notify transport
        if let Some(transport) = &self.transport {
            transport.secret_rotated(id).await?;
        }
        
        Ok(())
    }
    
    /// Check if a secret exists.
    pub async fn exists(&self, id: &str) -> Result<bool> {
        self.backend.exists(id).await
    }
    
    /// Get version history of a secret.
    pub async fn versions(&self, id: &str) -> Result<Vec<SecretVersion>> {
        // Check access policies
        if !self.policy_engine.can_read_versions(id).await {
            return Err(SecretsError::AccessDenied(
                id.to_string(),
                "version read permission denied".into()
            ));
        }
        
        self.backend.versions(id).await
    }
    
    /// Add an access policy to a secret.
    pub async fn add_policy(&self, id: &str, policy: AccessPolicy) -> Result<()> {
        // Only secret owners can modify policies
        if !self.policy_engine.can_manage_policies(id).await {
            return Err(SecretsError::AccessDenied(
                id.to_string(),
                "policy management permission denied".into()
            ));
        }
        
        let mut secret = self.backend.get(id).await?;
        secret.policies.push(policy);
        self.backend.update(secret).await?;
        
        Ok(())
    }
    
    /// Remove an access policy from a secret.
    pub async fn remove_policy(&self, id: &str, policy_id: &str) -> Result<()> {
        if !self.policy_engine.can_manage_policies(id).await {
            return Err(SecretsError::AccessDenied(
                id.to_string(),
                "policy management permission denied".into()
            ));
        }
        
        let mut secret = self.backend.get(id).await?;
        secret.policies.retain(|p| p.id != policy_id);
        self.backend.update(secret).await?;
        
        Ok(())
    }
    
    /// Rotate encryption keys.
    pub async fn rotate_keys(&self, algorithm: crate::crypto::KeyAlgorithm) -> Result<String> {
        self.key_manager.rotate(algorithm).await
    }
    
    /// Start automatic rotation scheduler.
    pub async fn start_rotation_scheduler(&self) -> Result<()> {
        if let Some(scheduler) = &self.rotation_scheduler {
            scheduler.start().await?;
        }
        Ok(())
    }
    
    /// Stop automatic rotation scheduler.
    pub async fn stop_rotation_scheduler(&self) -> Result<()> {
        if let Some(scheduler) = &self.rotation_scheduler {
            scheduler.stop().await?;
        }
        Ok(())
    }
    
    /// Export secrets for backup.
    pub async fn export(&self, query: &SecretQuery) -> Result<Vec<Secret>> {
        // Check export permission
        if !self.policy_engine.can_export().await {
            return Err(SecretsError::AccessDenied(
                "".into(),
                "export permission denied".into()
            ));
        }
        
        self.list(query).await
    }
    
    /// Import secrets from backup.
    pub async fn import(&self, secrets: Vec<Secret>) -> Result<()> {
        // Check import permission
        if !self.policy_engine.can_import().await {
            return Err(SecretsError::AccessDenied(
                "".into(),
                "import permission denied".into()
            ));
        }
        
        for secret in secrets {
            self.put(secret).await?;
        }
        
        Ok(())
    }
    
    /// Get statistics about secrets.
    pub async fn stats(&self) -> Result<SecretStats> {
        let all_secrets = self.backend.list(&SecretQuery {
            id_prefix: None,
            tags: Vec::new(),
            metadata: std::collections::HashMap::new(),
            include_expired: true,
            limit: None,
            offset: None,
        }).await?;
        
        let total = all_secrets.len();
        let expired = all_secrets.iter().filter(|s| s.is_expired()).count();
        let needs_rotation = all_secrets.iter().filter(|s| s.needs_rotation()).count();
        let tagged = all_secrets.iter().filter(|s| !s.tags.is_empty()).count();
        
        Ok(SecretStats {
            total,
            expired,
            needs_rotation,
            tagged,
            by_tag: std::collections::HashMap::new(), // Would need to compute
        })
    }
}

/// Statistics about secrets.
#[derive(Debug, Clone)]
pub struct SecretStats {
    /// Total number of secrets.
    pub total: usize,
    
    /// Number of expired secrets.
    pub expired: usize,
    
    /// Number of secrets needing rotation.
    pub needs_rotation: usize,
    
    /// Number of secrets with tags.
    pub tagged: usize,
    
    /// Count by tag.
    pub by_tag: std::collections::HashMap<String, usize>,
}