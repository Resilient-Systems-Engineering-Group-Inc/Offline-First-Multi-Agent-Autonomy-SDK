//! Agent Package Management System
//!
//! This crate provides a comprehensive package management system for agent capabilities,
//! libraries, plugins, and tools in offline‑first multi‑agent systems.
//!
//! ## Overview
//!
//! The package manager supports:
//! - **Multiple package types**: Agents, capabilities, libraries, plugins, tools
//! - **Dependency resolution**: Semantic versioning with conflict detection
//! - **Multiple registries**: Local, remote, and distributed registries
//! - **Caching**: Local cache for offline operation
//! - **Verification**: Checksum validation and signature verification
//! - **Installation**: Archive extraction and file placement
//! - **Integration**: Works with mesh transport, distributed KV, and event bus
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                    Package Manager                       │
//! ├─────────────┬─────────────┬─────────────┬───────────────┤
//! │  Repository │  Resolver   │  Installer  │     Cache     │
//! │  (local/    │ (dependency │  (extract/  │  (local       │
//! │   remote)   │  graph)     │   place)    │   storage)    │
//! └─────────────┴─────────────┴─────────────┴───────────────┘
//! ```
//!
//! ## Usage
//!
//! ```rust
//! use agent_package_management::{PackageManager, RegistryConfig};
//! use std::path::PathBuf;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a package manager with local cache
//!     let cache_dir = PathBuf::from("/tmp/agent-packages");
//!     let mut manager = PackageManager::new(cache_dir)?;
//!
//!     // Add a remote registry
//!     let registry = RegistryConfig {
//!         url: "https://packages.agent-sdk.org".to_string(),
//!         auth_token: None,
//!         timeout_secs: 30,
//!         verify_ssl: true,
//!     };
//!     manager.add_registry(registry).await?;
//!
//!     // Install a package
//!     let result = manager.install("example-agent", "^1.0.0").await?;
//!     println!("Installed to: {}", result.install_path);
//!
//!     Ok(())
//! }
//! ```

pub mod cache;
pub mod error;
pub mod installer;
pub mod manager;
pub mod repository;
pub mod resolver;
pub mod types;

// Re-exports
pub use crate::error::{PackageError, Result};
pub use crate::manager::PackageManager;
pub use crate::repository::{LocalRepository, RemoteRepository, Repository};
pub use crate::resolver::DependencyResolver;
pub use crate::types::*;

#[cfg(feature = "distributed")]
pub mod distributed;

#[cfg(feature = "events")]
pub mod events;

#[cfg(test)]
mod tests;