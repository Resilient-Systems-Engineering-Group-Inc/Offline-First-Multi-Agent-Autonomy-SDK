//! Debugging tools for distributed offline‑first multi‑agent systems.
//!
//! This crate provides a distributed debugger with sessions, commands, logging,
//! metrics collection, and a web‑based UI for inspecting and controlling agents.
//!
//! # Quick Start
//!
//! ```no_run
//! use distributed_debugger::{DebuggerManager, DebugCommand};
//! use mesh_transport::MeshTransport;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let transport = MeshTransport::in_memory().await?;
//!     let debugger = DebuggerManager::new(transport);
//!     let session_id = debugger.start_session(vec![1, 2, 3]);
//!     let response = debugger.send_command(session_id, 1, DebugCommand::Pause).await?;
//!     println!("Response: {:?}", response);
//!     Ok(())
//! }
//! ```

pub mod error;
pub mod manager;
pub mod session;

pub use error::*;
pub use manager::*;
pub use session::*;