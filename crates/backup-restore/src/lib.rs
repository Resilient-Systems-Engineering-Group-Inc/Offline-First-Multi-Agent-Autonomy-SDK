//! Backup and restore mechanism for distributed agent state.
//!
//! This crate provides functionality for creating backups of distributed
//! state, verifying integrity, and restoring from backups.
//!
//! Features:
//! - Zlib and Gzip compression
//! - SHA-256 checksum verification
//! - Configurable compression levels
//! - Backup metadata tracking
//! - Integrity verification

pub mod error;

// Re-export commonly used types
pub use error::{BackupError, Result};

use sha2::{Sha256, Digest};
use std::io::{Read, Write};

/// Current version of the backup-restore crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize the backup-restore system.
pub fn init() {
    tracing::info!("Backup-Restore v{} initialized", VERSION);
}

/// Backup configuration.
#[derive(Debug, Clone)]
pub struct BackupConfig {
    /// Compression algorithm to use.
    pub compression: CompressionAlgo,
    /// Compression level (1-9, only used for Zlib/Gzip).
    pub compression_level: u32,
    /// Whether to include checksums.
    pub include_checksums: bool,
    /// Maximum backup size in bytes.
    pub max_size_bytes: Option<u64>,
    /// Whether to split large backups.
    pub split_large_backups: bool,
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            compression: CompressionAlgo::Zlib,
            compression_level: 6,
            include_checksums: true,
            max_size_bytes: Some(1024 * 1024 * 1024), // 1 GB
            split_large_backups: true,
        }
    }
}

/// Compression algorithms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionAlgo {
    /// No compression.
    None,
    /// Zlib compression.
    Zlib,
    /// Gzip compression.
    Gzip,
}

/// Backup metadata.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BackupMetadata {
    /// Backup ID.
    pub id: String,
    /// Creation timestamp.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Original size in bytes (before compression).
    pub original_size_bytes: u64,
    /// Compressed size in bytes.
    pub compressed_size_bytes: u64,
    /// Checksum (SHA-256) of the original data.
    pub checksum: String,
    /// Number of items in backup.
    pub item_count: usize,
    /// Compression algorithm used.
    pub compression: String,
    /// Version of the system that created the backup.
    pub system_version: String,
}

/// Backup manager.
pub struct BackupManager {
    config: BackupConfig,
}

impl BackupManager {
    /// Create a new backup manager.
    pub fn new(config: BackupConfig) -> Self {
        Self { config }
    }

    /// Create a backup of the given data.
    ///
    /// This method:
    /// 1. Computes a SHA-256 checksum of the original data
    /// 2. Compresses the data using the configured algorithm
    /// 3. Packages the result with metadata
    pub async fn create_backup(&self, data: &[u8]) -> Result<(BackupMetadata, Vec<u8>)> {
        // Compute SHA-256 checksum of original data
        let checksum = if self.config.include_checksums {
            let mut hasher = Sha256::new();
            hasher.update(data);
            format!("{:x}", hasher.finalize())
        } else {
            String::new()
        };

        // Compress the data
        let compressed = self.compress(data)?;

        let metadata = BackupMetadata {
            id: uuid::Uuid::new_v4().to_string(),
            created_at: chrono::Utc::now(),
            original_size_bytes: data.len() as u64,
            compressed_size_bytes: compressed.len() as u64,
            checksum,
            item_count: 1,
            compression: self.config.compression.to_string(),
            system_version: VERSION.to_string(),
        };

        Ok((metadata, compressed))
    }

    /// Restore from a backup.
    ///
    /// This method:
    /// 1. Decompresses the data
    /// 2. Verifies the SHA-256 checksum against the original
    pub async fn restore_backup(&self, backup: &[u8], metadata: &BackupMetadata) -> Result<Vec<u8>> {
        // Decompress the data
        let decompressed = self.decompress(backup, &metadata.compression)?;

        // Verify checksum if present
        if !metadata.checksum.is_empty() {
            let mut hasher = Sha256::new();
            hasher.update(&decompressed);
            let computed = format!("{:x}", hasher.finalize());

            if computed != metadata.checksum {
                return Err(BackupError::ChecksumMismatch(
                    metadata.checksum.clone(),
                    computed,
                ));
            }
        }

        Ok(decompressed)
    }

    /// Verify backup integrity.
    ///
    /// This method:
    /// 1. Attempts to decompress the data
    /// 2. Verifies the SHA-256 checksum
    /// 3. Validates the metadata
    pub async fn verify_backup(&self, backup: &[u8], metadata: &BackupMetadata) -> Result<bool> {
        // Validate metadata fields
        if metadata.id.is_empty() {
            return Err(BackupError::InvalidFormat("Backup ID is empty".to_string()));
        }

        if metadata.compressed_size_bytes != backup.len() as u64 {
            return Err(BackupError::VerificationFailed(format!(
                "Compressed size mismatch: metadata says {}, actual is {}",
                metadata.compressed_size_bytes,
                backup.len()
            )));
        }

        // Try to decompress
        let decompressed = self.decompress(backup, &metadata.compression)?;

        // Verify original size
        if metadata.original_size_bytes != decompressed.len() as u64 {
            return Err(BackupError::VerificationFailed(format!(
                "Original size mismatch: metadata says {}, actual is {}",
                metadata.original_size_bytes,
                decompressed.len()
            )));
        }

        // Verify checksum
        if !metadata.checksum.is_empty() {
            let mut hasher = Sha256::new();
            hasher.update(&decompressed);
            let computed = format!("{:x}", hasher.finalize());

            if computed != metadata.checksum {
                return Err(BackupError::ChecksumMismatch(
                    metadata.checksum.clone(),
                    computed,
                ));
            }
        }

        Ok(true)
    }

    /// Compress data using the configured algorithm.
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        match self.config.compression {
            CompressionAlgo::None => Ok(data.to_vec()),
            CompressionAlgo::Zlib => {
                let mut encoder = flate2::write::ZlibEncoder::new(
                    Vec::new(),
                    flate2::Compression::new(self.config.compression_level),
                );
                encoder
                    .write_all(data)
                    .map_err(|e| BackupError::Compression(format!("Zlib compression failed: {}", e)))?;
                encoder
                    .finish()
                    .map_err(|e| BackupError::Compression(format!("Zlib compression finalize failed: {}", e)))
            }
            CompressionAlgo::Gzip => {
                let mut encoder = flate2::write::GzEncoder::new(
                    Vec::new(),
                    flate2::Compression::new(self.config.compression_level),
                );
                encoder
                    .write_all(data)
                    .map_err(|e| BackupError::Compression(format!("Gzip compression failed: {}", e)))?;
                encoder
                    .finish()
                    .map_err(|e| BackupError::Compression(format!("Gzip compression finalize failed: {}", e)))
            }
        }
    }

    /// Decompress data.
    fn decompress(&self, data: &[u8], compression: &str) -> Result<Vec<u8>> {
        match compression {
            "none" => Ok(data.to_vec()),
            "zlib" => {
                let mut decoder = flate2::read::ZlibDecoder::new(data);
                let mut decompressed = Vec::new();
                decoder
                    .read_to_end(&mut decompressed)
                    .map_err(|e| BackupError::Compression(format!("Zlib decompression failed: {}", e)))?;
                Ok(decompressed)
            }
            "gzip" => {
                let mut decoder = flate2::read::GzDecoder::new(data);
                let mut decompressed = Vec::new();
                decoder
                    .read_to_end(&mut decompressed)
                    .map_err(|e| BackupError::Compression(format!("Gzip decompression failed: {}", e)))?;
                Ok(decompressed)
            }
            other => Err(BackupError::InvalidFormat(format!(
                "Unknown compression algorithm: {}",
                other
            ))),
        }
    }
}

impl CompressionAlgo {
    fn to_string(&self) -> String {
        match self {
            CompressionAlgo::None => "none".to_string(),
            CompressionAlgo::Zlib => "zlib".to_string(),
            CompressionAlgo::Gzip => "gzip".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_backup_creation_no_compression() {
        let config = BackupConfig {
            compression: CompressionAlgo::None,
            include_checksums: true,
            ..Default::default()
        };
        let manager = BackupManager::new(config);

        let data = b"test backup data";
        let (metadata, backup) = manager.create_backup(data).await.unwrap();

        assert!(!metadata.id.is_empty());
        assert_eq!(metadata.original_size_bytes, data.len() as u64);
        assert_eq!(metadata.compressed_size_bytes, data.len() as u64);
        assert!(!metadata.checksum.is_empty());
        assert_eq!(backup, data);
    }

    #[tokio::test]
    async fn test_backup_creation_zlib() {
        let config = BackupConfig {
            compression: CompressionAlgo::Zlib,
            include_checksums: true,
            ..Default::default()
        };
        let manager = BackupManager::new(config);

        let data = b"test backup data with some repetition to make compression worthwhile";
        let (metadata, backup) = manager.create_backup(data).await.unwrap();

        assert!(!metadata.id.is_empty());
        assert_eq!(metadata.original_size_bytes, data.len() as u64);
        assert!(metadata.compressed_size_bytes <= data.len() as u64);
        assert!(!metadata.checksum.is_empty());
        // Compressed data should be different from original
        assert_ne!(backup, data);
    }

    #[tokio::test]
    async fn test_backup_restore_no_compression() {
        let config = BackupConfig {
            compression: CompressionAlgo::None,
            include_checksums: true,
            ..Default::default()
        };
        let manager = BackupManager::new(config);

        let data = b"test data";
        let (metadata, backup) = manager.create_backup(data).await.unwrap();

        let restored = manager.restore_backup(&backup, &metadata).await.unwrap();
        assert_eq!(restored, data);
    }

    #[tokio::test]
    async fn test_backup_restore_zlib() {
        let config = BackupConfig {
            compression: CompressionAlgo::Zlib,
            include_checksums: true,
            ..Default::default()
        };
        let manager = BackupManager::new(config);

        let data = b"test data for zlib compression roundtrip";
        let (metadata, backup) = manager.create_backup(data).await.unwrap();

        let restored = manager.restore_backup(&backup, &metadata).await.unwrap();
        assert_eq!(restored, data);
    }

    #[tokio::test]
    async fn test_backup_restore_gzip() {
        let config = BackupConfig {
            compression: CompressionAlgo::Gzip,
            include_checksums: true,
            ..Default::default()
        };
        let manager = BackupManager::new(config);

        let data = b"test data for gzip compression roundtrip";
        let (metadata, backup) = manager.create_backup(data).await.unwrap();

        let restored = manager.restore_backup(&backup, &metadata).await.unwrap();
        assert_eq!(restored, data);
    }

    #[tokio::test]
    async fn test_backup_verify_integrity() {
        let config = BackupConfig {
            compression: CompressionAlgo::Zlib,
            include_checksums: true,
            ..Default::default()
        };
        let manager = BackupManager::new(config);

        let data = b"test data for integrity verification";
        let (metadata, backup) = manager.create_backup(data).await.unwrap();

        let result = manager.verify_backup(&backup, &metadata).await.unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn test_backup_verify_tampered_data() {
        let config = BackupConfig {
            compression: CompressionAlgo::None,
            include_checksums: true,
            ..Default::default()
        };
        let manager = BackupManager::new(config);

        let data = b"test data";
        let (metadata, backup) = manager.create_backup(data).await.unwrap();

        // Tamper with the backup data
        let mut tampered = backup.clone();
        if !tampered.is_empty() {
            tampered[0] ^= 0xFF; // Flip all bits in first byte
        }

        let result = manager.verify_backup(&tampered, &metadata).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_backup_verify_tampered_metadata() {
        let config = BackupConfig {
            compression: CompressionAlgo::None,
            include_checksums: true,
            ..Default::default()
        };
        let manager = BackupManager::new(config);

        let data = b"test data";
        let (mut metadata, backup) = manager.create_backup(data).await.unwrap();

        // Tamper with the metadata checksum
        metadata.checksum = "tampered".to_string();

        let result = manager.verify_backup(&backup, &metadata).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_backup_without_checksum() {
        let config = BackupConfig {
            compression: CompressionAlgo::None,
            include_checksums: false,
            ..Default::default()
        };
        let manager = BackupManager::new(config);

        let data = b"test data";
        let (metadata, backup) = manager.create_backup(data).await.unwrap();

        assert!(metadata.checksum.is_empty());

        let restored = manager.restore_backup(&backup, &metadata).await.unwrap();
        assert_eq!(restored, data);
    }

    #[tokio::test]
    async fn test_large_data_compression() {
        let config = BackupConfig {
            compression: CompressionAlgo::Zlib,
            include_checksums: true,
            ..Default::default()
        };
        let manager = BackupManager::new(config);

        // Create a larger dataset with repetition (good for compression)
        let data = vec![b'A'; 10_000];
        let (metadata, backup) = manager.create_backup(&data).await.unwrap();

        assert_eq!(metadata.original_size_bytes, 10_000);
        // Compressed size should be significantly smaller for repeated data
        assert!(metadata.compressed_size_bytes < 10_000);
        assert!(metadata.compressed_size_bytes > 0);

        let restored = manager.restore_backup(&backup, &metadata).await.unwrap();
        assert_eq!(restored, data);
    }
}