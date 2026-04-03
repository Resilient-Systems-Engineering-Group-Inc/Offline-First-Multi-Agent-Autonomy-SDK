//! Anomaly detection for data quality.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use statrs::statistics::Statistics;

use crate::error::{DataQualityError, Result};

/// Type of anomaly.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AnomalyType {
    /// Value is outside expected range.
    OutOfRange,
    /// Sudden spike or drop.
    Spike,
    /// Missing data point.
    Missing,
    /// Unusual pattern (e.g., seasonality broken).
    Pattern,
    /// Custom anomaly.
    Custom(String),
}

/// Anomaly detection result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    /// Type of anomaly.
    pub anomaly_type: AnomalyType,
    /// Description.
    pub description: String,
    /// Severity score (0‑1, higher is more severe).
    pub severity: f64,
    /// Timestamp of detection.
    pub timestamp: u64,
    /// Additional metadata.
    pub metadata: serde_json::Value,
}

/// Trait for anomaly detectors.
pub trait AnomalyDetector: Send + Sync {
    /// Process a new data point and return anomalies if any.
    fn process(&mut self, value: f64, timestamp: u64) -> Vec<Anomaly>;

    /// Reset detector state.
    fn reset(&mut self);
}

/// Simple threshold‑based detector.
pub struct ThresholdDetector {
    lower: f64,
    upper: f64,
}

impl ThresholdDetector {
    /// Create a new detector with lower and upper bounds.
    pub fn new(lower: f64, upper: f64) -> Self {
        Self { lower, upper }
    }
}

impl AnomalyDetector for ThresholdDetector {
    fn process(&mut self, value: f64, timestamp: u64) -> Vec<Anomaly> {
        if value < self.lower || value > self.upper {
            vec![Anomaly {
                anomaly_type: AnomalyType::OutOfRange,
                description: format!("Value {} outside [{}, {}]", value, self.lower, self.upper),
                severity: 0.8,
                timestamp,
                metadata: serde_json::json!({ "value": value, "lower": self.lower, "upper": self.upper }),
            }]
        } else {
            vec![]
        }
    }

    fn reset(&mut self) {
        // No state to reset
    }
}

/// Moving average based spike detector.
pub struct SpikeDetector {
    window: VecDeque<f64>,
    window_size: usize,
    threshold: f64, // number of standard deviations
}

impl SpikeDetector {
    /// Create a new spike detector.
    pub fn new(window_size: usize, threshold: f64) -> Self {
        Self {
            window: VecDeque::with_capacity(window_size),
            window_size,
            threshold,
        }
    }

    fn mean_std(&self) -> Option<(f64, f64)> {
        if self.window.len() < 2 {
            return None;
        }
        let mean = self.window.iter().sum::<f64>() / self.window.len() as f64;
        let variance = self.window.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / (self.window.len() - 1) as f64;
        Some((mean, variance.sqrt()))
    }
}

impl AnomalyDetector for SpikeDetector {
    fn process(&mut self, value: f64, timestamp: u64) -> Vec<Anomaly> {
        let mut anomalies = Vec::new();
        if let Some((mean, std)) = self.mean_std() {
            let z = (value - mean).abs() / std;
            if z > self.threshold {
                anomalies.push(Anomaly {
                    anomaly_type: AnomalyType::Spike,
                    description: format!("Spike detected: z‑score = {:.2}", z),
                    severity: z.min(1.0),
                    timestamp,
                    metadata: serde_json::json!({ "value": value, "mean": mean, "std": std, "z": z }),
                });
            }
        }
        self.window.push_back(value);
        if self.window.len() > self.window_size {
            self.window.pop_front();
        }
        anomalies
    }

    fn reset(&mut self) {
        self.window.clear();
    }
}

/// Missing data detector.
pub struct MissingDetector {
    expected_interval: u64, // seconds
    last_timestamp: Option<u64>,
}

impl MissingDetector {
    /// Create a new missing detector.
    pub fn new(expected_interval: u64) -> Self {
        Self {
            expected_interval,
            last_timestamp: None,
        }
    }
}

impl AnomalyDetector for MissingDetector {
    fn process(&mut self, value: f64, timestamp: u64) -> Vec<Anomaly> {
        let mut anomalies = Vec::new();
        if let Some(last) = self.last_timestamp {
            let gap = timestamp - last;
            if gap > self.expected_interval * 2 {
                anomalies.push(Anomaly {
                    anomaly_type: AnomalyType::Missing,
                    description: format!("Missing data: gap {} seconds", gap),
                    severity: 0.5,
                    timestamp,
                    metadata: serde_json::json!({ "gap": gap, "expected": self.expected_interval }),
                });
            }
        }
        self.last_timestamp = Some(timestamp);
        anomalies
    }

    fn reset(&mut self) {
        self.last_timestamp = None;
    }
}

/// Composite detector that runs multiple detectors.
pub struct CompositeDetector {
    detectors: Vec<Box<dyn AnomalyDetector>>,
}

impl CompositeDetector {
    /// Create a new composite detector.
    pub fn new() -> Self {
        Self {
            detectors: Vec::new(),
        }
    }

    /// Add a detector.
    pub fn add_detector(&mut self, detector: Box<dyn AnomalyDetector>) {
        self.detectors.push(detector);
    }
}

impl AnomalyDetector for CompositeDetector {
    fn process(&mut self, value: f64, timestamp: u64) -> Vec<Anomaly> {
        let mut anomalies = Vec::new();
        for detector in &mut self.detectors {
            anomalies.extend(detector.process(value, timestamp));
        }
        anomalies
    }

    fn reset(&mut self) {
        for detector in &mut self.detectors {
            detector.reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_threshold_detector() {
        let mut detector = ThresholdDetector::new(0.0, 10.0);
        let anomalies = detector.process(15.0, 1000);
        assert_eq!(anomalies.len(), 1);
        assert_eq!(anomalies[0].anomaly_type, AnomalyType::OutOfRange);
    }

    #[test]
    fn test_spike_detector() {
        let mut detector = SpikeDetector::new(5, 2.0);
        detector.process(1.0, 1000);
        detector.process(1.1, 1001);
        detector.process(1.2, 1002);
        detector.process(1.3, 1003);
        detector.process(1.4, 1004);
        let anomalies = detector.process(10.0, 1005); // spike
        assert!(!anomalies.is_empty());
    }
}