//! Metadata indexing for fast querying.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use regex::Regex;

use crate::error::{MetadataError, Result};
use crate::model::{Metadata, MetadataId};

/// Index field type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IndexField {
    /// Tag field.
    Tag,
    /// Custom field path (e.g., "content.name").
    Field(String),
}

/// Index entry.
#[derive(Debug, Clone)]
struct IndexEntry {
    field: IndexField,
    value: String,
    metadata_ids: HashSet<MetadataId>,
}

/// Metadata index for fast lookups.
pub struct MetadataIndex {
    /// Map from (field, value) to set of metadata IDs.
    index: Arc<RwLock<HashMap<(IndexField, String), HashSet<MetadataId>>>>,
    /// Inverted index from metadata ID to set of (field, value).
    reverse_index: Arc<RwLock<HashMap<MetadataId, HashSet<(IndexField, String)>>>>,
}

impl MetadataIndex {
    /// Create a new empty index.
    pub fn new() -> Self {
        Self {
            index: Arc::new(RwLock::new(HashMap::new())),
            reverse_index: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Index a metadata entry.
    pub async fn index(&self, metadata: &Metadata) -> Result<()> {
        let mut index = self.index.write().await;
        let mut reverse = self.reverse_index.write().await;

        // Index tags
        for tag in &metadata.tags {
            let key = (IndexField::Tag, tag.clone());
            index.entry(key.clone()).or_insert_with(HashSet::new).insert(metadata.id);
            reverse.entry(metadata.id).or_insert_with(HashSet::new).insert(key);
        }

        // Index fields from content (simple flattening)
        self.index_json_fields(&mut index, &mut reverse, metadata).await;

        Ok(())
    }

    async fn index_json_fields(
        &self,
        index: &mut HashMap<(IndexField, String), HashSet<MetadataId>>,
        reverse: &mut HashMap<MetadataId, HashSet<(IndexField, String)>>,
        metadata: &Metadata,
    ) {
        // For simplicity, we only index top‑level string fields.
        if let Some(obj) = metadata.content.as_object() {
            for (key, value) in obj {
                if let Some(str_val) = value.as_str() {
                    let field = IndexField::Field(key.clone());
                    let key = (field, str_val.to_string());
                    index.entry(key.clone()).or_insert_with(HashSet::new).insert(metadata.id);
                    reverse.entry(metadata.id).or_insert_with(HashSet::new).insert(key);
                }
            }
        }
    }

    /// Remove a metadata entry from the index.
    pub async fn remove(&self, metadata_id: MetadataId) -> Result<()> {
        let mut index = self.index.write().await;
        let mut reverse = self.reverse_index.write().await;

        if let Some(keys) = reverse.remove(&metadata_id) {
            for key in keys {
                if let Some(set) = index.get_mut(&key) {
                    set.remove(&metadata_id);
                    if set.is_empty() {
                        index.remove(&key);
                    }
                }
            }
        }

        Ok(())
    }

    /// Query the index for metadata IDs matching a field‑value pair.
    pub async fn query(&self, field: IndexField, value: &str) -> Result<HashSet<MetadataId>> {
        let index = self.index.read().await;
        Ok(index.get(&(field, value.to_string())).cloned().unwrap_or_default())
    }

    /// Query by tag.
    pub async fn query_by_tag(&self, tag: &str) -> Result<HashSet<MetadataId>> {
        self.query(IndexField::Tag, tag).await
    }

    /// Query by field path.
    pub async fn query_by_field(&self, field_path: &str, value: &str) -> Result<HashSet<MetadataId>> {
        self.query(IndexField::Field(field_path.to_string()), value).await
    }

    /// Full‑text search across all indexed string fields (simple regex).
    pub async fn search(&self, pattern: &str) -> Result<HashSet<MetadataId>> {
        let re = Regex::new(pattern).map_err(|e| MetadataError::Index(e.to_string()))?;
        let index = self.index.read().await;
        let mut result = HashSet::new();
        for ((field, value), ids) in index.iter() {
            if re.is_match(value) {
                result.extend(ids);
            }
        }
        Ok(result)
    }
}

impl Default for MetadataIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Metadata, MetadataType};

    #[tokio::test]
    async fn test_index_tag() {
        let index = MetadataIndex::new();
        let mut metadata = Metadata::new(
            MetadataType::Agent,
            "agent1",
            serde_json::json!({}),
        );
        metadata.add_tag("test");
        index.index(&metadata).await.unwrap();
        let ids = index.query_by_tag("test").await.unwrap();
        assert!(ids.contains(&metadata.id));
    }
}