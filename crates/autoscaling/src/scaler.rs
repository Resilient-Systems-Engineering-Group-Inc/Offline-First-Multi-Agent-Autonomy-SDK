//! Scalers that implement actual agent addition/removal.

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::error::AutoscalingError;

/// Trait for scaling agents.
#[async_trait::async_trait]
pub trait Scaler: Send + Sync {
    /// Add an agent.
    async fn add_agent(&self) -> Result<(), AutoscalingError>;
    /// Remove an agent.
    async fn remove_agent(&self) -> Result<(), AutoscalingError>;
    /// Get current agent count.
    async fn agent_count(&self) -> Result<usize, AutoscalingError>;
    /// Get average CPU usage across agents (0‑1).
    async fn average_cpu_usage(&self) -> Result<f64, AutoscalingError>;
    /// Get average memory usage across agents (0‑1).
    async fn average_memory_usage(&self) -> Result<f64, AutoscalingError>;
    /// Get number of pending tasks.
    async fn pending_tasks(&self) -> Result<usize, AutoscalingError>;
}

/// Dummy scaler for testing.
pub struct DummyScaler {
    agent_count: Arc<RwLock<usize>>,
}

impl DummyScaler {
    /// Create a new dummy scaler.
    pub fn new(initial_count: usize) -> Self {
        Self {
            agent_count: Arc::new(RwLock::new(initial_count)),
        }
    }
}

#[async_trait::async_trait]
impl Scaler for DummyScaler {
    async fn add_agent(&self) -> Result<(), AutoscalingError> {
        let mut count = self.agent_count.write().await;
        *count += 1;
        info!("DummyScaler: added agent, total {}", *count);
        Ok(())
    }

    async fn remove_agent(&self) -> Result<(), AutoscalingError> {
        let mut count = self.agent_count.write().await;
        if *count > 0 {
            *count -= 1;
            info!("DummyScaler: removed agent, total {}", *count);
        } else {
            warn!("DummyScaler: no agents to remove");
        }
        Ok(())
    }

    async fn agent_count(&self) -> Result<usize, AutoscalingError> {
        Ok(*self.agent_count.read().await)
    }

    async fn average_cpu_usage(&self) -> Result<f64, AutoscalingError> {
        // Simulate some CPU usage.
        Ok(0.5)
    }

    async fn average_memory_usage(&self) -> Result<f64, AutoscalingError> {
        // Simulate some memory usage.
        Ok(0.4)
    }

    async fn pending_tasks(&self) -> Result<usize, AutoscalingError> {
        // Simulate pending tasks.
        Ok(5)
    }
}

/// Agent scaler that integrates with the mesh transport.
pub struct AgentScaler {
    /// Underlying dummy scaler for now.
    dummy: DummyScaler,
    // In a real implementation, you would hold references to
    // mesh transport, agent manager, etc.
}

impl AgentScaler {
    /// Create a new agent scaler.
    pub fn new(initial_count: usize) -> Self {
        Self {
            dummy: DummyScaler::new(initial_count),
        }
    }
}

#[async_trait::async_trait]
impl Scaler for AgentScaler {
    async fn add_agent(&self) -> Result<(), AutoscalingError> {
        // In a real implementation, this would spawn a new agent process,
        // register it with the mesh, etc.
        self.dummy.add_agent().await
    }

    async fn remove_agent(&self) -> Result<(), AutoscalingError> {
        // In a real implementation, this would gracefully shut down an agent.
        self.dummy.remove_agent().await
    }

    async fn agent_count(&self) -> Result<usize, AutoscalingError> {
        self.dummy.agent_count().await
    }

    async fn average_cpu_usage(&self) -> Result<f64, AutoscalingError> {
        // Query resource monitor.
        self.dummy.average_cpu_usage().await
    }

    async fn average_memory_usage(&self) -> Result<f64, AutoscalingError> {
        self.dummy.average_memory_usage().await
    }

    async fn pending_tasks(&self) -> Result<usize, AutoscalingError> {
        self.dummy.pending_tasks().await
    }
}