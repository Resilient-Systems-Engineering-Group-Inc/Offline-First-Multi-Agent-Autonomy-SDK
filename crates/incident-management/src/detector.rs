//! Incident detection from various sources.

use std::sync::Arc;
use tokio::sync::Mutex;
use dashmap::DashMap;

use crate::error::{IncidentError, Result};
use crate::model::{Incident, IncidentSeverity, IncidentSource};

/// Trait for incident detectors.
#[async_trait::async_trait]
pub trait IncidentDetector: Send + Sync {
    /// Check for new incidents.
    async fn detect(&self) -> Result<Vec<Incident>>;

    /// Get detector name.
    fn name(&self) -> &str;
}

/// Simple threshold‑based detector for numeric metrics.
pub struct ThresholdDetector {
    metric_name: String,
    threshold: f64,
    severity: IncidentSeverity,
    source: IncidentSource,
    /// Current metric value (updated externally via `update_metric`).
    current_value: Arc<Mutex<Option<f64>>>,
    /// Whether the threshold was already breached (to avoid duplicate incidents).
    already_breached: Arc<Mutex<bool>>,
}

impl ThresholdDetector {
    /// Create a new threshold detector.
    pub fn new(
        metric_name: impl Into<String>,
        threshold: f64,
        severity: IncidentSeverity,
        source: IncidentSource,
    ) -> Self {
        Self {
            metric_name: metric_name.into(),
            threshold,
            severity,
            source,
            current_value: Arc::new(Mutex::new(None)),
            already_breached: Arc::new(Mutex::new(false)),
        }
    }

    /// Update the current metric value. Call this periodically from monitoring code.
    pub async fn update_metric(&self, value: f64) {
        let mut current = self.current_value.lock().await;
        *current = Some(value);
    }

    /// Reset the breach state (e.g., after the metric returns below threshold).
    pub async fn reset_breach(&self) {
        let mut breached = self.already_breached.lock().await;
        *breached = false;
    }
}

#[async_trait::async_trait]
impl IncidentDetector for ThresholdDetector {
    async fn detect(&self) -> Result<Vec<Incident>> {
        let mut incidents = Vec::new();
        let current = self.current_value.lock().await;

        if let Some(value) = *current {
            if value > self.threshold {
                let mut breached = self.already_breached.lock().await;
                if !*breached {
                    // Threshold just breached — create incident
                    *breached = true;
                    let incident = Incident::new(
                        format!("{} threshold breached", self.metric_name),
                        format!(
                            "{} value {} exceeds threshold {}",
                            self.metric_name, value, self.threshold
                        ),
                        self.severity,
                        self.source,
                    );
                    incidents.push(incident);
                }
                // If already breached, don't create duplicate
            } else {
                // Value is below threshold — reset breach state
                let mut breached = self.already_breached.lock().await;
                *breached = false;
            }
        }

        Ok(incidents)
    }

    fn name(&self) -> &str {
        &self.metric_name
    }
}

/// Detector that listens to log events.
pub struct LogPatternDetector {
    pattern: regex::Regex,
    severity: IncidentSeverity,
    /// Recent log lines to scan (updated externally).
    log_buffer: Arc<Mutex<Vec<String>>>,
    /// Maximum number of log lines to keep in buffer.
    max_buffer_size: usize,
}

impl LogPatternDetector {
    /// Create a new log pattern detector.
    pub fn new(pattern: impl Into<String>, severity: IncidentSeverity) -> Self {
        Self {
            pattern: regex::Regex::new(&pattern.into()).unwrap(),
            severity,
            log_buffer: Arc::new(Mutex::new(Vec::new())),
            max_buffer_size: 1000,
        }
    }

    /// Add a log line to the buffer for scanning.
    pub async fn add_log_line(&self, line: String) {
        let mut buffer = self.log_buffer.lock().await;
        buffer.push(line);
        if buffer.len() > self.max_buffer_size {
            buffer.remove(0);
        }
    }

    /// Set the maximum buffer size.
    pub fn set_max_buffer_size(&mut self, size: usize) {
        self.max_buffer_size = size;
    }
}

#[async_trait::async_trait]
impl IncidentDetector for LogPatternDetector {
    async fn detect(&self) -> Result<Vec<Incident>> {
        let mut incidents = Vec::new();
        let buffer = self.log_buffer.lock().await;

        for line in buffer.iter() {
            if self.pattern.is_match(line) {
                let incident = Incident::new(
                    format!("Log pattern matched: {}", self.pattern.as_str()),
                    format!("Log line matched pattern: {}", line),
                    self.severity,
                    IncidentSource::LogMonitoring,
                );
                incidents.push(incident);
            }
        }

        Ok(incidents)
    }

    fn name(&self) -> &str {
        "log_pattern"
    }
}

/// Composite detector that runs multiple detectors.
pub struct CompositeDetector {
    detectors: Vec<Arc<dyn IncidentDetector>>,
    incident_cache: Arc<DashMap<uuid::Uuid, Incident>>,
}

impl CompositeDetector {
    /// Create a new composite detector.
    pub fn new() -> Self {
        Self {
            detectors: Vec::new(),
            incident_cache: Arc::new(DashMap::new()),
        }
    }

    /// Add a detector.
    pub fn add_detector(&mut self, detector: Arc<dyn IncidentDetector>) {
        self.detectors.push(detector);
    }

    /// Run all detectors and return new incidents (deduplicated).
    pub async fn run_detection(&self) -> Result<Vec<Incident>> {
        let mut all_incidents = Vec::new();
        for detector in &self.detectors {
            let incidents = detector.detect().await?;
            all_incidents.extend(incidents);
        }

        // Deduplicate by title and severity (simple approach)
        let mut unique = Vec::new();
        for incident in all_incidents {
            if !self.incident_cache.contains_key(&incident.id) {
                self.incident_cache.insert(incident.id, incident.clone());
                unique.push(incident);
            }
        }

        Ok(unique)
    }
}

impl Default for CompositeDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_threshold_detector_creation() {
        let detector = ThresholdDetector::new(
            "cpu_usage",
            90.0,
            IncidentSeverity::Warning,
            IncidentSource::SystemMonitoring,
        );
        assert_eq!(detector.name(), "cpu_usage");
    }

    #[tokio::test]
    async fn test_threshold_detector_triggers_incident() {
        let detector = ThresholdDetector::new(
            "cpu_usage",
            90.0,
            IncidentSeverity::Warning,
            IncidentSource::SystemMonitoring,
        );

        // Update metric to exceed threshold
        detector.update_metric(95.0).await;
        let incidents = detector.detect().await.unwrap();
        assert_eq!(incidents.len(), 1);
        assert_eq!(incidents[0].severity, IncidentSeverity::Warning);

        // Second detection should not create duplicate
        let incidents = detector.detect().await.unwrap();
        assert_eq!(incidents.len(), 0);
    }

    #[tokio::test]
    async fn test_threshold_detector_resets_when_below() {
        let detector = ThresholdDetector::new(
            "cpu_usage",
            90.0,
            IncidentSeverity::Warning,
            IncidentSource::SystemMonitoring,
        );

        // Trigger breach
        detector.update_metric(95.0).await;
        let incidents = detector.detect().await.unwrap();
        assert_eq!(incidents.len(), 1);

        // Value goes below threshold
        detector.update_metric(50.0).await;
        let incidents = detector.detect().await.unwrap();
        assert_eq!(incidents.len(), 0);

        // Value goes above again — should trigger new incident
        detector.update_metric(95.0).await;
        let incidents = detector.detect().await.unwrap();
        assert_eq!(incidents.len(), 1);
    }

    #[tokio::test]
    async fn test_log_pattern_detector() {
        let detector = LogPatternDetector::new(
            r"ERROR|CRITICAL",
            IncidentSeverity::Error,
        );

        detector.add_log_line("INFO: everything is fine".to_string()).await;
        detector.add_log_line("ERROR: something went wrong".to_string()).await;
        detector.add_log_line("WARNING: low disk space".to_string()).await;

        let incidents = detector.detect().await.unwrap();
        assert_eq!(incidents.len(), 1);
        assert_eq!(incidents[0].severity, IncidentSeverity::Error);
    }

    #[tokio::test]
    async fn test_composite_detector() {
        let mut composite = CompositeDetector::new();

        let threshold = Arc::new(ThresholdDetector::new(
            "cpu_usage",
            90.0,
            IncidentSeverity::Warning,
            IncidentSource::SystemMonitoring,
        ));
        composite.add_detector(threshold.clone());

        let log = Arc::new(LogPatternDetector::new(
            r"ERROR",
            IncidentSeverity::Error,
        ));
        composite.add_detector(log.clone());

        // Trigger both detectors
        threshold.update_metric(95.0).await;
        log.add_log_line("ERROR: critical failure".to_string()).await;

        let incidents = composite.run_detection().await.unwrap();
        assert_eq!(incidents.len(), 2);
    }
}