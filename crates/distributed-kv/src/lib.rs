//! Distributed key‑value store based on CRDTs.
//!
//! This crate provides a replicated, eventually consistent key‑value store
//! that uses CRDTs for conflict‑free merging and mesh‑transport for peer‑to‑peer
//! synchronization.

pub mod error;
pub mod store;
pub mod replication;
pub mod query;
pub mod persistence;

pub use error::{Error, Result};
pub use store::DistributedKV;
pub use replication::ReplicationManager;
pub use query::{Query, QueryResult, Index};
pub use persistence::{PersistentStore, Snapshot};

/// Pre‑import of commonly used types.
pub mod prelude {
    pub use crate::DistributedKV;
    pub use crate::Error;
    pub use crate::Query;
    pub use crate::ReplicationManager;
}