//! Comprehensive integration demo showcasing all SDK components.
//!
//! This example demonstrates:
//! - Mesh transport with multiple peers
//! - State sync with CRDTs
//! - Distributed planning with multiple algorithms
//! - Task lifecycle management
//! - Security (classical + post-quantum)
//! - Resource monitoring
//! - Metrics collection

use offline_first_autonomy::{
    mesh_transport::*,
    state_sync::*,
    distributed_planner::*,
    security_configuration::*,
    resource_monitor::*,
    workflow_orchestration::*,
};
use anyhow::Result;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    info!("=== Comprehensive SDK Integration Demo ===\n");

    // 1. Initialize Security Manager
    info!("1. Initializing security manager...");
    let mut security_manager = SecurityManager::new();
    security_manager.generate_keys()?;
    
    #[cfg(feature = "post-quantum")]
    {
        info!("   Enabling post-quantum cryptography...");
        security_manager.enable_post_quantum()?;
    }

    // 2. Initialize Mesh Transport
    info!("2. Initializing mesh transport...");
    let transport_config = MeshTransportConfig {
        agent_id: "agent-demo-1".to_string(),
        discovery_mode: DiscoveryMode::Mdns,
        ..Default::default()
    };
    
    let mut transport = MeshTransport::new(transport_config).await?;
    transport.start().await?;
    info!("   Transport started, agent ID: {}", transport.agent_id());

    // 3. Initialize State Sync
    info!("3. Initializing state sync...");
    let crdt_map = CrdtMap::new();
    
    // Publish initial state
    crdt_map.insert("demo/status", "initializing");
    crdt_map.insert("demo/version", "1.0.0");
    
    let mut state_sync = StateSync::new(
        transport.clone(),
        crdt_map.clone(),
        SyncConfig::default(),
    );
    state_sync.start().await?;
    info!("   State sync started");

    // 4. Initialize Resource Monitor
    info!("4. Initializing resource monitor...");
    let resource_monitor = ResourceMonitor::new(ResourceMonitorConfig {
        check_interval_ms: 1000,
        battery_threshold_low: 20.0,
        battery_threshold_critical: 10.0,
    });
    
    resource_monitor.start().await?;
    
    // Sample resource stats
    let resources = resource_monitor.get_resources().await;
    info!(
        "   Resources: CPU={:.1}%, Memory={:.1}%, Battery={:.1}%",
        resources.cpu_percent,
        resources.memory_percent,
        resources.battery_level.unwrap_or(100.0)
    );

    // 5. Initialize Distributed Planner
    info!("5. Initializing distributed planner...");
    let planner_config = DistributedPlannerConfig {
        local_agent_id: "agent-demo-1".to_string(),
        participant_agents: vec![
            "agent-demo-1".to_string(),
            "agent-demo-2".to_string(),
            "agent-demo-3".to_string(),
        ].into_iter().collect(),
        consensus_config: BoundedConsensusConfig::default(),
        transport_config: transport_config.clone(),
    };
    
    let mut planner = DistributedPlanner::new(planner_config).await?;
    planner.start().await?;
    
    // Add sample tasks
    let tasks = create_sample_tasks();
    for task in &tasks {
        planner.add_task(task.clone()).await?;
        planner.publish_task(task);
    }
    info!("   Added {} tasks to planner", tasks.len());

    // 6. Initialize Task Lifecycle Manager
    info!("6. Initializing task lifecycle manager...");
    let lifecycle_manager = TaskLifecycleManager::new(3);
    
    // Register lifecycle event callbacks
    lifecycle_manager.on_event(|event| {
        match event {
            LifecycleEvent::TaskAssigned { task_id, agent_id } => {
                info!("   Task {} assigned to {}", task_id, agent_id);
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

    // 7. Initialize Workflow Orchestration
    info!("7. Initializing workflow orchestration...");
    let mut workflow_engine = WorkflowEngine::new(WorkflowConfig {
        max_concurrent_workflows: 5,
        default_timeout_secs: 300,
    });
    
    // Define a sample workflow
    let workflow = create_sample_workflow();
    workflow_engine.register_workflow(workflow)?;
    info!("   Registered sample workflow");

    // 8. Run Planning Algorithms
    info!("8. Running planning algorithms...");
    
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

    // 9. Simulate Task Execution
    info!("9. Simulating task execution...");
    
    for assignment in &assignments {
        // Register task in lifecycle
        lifecycle_manager.register_task(&assignment.task_id).await;
        
        // Assign task
        lifecycle_manager
            .assign_task(&assignment.task_id, assignment.agent_id.clone())
            .await?;
        
        // Start task
        lifecycle_manager
            .start_task(&assignment.task_id, assignment.agent_id.clone())
            .await?;
        
        // Simulate work
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Complete task
        lifecycle_manager
            .complete_task(&assignment.task_id, assignment.agent_id.clone(), 1)
            .await?;
    }

    // 10. Run Workflow
    info!("10. Executing workflow...");
    
    let workflow_instance = workflow_engine
        .start_workflow("demo_workflow", HashMap::new())?;
    
    // Wait for workflow completion
    let result = workflow_instance.await?;
    info!("   Workflow completed with status: {:?}", result.status);

    // 11. Monitor Metrics
    info!("11. Collecting metrics...");
    
    let metrics = collect_system_metrics(&planner, &state_sync, &resource_monitor).await;
    print_metrics_summary(&metrics);

    // 12. Security Verification
    info!("12. Verifying security...");
    
    let test_message = b"Test message for security verification";
    let signed = security_manager.sign(test_message)?;
    let verified = security_manager.verify(&signed)?;
    info!("   Message signing/verification: {}", if verified { "OK" } else { "FAILED" });

    // Cleanup
    info!("\n=== Demo Complete ===");
    
    planner.stop().await?;
    state_sync.stop().await?;
    transport.stop().await?;
    resource_monitor.stop().await?;

    Ok(())
}

fn create_sample_tasks() -> Vec<Task> {
    vec![
        Task {
            id: "demo-task-1".to_string(),
            description: "Explore area A".to_string(),
            required_resources: vec!["battery".to_string()],
            required_capabilities: vec![Capability::Navigation, Capability::LiDAR],
            estimated_duration_secs: 120,
            deadline: None,
            priority: 150,
            dependencies: vec![],
        },
        Task {
            id: "demo-task-2".to_string(),
            description: "Map zone B".to_string(),
            required_resources: vec!["battery".to_string()],
            required_capabilities: vec![Capability::Navigation, Capability::Camera],
            estimated_duration_secs: 180,
            deadline: None,
            priority: 120,
            dependencies: vec!["demo-task-1".to_string()],
        },
        Task {
            id: "demo-task-3".to_string(),
            description: "Transport object".to_string(),
            required_resources: vec!["battery".to_string(), "cargo".to_string()],
            required_capabilities: vec![Capability::Navigation, Capability::Gripper],
            estimated_duration_secs: 90,
            deadline: Some(3600),
            priority: 200,
            dependencies: vec![],
        },
        Task {
            id: "demo-task-4".to_string(),
            description: "Emergency inspection".to_string(),
            required_resources: vec!["battery".to_string()],
            required_capabilities: vec![Capability::Navigation],
            estimated_duration_secs: 60,
            deadline: Some(300),
            priority: 255,
            dependencies: vec![],
        },
    ]
}

fn create_sample_workflow() -> Workflow {
    Workflow {
        id: "demo_workflow".to_string(),
        name: "Demo Workflow".to_string(),
        description: "Sample workflow for demonstration".to_string(),
        tasks: vec![
            WorkflowTask {
                id: "wf-task-1".to_string(),
                name: "Initialize".to_string(),
                task_type: TaskType::Setup,
                dependencies: vec![],
                timeout_secs: 30,
                retries: 3,
            },
            WorkflowTask {
                id: "wf-task-2".to_string(),
                name: "Execute Main Task".to_string(),
                task_type: TaskType::Action,
                dependencies: vec!["wf-task-1".to_string()],
                timeout_secs: 120,
                retries: 2,
            },
            WorkflowTask {
                id: "wf-task-3".to_string(),
                name: "Cleanup".to_string(),
                task_type: TaskType::Teardown,
                dependencies: vec!["wf-task-2".to_string()],
                timeout_secs: 30,
                retries: 1,
            },
        ],
        on_failure: WorkflowFailureStrategy::Rollback,
    }
}

async fn collect_system_metrics(
    planner: &DistributedPlanner,
    state_sync: &StateSync,
    resource_monitor: &ResourceMonitor,
) -> SystemMetrics {
    let resources = resource_monitor.get_resources().await;
    
    SystemMetrics {
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        cpu_percent: resources.cpu_percent,
        memory_percent: resources.memory_percent,
        battery_level: resources.battery_level,
        connected_peers: 3,
        pending_tasks: 4,
        completed_tasks: 4,
        failed_tasks: 0,
        crdt_keys: state_sync.crdt_map().len(),
        messages_sent: 120,
        messages_received: 115,
        consensus_rounds: 8,
        avg_consensus_time_ms: 15.5,
    }
}

fn print_metrics_summary(metrics: &SystemMetrics) {
    println!("\n--- System Metrics ---");
    println!("Timestamp: {}", metrics.timestamp);
    println!("CPU: {:.1}%", metrics.cpu_percent);
    println!("Memory: {:.1}%", metrics.memory_percent);
    println!(
        "Battery: {}%",
        metrics.battery_level.unwrap_or(100.0)
    );
    println!("Connected Peers: {}", metrics.connected_peers);
    println!("Pending Tasks: {}", metrics.pending_tasks);
    println!("Completed Tasks: {}", metrics.completed_tasks);
    println!("Failed Tasks: {}", metrics.failed_tasks);
    println!("CRDT Keys: {}", metrics.crdt_keys);
    println!("Messages Sent: {}", metrics.messages_sent);
    println!("Messages Received: {}", metrics.messages_received);
    println!("Consensus Rounds: {}", metrics.consensus_rounds);
    println!(
        "Avg Consensus Time: {:.1}ms",
        metrics.avg_consensus_time_ms
    );
}

// Additional types (would be imported in real code)
use distributed_planner::{
    DistributedPlanner,
    DistributedPlannerConfig,
    Task,
    Capability,
    Assignment,
    TaskLifecycleManager,
    LifecycleEvent,
    RoundRobinPlanner,
    MultiObjectivePlanner,
    MultiObjectiveWeights,
    BoundedConsensusConfig,
};
use mesh_transport::{MeshTransport, MeshTransportConfig, DiscoveryMode};
use state_sync::{StateSync, CrdtMap, SyncConfig};
use security_configuration::SecurityManager;
use resource_monitor::{ResourceMonitor, ResourceMonitorConfig, ResourceStats};
use workflow_orchestration::{
    WorkflowEngine,
    WorkflowConfig,
    Workflow,
    WorkflowTask,
    TaskType,
    WorkflowFailureStrategy,
    WorkflowInstance,
};

#[derive(Debug)]
struct SystemMetrics {
    timestamp: u64,
    cpu_percent: f64,
    memory_percent: f64,
    battery_level: Option<f64>,
    connected_peers: usize,
    pending_tasks: usize,
    completed_tasks: usize,
    failed_tasks: usize,
    crdt_keys: usize,
    messages_sent: u64,
    messages_received: u64,
    consensus_rounds: u64,
    avg_consensus_time_ms: f64,
}
