//! Querying and indexing.

use crate::error::{Error, Result};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

/// A query over key‑value pairs.
#[derive(Debug, Clone)]
pub enum Query {
    /// Exact match on key.
    Key(String),
    /// Prefix match on key.
    Prefix(String),
    /// Range over keys (inclusive).
    Range { start: String, end: String },
    /// Filter by JSON value field.
    FieldEquals { field: String, value: Value },
    /// Logical AND of multiple queries.
    And(Vec<Query>),
    /// Logical OR of multiple queries.
    Or(Vec<Query>),
}

/// Result of a query.
#[derive(Debug)]
pub struct QueryResult {
    /// Matching key‑value pairs.
    pub entries: Vec<(String, Value)>,
}

/// An index that can speed up certain queries.
pub struct Index {
    /// Index name.
    name: String,
    /// Index type.
    index_type: IndexType,
    /// Mapping from indexed value to keys.
    data: BTreeMap<Value, Vec<String>>,
}

#[derive(Debug, Clone)]
pub enum IndexType {
    /// Index on exact key.
    Key,
    /// Index on a specific JSON field.
    Field(String),
    /// Composite index on multiple fields.
    Composite(Vec<String>),
}

impl Index {
    /// Create a new index.
    pub fn new(name: String, index_type: IndexType) -> Self {
        Self {
            name,
            index_type,
            data: BTreeMap::new(),
        }
    }

    /// Update index with a new key‑value pair.
    pub async fn update(&mut self, key: &str, value: &Value) -> Result<()> {
        let indexed_value = self.extract(value);
        self.data
            .entry(indexed_value)
            .or_insert_with(Vec::new)
            .push(key.to_string());
        Ok(())
    }

    /// Remove a key from the index.
    pub async fn remove(&mut self, key: &str) -> Result<()> {
        // This is inefficient; we'd need reverse mapping.
        // For simplicity, we just clear the index (or do nothing).
        // In a real implementation, we'd maintain a second map.
        Ok(())
    }

    /// Query the index.
    pub fn query(&self, query: &Query) -> Option<Vec<String>> {
        match query {
            Query::Key(k) => {
                // Look up by key? Not supported by this index.
                None
            }
            Query::Prefix(prefix) => {
                // Could be supported with a trie, but we have BTreeMap.
                None
            }
            _ => None,
        }
    }

    /// Extract the value to index based on index type.
    fn extract(&self, value: &Value) -> Value {
        match &self.index_type {
            IndexType::Key => Value::String("key".to_string()), // placeholder
            IndexType::Field(field) => value.get(field).cloned().unwrap_or(Value::Null),
            IndexType::Composite(fields) => {
                let mut obj = serde_json::Map::new();
                for f in fields {
                    obj.insert(f.clone(), value.get(f).cloned().unwrap_or(Value::Null));
                }
                Value::Object(obj)
            }
        }
    }
}

/// Simple query executor (naive scan).
pub fn execute_query(
    data: &HashMap<String, Value>,
    query: &Query,
) -> QueryResult {
    let mut entries = Vec::new();
    for (key, value) in data {
        if matches_query(key, value, query) {
            entries.push((key.clone(), value.clone()));
        }
    }
    QueryResult { entries }
}

fn matches_query(key: &str, value: &Value, query: &Query) -> bool {
    match query {
        Query::Key(k) => key == k,
        Query::Prefix(prefix) => key.starts_with(prefix),
        Query::Range { start, end } => key >= start && key <= end,
        Query::FieldEquals { field, value: v } => {
            value.get(field).map(|f| f == v).unwrap_or(false)
        }
        Query::And(queries) => queries.iter().all(|q| matches_query(key, value, q)),
        Query::Or(queries) => queries.iter().any(|q| matches_query(key, value, q)),
    }
}