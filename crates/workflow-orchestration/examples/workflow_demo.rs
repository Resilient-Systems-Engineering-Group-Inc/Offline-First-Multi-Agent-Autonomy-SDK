//! Workflow orchestration demo.
//!
//! Demonstrates:
//! - Loading workflow from YAML
//! - Starting workflow execution
//! - Monitoring progress
//! - Handling completion

use workflow_orchestration::{
    WorkflowEngine,
    WorkflowParser,
    WorkflowFailureStrategy,
};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{info, Level};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("=== Workflow Orchestration Demo ===\n");

    // Create workflow engine
    let engine = WorkflowEngine::new(max_concurrent = 4);

    // Load workflow from YAML
    info!("Loading workflow from YAML...");
    let workflow = WorkflowParser::load_from_file(
        "crates/workflow-orchestration/examples/workflow_example.yaml"
    )?;

    info!("Workflow: {} ({})", workflow.name, workflow.id);
    info!("Tasks: {}", workflow.tasks.len());
    info!("Failure strategy: {:?}", workflow.on_failure);

    // Register workflow
    let workflow_id = engine.register_workflow(workflow).await?;
    info!("Workflow registered: {}\n", workflow_id);

    // Create workflow parameters
    let mut parameters = HashMap::new();
    parameters.insert("warehouse_id".to_string(), "warehouse_001".to_string());
    parameters.insert("robot_count".to_string(), "4".to_string());
    parameters.insert("map_resolution".to_string(), "0.05".to_string());

    // Start workflow
    info!("Starting workflow execution...");
    let handle = engine.start_workflow(&workflow_id, parameters).await?;
    let instance_id = handle.instance_id().to_string();

    // Monitor progress
    tokio::spawn(async move {
        loop {
            let progress = handle.progress().await;
            let status = handle.status().await;
            
            info!("Progress: {:.1}% - Status: {:?}", progress, status);

            if progress >= 100.0 || status.unwrap_or_default().is_complete() {
                break;
            }

            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    });

    // Wait for completion
    info!("\nWaiting for workflow completion...");
    let result = handle.await_completion().await?;

    // Print results
    println!("\n=== Workflow Results ===");
    println!("Instance ID: {}", result.instance_id);
    println!("Status: {:?}", result.status);
    println!("Duration: {}s", result.duration_secs);
    println!("Completed tasks: {}", result.completed_tasks);
    println!("Failed tasks: {}", result.failed_tasks);

    if let Some(error) = result.error {
        println!("Error: {}", error);
    }

    // Print task outputs
    if !result.output.is_empty() {
        println!("\nTask Outputs:");
        for (task_id, output) in &result.output {
            println!("  {}:", task_id);
            for (key, value) in output {
                println!("    {}: {}", key, value);
            }
        }
    }

    // List all instances
    let instances = engine.list_instances().await;
    info!("\nTotal workflow instances: {}", instances.len());

    // Clean up
    engine.delete_instance(&instance_id).await;
    info!("Instance deleted");

    info!("\n=== Demo Complete ===");

    Ok(())
}

// Extension trait for checking workflow status completion
trait WorkflowStatusExt {
    fn is_complete(&self) -> bool;
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
}
