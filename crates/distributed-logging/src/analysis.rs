//! Log analysis and anomaly detection.
//!
//! This module provides functionality for analyzing log streams, detecting
//! patterns, anomalies, and generating insights from distributed logs.

use chrono::{DateTime, Duration, Utc};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::Result;
use crate::log_record::{LogLevel, LogRecord};

/// Statistical summary of log records.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LogStatistics {
    /// Total number of log records.
    pub total_records: usize,
    /// Number of records by log level.
    pub records_by_level: HashMap<LogLevel, usize>,
    /// Number of records by source (agent ID).
    pub records_by_source: HashMap<String, usize>,
    /// Earliest timestamp in the dataset.
    pub earliest_timestamp: Option<DateTime<Utc>>,
    /// Latest timestamp in the dataset.
    pub latest_timestamp: Option<DateTime<Utc>>,
    /// Most frequent log messages (top N).
    pub frequent_messages: Vec<(String, usize)>,
    /// Error rate (errors / total records).
    pub error_rate: f64,
    /// Average log rate (records per second).
    pub avg_rate_per_second: f64,
}

/// Pattern detected in log messages.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LogPattern {
    /// Pattern identifier.
    pub id: String,
    /// Regular expression or template of the pattern.
    pub pattern: String,
    /// Number of occurrences.
    pub occurrences: usize,
    /// First occurrence timestamp.
    pub first_seen: DateTime<Utc>,
    /// Last occurrence timestamp.
    pub last_seen: DateTime<Utc>,
    /// Sources (agent IDs) that produced this pattern.
    pub sources: HashSet<String>,
    /// Whether this pattern indicates a potential issue.
    pub is_anomaly: bool,
}

/// Anomaly detection rule.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum AnomalyRule {
    /// Sudden increase in error rate.
    ErrorRateSpike {
        threshold: f64,          // Error rate threshold (0.0 to 1.0)
        window_seconds: u64,     // Time window to analyze
        increase_factor: f64,    // Minimum increase factor to trigger
    },
    /// Unusual log message frequency.
    UnusualFrequency {
        message_pattern: String, // Pattern to match
        normal_rate: f64,        // Normal rate (per minute)
        deviation_factor: f64,   // Deviation factor to trigger
    },
    /// Missing expected logs (heartbeats).
    MissingLogs {
        source: String,          // Expected source
        interval_seconds: u64,   // Expected interval
        grace_period_seconds: u64, // Grace period before alert
    },
    /// Correlation anomaly (multiple related errors).
    CorrelationAnomaly {
        patterns: Vec<String>,   // Patterns that should correlate
        max_time_gap_seconds: u64, // Maximum time gap between patterns
    },
}

/// Detected anomaly.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Anomaly {
    /// Unique anomaly identifier.
    pub id: uuid::Uuid,
    /// Anomaly type/rule that triggered.
    pub rule: String,
    /// Severity level (1-10, higher is more severe).
    pub severity: u8,
    /// Description of the anomaly.
    pub description: String,
    /// Timestamp when anomaly was detected.
    pub detected_at: DateTime<Utc>,
    /// Relevant log records that contributed to the anomaly.
    pub related_records: Vec<LogRecord>,
    /// Additional context/metadata.
    pub context: HashMap<String, serde_json::Value>,
}

/// Log analyzer configuration.
#[derive(Debug, Clone)]
pub struct LogAnalyzerConfig {
    /// Whether to enable real-time analysis.
    pub real_time_enabled: bool,
    /// Time window for statistical analysis (seconds).
    pub analysis_window_seconds: u64,
    /// Maximum number of patterns to track.
    pub max_patterns: usize,
    /// Anomaly detection rules.
    pub anomaly_rules: Vec<AnomalyRule>,
    /// Whether to persist analysis results.
    pub persist_results: bool,
}

impl Default for LogAnalyzerConfig {
    fn default() -> Self {
        Self {
            real_time_enabled: true,
            analysis_window_seconds: 300, // 5 minutes
            max_patterns: 100,
            anomaly_rules: vec![
                AnomalyRule::ErrorRateSpike {
                    threshold: 0.1,
                    window_seconds: 60,
                    increase_factor: 3.0,
                },
                AnomalyRule::MissingLogs {
                    source: "heartbeat".to_string(),
                    interval_seconds: 30,
                    grace_period_seconds: 60,
                },
            ],
            persist_results: false,
        }
    }
}

/// Main log analyzer.
pub struct LogAnalyzer {
    config: LogAnalyzerConfig,
    statistics: Arc<RwLock<LogStatistics>>,
    patterns: Arc<RwLock<HashMap<String, LogPattern>>>,
    recent_records: Arc<RwLock<VecDeque<LogRecord>>>,
    anomalies: Arc<RwLock<Vec<Anomaly>>>,
}

impl LogAnalyzer {
    /// Create a new log analyzer with the given configuration.
    pub fn new(config: LogAnalyzerConfig) -> Self {
        let statistics = LogStatistics {
            total_records: 0,
            records_by_level: HashMap::new(),
            records_by_source: HashMap::new(),
            earliest_timestamp: None,
            latest_timestamp: None,
            frequent_messages: Vec::new(),
            error_rate: 0.0,
            avg_rate_per_second: 0.0,
        };

        Self {
            config,
            statistics: Arc::new(RwLock::new(statistics)),
            patterns: Arc::new(RwLock::new(HashMap::new())),
            recent_records: Arc::new(RwLock::new(VecDeque::new())),
            anomalies: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Process a single log record.
    pub async fn process_record(&self, record: &LogRecord) -> Result<()> {
        // Update statistics
        self.update_statistics(record).await?;
        
        // Store in recent records (for time-window analysis)
        self.store_recent_record(record).await?;
        
        // Detect patterns
        self.detect_patterns(record).await?;
        
        // Check for anomalies
        self.check_anomalies(record).await?;
        
        Ok(())
    }

    /// Process a batch of log records.
    pub async fn process_batch(&self, records: &[LogRecord]) -> Result<()> {
        for record in records {
            self.process_record(record).await?;
        }
        Ok(())
    }

    /// Get current statistics.
    pub async fn get_statistics(&self) -> Result<LogStatistics> {
        let stats = self.statistics.read().await;
        Ok(stats.clone())
    }

    /// Get detected patterns.
    pub async fn get_patterns(&self) -> Result<Vec<LogPattern>> {
        let patterns = self.patterns.read().await;
        Ok(patterns.values().cloned().collect())
    }

    /// Get detected anomalies.
    pub async fn get_anomalies(&self, limit: Option<usize>) -> Result<Vec<Anomaly>> {
        let anomalies = self.anomalies.read().await;
        let result = if let Some(limit) = limit {
            anomalies.iter().take(limit).cloned().collect()
        } else {
            anomalies.clone()
        };
        Ok(result)
    }

    /// Run periodic analysis on the recent records window.
    pub async fn run_periodic_analysis(&self) -> Result<()> {
        let records = self.recent_records.read().await;
        
        if records.is_empty() {
            return Ok(());
        }
        
        // Calculate time-based statistics
        let now = Utc::now();
        let window_start = now - Duration::seconds(self.config.analysis_window_seconds as i64);
        
        let window_records: Vec<&LogRecord> = records
            .iter()
            .filter(|r| r.timestamp >= window_start)
            .collect();
        
        if window_records.is_empty() {
            return Ok(());
        }
        
        // Calculate error rate in window
        let total = window_records.len();
        let errors = window_records.iter()
            .filter(|r| r.level == LogLevel::Error)
            .count();
        
        let error_rate = if total > 0 { errors as f64 / total as f64 } else { 0.0 };
        
        // Update statistics with window-specific data
        let mut stats = self.statistics.write().await;
        stats.error_rate = error_rate;
        
        // Calculate rate per second
        let time_span = if let (Some(first), Some(last)) = (stats.earliest_timestamp, stats.latest_timestamp) {
            let duration = last.signed_duration_since(first);
            if duration.num_seconds() > 0 {
                stats.total_records as f64 / duration.num_seconds() as f64
            } else {
                0.0
            }
        } else {
            0.0
        };
        stats.avg_rate_per_second = time_span;
        
        Ok(())
    }

    /// Update statistics with a new record.
    async fn update_statistics(&self, record: &LogRecord) -> Result<()> {
        let mut stats = self.statistics.write().await;
        
        stats.total_records += 1;
        
        // Update level count
        *stats.records_by_level.entry(record.level.clone()).or_insert(0) += 1;
        
        // Update source count
        if let Some(source) = &record.source {
            *stats.records_by_source.entry(source.clone()).or_insert(0) += 1;
        }
        
        // Update timestamps
        if stats.earliest_timestamp.is_none() || record.timestamp < stats.earliest_timestamp.unwrap() {
            stats.earliest_timestamp = Some(record.timestamp);
        }
        if stats.latest_timestamp.is_none() || record.timestamp > stats.latest_timestamp.unwrap() {
            stats.latest_timestamp = Some(record.timestamp);
        }
        
        // Update frequent messages (simplified)
        // In a real implementation, you would use a more sophisticated algorithm
        let message_key = record.message.clone();
        // For simplicity, we'll just track the message as-is
        // In production, you might want to normalize or template the message
        
        Ok(())
    }

    /// Store record in recent records queue.
    async fn store_recent_record(&self, record: &LogRecord) -> Result<()> {
        let mut records = self.recent_records.write().await;
        
        // Add new record
        records.push_back(record.clone());
        
        // Remove old records outside the analysis window
        let cutoff = Utc::now() - Duration::seconds(self.config.analysis_window_seconds as i64);
        while let Some(front) = records.front() {
            if front.timestamp < cutoff {
                records.pop_front();
            } else {
                break;
            }
        }
        
        // Limit queue size
        while records.len() > 10000 {
            records.pop_front();
        }
        
        Ok(())
    }

    /// Detect patterns in log messages.
    async fn detect_patterns(&self, record: &LogRecord) -> Result<()> {
        // Simple pattern detection based on message structure
        // In production, you would use more sophisticated pattern mining
        
        let message = &record.message;
        
        // Extract potential pattern (simplified: use first few words)
        let words: Vec<&str> = message.split_whitespace().collect();
        if words.len() >= 3 {
            let pattern_key = format!("{} {} ...", words[0], words[1]);
            
            let mut patterns = self.patterns.write().await;
            
            if let Some(pattern) = patterns.get_mut(&pattern_key) {
                pattern.occurrences += 1;
                pattern.last_seen = record.timestamp;
                if let Some(source) = &record.source {
                    pattern.sources.insert(source.clone());
                }
                
                // Check if this pattern might be anomalous
                // Simple heuristic: if error level and frequent
                if record.level == LogLevel::Error && pattern.occurrences > 10 {
                    pattern.is_anomaly = true;
                }
            } else {
                // Create new pattern
                let mut sources = HashSet::new();
                if let Some(source) = &record.source {
                    sources.insert(source.clone());
                }
                
                let pattern = LogPattern {
                    id: uuid::Uuid::new_v4().to_string(),
                    pattern: pattern_key.clone(),
                    occurrences: 1,
                    first_seen: record.timestamp,
                    last_seen: record.timestamp,
                    sources,
                    is_anomaly: record.level == LogLevel::Error,
                };
                
                patterns.insert(pattern_key, pattern);
                
                // Limit number of patterns
                if patterns.len() > self.config.max_patterns {
                    // Remove least frequent pattern
                    let mut entries: Vec<_> = patterns.iter().collect();
                    entries.sort_by_key(|(_, p)| p.occurrences);
                    if let Some((key, _)) = entries.first() {
                        let key = key.clone().to_string();
                        patterns.remove(&key);
                    }
                }
            }
        }
        
        Ok(())
    }

    /// Check for anomalies based on configured rules.
    async fn check_anomalies(&self, record: &LogRecord) -> Result<()> {
        let mut anomalies = self.anomalies.write().await;
        
        // Check each rule
        for rule in &self.config.anomaly_rules {
            match rule {
                AnomalyRule::ErrorRateSpike { threshold, window_seconds, increase_factor } => {
                    // Simplified check: if this is an error record
                    if record.level == LogLevel::Error {
                        // In production, you would calculate actual error rate
                        // and compare with historical baseline
                        let stats = self.statistics.read().await;
                        if stats.error_rate > *threshold {
                            let anomaly = Anomaly {
                                id: uuid::Uuid::new_v4(),
                                rule: "ErrorRateSpike".to_string(),
                                severity: 7,
                                description: format!("Error rate spike detected: {:.2}% exceeds threshold {:.2}%", 
                                    stats.error_rate * 100.0, threshold * 100.0),
                                detected_at: Utc::now(),
                                related_records: vec![record.clone()],
                                context: HashMap::from([
                                    ("error_rate".to_string(), serde_json::json!(stats.error_rate)),
                                    ("threshold".to_string(), serde_json::json!(threshold)),
                                    ("window_seconds".to_string(), serde_json::json!(window_seconds)),
                                ]),
                            };
                            anomalies.push(anomaly);
                        }
                    }
                }
                AnomalyRule::UnusualFrequency { message_pattern, normal_rate, deviation_factor } => {
                    // Check if message matches pattern
                    if record.message.contains(message_pattern) {
                        // In production, you would track frequency and compare with normal rate
                        // For now, we'll just note if it's an error with this pattern
                        if record.level == LogLevel::Error {
                            let anomaly = Anomaly {
                                id: uuid::Uuid::new_v4(),
                                rule: "UnusualFrequency".to_string(),
                                severity: 5,
                                description: format!("Unusual frequency of pattern '{}' detected", message_pattern),
                                detected_at: Utc::now(),
                                related_records: vec![record.clone()],
                                context: HashMap::from([
                                    ("pattern".to_string(), serde_json::json!(message_pattern)),
                                    ("message".to_string(), serde_json::json!(record.message)),
                                ]),
                            };
                            anomalies.push(anomaly);
                        }
                    }
                }
                AnomalyRule::MissingLogs { source, interval_seconds, grace_period_seconds } => {
                    // This would require tracking expected logs and their timing
                    // Implementation would be more complex
                }
                AnomalyRule::CorrelationAnomaly { patterns, max_time_gap_seconds } => {
                    // Check if this record matches any pattern in the correlation set
                    for pattern in patterns {
                        if record.message.contains(pattern) {
                            // In production, you would track timing of correlated patterns
                        }
                    }
                }
            }
        }
        
        // Limit number of stored anomalies
        if anomalies.len() > 1000 {
            anomalies.drain(0..anomalies.len() - 500);
        }
        
        Ok(())
    }

    /// Clear all analysis data.
    pub async fn clear(&self) -> Result<()> {
        let mut stats = self.statistics.write().await;
        *stats = LogStatistics {
            total_records: 0,
            records_by_level: HashMap::new(),
            records_by_source: HashMap::new(),
            earliest_timestamp: None,
            latest_timestamp: None,
            frequent_messages: Vec::new(),
            error_rate: 0.0,
            avg_rate_per_second: 0.0,
        };
        
        let mut patterns = self.patterns.write().await;
        patterns.clear();
        
        let mut records = self.recent_records.write().await;
        records.clear();
        
        let mut anomalies = self.anomalies.write().await;
        anomalies.clear();
        
        Ok(())
    }
}

/// Utility functions for log analysis.
pub mod utils {
    use super::*;
    
    /// Extract common prefixes from log messages (simple templating).
    pub fn extract_template(message: &str) -> String {
        // Simple implementation: replace numbers and quoted strings with placeholders
        let mut template = message.to_string();
        
        // Replace numbers with <NUM>
        template = regex::Regex::new(r"\b\d+\b").unwrap().replace_all(&template, "<NUM>").to_string();
        
        // Replace quoted strings with <STR>
        template = regex::Regex::new(r#""[^"]*""#).unwrap().replace_all(&template, "<STR>").to_string();
        
        // Replace UUIDs with <UUID>
        template = regex::Regex::new(r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}")
            .unwrap()
            .replace_all(&template, "<UUID>")
            .to_string();
        
        template
    }
    
    /// Calculate similarity between two log messages.
    pub fn message_similarity(a: &str, b: &str) -> f64 {
        // Simple Jaccard similarity on word sets
        let a_words: HashSet<&str> = a.split_whitespace().collect();
        let b_words: HashSet<&str> = b.split_whitespace().collect();
        
        let intersection = a_words.intersection(&b_words).count();
        let union = a_words.union(&b_words).count();
        
        if union == 0 {
            0.0
        } else {
            intersection as f64 / union as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log_record::LogRecord;
    
    #[tokio::test]
    async fn test_analyzer_creation() {
        let config = LogAnalyzerConfig::default();
        let analyzer = LogAnalyzer::new(config);
        
        let stats = analyzer.get_statistics().await.unwrap();
        assert_eq!(stats.total_records, 0);
        assert!(stats.records_by_level.is_empty());
    }
    
    #[tokio::test]
    async fn test_process_record() {
        let config = LogAnalyzerConfig::default();
        let analyzer = LogAnalyzer::new(config);
        
        let record = LogRecord {
            id: uuid::Uuid::new_v4(),
            timestamp: Utc::now(),
            level: LogLevel::Info,
            source: Some("test-agent".to_string()),
            message: "System started successfully".to_string(),
            metadata: HashMap::new(),
        };
        
        analyzer.process_record(&record).await.unwrap();
        
        let stats = analyzer.get_statistics().await.unwrap();
        assert_eq!(stats.total_records, 1);
        assert_eq!(stats.records_by_level.get(&LogLevel::Info), Some(&1));
    }
    
    #[tokio::test]
    async fn test_pattern_detection() {
        let config = LogAnalyzerConfig::default();
        let analyzer = LogAnalyzer::new(config);
        
        let record1 = LogRecord {
            id: uuid::Uuid::new_v4(),
            timestamp: Utc::now(),
            level: LogLevel::Error,
            source: Some("agent-1".to_string()),
            message: "Connection failed to host 192.168.1.1".to_string(),
            metadata: HashMap::new(),
        };
        
        let record2 = LogRecord {
            id: uuid::Uuid::new_v4(),
            timestamp: Utc::now(),
            level: LogLevel::Error,
            source: Some("agent-2".to_string()),
            message: "Connection failed to host 192.168.1.2".to_string(),
            metadata: HashMap::new(),
        };
        
        analyzer.process_record(&record1).await.unwrap();
        analyzer.process_record(&record2).await.unwrap();
        
        let patterns = analyzer.get_patterns().await.unwrap();
        assert!(!patterns.is_empty());
    }
}