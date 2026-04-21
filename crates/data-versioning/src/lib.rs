//! Data versioning with snapshots, rollback, and lineage tracking for offline‑first multi‑agent systems.
//!
//! This crate provides:
//! - Version tracking and snapshot creation for distributed state (e.g., CRDT maps)
//! - Rollback capabilities for disaster recovery
//! - Comprehensive data lineage tracking with provenance information
//! - Integration with the SDK's state‑sync system
//! - Audit trails and compliance tracking
//!
//! # Quick Start
//!
//! ## Basic Versioning
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
//!
//! ## Lineage Tracking
//! ```no_run
//! use data_versioning::lineage::{LineageTracker, DataOrigin, DataReference, LineageBuilder};
//! use data_versioning::version::Version;
//! use common::types::AgentId;
//!
//! fn example() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut tracker = LineageTracker::new();
//!
//!     let data_ref = DataReference {
//!         data_id: "sensor_data".to_string(),
//!         version: Version::new(1, AgentId::from_u128(123)),
//!         location: None,
//!     };
//!
//!     let origin = DataOrigin::Sensor {
//!         sensor_id: "temp_sensor_1".to_string(),
//!         timestamp: 1000,
//!         location: Some("room_a".to_string()),
//!     };
//!
//!     let lineage = LineageBuilder::new(data_ref.clone(), origin)
//!         .add_quality_metric("accuracy".to_string(), 0.95)
//!         .build();
//!
//!     tracker.register_lineage(lineage)?;
//!
//!     // Query provenance
//!     let provenance = tracker.get_provenance(&data_ref);
//!     println!("Data origin: {:?}", provenance.unwrap().origin);
//!
//!     Ok(())
//! }
//! ```

pub mod error;
pub mod integration;
pub mod lineage;
pub mod manager;
pub mod version;

pub use error::*;
pub use integration::*;
pub use lineage::*;
pub use manager::*;
pub use version::*;