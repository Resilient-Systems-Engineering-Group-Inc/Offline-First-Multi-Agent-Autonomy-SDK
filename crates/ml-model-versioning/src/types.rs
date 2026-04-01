//! Core types for ML model versioning.

use chrono::{DateTime, Utc};
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// Unique identifier for a model.
pub type ModelId = String;

/// Unique identifier for a model version.
pub type VersionId = String;

/// Model metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetadata {
    /// Unique model identifier.
    pub id: ModelId,
    /// Human-readable model name.
    pub name: String,
    /// Model description.
    pub description: String,
    /// Model type (e.g., "neural_network", "decision_tree", "rl_policy").
    pub model_type: String,
    /// Framework used (e.g., "tensorflow", "pytorch", "onnx").
    pub framework: String,
    /// Input schema (JSON schema or description).
    pub input_schema: Option<serde_json::Value>,
    /// Output schema (JSON schema or description).
    pub output_schema: Option<serde_json::Value>,
    /// Tags for categorization.
    pub tags: Vec<String>,
    /// Custom metadata key-value pairs.
    pub custom_metadata: HashMap<String, serde_json::Value>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}

/// Model version metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelVersion {
    /// Model identifier.
    pub model_id: ModelId,
    /// Version identifier (e.g., "v1.0.0", "latest", "experimental").
    pub version: VersionId,
    /// Semantic version (if applicable).
    pub semver: Option<Version>,
    /// Description of changes in this version.
    pub changelog: String,
    /// SHA-256 checksum of the model binary.
    pub checksum: String,
    /// Size of the model binary in bytes.
    pub size_bytes: u64,
    /// Storage location (URI, file path, or distributed key).
    pub storage_location: String,
    /// Dependencies (other model versions or libraries).
    pub dependencies: Vec<Dependency>,
    /// Training metrics (accuracy, loss, etc.).
    pub metrics: HashMap<String, f64>,
    /// Hyperparameters used for training.
    pub hyperparameters: HashMap<String, serde_json::Value>,
    /// Training dataset information.
    pub training_data: Option<TrainingDataInfo>,
    /// Whether this version is the default/latest.
    pub is_default: bool,
    /// Whether this version is deprecated.
    pub is_deprecated: bool,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Author/creator of this version.
    pub author: String,
}

/// Dependency on another model or library.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    /// Dependency type: "model", "library", "framework".
    pub dep_type: String,
    /// Identifier (model ID or library name).
    pub identifier: String,
    /// Version constraint (semver range).
    pub version_constraint: String,
}

/// Information about training data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingDataInfo {
    /// Dataset identifier or name.
    pub dataset_id: String,
    /// Dataset version.
    pub dataset_version: String,
    /// Number of samples.
    pub sample_count: u64,
    /// Data split (train/validation/test percentages).
    pub split: Option<DataSplit>,
}

/// Data split percentages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSplit {
    /// Training set percentage (0-100).
    pub train_percent: f32,
    /// Validation set percentage (0-100).
    pub validation_percent: f32,
    /// Test set percentage (0-100).
    pub test_percent: f32,
}

/// Model version query filters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionQuery {
    /// Filter by model ID.
    pub model_id: Option<ModelId>,
    /// Filter by version tag (e.g., "latest", "stable").
    pub tag: Option<String>,
    /// Filter by framework.
    pub framework: Option<String>,
    /// Filter by model type.
    pub model_type: Option<String>,
    /// Filter by minimum accuracy (or other metric).
    pub min_metric: Option<MetricFilter>,
    /// Limit number of results.
    pub limit: Option<usize>,
    /// Offset for pagination.
    pub offset: Option<usize>,
    /// Sort order.
    pub sort_by: Option<SortField>,
    /// Sort direction.
    pub sort_desc: bool,
}

/// Metric filter for querying.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricFilter {
    /// Metric name (e.g., "accuracy", "f1_score").
    pub metric_name: String,
    /// Minimum value.
    pub min_value: f64,
    /// Maximum value.
    pub max_value: Option<f64>,
}

/// Fields to sort by.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortField {
    /// Sort by version (semantic or lexicographic).
    Version,
    /// Sort by creation date.
    CreatedAt,
    /// Sort by a specific metric.
    Metric(String),
    /// Sort by model size.
    Size,
}

/// Result of a version query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionQueryResult {
    /// Matching versions.
    pub versions: Vec<ModelVersion>,
    /// Total count (for pagination).
    pub total_count: usize,
    /// Whether there are more results.
    pub has_more: bool,
}

/// Model binary with metadata.
#[derive(Debug, Clone)]
pub struct ModelBinary {
    /// Model version metadata.
    pub metadata: ModelVersion,
    /// Raw model bytes.
    pub data: Vec<u8>,
}

impl ModelBinary {
    /// Compute SHA-256 checksum of the data.
    pub fn compute_checksum(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(&self.data);
        hex::encode(hasher.finalize())
    }

    /// Verify that the stored checksum matches the computed checksum.
    pub fn verify_checksum(&self) -> bool {
        self.compute_checksum() == self.metadata.checksum
    }
}

/// Version creation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVersionRequest {
    /// Model ID (must exist).
    pub model_id: ModelId,
    /// Version identifier.
    pub version: VersionId,
    /// Optional semantic version.
    pub semver: Option<Version>,
    /// Changelog description.
    pub changelog: String,
    /// Model binary data.
    pub data: Vec<u8>,
    /// Dependencies.
    pub dependencies: Vec<Dependency>,
    /// Training metrics.
    pub metrics: HashMap<String, f64>,
    /// Hyperparameters.
    pub hyperparameters: HashMap<String, serde_json::Value>,
    /// Training data info.
    pub training_data: Option<TrainingDataInfo>,
    /// Whether to set as default version.
    pub set_as_default: bool,
    /// Author/creator.
    pub author: String,
}

/// Version update request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateVersionRequest {
    /// New changelog (if any).
    pub changelog: Option<String>,
    /// New dependencies (if any).
    pub dependencies: Option<Vec<Dependency>>,
    /// New metrics (if any).
    pub metrics: Option<HashMap<String, f64>>,
    /// Whether to deprecate this version.
    pub deprecate: Option<bool>,
    /// Whether to set as default.
    pub set_as_default: Option<bool>,
}

/// Statistics about model registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryStats {
    /// Total number of models.
    pub total_models: usize,
    /// Total number of versions.
    pub total_versions: usize,
    /// Total storage used in bytes.
    pub total_storage_bytes: u64,
    /// Number of models by framework.
    pub models_by_framework: HashMap<String, usize>,
    /// Number of versions by model type.
    pub versions_by_type: HashMap<String, usize>,
    /// Latest activity timestamp.
    pub latest_activity: DateTime<Utc>,
}