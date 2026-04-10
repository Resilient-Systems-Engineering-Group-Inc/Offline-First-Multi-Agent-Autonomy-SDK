//! Demonstration of advanced infrastructure orchestration features.
//!
//! This example shows:
//! 1. Creating an infrastructure orchestrator
//! 2. Performing health checks
//! 3. Estimating costs
//! 4. Dynamic updates based on agent metrics
//! 5. Multi-cloud deployment configuration

use infrastructure_integration::{
    DeploymentConfig, InfrastructureOrchestrator, DynamicUpdateConfig,
    MultiCloudConfig, LoadBalancingStrategy, FailoverConfig,
    AgentMetrics,
};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Infrastructure Orchestration Demo ===\n");

    // 1. Create a deployment configuration
    let config = DeploymentConfig::default();
    println!("1. Created default deployment configuration:");
    println!("   - Provider: {:?}", config.provider);
    println!("   - Region: {}", config.region);
    println!("   - Agents: {} instances", config.agents[0].count);
    println!("   - Machine type: {}", config.agents[0].machine_type);

    // 2. Create dynamic update configuration
    let dynamic_config = DynamicUpdateConfig {
        enable_autoscaling: true,
        min_agents: 2,
        max_agents: 10,
        cpu_threshold: 70.0,
        memory_threshold: 80.0,
        cooldown_seconds: 300,
        allow_cross_provider: true,
    };

    // 3. Create multi-cloud configuration (optional)
    let multi_cloud_config = MultiCloudConfig {
        primary_provider: config.provider.clone(),
        fallback_providers: vec![
            infrastructure_integration::config::CloudProvider::Azure,
            infrastructure_integration::config::CloudProvider::Gcp,
        ],
        distribute_resources: true,
        load_balancing_strategy: LoadBalancingStrategy::CostOptimized,
        failover_config: FailoverConfig {
            enabled: true,
            health_check_interval_seconds: 30,
            max_failover_time_seconds: 300,
            excluded_providers: std::collections::HashSet::new(),
        },
    };

    // 4. Create the orchestrator
    let orchestrator = InfrastructureOrchestrator::new(
        config,
        dynamic_config,
        Some(multi_cloud_config),
    );
    println!("\n2. Created infrastructure orchestrator");
    println!("   - Deployment ID: {}", orchestrator.get_state().await.deployment_id);

    // 5. Perform health check
    println!("\n3. Performing health check...");
    let health_result = orchestrator.perform_health_check().await?;
    println!("   - Status: {:?}", health_result.health.status);
    println!("   - Healthy resources: {}", health_result.health.healthy_count);
    println!("   - Duration: {:?}", health_result.duration);

    // 6. Estimate costs
    println!("\n4. Estimating costs...");
    let cost_estimate = orchestrator.estimate_cost(None).await?;
    println!("   - Monthly cost: ${:.2}", cost_estimate.monthly_cost);
    println!("   - Hourly cost: ${:.4}", cost_estimate.hourly_cost);
    println!("   - Confidence: {:.0}%", cost_estimate.confidence * 100.0);

    // 7. Simulate agent metrics and trigger dynamic updates
    println!("\n5. Simulating high CPU usage (85%) to trigger scaling...");
    let mut agent_metrics = HashMap::new();
    agent_metrics.insert(
        "agent-1".to_string(),
        AgentMetrics {
            cpu_usage: 85.0,  // High CPU
            memory_usage: 65.0,
            network_throughput: 1000.0,
            disk_io: 50.0,
            active_tasks: 15,
            uptime_seconds: 3600,
        },
    );
    agent_metrics.insert(
        "agent-2".to_string(),
        AgentMetrics {
            cpu_usage: 90.0,  // Very high CPU
            memory_usage: 70.0,
            network_throughput: 1200.0,
            disk_io: 60.0,
            active_tasks: 20,
            uptime_seconds: 3600,
        },
    );

    let update_actions = orchestrator.update_based_on_agent_state(&agent_metrics).await?;
    
    if !update_actions.is_empty() {
        println!("   - Triggered {} update action(s):", update_actions.len());
        for (i, action) in update_actions.iter().enumerate() {
            println!("     {}. {:?}", i + 1, action);
        }
    } else {
        println!("   - No scaling needed at this time");
    }

    // 8. Show final state
    println!("\n6. Final infrastructure state:");
    let state = orchestrator.get_state().await;
    println!("   - Deployment ID: {}", state.deployment_id);
    println!("   - Created: {}", state.created_at);
    println!("   - Updated: {}", state.updated_at);
    println!("   - Health status: {:?}", state.health.status);
    println!("   - Estimated cost: ${:.2}/month", 
        state.estimated_cost.unwrap_or(0.0));

    // 9. Demonstrate drift detection
    println!("\n7. Checking for infrastructure drift...");
    let drift_result = orchestrator.detect_drift().await?;
    if drift_result.drift_detected {
        println!("   - Drift detected in {} resources", drift_result.drifted_resources);
        for drift in &drift_result.drifts {
            println!("     * {}: {} (expected: {}, actual: {})", 
                drift.resource_id, drift.field, drift.expected, drift.actual);
        }
    } else {
        println!("   - No infrastructure drift detected");
    }

    println!("\n=== Demo completed successfully ===");
    Ok(())
}