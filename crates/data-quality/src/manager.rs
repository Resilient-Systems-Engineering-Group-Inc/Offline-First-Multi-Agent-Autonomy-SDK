//! Data quality manager orchestrating validation, metrics, anomaly detection, and rules.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::{DataQualityError, Result};
use crate::validation::{Validator, ValidationResult};
use crate::metrics::{QualityMetrics, MetricCollector};
use crate::anomaly::{AnomalyDetector, Anomaly};
use crate::rules::{RuleEngine, RuleContext, RuleEvaluation};

/// Configuration for data quality manager.
#[derive(Debug, Clone)]
pub struct DataQualityConfig {
    /// Whether to enable validation.
    pub enable_validation: bool,
    /// Whether to enable metric collection.
    pub enable_metrics: bool,
    /// Whether to enable anomaly detection.
    pub enable_anomaly_detection: bool,
    /// Whether to enable rule evaluation.
    pub enable_rules: bool,
    /// Namespace for Prometheus metrics.
    pub metrics_namespace: String,
}

impl Default for DataQualityConfig {
    fn default() -> Self {
        Self {
            enable_validation: true,
            enable_metrics: true,
            enable_anomaly_detection: true,
            enable_rules: true,
            metrics_namespace: "data_quality".to_string(),
        }
    }
}

/// Data quality manager.
pub struct DataQualityManager {
    config: DataQualityConfig,
    validator: Option<Validator>,
    metric_collector: Option<MetricCollector>,
    anomaly_detectors: HashMap<String, Box<dyn AnomalyDetector>>,
    rule_engine: Option<RuleEngine>,
    /// Latest quality metrics snapshot.
    latest_metrics: Arc<RwLock<QualityMetrics>>,
}

impl DataQualityManager {
    /// Create a new data quality manager with default configuration.
    pub fn new() -> Result<Self> {
        Self::with_config(DataQualityConfig::default())
    }

    /// Create a new data quality manager with custom configuration.
    pub fn with_config(config: DataQualityConfig) -> Result<Self> {
        let metric_collector = if config.enable_metrics {
            Some(MetricCollector::new(&config.metrics_namespace)?)
        } else {
            None
        };

        Ok(Self {
            config,
            validator: None,
            metric_collector,
            anomaly_detectors: HashMap::new(),
            rule_engine: None,
            latest_metrics: Arc::new(RwLock::new(QualityMetrics::new())),
        })
    }

    /// Set the validator.
    pub fn set_validator(&mut self, validator: Validator) {
        self.validator = Some(validator);
    }

    /// Add an anomaly detector for a specific metric.
    pub fn add_anomaly_detector(&mut self, metric_name: impl Into<String>, detector: Box<dyn AnomalyDetector>) {
        self.anomaly_detectors.insert(metric_name.into(), detector);
    }

    /// Set the rule engine.
    pub fn set_rule_engine(&mut self, rule_engine: RuleEngine) {
        self.rule_engine = Some(rule_engine);
    }

    /// Process a data record (JSON object).
    pub async fn process_record(&mut self, record: &serde_json::Value) -> Result<ProcessingResult> {
        let mut validation_results = Vec::new();
        let mut anomalies = Vec::new();
        let mut rule_evaluations = Vec::new();

        // Validation
        if self.config.enable_validation {
            if let Some(validator) = &self.validator {
                validation_results.extend(validator.validate(record)?);
            }
        }

        // Update metrics
        if let Some(collector) = &self.metric_collector {
            let all_passed = validation_results.iter().all(|r| r.passed);
            if all_passed {
                collector.record_valid();
            } else {
                collector.record_invalid();
            }
            // Example completeness: assume 100% if valid, else 0%
            collector.set_completeness(if all_passed { 100.0 } else { 0.0 });
        }

        // Anomaly detection on numeric fields
        if self.config.enable_anomaly_detection {
            if let Some(obj) = record.as_object() {
                for (field, value) in obj {
                    if let Some(detector) = self.anomaly_detectors.get_mut(field) {
                        if let Some(num) = value.as_f64() {
                            let timestamp = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs();
                            anomalies.extend(detector.process(num, timestamp));
                        }
                    }
                }
            }
        }

        // Rule evaluation
        if self.config.enable_rules {
            if let Some(engine) = &self.rule_engine {
                let mut context = RuleContext::new();
                context.add_validation_results(validation_results.clone());
                context.add_anomalies(anomalies.clone());
                // Add some metrics
                if let Some(collector) = &self.metric_collector {
                    // In a real implementation, you'd fetch current metrics.
                }
                rule_evaluations = engine.evaluate(&context);
            }
        }

        // Update latest metrics snapshot
        let mut metrics = QualityMetrics::new();
        metrics.add_metric("valid_records", validation_results.iter().filter(|r| r.passed).count() as f64);
        metrics.add_metric("invalid_records", validation_results.iter().filter(|r| !r.passed).count() as f64);
        metrics.add_metric("anomalies", anomalies.len() as f64);
        *self.latest_metrics.write().await = metrics;

        Ok(ProcessingResult {
            validation_results,
            anomalies,
            rule_evaluations,
        })
    }

    /// Get the latest quality metrics.
    pub async fn get_metrics(&self) -> QualityMetrics {
        self.latest_metrics.read().await.clone()
    }

    /// Export Prometheus metrics.
    pub fn export_prometheus(&self) -> Result<Option<String>> {
        match &self.metric_collector {
            Some(collector) => Ok(Some(collector.export()?)),
            None => Ok(None),
        }
    }
}

/// Result of processing a record.
#[derive(Debug, Clone)]
pub struct ProcessingResult {
    /// Validation results.
    pub validation_results: Vec<ValidationResult>,
    /// Anomalies detected.
    pub anomalies: Vec<Anomaly>,
    /// Rule evaluations.
    pub rule_evaluations: Vec<RuleEvaluation>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::{FieldValidator, ValidationRule};

    #[tokio::test]
    async fn test_manager_basic() -> Result<()> {
        let config = DataQualityConfig {
            enable_validation: true,
            enable_metrics: false,
            enable_anomaly_detection: false,
            enable_rules: false,
            ..Default::default()
        };
        let mut manager = DataQualityManager::with_config(config)?;
        let mut validator = Validator::new();
        let field_validator = FieldValidator::new("temperature")
            .with_rule(ValidationRule::Range { min: -50.0, max: 50.0 });
        validator.add_field_validator(field_validator);
        manager.set_validator(validator);

        let record = serde_json::json!({ "temperature": 25.0 });
        let result = manager.process_record(&record).await?;
        assert!(result.validation_results[0].passed);
        Ok(())
    }
}