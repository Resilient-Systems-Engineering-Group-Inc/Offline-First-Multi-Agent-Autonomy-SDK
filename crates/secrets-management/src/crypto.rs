//! Cryptographic utilities for secret encryption and key management.

use zeroize::{Zeroize, ZeroizeOnDrop};
use rand::{RngCore, rngs::OsRng};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

use crate::error::{SecretsError, Result};

/// Encryption key with metadata.
#[derive(Debug, Clone, Zeroize, ZeroizeOnDrop)]
pub struct EncryptionKey {
    /// Key ID.
    pub id: String,
    
    /// The actual key bytes.
    #[zeroize(skip)]
    key: Vec<u8>,
    
    /// Key algorithm.
    pub algorithm: KeyAlgorithm,
    
    /// Creation timestamp.
    pub created_at: u64,
    
    /// Expiration timestamp (None = never expires).
    pub expires_at: Option<u64>,
    
    /// Whether this key is active.
    pub active: bool,
    
    /// Tags for categorization.
    pub tags: Vec<String>,
}

impl EncryptionKey {
    /// Generate a new random key.
    pub fn generate(id: impl Into<String>, algorithm: KeyAlgorithm, key_size: usize) -> Result<Self> {
        let mut key = vec![0u8; key_size];
        OsRng.fill_bytes(&mut key);
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Ok(Self {
            id: id.into(),
            key,
            algorithm,
            created_at: now,
            expires_at: None,
            active: true,
            tags: Vec::new(),
        })
    }
    
    /// Get the key bytes (consumes self to prevent copying).
    pub fn into_bytes(self) -> Vec<u8> {
        self.key
    }
    
    /// Get a reference to the key bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.key
    }
    
    /// Check if the key has expired.
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map(|expires| {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                now > expires
            })
            .unwrap_or(false)
    }
}

/// Encryption algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAlgorithm {
    /// AES‑256‑GCM.
    Aes256Gcm,
    
    /// ChaCha20‑Poly1305.
    ChaCha20Poly1305,
    
    /// XChaCha20‑Poly1305.
    XChaCha20Poly1305,
}

impl KeyAlgorithm {
    /// Get the recommended key size in bytes.
    pub fn key_size(&self) -> usize {
        match self {
            KeyAlgorithm::Aes256Gcm => 32,
            KeyAlgorithm::ChaCha20Poly1305 => 32,
            KeyAlgorithm::XChaCha20Poly1305 => 32,
        }
    }
    
    /// Get the nonce size in bytes.
    pub fn nonce_size(&self) -> usize {
        match self {
            KeyAlgorithm::Aes256Gcm => 12,
            KeyAlgorithm::ChaCha20Poly1305 => 12,
            KeyAlgorithm::XChaCha20Poly1305 => 24,
        }
    }
}

/// Strategy for key rotation.
#[derive(Debug, Clone)]
pub enum KeyRotationStrategy {
    /// Rotate keys after a fixed interval (seconds).
    Interval(u64),
    
    /// Rotate after a certain number of uses.
    UsageCount(u64),
    
    /// Rotate when specific conditions are met.
    Conditional(Box<dyn Fn() -> bool + Send + Sync>),
    
    /// Manual rotation only.
    Manual,
}

/// Manager for encryption keys.
#[derive(Debug)]
pub struct KeyManager {
    /// Current active key ID.
    current_key_id: RwLock<String>,
    
    /// All keys by ID.
    keys: RwLock<HashMap<String, EncryptionKey>>,
    
    /// Rotation strategy.
    rotation_strategy: KeyRotationStrategy,
    
    /// Key history (for decryption of old data).
    key_history: RwLock<Vec<String>>,
}

impl KeyManager {
    /// Create a new key manager with a default key.
    pub async fn new() -> Result<Self> {
        let mut keys = HashMap::new();
        let key = EncryptionKey::generate(
            "default",
            KeyAlgorithm::Aes256Gcm,
            KeyAlgorithm::Aes256Gcm.key_size(),
        )?;
        
        let key_id = key.id.clone();
        keys.insert(key_id.clone(), key);
        
        Ok(Self {
            current_key_id: RwLock::new(key_id.clone()),
            keys: RwLock::new(keys),
            rotation_strategy: KeyRotationStrategy::Interval(30 * 24 * 3600), // 30 days
            key_history: RwLock::new(vec![key_id]),
        })
    }
    
    /// Get the current active key.
    pub async fn current_key(&self) -> Result<EncryptionKey> {
        let key_id = self.current_key_id.read().await.clone();
        let keys = self.keys.read().await;
        keys.get(&key_id)
            .cloned()
            .ok_or_else(|| SecretsError::NotFound(format!("key {} not found", key_id)))
    }
    
    /// Rotate to a new key.
    pub async fn rotate(&self, algorithm: KeyAlgorithm) -> Result<String> {
        let new_id = format!("key_{}", SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs());
        
        let new_key = EncryptionKey::generate(
            &new_id,
            algorithm,
            algorithm.key_size(),
        )?;
        
        let mut keys = self.keys.write().await;
        let mut current_key_id = self.current_key_id.write().await;
        let mut key_history = self.key_history.write().await;
        
        // Deactivate old key
        if let Some(old_key) = keys.get_mut(&*current_key_id) {
            old_key.active = false;
        }
        
        // Store new key
        keys.insert(new_id.clone(), new_key);
        *current_key_id = new_id.clone();
        key_history.push(new_id.clone());
        
        Ok(new_id)
    }
    
    /// Get a key by ID.
    pub async fn get_key(&self, key_id: &str) -> Result<EncryptionKey> {
        let keys = self.keys.read().await;
        keys.get(key_id)
            .cloned()
            .ok_or_else(|| SecretsError::NotFound(format!("key {} not found", key_id)))
    }
    
    /// Encrypt data with the current key.
    #[cfg(feature = "encryption")]
    pub async fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        use aes_gcm::{Aes256Gcm, KeyInit, aead::{Aead, Payload}};
        use aes_gcm::aead::generic_array::GenericArray;
        
        let key = self.current_key().await?;
        if key.algorithm != KeyAlgorithm::Aes256Gcm {
            return Err(SecretsError::Crypto(
                format!("unsupported algorithm {:?}", key.algorithm)
            ));
        }
        
        let cipher = Aes256Gcm::new(GenericArray::from_slice(key.as_bytes()));
        let nonce = self.generate_nonce(key.algorithm.nonce_size());
        
        cipher.encrypt(
            GenericArray::from_slice(&nonce),
            Payload { msg: plaintext, aad: &[] },
        )
        .map_err(|e| SecretsError::Crypto(format!("encryption failed: {:?}", e)))
    }
    
    /// Decrypt data with a key.
    #[cfg(feature = "encryption")]
    pub async fn decrypt(&self, ciphertext: &[u8], key_id: &str) -> Result<Vec<u8>> {
        use aes_gcm::{Aes256Gcm, KeyInit, aead::{Aead, Payload}};
        use aes_gcm::aead::generic_array::GenericArray;
        
        let key = self.get_key(key_id).await?;
        if key.algorithm != KeyAlgorithm::Aes256Gcm {
            return Err(SecretsError::Crypto(
                format!("unsupported algorithm {:?}", key.algorithm)
            ));
        }
        
        // Extract nonce (first nonce_size bytes)
        let nonce_size = key.algorithm.nonce_size();
        if ciphertext.len() < nonce_size {
            return Err(SecretsError::Crypto("ciphertext too short".into()));
        }
        
        let nonce = &ciphertext[..nonce_size];
        let actual_ciphertext = &ciphertext[nonce_size..];
        
        let cipher = Aes256Gcm::new(GenericArray::from_slice(key.as_bytes()));
        
        cipher.decrypt(
            GenericArray::from_slice(nonce),
            Payload { msg: actual_ciphertext, aad: &[] },
        )
        .map_err(|e| SecretsError::Crypto(format!("decryption failed: {:?}", e)))
    }
    
    /// Generate a random nonce.
    fn generate_nonce(&self, size: usize) -> Vec<u8> {
        let mut nonce = vec![0u8; size];
        OsRng.fill_bytes(&mut nonce);
        nonce
    }
    
    /// Check if rotation is needed based on strategy.
    pub async fn rotation_needed(&self) -> bool {
        match &self.rotation_strategy {
            KeyRotationStrategy::Interval(interval) => {
                let key = match self.current_key().await {
                    Ok(k) => k,
                    Err(_) => return false,
                };
                
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                
                now - key.created_at > *interval
            }
            KeyRotationStrategy::UsageCount(_) => {
                // Would need to track usage count
                false
            }
            KeyRotationStrategy::Conditional(cond) => cond(),
            KeyRotationStrategy::Manual => false,
        }
    }
}

/// Simple in‑memory key store for testing.
#[derive(Debug, Default)]
pub struct MemoryKeyStore {
    keys: RwLock<HashMap<String, Vec<u8>>>,
}

impl MemoryKeyStore {
    /// Create a new empty key store.
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Store a key.
    pub async fn put(&self, key_id: String, key: Vec<u8>) {
        let mut keys = self.keys.write().await;
        keys.insert(key_id, key);
    }
    
    /// Retrieve a key.
    pub async fn get(&self, key_id: &str) -> Option<Vec<u8>> {
        let keys = self.keys.read().await;
        keys.get(key_id).cloned()
    }
    
    /// Delete a key.
    pub async fn delete(&self, key_id: &str) {
        let mut keys = self.keys.write().await;
        keys.remove(key_id);
    }
}