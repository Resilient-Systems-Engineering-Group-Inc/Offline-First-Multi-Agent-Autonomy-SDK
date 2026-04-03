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
        }
    }
}

#[async_trait::async_trait]
impl IncidentDetector for ThresholdDetector {
    async fn detect(&self) -> Result<Vec<Incident>> {
        // In a real implementation, you would fetch the current metric value
        // from a metrics collector. For now, we return an empty list.
        Ok(Vec::new())
    }

    fn name(&self) -> &str {
        &self.metric_name
    }
}

/// Detector that listens to log events.
pub struct LogPatternDetector {
    pattern: regex::Regex,
    severity: IncidentSeverity,
}

impl LogPatternDetector {
    /// Create a new log pattern detector.
    pub fn new(pattern: impl Into<String>, severity: IncidentSeverity) -> Self {
        Self {
            pattern: regex::Regex::new(&pattern.into()).unwrap(),
            severity,
        }
    }
}

#[async_trait::async_trait]
impl IncidentDetector for LogPatternDetector {
    async fn detect(&self) -> Result<Vec<Incident>> {
        // In a real implementation, you would scan recent logs.
        Ok(Vec::new())
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
}