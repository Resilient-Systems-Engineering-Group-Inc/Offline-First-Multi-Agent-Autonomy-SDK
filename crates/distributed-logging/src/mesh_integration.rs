//! Integration with mesh transport for distributed logging.

use crate::error::{Result, LogError};
use crate::log_record::LogRecord;
use crate::transport::LogTransport;
use async_trait::async_trait;
use futures::stream::BoxStream;
use std::sync::Arc;

/// Transport that sends logs via the mesh network.
#[cfg(feature = "mesh-transport")]
pub struct MeshTransportAdapter {
    // In a real implementation you would hold a reference to a mesh transport.
    // For now we just stub the methods.
}

#[cfg(feature = "mesh-transport")]
impl MeshTransportAdapter {
    /// Creates a new mesh transport adapter.
    pub fn new() -> Self {
        Self {}
    }
}

#[cfg(feature = "mesh-transport")]
#[async_trait]
impl LogTransport for MeshTransportAdapter {
    async fn send(&self, record: &LogRecord) -> Result<()> {
        tracing::info!("[MeshTransportAdapter] Sending log record: {}", record.message);
        // Stub: in reality you would serialize the record and send via mesh.
        Ok(())
    }

    async fn send_batch(&self, records: &[LogRecord]) -> Result<()> {
        tracing::info!("[MeshTransportAdapter] Sending batch of {} records", records.len());
        Ok(())
    }

    fn incoming(&self) -> BoxStream<'static, Result<LogRecord>> {
        // Stub: no incoming logs.
        futures::stream::empty().boxed()
    }
}

/// Configuration for mesh‑based log distribution.
#[cfg(feature = "mesh-transport")]
pub struct MeshLoggingConfig {
    /// Whether to broadcast logs to all peers.
    pub broadcast: bool,
    /// Whether to compress logs before sending.
    pub compress: bool,
    /// Maximum log size per message.
    pub max_log_size: usize,
}

#[cfg(feature = "mesh-transport")]
impl Default for MeshLoggingConfig {
    fn default() -> Self {
        Self {
            broadcast: true,
            compress: true,
            max_log_size: 65536,
        }
    }
}

/// Distributed logger that uses the mesh network.
#[cfg(feature = "mesh-transport")]
pub struct MeshLogger {
    config: MeshLoggingConfig,
    transport: Arc<dyn LogTransport>,
}

#[cfg(feature = "mesh-transport")]
impl MeshLogger {
    /// Creates a new mesh logger.
    pub fn new(config: MeshLoggingConfig, transport: Arc<dyn LogTransport>) -> Self {
        Self { config, transport }
    }

    /// Logs a record and distributes it via mesh.
    pub async fn log(&self, record: LogRecord) -> Result<()> {
        self.transport.send(&record).await
    }

    /// Returns a stream of logs received from other agents.
    pub fn incoming_logs(&self) -> BoxStream<'static, Result<LogRecord>> {
        self.transport.incoming()
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "mesh-transport")]
    use super::*;

    #[cfg(feature = "mesh-transport")]
    #[tokio::test]
    async fn test_mesh_logger_stub() {
        let transport = Arc::new(MeshTransportAdapter::new());
        let config = MeshLoggingConfig::default();
        let logger = MeshLogger::new(config, transport);
        let record = LogRecord::new(
            crate::log_record::LogLevel::Info,
            "agent",
            "test",
            "hello mesh",
        );
        assert!(logger.log(record).await.is_ok());
    }
}