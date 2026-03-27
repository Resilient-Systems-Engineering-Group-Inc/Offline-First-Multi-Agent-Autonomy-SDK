//! Custom Resource Definitions (CRDs) for the operator.

use k8s_openapi::apimachinery::pkg::apis::meta::v1::Condition;
use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Specification of an autonomous agent.
#[derive(CustomResource, Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[kube(
    group = "autonomy.sdk",
    version = "v1alpha1",
    kind = "Agent",
    plural = "agents",
    namespaced,
    status = "AgentStatus",
    derive = "PartialEq",
    printcolumn = r#"{"name":"State", "type":"string", "jsonPath":".status.state"}"#,
    printcolumn = r#"{"name":"Age", "type":"date", "jsonPath":".metadata.creationTimestamp"}"#
)]
pub struct AgentSpec {
    /// Unique identifier of the agent within the mesh.
    pub agent_id: String,
    /// Capabilities of the agent (e.g., ["compute", "storage", "sensor"]).
    #[serde(default)]
    pub capabilities: Vec<String>,
    /// Resource limits (CPU, memory, etc.).
    #[serde(default)]
    pub resources: ResourceLimits,
    /// Configuration for the mesh transport.
    #[serde(default)]
    pub transport: TransportConfig,
}

/// Resource limits for an agent.
#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
pub struct ResourceLimits {
    /// CPU limit in millicores.
    #[serde(default)]
    pub cpu: String,
    /// Memory limit in MiB.
    #[serde(default)]
    pub memory: String,
    /// Storage limit in GiB.
    #[serde(default)]
    pub storage: String,
}

/// Transport configuration for mesh networking.
#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
pub struct TransportConfig {
    /// Network interface to bind to.
    #[serde(default = "default_interface")]
    pub interface: String,
    /// Listening port for mesh communication.
    #[serde(default = "default_port")]
    pub port: u16,
    /// Bootstrap peers for joining the mesh.
    #[serde(default)]
    pub bootstrap_peers: Vec<String>,
}

fn default_interface() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    5000
}

/// Status of an Agent resource.
#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
pub struct AgentStatus {
    /// Current state of the agent (Pending, Running, Error, Terminated).
    pub state: String,
    /// Detailed message about the state.
    #[serde(default)]
    pub message: String,
    /// Conditions representing the latest observations.
    #[serde(default)]
    pub conditions: Vec<Condition>,
    /// Timestamp when the agent was last updated.
    #[serde(default)]
    pub last_updated: Option<String>,
}

/// Specification of a task to be executed by agents.
#[derive(CustomResource, Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[kube(
    group = "autonomy.sdk",
    version = "v1alpha1",
    kind = "Task",
    plural = "tasks",
    namespaced,
    status = "TaskStatus",
    derive = "PartialEq",
    printcolumn = r#"{"name":"Status", "type":"string", "jsonPath":".status.phase"}"#,
    printcolumn = r#"{"name":"Assigned", "type":"string", "jsonPath":".status.assignedAgent"}"#,
    printcolumn = r#"{"name":"Age", "type":"date", "jsonPath":".metadata.creationTimestamp"}"#
)]
pub struct TaskSpec {
    /// Description of the task.
    pub description: String,
    /// Required capabilities for executing the task.
    #[serde(default)]
    pub required_capabilities: Vec<String>,
    /// Priority of the task (higher = more important).
    #[serde(default)]
    pub priority: i32,
    /// Deadline in seconds from creation.
    #[serde(default)]
    pub deadline_seconds: Option<u64>,
    /// Payload data (JSON string).
    #[serde(default)]
    pub payload: String,
}

/// Status of a Task resource.
#[derive(Clone, Debug, Default, Deserialize, Serialize, JsonSchema)]
pub struct TaskStatus {
    /// Current phase (Pending, Assigned, Running, Completed, Failed).
    pub phase: String,
    /// Agent ID to which the task is assigned (if any).
    #[serde(default)]
    pub assigned_agent: Option<String>,
    /// Result of the task (if completed).
    #[serde(default)]
    pub result: Option<String>,
    /// Conditions representing the latest observations.
    #[serde(default)]
    pub conditions: Vec<Condition>,
    /// Timestamp when the task was last updated.
    #[serde(default)]
    pub last_updated: Option<String>,
}