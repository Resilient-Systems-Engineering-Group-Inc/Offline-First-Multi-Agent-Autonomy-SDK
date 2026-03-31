//! Package definitions for OTA updates.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Package identifier (e.g., "agent-core", "mesh-transport").
pub type PackageId = String;

/// Semantic version.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    /// Parse from string.
    pub fn parse(s: &str) -> Result<Self, crate::error::Error> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(crate::error::Error::Validation(
                "Version must be in format major.minor.patch".into(),
            ));
        }
        Ok(Self {
            major: parts[0].parse().map_err(|e| {
                crate::error::Error::Validation(format!("Invalid major version: {}", e))
            })?,
            minor: parts[1].parse().map_err(|e| {
                crate::error::Error::Validation(format!("Invalid minor version: {}", e))
            })?,
            patch: parts[2].parse().map_err(|e| {
                crate::error::Error::Validation(format!("Invalid patch version: {}", e))
            })?,
        })
    }

    /// Convert to string.
    pub fn to_string(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// OTA package metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    /// Package identifier.
    pub id: PackageId,
    /// Version.
    pub version: Version,
    /// Description.
    pub description: String,
    /// Dependencies (package IDs with version constraints).
    pub dependencies: Vec<(PackageId, String)>,
    /// Size in bytes.
    pub size: u64,
    /// SHA‑256 hash of the payload.
    pub sha256: String,
    /// Signature (Ed25519).
    pub signature: Vec<u8>,
    /// Delta‑base version (if this is a delta update).
    pub delta_base: Option<Version>,
}

impl Package {
    /// Validate the package (check hash, signature, etc.).
    pub fn validate(&self) -> Result<(), crate::error::Error> {
        // TODO: implement actual validation
        Ok(())
    }

    /// Apply this package to the target directory.
    pub async fn apply(&self, target_dir: PathBuf) -> Result<(), crate::error::Error> {
        // TODO: implement extraction and application logic
        tracing::info!(
            "Applying package {} {} to {:?}",
            self.id,
            self.version.to_string(),
            target_dir
        );
        Ok(())
    }
}