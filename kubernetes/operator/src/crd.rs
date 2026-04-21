//! Custom Resource Definitions (CRDs) for the SDK.

use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Agent Custom Resource.
#[derive(CustomResource, Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[kube(
    group = "sdk.autonomy.io",
    version = "v1alpha1",
    kind = "Agent",
    namespaced,
    shortname = "agent"
)]
pub struct AgentSpec {
    pub name: String,
    pub capabilities: Vec<String>,
    pub resources: AgentResources,
    pub config: Option<AgentConfig>,
    pub image: String,
    pub replicas: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct AgentResources {
    pub cpu_limit: String,
    pub memory_limit: String,
    pub cpu_request: String,
    pub memory_request: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct AgentConfig {
    pub mesh_enabled: bool,
    pub tracing_enabled: bool,
    pub metrics_enabled: bool,
    pub log_level: String,
}

/// Task Custom Resource.
#[derive(CustomResource, Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[kube(
    group = "sdk.autonomy.io",
    version = "v1alpha1",
    kind = "Task",
    namespaced,
    shortname = "task"
)]
pub struct TaskSpec {
    pub description: String,
    pub priority: i32,
    pub required_capabilities: Vec<String>,
    pub dependencies: Vec<String>,
    pub parameters: serde_json::Value,
    pub deadline: Option<String>,
}

/// Workflow Custom Resource.
#[derive(CustomResource, Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[kube(
    group = "sdk.autonomy.io",
    version = "v1alpha1",
    kind = "Workflow",
    namespaced,
    shortname = "wf"
)]
pub struct WorkflowSpec {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub tasks: Vec<WorkflowTask>,
    pub triggers: Vec<WorkflowTrigger>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct WorkflowTask {
    pub id: String,
    pub name: String,
    pub action: String,
    pub parameters: serde_json::Value,
    pub retries: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct WorkflowTrigger {
    pub event: String,
    pub conditions: serde_json::Value,
}

/// Cluster Configuration Custom Resource.
#[derive(CustomResource, Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[kube(
    group = "sdk.autonomy.io",
    version = "v1alpha1",
    kind = "ClusterConfig",
    namespaced,
    plural = "clusterconfigs"
)]
pub struct ClusterConfigSpec {
    pub mesh_config: MeshConfig,
    pub security_config: SecurityConfig,
    pub monitoring_config: MonitoringConfig,
    pub edge_config: Option<EdgeConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct MeshConfig {
    pub protocol: String,
    pub discovery_interval_ms: u64,
    pub heartbeat_interval_ms: u64,
    pub max_peers: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct SecurityConfig {
    pub enable_pq_crypto: bool,
    pub jwt_expiry_secs: u64,
    pub rbac_enabled: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct MonitoringConfig {
    pub prometheus_enabled: bool,
    pub jaeger_enabled: bool,
    pub metrics_port: u16,
    pub tracing_sample_rate: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct EdgeConfig {
    pub enable_edge_computing: bool,
    pub sync_interval_ms: u64,
    pub max_edge_tasks: usize,
}
