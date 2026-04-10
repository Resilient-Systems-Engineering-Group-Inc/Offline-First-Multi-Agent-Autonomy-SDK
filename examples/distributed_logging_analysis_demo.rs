//! Demonstration of distributed logging with analysis capabilities.
//!
//! This example shows how to:
//! 1. Create a distributed logger
//! 2. Generate synthetic log records from multiple agents
//! 3. Analyze logs for patterns and anomalies
//! 4. Display statistics and detected issues

use distributed_logging::{
    Logger, LogLevel, LogRecord, LogAnalyzer, LogAnalyzerConfig, 
    LogStatistics, AnomalyRule, utils
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use chrono::Utc;
use rand::Rng;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Distributed Logging Analysis Demo ===");
    
    // 1. Create log analyzer with custom configuration
    println!("Creating log analyzer...");
    let config = LogAnalyzerConfig {
        real_time_enabled: true,
        analysis_window_seconds: 60, // 1 minute window
        max_patterns: 50,
        anomaly_rules: vec![
            AnomalyRule::ErrorRateSpike {
                threshold: 0.2, // 20% error rate threshold
                window_seconds: 30,
                increase_factor: 2.0,
            },
            AnomalyRule::UnusualFrequency {
                message_pattern: "timeout".to_string(),
                normal_rate: 0.1, // 0.1 per minute
                deviation_factor: 5.0,
            },
        ],
        persist_results: false,
    };
    
    let analyzer = Arc::new(LogAnalyzer::new(config));
    
    // 2. Create simulated agents that generate logs
    println!("Starting simulated agents...");
    let agents = vec!["agent-1", "agent-2", "agent-3", "agent-4"];
    let mut tasks = Vec::new();
    
    for agent_id in agents {
        let analyzer_clone = analyzer.clone();
        let task = tokio::spawn(async move {
            simulate_agent_logs(agent_id, analyzer_clone).await;
        });
        tasks.push(task);
    }
    
    // 3. Let the simulation run for a while
    println!("Simulating for 30 seconds...");
    sleep(Duration::from_secs(30)).await;
    
    // 4. Stop simulation (in real scenario, you'd have proper shutdown)
    println!("Stopping simulation...");
    for task in tasks {
        task.abort(); // Simple abort for demo
    }
    
    // 5. Display analysis results
    println!("\n=== Analysis Results ===");
    
    // Statistics
    let stats = analyzer.get_statistics().await?;
    display_statistics(&stats);
    
    // Patterns
    let patterns = analyzer.get_patterns().await?;
    display_patterns(&patterns);
    
    // Anomalies
    let anomalies = analyzer.get_anomalies(Some(10)).await?;
    display_anomalies(&anomalies);
    
    // 6. Demonstrate log message templating
    println!("\n=== Log Message Templating Demo ===");
    let sample_messages = vec![
        "Connection to 192.168.1.1:8080 failed after 5000ms",
        "Connection to 10.0.0.5:443 failed after 3000ms",
        "User alice logged in from IP 203.0.113.5",
        "User bob logged in from IP 198.51.100.10",
    ];
    
    for msg in sample_messages {
        let template = utils::extract_template(msg);
        println!("  Original: {}", msg);
        println!("  Template: {}", template);
        println!();
    }
    
    println!("=== Demo Complete ===");
    Ok(())
}

/// Simulate an agent generating logs.
async fn simulate_agent_logs(agent_id: &str, analyzer: Arc<LogAnalyzer>) {
    let mut rng = rand::thread_rng();
    let mut error_count = 0;
    
    for i in 0..100 { // Generate up to 100 log entries
        // Random delay between logs
        let delay = rng.gen_range(100..500);
        sleep(Duration::from_millis(delay)).await;
        
        // Determine log level (mostly info, some warnings, occasional errors)
        let level_roll = rng.gen_range(0..100);
        let level = if level_roll < 5 {
            error_count += 1;
            LogLevel::Error
        } else if level_roll < 15 {
            LogLevel::Warn
        } else {
            LogLevel::Info
        };
        
        // Generate log message based on level
        let message = match level {
            LogLevel::Error => {
                // Simulate various error types
                let error_type = rng.gen_range(0..4);
                match error_type {
                    0 => format!("Connection timeout to server {}", rng.gen_range(1..10)),
                    1 => "Disk write failed: no space left on device".to_string(),
                    2 => format!("HTTP request failed with status {}", rng.gen_range(400..600)),
                    3 => "Database connection pool exhausted".to_string(),
                    _ => "Unknown error occurred".to_string(),
                }
            }
            LogLevel::Warn => {
                let warn_type = rng.gen_range(0..3);
                match warn_type {
                    0 => "High memory usage detected (85%)".to_string(),
                    1 => "Response time above threshold (2.5s)".to_string(),
                    2 => "Unusual network activity detected".to_string(),
                    _ => "Warning condition".to_string(),
                }
            }
            LogLevel::Info => {
                let info_type = rng.gen_range(0..5);
                match info_type {
                    0 => format!("Processing request #{}", rng.gen_range(1000..9999)),
                    1 => "Heartbeat sent successfully".to_string(),
                    2 => "Task completed in 150ms".to_string(),
                    3 => format!("Cache hit ratio: {}%", rng.gen_range(80..99)),
                    4 => "System metrics collected".to_string(),
                    _ => "Info message".to_string(),
                }
            }
            _ => "Log message".to_string(),
        };
        
        // Create log record
        let record = LogRecord {
            id: uuid::Uuid::new_v4(),
            timestamp: Utc::now(),
            level,
            source: Some(agent_id.to_string()),
            message,
            metadata: HashMap::from([
                ("iteration".to_string(), serde_json::json!(i)),
                ("agent_version".to_string(), serde_json::json!("1.0.0")),
            ]),
        };
        
        // Process the log record through analyzer
        if let Err(e) = analyzer.process_record(&record).await {
            eprintln!("Error processing log from {}: {}", agent_id, e);
        }
        
        // Occasionally inject a burst of errors to trigger anomaly detection
        if i == 50 && error_count < 3 {
            // Inject several error logs in quick succession
            for j in 0..5 {
                let error_record = LogRecord {
                    id: uuid::Uuid::new_v4(),
                    timestamp: Utc::now(),
                    level: LogLevel::Error,
                    source: Some(agent_id.to_string()),
                    message: format!("Simulated burst error #{}", j),
                    metadata: HashMap::new(),
                };
                
                if let Err(e) = analyzer.process_record(&error_record).await {
                    eprintln!("Error processing burst log: {}", e);
                }
            }
        }
    }
}

/// Display statistics in a readable format.
fn display_statistics(stats: &LogStatistics) {
    println!("Log Statistics:");
    println!("  Total records: {}", stats.total_records);
    println!("  Records by level:");
    for (level, count) in &stats.records_by_level {
        println!("    {:?}: {}", level, count);
    }
    
    println!("  Records by source (top 5):");
    let mut sources: Vec<_> = stats.records_by_source.iter().collect();
    sources.sort_by(|a, b| b.1.cmp(a.1));
    for (source, count) in sources.iter().take(5) {
        println!("    {}: {}", source, count);
    }
    
    if let (Some(earliest), Some(latest)) = (stats.earliest_timestamp, stats.latest_timestamp) {
        let duration = latest.signed_duration_since(earliest);
        println!("  Time range: {} to {} ({:.1} seconds)", 
            earliest.format("%H:%M:%S"), 
            latest.format("%H:%M:%S"),
            duration.num_seconds());
    }
    
    println!("  Error rate: {:.2}%", stats.error_rate * 100.0);
    println!("  Average rate: {:.2} records/second", stats.avg_rate_per_second);
}

/// Display detected patterns.
fn display_patterns(patterns: &[LogPattern]) {
    println!("\nDetected Patterns (top 10 by frequency):");
    
    let mut sorted_patterns = patterns.to_vec();
    sorted_patterns.sort_by(|a, b| b.occurrences.cmp(&a.occurrences));
    
    for (i, pattern) in sorted_patterns.iter().take(10).enumerate() {
        println!("  {}. {}", i + 1, pattern.pattern);
        println!("     Occurrences: {}, Sources: {}, Anomaly: {}", 
            pattern.occurrences, 
            pattern.sources.len(),
            pattern.is_anomaly);
    }
}

/// Display detected anomalies.
fn display_anomalies(anomalies: &[Anomaly]) {
    println!("\nDetected Anomalies:");
    
    if anomalies.is_empty() {
        println!("  No anomalies detected.");
        return;
    }
    
    for (i, anomaly) in anomalies.iter().enumerate() {
        println!("  {}. [Severity {}] {}", i + 1, anomaly.severity, anomaly.rule);
        println!("     Description: {}", anomaly.description);
        println!("     Detected: {}", anomaly.detected_at.format("%H:%M:%S"));
    }
}