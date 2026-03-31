//! Autoscaling controller.

use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{interval, Duration};
use tracing::{info, warn, error};

use crate::error::AutoscalingError;
use crate::policy::{ScalingPolicy, ScalingMetrics, ScalingDecision};
use crate::scaler::{Scaler, AgentScaler};

/// Configuration for the autoscaling controller.
#[derive(Debug, Clone)]
pub struct AutoscalingConfig {
    /// Evaluation interval in seconds.
    pub evaluation_interval_secs: u64,
    /// Minimum agents allowed.
    pub min_agents: usize,
    /// Maximum agents allowed.
    pub max_agents: usize,
    /// Cooldown period after scaling (seconds).
    pub cooldown_secs: u64,
    /// Whether to enable predictive scaling.
    pub predictive: bool,
}

impl Default for AutoscalingConfig {
    fn default() -> Self {
        Self {
            evaluation_interval_secs: 30,
            min_agents: 1,
            max_agents: 20,
            cooldown_secs: 60,
            predictive: false,
        }
    }
}

/// Events emitted by the controller.
#[derive(Debug, Clone)]
pub enum AutoscalingEvent {
    /// Scaling decision made.
    ScalingDecision(ScalingDecision),
    /// Metrics collected.
    MetricsCollected(ScalingMetrics),
    /// Agent added.
    AgentAdded(usize),
    /// Agent removed.
    AgentRemoved(usize),
    /// Error occurred.
    Error(String),
}

/// Autoscaling controller.
pub struct AutoscalingController {
    config: AutoscalingConfig,
    policy: Box<dyn ScalingPolicy>,
    scaler: Arc<dyn Scaler>,
    event_tx: mpsc::UnboundedSender<AutoscalingEvent>,
    /// Last scaling timestamp.
    last_scaling: Arc<RwLock<Option<u64>>>,
    /// Current agent count.
    agent_count: Arc<RwLock<usize>>,
}

impl AutoscalingController {
    /// Create a new autoscaling controller.
    pub fn new(
        config: AutoscalingConfig,
        policy: Box<dyn ScalingPolicy>,
        scaler: Arc<dyn Scaler>,
    ) -> (Self, mpsc::UnboundedReceiver<AutoscalingEvent>) {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let controller = Self {
            config,
            policy,
            scaler,
            event_tx,
            last_scaling: Arc::new(RwLock::new(None)),
            agent_count: Arc::new(RwLock::new(0)),
        };
        (controller, event_rx)
    }

    /// Start the controller loop.
    pub async fn start(&self) -> Result<(), AutoscalingError> {
        info!("Starting autoscaling controller");
        let mut interval = interval(Duration::from_secs(self.config.evaluation_interval_secs));

        loop {
            interval.tick().await;

            // Collect metrics.
            let metrics = match self.collect_metrics().await {
                Ok(m) => m,
                Err(e) => {
                    error!("Failed to collect metrics: {}", e);
                    continue;
                }
            };

            self.event_tx
                .send(AutoscalingEvent::MetricsCollected(metrics.clone()))
                .map_err(|e| AutoscalingError::Communication(e.to_string()))?;

            // Check cooldown.
            if self.in_cooldown().await {
                info!("In cooldown period, skipping evaluation");
                continue;
            }

            // Evaluate policy.
            let decision = self.policy.evaluate(&metrics).await;
            self.event_tx
                .send(AutoscalingEvent::ScalingDecision(decision))
                .map_err(|e| AutoscalingError::Communication(e.to_string()))?;

            // Execute scaling.
            if let Err(e) = self.execute_scaling(decision).await {
                error!("Scaling execution failed: {}", e);
                self.event_tx
                    .send(AutoscalingEvent::Error(e.to_string()))
                    .unwrap();
            }
        }
    }

    /// Collect metrics from the scaler.
    async fn collect_metrics(&self) -> Result<ScalingMetrics, AutoscalingError> {
        let agent_count = self.scaler.agent_count().await?;
        let avg_cpu = self.scaler.average_cpu_usage().await.unwrap_or(0.0);
        let avg_memory = self.scaler.average_memory_usage().await.unwrap_or(0.0);
        let pending_tasks = self.scaler.pending_tasks().await.unwrap_or(0);

        let metrics = ScalingMetrics {
            agent_count,
            avg_cpu_usage: avg_cpu,
            avg_memory_usage: avg_memory,
            pending_tasks,
            avg_task_latency_ms: 0.0, // placeholder
            network_bandwidth: 0.0,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        Ok(metrics)
    }

    /// Check if we are in cooldown period.
    async fn in_cooldown(&self) -> bool {
        let last = self.last_scaling.read().await;
        match *last {
            Some(ts) => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                now - ts < self.config.cooldown_secs
            }
            None => false,
        }
    }

    /// Execute scaling decision.
    async fn execute_scaling(&self, decision: ScalingDecision) -> Result<(), AutoscalingError> {
        match decision {
            ScalingDecision::ScaleUp(count) => {
                info!("Scaling up by {} agents", count);
                for _ in 0..count {
                    self.scaler.add_agent().await?;
                }
                self.event_tx
                    .send(AutoscalingEvent::AgentAdded(count))
                    .unwrap();
            }
            ScalingDecision::ScaleDown(count) => {
                info!("Scaling down by {} agents", count);
                for _ in 0..count {
                    self.scaler.remove_agent().await?;
                }
                self.event_tx
                    .send(AutoscalingEvent::AgentRemoved(count))
                    .unwrap();
            }
            ScalingDecision::NoChange => {
                // Nothing to do.
            }
        }

        // Update last scaling timestamp.
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        *self.last_scaling.write().await = Some(now);

        Ok(())
    }

    /// Get current agent count.
    pub async fn agent_count(&self) -> usize {
        *self.agent_count.read().await
    }
}