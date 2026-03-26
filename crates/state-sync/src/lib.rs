//! Conflict‑free replicated data types (CRDTs) for state synchronization.

pub mod crdt_map;
pub mod delta;
pub mod sync;

pub use crdt_map::CrdtMap;
pub use delta::Delta;
pub use sync::StateSync;