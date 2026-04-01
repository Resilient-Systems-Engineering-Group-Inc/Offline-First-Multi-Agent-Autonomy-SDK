//! Distributed caching with eviction policies and replication for offline‑first multi‑agent systems.
//!
//! This crate provides a flexible caching layer that can be used locally or distributed
//! across agents. It includes multiple eviction policies (LRU, LFU, FIFO, Random, Size‑aware),
//! TTL support, and replication over mesh transport.
//!
//! # Quick Start
//!
//! ```no_run
//! use distributed_cache::{LocalCache, LruPolicy, DistributedCache, DistributedCacheConfig};
//! use mesh_transport::MeshTransport;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Local cache
//!     let policy = LruPolicy::new(100);
//!     let local = LocalCache::new(policy, 100, 10_000_000);
//!
//!     // Distributed cache (requires transport)
//!     let config = DistributedCacheConfig::default();
//!     let transport = MeshTransport::in_memory().await?;
//!     let distributed = DistributedCache::new(local, transport, config, 0);
//!
//!     // Use cache
//!     distributed.put("key".to_string(), "value".to_string(), 60, 100).await?;
//!     let val = distributed.get(&"key".to_string()).await?;
//!     println!("Got: {:?}", val);
//!     Ok(())
//! }
//! ```

pub mod distributed;
pub mod error;
pub mod item;
pub mod local;
pub mod policy;

pub use distributed::{CacheBackend, DistributedCache, DistributedCacheConfig};
pub use error::*;
pub use item::*;
pub use local::LocalCache;
pub use policy::*;