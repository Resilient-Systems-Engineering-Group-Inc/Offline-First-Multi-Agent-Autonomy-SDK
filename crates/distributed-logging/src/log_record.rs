//! Log record structure and serialization.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Log severity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LogLevel {
    /// Trace – extremely verbose debugging.
    Trace,
    /// Debug – debugging information.
    Debug,
    /// Info – normal operational messages.
    Info,
    /// Warn – warnings that may indicate problems.
    Warn,
    /// Error – error conditions that need attention.
    Error,
    /// Critical – critical failures.
    Critical,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Trace => write!(f, "TRACE"),
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
            LogLevel::Critical => write!(f, "CRITICAL"),
        }
    }
}

impl TryFrom<&str> for LogLevel {
    type Error = crate::error::LogError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s.to_ascii_lowercase().as_str() {
            "trace" => Ok(LogLevel::Trace),
            "debug" => Ok(LogLevel::Debug),
            "info" => Ok(LogLevel::Info),
            "warn" | "warning" => Ok(LogLevel::Warn),
            "error" => Ok(LogLevel::Error),
            "critical" => Ok(LogLevel::Critical),
            _ => Err(crate::error::LogError::InvalidLevel(s.to_string())),
        }
    }
}

/// A single log record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRecord {
    /// Unique identifier for this log entry.
    pub id: String,
    /// Timestamp when the log was created.
    pub timestamp: DateTime<Utc>,
    /// Log level.
    pub level: LogLevel,
    /// The agent that produced the log.
    pub agent_id: String,
    /// Component/module that generated the log.
    pub component: String,
    /// The log message.
    pub message: String,
    /// Optional structured fields (key‑value pairs).
    #[serde(default)]
    pub fields: HashMap<String, serde_json::Value>,
    /// Optional stack trace or error details.
    #[serde(default)]
    pub stack_trace: Option<String>,
    /// Optional correlation ID for tracing across logs.
    #[serde(default)]
    pub correlation_id: Option<String>,
}

impl LogRecord {
    /// Creates a new log record.
    pub fn new(
        level: LogLevel,
        agent_id: impl Into<String>,
        component: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            level,
            agent_id: agent_id.into(),
            component: component.into(),
            message: message.into(),
            fields: HashMap::new(),
            stack_trace: None,
            correlation_id: None,
        }
    }

    /// Adds a structured field.
    pub fn with_field(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.fields.insert(key.into(), value);
        self
    }

    /// Adds a correlation ID.
    pub fn with_correlation_id(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }

    /// Adds a stack trace.
    pub fn with_stack_trace(mut self, trace: impl Into<String>) -> Self {
        self.stack_trace = Some(trace.into());
        self
    }

    /// Serializes the log record to JSON.
    pub fn to_json(&self) -> Result<String, crate::error::LogError> {
        serde_json::to_string_pretty(self)
            .map_err(|e| crate::error::LogError::Serialization(e.to_string()))
    }

    /// Serializes the log record to CBOR (binary).
    pub fn to_cbor(&self) -> Result<Vec<u8>, crate::error::LogError> {
        serde_cbor::to_vec(self)
            .map_err(|e| crate::error::LogError::Serialization(e.to_string()))
    }

    /// Deserializes a log record from JSON.
    pub fn from_json(json: &str) -> Result<Self, crate::error::LogError> {
        serde_json::from_str(json)
            .map_err(|e| crate::error::LogError::Serialization(e.to_string()))
    }

    /// Deserializes a log record from CBOR.
    pub fn from_cbor(bytes: &[u8]) -> Result<Self, crate::error::LogError> {
        serde_cbor::from_slice(bytes)
            .map_err(|e| crate::error::LogError::Serialization(e.to_string()))
    }

    /// Returns a human‑readable representation of the log.
    pub fn to_human_readable(&self) -> String {
        format!(
            "{} [{}] {}@{}: {}",
            self.timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
            self.level,
            self.agent_id,
            self.component,
            self.message
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_from_str() {
        assert_eq!(LogLevel::try_from("info").unwrap(), LogLevel::Info);
        assert_eq!(LogLevel::try_from("WARN").unwrap(), LogLevel::Warn);
        assert!(LogLevel::try_from("unknown").is_err());
    }

    #[test]
    fn test_log_record_creation() {
        let record = LogRecord::new(
            LogLevel::Info,
            "agent-1",
            "network",
            "Connection established",
        );
        assert_eq!(record.agent_id, "agent-1");
        assert_eq!(record.component, "network");
        assert_eq!(record.level, LogLevel::Info);
        assert!(record.id.len() > 0);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let mut record = LogRecord::new(
            LogLevel::Error,
            "agent-2",
            "security",
            "Authentication failed",
        );
        record.fields.insert("attempts".to_string(), serde_json::json!(3));
        let json = record.to_json().unwrap();
        let parsed = LogRecord::from_json(&json).unwrap();
        assert_eq!(parsed.agent_id, record.agent_id);
        assert_eq!(parsed.fields.get("attempts").unwrap(), &serde_json::json!(3));
    }
}