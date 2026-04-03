//! Metadata management for offline‑first multi‑agent systems.
//!
//! Provides storage, indexing, querying, and versioning of metadata
//! associated with agents, tasks, workflows, and other entities.

pub mod error;
pub mod model;
pub mod storage;
pub mod index;
pub mod query;
pub mod versioning;

pub use error::MetadataError;
pub use model::{Metadata, MetadataSchema, MetadataType};
pub use storage::MetadataStorage;
pub use index::MetadataIndex;
pub use query::MetadataQuery;
pub use versioning::MetadataVersioning;

/// Re‑export of common types.
pub mod prelude {
    pub use super::{
        MetadataError,
        Metadata,
        MetadataSchema,
        MetadataType,
        MetadataStorage,
        MetadataIndex,
        MetadataQuery,
        MetadataVersioning,
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}