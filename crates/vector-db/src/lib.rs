//! Vector database for semantic search and similarity queries.
//!
//! Provides:
//! - High-performance vector indexing (FAISS)
//! - Similarity search (cosine, L2, inner product)
//! - Persistent storage (Parquet/Arrow)
//! - Batch operations

pub mod index;
pub mod storage;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::sync::RwLock;
use tracing::info;

pub use index::*;
pub use storage::*;

/// Vector database configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorDBConfig {
    pub data_dir: PathBuf,
    pub dimension: usize,
    pub metric_type: MetricType,
    pub index_type: IndexType,
    pub max_vectors: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MetricType {
    Cosine,
    L2,
    InnerProduct,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndexType {
    Flat,
    IVF,
    HNSW,
    PQ,
}

impl Default for VectorDBConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("./vector_data"),
            dimension: 768,
            metric_type: MetricType::Cosine,
            index_type: IndexType::HNSW,
            max_vectors: 1_000_000,
        }
    }
}

/// Vector database.
pub struct VectorDB {
    config: VectorDBConfig,
    index: RwLock<Option<VectorIndex>>,
    metadata: RwLock<Vec<VectorMetadata>>,
}

/// Vector metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorMetadata {
    pub id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub tags: Vec<String>,
    pub custom_metadata: serde_json::Value,
}

impl VectorDB {
    /// Create new vector database.
    pub fn new(config: VectorDBConfig) -> Self {
        Self {
            config,
            index: RwLock::new(None),
            metadata: RwLock::new(vec![]),
        }
    }

    /// Initialize database.
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing vector database...");

        // Create data directory
        tokio::fs::create_dir_all(&self.config.data_dir).await?;

        // Create index
        let vector_index = VectorIndex::new(&self.config);
        *self.index.write().await = Some(vector_index);

        info!("Vector database initialized");
        Ok(())
    }

    /// Add vector.
    pub async fn add(&self, vector: &[f32], metadata: VectorMetadata) -> Result<String> {
        let mut index = self.index.write().await;
        let index = index.as_mut().ok_or_else(|| anyhow::anyhow!("Database not initialized"))?;

        let id = metadata.id.clone();
        
        index.add(vector, &id).await?;
        
        let mut metadata_list = self.metadata.write().await;
        metadata_list.push(metadata);

        Ok(id)
    }

    /// Add multiple vectors.
    pub async fn add_batch(&self, vectors: &[Vec<f32>], metadatas: Vec<VectorMetadata>) -> Result<Vec<String>> {
        let mut index = self.index.write().await;
        let index = index.as_mut().ok_or_else(|| anyhow::anyhow!("Database not initialized"))?;

        let mut ids = vec![];
        
        for (vector, metadata) in vectors.iter().zip(metadatas.iter()) {
            index.add(vector, &metadata.id).await?;
            ids.push(metadata.id.clone());
        }

        let mut metadata_list = self.metadata.write().await;
        metadata_list.extend(metadatas);

        Ok(ids)
    }

    /// Search for similar vectors.
    pub async fn search(&self, query: &[f32], top_k: usize) -> Result<Vec<SearchResult>> {
        let index = self.index.read().await;
        let index = index.as_ref().ok_or_else(|| anyhow::anyhow!("Database not initialized"))?;

        let results = index.search(query, top_k).await?;
        
        Ok(results)
    }

    /// Search with metadata filter.
    pub async fn search_filtered(&self, query: &[f32], top_k: usize, filter: &MetadataFilter) -> Result<Vec<SearchResult>> {
        let results = self.search(query, top_k).await?;
        
        Ok(results.into_iter()
            .filter(|r| filter.matches(&r.metadata))
            .collect())
    }

    /// Get vector by ID.
    pub async fn get(&self, id: &str) -> Result<Option<Vec<f32>>> {
        let index = self.index.read().await;
        let index = index.as_ref().ok_or_else(|| anyhow::anyhow!("Database not initialized"))?;

        index.get(id).await
    }

    /// Delete vector.
    pub async fn delete(&self, id: &str) -> Result<bool> {
        let mut index = self.index.write().await;
        let index = index.as_mut().ok_or_else(|| anyhow::anyhow!("Database not initialized"))?;

        let deleted = index.delete(id).await?;

        if deleted {
            let mut metadata_list = self.metadata.write().await;
            metadata_list.retain(|m| m.id != id);
        }

        Ok(deleted)
    }

    /// Update vector.
    pub async fn update(&self, id: &str, vector: &[f32]) -> Result<bool> {
        let mut index = self.index.write().await;
        let index = index.as_mut().ok_or_else(|| anyhow::anyhow!("Database not initialized"))?;

        index.update(id, vector).await
    }

    /// Save database to disk.
    pub async fn save(&self) -> Result<()> {
        let index = self.index.read().await;
        
        if let Some(index) = index.as_ref() {
            index.save(&self.config.data_dir).await?;
        }

        let metadata = self.metadata.read().await;
        let metadata_path = self.config.data_dir.join("metadata.json");
        let content = serde_json::to_string_pretty(&*metadata)?;
        tokio::fs::write(metadata_path, content).await?;

        info!("Database saved");
        Ok(())
    }

    /// Load database from disk.
    pub async fn load(&self) -> Result<()> {
        let data_dir = &self.config.data_dir;

        if !data_dir.exists() {
            return Err(anyhow::anyhow!("Data directory does not exist"));
        }

        // Load index
        let vector_index = VectorIndex::load(data_dir, &self.config).await?;
        *self.index.write().await = Some(vector_index);

        // Load metadata
        let metadata_path = data_dir.join("metadata.json");
        if metadata_path.exists() {
            let content = tokio::fs::read_to_string(metadata_path).await?;
            let metadata: Vec<VectorMetadata> = serde_json::from_str(&content)?;
            *self.metadata.write().await = metadata;
        }

        info!("Database loaded");
        Ok(())
    }

    /// Get database statistics.
    pub async fn get_stats(&self) -> VectorDBStats {
        let index = self.index.read().await;
        let metadata = self.metadata.read().await;

        VectorDBStats {
            total_vectors: metadata.len() as i64,
            dimension: self.config.dimension as i32,
            metric_type: format!("{:?}", self.config.metric_type),
            index_type: format!("{:?}", self.config.index_type),
        }
    }
}

/// Search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub score: f32,
    pub metadata: serde_json::Value,
}

/// Metadata filter.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MetadataFilter {
    pub tags: Vec<String>,
    pub custom: serde_json::Value,
}

impl MetadataFilter {
    pub fn matches(&self, metadata: &serde_json::Value) -> bool {
        if self.tags.is_empty() && self.custom.is_null() {
            return true;
        }

        // Check tags
        if !self.tags.is_empty() {
            if let Some(tags) = metadata.get("tags").and_then(|v| v.as_array()) {
                let has_all_tags = self.tags.iter().all(|t| {
                    tags.iter().any(|tag| tag.as_str() == Some(t))
                });
                if !has_all_tags {
                    return false;
                }
            }
        }

        true
    }
}

/// Vector database statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorDBStats {
    pub total_vectors: i64,
    pub dimension: i32,
    pub metric_type: String,
    pub index_type: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_vector_db() {
        let config = VectorDBConfig::default();
        let db = VectorDB::new(config);
        
        db.initialize().await.unwrap();

        // Add vectors
        let vector1 = vec![0.1; 768];
        let metadata1 = VectorMetadata {
            id: "vec-1".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            tags: vec!["test".to_string()],
            custom_metadata: serde_json::json!({}),
        };

        db.add(&vector1, metadata1).await.unwrap();

        // Search
        let results = db.search(&vector1, 10).await.unwrap();
        assert!(!results.is_empty());

        // Get stats
        let stats = db.get_stats().await;
        assert_eq!(stats.total_vectors, 1);
    }
}
