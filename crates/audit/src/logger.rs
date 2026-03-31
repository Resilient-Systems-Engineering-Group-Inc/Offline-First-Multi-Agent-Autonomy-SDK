//! Audit logger.

use crate::backend::{Backend, FileBackend};
use crate::error::Error;
use crate::event::AuditEvent;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Central audit logger.
pub struct AuditLogger {
    backends: Vec<Arc<dyn Backend + Send + Sync>>,
    cache: Arc<DashMap<uuid::Uuid, AuditEvent>>,
}

impl AuditLogger {
    /// Create a new audit logger with default backends.
    pub fn new() -> Result<Self, Error> {
        let file_backend = Arc::new(FileBackend::new("audit.log")?);
        Ok(Self {
            backends: vec![file_backend],
            cache: Arc::new(DashMap::new()),
        })
    }

    /// Add a backend.
    pub fn add_backend(&mut self, backend: Arc<dyn Backend + Send + Sync>) {
        self.backends.push(backend);
    }

    /// Log an audit event.
    pub async fn log(&self, event: AuditEvent) -> Result<(), Error> {
        // Store in cache.
        self.cache.insert(event.id, event.clone());

        // Write to all backends.
        for backend in &self.backends {
            backend.write(&event).await?;
        }

        Ok(())
    }

    /// Retrieve an event by ID.
    pub fn get(&self, id: uuid::Uuid) -> Option<AuditEvent> {
        self.cache.get(&id).map(|e| e.clone())
    }

    /// Flush all backends.
    pub async fn flush(&self) -> Result<(), Error> {
        for backend in &self.backends {
            backend.flush().await?;
        }
        Ok(())
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new().unwrap_or_else(|e| {
            panic!("Failed to create default audit logger: {}", e);
        })
    }
}