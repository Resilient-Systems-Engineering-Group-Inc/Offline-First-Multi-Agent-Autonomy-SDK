//! Natural Language Processing for the Multi-Agent SDK.
//!
//! Provides:
//! - Intent recognition
//! - Entity extraction
//! - Task parsing from natural language
//! - Semantic search

pub mod intent;
pub mod entities;
pub mod parser;
pub mod semantic_search;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::info;

pub use intent::*;
pub use entities::*;
pub use parser::*;
pub use semantic_search::*;

/// NLP configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NLPConfig {
    pub model_path: String,
    pub embedding_dimension: usize,
    pub max_tokens: usize,
    pub language: String,
}

impl Default for NLPConfig {
    fn default() -> Self {
        Self {
            model_path: "./models/nlp".to_string(),
            embedding_dimension: 768,
            max_tokens: 512,
            language: "en".to_string(),
        }
    }
}

/// NLP processor.
pub struct NLPProcessor {
    config: NLPConfig,
    intent_classifier: RwLock<Option<IntentClassifier>>,
    entity_extractor: RwLock<Option<EntityExtractor>>,
    semantic_search: RwLock<Option<SemanticSearch>>,
}

impl NLPProcessor {
    /// Create new NLP processor.
    pub fn new(config: NLPConfig) -> Self {
        Self {
            config,
            intent_classifier: RwLock::new(None),
            entity_extractor: RwLock::new(None),
            semantic_search: RwLock::new(None),
        }
    }

    /// Initialize NLP models.
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing NLP processor...");

        *self.intent_classifier.write().await = Some(IntentClassifier::new());
        *self.entity_extractor.write().await = Some(EntityExtractor::new());
        *self.semantic_search.write().await = Some(SemanticSearch::new(&self.config));

        info!("NLP processor initialized");
        Ok(())
    }

    /// Parse natural language command.
    pub async fn parse_command(&self, text: &str) -> Result<ParsedCommand> {
        let intent_classifier = self.intent_classifier.read().await;
        let entity_extractor = self.entity_extractor.read().await;

        if intent_classifier.is_none() || entity_extractor.is_none() {
            return Err(anyhow::anyhow!("NLP processor not initialized"));
        }

        // Detect intent
        let intent = intent_classifier.as_ref().unwrap().classify(text).await?;

        // Extract entities
        let entities = entity_extractor.as_ref().unwrap().extract(text).await?;

        Ok(ParsedCommand {
            text: text.to_string(),
            intent,
            entities,
            confidence: 0.95,
        })
    }

    /// Convert natural language to task.
    pub async fn text_to_task(&self, text: &str) -> Result<TaskDefinition> {
        let parsed = self.parse_command(text).await?;

        let mut task = TaskDefinition::new("auto-generated");

        // Extract task parameters from entities
        for entity in &parsed.entities {
            match entity.entity_type.as_str() {
                "priority" => {
                    if let Ok(p) = entity.value.parse::<i32>() {
                        task.priority = p;
                    }
                }
                "duration" => {
                    task.expected_duration_secs = Some(parse_duration(&entity.value));
                }
                "location" => {
                    task.metadata.insert("location".to_string(), json!(entity.value));
                }
                "agent_type" => {
                    task.required_capabilities.push(entity.value.clone());
                }
                _ => {}
            }
        }

        // Set description from text
        task.description = text.to_string();

        // Set intent as task type
        task.task_type = format!("{:?}", parsed.intent.intent_type);

        Ok(task)
    }

    /// Semantic search for tasks.
    pub async fn search_tasks(&self, query: &str, limit: usize) -> Result<Vec<TaskDefinition>> {
        let semantic_search = self.semantic_search.read().await;

        if semantic_search.is_none() {
            return Err(anyhow::anyhow!("Semantic search not initialized"));
        }

        semantic_search.as_ref().unwrap().search(query, limit).await
    }

    /// Add task to semantic index.
    pub async fn index_task(&self, task: &TaskDefinition) -> Result<()> {
        let semantic_search = self.semantic_search.write().await;

        if semantic_search.is_none() {
            return Err(anyhow::anyhow!("Semantic search not initialized"));
        }

        semantic_search.as_ref().unwrap().index(task).await
    }

    /// Get NLP statistics.
    pub async fn get_stats(&self) -> NLPStats {
        NLPStats {
            total_commands_processed: 0,
            avg_confidence: 0.0,
            intents_detected: 0,
            entities_extracted: 0,
        }
    }
}

/// Parsed command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedCommand {
    pub text: String,
    pub intent: Intent,
    pub entities: Vec<Entity>,
    pub confidence: f64,
}

/// Task definition from NLP.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDefinition {
    pub id: String,
    pub description: String,
    pub task_type: String,
    pub priority: i32,
    pub required_capabilities: Vec<String>,
    pub expected_duration_secs: Option<u64>,
    pub metadata: serde_json::Value,
}

impl TaskDefinition {
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            description: String::new(),
            task_type: String::new(),
            priority: 100,
            required_capabilities: vec![],
            expected_duration_secs: None,
            metadata: serde_json::json!({}),
        }
    }
}

/// Parse duration string.
fn parse_duration(s: &str) -> u64 {
    // Simple duration parser (e.g., "5m", "1h", "30s")
    let re = regex::Regex::new(r"(\d+)\s*(s|m|h|d)?").unwrap();
    
    if let Some(caps) = re.captures(s) {
        let value: u64 = caps.get(1).unwrap().as_str().parse().unwrap_or(0);
        let unit = caps.get(2).map_or("s", |m| m.as_str());
        
        match unit {
            "s" => value,
            "m" => value * 60,
            "h" => value * 3600,
            "d" => value * 86400,
            _ => value,
        }
    } else {
        0
    }
}

/// NLP statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NLPStats {
    pub total_commands_processed: i64,
    pub avg_confidence: f64,
    pub intents_detected: i64,
    pub entities_extracted: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_nlp_processor() {
        let config = NLPConfig::default();
        let processor = NLPProcessor::new(config);
        
        // Initialize
        processor.initialize().await.unwrap();

        // Parse command
        let command = "Create a high priority task for exploration";
        let parsed = processor.parse_command(command).await.unwrap();
        
        assert!(!parsed.intent.intent_type.to_string().is_empty());
        assert!(parsed.confidence > 0.0);
    }

    #[test]
    fn test_duration_parsing() {
        assert_eq!(parse_duration("5s"), 5);
        assert_eq!(parse_duration("5m"), 300);
        assert_eq!(parse_duration("1h"), 3600);
        assert_eq!(parse_duration("1d"), 86400);
    }
}
