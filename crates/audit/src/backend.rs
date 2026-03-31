//! Backends for audit logging.

use crate::error::Error;
use crate::event::AuditEvent;
use async_trait::async_trait;
use std::path::PathBuf;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncWriteExt, BufWriter};

/// Trait for audit backends.
#[async_trait]
pub trait Backend {
    /// Write an audit event.
    async fn write(&self, event: &AuditEvent) -> Result<(), Error>;
    /// Flush any buffered data.
    async fn flush(&self) -> Result<(), Error>;
}

/// File backend.
pub struct FileBackend {
    path: PathBuf,
    writer: RwLock<BufWriter<File>>,
}

impl FileBackend {
    /// Create a new file backend.
    pub fn new<P: Into<PathBuf>>(path: P) -> Result<Self, Error> {
        let path = path.into();
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| Error::Io(e))?;
        let writer = BufWriter::new(file);
        Ok(Self {
            path,
            writer: RwLock::new(writer),
        })
    }
}

#[async_trait]
impl Backend for FileBackend {
    async fn write(&self, event: &AuditEvent) -> Result<(), Error> {
        let line = serde_json::to_string(event)
            .map_err(|e| Error::Serialization(e.to_string()))?;
        let mut writer = self.writer.write().await;
        writer
            .write_all(line.as_bytes())
            .await
            .map_err(Error::Io)?;
        writer.write_all(b"\n").await.map_err(Error::Io)?;
        Ok(())
    }

    async fn flush(&self) -> Result<(), Error> {
        let mut writer = self.writer.write().await;
        writer.flush().await.map_err(Error::Io)
    }
}

/// Elasticsearch backend (requires `elasticsearch` feature).
#[cfg(feature = "elasticsearch")]
pub struct ElasticsearchBackend {
    client: elasticsearch::Elasticsearch,
    index: String,
}

#[cfg(feature = "elasticsearch")]
impl ElasticsearchBackend {
    /// Create a new Elasticsearch backend.
    pub fn new(client: elasticsearch::Elasticsearch, index: String) -> Self {
        Self { client, index }
    }
}

#[cfg(feature = "elasticsearch")]
#[async_trait]
impl Backend for ElasticsearchBackend {
    async fn write(&self, event: &AuditEvent) -> Result<(), Error> {
        use elasticsearch::IndexParts;

        let response = self
            .client
            .index(IndexParts::Index(&self.index))
            .body(event)
            .send()
            .await
            .map_err(|e| Error::Backend(format!("Elasticsearch error: {}", e)))?;

        if !response.status_code().is_success() {
            return Err(Error::Backend(format!(
                "Elasticsearch returned {}",
                response.status_code()
            )));
        }
        Ok(())
    }

    async fn flush(&self) -> Result<(), Error> {
        // Elasticsearch does not need explicit flush.
        Ok(())
    }
}

/// Loki backend (requires `loki` feature).
#[cfg(feature = "loki")]
pub struct LokiBackend {
    url: String,
    client: reqwest::Client,
    labels: std::collections::HashMap<String, String>,
}

#[cfg(feature = "loki")]
impl LokiBackend {
    /// Create a new Loki backend.
    pub fn new(url: String, labels: std::collections::HashMap<String, String>) -> Self {
        Self {
            url,
            client: reqwest::Client::new(),
            labels,
        }
    }
}

#[cfg(feature = "loki")]
#[async_trait]
impl Backend for LokiBackend {
    async fn write(&self, event: &AuditEvent) -> Result<(), Error> {
        let log_line = serde_json::to_string(event)
            .map_err(|e| Error::Serialization(e.to_string()))?;
        let timestamp_ns = event.timestamp.timestamp_nanos();

        let payload = serde_json::json!({
            "streams": [{
                "stream": self.labels,
                "values": [[timestamp_ns.to_string(), log_line]]
            }]
        });

        let response = self
            .client
            .post(&self.url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| Error::Backend(format!("Loki error: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::Backend(format!(
                "Loki returned {}",
                response.status()
            )));
        }
        Ok(())
    }

    async fn flush(&self) -> Result<(), Error> {
        Ok(())
    }
}