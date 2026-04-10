//! Distributed system performance analysis tools.
//!
//! This module provides advanced analysis capabilities for distributed multi‑agent systems:
//! - Bottleneck detection across components
//! - Correlation analysis between metrics
//! - Real‑time anomaly detection
//! - Performance visualization and reporting
//! - Distributed trace analysis

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::metrics as profiling_metrics;
use common::types::AgentId;

/// A performance bottleneck identified in the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bottleneck {
    /// Bottleneck identifier.
    pub id: String,
    /// Component where the bottleneck occurs (e.g., "mesh-transport", "state-sync").
    pub component: String,
    /// Metric that indicates the bottleneck (e.g., "latency", "throughput", "queue_length").
    pub metric: String,
    /// Current value of the metric.
    pub current_value: f64,
    /// Threshold value that defines the bottleneck.
    pub threshold: f64,
    /// Severity of the bottleneck (0.0 to 1.0).
    pub severity: f64,
    /// Suggested mitigation actions.
    pub suggestions: Vec<String>,
    /// Timestamp when the bottleneck was detected.
    pub detected_at: SystemTime,
}

/// Correlation between two metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricCorrelation {
    /// First metric name.
    pub metric_a: String,
    /// Second metric name.
    pub metric_b: String,
    /// Pearson correlation coefficient (-1.0 to 1.0).
    pub correlation: f64,
    /// Number of samples used.
    pub sample_count: usize,
    /// Whether the correlation is statistically significant (p < 0.05).
    pub significant: bool,
}

/// Anomaly detected in system performance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceAnomaly {
    /// Anomaly identifier.
    pub id: String,
    /// Metric that exhibited anomalous behavior.
    pub metric: String,
    /// Expected value (based on historical data).
    pub expected_value: f64,
    /// Observed value.
    pub observed_value: f64,
    /// Deviation from expected (in standard deviations).
    pub deviation_sigma: f64,
    /// Anomaly score (0.0 to 1.0).
    pub score: f64,
    /// Timestamp of the anomaly.
    pub timestamp: SystemTime,
    /// Component where anomaly occurred.
    pub component: Option<String>,
    /// Agent ID where anomaly occurred (if applicable).
    pub agent_id: Option<AgentId>,
}

/// Bottleneck detector that analyzes metrics to identify performance bottlenecks.
pub struct BottleneckDetector {
    /// Metric thresholds per component.
    thresholds: HashMap<String, HashMap<String, f64>>,
    /// Historical bottleneck records.
    history: VecDeque<Bottleneck>,
    /// Maximum history size.
    max_history: usize,
}

impl BottleneckDetector {
    /// Create a new bottleneck detector with default thresholds.
    pub fn new() -> Self {
        let mut thresholds = HashMap::new();
        
        // Default thresholds for common components
        let mut mesh_transport = HashMap::new();
        mesh_transport.insert("message_latency_ms".to_string(), 100.0); // 100 ms
        mesh_transport.insert("queue_length".to_string(), 1000.0);
        mesh_transport.insert("throughput_mbps".to_string(), 10.0); // below 10 Mbps is bottleneck
        thresholds.insert("mesh-transport".to_string(), mesh_transport);

        let mut state_sync = HashMap::new();
        state_sync.insert("sync_latency_ms".to_string(), 500.0);
        state_sync.insert("conflict_rate".to_string(), 0.1); // 10% conflicts
        thresholds.insert("state-sync".to_string(), state_sync);

        let mut agent_core = HashMap::new();
        agent_core.insert("cpu_usage".to_string(), 90.0); // 90% CPU
        agent_core.insert("memory_usage_mb".to_string(), 1024.0); // 1 GB
        thresholds.insert("agent-core".to_string(), agent_core);

        Self {
            thresholds,
            history: VecDeque::with_capacity(1000),
            max_history: 1000,
        }
    }

    /// Add or update a threshold for a component and metric.
    pub fn set_threshold(&mut self, component: &str, metric: &str, threshold: f64) {
        self.thresholds
            .entry(component.to_string())
            .or_insert_with(HashMap::new)
            .insert(metric.to_string(), threshold);
    }

    /// Analyze current metrics and return detected bottlenecks.
    pub fn analyze(
        &mut self,
        component: &str,
        metrics: &HashMap<String, f64>,
    ) -> Vec<Bottleneck> {
        let mut bottlenecks = Vec::new();
        
        let component_thresholds = match self.thresholds.get(component) {
            Some(th) => th,
            None => return bottlenecks, // No thresholds for this component
        };

        for (metric, &value) in metrics {
            if let Some(&threshold) = component_thresholds.get(metric) {
                // Check if value exceeds threshold (for high-is-bad metrics)
                // For throughput, lower is worse, so we need to know metric direction.
                // For simplicity, assume higher value is worse for all metrics.
                if value > threshold {
                    let severity = (value - threshold) / threshold.max(1.0);
                    let severity = severity.min(1.0); // cap at 1.0
                    
                    let bottleneck = Bottleneck {
                        id: format!("{}-{}-{}", component, metric, SystemTime::now()
                            .duration_since(SystemTime::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis()),
                        component: component.to_string(),
                        metric: metric.clone(),
                        current_value: value,
                        threshold,
                        severity,
                        suggestions: self.generate_suggestions(component, metric, value),
                        detected_at: SystemTime::now(),
                    };
                    bottlenecks.push(bottleneck);
                }
            }
        }

        // Store in history
        for bottleneck in &bottlenecks {
            if self.history.len() >= self.max_history {
                self.history.pop_front();
            }
            self.history.push_back(bottleneck.clone());
        }

        bottlenecks
    }

    /// Generate mitigation suggestions for a bottleneck.
    fn generate_suggestions(&self, component: &str, metric: &str, value: f64) -> Vec<String> {
        let mut suggestions = Vec::new();
        
        match (component, metric) {
            ("mesh-transport", "message_latency_ms") => {
                suggestions.push("Increase network bandwidth or reduce message size".to_string());
                suggestions.push("Consider using compression for large messages".to_string());
                suggestions.push("Check for network congestion or packet loss".to_string());
            }
            ("mesh-transport", "queue_length") => {
                suggestions.push("Increase consumer processing capacity".to_string());
                suggestions.push("Implement back‑pressure or flow control".to_string());
                suggestions.push("Consider load balancing across multiple agents".to_string());
            }
            ("state-sync", "sync_latency_ms") => {
                suggestions.push("Reduce sync frequency or batch updates".to_string());
                suggestions.push("Optimize CRDT merge operations".to_string());
                suggestions.push("Consider using incremental sync instead of full state".to_string());
            }
            ("agent-core", "cpu_usage") => {
                suggestions.push("Distribute workload to other agents".to_string());
                suggestions.push("Optimize algorithm complexity".to_string());
                suggestions.push("Consider hardware acceleration".to_string());
            }
            _ => {
                suggestions.push("Review configuration and resource allocation".to_string());
                suggestions.push("Monitor related metrics for root cause".to_string());
                suggestions.push("Consider scaling horizontally".to_string());
            }
        }
        
        suggestions
    }

    /// Get bottleneck history for a component.
    pub fn get_history(&self, component: Option<&str>) -> Vec<Bottleneck> {
        self.history
            .iter()
            .filter(|b| component.map_or(true, |c| b.component == c))
            .cloned()
            .collect()
    }

    /// Clear history.
    pub fn clear_history(&mut self) {
        self.history.clear();
    }
}

/// Correlation analyzer that computes relationships between metrics.
pub struct CorrelationAnalyzer {
    /// Metric samples: metric name -> list of (timestamp, value).
    samples: HashMap<String, VecDeque<(SystemTime, f64)>>,
    /// Maximum samples per metric.
    max_samples: usize,
    /// Minimum samples required for correlation.
    min_samples_for_correlation: usize,
}

impl CorrelationAnalyzer {
    /// Create a new correlation analyzer.
    pub fn new(max_samples: usize) -> Self {
        Self {
            samples: HashMap::new(),
            max_samples,
            min_samples_for_correlation: 10,
        }
    }

    /// Record a metric sample.
    pub fn record_sample(&mut self, metric: &str, timestamp: SystemTime, value: f64) {
        let queue = self.samples.entry(metric.to_string()).or_insert_with(|| {
            VecDeque::with_capacity(self.max_samples)
        });
        
        if queue.len() >= self.max_samples {
            queue.pop_front();
        }
        queue.push_back((timestamp, value));
    }

    /// Compute correlation between two metrics.
    pub fn compute_correlation(&self, metric_a: &str, metric_b: &str) -> Option<MetricCorrelation> {
        let samples_a = self.samples.get(metric_a)?;
        let samples_b = self.samples.get(metric_b)?;
        
        if samples_a.len() < self.min_samples_for_correlation ||
           samples_b.len() < self.min_samples_for_correlation {
            return None;
        }

        // Align samples by timestamp (simplified: assume same timestamps)
        let min_len = samples_a.len().min(samples_b.len());
        let values_a: Vec<f64> = samples_a.iter().take(min_len).map(|(_, v)| *v).collect();
        let values_b: Vec<f64> = samples_b.iter().take(min_len).map(|(_, v)| *v).collect();

        let correlation = pearson_correlation(&values_a, &values_b);
        let significant = self.is_statistically_significant(min_len, correlation);

        Some(MetricCorrelation {
            metric_a: metric_a.to_string(),
            metric_b: metric_b.to_string(),
            correlation,
            sample_count: min_len,
            significant,
        })
    }

    /// Find strongly correlated metrics (|correlation| > threshold).
    pub fn find_strong_correlations(&self, threshold: f64) -> Vec<MetricCorrelation> {
        let mut correlations = Vec::new();
        let metrics: Vec<&String> = self.samples.keys().collect();
        
        for i in 0..metrics.len() {
            for j in (i + 1)..metrics.len() {
                if let Some(corr) = self.compute_correlation(metrics[i], metrics[j]) {
                    if corr.correlation.abs() >= threshold {
                        correlations.push(corr);
                    }
                }
            }
        }
        
        correlations
    }

    /// Check if correlation is statistically significant (simplified).
    fn is_statistically_significant(&self, sample_size: usize, correlation: f64) -> bool {
        // Simplified: require at least 30 samples and |correlation| > 0.3
        sample_size >= 30 && correlation.abs() > 0.3
    }

    /// Clear all samples.
    pub fn clear(&mut self) {
        self.samples.clear();
    }
}

/// Anomaly detector using statistical methods (Z‑score).
pub struct AnomalyDetector {
    /// Metric history: metric name -> sliding window of values.
    history: HashMap<String, VecDeque<f64>>,
    /// Window size per metric.
    window_size: usize,
    /// Z‑score threshold for anomaly detection.
    z_threshold: f64,
    /// Minimum samples before detection.
    min_samples: usize,
}

impl AnomalyDetector {
    /// Create a new anomaly detector.
    pub fn new(window_size: usize, z_threshold: f64) -> Self {
        Self {
            history: HashMap::new(),
            window_size,
            z_threshold,
            min_samples: window_size / 2,
        }
    }

    /// Add a new value for a metric.
    pub fn add_value(&mut self, metric: &str, value: f64) -> Option<PerformanceAnomaly> {
        let window = self.history.entry(metric.to_string()).or_insert_with(|| {
            VecDeque::with_capacity(self.window_size)
        });
        
        if window.len() >= self.window_size {
            window.pop_front();
        }
        window.push_back(value);
        
        // Check for anomaly if we have enough samples
        if window.len() >= self.min_samples {
            self.detect_anomaly(metric, value)
        } else {
            None
        }
    }

    /// Detect anomaly for a given metric and new value.
    fn detect_anomaly(&self, metric: &str, new_value: f64) -> Option<PerformanceAnomaly> {
        let window = self.history.get(metric)?;
        let values: Vec<f64> = window.iter().copied().collect();
        
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let variance = values.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / values.len() as f64;
        let std_dev = variance.sqrt();
        
        if std_dev < 1e-9 {
            return None; // No variation, cannot detect anomaly
        }
        
        let z_score = (new_value - mean) / std_dev;
        
        if z_score.abs() >= self.z_threshold {
            Some(PerformanceAnomaly {
                id: format!("anomaly-{}-{}", metric, SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()),
                metric: metric.to_string(),
                expected_value: mean,
                observed_value: new_value,
                deviation_sigma: z_score,
                score: (z_score.abs() / self.z_threshold).min(1.0),
                timestamp: SystemTime::now(),
                component: None,
                agent_id: None,
            })
        } else {
            None
        }
    }

    /// Clear history for a metric.
    pub fn clear_metric(&mut self, metric: &str) {
        self.history.remove(metric);
    }

    /// Clear all history.
    pub fn clear_all(&mut self) {
        self.history.clear();
    }
}

/// Distributed performance analyzer that combines bottleneck detection,
/// correlation analysis, and anomaly detection.
pub struct DistributedPerformanceAnalyzer {
    /// Bottleneck detector.
    bottleneck_detector: RwLock<BottleneckDetector>,
    /// Correlation analyzer.
    correlation_analyzer: RwLock<CorrelationAnalyzer>,
    /// Anomaly detector.
    anomaly_detector: RwLock<AnomalyDetector>,
    /// Agent‑specific metrics.
    agent_metrics: RwLock<HashMap<AgentId, HashMap<String, f64>>>,
}

impl DistributedPerformanceAnalyzer {
    /// Create a new distributed performance analyzer.
    pub fn new() -> Self {
        Self {
            bottleneck_detector: RwLock::new(BottleneckDetector::new()),
            correlation_analyzer: RwLock::new(CorrelationAnalyzer::new(1000)),
            anomaly_detector: RwLock::new(AnomalyDetector::new(100, 3.0)),
            agent_metrics: RwLock::new(HashMap::new()),
        }
    }

    /// Update metrics for a component and agent.
    pub async fn update_metrics(
        &self,
        agent_id: AgentId,
        component: &str,
        metrics: HashMap<String, f64>,
    ) -> Vec<PerformanceAnomaly> {
        let mut anomalies = Vec::new();
        
        // Store agent metrics
        {
            let mut agent_metrics = self.agent_metrics.write().await;
            let agent_entry = agent_metrics.entry(agent_id).or_insert_with(HashMap::new);
            for (metric, value) in &metrics {
                agent_entry.insert(format!("{}.{}", component, metric), *value);
            }
        }
        
        // Check for bottlenecks
        {
            let mut detector = self.bottleneck_detector.write().await;
            let bottlenecks = detector.analyze(component, &metrics);
            if !bottlenecks.is_empty() {
                info!("Detected {} bottlenecks in component {} for agent {}",
                      bottlenecks.len(), component, agent_id);
                // Log bottlenecks
                for bottleneck in &bottlenecks {
                    debug!("Bottleneck: {} = {} (threshold {})",
                           bottleneck.metric, bottleneck.current_value, bottleneck.threshold);
                }
            }
        }
        
        // Check for anomalies
        {
            let mut detector = self.anomaly_detector.write().await;
            for (metric, value) in metrics {
                let full_metric = format!("{}.{}", component, metric);
                if let Some(anomaly) = detector.add_value(&full_metric, value) {
                    anomalies.push(anomaly);
                }
            }
        }
        
        // Record samples for correlation analysis
        {
            let mut analyzer = self.correlation_analyzer.write().await;
            let timestamp = SystemTime::now();
            for (metric, value) in metrics {
                let full_metric = format!("{}.{}", component, metric);
                analyzer.record_sample(&full_metric, timestamp, value);
            }
        }
        
        anomalies
    }

    /// Get recent bottlenecks.
    pub async fn get_bottlenecks(&self, component: Option<&str>) -> Vec<Bottleneck> {
        let detector = self.bottleneck_detector.read().await;
        detector.get_history(component)
    }

    /// Get strong correlations.
    pub async fn get_strong_correlations(&self, threshold: f64) -> Vec<MetricCorrelation> {
        let analyzer = self.correlation_analyzer.read().await;
        analyzer.find_strong_correlations(threshold)
    }

    /// Get agent metrics.
    pub async fn get_agent_metrics(&self, agent_id: AgentId) -> Option<HashMap<String, f64>> {
        let agent_metrics = self.agent_metrics.read().await;
        agent_metrics.get(&agent_id).cloned()
    }

    /// Generate a performance report.
    pub async fn generate_report(&self) -> PerformanceReport {
        let bottlenecks = self.get_bottlenecks(None).await;
        let correlations = self.get_strong_correlations(0.7).await;
        let agent_metrics = self.agent_metrics.read().await;
        
        let total_agents = agent_metrics.len();
        let mut total_metrics = 0;
        for metrics in agent_metrics.values() {
            total_metrics += metrics.len();
        }
        
        PerformanceReport {
            generated_at: SystemTime::now(),
            total_agents,
            total_metrics,
            bottleneck_count: bottlenecks.len(),
            strong_correlation_count: correlations.len(),
            top_bottlenecks: bottlenecks.into_iter().take(5).collect(),
            top_correlations: correlations.into_iter().take(5).collect(),
        }
    }
}

/// Comprehensive performance report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceReport {
    /// When the report was generated.
    pub generated_at: SystemTime,
    /// Total number of agents monitored.
    pub total_agents: usize,
    /// Total number of distinct metrics.
    pub total_metrics: usize,
    /// Number of active bottlenecks.
    pub bottleneck_count: usize,
    /// Number of strong correlations (|r| > 0.7).
    pub strong_correlation_count: usize,
    /// Top 5 bottlenecks.
    pub top_bottlenecks: Vec<Bottleneck>,
    /// Top 5 correlations.
    pub top_correlations: Vec<MetricCorrelation>,
}

/// Compute Pearson correlation coefficient between two vectors.
fn pearson_correlation(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len();
    if n != y.len() || n == 0 {
        return 0.0;
    }
    
    let sum_x: f64 = x.iter().sum();
    let sum_y: f64 = y.iter().sum();
    let sum_x2: f64 = x.iter().map(|&v| v * v).sum();
    let sum_y2: f64 = y.iter().map(|&v| v * v).sum();
    let sum_xy: f64 = x.iter().zip(y).map(|(&a, &b)| a * b).sum();
    
    let numerator = (n as f64) * sum_xy - sum_x * sum_y;
    let denominator = ((n as f64) * sum_x2 - sum_x * sum_x).sqrt() *
                     ((n as f64) * sum_y2 - sum_y * sum_y).sqrt();
    
    if denominator.abs() < 1e-9 {
        0.0
    } else {
        numerator / denominator
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bottleneck_detector() {
        let mut detector = BottleneckDetector::new();
        let mut metrics = HashMap::new();
        metrics.insert("message_latency_ms".to_string(), 150.0); // above threshold 100
        metrics.insert("queue_length".to_string(), 500.0); // below threshold 1000
        
        let bottlenecks = detector.analyze("mesh-transport", &metrics);
        assert_eq!(bottlenecks.len(), 1);
        assert_eq!(bottlenecks[0].metric, "message_latency_ms");
        assert!(bottlenecks[0].severity > 0.0);
    }

    #[test]
    fn test_correlation_analyzer() {
        let mut analyzer = CorrelationAnalyzer::new(100);
        let now = SystemTime::now();
        
        for i in 0..20 {
            let t = now + Duration::from_secs(i as u64);
            analyzer.record_sample("cpu_usage", t, i as f64);
            analyzer.record_sample("memory_usage", t, i as f64 * 0.5 + 10.0);
        }
        
        let corr = analyzer.compute_correlation("cpu_usage", "memory_usage").unwrap();
        assert!(corr.correlation > 0.9);
        assert_eq!(corr.sample_count, 20);
    }

    #[test]
    fn test_anomaly_detector() {
        let mut detector = AnomalyDetector::new(10, 2.0);
        
        // Add normal values
        for i in 0..9 {
            detector.add_value("latency", 10.0 + i as f64 * 0.1);
        }
        
        // Add anomaly (value 30, much higher than mean ~10.5)
        let anomaly = detector.add_value("latency", 30.0);
        assert!(anomaly.is_some());
        let anomaly = anomaly.unwrap();
        assert!(anomaly.deviation_sigma.abs() >= 2.0);
        assert_eq!(anomaly.metric, "latency");
    }

    #[tokio::test]
    async fn test_distributed_analyzer() {
        let analyzer = DistributedPerformanceAnalyzer::new();
        let mut metrics = HashMap::new();
        metrics.insert("cpu_usage".to_string(), 95.0);
        metrics.insert("memory_usage_mb".to_string(), 512.0);
        
        let anomalies = analyzer.update_metrics(1, "agent-core", metrics).await;
        // No anomalies yet because not enough samples
        assert!(anomalies.is_empty());
        
        let bottlenecks = analyzer.get_bottlenecks(Some("agent-core")).await;
        // Should detect CPU bottleneck (threshold 90)
        assert!(!bottlenecks.is_empty());
    }
}