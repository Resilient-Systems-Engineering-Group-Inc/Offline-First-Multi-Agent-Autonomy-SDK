//! Agent lifecycle management for offline-first multi-agent systems.
//!
//! This crate provides comprehensive lifecycle management for agents, including:
//! - State machine with valid transitions
//! - Health monitoring and automatic recovery
//! - Graceful startup and shutdown
//! - Registry for managing multiple agents
//! - Maintenance mode and suspension
//!
//! # Example
//! ```
//! use agent_lifecycle::{LifecycleManager, LifecycleManagerConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = LifecycleManagerConfig::default();
//!     let manager = LifecycleManager::new(config)?;
//!
//!     // Initialize the agent
//!     manager.initialize().await?;
//!
//!     // Start the agent
//!     manager.start().await?;
//!
//!     // Check health
//!     let health = manager.check_health().await?;
//!     println!("Agent health: {}", health);
//!
//!     // Stop the agent gracefully
//!     manager.stop().await?;
//!
//!     Ok(())
//! }
//! ```

pub mod error;
pub mod health;
pub mod manager;
pub mod registry;
pub mod state;

// Re-export commonly used types
pub use error::{LifecycleError, Result};
pub use health::{HealthCheckConfig, HealthMonitor, HealthStatus};
pub use manager::{AgentInfo, LifecycleManager, LifecycleManagerConfig};
pub use registry::{AgentRegistry, RegisteredAgent, RegistryStatistics};
pub use state::{AgentState, StateMachine};

/// Current version of the agent lifecycle crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize tracing for the lifecycle system.
pub fn init_tracing() {
    use tracing_subscriber::fmt;
    
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    
    fmt::init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }
}