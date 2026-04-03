//! Logger implementation.

use crate::error::Result;
use crate::log_record::{LogLevel, LogRecord};
use crate::sink::LogSink;
use std::sync::Arc;
use tokio::sync::Mutex;

/// A logger that writes log records to one or more sinks.
pub struct Logger {
    agent_id: String,
    component: String,
    sinks: Vec<Arc<dyn LogSink>>,
    min_level: LogLevel,
    enabled: bool,
}

impl Logger {
    /// Creates a new logger for the given agent and component.
    pub fn new(agent_id: impl Into<String>, component: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            component: component.into(),
            sinks: Vec::new(),
            min_level: LogLevel::Info,
            enabled: true,
        }
    }

    /// Sets the minimum log level (records below this level are ignored).
    pub fn with_min_level(mut self, level: LogLevel) -> Self {
        self.min_level = level;
        self
    }

    /// Adds a sink to which logs will be written.
    pub fn add_sink(&mut self, sink: Arc<dyn LogSink>) {
        self.sinks.push(sink);
    }

    /// Enables or disables the logger.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Logs a record at the given level.
    pub async fn log(&self, level: LogLevel, message: impl Into<String>) -> Result<()> {
        if !self.enabled || level < self.min_level {
            return Ok(());
        }

        let record = LogRecord::new(level, &self.agent_id, &self.component, message);
        self.write_record(record).await
    }

    /// Logs a trace message.
    pub async fn trace(&self, message: impl Into<String>) -> Result<()> {
        self.log(LogLevel::Trace, message).await
    }

    /// Logs a debug message.
    pub async fn debug(&self, message: impl Into<String>) -> Result<()> {
        self.log(LogLevel::Debug, message).await
    }

    /// Logs an info message.
    pub async fn info(&self, message: impl Into<String>) -> Result<()> {
        self.log(LogLevel::Info, message).await
    }

    /// Logs a warning message.
    pub async fn warn(&self, message: impl Into<String>) -> Result<()> {
        self.log(LogLevel::Warn, message).await
    }

    /// Logs an error message.
    pub async fn error(&self, message: impl Into<String>) -> Result<()> {
        self.log(LogLevel::Error, message).await
    }

    /// Logs a critical message.
    pub async fn critical(&self, message: impl Into<String>) -> Result<()> {
        self.log(LogLevel::Critical, message).await
    }

    /// Logs a record with additional fields.
    pub async fn log_with_fields(
        &self,
        level: LogLevel,
        message: impl Into<String>,
        fields: Vec<(&str, serde_json::Value)>,
    ) -> Result<()> {
        if !self.enabled || level < self.min_level {
            return Ok(());
        }

        let mut record = LogRecord::new(level, &self.agent_id, &self.component, message);
        for (key, value) in fields {
            record.fields.insert(key.to_string(), value);
        }
        self.write_record(record).await
    }

    /// Writes a record to all sinks.
    async fn write_record(&self, record: LogRecord) -> Result<()> {
        for sink in &self.sinks {
            sink.write(&record).await?;
        }
        Ok(())
    }
}

/// A thread‑safe, cloneable logger wrapper.
#[derive(Clone)]
pub struct SharedLogger {
    inner: Arc<Mutex<Logger>>,
}

impl SharedLogger {
    /// Creates a new shared logger.
    pub fn new(agent_id: impl Into<String>, component: impl Into<String>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Logger::new(agent_id, component))),
        }
    }

    /// Acquires a lock and returns a guard that can be used to log.
    pub async fn lock(&self) -> tokio::sync::MutexGuard<'_, Logger> {
        self.inner.lock().await
    }

    /// Logs a message at the given level (convenience method).
    pub async fn log(&self, level: LogLevel, message: impl Into<String>) -> Result<()> {
        let logger = self.inner.lock().await;
        logger.log(level, message).await
    }

    /// Logs an info message (convenience).
    pub async fn info(&self, message: impl Into<String>) -> Result<()> {
        self.log(LogLevel::Info, message).await
    }

    /// Logs an error message (convenience).
    pub async fn error(&self, message: impl Into<String>) -> Result<()> {
        self.log(LogLevel::Error, message).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sink::MemorySink;

    #[tokio::test]
    async fn test_logger_basic() {
        let sink = Arc::new(MemorySink::new());
        let mut logger = Logger::new("test-agent", "test");
        logger.add_sink(sink.clone());
        logger.info("Hello world").await.unwrap();

        let records = sink.records().await;
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].message, "Hello world");
        assert_eq!(records[0].level, LogLevel::Info);
    }

    #[tokio::test]
    async fn test_logger_level_filter() {
        let sink = Arc::new(MemorySink::new());
        let mut logger = Logger::new("agent", "comp");
        logger.add_sink(sink.clone());
        logger.with_min_level(LogLevel::Warn);
        logger.info("This should be filtered").await.unwrap();
        logger.warn("This should appear").await.unwrap();

        let records = sink.records().await;
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].level, LogLevel::Warn);
    }

    #[tokio::test]
    async fn test_shared_logger() {
        let logger = SharedLogger::new("shared-agent", "shared");
        logger.info("Test message").await.unwrap();
        // No sink added, but should not panic.
    }
}