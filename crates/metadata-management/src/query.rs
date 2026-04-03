//! Metadata query language and execution.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::error::{MetadataError, Result};
use crate::model::{Metadata, MetadataId, MetadataType};
use crate::storage::MetadataStorage;
use crate::index::MetadataIndex;

/// Query operator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryOperator {
    /// Equality.
    Eq,
    /// Inequality.
    Ne,
    /// Greater than.
    Gt,
    /// Greater than or equal.
    Ge,
    /// Less than.
    Lt,
    /// Less than or equal.
    Le,
    /// Contains (for strings).
    Contains,
    /// In list.
    In,
    /// Matches regex.
    Regex,
}

/// Query condition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryCondition {
    /// Field path (e.g., "tags", "content.name").
    pub field: String,
    /// Operator.
    pub operator: QueryOperator,
    /// Value (JSON).
    pub value: serde_json::Value,
}

/// Metadata query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataQuery {
    /// Optional metadata type filter.
    pub metadata_type: Option<MetadataType>,
    /// Optional entity ID filter.
    pub entity_id: Option<String>,
    /// List of conditions (AND).
    pub conditions: Vec<QueryCondition>,
    /// Limit number of results.
    pub limit: Option<usize>,
    /// Offset for pagination.
    pub offset: Option<usize>,
    /// Sort by field (not implemented).
    pub sort_by: Option<String>,
}

impl MetadataQuery {
    /// Create a new empty query.
    pub fn new() -> Self {
        Self {
            metadata_type: None,
            entity_id: None,
            conditions: Vec::new(),
            limit: None,
            offset: None,
            sort_by: None,
        }
    }

    /// Add a condition.
    pub fn with_condition(mut self, condition: QueryCondition) -> Self {
        self.conditions.push(condition);
        self
    }

    /// Set metadata type filter.
    pub fn with_type(mut self, metadata_type: MetadataType) -> Self {
        self.metadata_type = Some(metadata_type);
        self
    }

    /// Set entity ID filter.
    pub fn with_entity_id(mut self, entity_id: impl Into<String>) -> Self {
        self.entity_id = Some(entity_id.into());
        self
    }
}

impl Default for MetadataQuery {
    fn default() -> Self {
        Self::new()
    }
}

/// Query executor that uses storage and index.
pub struct MetadataQueryExecutor {
    storage: MetadataStorage,
    index: MetadataIndex,
}

impl MetadataQueryExecutor {
    /// Create a new query executor.
    pub fn new(storage: MetadataStorage, index: MetadataIndex) -> Self {
        Self { storage, index }
    }

    /// Execute a query and return matching metadata entries.
    pub async fn execute(&self, query: &MetadataQuery) -> Result<Vec<Metadata>> {
        // Step 1: Gather candidate IDs from index (if conditions can be indexed)
        let mut candidate_ids: Option<HashSet<MetadataId>> = None;
        for condition in &query.conditions {
            if let Some(ids) = self.evaluate_condition_index(condition).await? {
                candidate_ids = match candidate_ids {
                    Some(existing) => Some(existing.intersection(&ids).cloned().collect()),
                    None => Some(ids),
                };
            }
        }

        // Step 2: If no index support, we'll have to scan (not implemented)
        // For simplicity, we'll just fetch all metadata of given type/entity and filter.
        let mut results = Vec::new();
        let mut candidates = Vec::new();

        if let Some(metadata_type) = &query.metadata_type {
            candidates.extend(self.storage.list_by_type(metadata_type.clone()).await?);
        } else {
            // Without type filter, we cannot efficiently scan all.
            // In a real implementation, you'd have a secondary index.
            // For now, we return empty.
            return Ok(Vec::new());
        }

        // Step 3: Filter by entity ID if provided
        if let Some(entity_id) = &query.entity_id {
            candidates.retain(|m| &m.entity_id == entity_id);
        }

        // Step 4: Apply conditions
        for metadata in candidates {
            if self.evaluate_conditions(&metadata, &query.conditions).await? {
                results.push(metadata);
            }
        }

        // Step 5: Apply limit/offset
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(usize::MAX);
        let results = results.into_iter().skip(offset).take(limit).collect();

        Ok(results)
    }

    async fn evaluate_condition_index(
        &self,
        condition: &QueryCondition,
    ) -> Result<Option<HashSet<MetadataId>>> {
        // Only support equality on tags for now
        if condition.field == "tags" && condition.operator == QueryOperator::Eq {
            if let Some(tag) = condition.value.as_str() {
                let ids = self.index.query_by_tag(tag).await?;
                return Ok(Some(ids));
            }
        }
        Ok(None)
    }

    async fn evaluate_conditions(
        &self,
        metadata: &Metadata,
        conditions: &[QueryCondition],
    ) -> Result<bool> {
        for condition in conditions {
            if !self.evaluate_condition(metadata, condition).await? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    async fn evaluate_condition(
        &self,
        metadata: &Metadata,
        condition: &QueryCondition,
    ) -> Result<bool> {
        // Simple implementation: only supports tags and top‑level content fields.
        let field_value = self.extract_field(metadata, &condition.field).await;
        match condition.operator {
            QueryOperator::Eq => Ok(field_value == condition.value),
            QueryOperator::Ne => Ok(field_value != condition.value),
            QueryOperator::Contains => {
                if let Some(str_val) = condition.value.as_str() {
                    if let Some(field_str) = field_value.as_str() {
                        return Ok(field_str.contains(str_val));
                    }
                }
                Ok(false)
            }
            _ => Ok(false), // not implemented
        }
    }

    async fn extract_field(&self, metadata: &Metadata, field: &str) -> serde_json::Value {
        match field {
            "tags" => serde_json::Value::Array(
                metadata.tags.iter().map(|t| serde_json::Value::String(t.clone())).collect(),
            ),
            _ => {
                // Try to get from content
                if let Some(obj) = metadata.content.as_object() {
                    if let Some(val) = obj.get(field) {
                        return val.clone();
                    }
                }
                serde_json::Value::Null
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::MetadataType;

    #[tokio::test]
    async fn test_query_executor() {
        let storage = crate::storage::MetadataStorage::new(
            Arc::new(crate::storage::InMemoryMetadataStorage::new()),
        );
        let index = MetadataIndex::new();
        let executor = MetadataQueryExecutor::new(storage, index);
        // Empty query should return empty
        let results = executor.execute(&MetadataQuery::new()).await.unwrap();
        assert!(results.is_empty());
    }
}