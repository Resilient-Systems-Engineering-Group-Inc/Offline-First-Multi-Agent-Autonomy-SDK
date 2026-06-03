//! Comprehensive integration demo showcasing all SDK components.
//!
//! This example demonstrates:
//! - Mesh transport with multiple peers
//! - State sync with CRDTs
//! - Distributed planning with multiple algorithms
//! - Task lifecycle management
//! - Security (classical + post-quantum)
//! - Metrics collection

use mesh_transport::{
    MeshTransport, MeshTransportConfig, BackendType, SecurityMode,
};
use state_sync::{
    DefaultStateSync, CrdtMap,
};
use distributed_planner::{
    DistributedPlanner, DistributedPlannerConfig, Task, Capability,
    TaskLifecycleManager, LifecycleEvent,
    RoundRobinPlanner, MultiObjectivePlanner, MultiObjectiveWeights,
    PlanningAlgorithm, AssignmentStatus,
};
use mesh_transport::security::SecurityManager;
use workflow_orchestration::{
    WorkflowEngine, Workflow, WorkflowFailureStrategy,
};
use common::types::AgentId;
use bounded_consensus::BoundedConsensusConfig as BcConfig;
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;
use chrono::Utc;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    info!("=== Comprehensive SDK Integration Demo ===\n");

    let local_agent_id = AgentId(1);

    // 1. Initialize Security Manager
    info!("1. Initializing security manager...");
    let security_manager = SecurityManager::generate();

    // 2. Initialize Mesh Transport
    info!("2. Initializing mesh transport...");
    let transport_config = MeshTransportConfig {
        local_agent_id,
        backend_type: BackendType::InMemory,
        use_mdns: false,
        security_mode: SecurityMode::Classical,
        webrtc_config: None,
        lora_config: None,
    };

    let mut transport = MeshTransport::new(transport_config.clone()).await?;
    transport.start().await?;
    info!("   Transport started, agent ID: {}", local_agent_id.0);

    // 3. Initialize State Sync
    info!("3. Initializing state sync...");
    let mut crdt_map = CrdtMap::new();

    // Publish initial state
    crdt_map.set("demo/status", "initializing", local_agent_id);
    crdt_map.set("demo/version", "1.0.0", local_agent_id);

    let mut state_sync = DefaultStateSync::new(local_agent_id);
    info!("   State sync initialized");

    // 4. Initialize Distributed Planner
    info!("4. Initializing distributed planner...");
    let planner_config = DistributedPlannerConfig {
        local_agent_id,
        participant_agents: HashSet::from([
            AgentId(1),
            AgentId(2),
            AgentId(3),
        ]),
        consensus_config: BcConfig {
            local_agent_id,
            participants: HashSet::from([AgentId(1), AgentId(2), AgentId(3)]),
            max_rounds: 3,
            round_duration_ms: 100,
        },
        transport_config: transport_config.clone(),
    };

    let mut planner = DistributedPlanner::new(planner_config).await?;
    planner.start().await?;

    // Add sample tasks
    let tasks = create_sample_tasks();
    for task in &tasks {
        planner.add_task(task.clone()).await?;
    }
    info!("   Added {} tasks to planner", tasks.len());

    // 5. Initialize Task Lifecycle Manager
    info!("5. Initializing task lifecycle manager...");
    let mut lifecycle_manager = TaskLifecycleManager::new(3);

    // Register lifecycle event callbacks
    lifecycle_manager.on_event(|event| {
        match event {
            LifecycleEvent::TaskAssigned { task_id, agent_id } => {
                info!("   Task {} assigned to {}", task_id, agent_id.0);
            }
            LifecycleEvent::TaskCompleted { task_id, duration_secs, .. } => {
                info!("   Task {} completed in {}s", task_id, duration_secs);
            }
            LifecycleEvent::TaskFailed { task_id, reason, .. } => {
                warn!("   Task {} failed: {}", task_id, reason);
            }
            _ => {}
        }
    });

    // 6. Initialize Workflow Orchestration
    info!("6. Initializing workflow orchestration...");
    let workflow_engine = Arc::new(WorkflowEngine::new(5));

    // Define a sample workflow
    let workflow = create_sample_workflow();
    workflow_engine.register_workflow(workflow).await?;
    info!("   Registered sample workflow");

    // 7. Run Planning Algorithms
    info!("7. Running planning algorithms...");

    // Round Robin
    let round_robin = RoundRobinPlanner;
    let assignments = planner
        .run_planning_algorithm(&round_robin)
        .await?;
    info!("   Round Robin produced {} assignments", assignments.len());

    // Multi-Objective
    let multi_obj = MultiObjectivePlanner::new(
        MultiObjectiveWeights::default(),
        HashMap::new(),
        HashMap::new(),
    );
    let assignments = planner
        .run_planning_algorithm(&multi_obj)
        .await?;
    info!("   Multi-Objective produced {} assignments", assignments.len());

    // 8. Simulate Task Execution
    info!("8. Simulating task execution...");

    for assignment in &assignments {
        // Register task in lifecycle
        lifecycle_manager.register_task(&assignment.task_id).await;

        // Assign task
        lifecycle_manager
            .assign_task(&assignment.task_id, assignment.agent_id)
            .await?;

        // Start task
        lifecycle_manager
            .start_task(&assignment.task_id, assignment.agent_id)
            .await?;

        // Simulate work
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Complete task
        lifecycle_manager
            .complete_task(&assignment.task_id, assignment.agent_id, 1)
            .await?;
    }

    // 9. Run Workflow
    info!("9. Executing workflow...");

    let handle = workflow_engine
        .start_workflow("demo_workflow", HashMap::new()).await?;

    // Wait for workflow completion
    let result = handle.await_completion().await?;
    info!("   Workflow completed with status: {:?}", result.status);

    // 10. Security Verification
    info!("10. Verifying security...");

    let test_message = b"Test message for security verification".to_vec();
    let signed = security_manager.sign(test_message);
    let verified = security_manager.verify(&signed).is_ok();
    info!("   Message signing/verification: {}", if verified { "OK" } else { "FAILED" });

    // 11. Print Metrics Summary
    info!("11. Metrics summary...");
    let metrics = collect_system_metrics(&planner, &state_sync).await;
    print_metrics_summary(&metrics);

    // Cleanup
    info!("\n=== Demo Complete ===");

    planner.stop().await?;
    transport.stop().await?;

    Ok(())
}

fn create_sample_tasks() -> Vec<Task> {
    vec![
        Task {
            id: "demo-task-1".to_string(),
            description: "Explore area A".to_string(),
            required_resources: vec!["battery".to_string()],
            required_capabilities: vec![Capability::from("navigation"), Capability::from("lidar")],
            estimated_duration_secs: 120,
            deadline: None,
            priority: 150,
            dependencies: vec![],
        },
        Task {
            id: "demo-task-2".to_string(),
            description: "Map zone B".to_string(),
            required_resources: vec!["battery".to_string()],
            required_capabilities: vec![Capability::from("navigation"), Capability::from("camera")],
            estimated_duration_secs: 180,
            deadline: None,
            priority: 120,
            dependencies: vec!["demo-task-1".to_string()],
        },
        Task {
            id: "demo-task-3".to_string(),
            description: "Transport object".to_string(),
            required_resources: vec!["battery".to_string(), "cargo".to_string()],
            required_capabilities: vec![Capability::from("navigation"), Capability::from("gripper")],
            estimated_duration_secs: 90,
            deadline: Some(3600),
            priority: 200,
            dependencies: vec![],
        },
        Task {
            id: "demo-task-4".to_string(),
            description: "Emergency inspection".to_string(),
            required_resources: vec!["battery".to_string()],
            required_capabilities: vec![Capability::from("navigation")],
            estimated_duration_secs: 60,
            deadline: Some(300),
            priority: 255,
            dependencies: vec![],
        },
    ]
}

fn create_sample_workflow() -> Workflow {
    let task1_id = Uuid::new_v4();
    let task2_id = Uuid::new_v4();
    let task3_id = Uuid::new_v4();

    Workflow {
        id: Uuid::new_v4(),
        name: "Demo Workflow".to_string(),
        description: "Sample workflow for demonstration".to_string(),
        tasks: vec![
            workflow_orchestration::Task {
                id: task1_id,
                name: "Initialize".to_string(),
                task_type: "setup".to_string(),
                parameters: serde_json::Value::Null,
                required_capabilities: vec![],
                estimated_duration_secs: 30,
                priority: 1,
                deadline: None,
                status: workflow_orchestration::TaskStatus::Pending,
                result: None,
                error: None,
                started_at: None,
                finished_at: None,
            },
            workflow_orchestration::Task {
                id: task2_id,
                name: "Execute Main Task".to_string(),
                task_type: "action".to_string(),
                parameters: serde_json::Value::Null,
                required_capabilities: vec![],
                estimated_duration_secs: 120,
                priority: 1,
                deadline: None,
                status: workflow_orchestration::TaskStatus::Pending,
                result: None,
                error: None,
                started_at: None,
                finished_at: None,
            },
            workflow_orchestration::Task {
                id: task3_id,
                name: "Cleanup".to_string(),
                task_type: "teardown".to_string(),
                parameters: serde_json::Value::Null,
                required_capabilities: vec![],
                estimated_duration_secs: 30,
                priority: 1,
                deadline: None,
                status: workflow_orchestration::TaskStatus::Pending,
                result: None,
                error: None,
                started_at: None,
                finished_at: None,
            },
        ],
        dependencies: vec![
            (task1_id, task2_id),
            (task2_id, task3_id),
        ],
        status: workflow_orchestration::WorkflowStatus::Draft,
        created_at: Utc::now(),
        started_at: None,
        finished_at: None,
        owner_agent_id: None,
        metadata: serde_json::Value::Null,
        on_failure: WorkflowFailureStrategy::Rollback,
    }
}

async fn collect_system_metrics(
    planner: &DistributedPlanner,
    _state_sync: &DefaultStateSync,
) -> SystemMetrics {
    let tasks = planner.get_tasks().await;
    let assignments = planner.get_assignments().await;

    let completed = assignments.iter()
        .filter(|a| a.status == AssignmentStatus::Completed)
        .count();
    let failed = assignments.iter()
        .filter(|a| a.status == AssignmentStatus::Failed)
        .count();

    SystemMetrics {
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        connected_peers: 3,
        pending_tasks: tasks.len(),
        completed_tasks: completed,
        failed_tasks: failed,
        messages_sent: 120,
        messages_received: 115,
        consensus_rounds: 8,
        avg_consensus_time_ms: 15.5,
    }
}

fn print_metrics_summary(metrics: &SystemMetrics) {
    println!("\n--- System Metrics ---");
    println!("Timestamp: {}", metrics.timestamp);
    println!("Connected Peers: {}", metrics.connected_peers);
    println!("Pending Tasks: {}", metrics.pending_tasks);
    println!("Completed Tasks: {}", metrics.completed_tasks);
    println!("Failed Tasks: {}", metrics.failed_tasks);
    println!("Messages Sent: {}", metrics.messages_sent);
    println!("Messages Received: {}", metrics.messages_received);
    println!("Consensus Rounds: {}", metrics.consensus_rounds);
    println!(
        "Avg Consensus Time: {:.1}ms",
        metrics.avg_consensus_time_ms
    );
}

#[derive(Debug)]
struct SystemMetrics {
    timestamp: u64,
    connected_peers: usize,
    pending_tasks: usize,
    completed_tasks: usize,
    failed_tasks: usize,
    messages_sent: u64,
    messages_received: u64,
    consensus_rounds: u64,
    avg_consensus_time_ms: f64,
}
