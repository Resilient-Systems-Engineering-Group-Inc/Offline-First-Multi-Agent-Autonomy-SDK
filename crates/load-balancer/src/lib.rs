//! Load balancing for multi‑agent systems.
//!
//! This crate provides intelligent load balancing algorithms to distribute
//! tasks and workloads across a swarm of agents, ensuring optimal resource
//! utilization, fairness, and performance.
//!
//! ## Features
//!
//! - **Multiple load balancing strategies**: Round‑robin, least‑loaded, weighted,
//!   consistent hashing, adaptive, and predictive.
//! - **Real‑time metrics integration**: Uses resource‑monitor for CPU, memory,
//!   network, and custom metrics.
//! - **Adaptive algorithms**: Self‑adjusting based on observed performance.
//! - **Predictive load balancing**: Forecast future loads using time‑series analysis.
//! - **Integration with agent‑core**: Seamless integration with the agent lifecycle.
//! - **Distributed coordination**: Works across multiple nodes without central coordinator.
//!
//! ## Quick Start
//!
//! ```rust
//! use load_balancer::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a load balancer with least‑loaded strategy
//!     let mut balancer = LoadBalancer::new(
//!         LoadBalancingStrategy::LeastLoaded,
//!         LoadBalancerConfig::default(),
//!     );
//!
//!     // Register agents with their current loads
//!     balancer.register_agent("agent-1", AgentLoad::new(0.3)).await?;
//!     balancer.register_agent("agent-2", AgentLoad::new(0.1)).await?;
//!     balancer.register_agent("agent-3", AgentLoad::new(0.5)).await?;
//!
//!     // Select the best agent for a new task
//!     let selected = balancer.select_agent().await?;
//!     println!("Selected agent: {}", selected);
//!
//!     // Update agent load after task assignment
//!     balancer.update_agent_load(&selected, 0.4).await?;
//!
//!     Ok(())
//! }
//! ```

#![deny(missing_docs, unsafe_code)]

pub mod error;
pub mod strategy;
pub mod metrics;
pub mod adaptive;
pub mod predictive;
pub mod coordinator;

pub use error::LoadBalancingError;
pub use strategy::{
    LoadBalancingStrategy,
    RoundRobinStrategy,
    LeastLoadedStrategy,
    WeightedRoundRobinStrategy,
    ConsistentHashingStrategy,
    LoadBalancer,
    LoadBalancerConfig,
};
pub use metrics::{AgentLoad, LoadMetrics, LoadMetricsCollector};
pub use adaptive::{AdaptiveLoadBalancer, AdaptiveConfig};
pub use predictive::{PredictiveLoadBalancer, PredictiveConfig};
pub use coordinator::{DistributedLoadBalancer, DistributedCoordinator};

/// Prelude for convenient imports.
pub mod prelude {
    pub use super::{
        LoadBalancingError,
        LoadBalancingStrategy,
        LoadBalancer,
        LoadBalancerConfig,
        AgentLoad,
        LoadMetrics,
        AdaptiveLoadBalancer,
        PredictiveLoadBalancer,
        DistributedLoadBalancer,
    };
}