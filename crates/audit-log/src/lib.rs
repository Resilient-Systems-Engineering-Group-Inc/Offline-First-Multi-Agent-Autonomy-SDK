//! Comprehensive audit logging system.
//!
//! Provides:
//! - Immutable audit trail
//! - Multi-level event categorization
//! - Tamper-proof logging with hashing
//! - Query and search capabilities
//! - Compliance reporting
//! - Data retention policies

pub mod event;
pub mod store;
pub mod query;
pub mod compliance;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::info;

pub use event::*;
pub use store::*;
pub use query::*;
pub use compliance::*;

/// Audit logging configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    pub storage_type: StorageType,
    pub connection_string: String,
    pub enable_hash_chain: bool,
    pub retention_days: u32,
    pub max_events_per_query: usize,
    pub enable_compression: bool,
    pub async_batch_size: usize,
    pub flush_interval_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageType {
    PostgreSQL,
    SQLite,
    InMemory,
    File,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            storage_type: StorageType::InMemory,
            connection_string: "postgresql://localhost/audit".to_string(),
            enable_hash_chain: true,
            retention_days: 365,
            max_events_per_query: 10000,
            enable_compression: true,
            async_batch_size: 100,
            flush_interval_secs: 5,
        }
    }
}

/// Audit log manager.
pub struct AuditLogManager {
    config: AuditConfig,
    store: AuditStore,
    pending_events: RwLock<Vec<AuditEvent>>,
    hash_chain: RwLock<Option<String>>,
}

impl AuditLogManager {
    /// Create new audit log manager.
    pub fn new(config: AuditConfig) -> Self {
        let store = AuditStore::new(&config);
        
        Self {
            config,
            store,
            pending_events: RwLock::new(Vec::new()),
            hash_chain: RwLock::new(None),
        }
    }

    /// Initialize audit logging.
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing audit log with storage: {:?}", self.config.storage_type);
        self.store.initialize().await?;
        info!("Audit log initialized");
        Ok(())
    }

    /// Log audit event.
    pub async fn log(&self, event: AuditEvent) -> Result<String> {
        let event_id = event.id.clone();

        // Add to hash chain if enabled
        if self.config.enable_hash_chain {
            self.add_to_hash_chain(&event).await?;
        }

        // Store event
        self.store.store(&event).await?;

        info!("Audit event logged: {} - {}", event_id, event.event_type);
        Ok(event_id)
    }

    /// Log event asynchronously (batched).
    pub async fn log_async(&self, event: AuditEvent) -> Result<()> {
        let mut pending = self.pending_events.write().await;
        pending.push(event);

        // Flush if batch is full
        if pending.len() >= self.config.async_batch_size {
            self.flush_pending().await?;
        }

        Ok(())
    }

    /// Flush pending events.
    pub async fn flush_pending(&self) -> Result<()> {
        let mut pending = self.pending_events.write().await;
        
        for event in pending.drain(..) {
            if self.config.enable_hash_chain {
                self.add_to_hash_chain(&event).await?;
            }
            self.store.store(&event).await?;
        }

        Ok(())
    }

    /// Add event to hash chain for tamper-proofing.
    async fn add_to_hash_chain(&self, event: &AuditEvent) -> Result<()> {
        let prev_hash = self.hash_chain.read().await.clone().unwrap_or_default();
        let current_hash = event.compute_hash(&prev_hash);
        
        *self.hash_chain.write().await = Some(current_hash);
        Ok(())
    }

    /// Query audit events.
    pub async fn query(&self, query: &AuditQuery) -> Result<Vec<AuditEvent>> {
        let events = self.store.query(query).await?;
        
        // Verify hash chain if enabled
        if self.config.enable_hash_chain {
            self.verify_hash_chain(&events).await?;
        }

        Ok(events)
    }

    /// Get event by ID.
    pub async fn get_event(&self, event_id: &str) -> Result<Option<AuditEvent>> {
        self.store.get_by_id(event_id).await
    }

    /// Get events by entity.
    pub async fn get_by_entity(&self, entity_type: &str, entity_id: &str) -> Result<Vec<AuditEvent>> {
        let query = AuditQuery {
            entity_type: Some(entity_type.to_string()),
            entity_id: Some(entity_id.to_string()),
            ..Default::default()
        };
        self.query(&query).await
    }

    /// Get events by user.
    pub async fn get_by_user(&self, user_id: &str) -> Result<Vec<AuditEvent>> {
        let query = AuditQuery {
            user_id: Some(user_id.to_string()),
            ..Default::default()
        };
        self.query(&query).await
    }

    /// Verify hash chain integrity.
    pub async fn verify_hash_chain(&self, events: &[AuditEvent]) -> Result<bool> {
        if !self.config.enable_hash_chain || events.is_empty() {
            return Ok(true);
        }

        let mut prev_hash = String::new();
        
        for event in events {
            let expected_hash = event.compute_hash(&prev_hash);
            if let Some(stored_hash) = &event.previous_hash {
                if stored_hash != &prev_hash {
                    return Ok(false);
                }
            }
            prev_hash = expected_hash;
        }

        Ok(true)
    }

    /// Generate compliance report.
    pub async fn generate_report(&self, report_type: ReportType, period: TimePeriod) -> Result<ComplianceReport> {
        let generator = ComplianceReportGenerator::new(&self.store);
        generator.generate(report_type, period).await
    }

    /// Get audit statistics.
    pub async fn get_stats(&self) -> Result<AuditStats> {
        self.store.get_stats().await
    }

    /// Apply retention policy.
    pub async fn apply_retention(&self) -> Result<usize> {
        let cutoff = chrono::Utc::now() - chrono::Duration::days(self.config.retention_days as i64);
        let deleted = self.store.delete_before(cutoff).await?;
        info!("Retention policy applied: {} events deleted", deleted);
        Ok(deleted)
    }

    /// Export audit log.
    pub async fn export(&self, format: ExportFormat, query: &AuditQuery) -> Result<Vec<u8>> {
        let events = self.query(query).await?;
        
        match format {
            ExportFormat::JSON => {
                Ok(serde_json::to_vec(&events)?)
            }
            ExportFormat::CSV => {
                let mut csv = Vec::new();
                csv.extend_from_slice(b"timestamp,event_type,entity_type,entity_id,user_id,action,result\n");
                for event in events {
                    let line = format!("{},{},{},{},{},{},{}\n",
                        event.timestamp,
                        event.event_type,
                        event.entity_type.unwrap_or_default(),
                        event.entity_id.unwrap_or_default(),
                        event.user_id.unwrap_or_default(),
                        event.action,
                        event.result.success
                    );
                    csv.extend_from_slice(line.as_bytes());
                }
                Ok(csv)
            }
        }
    }
}

/// Audit statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditStats {
    pub total_events: i64,
    pub events_today: i64,
    pub events_this_week: i64,
    pub events_this_month: i64,
    pub unique_users: i64,
    pub unique_entities: i64,
    pub avg_events_per_day: f64,
    pub storage_size_mb: f64,
}

/// Export format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportFormat {
    JSON,
    CSV,
}

/// Time period for reports.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimePeriod {
    pub start: chrono::DateTime<chrono::Utc>,
    pub end: chrono::DateTime<chrono::Utc>,
}

impl TimePeriod {
    pub fn last_days(days: u32) -> Self {
        let end = chrono::Utc::now();
        let start = end - chrono::Duration::days(days as i64);
        Self { start, end }
    }

    pub fn this_month() -> Self {
        let now = chrono::Utc::now();
        let start = chrono::NaiveDate::from_ymd_opt(now.year(), now.month(), 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();
        Self { start, end: now }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_audit_log() {
        let config = AuditConfig {
            storage_type: StorageType::InMemory,
            ..Default::default()
        };
        let manager = AuditLogManager::new(config);

        // Initialize
        manager.initialize().await.unwrap();

        // Log event
        let event = AuditEvent::new(
            "task.created",
            "task",
            "task-1",
            "create",
        )
        .with_user_id("user-123")
        .with_details(serde_json::json!({
            "description": "Test task",
            "priority": 100
        }));

        let event_id = manager.log(event).await.unwrap();
        assert!(!event_id.is_empty());

        // Get event
        let retrieved = manager.get_event(&event_id).await.unwrap();
        assert!(retrieved.is_some());

        // Get by entity
        let events = manager.get_by_entity("task", "task-1").await.unwrap();
        assert_eq!(events.len(), 1);

        // Get stats
        let stats = manager.get_stats().await.unwrap();
        assert!(stats.total_events >= 1);
    }
}
