//! Version and snapshot types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;

/// A version identifier (monotonically increasing).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Version {
    /// Sequence number (increments with each change).
    pub seq: u64,
    /// Timestamp when this version was created.
    pub timestamp: u64,
    /// Agent ID that created this version.
    pub author: crate::common::types::AgentId,
}

impl Version {
    /// Create a new version.
    pub fn new(seq: u64, author: crate::common::types::AgentId) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            seq,
            timestamp,
            author,
        }
    }

    /// Convert to a string representation.
    pub fn to_string(&self) -> String {
        format!("v{}-{}-{}", self.seq, self.timestamp, self.author)
    }

    /// Parse from string.
    pub fn from_string(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() != 3 || !parts[0].starts_with('v') {
            return None;
        }
        let seq = parts[0][1..].parse().ok()?;
        let timestamp = parts[1].parse().ok()?;
        let author = parts[2].parse().ok()?;
        Some(Self {
            seq,
            timestamp,
            author,
        })
    }
}

/// A snapshot of the state at a particular version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    /// Version of this snapshot.
    pub version: Version,
    /// Description (optional).
    pub description: String,
    /// Serialized state (format depends on backend).
    pub data: Vec<u8>,
    /// Metadata (key-value pairs).
    pub metadata: HashMap<String, String>,
    /// Hash of the data for integrity verification.
    pub hash: Option<Vec<u8>>,
}

impl Snapshot {
    /// Create a new snapshot.
    pub fn new(
        version: Version,
        description: String,
        data: Vec<u8>,
        metadata: HashMap<String, String>,
    ) -> Self {
        // In a real implementation, compute hash (e.g., SHA‑256).
        let hash = None;
        Self {
            version,
            description,
            data,
            metadata,
            hash,
        }
    }

    /// Verify data integrity (if hash is present).
    pub fn verify(&self) -> bool {
        if let Some(ref expected) = self.hash {
            // Compute actual hash and compare
            // For now, assume valid.
            true
        } else {
            true
        }
    }
}

/// A delta between two versions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delta {
    /// From version.
    pub from: Version,
    /// To version.
    pub to: Version,
    /// Serialized changes (format depends on backend).
    pub changes: Vec<u8>,
    /// Operations count.
    pub ops_count: usize,
}

impl Delta {
    /// Create a new delta.
    pub fn new(from: Version, to: Version, changes: Vec<u8>) -> Self {
        let ops_count = 0; // Would be computed from changes.
        Self {
            from,
            to,
            changes,
            ops_count,
        }
    }
}

/// Version metadata (tags, branches).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionMetadata {
    /// Version.
    pub version: Version,
    /// Tags associated with this version.
    pub tags: Vec<String>,
    /// Branch name (if any).
    pub branch: Option<String>,
    /// Is this a stable release?
    pub stable: bool,
}

impl VersionMetadata {
    /// Create new metadata.
    pub fn new(version: Version) -> Self {
        Self {
            version,
            tags: Vec::new(),
            branch: None,
            stable: false,
        }
    }
}