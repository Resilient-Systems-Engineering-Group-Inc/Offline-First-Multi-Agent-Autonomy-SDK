//! High-level manager for ML model versioning.

use std::sync::Arc;

use async_trait::async_trait;

use crate::error::{ModelVersioningError, Result};
use crate::storage::ModelStorage;
use crate::types::{
    CreateVersionRequest, ModelBinary, ModelMetadata, ModelVersion, RegistryStats, UpdateVersionRequest,
    VersionQuery, VersionQueryResult,
};

/// Configuration for the model versioning manager.
#[derive(Debug, Clone)]
pub struct ModelVersioningConfig {
    /// Whether to validate checksums on retrieval.
    pub validate_checksums: bool,
    /// Whether to allow overwriting existing versions.
    pub allow_overwrite: bool,
    /// Default storage location prefix.
    pub default_storage_prefix: String,
    /// Maximum model size in bytes.
    pub max_model_size_bytes: Option<u64>,
}

impl Default for ModelVersioningConfig {
    fn default() -> Self {
        Self {
            validate_checksums: true,
            allow_overwrite: false,
            default_storage_prefix: "models/".to_string(),
            max_model_size_bytes: Some(1024 * 1024 * 1024), // 1 GB
        }
    }
}

/// High-level manager for ML model versioning.
pub struct ModelVersioningManager<S: ModelStorage> {
    storage: Arc<S>,
    config: ModelVersioningConfig,
}

impl<S: ModelStorage> ModelVersioningManager<S> {
    /// Create a new manager with the given storage backend and configuration.
    pub fn new(storage: S, config: ModelVersioningConfig) -> Self {
        Self {
            storage: Arc::new(storage),
            config,
        }
    }

    /// Get a reference to the underlying storage.
    pub fn storage(&self) -> &Arc<S> {
        &self.storage
    }

    /// Create a new model with metadata.
    pub async fn create_model(&self, metadata: ModelMetadata) -> Result<()> {
        // Validate metadata
        if metadata.id.is_empty() {
            return Err(ModelVersioningError::InvalidMetadata(
                "Model ID cannot be empty".to_string(),
            ));
        }

        self.storage.create_model(metadata).await
    }

    /// Register a new version of a model.
    pub async fn register_version(&self, request: CreateVersionRequest) -> Result<ModelVersion> {
        // Validate model size
        if let Some(max_size) = self.config.max_model_size_bytes {
            if request.data.len() as u64 > max_size {
                return Err(ModelVersioningError::InvalidMetadata(format!(
                    "Model size {} bytes exceeds maximum {} bytes",
                    request.data.len(),
                    max_size
                )));
            }
        }

        // Create version in storage
        let version = self.storage.create_version(request).await?;

        // If checksum validation is enabled, verify immediately
        if self.config.validate_checksums {
            let binary = self.storage.retrieve_binary(&version.model_id, &version.version).await?;
            if !binary.verify_checksum() {
                // Delete the invalid version
                let _ = self.storage.delete_version(&version.model_id, &version.version).await;
                return Err(ModelVersioningError::ChecksumMismatch(
                    version.checksum.clone(),
                    binary.compute_checksum(),
                ));
            }
        }

        Ok(version)
    }

    /// Get a model version by ID.
    pub async fn get_version(&self, model_id: &str, version_id: &str) -> Result<ModelVersion> {
        let version = self.storage.get_version(model_id, version_id).await?;
        
        // Validate checksum if enabled
        if self.config.validate_checksums && !version.is_deprecated {
            let binary = self.storage.retrieve_binary(model_id, version_id).await?;
            if !binary.verify_checksum() {
                return Err(ModelVersioningError::ChecksumMismatch(
                    version.checksum.clone(),
                    binary.compute_checksum(),
                ));
            }
        }
        
        Ok(version)
    }

    /// Get the default version of a model.
    pub async fn get_default_version(&self, model_id: &str) -> Result<ModelVersion> {
        self.storage.get_default_version(model_id).await
    }

    /// Retrieve model binary with validation.
    pub async fn retrieve_model(&self, model_id: &str, version_id: &str) -> Result<ModelBinary> {
        let binary = self.storage.retrieve_binary(model_id, version_id).await?;
        
        if self.config.validate_checksums && !binary.verify_checksum() {
            return Err(ModelVersioningError::ChecksumMismatch(
                binary.metadata.checksum.clone(),
                binary.compute_checksum(),
            ));
        }
        
        Ok(binary)
    }

    /// Query versions with advanced filtering.
    pub async fn query_versions(&self, query: VersionQuery) -> Result<VersionQueryResult> {
        self.storage.query_versions(query).await
    }

    /// Update version metadata.
    pub async fn update_version(
        &self,
        model_id: &str,
        version_id: &str,
        update: UpdateVersionRequest,
    ) -> Result<ModelVersion> {
        self.storage.update_version(model_id, version_id, update).await
    }

    /// Deprecate a version.
    pub async fn deprecate_version(&self, model_id: &str, version_id: &str) -> Result<ModelVersion> {
        let update = UpdateVersionRequest {
            changelog: None,
            dependencies: None,
            metrics: None,
            deprecate: Some(true),
            set_as_default: Some(false),
        };
        self.update_version(model_id, version_id, update).await
    }

    /// Promote a version to be the default.
    pub async fn promote_to_default(&self, model_id: &str, version_id: &str) -> Result<ModelVersion> {
        let update = UpdateVersionRequest {
            changelog: None,
            dependencies: None,
            metrics: None,
            deprecate: None,
            set_as_default: Some(true),
        };
        self.update_version(model_id, version_id, update).await
    }

    /// Compare two versions of a model.
    pub async fn compare_versions(
        &self,
        model_id: &str,
        version_a: &str,
        version_b: &str,
    ) -> Result<VersionComparison> {
        let version_a_meta = self.get_version(model_id, version_a).await?;
        let version_b_meta = self.get_version(model_id, version_b).await?;

        Ok(VersionComparison {
            model_id: model_id.to_string(),
            version_a: version_a_meta,
            version_b: version_b_meta,
            differences: Vec::new(), // In a real implementation, compute actual differences
        })
    }

    /// Get registry statistics.
    pub async fn get_stats(&self) -> Result<RegistryStats> {
        self.storage.get_stats().await
    }

    /// Export model registry to a portable format.
    pub async fn export_registry(&self, format: ExportFormat) -> Result<Vec<u8>> {
        // In a real implementation, serialize all models and versions
        match format {
            ExportFormat::Json => Ok(serde_json::to_vec(&"Export not implemented".to_string())?),
            ExportFormat::Cbor => Ok(serde_cbor::to_vec(&"Export not implemented".to_string())?),
        }
    }

    /// Import model registry from a portable format.
    pub async fn import_registry(&self, _data: &[u8], _format: ExportFormat) -> Result<()> {
        // In a real implementation, deserialize and import
        Ok(())
    }
}

/// Result of comparing two model versions.
#[derive(Debug, Clone)]
pub struct VersionComparison {
    /// Model ID.
    pub model_id: String,
    /// First version metadata.
    pub version_a: ModelVersion,
    /// Second version metadata.
    pub version_b: ModelVersion,
    /// List of differences between versions.
    pub differences: Vec<VersionDifference>,
}

/// A difference between two model versions.
#[derive(Debug, Clone)]
pub enum VersionDifference {
    /// Different checksums.
    ChecksumChanged { old: String, new: String },
    /// Different size.
    SizeChanged { old: u64, new: u64 },
    /// Different metrics.
    MetricChanged { name: String, old: f64, new: f64 },
    /// Different dependencies.
    DependenciesChanged,
    /// Different hyperparameters.
    HyperparametersChanged,
}

/// Export formats for the model registry.
#[derive(Debug, Clone, Copy)]
pub enum ExportFormat {
    /// JSON format.
    Json,
    /// CBOR format (binary).
    Cbor,
}

/// Integration with federated learning.
#[cfg(feature = "federated")]
pub mod federated_integration {
    use super::*;
    use crate::federated_learning::ModelUpdate;

    /// Register a federated learning model update as a new version.
    pub async fn register_federated_update(
        manager: &ModelVersioningManager<impl ModelStorage>,
        model_id: &str,
        update: ModelUpdate,
        author: &str,
    ) -> Result<ModelVersion> {
        let request = CreateVersionRequest {
            model_id: model_id.to_string(),
            version: format!("federated-{}", update.round),
            semver: None,
            changelog: format!("Federated learning round {}", update.round),
            data: update.model_data,
            dependencies: Vec::new(),
            metrics: update.metrics,
            hyperparameters: update.hyperparameters,
            training_data: None,
            set_as_default: true,
            author: author.to_string(),
        };

        manager.register_version(request).await
    }
}

/// Integration with RL planner.
#[cfg(feature = "rl")]
pub mod rl_integration {
    use super::*;
    use crate::rl_planner::Policy;

    /// Save an RL policy as a model version.
    pub async fn save_policy(
        manager: &ModelVersioningManager<impl ModelStorage>,
        model_id: &str,
        policy: &Policy,
        metrics: HashMap<String, f64>,
        author: &str,
    ) -> Result<ModelVersion> {
        let serialized = serde_json::to_vec(policy)
            .map_err(|e| ModelVersioningError::SerializationError(e.to_string()))?;

        let request = CreateVersionRequest {
            model_id: model_id.to_string(),
            version: format!("policy-{}", chrono::Utc::now().format("%Y%m%d-%H%M%S")),
            semver: None,
            changelog: "RL policy snapshot".to_string(),
            data: serialized,
            dependencies: Vec::new(),
            metrics,
            hyperparameters: HashMap::new(),
            training_data: None,
            set_as_default: true,
            author: author.to_string(),
        };

        manager.register_version(request).await
    }

    /// Load an RL policy from a model version.
    pub async fn load_policy(
        manager: &ModelVersioningManager<impl ModelStorage>,
        model_id: &str,
        version_id: Option<&str>,
    ) -> Result<Policy> {
        let version_id = match version_id {
            Some(v) => v.to_string(),
            None => manager.get_default_version(model_id).await?.version,
        };

        let binary = manager.retrieve_model(model_id, &version_id).await?;
        let policy: Policy = serde_json::from_slice(&binary.data)
            .map_err(|e| ModelVersioningError::SerializationError(e.to_string()))?;

        Ok(policy)
    }
}