//! Audit logging for security configuration changes.

use crate::config::{AuditConfig, AuditSeverity, AuditSink};
use crate::error::{Result, SecurityConfigError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

/// An audit event representing a security‑relevant action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Unique event ID.
    pub id: String,
    /// Timestamp of the event.
    pub timestamp: DateTime<Utc>,
    /// Severity level.
    pub severity: AuditSeverity,
    /// Component that generated the event.
    pub component: String,
    /// Action performed (e.g., "config_updated", "policy_violation").
    pub action: String,
    /// Agent ID (if applicable).
    pub agent_id: Option<String>,
    /// Resource affected.
    pub resource: String,
    /// Outcome (success/failure).
    pub outcome: Outcome,
    /// Additional details as key‑value pairs.
    pub details: serde_json::Value,
}

/// Outcome of an audited action.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Outcome {
    /// The action succeeded.
    Success,
    /// The action failed.
    Failure,
    /// The action was denied.
    Denied,
    /// The action was allowed.
    Allowed,
}

impl AuditEvent {
    /// Creates a new audit event.
    pub fn new(
        severity: AuditSeverity,
        component: impl Into<String>,
        action: impl Into<String>,
        resource: impl Into<String>,
        outcome: Outcome,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            severity,
            component: component.into(),
            action: action.into(),
            agent_id: None,
            resource: resource.into(),
            outcome,
            details: serde_json::json!({}),
        }
    }

    /// Sets the agent ID.
    pub fn with_agent_id(mut self, agent_id: impl Into<String>) -> Self {
        self.agent_id = Some(agent_id.into());
        self
    }

    /// Adds a detail key‑value pair.
    pub fn with_detail(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.details
            .as_object_mut()
            .unwrap()
            .insert(key.into(), value);
        self
    }

    /// Converts the event to JSON.
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).map_err(|e| SecurityConfigError::Json(e))
    }
}

/// Trait for audit sinks (destinations of audit events).
#[async_trait::async_trait]
pub trait AuditSinkTrait: Send + Sync {
    /// Writes an audit event to the sink.
    async fn write(&self, event: &AuditEvent) -> Result<()>;
}

/// Audit sink that writes to a file.
pub struct FileSink {
    path: String,
}

impl FileSink {
    /// Creates a new file sink.
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }
}

#[async_trait::async_trait]
impl AuditSinkTrait for FileSink {
    async fn write(&self, event: &AuditEvent) -> Result<()> {
        let line = format!("{}\n", event.to_json()?);
        tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .await
            .map_err(|e| SecurityConfigError::Audit(e.to_string()))?
            .write_all(line.as_bytes())
            .await
            .map_err(|e| SecurityConfigError::Audit(e.to_string()))?;
        Ok(())
    }
}

/// Audit sink that discards events (no‑op).
pub struct NullSink;

#[async_trait::async_trait]
impl AuditSinkTrait for NullSink {
    async fn write(&self, _event: &AuditEvent) -> Result<()> {
        Ok(())
    }
}

/// Audit sink that sends events via the mesh transport (if enabled).
#[cfg(feature = "mesh-transport")]
pub struct MeshSink {
    // In a real implementation you would hold a reference to a mesh transport.
}

#[cfg(feature = "mesh-transport")]
#[async_trait::async_trait]
impl AuditSinkTrait for MeshSink {
    async fn write(&self, event: &AuditEvent) -> Result<()> {
        // For now, just log to tracing.
        tracing::info!("[AUDIT via mesh] {:?}", event);
        Ok(())
    }
}

/// Main audit logger that routes events to the configured sink.
pub struct AuditLogger {
    config: AuditConfig,
    sink: Arc<dyn AuditSinkTrait>,
    enabled: bool,
}

impl AuditLogger {
    /// Creates a new audit logger from a configuration.
    pub fn new(config: AuditConfig) -> Result<Self> {
        let sink: Arc<dyn AuditSinkTrait> = match &config.sink {
            AuditSink::File { path } => Arc::new(FileSink::new(path)),
            AuditSink::Null => Arc::new(NullSink),
            #[cfg(feature = "mesh-transport")]
            AuditSink::Mesh => Arc::new(MeshSink {}),
            _ => {
                // For simplicity, fall back to Null sink.
                // In a real implementation you would implement Http, Syslog, etc.
                tracing::warn!("Unsupported audit sink type, using Null sink");
                Arc::new(NullSink)
            }
        };

        Ok(Self {
            config,
            sink,
            enabled: config.enabled,
        })
    }

    /// Logs an audit event if the logger is enabled and the event's severity is sufficient.
    pub async fn log(&self, event: AuditEvent) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }
        if event.severity < self.config.min_severity {
            return Ok(());
        }
        if !self.config.log_success && event.outcome == Outcome::Success {
            return Ok(());
        }

        self.sink.write(&event).await
    }

    /// Returns whether the logger is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Updates the configuration (e.g., after a hot‑reload).
    pub fn update_config(&mut self, config: AuditConfig) -> Result<()> {
        self.config = config.clone();
        self.enabled = config.enabled;
        // Re‑create the sink if needed (simplified: we keep the same sink)
        Ok(())
    }
}

/// Global audit logger (singleton pattern).
pub struct GlobalAuditLogger {
    inner: Arc<Mutex<Option<AuditLogger>>>,
}

impl GlobalAuditLogger {
    /// Creates a new global logger (initially uninitialized).
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(None)),
        }
    }

    /// Initializes the global logger with a configuration.
    pub async fn init(&self, config: AuditConfig) -> Result<()> {
        let mut guard = self.inner.lock().await;
        *guard = Some(AuditLogger::new(config)?);
        Ok(())
    }

    /// Logs an event using the global logger (if initialized).
    pub async fn log(&self, event: AuditEvent) -> Result<()> {
        let guard = self.inner.lock().await;
        if let Some(logger) = &*guard {
            logger.log(event).await
        } else {
            // If not initialized, just drop the event.
            Ok(())
        }
    }

    /// Returns whether the global logger is initialized and enabled.
    pub async fn is_enabled(&self) -> bool {
        let guard = self.inner.lock().await;
        guard.as_ref().map(|l| l.is_enabled()).unwrap_or(false)
    }
}

impl Default for GlobalAuditLogger {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_file_sink() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap().to_string();
        let sink = FileSink::new(&path);
        let event = AuditEvent::new(
            AuditSeverity::Info,
            "test",
            "action",
            "resource",
            Outcome::Success,
        );
        assert!(sink.write(&event).await.is_ok());
        // Verify file contains the event line
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("\"action\":\"action\""));
    }

    #[tokio::test]
    async fn test_audit_logger() {
        let config = AuditConfig {
            enabled: true,
            sink: AuditSink::Null,
            min_severity: AuditSeverity::Info,
            log_success: true,
        };
        let logger = AuditLogger::new(config).unwrap();
        let event = AuditEvent::new(
            AuditSeverity::Info,
            "test",
            "action",
            "resource",
            Outcome::Success,
        );
        assert!(logger.log(event).await.is_ok());
    }

    #[tokio::test]
    async fn test_global_logger() {
        let global = GlobalAuditLogger::new();
        assert!(!global.is_enabled().await);

        let config = AuditConfig {
            enabled: true,
            sink: AuditSink::Null,
            min_severity: AuditSeverity::Info,
            log_success: true,
        };
        assert!(global.init(config).await.is_ok());
        assert!(global.is_enabled().await);

        let event = AuditEvent::new(
            AuditSeverity::Info,
            "test",
            "action",
            "resource",
            Outcome::Success,
        );
        assert!(global.log(event).await.is_ok());
    }
}