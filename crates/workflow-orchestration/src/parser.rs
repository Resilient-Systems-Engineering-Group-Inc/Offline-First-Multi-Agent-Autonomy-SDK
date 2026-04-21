//! YAML/JSON workflow definition parser.
//!
//! Allows defining workflows in a human-readable format.

use crate::model::{
    Workflow, WorkflowTask, TaskType, WorkflowFailureStrategy,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use anyhow::{Result, anyhow};
use tracing::info;

/// YAML workflow definition (for parsing).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub workflow_id: String,
    pub name: String,
    pub description: Option<String>,
    pub version: Option<String>,
    pub tasks: Vec<TaskDefinition>,
    pub on_failure: Option<String>,
    pub timeout_secs: Option<u64>,
    pub tags: Option<Vec<String>>,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDefinition {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub task_type: Option<String>,
    pub dependencies: Option<Vec<String>>,
    pub timeout_secs: Option<u64>,
    pub retries: Option<u32>,
    pub retry_delay_ms: Option<u64>,
    pub required_capabilities: Option<Vec<String>>,
    pub parameters: Option<HashMap<String, String>>,
    pub on_success: Option<String>,
    pub on_failure: Option<String>,
}

/// Workflow parser for YAML/JSON definitions.
pub struct WorkflowParser;

impl WorkflowParser {
    /// Parse a workflow from a YAML string.
    pub fn parse_yaml(yaml_content: &str) -> Result<Workflow> {
        let definition: WorkflowDefinition = serde_yaml::from_str(yaml_content)
            .map_err(|e| anyhow!("Failed to parse YAML: {}", e))?;

        Self::convert_definition(definition)
    }

    /// Parse a workflow from a JSON string.
    pub fn parse_json(json_content: &str) -> Result<Workflow> {
        let definition: WorkflowDefinition = serde_json::from_str(json_content)
            .map_err(|e| anyhow!("Failed to parse JSON: {}", e))?;

        Self::convert_definition(definition)
    }

    /// Load and parse a workflow from a file.
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Workflow> {
        let content = fs::read_to_string(&path)
            .map_err(|e| anyhow!("Failed to read file: {}", e))?;

        let path_str = path.as_ref().to_string_lossy();
        info!("Loading workflow from: {}", path_str);

        if path_str.ends_with(".yaml") || path_str.ends_with(".yml") {
            Self::parse_yaml(&content)
        } else if path_str.ends_with(".json") {
            Self::parse_json(&content)
        } else {
            // Try YAML first, then JSON
            Self::parse_yaml(&content)
                .or_else(|_| Self::parse_json(&content))
                .map_err(|e| anyhow!("Failed to parse as YAML or JSON: {}", e))
        }
    }

    /// Convert a parsed definition into a Workflow.
    fn convert_definition(definition: WorkflowDefinition) -> Result<Workflow> {
        let mut workflow = Workflow::new(&definition.workflow_id, &definition.name);

        if let Some(desc) = definition.description {
            workflow.description = Some(desc);
        }

        if let Some(version) = definition.version {
            workflow.version = version;
        }

        if let Some(strategy) = definition.on_failure {
            workflow.on_failure = Self::parse_failure_strategy(&strategy)?;
        }

        if let Some(timeout) = definition.timeout_secs {
            workflow.timeout_secs = Some(timeout);
        }

        if let Some(tags) = definition.tags {
            workflow.tags = tags;
        }

        if let Some(metadata) = definition.metadata {
            workflow.metadata = metadata;
        }

        // Parse tasks
        for task_def in definition.tasks {
            let task = Self::parse_task(task_def)?;
            workflow.tasks.push(task);
        }

        Ok(workflow)
    }

    /// Parse a single task from definition.
    fn parse_task(def: TaskDefinition) -> Result<WorkflowTask> {
        let task_type = def.task_type
            .as_ref()
            .map(|t| Self::parse_task_type(t))
            .unwrap_or(TaskType::Action);

        let mut task = WorkflowTask::new(&def.id, &def.name, task_type);

        if let Some(desc) = def.description {
            task = task.with_description(&desc);
        }

        if let Some(deps) = def.dependencies {
            for dep in deps {
                task = task.with_dependency(&dep);
            }
        }

        if let Some(timeout) = def.timeout_secs {
            task = task.with_timeout(timeout);
        }

        if let Some(retries) = def.retries {
            task = task.with_retries(retries);
        }

        if let Some(delay) = def.retry_delay_ms {
            task.retry_delay_ms = delay;
        }

        if let Some(caps) = def.required_capabilities {
            task.required_capabilities = caps;
        }

        if let Some(params) = def.parameters {
            task.parameters = params;
        }

        if let Some(success) = def.on_success {
            task.on_success = Some(success);
        }

        if let Some(failure) = def.on_failure {
            task.on_failure = Some(failure);
        }

        Ok(task)
    }

    /// Parse task type from string.
    fn parse_task_type(s: &str) -> TaskType {
        match s.to_lowercase().as_str() {
            "setup" => TaskType::Setup,
            "action" => TaskType::Action,
            "condition" => TaskType::Condition,
            "parallel" => TaskType::Parallel,
            "join" => TaskType::Join,
            "teardown" => TaskType::Teardown,
            "custom" => TaskType::Custom,
            _ => TaskType::Action,
        }
    }

    /// Parse failure strategy from string.
    fn parse_failure_strategy(s: &str) -> Result<WorkflowFailureStrategy> {
        match s.to_lowercase().as_str() {
            "fail" => Ok(WorkflowFailureStrategy::Fail),
            "continue" => Ok(WorkflowFailureStrategy::Continue),
            "rollback" => Ok(WorkflowFailureStrategy::Rollback),
            "pause" => Ok(WorkflowFailureStrategy::Pause),
            _ => Err(anyhow!("Unknown failure strategy: {}", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_yaml_workflow() {
        let yaml = r#"
workflow_id: exploration_task
name: Area Exploration
description: Collaborative mapping of unknown area
version: 1.0.0

tasks:
  - id: initialize
    name: Initialize
    type: setup
    timeout_secs: 30
    retries: 3

  - id: explore_zone_a
    name: Explore Zone A
    type: action
    dependencies: [initialize]
    timeout_secs: 120
    required_capabilities: [navigation, lidar]

  - id: explore_zone_b
    name: Explore Zone B
    type: action
    dependencies: [initialize]
    timeout_secs: 120
    required_capabilities: [navigation, lidar]

  - id: merge_maps
    name: Merge Maps
    type: action
    dependencies: [explore_zone_a, explore_zone_b]
    timeout_secs: 60

  - id: cleanup
    name: Cleanup
    type: teardown
    dependencies: [merge_maps]
    timeout_secs: 30

on_failure: rollback
timeout_secs: 600
tags:
  - exploration
  - mapping
"#;

        let workflow = WorkflowParser::parse_yaml(yaml).unwrap();

        assert_eq!(workflow.id, "exploration_task");
        assert_eq!(workflow.name, "Area Exploration");
        assert_eq!(workflow.tasks.len(), 5);
        assert_eq!(workflow.on_failure, WorkflowFailureStrategy::Rollback);
    }

    #[test]
    fn test_parse_json_workflow() {
        let json = r#"
{
  "workflow_id": "test_workflow",
  "name": "Test Workflow",
  "description": "A test workflow",
  "tasks": [
    {
      "id": "task1",
      "name": "Task 1",
      "type": "setup"
    },
    {
      "id": "task2",
      "name": "Task 2",
      "type": "action",
      "dependencies": ["task1"]
    }
  ],
  "on_failure": "continue"
}
"#;

        let workflow = WorkflowParser::parse_json(json).unwrap();

        assert_eq!(workflow.id, "test_workflow");
        assert_eq!(workflow.tasks.len(), 2);
        assert_eq!(workflow.on_failure, WorkflowFailureStrategy::Continue);
    }
}
