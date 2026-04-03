//! Data models for secrets.

use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// A secret value with metadata.
#[derive(Debug, Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct Secret {
    /// Unique identifier for the secret.
    pub id: String,
    
    /// The encrypted secret value (base64).
    #[zeroize(skip)]
    pub encrypted_value: String,
    
    /// Metadata about the secret.
    pub metadata: SecretMetadata,
    
    /// Tags for categorization.
    pub tags: Vec<String>,
    
    /// Access policies.
    pub policies: Vec<AccessPolicy>,
}

impl Secret {
    /// Create a new secret with the given value.
    pub fn new(id: impl Into<String>, value: impl Into<String>, tags: Vec<String>) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Self {
            id: id.into(),
            encrypted_value: value.into(), // In real usage, this would be encrypted
            metadata: SecretMetadata {
                created_at: now,
                updated_at: now,
                version: 1,
                description: None,
                rotation_interval: None,
                last_rotated: None,
                expires_at: None,
            },
            tags,
            policies: Vec::new(),
        }
    }
    
    /// Get the secret value (decrypted).
    pub fn value(&self) -> &str {
        &self.encrypted_value // In real usage, this would decrypt
    }
    
    /// Check if the secret has expired.
    pub fn is_expired(&self) -> bool {
        self.metadata.expires_at
            .map(|expires| {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                now > expires
            })
            .unwrap_or(false)
    }
    
    /// Check if the secret needs rotation.
    pub fn needs_rotation(&self) -> bool {
        self.metadata.rotation_interval
            .and_then(|interval| {
                self.metadata.last_rotated.map(|last| {
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs();
                    now - last > interval
                })
            })
            .unwrap_or(false)
    }
}

/// Metadata about a secret.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretMetadata {
    /// Creation timestamp (seconds since Unix epoch).
    pub created_at: u64,
    
    /// Last update timestamp.
    pub updated_at: u64,
    
    /// Version number (incremented on each update).
    pub version: u32,
    
    /// Optional description.
    pub description: Option<String>,
    
    /// Rotation interval in seconds (None = no automatic rotation).
    pub rotation_interval: Option<u64>,
    
    /// When the secret was last rotated (seconds since Unix epoch).
    pub last_rotated: Option<u64>,
    
    /// Expiration timestamp (seconds since Unix epoch).
    pub expires_at: Option<u64>,
}

/// A version of a secret (for rotation history).
#[derive(Debug, Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct SecretVersion {
    /// Version number.
    pub version: u32,
    
    /// The encrypted secret value.
    #[zeroize(skip)]
    pub encrypted_value: String,
    
    /// When this version was created.
    pub created_at: u64,
    
    /// When this version was active (start).
    pub active_from: u64,
    
    /// When this version was retired (if any).
    pub active_to: Option<u64>,
}

/// Access policy for a secret.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessPolicy {
    /// Unique policy ID.
    pub id: String,
    
    /// Agent IDs allowed to access.
    pub allowed_agents: Vec<u64>,
    
    /// Required capabilities.
    pub required_capabilities: Vec<String>,
    
    /// Time window when access is allowed (start hour, end hour in UTC).
    pub time_window: Option<(u8, u8)>,
    
    /// Maximum number of accesses.
    pub max_accesses: Option<u32>,
    
    /// Current access count.
    pub access_count: u32,
}

impl AccessPolicy {
    /// Check if an agent can access the secret.
    pub fn can_access(&self, agent_id: u64, capabilities: &[String]) -> bool {
        // Check agent ID
        if !self.allowed_agents.contains(&agent_id) {
            return false;
        }
        
        // Check capabilities
        for required in &self.required_capabilities {
            if !capabilities.contains(required) {
                return false;
            }
        }
        
        // Check time window
        if let Some((start_hour, end_hour)) = self.time_window {
            use chrono::Timelike;
            let now = chrono::Utc::now();
            let hour = now.hour() as u8;
            if hour < start_hour || hour >= end_hour {
                return false;
            }
        }
        
        // Check max accesses
        if let Some(max) = self.max_accesses {
            if self.access_count >= max {
                return false;
            }
        }
        
        true
    }
}

/// Batch of secrets (for transport).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretBatch {
    /// Secrets in the batch.
    pub secrets: Vec<Secret>,
    
    /// Batch ID.
    pub batch_id: String,
    
    /// Source agent ID.
    pub source_agent: u64,
    
    /// Timestamp.
    pub timestamp: u64,
    
    /// Signature for verification.
    pub signature: Option<Vec<u8>>,
}

/// Secret query for filtering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretQuery {
    /// Match secret IDs (prefix).
    pub id_prefix: Option<String>,
    
    /// Match tags.
    pub tags: Vec<String>,
    
    /// Match metadata fields.
    pub metadata: HashMap<String, String>,
    
    /// Include expired secrets.
    pub include_expired: bool,
    
    /// Limit results.
    pub limit: Option<usize>,
    
    /// Offset for pagination.
    pub offset: Option<usize>,
}