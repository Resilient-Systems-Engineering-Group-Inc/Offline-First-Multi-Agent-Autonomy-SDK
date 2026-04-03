//! Demonstration of advanced autoscaling with predictive and multi‑metric policies.

use autoscaling::{
    advanced_scaling::{MetricsWindow, PredictiveScalingPolicy, MultiMetricScalingPolicy, MultiMetricConfig, PredictiveScalingConfig},
    policy::{ScalingPolicy, ScalingMetrics},
};
use std::time::{SystemTime, UNIX_EPOCH};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Advanced Autoscaling Demo ===");

    // 1. Create a metrics window to store historical data
    let mut window = MetricsWindow::new(10);
    println!("Created metrics window with capacity 10");

    // 2. Generate some sample metrics
    for i in 0..15 {
        let metrics = ScalingMetrics {
            agent_count: 5,
            avg_cpu_usage: 0.3 + (i as f64 * 0.05).min(0.9),
            avg_memory_usage: 0.4 + (i as f64 * 0.03).min(0.8),
            pending_tasks: i * 2,
            avg_task_latency_ms: 100.0 + (i as f64 * 10.0),
            network_bandwidth: 1024.0 * (i as f64 + 1.0),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() + i as u64,
        };
        window.push(metrics.clone());
        println!("  Added metrics {}: CPU={:.2}, tasks={}", i, metrics.avg_cpu_usage, metrics.pending_tasks);
    }

    println!("Window size after 15 pushes: {}", window.len());
    println!("Oldest timestamp: {}", window.oldest_timestamp());
    println!("Newest timestamp: {}", window.newest_timestamp());

    // 3. Create and test predictive scaling policy
    let predictive_config = PredictiveScalingConfig {
        window_size: 5,
        cpu_threshold: 0.7,
        memory_threshold: 0.75,
        pending_tasks_threshold: 15,
        scale_up_step: 2,
        scale_down_step: 1,
        min_agents: 2,
        max_agents: 20,
        trend_sensitivity: 0.1,
    };
    let predictive_policy = PredictiveScalingPolicy::new(predictive_config);
    println!("\nCreated PredictiveScalingPolicy with config:");
    println!("  CPU threshold: {}", predictive_config.cpu_threshold);
    println!("  Trend sensitivity: {}", predictive_config.trend_sensitivity);

    // Get latest metrics
    let latest = window.latest().unwrap();
    let decision = predictive_policy.evaluate(&latest).await;
    println!("Predictive policy decision: {:?}", decision);

    // 4. Create and test multi‑metric scaling policy
    let multi_config = MultiMetricConfig {
        weights: vec![
            ("cpu".to_string(), 0.4),
            ("memory".to_string(), 0.3),
            ("pending_tasks".to_string(), 0.2),
            ("latency".to_string(), 0.1),
        ],
        scale_up_threshold: 0.65,
        scale_down_threshold: 0.35,
        scale_up_step: 1,
        scale_down_step: 1,
        min_agents: 1,
        max_agents: 10,
    };
    let multi_policy = MultiMetricScalingPolicy::new(multi_config);
    println!("\nCreated MultiMetricScalingPolicy with weighted metrics");

    let decision2 = multi_policy.evaluate(&latest).await;
    println!("Multi‑metric policy decision: {:?}", decision2);

    // 5. Show trend analysis
    println!("\n=== Trend Analysis ===");
    if let Some(trend) = window.cpu_trend() {
        println!("CPU trend over window: {:.4} (positive = increasing)", trend);
    }
    if let Some(trend) = window.memory_trend() {
        println!("Memory trend over window: {:.4}", trend);
    }
    if let Some(trend) = window.pending_tasks_trend() {
        println!("Pending tasks trend over window: {:.4}", trend);
    }

    // 6. Demonstrate window statistics
    println!("\n=== Window Statistics ===");
    println!("CPU mean: {:.4}", window.cpu_mean().unwrap_or(0.0));
    println!("CPU stddev: {:.4}", window.cpu_stddev().unwrap_or(0.0));
    println!("Memory mean: {:.4}", window.memory_mean().unwrap_or(0.0));
    println!("Pending tasks mean: {:.4}", window.pending_tasks_mean().unwrap_or(0.0));

    // 7. Simulate a scaling loop
    println!("\n=== Simulated Scaling Loop ===");
    let mut simulated_agents = 5;
    for i in 0..5 {
        let simulated_metrics = ScalingMetrics {
            agent_count: simulated_agents,
            avg_cpu_usage: 0.6 + (i as f64 * 0.1),
            avg_memory_usage: 0.5,
            pending_tasks: 10 + i * 3,
            avg_task_latency_ms: 150.0,
            network_bandwidth: 2048.0,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() + 1000 + i as u64,
        };

        let decision = predictive_policy.evaluate(&simulated_metrics).await;
        match decision {
            autoscaling::policy::ScalingDecision::ScaleUp(n) => {
                simulated_agents += n;
                println!("Iteration {}: Scale up by {} → {} agents", i, n, simulated_agents);
            }
            autoscaling::policy::ScalingDecision::ScaleDown(n) => {
                simulated_agents = simulated_agents.saturating_sub(n);
                println!("Iteration {}: Scale down by {} → {} agents", i, n, simulated_agents);
            }
            autoscaling::policy::ScalingDecision::NoChange => {
                println!("Iteration {}: No change ({} agents)", i, simulated_agents);
            }
        }
    }

    println!("\n=== Demo Complete ===");
    Ok(())
}