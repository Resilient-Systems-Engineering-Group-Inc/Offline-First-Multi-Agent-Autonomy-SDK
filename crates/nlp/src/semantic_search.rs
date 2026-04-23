//! Semantic search module.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;

use crate::{NLPConfig, TaskDefinition};

/// Semantic search index.
pub struct SemanticSearch {
    config: NLPConfig,
    index: RwLock<HashMap<String, Document>>,
    embeddings: RwLock<HashMap<String, Vec<f32>>>,
}

/// Document in index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub content: String,
    pub metadata: serde_json::Value,
}

impl SemanticSearch {
    pub fn new(config: &NLPConfig) -> Self {
        Self {
            config: config.clone(),
            index: RwLock::new(HashMap::new()),
            embeddings: RwLock::new(HashMap::new()),
        }
    }

    /// Index a task.
    pub async fn index(&self, task: &TaskDefinition) -> Result<()> {
        let doc = Document {
            id: task.id.clone(),
            content: format!("{} {}", task.description, task.task_type),
            metadata: serde_json::json!({
                "task_type": task.task_type,
                "priority": task.priority,
                "capabilities": task.required_capabilities,
            }),
        };

        // Generate embedding (mock - in production would use actual model)
        let embedding = self.generate_embedding(&doc.content);

        let mut index = self.index.write().await;
        let mut embeddings = self.embeddings.write().await;

        index.insert(task.id.clone(), doc);
        embeddings.insert(task.id.clone(), embedding);

        Ok(())
    }

    /// Search for similar tasks.
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<TaskDefinition>> {
        let query_embedding = self.generate_embedding(query);
        let embeddings = self.embeddings.read().await;
        let index = self.index.read().await;

        // Calculate cosine similarity
        let mut scores: Vec<(String, f32)> = embeddings
            .iter()
            .map(|(id, emb)| {
                let similarity = self.cosine_similarity(&query_embedding, emb);
                (id.clone(), similarity)
            })
            .collect();

        // Sort by similarity
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Get top results
        let top_results: Vec<TaskDefinition> = scores
            .into_iter()
            .take(limit)
            .filter_map(|(id, _)| {
                index.get(&id).map(|doc| {
                    TaskDefinition {
                        id: doc.id.clone(),
                        description: doc.content.clone(),
                        task_type: doc.metadata.get("task_type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string(),
                        priority: doc.metadata.get("priority")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(100) as i32,
                        required_capabilities: doc.metadata.get("capabilities")
                            .and_then(|v| v.as_array())
                            .map(|arr| arr.iter()
                                .filter_map(|v| v.as_str())
                                .map(|s| s.to_string())
                                .collect())
                            .unwrap_or_default(),
                        expected_duration_secs: None,
                        metadata: doc.metadata.clone(),
                    }
                })
            })
            .collect();

        Ok(top_results)
    }

    /// Generate embedding (mock implementation).
    fn generate_embedding(&self, text: &str) -> Vec<f32> {
        // In production, would use transformer model
        // For now, use simple hash-based embedding
        let mut embedding = vec![0.0f32; self.config.embedding_dimension];
        
        for (i, byte) in text.bytes().enumerate() {
            let idx = i % self.config.embedding_dimension;
            embedding[idx] = embedding[idx] + (byte as f32) / 255.0;
        }

        // Normalize
        let norm = (embedding.iter().map(|x| x * x).sum::<f32>()).sqrt();
        if norm > 0.0 {
            for x in embedding.iter_mut() {
                *x /= norm;
            }
        }

        embedding
    }

    /// Cosine similarity.
    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            dot / (norm_a * norm_b)
        }
    }

    /// Remove from index.
    pub async fn remove(&self, id: &str) -> Result<()> {
        let mut index = self.index.write().await;
        let mut embeddings = self.embeddings.write().await;

        index.remove(id);
        embeddings.remove(id);

        Ok(())
    }

    /// Get index statistics.
    pub async fn get_stats(&self) -> SearchStats {
        let index = self.index.read().await;
        let embeddings = self.embeddings.read().await;

        SearchStats {
            total_documents: index.len() as i64,
            embedding_dimension: self.config.embedding_dimension as i32,
        }
    }
}

/// Search statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchStats {
    pub total_documents: i64,
    pub embedding_dimension: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_semantic_search() {
        let config = NLPConfig::default();
        let search = SemanticSearch::new(&config);

        // Create test tasks
        let task1 = TaskDefinition {
            id: "task-1".to_string(),
            description: "Explore zone A".to_string(),
            task_type: "exploration".to_string(),
            priority: 100,
            required_capabilities: vec!["navigation".to_string()],
            expected_duration_secs: None,
            metadata: serde_json::json!({}),
        };

        let task2 = TaskDefinition {
            id: "task-2".to_string(),
            description: "Map zone B".to_string(),
            task_type: "mapping".to_string(),
            priority: 80,
            required_capabilities: vec!["sensors".to_string()],
            expected_duration_secs: None,
            metadata: serde_json::json!({}),
        };

        // Index tasks
        search.index(&task1).await.unwrap();
        search.index(&task2).await.unwrap();

        // Search
        let results = search.search("explore area", 10).await.unwrap();
        assert!(!results.is_empty());

        // Verify task-1 is in results
        let has_task1 = results.iter().any(|t| t.id == "task-1");
        assert!(has_task1);
    }

    #[tokio::test]
    async fn test_embedding_generation() {
        let config = NLPConfig::default();
        let search = SemanticSearch::new(&config);

        let embedding = search.generate_embedding("test text");
        
        assert_eq!(embedding.len(), config.embedding_dimension);
        
        // Check normalization
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.01);
    }
}
