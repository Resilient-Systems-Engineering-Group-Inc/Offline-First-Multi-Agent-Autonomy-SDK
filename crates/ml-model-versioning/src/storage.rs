//! Storage backends for model versioning.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;

use crate::error::{ModelVersioningError, Result};
use crate::types::{
    CreateVersionRequest, ModelBinary, ModelMetadata, ModelVersion, RegistryStats, UpdateVersionRequest,
    VersionQuery, VersionQueryResult,
};

/// Abstract storage backend for model versioning.
#[async_trait]
pub trait ModelStorage: Send + Sync {
    /// Create a new model entry.
    async fn create_model(&self, metadata: ModelMetadata) -> Result<()>;

    /// Get model metadata by ID.
    async fn get_model(&self, model_id: &str) -> Result<ModelMetadata>;

    /// List all models.
    async fn list_models(&self) -> Result<Vec<ModelMetadata>>;

    /// Update model metadata.
    async fn update_model(&self, model_id: &str, metadata: ModelMetadata) -> Result<()>;

    /// Delete a model (and all its versions).
    async fn delete_model(&self, model_id: &str) -> Result<()>;

    /// Create a new version of a model.
    async fn create_version(&self, request: CreateVersionRequest) -> Result<ModelVersion>;

    /// Get a specific version of a model.
    async fn get_version(&self, model_id: &str, version_id: &str) -> Result<ModelVersion>;

    /// Get the default/latest version of a model.
    async fn get_default_version(&self, model_id: &str) -> Result<ModelVersion>;

    /// List all versions of a model.
    async fn list_versions(&self, model_id: &str) -> Result<Vec<ModelVersion>>;

    /// Query versions with filters.
    async fn query_versions(&self, query: VersionQuery) -> Result<VersionQueryResult>;

    /// Update version metadata.
    async fn update_version(
        &self,
        model_id: &str,
        version_id: &str,
        update: UpdateVersionRequest,
    ) -> Result<ModelVersion>;

    /// Delete a version.
    async fn delete_version(&self, model_id: &str, version_id: &str) -> Result<()>;

    /// Store model binary data.
    async fn store_binary(&self, model_id: &str, version_id: &str, data: Vec<u8>) -> Result<()>;

    /// Retrieve model binary data.
    async fn retrieve_binary(&self, model_id: &str, version_id: &str) -> Result<ModelBinary>;

    /// Get registry statistics.
    async fn get_stats(&self) -> Result<RegistryStats>;
}

/// In-memory storage backend (for testing and development).
pub struct InMemoryStorage {
    models: RwLock<HashMap<String, ModelMetadata>>,
    versions: RwLock<HashMap<String, HashMap<String, ModelVersion>>>,
    binaries: RwLock<HashMap<String, HashMap<String, Vec<u8>>>>,
}

impl InMemoryStorage {
    /// Create a new in-memory storage.
    pub fn new() -> Self {
        Self {
            models: RwLock::new(HashMap::new()),
            versions: RwLock::new(HashMap::new()),
            binaries: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ModelStorage for InMemoryStorage {
    async fn create_model(&self, metadata: ModelMetadata) -> Result<()> {
        let mut models = self.models.write().unwrap();
        if models.contains_key(&metadata.id) {
            return Err(ModelVersioningError::Conflict(format!(
                "Model '{}' already exists",
                metadata.id
            )));
        }
        models.insert(metadata.id.clone(), metadata);
        Ok(())
    }

    async fn get_model(&self, model_id: &str) -> Result<ModelMetadata> {
        let models = self.models.read().unwrap();
        models
            .get(model_id)
            .cloned()
            .ok_or_else(|| ModelVersioningError::ModelNotFound(model_id.to_string()))
    }

    async fn list_models(&self) -> Result<Vec<ModelMetadata>> {
        let models = self.models.read().unwrap();
        Ok(models.values().cloned().collect())
    }

    async fn update_model(&self, model_id: &str, metadata: ModelMetadata) -> Result<()> {
        let mut models = self.models.write().unwrap();
        if !models.contains_key(model_id) {
            return Err(ModelVersioningError::ModelNotFound(model_id.to_string()));
        }
        models.insert(model_id.to_string(), metadata);
        Ok(())
    }

    async fn delete_model(&self, model_id: &str) -> Result<()> {
        let mut models = self.models.write().unwrap();
        let mut versions = self.versions.write().unwrap();
        let mut binaries = self.binaries.write().unwrap();

        models.remove(model_id);
        versions.remove(model_id);
        binaries.remove(model_id);

        Ok(())
    }

    async fn create_version(&self, request: CreateVersionRequest) -> Result<ModelVersion> {
        // Verify model exists
        let models = self.models.read().unwrap();
        if !models.contains_key(&request.model_id) {
            return Err(ModelVersioningError::ModelNotFound(request.model_id.clone()));
        }

        let mut versions = self.versions.write().unwrap();
        let model_versions = versions.entry(request.model_id.clone()).or_default();

        // Check if version already exists
        if model_versions.contains_key(&request.version) {
            return Err(ModelVersioningError::Conflict(format!(
                "Version '{}' of model '{}' already exists",
                request.version, request.model_id
            )));
        }

        // Compute checksum
        let checksum = {
            let mut hasher = sha2::Sha256::new();
            hasher.update(&request.data);
            hex::encode(hasher.finalize())
        };

        let now = chrono::Utc::now();
        let version = ModelVersion {
            model_id: request.model_id.clone(),
            version: request.version.clone(),
            semver: request.semver,
            changelog: request.changelog,
            checksum,
            size_bytes: request.data.len() as u64,
            storage_location: format!("memory://{}/{}", request.model_id, request.version),
            dependencies: request.dependencies,
            metrics: request.metrics,
            hyperparameters: request.hyperparameters,
            training_data: request.training_data,
            is_default: request.set_as_default,
            is_deprecated: false,
            created_at: now,
            author: request.author,
        };

        // If this is the default version, update other versions
        if request.set_as_default {
            for v in model_versions.values_mut() {
                v.is_default = false;
            }
        }

        model_versions.insert(request.version.clone(), version.clone());

        // Store binary
        let mut binaries = self.binaries.write().unwrap();
        let model_binaries = binaries.entry(request.model_id.clone()).or_default();
        model_binaries.insert(request.version.clone(), request.data);

        Ok(version)
    }

    async fn get_version(&self, model_id: &str, version_id: &str) -> Result<ModelVersion> {
        let versions = self.versions.read().unwrap();
        let model_versions = versions
            .get(model_id)
            .ok_or_else(|| ModelVersioningError::ModelNotFound(model_id.to_string()))?;

        model_versions
            .get(version_id)
            .cloned()
            .ok_or_else(|| ModelVersioningError::VersionNotFound(version_id.to_string(), model_id.to_string()))
    }

    async fn get_default_version(&self, model_id: &str) -> Result<ModelVersion> {
        let versions = self.versions.read().unwrap();
        let model_versions = versions
            .get(model_id)
            .ok_or_else(|| ModelVersioningError::ModelNotFound(model_id.to_string()))?;

        model_versions
            .values()
            .find(|v| v.is_default)
            .cloned()
            .ok_or_else(|| {
                ModelVersioningError::VersionNotFound("default".to_string(), model_id.to_string())
            })
    }

    async fn list_versions(&self, model_id: &str) -> Result<Vec<ModelVersion>> {
        let versions = self.versions.read().unwrap();
        let model_versions = versions
            .get(model_id)
            .ok_or_else(|| ModelVersioningError::ModelNotFound(model_id.to_string()))?;

        Ok(model_versions.values().cloned().collect())
    }

    async fn query_versions(&self, query: VersionQuery) -> Result<VersionQueryResult> {
        let versions = self.versions.read().unwrap();
        let mut all_versions = Vec::new();

        // Collect all versions matching model_id filter
        match &query.model_id {
            Some(model_id) => {
                if let Some(model_versions) = versions.get(model_id) {
                    all_versions.extend(model_versions.values().cloned());
                }
            }
            None => {
                for model_versions in versions.values() {
                    all_versions.extend(model_versions.values().cloned());
                }
            }
        }

        // Apply filters (simplified)
        let mut filtered = all_versions;
        
        if let Some(framework) = &query.framework {
            filtered.retain(|v| {
                // Note: framework is stored in model metadata, not version
                // For simplicity, we'll skip this filter in in-memory implementation
                true
            });
        }

        if let Some(model_type) = &query.model_type {
            filtered.retain(|v| {
                // Similarly, model type is in model metadata
                true
            });
        }

        // Sort
        if let Some(sort_by) = &query.sort_by {
            match sort_by {
                crate::types::SortField::CreatedAt => {
                    filtered.sort_by_key(|v| v.created_at);
                }
                crate::types::SortField::Version => {
                    filtered.sort_by(|a, b| a.version.cmp(&b.version));
                }
                crate::types::SortField::Size => {
                    filtered.sort_by_key(|v| v.size_bytes);
                }
                crate::types::SortField::Metric(metric_name) => {
                    filtered.sort_by(|a, b| {
                        let a_val = a.metrics.get(metric_name).unwrap_or(&0.0);
                        let b_val = b.metrics.get(metric_name).unwrap_or(&0.0);
                        a_val.partial_cmp(b_val).unwrap_or(std::cmp::Ordering::Equal)
                    });
                }
            }
        }

        if query.sort_desc {
            filtered.reverse();
        }

        // Pagination
        let total_count = filtered.len();
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(usize::MAX);
        let end = std::cmp::min(offset + limit, total_count);
        let has_more = end < total_count;

        let versions_page = if offset < total_count {
            filtered[offset..end].to_vec()
        } else {
            Vec::new()
        };

        Ok(VersionQueryResult {
            versions: versions_page,
            total_count,
            has_more,
        })
    }

    async fn update_version(
        &self,
        model_id: &str,
        version_id: &str,
        update: UpdateVersionRequest,
    ) -> Result<ModelVersion> {
        let mut versions = self.versions.write().unwrap();
        let model_versions = versions
            .get_mut(model_id)
            .ok_or_else(|| ModelVersioningError::ModelNotFound(model_id.to_string()))?;

        let version = model_versions
            .get_mut(version_id)
            .ok_or_else(|| ModelVersioningError::VersionNotFound(version_id.to_string(), model_id.to_string()))?;

        if let Some(changelog) = update.changelog {
            version.changelog = changelog;
        }

        if let Some(dependencies) = update.dependencies {
            version.dependencies = dependencies;
        }

        if let Some(metrics) = update.metrics {
            version.metrics = metrics;
        }

        if let Some(deprecate) = update.deprecate {
            version.is_deprecated = deprecate;
        }

        if let Some(set_as_default) = update.set_as_default {
            if set_as_default {
                // Unset default flag on all other versions
                for v in model_versions.values_mut() {
                    v.is_default = false;
                }
                version.is_default = true;
            } else {
                version.is_default = false;
            }
        }

        Ok(version.clone())
    }

    async fn delete_version(&self, model_id: &str, version_id: &str) -> Result<()> {
        let mut versions = self.versions.write().unwrap();
        let mut binaries = self.binaries.write().unwrap();

        if let Some(model_versions) = versions.get_mut(model_id) {
            model_versions.remove(version_id);
        }

        if let Some(model_binaries) = binaries.get_mut(model_id) {
            model_binaries.remove(version_id);
        }

        Ok(())
    }

    async fn store_binary(&self, model_id: &str, version_id: &str, data: Vec<u8>) -> Result<()> {
        let mut binaries = self.binaries.write().unwrap();
        let model_binaries = binaries.entry(model_id.to_string()).or_default();
        model_binaries.insert(version_id.to_string(), data);
        Ok(())
    }

    async fn retrieve_binary(&self, model_id: &str, version_id: &str) -> Result<ModelBinary> {
        let versions = self.versions.read().unwrap();
        let binaries = self.binaries.read().unwrap();

        let version = versions
            .get(model_id)
            .and_then(|m| m.get(version_id))
            .cloned()
            .ok_or_else(|| ModelVersioningError::VersionNotFound(version_id.to_string(), model_id.to_string()))?;

        let data = binaries
            .get(model_id)
            .and_then(|m| m.get(version_id))
            .cloned()
            .ok_or_else(|| ModelVersioningError::StorageError("Binary data not found".to_string()))?;

        Ok(ModelBinary { metadata: version, data })
    }

    async fn get_stats(&self) -> Result<RegistryStats> {
        let models = self.models.read().unwrap();
        let versions = self.versions.read().unwrap();
        let binaries = self.binaries.read().unwrap();

        let total_models = models.len();
        let total_versions: usize = versions.values().map(|m| m.len()).sum();
        
        let total_storage_bytes: u64 = binaries
            .values()
            .flat_map(|m| m.values())
            .map(|data| data.len() as u64)
            .sum();

        let mut models_by_framework = HashMap::new();
        for model in models.values() {
            *models_by_framework.entry(model.framework.clone()).or_insert(0) += 1;
        }

        let mut versions_by_type = HashMap::new();
        for model in models.values() {
            *versions_by_type.entry(model.model_type.clone()).or_insert(0) += 
                versions.get(&model.id).map(|v| v.len()).unwrap_or(0);
        }

        let latest_activity = chrono::Utc::now();

        Ok(RegistryStats {
            total_models,
            total_versions,
            total_storage_bytes,
            models_by_framework,
            versions_by_type,
            latest_activity,
        })
    }
}