//! Data versioning with snapshots and rollback for offline‑first multi‑agent systems.
//!
//! This crate provides version tracking, snapshot creation, and rollback capabilities
//! for distributed state (e.g., CRDT maps). It integrates with the SDK's state‑sync
//! and can be used to maintain a history of changes, enabling audit trails and
//! disaster recovery.
//!
//! # Quick Start
//!
//! ```no_run
//! use data_versioning::{VersionManager, InMemoryStorage, VersionedCrdtMap};
//! use state_sync::crdt_map::CrdtMap;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let storage = InMemoryStorage::new();
//!     let manager = VersionManager::new(storage);
//!     let mut versioned_map = VersionedCrdtMap::new(manager);
//!
//!     // Modify the map
//!     versioned_map.map.set("key", "value", 0);
//!
//!     // Create a snapshot
//!     let v1 = versioned_map.snapshot("First version".to_string()).await?;
//!     println!("Snapshot created: {}", v1.to_string());
//!
//!     // Restore later
//!     versioned_map.restore(&v1).await?;
//!     Ok(())
//! }
//! ```

pub mod error;
pub mod integration;
pub mod manager;
pub mod version;

pub use error::*;
pub use integration::*;
pub use manager::*;
pub use version::*;