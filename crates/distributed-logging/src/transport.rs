//! Transport layer for sending logs across the network.

use crate::error::{Result, LogError};
use crate::log_record::LogRecord;
use async_trait::async_trait;
use bytes::Bytes;
use futures::stream::BoxStream;
use std::sync::Arc;

/// Trait for sending log records to remote destinations.
#[async_trait]
pub trait LogTransport: Send + Sync {
    /// Sends a single log record.
    async fn send(&self, record: &LogRecord) -> Result<()>;

    /// Sends a batch of log records.
    async fn send_batch(&self, records: &[LogRecord]) -> Result<()>;

    /// Returns a stream of incoming log records from other agents.
    fn incoming(&self) -> BoxStream<'static, Result<LogRecord>>;
}

/// Dummy transport that discards all logs (useful for testing).
pub struct NullTransport;

#[async_trait]
impl LogTransport for NullTransport {
    async fn send(&self, _record: &LogRecord) -> Result<()> {
        Ok(())
    }

    async fn send_batch(&self, _records: &[LogRecord]) -> Result<()> {
        Ok(())
    }

    fn incoming(&self) -> BoxStream<'static, Result<LogRecord>> {
        futures::stream::empty().boxed()
    }
}

/// Transport that writes logs to a local channel (in‑memory).
pub struct ChannelTransport {
    sender: tokio::sync::mpsc::UnboundedSender<LogRecord>,
    receiver: Arc<tokio::sync::Mutex<tokio::sync::mpsc::UnboundedReceiver<LogRecord>>>,
}

impl ChannelTransport {
    /// Creates a new channel transport with an unbounded channel.
    pub fn new() -> Self {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        Self {
            sender,
            receiver: Arc::new(tokio::sync::Mutex::new(receiver)),
        }
    }

    /// Returns a clone of the sender.
    pub fn sender(&self) -> tokio::sync::mpsc::UnboundedSender<LogRecord> {
        self.sender.clone()
    }
}

#[async_trait]
impl LogTransport for ChannelTransport {
    async fn send(&self, record: &LogRecord) -> Result<()> {
        self.sender
            .send(record.clone())
            .map_err(|e| LogError::Transport(e.to_string()))?;
        Ok(())
    }

    async fn send_batch(&self, records: &[LogRecord]) -> Result<()> {
        for record in records {
            self.send(record).await?;
        }
        Ok(())
    }

    fn incoming(&self) -> BoxStream<'static, Result<LogRecord>> {
        let receiver = self.receiver.clone();
        Box::pin(futures::stream::unfold(receiver, |rx| async move {
            let mut guard = rx.lock().await;
            guard.recv().await.map(|record| (Ok(record), rx))
        }))
    }
}

/// Configuration for network‑based log transport.
pub struct NetworkTransportConfig {
    /// Maximum packet size in bytes.
    pub max_packet_size: usize,
    /// Whether to compress payloads.
    pub compress: bool,
    /// Timeout for sending.
    pub send_timeout: std::time::Duration,
}

impl Default for NetworkTransportConfig {
    fn default() -> Self {
        Self {
            max_packet_size: 65536,
            compress: false,
            send_timeout: std::time::Duration::from_secs(5),
        }
    }
}

/// A sink that forwards logs via a transport.
pub struct TransportSink<T: LogTransport> {
    transport: Arc<T>,
}

impl<T: LogTransport> TransportSink<T> {
    /// Creates a new transport sink.
    pub fn new(transport: Arc<T>) -> Self {
        Self { transport }
    }
}

#[async_trait]
impl<T: LogTransport> crate::sink::LogSink for TransportSink<T> {
    async fn write(&self, record: &LogRecord) -> Result<()> {
        self.transport.send(record).await
    }

    async fn flush(&self) -> Result<()> {
        Ok(())
    }

    async fn close(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_null_transport() {
        let transport = NullTransport;
        let record = LogRecord::new(
            crate::log_record::LogLevel::Info,
            "agent",
            "test",
            "msg",
        );
        assert!(transport.send(&record).await.is_ok());
        let mut stream = transport.incoming();
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn test_channel_transport() {
        let transport = ChannelTransport::new();
        let record = LogRecord::new(
            crate::log_record::LogLevel::Warn,
            "agent",
            "test",
            "hello",
        );
        transport.send(&record).await.unwrap();

        let mut stream = transport.incoming();
        let received = stream.next().await.unwrap().unwrap();
        assert_eq!(received.message, "hello");
    }
}