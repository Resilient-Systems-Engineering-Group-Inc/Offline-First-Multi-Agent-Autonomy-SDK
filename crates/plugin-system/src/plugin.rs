//! Plugin trait and implementations.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Plugin trait that all plugins must implement.
#[async_trait]
pub trait Plugin: Send + Sync {
    /// Get plugin information.
    fn info(&self) -> PluginInfo;

    /// Get list of supported hooks.
    fn hooks(&self) -> Vec<String>;

    /// Execute a hook with arguments.
    async fn execute(&self, hook: &str, args: serde_json::Value) -> anyhow::Result<serde_json::Value>;

    /// Handle plugin events.
    async fn on_event(&self, event: crate::events::Event);

    /// Clone plugin (for hot-reload).
    fn clone_box(&self) -> Box<dyn Plugin + Send + Sync>;
}

/// Plugin metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub hooks: Vec<String>,
}

/// Plugin lifecycle hooks.
pub trait PluginHooks {
    /// Called when plugin is loaded.
    fn on_load(&self) {}

    /// Called when plugin is unloaded.
    fn on_unload(&self) {}

    /// Called before task execution.
    fn on_task_before(&self, task_id: &str) {}

    /// Called after task execution.
    fn on_task_after(&self, task_id: &str, result: &serde_json::Value) {}

    /// Called when agent connects.
    fn on_agent_connect(&self, agent_id: &str) {}

    /// Called when agent disconnects.
    fn on_agent_disconnect(&self, agent_id: &str) {}

    /// Called when workflow starts.
    fn on_workflow_start(&self, workflow_id: &str) {}

    /// Called when workflow completes.
    fn on_workflow_complete(&self, workflow_id: &str) {}
}

/// Example plugin implementation.
pub struct ExamplePlugin {
    info: PluginInfo,
}

#[async_trait]
impl Plugin for ExamplePlugin {
    fn info(&self) -> PluginInfo {
        self.info.clone()
    }

    fn hooks(&self) -> Vec<String> {
        vec![
            "on_task_created".to_string(),
            "on_task_completed".to_string(),
            "on_agent_connected".to_string(),
        ]
    }

    async fn execute(&self, hook: &str, args: serde_json::Value) -> anyhow::Result<serde_json::Value> {
        match hook {
            "on_task_created" => {
                let task_id = args["task_id"].as_str().unwrap_or("unknown");
                println!("Task created: {}", task_id);
                Ok(serde_json::json!({"processed": true}))
            }
            "on_task_completed" => {
                let result = args["result"].clone();
                Ok(serde_json::json!({"processed": true, "result": result}))
            }
            _ => Ok(serde_json::Value::Null),
        }
    }

    async fn on_event(&self, event: crate::events::Event) {
        println!("Plugin received event: {}", event.event_type);
    }

    fn clone_box(&self) -> Box<dyn Plugin + Send + Sync> {
        Box::new(self.clone())
    }
}

impl Clone for ExamplePlugin {
    fn clone(&self) -> Self {
        Self {
            info: self.info.clone(),
        }
    }
}

impl Default for ExamplePlugin {
    fn default() -> Self {
        Self {
            info: PluginInfo {
                id: uuid::Uuid::new_v4().to_string(),
                name: "Example Plugin".to_string(),
                version: "1.0.0".to_string(),
                description: "Example plugin for demonstration".to_string(),
                hooks: vec![
                    "on_task_created".to_string(),
                    "on_task_completed".to_string(),
                ],
            },
        }
    }
}
