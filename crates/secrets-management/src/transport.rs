//! Transport layer for distributed secret sharing.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{RwLock, mpsc};

use crate::error::{SecretsError, Result};
use crate::model::{Secret, SecretBatch, SecretQuery};
use crate::crypto::{KeyManager, EncryptionKey};

/// Message types for secret transport.
#[derive(Debug, Clone)]
pub enum SecretMessage {
    /// Announce a new or updated secret.
    Announce {
        secret_id: String,
        version: u32,
        timestamp: u64,
        source_agent: u64,
    },
    
    /// Request a secret.
    Request {
        secret_id: String,
        requestor: u64,
        nonce: u64,
    },
    
    /// Response with a secret.
    Response {
        secret_id: String,
        secret: Secret,
        nonce: u64,
        signature: Option<Vec<u8>>,
    },
    
    /// Delete announcement.
    Delete {
        secret_id: String,
        source_agent: u64,
        timestamp: u64,
    },
    
    /// Batch of secrets.
    Batch(SecretBatch),
    
    /// Key exchange for encrypted transport.
    KeyExchange {
        key_id: String,
        encrypted_key: Vec<u8>,
        algorithm: String,
    },
}

/// Trait for secret transport.
#[async_trait]
pub trait SecretTransport: Send + Sync {
    /// Announce a secret update to peers.
    async fn secret_updated(&self, secret_id: &str) -> Result<()>;
    
    /// Announce a secret deletion to peers.
    async fn secret_deleted(&self, secret_id: &str) -> Result<()>;
    
    /// Announce a secret rotation to peers.
    async fn secret_rotated(&self, secret_id: &str) -> Result<()>;
    
    /// Request a secret from peers.
    async fn request_secret(&self, secret_id: &str) -> Result<Option<Secret>>;
    
    /// Send a secret to a specific agent.
    async fn send_secret(&self, secret: &Secret, recipient: u64) -> Result<()>;
    
    /// Broadcast a batch of secrets.
    async fn broadcast_batch(&self, batch: SecretBatch) -> Result<()>;
    
    /// Get connected peers.
    async fn peers(&self) -> Vec<u64>;
    
    /// Start the transport.
    async fn start(&self) -> Result<()>;
    
    /// Stop the transport.
    async fn stop(&self) -> Result<()>;
}

/// In‑memory transport for testing.
#[derive(Debug, Default)]
pub struct InMemoryTransport {
    messages: RwLock<Vec<SecretMessage>>,
    secrets: RwLock<HashMap<String, Secret>>,
    peers: RwLock<Vec<u64>>,
    running: RwLock<bool>,
}

impl InMemoryTransport {
    /// Create a new in‑memory transport.
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Add a peer.
    pub async fn add_peer(&self, peer_id: u64) {
        let mut peers = self.peers.write().await;
        if !peers.contains(&peer_id) {
            peers.push(peer_id);
        }
    }
    
    /// Remove a peer.
    pub async fn remove_peer(&self, peer_id: u64) {
        let mut peers = self.peers.write().await;
        peers.retain(|&id| id != peer_id);
    }
    
    /// Get all messages.
    pub async fn messages(&self) -> Vec<SecretMessage> {
        self.messages.read().await.clone()
    }
    
    /// Clear messages.
    pub async fn clear_messages(&self) {
        self.messages.write().await.clear();
    }
}

#[async_trait]
impl SecretTransport for InMemoryTransport {
    async fn secret_updated(&self, secret_id: &str) -> Result<()> {
        let mut messages = self.messages.write().await;
        messages.push(SecretMessage::Announce {
            secret_id: secret_id.to_string(),
            version: 1,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            source_agent: 0, // Default agent
        });
        
        Ok(())
    }
    
    async fn secret_deleted(&self, secret_id: &str) -> Result<()> {
        let mut messages = self.messages.write().await;
        messages.push(SecretMessage::Delete {
            secret_id: secret_id.to_string(),
            source_agent: 0,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        });
        
        let mut secrets = self.secrets.write().await;
        secrets.remove(secret_id);
        
        Ok(())
    }
    
    async fn secret_rotated(&self, secret_id: &str) -> Result<()> {
        // Same as update for now
        self.secret_updated(secret_id).await
    }
    
    async fn request_secret(&self, secret_id: &str) -> Result<Option<Secret>> {
        let secrets = self.secrets.read().await;
        Ok(secrets.get(secret_id).cloned())
    }
    
    async fn send_secret(&self, secret: &Secret, _recipient: u64) -> Result<()> {
        let mut secrets = self.secrets.write().await;
        secrets.insert(secret.id.clone(), secret.clone());
        
        let mut messages = self.messages.write().await;
        messages.push(SecretMessage::Response {
            secret_id: secret.id.clone(),
            secret: secret.clone(),
            nonce: 0,
            signature: None,
        });
        
        Ok(())
    }
    
    async fn broadcast_batch(&self, batch: SecretBatch) -> Result<()> {
        let mut messages = self.messages.write().await;
        messages.push(SecretMessage::Batch(batch));
        Ok(())
    }
    
    async fn peers(&self) -> Vec<u64> {
        self.peers.read().await.clone()
    }
    
    async fn start(&self) -> Result<()> {
        let mut running = self.running.write().await;
        *running = true;
        Ok(())
    }
    
    async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().await;
        *running = false;
        Ok(())
    }
}

/// Mesh‑based transport using the mesh‑transport crate.
#[cfg(feature = "mesh-transport")]
pub struct MeshSecretTransport {
    mesh: Arc<dyn mesh_transport::Transport>,
    key_manager: Arc<KeyManager>,
    local_agent: u64,
    secrets_cache: RwLock<HashMap<String, Secret>>,
    message_rx: mpsc::UnboundedReceiver<SecretMessage>,
    message_tx: mpsc::UnboundedSender<SecretMessage>,
}

#[cfg(feature = "mesh-transport")]
impl MeshSecretTransport {
    /// Create a new mesh‑based secret transport.
    pub async fn new(
        mesh: Arc<dyn mesh_transport::Transport>,
        key_manager: Arc<KeyManager>,
        local_agent: u64,
    ) -> Result<Self> {
        let (message_tx, message_rx) = mpsc::unbounded_channel();
        
        Ok(Self {
            mesh,
            key_manager,
            local_agent,
            secrets_cache: RwLock::new(HashMap::new()),
            message_rx,
            message_tx,
        })
    }
    
    /// Handle incoming mesh messages.
    async fn handle_mesh_message(&self, payload: Vec<u8>) -> Result<()> {
        // Decrypt if needed
        let decrypted = if payload.len() > 0 && payload[0] == 0x01 {
            // Encrypted payload
            self.key_manager.decrypt(&payload[1..], "default").await?
        } else {
            payload
        };
        
        // Deserialize message
        let message: SecretMessage = serde_json::from_slice(&decrypted)
            .map_err(|e| SecretsError::Serialization(e))?;
        
        // Process message
        match message {
            SecretMessage::Announce { secret_id, .. } => {
                log::debug!("Received announcement for secret {}", secret_id);
                // Could request the secret if we need it
            }
            SecretMessage::Response { secret_id, secret, .. } => {
                log::debug!("Received secret {}", secret_id);
                let mut cache = self.secrets_cache.write().await;
                cache.insert(secret_id, secret);
            }
            SecretMessage::Batch(batch) => {
                log::debug!("Received batch with {} secrets", batch.secrets.len());
                let mut cache = self.secrets_cache.write().await;
                for secret in batch.secrets {
                    cache.insert(secret.id.clone(), secret);
                }
            }
            _ => {}
        }
        
        // Forward to local listeners
        let _ = self.message_tx.send(message);
        
        Ok(())
    }
    
    /// Serialize and encrypt a message.
    async fn prepare_message(&self, message: &SecretMessage) -> Result<Vec<u8>> {
        let json = serde_json::to_vec(message)
            .map_err(|e| SecretsError::Serialization(e))?;
        
        // Encrypt with current key
        let encrypted = self.key_manager.encrypt(&json).await?;
        
        // Prepend version byte
        let mut result = vec![0x01]; // Version 1 = encrypted
        result.extend(encrypted);
        
        Ok(result)
    }
}

#[cfg(feature = "mesh-transport")]
#[async_trait]
impl SecretTransport for MeshSecretTransport {
    async fn secret_updated(&self, secret_id: &str) -> Result<()> {
        let message = SecretMessage::Announce {
            secret_id: secret_id.to_string(),
            version: 1,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            source_agent: self.local_agent,
        };
        
        let payload = self.prepare_message(&message).await?;
        self.mesh.broadcast(payload).await
            .map_err(|e| SecretsError::Transport(format!("{:?}", e)))?;
        
        Ok(())
    }
    
    async fn secret_deleted(&self, secret_id: &str) -> Result<()> {
        let message = SecretMessage::Delete {
            secret_id: secret_id.to_string(),
            source_agent: self.local_agent,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        
        let payload = self.prepare_message(&message).await?;
        self.mesh.broadcast(payload).await
            .map_err(|e| SecretsError::Transport(format!("{:?}", e)))?;
        
        // Remove from local cache
        let mut cache = self.secrets_cache.write().await;
        cache.remove(secret_id);
        
        Ok(())
    }
    
    async fn secret_rotated(&self, secret_id: &str) -> Result<()> {
        self.secret_updated(secret_id).await
    }
    
    async fn request_secret(&self, secret_id: &str) -> Result<Option<Secret>> {
        // Check cache first
        {
            let cache = self.secrets_cache.read().await;
            if let Some(secret) = cache.get(secret_id) {
                return Ok(Some(secret.clone()));
            }
        }
        
        // Send request
        let nonce = rand::random();
        let message = SecretMessage::Request {
            secret_id: secret_id.to_string(),
            requestor: self.local_agent,
            nonce,
        };
        
        let payload = self.prepare_message(&message).await?;
        self.mesh.broadcast(payload).await
            .map_err(|e| SecretsError::Transport(format!("{:?}", e)))?;
        
        // Wait for response (simplified - would need proper async waiting)
        Ok(None)
    }
    
    async fn send_secret(&self, secret: &Secret, recipient: u64) -> Result<()> {
        let message = SecretMessage::Response {
            secret_id: secret.id.clone(),
            secret: secret.clone(),
            nonce: 0,
            signature: None,
        };
        
        let payload = self.prepare_message(&message).await?;
        self.mesh.send_to(recipient, payload).await
            .map_err(|e| SecretsError::Transport(format!("{:?}", e)))?;
        
        Ok(())
    }
    
    async fn broadcast_batch(&self, batch: SecretBatch) -> Result<()> {
        let message = SecretMessage::Batch(batch);
        let payload = self.prepare_message(&message).await?;
        self.mesh.broadcast(payload).await
            .map_err(|e| SecretsError::Transport(format!("{:?}", e)))?;
        
        Ok(())
    }
    
    async fn peers(&self) -> Vec<u64> {
        // Would need to get peers from mesh transport
        Vec::new()
    }
    
    async fn start(&self) -> Result<()> {
        // Start listening for mesh messages
        Ok(())
    }
    
    async fn stop(&self) -> Result<()> {
        Ok(())
    }
}

/// No‑op transport for when distribution is not needed.
#[derive(Debug)]
pub struct NoopTransport;

#[async_trait]
impl SecretTransport for NoopTransport {
    async fn secret_updated(&self, _secret_id: &str) -> Result<()> {
        Ok(())
    }
    
    async fn secret_deleted(&self, _secret_id: &str) -> Result<()> {
        Ok(())
    }
    
    async fn secret_rotated(&self, _secret_id: &str) -> Result<()> {
        Ok(())
    }
    
    async fn request_secret(&self, _secret_id: &str) -> Result<Option<Secret>> {
        Ok(None)
    }
    
    async fn send_secret(&self, _secret: &Secret, _recipient: u64) -> Result<()> {
        Ok(())
    }
    
    async fn broadcast_batch(&self, _batch: SecretBatch) -> Result<()> {
        Ok(())
    }
    
    async fn peers(&self) -> Vec<u64> {
        Vec::new()
    }
    
    async fn start(&self) -> Result<()> {
        Ok(())
    }
    
    async fn stop(&self) -> Result<()> {
        Ok(())
    }
}