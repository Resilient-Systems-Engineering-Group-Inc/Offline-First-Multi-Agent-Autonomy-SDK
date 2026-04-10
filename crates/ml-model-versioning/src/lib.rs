//! ML Model Versioning System for Offline‑First Multi‑Agent Autonomy SDK.
//!
//! This crate provides a comprehensive system for versioning machine learning models
//! used in federated learning, reinforcement learning, and other distributed ML scenarios.
//!
//! # Features
//!
//! - **Model metadata management**: Store and retrieve model metadata (name, type, framework, etc.)
//! - **Version control**: Semantic versioning, changelogs, dependencies
//! - **Checksum validation**: SHA‑256 checksums for data integrity
//! - **Storage backends**: In‑memory, file‑system, distributed KV (optional)
//! - **Query system**: Filter and sort versions by metrics, framework, type, etc.
//! - **A/B testing**: Experimentation framework for comparing model versions
//! - **Integration**: Federated learning and RL planner integration (optional features)
//! - **Statistics**: Registry‑level statistics and analytics
//!
//! # Architecture
//!
//! The system is built around four core abstractions:
//!
//! 1. **`ModelStorage` trait**: Abstract storage backend (in‑memory, file, database, distributed KV)
//! 2. **`ModelVersioningManager`**: High‑level API for model/version operations
//! 3. **`ModelMetadata` / `ModelVersion`**: Core data structures
//! 4. **`ABTestingManager`**: A/B testing framework for comparing model versions
//!
//! # Examples
//!
//! ## Basic usage with in‑memory storage
//! ```
//! use ml_model_versioning::{
//!     ModelVersioningManager, ModelVersioningConfig,
//!     storage::InMemoryStorage,
//!     types::{ModelMetadata, CreateVersionRequest},
//! };
//! use chrono::Utc;
//! use std::collections::HashMap;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create storage and manager
//!     let storage = InMemoryStorage::new();
//!     let config = ModelVersioningConfig::default();
//!     let manager = ModelVersioningManager::new(storage, config);
//!
//!     // Create a model
//!     let metadata = ModelMetadata {
//!         id: "my-model".to_string(),
//!         name: "My Neural Network".to_string(),
//!         description: "A simple classifier".to_string(),
//!         model_type: "neural_network".to_string(),
//!         framework: "pytorch".to_string(),
//!         input_schema: None,
//!         output_schema: None,
//!         tags: vec!["classification".to_string(), "demo".to_string()],
//!         custom_metadata: HashMap::new(),
//!         created_at: Utc::now(),
//!         updated_at: Utc::now(),
//!     };
//!
//!     manager.create_model(metadata).await?;
//!
//!     // Register a version
//!     let request = CreateVersionRequest {
//!         model_id: "my-model".to_string(),
//!         version: "v1.0.0".to_string(),
//!         semver: Some(semver::Version::parse("1.0.0")?),
//!         changelog: "Initial release".to_string(),
//!         data: vec![0x01, 0x02, 0x03], // Actual model weights
//!         dependencies: Vec::new(),
//!         metrics: [("accuracy".to_string(), 0.95)].into_iter().collect(),
//!         hyperparameters: [("learning_rate".to_string(), serde_json::json!(0.001))].into_iter().collect(),
//!         training_data: None,
//!         set_as_default: true,
//!         author: "alice".to_string(),
//!     };
//!
//!     let version = manager.register_version(request).await?;
//!     println!("Registered version: {}", version.version);
//!
//!     // Retrieve the model
//!     let retrieved = manager.get_version("my-model", "v1.0.0").await?;
//!     println!("Accuracy: {}", retrieved.metrics.get("accuracy").unwrap());
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Integration with federated learning (optional feature `federated`)
//! ```ignore
//! use ml_model_versioning::manager::federated_integration::register_federated_update;
//! use federated_learning::ModelUpdate;
//!
//! async fn handle_federated_round(
//!     manager: &ModelVersioningManager<impl ModelStorage>,
//!     update: ModelUpdate,
//! ) -> Result<()> {
//!     register_federated_update(manager, "global-model", update, "federated-server").await?;
//!     Ok(())
//! }
//! ```

pub mod ab_testing;
pub mod error;
pub mod manager;
pub mod storage;
pub mod types;

// Re-export commonly used types
pub use error::{ModelVersioningError, Result};
pub use manager::{ModelVersioningConfig, ModelVersioningManager};
pub use storage::{InMemoryStorage, ModelStorage};
pub use types::{
    CreateVersionRequest, DataSplit, Dependency, MetricFilter, ModelBinary, ModelMetadata, ModelVersion,
    RegistryStats, SortField, TrainingDataInfo, UpdateVersionRequest, VersionQuery, VersionQueryResult,
};

// Re-export A/B testing types
pub use ab_testing::{
    ABExperimentConfig, ABTestingManager, ExperimentResult, Observation, StatisticalTest,
    TrafficAllocationStrategy, Variant,
};

/// Current version of the ML model versioning crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize the model versioning system.
pub fn init() {
    tracing::info!("ML Model Versioning v{} initialized", VERSION);
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_basic_workflow() {
        let storage = InMemoryStorage::new();
        let config = ModelVersioningConfig::default();
        let manager = ModelVersioningManager::new(storage, config);

        // Create model
        let metadata = ModelMetadata {
            id: "test-model".to_string(),
            name: "Test Model".to_string(),
            description: "Test".to_string(),
            model_type: "test".to_string(),
            framework: "test".to_string(),
            input_schema: None,
            output_schema: None,
            tags: vec![],
            custom_metadata: HashMap::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        manager.create_model(metadata).await.unwrap();

        // Create version
        let request = CreateVersionRequest {
            model_id: "test-model".to_string(),
            version: "v1".to_string(),
            semver: None,
            changelog: "Test version".to_string(),
            data: vec![1, 2, 3, 4, 5],
            dependencies: Vec::new(),
            metrics: HashMap::new(),
            hyperparameters: HashMap::new(),
            training_data: None,
            set_as_default: true,
            author: "test".to_string(),
        };

        let version = manager.register_version(request).await.unwrap();
        assert_eq!(version.model_id, "test-model");
        assert_eq!(version.version, "v1");
        assert!(version.is_default);

        // Retrieve version
        let retrieved = manager.get_version("test-model", "v1").await.unwrap();
        assert_eq!(retrieved.version, "v1");

        // Get default version
        let default = manager.get_default_version("test-model").await.unwrap();
        assert_eq!(default.version, "v1");

        // List versions
        let versions = manager.storage().list_versions("test-model").await.unwrap();
        assert_eq!(versions.len(), 1);

        // Get stats
        let stats = manager.get_stats().await.unwrap();
        assert_eq!(stats.total_models, 1);
        assert_eq!(stats.total_versions, 1);
    }

    #[tokio::test]
    async fn test_checksum_validation() {
        let storage = InMemoryStorage::new();
        let config = ModelVersioningConfig {
            validate_checksums: true,
            ..Default::default()
        };
        let manager = ModelVersioningManager::new(storage, config);

        let metadata = ModelMetadata {
            id: "checksum-model".to_string(),
            name: "Test".to_string(),
            description: "Test".to_string(),
            model_type: "test".to_string(),
            framework: "test".to_string(),
            input_schema: None,
            output_schema: None,
            tags: vec![],
            custom_metadata: HashMap::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        manager.create_model(metadata).await.unwrap();

        // Create version with valid data
        let request = CreateVersionRequest {
            model_id: "checksum-model".to_string(),
            version: "v1".to_string(),
            semver: None,
            changelog: "Test".to_string(),
            data: vec![1, 2, 3, 4, 5],
            dependencies: Vec::new(),
            metrics: HashMap::new(),
            hyperparameters: HashMap::new(),
            training_data: None,
            set_as_default: true,
            author: "test".to_string(),
        };

        let version = manager.register_version(request).await.unwrap();
        
        // Retrieve should succeed
        let retrieved = manager.get_version("checksum-model", "v1").await.unwrap();
        assert_eq!(retrieved.checksum, version.checksum);
    }
}