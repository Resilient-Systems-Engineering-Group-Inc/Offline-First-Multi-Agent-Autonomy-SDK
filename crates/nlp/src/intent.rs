//! Intent recognition module.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Intent types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IntentType {
    CreateTask,
    UpdateTask,
    DeleteTask,
    ListTasks,
    GetTask,
    CreateAgent,
    RemoveAgent,
    ListAgents,
    GetAgent,
    StartWorkflow,
    StopWorkflow,
    ListWorkflows,
    GetWorkflow,
    SystemHealth,
    GetMetrics,
    ConfigureSystem,
    Unknown,
}

/// Detected intent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    pub intent_type: IntentType,
    pub confidence: f64,
    pub alternatives: Vec<IntentType>,
}

impl Intent {
    pub fn new(intent_type: IntentType, confidence: f64) -> Self {
        Self {
            intent_type,
            confidence,
            alternatives: vec![],
        }
    }
}

/// Intent classifier.
pub struct IntentClassifier {
    intents: Vec<(String, IntentType)>,
}

impl IntentClassifier {
    pub fn new() -> Self {
        let mut intents = vec![
            // Task intents
            ("create.*task".to_string(), IntentType::CreateTask),
            ("new.*task".to_string(), IntentType::CreateTask),
            ("add.*task".to_string(), IntentType::CreateTask),
            ("update.*task".to_string(), IntentType::UpdateTask),
            ("modify.*task".to_string(), IntentType::UpdateTask),
            ("delete.*task".to_string(), IntentType::DeleteTask),
            ("remove.*task".to_string(), IntentType::DeleteTask),
            ("list.*tasks".to_string(), IntentType::ListTasks),
            ("get.*task".to_string(), IntentType::GetTask),
            ("show.*task".to_string(), IntentType::GetTask),
            
            // Agent intents
            ("create.*agent".to_string(), IntentType::CreateAgent),
            ("register.*agent".to_string(), IntentType::CreateAgent),
            ("add.*agent".to_string(), IntentType::CreateAgent),
            ("remove.*agent".to_string(), IntentType::RemoveAgent),
            ("delete.*agent".to_string(), IntentType::RemoveAgent),
            ("list.*agents".to_string(), IntentType::ListAgents),
            ("get.*agent".to_string(), IntentType::GetAgent),
            
            // Workflow intents
            ("start.*workflow".to_string(), IntentType::StartWorkflow),
            ("run.*workflow".to_string(), IntentType::StartWorkflow),
            ("execute.*workflow".to_string(), IntentType::StartWorkflow),
            ("stop.*workflow".to_string(), IntentType::StopWorkflow),
            ("cancel.*workflow".to_string(), IntentType::StopWorkflow),
            ("list.*workflows".to_string(), IntentType::ListWorkflows),
            ("get.*workflow".to_string(), IntentType::GetWorkflow),
            
            // System intents
            ("health".to_string(), IntentType::SystemHealth),
            ("status".to_string(), IntentType::SystemHealth),
            ("metrics".to_string(), IntentType::GetMetrics),
            ("stats".to_string(), IntentType::GetMetrics),
            ("config.*".to_string(), IntentType::ConfigureSystem),
            ("settings.*".to_string(), IntentType::ConfigureSystem),
        ];

        intents.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

        Self { intents }
    }

    /// Classify intent from text.
    pub async fn classify(&self, text: &str) -> Result<Intent> {
        let text_lower = text.to_lowercase();
        
        for (pattern, intent_type) in &self.intents {
            if regex::Regex::new(pattern)
                .map_or(false, |re| re.is_match(&text_lower))
            {
                return Ok(Intent::new(intent_type.clone(), 0.95));
            }
        }

        Ok(Intent::new(IntentType::Unknown, 0.5))
    }

    /// Classify with alternatives.
    pub async fn classify_with_alternatives(&self, text: &str) -> Result<Intent> {
        let text_lower = text.to_lowercase();
        let mut matches = vec![];

        for (pattern, intent_type) in &self.intents {
            if regex::Regex::new(pattern)
                .map_or(false, |re| re.is_match(&text_lower))
            {
                matches.push((pattern, intent_type));
            }
        }

        if matches.is_empty() {
            return Ok(Intent::new(IntentType::Unknown, 0.5));
        }

        let best = matches.remove(0);
        let alternatives: Vec<IntentType> = matches
            .iter()
            .take(2)
            .map(|(_, t)| t.clone())
            .collect();

        Ok(Intent {
            intent_type: best.1.clone(),
            confidence: 0.95,
            alternatives,
        })
    }

    /// Get all supported intents.
    pub fn get_all_intents(&self) -> Vec<IntentType> {
        self.intents.iter().map(|(_, t)| t.clone()).collect()
    }
}

impl Default for IntentClassifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_intent_classification() {
        let classifier = IntentClassifier::new();

        // Test task creation
        let intent = classifier.classify("create a new task").await.unwrap();
        assert_eq!(intent.intent_type, IntentType::CreateTask);

        // Test agent listing
        let intent = classifier.classify("list all agents").await.unwrap();
        assert_eq!(intent.intent_type, IntentType::ListAgents);

        // Test workflow start
        let intent = classifier.classify("start workflow").await.unwrap();
        assert_eq!(intent.intent_type, IntentType::StartWorkflow);

        // Test system health
        let intent = classifier.classify("system health check").await.unwrap();
        assert_eq!(intent.intent_type, IntentType::SystemHealth);
    }

    #[tokio::test]
    async fn test_intent_alternatives() {
        let classifier = IntentClassifier::new();

        let intent = classifier.classify_with_alternatives("create task")
            .await
            .unwrap();
        
        assert_eq!(intent.intent_type, IntentType::CreateTask);
        assert!(!intent.alternatives.is_empty());
    }
}
