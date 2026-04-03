//! Data quality management system for offline‑first multi‑agent systems.
//!
//! Provides validation, quality metrics, anomaly detection, and rule‑based
//! quality assessment for data flowing through the agent network.

pub mod error;
pub mod validation;
pub mod metrics;
pub mod anomaly;
pub mod rules;
pub mod manager;

pub use error::DataQualityError;
pub use validation::{Validator, ValidationRule, ValidationResult};
pub use metrics::{QualityMetrics, QualityMetric, MetricCollector};
pub use anomaly::{AnomalyDetector, AnomalyType};
pub use rules::{QualityRule, RuleEngine};
pub use manager::DataQualityManager;

/// Re‑export of common types.
pub mod prelude {
    pub use super::{
        DataQualityError,
        Validator,
        ValidationRule,
        ValidationResult,
        QualityMetrics,
        QualityMetric,
        MetricCollector,
        AnomalyDetector,
        AnomalyType,
        QualityRule,
        RuleEngine,
        DataQualityManager,
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}