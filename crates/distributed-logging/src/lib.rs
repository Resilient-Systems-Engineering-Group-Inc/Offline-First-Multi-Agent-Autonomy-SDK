//! Distributed logging for multi‑agent systems.
//!
//! This crate provides a structured, distributed logging system that:
//!
//! - Collects logs from multiple agents across the mesh network
//! - Supports multiple log levels (trace, debug, info, warn, error)
//! - Allows log aggregation, filtering, and forwarding
//! - Provides compression and efficient serialization
//! - Integrates with existing mesh transport and state synchronization
//!
//! # Example
//! ```
//! use distributed_logging::{Logger, LogLevel, LogRecord};
//!
//! let logger = Logger::new("agent-1");
//! logger.info("System started", None);
//! ```

pub mod error;
pub mod log_record;
pub mod logger;
pub mod aggregator;
pub mod sink;
pub mod transport;
pub mod analysis;

#[cfg(feature = "compression")]
pub mod compression;

#[cfg(feature = "mesh")]
pub mod mesh_integration;

#[cfg(feature = "sync")]
pub mod sync_integration;

pub use error::*;
pub use log_record::*;
pub use logger::*;
pub use aggregator::*;
pub use sink::*;
pub use analysis::*;

/// Re‑export of common types for convenience.
pub mod prelude {
    pub use super::{
        Logger, LogLevel, LogRecord, LogSink, Aggregator, DistributedLogger,
        LogAnalyzer, LogAnalyzerConfig, LogStatistics, LogPattern, Anomaly, AnomalyRule,
    };
    #[cfg(feature = "compression")]
    pub use super::compression::*;
    #[cfg(feature = "mesh")]
    pub use super::mesh_integration::*;
    #[cfg(feature = "sync")]
    pub use super::sync_integration::*;
    pub use super::analysis::utils;
}

/// Initializes the distributed logging subsystem.
pub fn init() {
    tracing::info!("Distributed logging subsystem initialized");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init() {
        init();
    }
}