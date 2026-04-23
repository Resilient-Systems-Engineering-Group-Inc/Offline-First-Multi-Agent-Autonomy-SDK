//! Entity extraction module.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Entity types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EntityType {
    TaskId,
    AgentId,
    WorkflowId,
    Priority,
    Duration,
    Location,
    AgentType,
    Capability,
    Timestamp,
    Status,
    Custom(String),
}

/// Extracted entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub entity_type: EntityType,
    pub value: String,
    pub start: usize,
    pub end: usize,
    pub confidence: f64,
}

impl Entity {
    pub fn new(entity_type: EntityType, value: &str, start: usize, end: usize) -> Self {
        Self {
            entity_type,
            value: value.to_string(),
            start,
            end,
            confidence: 0.95,
        }
    }
}

/// Entity extractor.
pub struct EntityExtractor {
    patterns: Vec<(String, EntityType)>,
}

impl EntityExtractor {
    pub fn new() -> Self {
        let patterns = vec![
            // Priority patterns
            (r"(high|urgent|critical)\s*priority".to_string(), EntityType::Priority),
            (r"priority\s*(\d+)".to_string(), EntityType::Priority),
            (r"(low|medium|normal)\s*priority".to_string(), EntityType::Priority),
            
            // Duration patterns
            (r"(\d+)\s*(second|minute|hour|day)s?".to_string(), EntityType::Duration),
            (r"(\d+)\s*(s|m|h|d)\b".to_string(), EntityType::Duration),
            (r"for\s+(\d+)\s*(second|minute|hour|day)s?".to_string(), EntityType::Duration),
            
            // Location patterns
            (r"(at|in|near)\s+(\w[\w\s]+)".to_string(), EntityType::Location),
            (r"location[:\s]+(\w[\w\s]+)".to_string(), EntityType::Location),
            (r"(zone|area|region)\s*(\w+)".to_string(), EntityType::Location),
            
            // Agent type patterns
            (r"(drone|robot|vehicle|agent)\s*(\w+)".to_string(), EntityType::AgentType),
            (r"type[:\s]+(\w+)".to_string(), EntityType::AgentType),
            
            // Capability patterns
            (r"(with|having|capable)\s*of\s+(\w+)".to_string(), EntityType::Capability),
            (r"(needs|requires)\s+(\w+)".to_string(), EntityType::Capability),
            
            // Status patterns
            (r"(pending|running|completed|failed|cancelled)".to_string(), EntityType::Status),
            (r"status[:\s]+(\w+)".to_string(), EntityType::Status),
            
            // UUID patterns (task/agent/workflow IDs)
            (r"[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}".to_string(), EntityType::TaskId),
        ];

        Self { patterns }
    }

    /// Extract entities from text.
    pub async fn extract(&self, text: &str) -> Result<Vec<Entity>> {
        let mut entities = vec![];

        for (pattern, entity_type) in &self.patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                for mat in re.find_iter(text) {
                    let start = mat.start();
                    let end = mat.end();
                    let value = mat.as_str().to_string();

                    entities.push(Entity {
                        entity_type: entity_type.clone(),
                        value,
                        start,
                        end,
                        confidence: 0.95,
                    });
                }
            }
        }

        // Sort by start position
        entities.sort_by_key(|e| e.start);

        // Remove duplicates
        entities.dedup_by(|a, b| a.start == b.start && a.end == b.end);

        Ok(entities)
    }

    /// Extract specific entity type.
    pub async fn extract_type(&self, text: &str, entity_type: &EntityType) -> Result<Vec<Entity>> {
        let all_entities = self.extract(text).await?;
        Ok(all_entities
            .into_iter()
            .filter(|e| &e.entity_type == entity_type)
            .collect())
    }

    /// Get first entity of type.
    pub async fn extract_first(&self, text: &str, entity_type: &EntityType) -> Result<Option<Entity>> {
        let entities = self.extract_type(text, entity_type).await?;
        Ok(entities.into_iter().next())
    }
}

impl Default for EntityExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_entity_extraction() {
        let extractor = EntityExtractor::new();

        let text = "Create a high priority task for zone A with 5m duration";
        let entities = extractor.extract(text).await.unwrap();

        assert!(!entities.is_empty());
        
        // Check for priority entity
        let priorities: Vec<_> = entities
            .iter()
            .filter(|e| e.entity_type == EntityType::Priority)
            .collect();
        assert!(!priorities.is_empty());

        // Check for duration entity
        let durations: Vec<_> = entities
            .iter()
            .filter(|e| e.entity_type == EntityType::Duration)
            .collect();
        assert!(!durations.is_empty());
    }

    #[tokio::test]
    async fn test_specific_entity_extraction() {
        let extractor = EntityExtractor::new();

        let text = "Task 123e4567-e89b-12d3-a456-426614174000 is running";
        let task_ids = extractor.extract_type(text, &EntityType::TaskId).await.unwrap();

        assert_eq!(task_ids.len(), 1);
        assert_eq!(task_ids[0].value, "123e4567-e89b-12d3-a456-426614174000");
    }
}
