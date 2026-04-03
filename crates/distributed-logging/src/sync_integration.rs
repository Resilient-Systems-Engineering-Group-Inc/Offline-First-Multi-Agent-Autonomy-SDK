//! Integration with state synchronization for log replication.

use crate::error::{Result, LogError};
use crate::log_record::LogRecord;
use async_trait::async_trait;
use std::sync::Arc;

/// Trait for synchronizing logs across agents.
#[async_trait]
pub trait LogSync: Send + Sync {
    /// Synchronizes local logs with a remote peer.
    async fn sync_with_peer(&self, peer_id: u64) -> Result<()>;

    /// Returns logs that are missing on the given peer.
    async fn missing_logs(&self, peer_id: u64) -> Result<Vec<LogRecord>>;

    /// Applies incoming logs from a peer.
    async fn apply_logs(&self, logs: Vec<LogRecord>) -> Result<()>;
}

/// Dummy sync implementation that does nothing.
pub struct NullLogSync;

#[async_trait]
impl LogSync for NullLogSync {
    async fn sync_with_peer(&self, _peer_id: u64) -> Result<()> {
        Ok(())
    }

    async fn missing_logs(&self, _peer_id: u64) -> Result<Vec<LogRecord>> {
        Ok(Vec::new())
    }

    async fn apply_logs(&self, _logs: Vec<LogRecord>) -> Result<()> {
        Ok(())
    }
}

/// Configuration for log synchronization.
pub struct LogSyncConfig {
    /// Whether to enable automatic synchronization.
    pub enabled: bool,
    /// Sync interval in seconds.
    pub interval_secs: u64,
    /// Maximum number of logs to sync per round.
    pub max_logs_per_sync: usize,
}

impl Default for LogSyncConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_secs: 30,
            max_logs_per_sync: 1000,
        }
    }
}

/// Manager that coordinates log synchronization.
pub struct LogSyncManager {
    config: LogSyncConfig,
    sync: Arc<dyn LogSync>,
}

impl LogSyncManager {
    /// Creates a new sync manager.
    pub fn new(config: LogSyncConfig, sync: Arc<dyn LogSync>) -> Self {
        Self { config, sync }
    }

    /// Starts the background synchronization task.
    pub fn start(&self) -> tokio::task::JoinHandle<()> {
        let sync = self.sync.clone();
        let interval = std::time::Duration::from_secs(self.config.interval_secs);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(interval).await;
                // In a real implementation you would iterate over known peers.
                // For now we just log.
                tracing::debug!("Log sync round (stub)");
            }
        })
    }

    /// Triggers an immediate sync with a specific peer.
    pub async fn sync_with_peer(&self, peer_id: u64) -> Result<()> {
        self.sync.sync_with_peer(peer_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_null_log_sync() {
        let sync = NullLogSync;
        assert!(sync.sync_with_peer(1).await.is_ok());
        let missing = sync.missing_logs(1).await.unwrap();
        assert!(missing.is_empty());
        assert!(sync.apply_logs(vec![]).await.is_ok());
    }

    #[tokio::test]
    async fn test_log_sync_manager() {
        let config = LogSyncConfig::default();
        let sync = Arc::new(NullLogSync);
        let manager = LogSyncManager::new(config, sync);
        let handle = manager.start();
        handle.abort(); // stop immediately
    }
}