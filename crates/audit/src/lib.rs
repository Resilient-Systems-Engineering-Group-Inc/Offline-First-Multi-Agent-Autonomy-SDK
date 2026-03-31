//! Audit logging and journaling for the Offline‑First Multi‑Agent Autonomy SDK.
//!
//! This crate provides structured audit logs, search capabilities, and
//! integration with external log aggregators (Elasticsearch, Loki).

#![deny(missing_docs, unsafe_code)]

pub mod backend;
pub mod error;
pub mod event;
pub mod logger;
pub mod search;

pub use error::Error;
pub use event::AuditEvent;
pub use logger::AuditLogger;