//! Edge computing support for the SDK.
//!
//! Provides:
//! - Edge device management
//! - Resource-aware task scheduling
//! - Edge-cloud synchronization
//! - Offline-first edge computing

pub mod edge_device;
pub mod scheduler;
pub mod sync;
pub mod resources;

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

pub use edge_device::*;
pub use scheduler::*;
pub use sync::*;
pub use resources::*;

/// Edge node configuration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EdgeConfig {
    pub node_id: String,
    pub edge_type: EdgeType,
    pub location: Option<String>,
    pub capabilities: Vec<String>,
    pub resource_limits: ResourceLimits,
    pub connectivity: ConnectivityConfig,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum EdgeType {
    Gateway,
    Edge,
    Fog,
    Cloud,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResourceLimits {
    pub max_cpu_percent: f64,
    pub max_memory_mb: u64,
    pub max_storage_mb: u64,
    pub max_network_bandwidth_mbps: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConnectivityConfig {
    pub is_online: bool,
    pub connection_quality: f64, // 0.0 to 1.0
    pub latency_ms: f64,
    pub bandwidth_mbps: f64,
}

impl Default for EdgeConfig {
    fn default() -> Self {
        Self {
            node_id: uuid::Uuid::new_v4().to_string(),
            edge_type: EdgeType::Edge,
            location: None,
            capabilities: vec![],
            resource_limits: ResourceLimits::default(),
            connectivity: ConnectivityConfig::default(),
            metadata: HashMap::new(),
        }
    }
}

/// Edge manager for coordinating edge devices.
pub struct EdgeManager {
    edges: Arc<RwLock<HashMap<String, EdgeDevice>>>,
    config: EdgeConfig,
}

impl EdgeManager {
    /// Create new edge manager.
    pub fn new(config: EdgeConfig) -> Self {
        Self {
            edges: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Register edge device.
    pub async fn register_edge(&self, edge: EdgeDevice) {
        let mut edges = self.edges.write().await;
        edges.insert(edge.id.clone(), edge);
        info!("Edge device registered: {}", edge.id);
    }

    /// Unregister edge device.
    pub async fn unregister_edge(&self, edge_id: &str) {
        let mut edges = self.edges.write().await;
        edges.remove(edge_id);
        info!("Edge device unregistered: {}", edge_id);
    }

    /// Get all registered edges.
    pub async fn list_edges(&self) -> Vec<EdgeDevice> {
        let edges = self.edges.read().await;
        edges.values().cloned().collect()
    }

    /// Get edge by ID.
    pub async fn get_edge(&self, edge_id: &str) -> Option<EdgeDevice> {
        let edges = self.edges.read().await;
        edges.get(edge_id).cloned()
    }

    /// Find edges with specific capabilities.
    pub async fn find_edges_by_capability(&self, capability: &str) -> Vec<EdgeDevice> {
        let edges = self.edges.read().await;
        edges
            .values()
            .filter(|edge| edge.capabilities.contains(&capability.to_string()))
            .cloned()
            .collect()
    }

    /// Find available edges (not at capacity).
    pub async fn find_available_edges(&self) -> Vec<EdgeDevice> {
        let edges = self.edges.read().await;
        edges
            .values()
            .filter(|edge| edge.is_available())
            .cloned()
            .collect()
    }

    /// Schedule task to edge.
    pub async fn schedule_task(&self, task: &EdgeTask) -> Result<String> {
        let available_edges = self.find_available_edges().await;
        
        if available_edges.is_empty() {
            return Err(anyhow::anyhow!("No available edges for task"));
        }

        // Select best edge based on resources and capabilities
        let selected_edge = self.select_best_edge(&available_edges, task).await;

        // Assign task to edge
        let mut edges = self.edges.write().await;
        if let Some(edge) = edges.get_mut(&selected_edge.id) {
            edge.assign_task(task.clone());
            info!("Task {} assigned to edge {}", task.id, edge.id);
            Ok(task.id.clone())
        } else {
            Err(anyhow::anyhow!("Edge not found"))
        }
    }

    /// Select best edge for task.
    async fn select_best_edge(&self, candidates: &[EdgeDevice], task: &EdgeTask) -> EdgeDevice {
        candidates
            .iter()
            .min_by(|a, b| {
                let score_a = self.calculate_edge_score(a, task);
                let score_b = self.calculate_edge_score(b, task);
                score_a.partial_cmp(&score_b).unwrap()
            })
            .cloned()
            .unwrap()
    }

    /// Calculate edge score (lower is better).
    fn calculate_edge_score(&self, edge: &EdgeDevice, task: &EdgeTask) -> f64 {
        let resource_score = edge.available_resources.score();
        let latency_score = edge.connectivity.latency_ms / 100.0;
        let capability_score = if edge.has_capability(&task.required_capability) {
            0.0
        } else {
            1000.0
        };

        resource_score + latency_score + capability_score
    }

    /// Sync edge state with cloud.
    pub async fn sync_with_cloud(&self) -> Result<()> {
        let edges = self.edges.read().await;
        
        for edge in edges.values() {
            if edge.is_online() {
                edge.sync_state().await?;
            }
        }

        Ok(())
    }

    /// Get edge statistics.
    pub async fn get_stats(&self) -> EdgeStats {
        let edges = self.edges.read().await;
        
        let total = edges.len();
        let online = edges.values().filter(|e| e.is_online()).count();
        let available = edges.values().filter(|e| e.is_available()).count();
        let total_tasks: usize = edges.values().map(|e| e.active_tasks.len()).sum();

        EdgeStats {
            total_edges: total as i64,
            online_edges: online as i64,
            available_edges: available as i64,
            total_active_tasks: total_tasks as i64,
            avg_cpu_usage: self.calculate_avg_cpu(&edges),
            avg_memory_usage: self.calculate_avg_memory(&edges),
        }
    }

    fn calculate_avg_cpu(&self, edges: &HashMap<String, EdgeDevice>) -> f64 {
        let total: f64 = edges.values().map(|e| e.resources.cpu_percent).sum();
        let count = edges.len();
        if count > 0 {
            total / count as f64
        } else {
            0.0
        }
    }

    fn calculate_avg_memory(&self, edges: &HashMap<String, EdgeDevice>) -> f64 {
        let total: f64 = edges.values().map(|e| e.resources.memory_percent).sum();
        let count = edges.len();
        if count > 0 {
            total / count as f64
        } else {
            0.0
        }
    }
}

/// Edge statistics.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EdgeStats {
    pub total_edges: i64,
    pub online_edges: i64,
    pub available_edges: i64,
    pub total_active_tasks: i64,
    pub avg_cpu_usage: f64,
    pub avg_memory_usage: f64,
}

/// Edge task.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EdgeTask {
    pub id: String,
    pub description: String,
    pub required_capability: String,
    pub resource_requirements: ResourceRequirements,
    pub priority: u8,
    pub deadline: Option<u64>,
    pub metadata: HashMap<String, String>,
}

impl EdgeTask {
    pub fn new(id: &str, description: &str, capability: &str) -> Self {
        Self {
            id: id.to_string(),
            description: description.to_string(),
            required_capability: capability.to_string(),
            resource_requirements: ResourceRequirements::default(),
            priority: 100,
            deadline: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_resources(mut self, resources: ResourceRequirements) -> Self {
        self.resource_requirements = resources;
        self
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_deadline(mut self, deadline: u64) -> Self {
        self.deadline = Some(deadline);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_edge_manager() {
        let config = EdgeConfig::default();
        let manager = EdgeManager::new(config);

        // Create edge device
        let edge = EdgeDevice::new("edge-1");
        manager.register_edge(edge).await;

        // List edges
        let edges = manager.list_edges().await;
        assert_eq!(edges.len(), 1);

        // Get stats
        let stats = manager.get_stats().await;
        assert_eq!(stats.total_edges, 1);
    }

    #[tokio::test]
    async fn test_task_scheduling() {
        let config = EdgeConfig::default();
        let manager = EdgeManager::new(config);

        // Create edge with capability
        let mut edge = EdgeDevice::new("edge-1");
        edge.capabilities = vec!["lidar".to_string()];
        manager.register_edge(edge).await;

        // Create task
        let task = EdgeTask::new("task-1", "Scan area", "lidar");

        // Schedule task
        let result = manager.schedule_task(&task).await;
        assert!(result.is_ok());
    }
}
