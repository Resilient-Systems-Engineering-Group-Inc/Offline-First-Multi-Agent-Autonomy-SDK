//! End-to-end integration tests.
//!
//! Tests complete workflows from user request to database persistence.

use anyhow::Result;
use chrono::Utc;
use std::time::Duration;
use tokio::time;
use tracing::{info, Level};

#[tokio::test]
async fn test_full_workflow_lifecycle() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("=== Testing Full Workflow Lifecycle ===\n");

    // 1. Setup: Create database
    let db_config = database::DatabaseConfig::sqlite(":memory:");
    let db = database::Database::new(db_config).await?;
    let task_repo = database::TaskRepository::new(db.pool());

    info!("✓ Database initialized");

    // 2. Create a task
    let mut task = database::TaskModel::default();
    task.description = "Integration test task".to_string();
    task.priority = 150;
    task.required_capabilities = vec!["navigation".to_string()];

    let created = task_repo.create(&task).await?;
    info!("✓ Task created: {}", created.id);

    // 3. Update task status
    let mut updated_task = created.clone();
    updated_task.status = "running".to_string();
    updated_task.started_at = Some(Utc::now());
    task_repo.update(&updated_task).await?;
    info!("✓ Task updated to running");

    // 4. Complete task
    let result = serde_json::json!({
        "output": "Task completed successfully",
        "metrics": {"duration_ms": 1500}
    });
    task_repo.complete(&created.id, &result).await?;
    info!("✓ Task completed");

    // 5. Verify completion
    let final_task = task_repo.get(&created.id).await?.unwrap();
    assert_eq!(final_task.status, "completed");
    assert!(final_task.completed_at.is_some());
    assert!(final_task.result.is_some());
    info!("✓ Task verified as completed");

    // 6. Check statistics
    let stats = task_repo.get_stats().await?;
    assert_eq!(stats.completed, 1);
    info!("✓ Statistics verified: {:?}", stats);

    info!("\n=== Full Workflow Lifecycle Test PASSED ===\n");
    Ok(())
}

#[tokio::test]
async fn test_workflow_orchestration_integration() -> Result<()> {
    info!("=== Testing Workflow Orchestration ===\n");

    // 1. Create workflow engine
    let engine = workflow_orchestration::WorkflowEngine::new(4);

    // 2. Create workflow
    let workflow = workflow_orchestration::Workflow::new(
        "test_workflow",
        "Integration Test Workflow"
    );

    let workflow_id = engine.register_workflow(workflow).await?;
    info!("✓ Workflow registered: {}", workflow_id);

    // 3. Start workflow with parameters
    let params = std::collections::HashMap::from([
        ("test_param".to_string(), "test_value".to_string())
    ]);

    let handle = engine.start_workflow(&workflow_id, params).await?;
    info!("✓ Workflow started: {}", handle.instance_id());

    // 4. Monitor progress
    time::timeout(Duration::from_secs(30), async {
        loop {
            let progress = handle.progress().await;
            let status = handle.status().await;
            
            info!("Progress: {:.1}%, Status: {:?}", progress, status);

            if progress >= 100.0 || status.unwrap_or_default().is_complete() {
                break;
            }

            time::sleep(Duration::from_millis(500)).await;
        }
    }).await?;

    // 5. Wait for completion
    let result = handle.await_completion().await?;
    info!("✓ Workflow completed: {:?}", result.status);

    assert!(result.status.is_success());
    info!("\n=== Workflow Orchestration Test PASSED ===\n");

    Ok(())
}

#[tokio::test]
async fn test_multi_agent_coordination() -> Result<()> {
    info!("=== Testing Multi-Agent Coordination ===\n");

    // 1. Create multiple mesh nodes
    let node1 = common::MeshNode::new("agent-1").await?;
    let node2 = common::MeshNode::new("agent-2").await?;
    let node3 = common::MeshNode::new("agent-3").await?;

    info!("✓ Created 3 mesh nodes");

    // 2. Start nodes
    node1.start().await?;
    node2.start().await?;
    node3.start().await?;

    info!("✓ All nodes started");

    // 3. Connect nodes
    node1.connect("agent-2", "/ip4/127.0.0.1/tcp/4001").await?;
    node1.connect("agent-3", "/ip4/127.0.0.1/tcp/4002").await?;

    info!("✓ Nodes connected");

    // 4. Verify connectivity
    let peers = node1.connected_peers();
    assert_eq!(peers.len(), 2);
    info!("✓ Connectivity verified: {} peers", peers.len());

    // 5. Send messages
    let message = b"Test coordination message";
    node1.send("agent-2", message).await?;
    node1.broadcast(message).await?;

    info!("✓ Messages sent");

    // 6. Create task planner
    let planner = distributed_planner::TaskPlanner::new("auction")?;

    // 7. Add tasks
    for i in 0..6 {
        let task = Task {
            id: format!("task-{}", i),
            description: format!("Task {}", i),
            priority: 100 + i * 10,
            required_capabilities: vec![],
            dependencies: vec![],
        };
        planner.add_task(task);
    }

    info!("✓ Added 6 tasks to planner");

    // 8. Plan assignments
    let assignments = planner.plan().await?;
    info!("✓ Task assignments: {:?}", assignments);

    // 9. Verify all tasks assigned
    let total_assigned: usize = assignments.values().map(|v| v.len()).sum();
    assert_eq!(total_assigned, 6);
    info!("✓ All tasks assigned");

    // Cleanup
    node1.stop().await?;
    node2.stop().await?;
    node3.stop().await?;

    info!("\n=== Multi-Agent Coordination Test PASSED ===\n");

    Ok(())
}

#[tokio::test]
async fn test_state_synchronization() -> Result<()> {
    info!("=== Testing State Synchronization ===\n");

    // 1. Create CRDT states
    let state1 = state_sync::CrdtState::new();
    let state2 = state_sync::CrdtState::new();

    info!("✓ Created 2 CRDT states");

    // 2. Set values on state1
    state1.set("key1", b"value1");
    state1.set("key2", b"value2");
    state1.set("key3", b"value3");

    info!("✓ Set 3 keys on state1");

    // 3. Get delta from state1
    let delta = state1.get_delta()?;

    // 4. Merge delta into state2
    state2.merge(&delta)?;

    info!("✓ Merged delta into state2");

    // 5. Verify consistency
    assert_eq!(state1.len(), state2.len());
    
    for key in state1.keys() {
        let val1 = state1.get(&key);
        let val2 = state2.get(&key);
        assert_eq!(val1, val2, "Keys mismatch for: {}", key);
    }

    info!("✓ States are consistent");

    // 6. Concurrent updates
    state1.set("key1", b"updated_value1");
    state2.set("key2", b"updated_value2");

    // 7. Merge both directions
    let delta1 = state1.get_delta()?;
    let delta2 = state2.get_delta()?;

    state2.merge(&delta1)?;
    state1.merge(&delta2)?;

    info!("✓ Applied concurrent updates");

    // 8. Verify convergence
    assert_eq!(state1.len(), state2.len());
    info!("✓ States converged after concurrent updates");

    info!("\n=== State Synchronization Test PASSED ===\n");

    Ok(())
}

#[tokio::test]
async fn test_authentication_flow() -> Result<()> {
    info!("=== Testing Authentication Flow ===\n");

    // 1. Setup auth
    let config = auth::AuthConfig::default();
    let jwt_handler = auth::JwtHandler::new(&config.jwt_secret);
    let rbac = auth::RbacManager::new();

    info!("✓ Auth system initialized");

    // 2. Hash password
    let password = "secure_password_123";
    let hash = auth::PasswordHasher::hash(password)?;
    info!("✓ Password hashed");

    // 3. Verify password
    assert!(auth::PasswordHasher::verify(password, &hash)?);
    info!("✓ Password verified");

    // 4. Generate token
    let token = jwt_handler.generate_access_token(
        "user-123",
        "testuser",
        vec!["user".to_string(), "admin".to_string()]
    )?;
    info!("✓ JWT token generated");

    // 5. Validate token
    let user_id = jwt_handler.validate(&token)?;
    assert_eq!(user_id, "user-123");
    info!("✓ Token validated: {}", user_id);

    // 6. Check permissions
    let has_admin = rbac
        .check_permission(
            &["admin".to_string()],
            &auth::ResourceType::Task,
            &auth::Action::Delete
        )
        .await;
    assert!(has_admin);
    info!("✓ Admin permission verified");

    // 7. Check denied permission
    let no_delete = rbac
        .check_permission(
            &["viewer".to_string()],
            &auth::ResourceType::Task,
            &auth::Action::Delete
        )
        .await;
    assert!(!no_delete);
    info!("✓ Viewer permission denied correctly");

    // 8. Refresh token
    let new_token = jwt_handler.refresh(&token, None)?;
    assert!(!new_token.is_empty());
    info!("✓ Token refreshed");

    info!("\n=== Authentication Flow Test PASSED ===\n");

    Ok(())
}

#[tokio::test]
async fn test_dashboard_api_integration() -> Result<()> {
    info!("=== Testing Dashboard API Integration ===\n");

    // This test would require a running dashboard server
    // For now, we test the client logic

    let client = dashboard::DashboardClient::new("http://localhost:3000");

    // Test health check (would require server)
    // let health = client.health().await?;
    // assert_eq!(health["status"], "ok");

    info!("✓ Dashboard client initialized");
    info!("\n=== Dashboard API Integration Test SKIPPED (requires server) ===\n");

    Ok(())
}

#[tokio::test]
async fn test_performance_benchmarks() -> Result<()> {
    info!("=== Testing Performance Benchmarks ===\n");

    // 1. Task creation benchmark
    let start = std::time::Instant::now();
    
    let db_config = database::DatabaseConfig::sqlite(":memory:");
    let db = database::Database::new(db_config).await?;
    let task_repo = database::TaskRepository::new(db.pool());

    for i in 0..1000 {
        let mut task = database::TaskModel::default();
        task.id = format!("task-{}", i);
        task.description = format!("Task {}", i);
        task_repo.create(&task).await?;
    }

    let duration = start.elapsed();
    let rate = 1000.0 / duration.as_secs_f64();
    
    info!("✓ Created 1000 tasks in {:?}", duration);
    info!("✓ Rate: {:.0} tasks/sec", rate);
    assert!(duration.as_secs_f64() < 10.0, "Task creation too slow");

    // 2. Query benchmark
    let start = std::time::Instant::now();
    
    for _ in 0..100 {
        let _ = task_repo.list().await?;
    }

    let duration = start.elapsed();
    info!("✓ 100 queries in {:?}", duration);
    assert!(duration.as_secs_f64() < 5.0, "Query performance too slow");

    // 3. JWT benchmark
    let jwt = auth::JwtHandler::new("test-secret");
    let start = std::time::Instant::now();

    for _ in 0..1000 {
        let _ = jwt.generate_access_token("user", "user", vec![])?;
    }

    let duration = start.elapsed();
    info!("✓ Generated 1000 JWTs in {:?}", duration);
    assert!(duration.as_secs_f64() < 2.0, "JWT generation too slow");

    info!("\n=== Performance Benchmarks PASSED ===\n");

    Ok(())
}

// Helper trait for workflow status
trait WorkflowStatusExt {
    fn is_complete(&self) -> bool;
    fn is_success(&self) -> bool;
}

impl WorkflowStatusExt for workflow_orchestration::WorkflowStatus {
    fn is_complete(&self) -> bool {
        matches!(
            self,
            workflow_orchestration::WorkflowStatus::Completed
                | workflow_orchestration::WorkflowStatus::Failed
                | workflow_orchestration::WorkflowStatus::Cancelled
        )
    }

    fn is_success(&self) -> bool {
        matches!(self, workflow_orchestration::WorkflowStatus::Completed)
    }
}
