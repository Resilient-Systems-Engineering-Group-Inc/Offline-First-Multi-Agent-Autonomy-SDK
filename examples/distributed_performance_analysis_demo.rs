//! Distributed performance analysis demonstration.
//!
//! This example shows how to use the distributed analysis tools to detect
//! bottlenecks, correlations, and anomalies in a multi‑agent system.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use profiling::{
    DistributedPerformanceAnalyzer, BottleneckDetector, CorrelationAnalyzer, AnomalyDetector,
    init_distributed_analysis,
};
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Distributed Performance Analysis Demo ===");

    // 1. Initialize distributed analyzer
    println!("1. Initializing distributed performance analyzer...");
    let analyzer = init_distributed_analysis().await;

    // 2. Simulate metrics from multiple agents
    println!("2. Simulating metrics from 3 agents...");
    let agents = vec![1001, 1002, 1003];
    let components = vec!["mesh-transport", "state-sync", "agent-core"];

    for step in 0..50 {
        println!("   Step {}:", step + 1);
        for &agent_id in &agents {
            for &component in &components {
                let mut metrics = HashMap::new();

                // Generate realistic metrics with some noise and trends
                match component {
                    "mesh-transport" => {
                        let latency = 80.0 + (step as f64).sin() * 20.0 + rand::random::<f64>() * 10.0;
                        let queue_len = 500.0 + (step as f64).cos() * 200.0 + rand::random::<f64>() * 50.0;
                        let throughput = 15.0 + (step as f64).sin() * 5.0 + rand::random::<f64>() * 2.0;
                        metrics.insert("message_latency_ms".to_string(), latency);
                        metrics.insert("queue_length".to_string(), queue_len);
                        metrics.insert("throughput_mbps".to_string(), throughput);
                    }
                    "state-sync" => {
                        let sync_latency = 300.0 + (step as f64).sin() * 100.0 + rand::random::<f64>() * 30.0;
                        let conflict_rate = 0.05 + (step as f64).cos() * 0.03 + rand::random::<f64>() * 0.01;
                        metrics.insert("sync_latency_ms".to_string(), sync_latency);
                        metrics.insert("conflict_rate".to_string(), conflict_rate);
                    }
                    "agent-core" => {
                        let cpu_usage = 70.0 + (step as f64).sin() * 20.0 + rand::random::<f64>() * 10.0;
                        let memory_usage = 800.0 + (step as f64).cos() * 200.0 + rand::random::<f64>() * 50.0;
                        metrics.insert("cpu_usage".to_string(), cpu_usage);
                        metrics.insert("memory_usage_mb".to_string(), memory_usage);
                    }
                    _ => {}
                }

                // Inject an anomaly at step 25 for agent 1002
                if step == 25 && agent_id == 1002 && component == "mesh-transport" {
                    metrics.insert("message_latency_ms".to_string(), 300.0); // spike
                }

                // Update analyzer
                let anomalies = analyzer.update_metrics(agent_id, component, metrics).await;
                if !anomalies.is_empty() {
                    println!("     ! Anomaly detected for agent {}: {:?}", agent_id, anomalies[0].metric);
                }
            }
        }

        // Sleep to simulate real‑time interval
        sleep(Duration::from_millis(100)).await;
    }

    // 3. Generate and display report
    println!("3. Generating performance report...");
    let report = analyzer.generate_report().await;
    println!("   Report generated at {:?}", report.generated_at);
    println!("   Total agents monitored: {}", report.total_agents);
    println!("   Total metrics collected: {}", report.total_metrics);
    println!("   Active bottlenecks: {}", report.bottleneck_count);
    println!("   Strong correlations: {}", report.strong_correlation_count);

    if !report.top_bottlenecks.is_empty() {
        println!("   Top bottlenecks:");
        for (i, bottleneck) in report.top_bottlenecks.iter().enumerate() {
            println!("     {}. {}: {} = {:.1} (threshold {:.1})",
                     i + 1, bottleneck.component, bottleneck.metric,
                     bottleneck.current_value, bottleneck.threshold);
        }
    }

    if !report.top_correlations.is_empty() {
        println!("   Top correlations:");
        for (i, corr) in report.top_correlations.iter().enumerate() {
            println!("     {}. {} ↔ {}: r = {:.3} ({} samples)",
                     i + 1, corr.metric_a, corr.metric_b,
                     corr.correlation, corr.sample_count);
        }
    }

    // 4. Stand‑alone detectors demonstration
    println!("4. Stand‑alone detectors demonstration...");

    // Bottleneck detector
    println!("   a) Bottleneck detector:");
    let mut bottleneck_detector = BottleneckDetector::new();
    let mut test_metrics = HashMap::new();
    test_metrics.insert("cpu_usage".to_string(), 95.0);
    test_metrics.insert("memory_usage_mb".to_string(), 512.0);
    let bottlenecks = bottleneck_detector.analyze("agent-core", &test_metrics);
    for bottleneck in &bottlenecks {
        println!("      - {} bottleneck: {:.1} > {:.1} (severity {:.2})",
                 bottleneck.metric, bottleneck.current_value,
                 bottleneck.threshold, bottleneck.severity);
    }

    // Correlation analyzer
    println!("   b) Correlation analyzer:");
    let mut correlation_analyzer = CorrelationAnalyzer::new(100);
    let now = SystemTime::now();
    for i in 0..30 {
        let t = now + Duration::from_secs(i as u64);
        correlation_analyzer.record_sample("cpu_usage", t, i as f64 * 0.5 + 30.0);
        correlation_analyzer.record_sample("memory_usage", t, i as f64 * 0.3 + 200.0);
    }
    let corr = correlation_analyzer.compute_correlation("cpu_usage", "memory_usage");
    if let Some(c) = corr {
        println!("      CPU ↔ Memory correlation: r = {:.3} (significant: {})",
                 c.correlation, c.significant);
    }

    // Anomaly detector
    println!("   c) Anomaly detector:");
    let mut anomaly_detector = AnomalyDetector::new(20, 2.5);
    for i in 0..19 {
        anomaly_detector.add_value("latency", 50.0 + i as f64 * 0.5);
    }
    let anomaly = anomaly_detector.add_value("latency", 120.0); // outlier
    match anomaly {
        Some(a) => println!("      Anomaly detected: {} = {:.1} (expected {:.1}, σ = {:.1})",
                            a.metric, a.observed_value, a.expected_value, a.deviation_sigma),
        None => println!("      No anomaly detected (unexpected)"),
    }

    println!("=== Demo completed successfully ===");
    Ok(())
}