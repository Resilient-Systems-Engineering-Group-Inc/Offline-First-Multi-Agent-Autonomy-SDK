//! Edge-cloud synchronization.

use crate::{EdgeDevice, EdgeTask};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn};

/// Sync manager for edge-cloud coordination.
pub struct SyncManager {
    sync_interval_ms: u64,
    last_sync: u64,
    pending_syncs: HashMap<String, SyncOperation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncOperation {
    pub operation_id: String,
    pub sync_type: SyncType,
    pub data: SyncData,
    pub status: SyncStatus,
    pub created_at: u64,
    pub completed_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncType {
    State,
    Task,
    Configuration,
    Metrics,
    Log,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncData {
    EdgeState(EdgeDevice),
    TaskResult(TaskSyncData),
    Configuration(ConfigSyncData),
    Metrics(MetricsSyncData),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSyncData {
    pub task_id: String,
    pub result: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSyncData {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSyncData {
    pub metrics: HashMap<String, f64>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

impl SyncManager {
    /// Create new sync manager.
    pub fn new(sync_interval_ms: u64) -> Self {
        Self {
            sync_interval_ms,
            last_sync: 0,
            pending_syncs: HashMap::new(),
        }
    }

    /// Queue state sync for edge.
    pub fn queue_state_sync(&mut self, edge: EdgeDevice) {
        let operation = SyncOperation {
            operation_id: uuid::Uuid::new_v4().to_string(),
            sync_type: SyncType::State,
            data: SyncData::EdgeState(edge),
            status: SyncStatus::Pending,
            created_at: chrono::Utc::now().timestamp() as u64,
            completed_at: None,
        };

        self.pending_syncs.insert(operation.operation_id.clone(), operation);
        info!("Queued state sync: {}", operation.operation_id);
    }

    /// Queue task result sync.
    pub fn queue_task_sync(&mut self, task_id: &str, result: Option<String>, error: Option<String>) {
        let operation = SyncOperation {
            operation_id: uuid::Uuid::new_v4().to_string(),
            sync_type: SyncType::Task,
            data: SyncData::TaskResult(TaskSyncData {
                task_id: task_id.to_string(),
                result,
                error,
            }),
            status: SyncStatus::Pending,
            created_at: chrono::Utc::now().timestamp() as u64,
            completed_at: None,
        };

        self.pending_syncs.insert(operation.operation_id.clone(), operation);
        info!("Queued task sync: {}", operation.operation_id);
    }

    /// Sync pending operations.
    pub async fn sync_pending(&mut self) -> Result<Vec<String>> {
        let mut completed = Vec::new();

        for (id, operation) in &mut self.pending_syncs {
            if operation.status == SyncStatus::Pending {
                // Check if should sync
                if self.should_sync() {
                    operation.status = SyncStatus::InProgress;

                    // Perform sync (would call cloud API)
                    match self.execute_sync(operation).await {
                        Ok(_) => {
                            operation.status = SyncStatus::Completed;
                            operation.completed_at = Some(chrono::Utc::now().timestamp() as u64);
                            completed.push(id.clone());
                            info!("Sync completed: {}", id);
                        }
                        Err(e) => {
                            operation.status = SyncStatus::Failed;
                            warn!("Sync failed {}: {}", id, e);
                        }
                    }
                }
            }
        }

        // Remove completed operations
        completed.iter().for_each(|id| {
            self.pending_syncs.remove(id);
        });

        Ok(completed)
    }

    /// Check if should sync based on interval.
    fn should_sync(&self) -> bool {
        let now = chrono::Utc::now().timestamp() as u64;
        (now - self.last_sync) * 1000 >= self.sync_interval_ms
    }

    /// Execute sync operation.
    async fn execute_sync(&self, operation: &SyncOperation) -> Result<()> {
        // Simulate sync operation
        // In production, would call cloud API
        
        // Update last sync time
        // self.last_sync = chrono::Utc::now().timestamp() as u64;

        Ok(())
    }

    /// Get pending sync count.
    pub fn pending_count(&self) -> usize {
        self.pending_syncs.len()
    }

    /// Clear old sync operations.
    pub fn clear_old_syncs(&mut self, max_age_secs: u64) {
        let now = chrono::Utc::now().timestamp() as u64;
        
        self.pending_syncs.retain(|_, op| {
            now - op.created_at < max_age_secs
        });
    }

    /// Get sync statistics.
    pub fn get_stats(&self) -> SyncStats {
        let total = self.pending_syncs.len();
        let pending = self.pending_syncs.values()
            .filter(|op| op.status == SyncStatus::Pending)
            .count();
        let in_progress = self.pending_syncs.values()
            .filter(|op| op.status == SyncStatus::InProgress)
            .count();

        SyncStats {
            total_pending: total as i64,
            pending,
            in_progress,
            last_sync_time: self.last_sync,
        }
    }
}

/// Sync statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStats {
    pub total_pending: i64,
    pub pending: usize,
    pub in_progress: usize,
    pub last_sync_time: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sync_manager() {
        let mut manager = SyncManager::new(1000); // 1 second interval

        // Queue sync
        let edge = EdgeDevice::new("edge-1");
        manager.queue_state_sync(edge);

        assert_eq!(manager.pending_count(), 1);

        // Get stats
        let stats = manager.get_stats();
        assert_eq!(stats.pending, 1);
    }
}
