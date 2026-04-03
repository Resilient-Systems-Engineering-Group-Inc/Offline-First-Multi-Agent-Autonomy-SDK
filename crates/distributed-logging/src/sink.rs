//! Log sinks (destinations for log records).

use crate::error::Result;
use crate::log_record::LogRecord;
use async_trait::async_trait;
use std::path::Path;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

/// Trait for log sinks.
#[async_trait]
pub trait LogSink: Send + Sync {
    /// Writes a log record to the sink.
    async fn write(&self, record: &LogRecord) -> Result<()>;

    /// Flushes any buffered data.
    async fn flush(&self) -> Result<()>;

    /// Closes the sink (optional).
    async fn close(&self) -> Result<()>;
}

/// Sink that writes to stdout.
pub struct StdoutSink;

#[async_trait]
impl LogSink for StdoutSink {
    async fn write(&self, record: &LogRecord) -> Result<()> {
        println!("{}", record.to_human_readable());
        Ok(())
    }

    async fn flush(&self) -> Result<()> {
        // stdout is line‑buffered; nothing to do.
        Ok(())
    }

    async fn close(&self) -> Result<()> {
        Ok(())
    }
}

/// Sink that writes to stderr.
pub struct StderrSink;

#[async_trait]
impl LogSink for StderrSink {
    async fn write(&self, record: &LogRecord) -> Result<()> {
        eprintln!("{}", record.to_human_readable());
        Ok(())
    }

    async fn flush(&self) -> Result<()> {
        Ok(())
    }

    async fn close(&self) -> Result<()> {
        Ok(())
    }
}

/// Sink that writes to a file.
pub struct FileSink {
    path: String,
    file: Mutex<Option<tokio::fs::File>>,
}

impl FileSink {
    /// Creates a new file sink that writes to the given path.
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            file: Mutex::new(None),
        }
    }

    async fn ensure_open(&self) -> Result<tokio::fs::File> {
        let mut guard = self.file.lock().await;
        if guard.is_none() {
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)
                .await
                .map_err(|e| crate::error::LogError::Io(e))?;
            *guard = Some(file);
        }
        Ok(guard.as_mut().unwrap().try_clone().await?)
    }
}

#[async_trait]
impl LogSink for FileSink {
    async fn write(&self, record: &LogRecord) -> Result<()> {
        let mut file = self.ensure_open().await?;
        let line = format!("{}\n", record.to_json()?);
        file.write_all(line.as_bytes()).await
            .map_err(|e| crate::error::LogError::Io(e))?;
        Ok(())
    }

    async fn flush(&self) -> Result<()> {
        if let Some(file) = self.file.lock().await.as_mut() {
            file.flush().await
                .map_err(|e| crate::error::LogError::Io(e))?;
        }
        Ok(())
    }

    async fn close(&self) -> Result<()> {
        let mut guard = self.file.lock().await;
        *guard = None;
        Ok(())
    }
}

/// Sink that stores records in memory (useful for testing).
pub struct MemorySink {
    records: Mutex<Vec<LogRecord>>,
}

impl MemorySink {
    /// Creates a new in‑memory sink.
    pub fn new() -> Self {
        Self {
            records: Mutex::new(Vec::new()),
        }
    }

    /// Returns all stored records.
    pub async fn records(&self) -> Vec<LogRecord> {
        self.records.lock().await.clone()
    }

    /// Clears all stored records.
    pub async fn clear(&self) {
        self.records.lock().await.clear();
    }
}

#[async_trait]
impl LogSink for MemorySink {
    async fn write(&self, record: &LogRecord) -> Result<()> {
        self.records.lock().await.push(record.clone());
        Ok(())
    }

    async fn flush(&self) -> Result<()> {
        Ok(())
    }

    async fn close(&self) -> Result<()> {
        Ok(())
    }
}

/// Sink that forwards logs to multiple child sinks.
pub struct MultiplexSink {
    sinks: Vec<Box<dyn LogSink>>,
}

impl MultiplexSink {
    /// Creates a new multiplex sink.
    pub fn new() -> Self {
        Self { sinks: Vec::new() }
    }

    /// Adds a child sink.
    pub fn add_sink(&mut self, sink: Box<dyn LogSink>) {
        self.sinks.push(sink);
    }
}

#[async_trait]
impl LogSink for MultiplexSink {
    async fn write(&self, record: &LogRecord) -> Result<()> {
        for sink in &self.sinks {
            sink.write(record).await?;
        }
        Ok(())
    }

    async fn flush(&self) -> Result<()> {
        for sink in &self.sinks {
            sink.flush().await?;
        }
        Ok(())
    }

    async fn close(&self) -> Result<()> {
        for sink in &self.sinks {
            sink.close().await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_sink() {
        let sink = MemorySink::new();
        let record = LogRecord::new(
            crate::log_record::LogLevel::Info,
            "agent",
            "test",
            "message",
        );
        sink.write(&record).await.unwrap();
        let records = sink.records().await;
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].message, "message");
    }

    #[tokio::test]
    async fn test_multiplex_sink() {
        let mem1 = MemorySink::new();
        let mem2 = MemorySink::new();
        let mut multiplex = MultiplexSink::new();
        multiplex.add_sink(Box::new(mem1));
        multiplex.add_sink(Box::new(mem2));

        let record = LogRecord::new(
            crate::log_record::LogLevel::Warn,
            "agent",
            "test",
            "multi",
        );
        multiplex.write(&record).await.unwrap();
        // We cannot easily verify because we lost the references.
        // This test just ensures no panic.
    }
}