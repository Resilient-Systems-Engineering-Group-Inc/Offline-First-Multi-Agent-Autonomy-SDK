//! Container runtime integration for agent deployment.
//!
//! This crate provides integration with container runtimes (Docker, containerd)
//! for deploying agents as containers in offline‑first multi‑agent systems.
//!
//! ## Features
//!
//! - **Docker integration**: Build, run, and manage containers via Docker Engine API
//! - **Containerd integration**: Direct integration with containerd runtime
//! - **Image management**: Pull, push, and manage container images
//! - **Container lifecycle**: Start, stop, restart, and monitor containers
//! - **Resource constraints**: CPU, memory, and network limits
//! - **Networking**: Container networking and port mapping
//! - **Volume management**: Persistent storage for containers
//! - **Health checks**: Container health monitoring
//! - **Log collection**: Container log aggregation
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │               Container Runtime Manager                  │
//! ├─────────────┬─────────────┬─────────────┬───────────────┤
//! │   Docker    │  Containerd │    Image    │   Container   │
//! │   Adapter   │   Adapter   │   Manager   │   Manager     │
//! └─────────────┴─────────────┴─────────────┴───────────────┘
//! ```
//!
//! ## Usage
//!
//! ```rust
//! use container_integration::{ContainerRuntime, DockerConfig, ContainerSpec};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create Docker runtime
//!     let config = DockerConfig::default();
//!     let runtime = ContainerRuntime::docker(config).await?;
//!
//!     // Create container specification
//!     let spec = ContainerSpec {
//!         name: "my-agent".to_string(),
//!         image: "my-agent:latest".to_string(),
//!         command: Some(vec!["/app/agent".to_string()]),
//!         env: vec!["AGENT_ID=1".to_string()],
//!         ..Default::default()
//!     };
//!
//!     // Run container
//!     let container_id = runtime.create_container(&spec).await?;
//!     runtime.start_container(&container_id).await?;
//!
//!     Ok(())
//! }
//! ```

pub mod docker;
pub mod containerd;
pub mod error;
pub mod image;
pub mod container;
pub mod manager;
pub mod types;

// Re-exports
pub use crate::error::{ContainerError, Result};
pub use crate::manager::ContainerRuntime;
pub use crate::types::*;

#[cfg(test)]
mod tests;