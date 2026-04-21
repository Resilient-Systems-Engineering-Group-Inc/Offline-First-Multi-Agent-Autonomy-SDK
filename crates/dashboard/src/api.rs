//! REST API for dashboard.
//!
//! Provides endpoints for:
//! - Agent status and management
//! - Task monitoring
//! - Workflow control
//! - Metrics retrieval
//! - System health checks

use warp::Filter;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};

use crate::metrics::MetricsCollector;
use crate::websocket::WebSocketManager;

/// API state shared across handlers.
#[derive(Clone)]
pub struct ApiState {
    pub metrics: Arc<RwLock<MetricsCollector>>,
    pub websocket: Arc<RwLock<WebSocketManager>>,
    pub agents: Arc<RwLock<HashMap<String, AgentInfo>>>,
    pub tasks: Arc<RwLock<HashMap<String, TaskInfo>>>,
    pub workflows: Arc<RwLock<HashMap<String, WorkflowInfo>>>,
}

impl ApiState {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(MetricsCollector::new())),
            websocket: Arc::new(RwLock::new(WebSocketManager::new())),
            agents: Arc::new(RwLock::new(HashMap::new())),
            tasks: Arc::new(RwLock::new(HashMap::new())),
            workflows: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

// ============ Request/Response Types ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: String,
    pub status: String,
    pub capabilities: Vec<String>,
    pub resources: ResourceStats,
    pub connected_peers: usize,
    pub active_tasks: Vec<String>,
    pub last_heartbeat: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceStats {
    pub cpu_percent: f64,
    pub memory_percent: f64,
    pub disk_percent: f64,
    pub battery_level: Option<f64>,
    pub network_latency_ms: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
    pub id: String,
    pub description: String,
    pub status: String,
    pub priority: u8,
    pub assigned_agent: Option<String>,
    pub progress: f64,
    pub created_at: u64,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowInfo {
    pub id: String,
    pub name: String,
    pub status: String,
    pub progress: f64,
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub started_at: u64,
    pub completed_at: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub timestamp: u64,
    pub agents_count: usize,
    pub tasks_count: usize,
    pub workflows_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MetricsResponse {
    pub timestamp: u64,
    pub total_agents: usize,
    pub active_agents: usize,
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub failed_tasks: usize,
    pub pending_tasks: usize,
    pub network_latency_ms: f64,
    pub message_rate: f64,
    pub consensus_rounds: u64,
    pub avg_consensus_time_ms: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskCreateRequest {
    pub description: String,
    pub priority: Option<u8>,
    pub required_capabilities: Option<Vec<String>>,
    pub deadline: Option<u64>,
    pub dependencies: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskAssignRequest {
    pub task_id: String,
    pub agent_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkflowStartRequest {
    pub workflow_id: String,
    pub parameters: Option<HashMap<String, String>>,
}

// ============ API Routes ============

/// Create all API routes.
pub fn routes(
    state: ApiState,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    let health = health_endpoint();
    let metrics = metrics_endpoint(state.clone());
    let agents = agent_routes(state.clone());
    let tasks = task_routes(state.clone());
    let workflows = workflow_routes(state.clone());
    let websocket = websocket_endpoint(state.clone());

    health
        .or(metrics)
        .or(agents)
        .or(tasks)
        .or(workflows)
        .or(websocket)
        .recover(handle_rejection)
}

// Health check endpoint
fn health_endpoint() -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("api" / "health")
        .and(warp::get())
        .and_then(|| async {
            let response = HealthResponse {
                status: "ok".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                agents_count: 0,
                tasks_count: 0,
                workflows_count: 0,
            };
            Ok(warp::reply::json(&response))
        })
}

// Metrics endpoint
fn metrics_endpoint(
    state: ApiState,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("api" / "metrics")
        .and(warp::get())
        .and(with_state(state))
        .and_then(handle_get_metrics)
}

// Agent routes
fn agent_routes(
    state: ApiState,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    let list_agents = warp::path!("api" / "agents")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(handle_list_agents);

    let get_agent = warp::path!("api" / "agents" / String)
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(handle_get_agent);

    let update_agent = warp::path!("api" / "agents" / String)
        .and(warp::put())
        .and(warp::body::json())
        .and(with_state(state.clone()))
        .and_then(handle_update_agent);

    list_agents.or(get_agent).or(update_agent)
}

// Task routes
fn task_routes(
    state: ApiState,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    let list_tasks = warp::path!("api" / "tasks")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(handle_list_tasks);

    let create_task = warp::path!("api" / "tasks")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_state(state.clone()))
        .and_then(handle_create_task);

    let get_task = warp::path!("api" / "tasks" / String)
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(handle_get_task);

    let assign_task = warp::path!("api" / "tasks" / String / "assign")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_state(state.clone()))
        .and_then(handle_assign_task);

    let cancel_task = warp::path!("api" / "tasks" / String / "cancel")
        .and(warp::post())
        .and(with_state(state.clone()))
        .and_then(handle_cancel_task);

    list_tasks
        .or(create_task)
        .or(get_task)
        .or(assign_task)
        .or(cancel_task)
}

// Workflow routes
fn workflow_routes(
    state: ApiState,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    let list_workflows = warp::path!("api" / "workflows")
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(handle_list_workflows);

    let start_workflow = warp::path!("api" / "workflows" / "start")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_state(state.clone()))
        .and_then(handle_start_workflow);

    let get_workflow = warp::path!("api" / "workflows" / String)
        .and(warp::get())
        .and(with_state(state.clone()))
        .and_then(handle_get_workflow);

    let pause_workflow = warp::path!("api" / "workflows" / String / "pause")
        .and(warp::post())
        .and(with_state(state.clone()))
        .and_then(handle_pause_workflow);

    let resume_workflow = warp::path!("api" / "workflows" / String / "resume")
        .and(warp::post())
        .and(with_state(state.clone()))
        .and_then(handle_resume_workflow);

    let cancel_workflow = warp::path!("api" / "workflows" / String / "cancel")
        .and(warp::post())
        .and(with_state(state))
        .and_then(handle_cancel_workflow);

    list_workflows
        .or(start_workflow)
        .or(get_workflow)
        .or(pause_workflow)
        .or(resume_workflow)
        .or(cancel_workflow)
}

// WebSocket endpoint
fn websocket_endpoint(
    state: ApiState,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("ws")
        .and(warp::ws())
        .and(with_state(state))
        .and_then(handle_websocket)
}

// ============ Handler Implementations ============

async fn handle_get_metrics(
    state: ApiState,
) -> Result<impl warp::Reply, warp::Rejection> {
    let metrics = state.metrics.read().await;
    let response = MetricsResponse {
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        total_agents: metrics.total_agents,
        active_agents: metrics.active_agents,
        total_tasks: metrics.total_tasks,
        completed_tasks: metrics.completed_tasks,
        failed_tasks: metrics.failed_tasks,
        pending_tasks: metrics.pending_tasks,
        network_latency_ms: metrics.network_latency_ms,
        message_rate: metrics.message_rate,
        consensus_rounds: metrics.consensus_rounds,
        avg_consensus_time_ms: metrics.avg_consensus_time_ms,
    };
    Ok(warp::reply::json(&response))
}

async fn handle_list_agents(state: ApiState) -> Result<impl warp::Reply, warp::Rejection> {
    let agents = state.agents.read().await;
    Ok(warp::reply::json(&agents.values().collect::<Vec<_>>()))
}

async fn handle_get_agent(
    agent_id: String,
    state: ApiState,
) -> Result<impl warp::Reply, warp::Rejection> {
    let agents = state.agents.read().await;
    match agents.get(&agent_id) {
        Some(agent) => Ok(warp::reply::json(agent)),
        None => Err(warp::reject::not_found()),
    }
}

async fn handle_update_agent(
    agent_id: String,
    agent_info: AgentInfo,
    state: ApiState,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mut agents = state.agents.write().await;
    agents.insert(agent_id, agent_info);
    Ok(warp::reply::json(&"Agent updated"))
}

async fn handle_list_tasks(state: ApiState) -> Result<impl warp::Reply, warp::Rejection> {
    let tasks = state.tasks.read().await;
    Ok(warp::reply::json(&tasks.values().collect::<Vec<_>>()))
}

async fn handle_create_task(
    request: TaskCreateRequest,
    state: ApiState,
) -> Result<impl warp::Reply, warp::Rejection> {
    let task_id = uuid::Uuid::new_v4().to_string();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let task_info = TaskInfo {
        id: task_id.clone(),
        description: request.description,
        status: "pending".to_string(),
        priority: request.priority.unwrap_or(100),
        assigned_agent: None,
        progress: 0.0,
        created_at: now,
        started_at: None,
        completed_at: None,
    };

    {
        let mut tasks = state.tasks.write().await;
        tasks.insert(task_id.clone(), task_info.clone());
    }

    // Broadcast via WebSocket
    {
        let mut ws = state.websocket.write().await;
        ws.broadcast_task_created(&task_info).await;
    }

    Ok(warp::reply::json(&task_info))
}

async fn handle_get_task(
    task_id: String,
    state: ApiState,
) -> Result<impl warp::Reply, warp::Rejection> {
    let tasks = state.tasks.read().await;
    match tasks.get(&task_id) {
        Some(task) => Ok(warp::reply::json(task)),
        None => Err(warp::reject::not_found()),
    }
}

async fn handle_assign_task(
    task_id: String,
    request: TaskAssignRequest,
    state: ApiState,
) -> Result<impl warp::Reply, warp::Rejection> {
    if task_id != request.task_id {
        return Err(warp::reject::bad_request());
    }

    let mut tasks = state.tasks.write().await;
    if let Some(task) = tasks.get_mut(&task_id) {
        task.assigned_agent = Some(request.agent_id);
        task.status = "assigned".to_string();

        // Broadcast update
        let task_clone = task.clone();
        drop(tasks);

        let mut ws = state.websocket.write().await;
        ws.broadcast_task_updated(&task_clone).await;
    }

    Ok(warp::reply::json(&"Task assigned"))
}

async fn handle_cancel_task(
    task_id: String,
    state: ApiState,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mut tasks = state.tasks.write().await;
    if let Some(task) = tasks.get_mut(&task_id) {
        task.status = "cancelled".to_string();
        task.completed_at = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        );

        let task_clone = task.clone();
        drop(tasks);

        let mut ws = state.websocket.write().await;
        ws.broadcast_task_updated(&task_clone).await;
    }

    Ok(warp::reply::json(&"Task cancelled"))
}

async fn handle_list_workflows(state: ApiState) -> Result<impl warp::Reply, warp::Rejection> {
    let workflows = state.workflows.read().await;
    Ok(warp::reply::json(&workflows.values().collect::<Vec<_>>()))
}

async fn handle_start_workflow(
    request: WorkflowStartRequest,
    state: ApiState,
) -> Result<impl warp::Reply, warp::Rejection> {
    let workflow_id = uuid::Uuid::new_v4().to_string();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let workflow_info = WorkflowInfo {
        id: workflow_id.clone(),
        name: request.workflow_id,
        status: "running".to_string(),
        progress: 0.0,
        total_tasks: 0,
        completed_tasks: 0,
        started_at: now,
        completed_at: None,
    };

    {
        let mut workflows = state.workflows.write().await;
        workflows.insert(workflow_id.clone(), workflow_info.clone());
    }

    Ok(warp::reply::json(&workflow_info))
}

async fn handle_get_workflow(
    workflow_id: String,
    state: ApiState,
) -> Result<impl warp::Reply, warp::Rejection> {
    let workflows = state.workflows.read().await;
    match workflows.get(&workflow_id) {
        Some(workflow) => Ok(warp::reply::json(workflow)),
        None => Err(warp::reject::not_found()),
    }
}

async fn handle_pause_workflow(
    workflow_id: String,
    state: ApiState,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mut workflows = state.workflows.write().await;
    if let Some(workflow) = workflows.get_mut(&workflow_id) {
        workflow.status = "paused".to_string();
    }
    Ok(warp::reply::json(&"Workflow paused"))
}

async fn handle_resume_workflow(
    workflow_id: String,
    state: ApiState,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mut workflows = state.workflows.write().await;
    if let Some(workflow) = workflows.get_mut(&workflow_id) {
        workflow.status = "running".to_string();
    }
    Ok(warp::reply::json(&"Workflow resumed"))
}

async fn handle_cancel_workflow(
    workflow_id: String,
    state: ApiState,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mut workflows = state.workflows.write().await;
    if let Some(workflow) = workflows.get_mut(&workflow_id) {
        workflow.status = "cancelled".to_string();
        workflow.completed_at = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        );
    }
    Ok(warp::reply::json(&"Workflow cancelled"))
}

async fn handle_websocket(
    ws: warp::ws::Ws,
    state: ApiState,
) -> Result<impl warp::Reply, warp::Rejection> {
    Ok(ws.on_upgrade(move |websocket| {
        let state = state.clone();
        async move {
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
            
            {
                let mut ws_manager = state.websocket.write().await;
                ws_manager.add_client(tx).await;
            }

            let (mut user_tx, user_rx) = websocket.split();

            // Send messages to client
            let send_task = tokio::spawn(async move {
                use tokio_stream::wrappers::UnboundedReceiverStream;
                let stream = UnboundedReceiverStream::new(user_rx);
                if let Err(e) = tokio::io::AsyncWriteExt::write_all(&mut user_tx, &[]).await {
                    tracing::error!("WebSocket send error: {}", e);
                }
            });

            // Receive from client
            let recv_task = tokio::spawn(async move {
                // Handle client messages
            });

            tokio::select! {
                _ = send_task => {},
                _ = recv_task => {},
            }
        }
    }))
}

// Helper to extract state
fn with_state(
    state: ApiState,
) -> impl Filter<Extract = (ApiState,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || state.clone())
}

// Error handling
async fn handle_rejection(err: warp::Rejection) -> Result<impl warp::Reply, std::convert::Infallible> {
    if err.find::<warp::filters::body::BodyDeserializeError>().is_some() {
        Ok(warp::reply::with_status(
            "Invalid body".to_string(),
            warp::http::StatusCode::BAD_REQUEST,
        ))
    } else if err.find::<warp::reject::NotFound>().is_some() {
        Ok(warp::reply::with_status(
            "Not found".to_string(),
            warp::http::StatusCode::NOT_FOUND,
        ))
    } else {
        Ok(warp::reply::with_status(
            "Internal error".to_string(),
            warp::http::StatusCode::INTERNAL_SERVER_ERROR,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use warp::test::request;

    #[tokio::test]
    async fn test_health_endpoint() {
        let state = ApiState::new();
        let filter = routes(state);

        let response = request()
            .path("/api/health")
            .method("GET")
            .reply(&filter)
            .await;

        assert_eq!(response.status(), 200);
    }

    #[tokio::test]
    async fn test_create_task() {
        let state = ApiState::new();
        let filter = routes(state);

        let body = r#"{"description": "Test task", "priority": 150}"#;

        let response = request()
            .path("/api/tasks")
            .method("POST")
            .body(body)
            .header("content-type", "application/json")
            .reply(&filter)
            .await;

        assert_eq!(response.status(), 200);
    }
}
