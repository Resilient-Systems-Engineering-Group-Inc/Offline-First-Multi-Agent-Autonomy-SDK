//! Scheduler that executes nodes based on dependency graph.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{info, warn, error};

use crate::error::DependencyError;
use crate::graph::{DependencyGraph, NodeId};
use crate::event::DependencyEvent;

/// Status of a node in the scheduler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeStatus {
    /// Node is pending (not yet started).
    Pending,
    /// Node is currently running.
    Running,
    /// Node has completed successfully.
    Completed,
    /// Node has failed.
    Failed,
    /// Node is blocked by dependencies.
    Blocked,
}

/// Result of executing a node.
pub type NodeResult = Result<(), String>;

/// Trait for executing a node.
#[async_trait::async_trait]
pub trait NodeExecutor: Send + Sync {
    /// Execute a node given its ID and metadata.
    async fn execute(&self, node_id: NodeId, metadata: serde_json::Value) -> NodeResult;
}

/// Default executor that logs and returns success.
pub struct DefaultExecutor;

#[async_trait::async_trait]
impl NodeExecutor for DefaultExecutor {
    async fn execute(&self, node_id: NodeId, _metadata: serde_json::Value) -> NodeResult {
        info!("DefaultExecutor executing node {}", node_id);
        Ok(())
    }
}

/// Scheduler configuration.
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// Maximum number of concurrent nodes.
    pub max_concurrent: usize,
    /// Retry count for failed nodes.
    pub retry_count: u32,
    /// Timeout per node in seconds.
    pub timeout_secs: u64,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 4,
            retry_count: 3,
            timeout_secs: 30,
        }
    }
}

/// Dependency scheduler.
pub struct DependencyScheduler {
    /// The dependency graph.
    graph: Arc<RwLock<DependencyGraph>>,
    /// Node executor.
    executor: Arc<dyn NodeExecutor>,
    /// Configuration.
    config: SchedulerConfig,
    /// Current status of each node.
    status: Arc<RwLock<HashMap<NodeId, NodeStatus>>>,
    /// Event sender for notifications.
    event_tx: mpsc::UnboundedSender<DependencyEvent>,
}

impl DependencyScheduler {
    /// Create a new scheduler.
    pub fn new(
        graph: DependencyGraph,
        executor: Arc<dyn NodeExecutor>,
        config: SchedulerConfig,
    ) -> (Self, mpsc::UnboundedReceiver<DependencyEvent>) {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let status = Arc::new(RwLock::new(HashMap::new()));
        let graph = Arc::new(RwLock::new(graph));

        // Initialize all nodes as pending.
        let mut status_map = HashMap::new();
        for node in graph.read().unwrap().nodes() {
            status_map.insert(node.id, NodeStatus::Pending);
        }
        *status.write().unwrap() = status_map;

        let scheduler = Self {
            graph,
            executor,
            config,
            status,
            event_tx,
        };
        (scheduler, event_rx)
    }

    /// Start the scheduler.
    pub async fn start(&self) -> Result<(), DependencyError> {
        info!("Starting dependency scheduler");
        self.event_tx
            .send(DependencyEvent::SchedulerStarted)
            .map_err(|e| DependencyError::Other(e.to_string()))?;

        let topological_order = self.graph.read().unwrap().topological_sort()?;
        info!("Topological order: {:?}", topological_order);

        let mut running = HashSet::new();
        let mut completed = HashSet::new();
        let mut failed = HashSet::new();

        for node_id in topological_order {
            // Wait until we have capacity.
            while running.len() >= self.config.max_concurrent {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }

            // Check dependencies.
            let deps = self.graph.read().unwrap().predecessors(node_id);
            let all_deps_completed = deps.iter().all(|dep| completed.contains(dep));
            if !all_deps_completed {
                self.set_status(node_id, NodeStatus::Blocked).await;
                continue;
            }

            // Execute node.
            running.insert(node_id);
            self.set_status(node_id, NodeStatus::Running).await;
            self.event_tx
                .send(DependencyEvent::NodeStarted(node_id))
                .unwrap();

            let executor = self.executor.clone();
            let graph = self.graph.clone();
            let event_tx = self.event_tx.clone();
            let node = self
                .graph
                .read()
                .unwrap()
                .node(node_id)
                .cloned()
                .ok_or_else(|| DependencyError::node_not_found(node_id.to_string()))?;

            let handle = tokio::spawn(async move {
                let result = executor.execute(node_id, node.metadata).await;
                (node_id, result)
            });

            let (completed_node_id, result) = match tokio::time::timeout(
                tokio::time::Duration::from_secs(self.config.timeout_secs),
                handle,
            )
            .await
            {
                Ok(Ok((id, res))) => (id, res),
                Ok(Err(join_err)) => (node_id, Err(join_err.to_string())),
                Err(_) => (node_id, Err("timeout".to_string())),
            };

            running.remove(&completed_node_id);
            match result {
                Ok(()) => {
                    completed.insert(completed_node_id);
                    self.set_status(completed_node_id, NodeStatus::Completed)
                        .await;
                    event_tx
                        .send(DependencyEvent::NodeCompleted(completed_node_id))
                        .unwrap();
                }
                Err(err) => {
                    failed.insert(completed_node_id);
                    self.set_status(completed_node_id, NodeStatus::Failed).await;
                    event_tx
                        .send(DependencyEvent::NodeFailed(completed_node_id, err))
                        .unwrap();
                }
            }
        }

        // Wait for all running nodes to finish.
        while !running.is_empty() {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        info!("Scheduler finished");
        self.event_tx
            .send(DependencyEvent::SchedulerFinished)
            .unwrap();

        Ok(())
    }

    /// Set status of a node.
    async fn set_status(&self, node_id: NodeId, status: NodeStatus) {
        self.status.write().unwrap().insert(node_id, status);
    }

    /// Get status of a node.
    pub async fn node_status(&self, node_id: NodeId) -> Option<NodeStatus> {
        self.status.read().unwrap().get(&node_id).cloned()
    }

    /// Get overall progress (completed / total).
    pub async fn progress(&self) -> (usize, usize) {
        let status = self.status.read().unwrap();
        let total = status.len();
        let completed = status
            .values()
            .filter(|&s| *s == NodeStatus::Completed)
            .count();
        (completed, total)
    }
}