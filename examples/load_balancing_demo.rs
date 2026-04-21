//! Load balancing demonstration for multi‑agent systems.
//!
//! This example shows various load balancing strategies and their application
//! in a simulated multi‑agent environment.

use std::collections::HashMap;
use std::time::Duration;
use load_balancer::prelude::*;
use load_balancer::metrics::AgentLoad;
use load_balancer::adaptive::{AdaptiveLoadBalancer, AdaptiveConfig, PerformanceFeedback};
use load_balancer::predictive::{PredictiveLoadBalancer, PredictiveConfig};
use load_balancer::coordinator::{DistributedLoadBalancer, DistributedCoordinatorConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Load Balancing for Multi‑Agent Systems Demo ===");
    
    // 1. Basic load balancing strategies
    println!("\n1. Basic Load Balancing Strategies:");
    
    // Create a load balancer with least‑loaded strategy
    let mut balancer = LoadBalancer::new(
        LoadBalancingStrategy::LeastLoaded,
        LoadBalancerConfig::default(),
    );
    
    // Register agents with different loads
    println!("   Registering agents with initial loads...");
    balancer.register_agent("agent-1", AgentLoad::comprehensive(0.3, 0.2, 5, 2, 50.0)).await?;
    balancer.register_agent("agent-2", AgentLoad::comprehensive(0.1, 0.1, 2, 0, 20.0)).await?;
    balancer.register_agent("agent-3", AgentLoad::comprehensive(0.5, 0.4, 10, 5, 100.0)).await?;
    balancer.register_agent("agent-4", AgentLoad::comprehensive(0.2, 0.3, 3, 1, 30.0)).await?;
    
    println!("   Agent loads:");
    for (agent_id, load) in balancer.get_all_agent_loads() {
        println!("     {}: total load = {:.3}, CPU = {:.1}%, tasks = {}", 
            agent_id, load.total_load(), load.cpu_utilization * 100.0, load.active_tasks);
    }
    
    // Test different strategies
    println!("\n   Testing different strategies:");
    
    // Round‑robin
    balancer.set_strategy(LoadBalancingStrategy::RoundRobin);
    for i in 0..3 {
        let selected = balancer.select_agent().await?;
        println!("     Round‑robin selection {}: {}", i + 1, selected);
    }
    
    // Least‑loaded
    balancer.set_strategy(LoadBalancingStrategy::LeastLoaded);
    let selected = balancer.select_agent().await?;
    println!("     Least‑loaded selection: {} (should be agent-2)", selected);
    
    // Weighted round‑robin
    balancer.set_strategy(LoadBalancingStrategy::WeightedRoundRobin);
    println!("     Weighted round‑robin selections:");
    for i in 0..5 {
        let selected = balancer.select_agent().await?;
        println!("       Selection {}: {}", i + 1, selected);
    }
    
    // Consistent hashing
    balancer.set_strategy(LoadBalancingStrategy::ConsistentHashing);
    let key1 = "session-123";
    let selected1 = balancer.select_agent_with_key(Some(key1)).await?;
    let selected2 = balancer.select_agent_with_key(Some(key1)).await?;
    println!("     Consistent hashing for key '{}': {} (consistent: {})", 
        key1, selected1, selected1 == selected2);
    
    // 2. Adaptive load balancing
    println!("\n2. Adaptive Load Balancing:");
    
    let adaptive_config = AdaptiveConfig {
        learning_rate: 0.2,
        exploration_rate: 0.4,
        ..Default::default()
    };
    
    let mut adaptive_balancer = AdaptiveLoadBalancer::new(balancer, adaptive_config);
    
    println!("   Initial exploration rate: {:.1}%", adaptive_balancer.exploration_rate() * 100.0);
    println!("   Best strategy initially: {}", adaptive_balancer.get_best_strategy());
    
    // Simulate some tasks with feedback
    println!("   Simulating tasks with feedback...");
    for i in 0..10 {
        let selected = adaptive_balancer.select_agent().await?;
        
        // Simulate performance (better for less loaded agents)
        let load = adaptive_balancer.base_balancer.get_agent_load(&selected)
            .map(|l| l.total_load())
            .unwrap_or(0.5);
        
        let response_time = 50.0 + load * 100.0; // Higher load -> slower response
        let success = rand::random::<f64>() > 0.1; // 90% success rate
        
        let feedback = PerformanceFeedback::new(success, response_time)
            .with_task_complexity(0.3);
        
        adaptive_balancer.update_feedback(&selected, &feedback).await?;
        
        if i % 3 == 0 {
            println!("     Task {}: selected {}, response {:.1}ms, success: {}", 
                i + 1, selected, response_time, success);
        }
    }
    
    println!("   Final exploration rate: {:.1}%", adaptive_balancer.exploration_rate() * 100.0);
    println!("   Best strategy after learning: {}", adaptive_balancer.get_best_strategy());
    
    // 3. Predictive load balancing
    println!("\n3. Predictive Load Balancing:");
    
    let predictive_config = PredictiveConfig {
        forecast_horizon_secs: 30,
        history_window: 20,
        ..Default::default()
    };
    
    let mut predictor = PredictiveLoadBalancer::new(predictive_config);
    
    // Simulate load patterns
    println!("   Simulating load patterns...");
    let agents = vec!["agent-a", "agent-b", "agent-c"];
    
    for minute in 0..15 {
        for agent_id in &agents {
            // Simulate different load patterns
            let base_load = match *agent_id {
                "agent-a" => 0.3 + 0.1 * (minute as f64 / 10.0).sin(), // Gradually increasing
                "agent-b" => 0.5 + 0.3 * (minute as f64 / 3.0).sin(),  // Oscillating
                "agent-c" => 0.2 + 0.05 * minute as f64,               // Linear increase
                _ => 0.4,
            };
            
            let noise = rand::random::<f64>() * 0.1;
            let load = (base_load + noise).clamp(0.0, 1.0);
            
            predictor.update_load(agent_id, load);
        }
        
        if minute % 5 == 0 {
            predictor.generate_predictions();
            
            println!("     Minute {} predictions:", minute);
            for agent_id in &agents {
                if let Some(pred) = predictor.get_prediction(agent_id) {
                    println!("       {}: predicted load {:.3} (confidence: {:.1}%)", 
                        agent_id, pred.predicted_load, pred.confidence * 100.0);
                }
            }
        }
    }
    
    // Find best agent based on predictions
    if let Some(best_pred) = predictor.find_best_agent() {
        println!("   Best agent based on predictions: {} (load: {:.3})", 
            best_pred.agent_id, best_pred.predicted_load);
    }
    
    // 4. Distributed load balancing
    println!("\n4. Distributed Load Balancing:");
    
    let coordinator_config = DistributedCoordinatorConfig {
        heartbeat_interval_secs: 2,
        heartbeat_timeout_secs: 10,
        quorum_size: 2,
        ..Default::default()
    };
    
    let balancer_config = LoadBalancerConfig::default();
    
    let mut distributed_lb = DistributedLoadBalancer::new(
        "coordinator-1",
        coordinator_config,
        balancer_config,
    ).await?;
    
    distributed_lb.start().await?;
    
    // Register this node as an agent
    println!("   Registering local agent...");
    let capabilities = HashMap::from([
        ("cpu_cores".to_string(), 4.0),
        ("memory_gb".to_string(), 8.0),
        ("gpu".to_string(), 1.0),
    ]);
    
    distributed_lb.register_as_agent("local-agent", capabilities).await?;
    
    // Simulate other agents joining
    println!("   Simulating other agents joining...");
    tokio::time::sleep(Duration::from_secs(1)).await;
    
    // Update local load
    let local_load = AgentLoad::comprehensive(0.4, 0.3, 8, 2, 45.0);
    distributed_lb.update_local_load(local_load).await?;
    
    // Check system state
    let agent_count = distributed_lb.agent_count().await;
    let has_quorum = distributed_lb.has_quorum().await;
    
    println!("   System state: {} agents, quorum: {}", agent_count, has_quorum);
    
    // Request load balancing decision
    println!("   Requesting load balancing decision...");
    let requirements = HashMap::from([
        ("min_cpu".to_string(), 2.0),
        ("min_memory".to_string(), 4.0),
    ]);
    
    match distributed_lb.select_agent(0.5, requirements).await {
        Ok(selected) => println!("   Selected agent: {}", selected),
        Err(e) => println!("   Error selecting agent: {}", e),
    }
    
    // 5. Load metrics and analysis
    println!("\n5. Load Metrics and Analysis:");
    
    let mut metrics_collector = LoadMetricsCollector::new(Duration::from_secs(5));
    
    // Add some metrics
    let agents_data = vec![
        ("agent-x", AgentLoad::comprehensive(0.2, 0.1, 3, 1, 25.0)),
        ("agent-y", AgentLoad::comprehensive(0.6, 0.5, 12, 4, 120.0)),
        ("agent-z", AgentLoad::comprehensive(0.4, 0.3, 6, 2, 60.0)),
    ];
    
    for (agent_id, load) in agents_data {
        metrics_collector.update(agent_id, load);
    }
    
    let metrics = metrics_collector.get_metrics();
    let stats = &metrics.statistics;
    
    println!("   Load statistics:");
    println!("     Agents: {}", stats.agent_count);
    println!("     Average load: {:.3}", stats.average_load);
    println!("     Min load: {:.3}, Max load: {:.3}", stats.min_load, stats.max_load);
    println!("     Load imbalance: {:.3}", stats.imbalance_score);
    println!("     Standard deviation: {:.3}", stats.std_dev_load);
    
    // Find overloaded agents
    let overloaded = metrics.find_overloaded(0.7);
    if !overloaded.is_empty() {
        println!("   Overloaded agents (threshold 0.7):");
        for (agent_id, load) in overloaded {
            println!("     {}: load = {:.3}", agent_id, load.total_load());
        }
    }
    
    // Find least loaded agent
    if let Some((agent_id, load)) = metrics.find_least_loaded() {
        println!("   Least loaded agent: {} (load = {:.3})", agent_id, load.total_load());
    }
    
    println!("\n=== Demo Completed Successfully ===");
    println!("\nSummary of Load Balancing Techniques Demonstrated:");
    println!("1. Basic strategies: Round‑robin, Least‑loaded, Weighted, Consistent hashing");
    println!("2. Adaptive learning: Reinforcement learning for strategy selection");
    println!("3. Predictive forecasting: Time‑series analysis for future load prediction");
    println!("4. Distributed coordination: Multi‑node coordination with heartbeats");
    println!("5. Comprehensive metrics: Load scoring, statistics, and imbalance detection");
    
    Ok(())
}