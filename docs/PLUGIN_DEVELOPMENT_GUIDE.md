# Plugin Development Guide

## Overview

The SDK plugin system allows you to extend functionality without modifying the core codebase. Plugins can:

- ✅ **Intercept events** - Task creation, agent connection, workflow execution
- ✅ **Modify behavior** - Hook into task planning, resource allocation
- ✅ **Add new features** - Custom algorithms, integrations
- ✅ **Hot-reload** - Update plugins without restarting

## Getting Started

### Plugin Structure

A plugin is a dynamic library (`.so`, `.dll`, `.dylib`) that implements the `Plugin` trait:

```rust
use plugin_system::{Plugin, PluginInfo, Event};
use async_trait::async_trait;

pub struct MyPlugin {
    id: String,
    name: String,
}

#[async_trait]
impl Plugin for MyPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: self.id.clone(),
            name: self.name.clone(),
            version: "1.0.0".to_string(),
            description: "My custom plugin".to_string(),
            hooks: vec!["on_task_created".to_string()],
        }
    }

    fn hooks(&self) -> Vec<String> {
        vec!["on_task_created".to_string()]
    }

    async fn execute(&self, hook: &str, args: serde_json::Value) -> Result<serde_json::Value> {
        match hook {
            "on_task_created" => {
                println!("Task created: {}", args["task_id"]);
                Ok(serde_json::json!({"processed": true}))
            }
            _ => Ok(serde_json::Value::Null),
        }
    }

    async fn on_event(&self, event: Event) {
        println!("Event: {}", event.event_type);
    }

    fn clone_box(&self) -> Box<dyn Plugin + Send + Sync> {
        Box::new(self.clone())
    }
}
```

### Cargo Configuration

```toml
[package]
name = "my-sdk-plugin"
version = "1.0.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]  # Dynamic library

[dependencies]
plugin-system = "0.1"
async-trait = "0.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

## Available Hooks

### Task Hooks

| Hook | Called When | Arguments |
|------|-------------|-----------|
| `on_task_created` | New task created | `{"task_id": "...", "description": "..."}` |
| `on_task_assigned` | Task assigned to agent | `{"task_id": "...", "agent_id": "..."}` |
| `on_task_started` | Task execution started | `{"task_id": "..."}` |
| `on_task_completed` | Task completed successfully | `{"task_id": "...", "result": {...}}` |
| `on_task_failed` | Task failed | `{"task_id": "...", "error": "..."}` |

### Agent Hooks

| Hook | Called When | Arguments |
|------|-------------|-----------|
| `on_agent_connected` | Agent joins mesh | `{"agent_id": "..."}` |
| `on_agent_disconnected` | Agent leaves mesh | `{"agent_id": "..."}` |
| `on_agent_heartbeat` | Agent heartbeat received | `{"agent_id": "...", "metrics": {...}}` |

### Workflow Hooks

| Hook | Called When | Arguments |
|------|-------------|-----------|
| `on_workflow_started` | Workflow instance started | `{"workflow_id": "..."}` |
| `on_workflow_step` | Workflow step executed | `{"workflow_id": "...", "step": "..."}` |
| `on_workflow_completed` | Workflow completed | `{"workflow_id": "...", "output": {...}}` |

### System Hooks

| Hook | Called When | Arguments |
|------|-------------|-----------|
| `on_plugin_loaded` | Plugin loaded | `{"plugin_id": "..."}` |
| `on_plugin_unloaded` | Plugin unloaded | `{"plugin_id": "..."}` |
| `on_config_changed` | Configuration updated | `{"key": "...", "value": "..."}` |

## Examples

### Task Logging Plugin

```rust
use plugin_system::{Plugin, PluginInfo, Event};
use async_trait::async_trait;
use chrono::Utc;

pub struct TaskLoggerPlugin;

#[async_trait]
impl Plugin for TaskLoggerPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Task Logger".to_string(),
            version: "1.0.0".to_string(),
            description: "Logs all task events".to_string(),
            hooks: vec![
                "on_task_created".to_string(),
                "on_task_completed".to_string(),
                "on_task_failed".to_string(),
            ],
        }
    }

    fn hooks(&self) -> Vec<String> {
        vec![
            "on_task_created".to_string(),
            "on_task_completed".to_string(),
            "on_task_failed".to_string(),
        ]
    }

    async fn execute(&self, hook: &str, args: serde_json::Value) -> Result<serde_json::Value> {
        let timestamp = Utc::now().to_rfc3339();
        
        match hook {
            "on_task_created" => {
                println!("[{}] Task created: {} - {}", 
                    timestamp,
                    args["task_id"],
                    args["description"]
                );
            }
            "on_task_completed" => {
                println!("[{}] Task completed: {} - {:?}", 
                    timestamp,
                    args["task_id"],
                    args["result"]
                );
            }
            "on_task_failed" => {
                eprintln!("[{}] Task failed: {} - {}", 
                    timestamp,
                    args["task_id"],
                    args["error"]
                );
            }
            _ => {}
        }

        Ok(serde_json::Value::Null)
    }

    async fn on_event(&self, event: Event) {
        println!("Event received: {}", event.event_type);
    }

    fn clone_box(&self) -> Box<dyn Plugin + Send + Sync> {
        Box::new(self.clone())
    }
}

impl Clone for TaskLoggerPlugin {
    fn clone(&self) -> Self {
        Self
    }
}
```

### Custom Task Planner Plugin

```rust
use plugin_system::{Plugin, PluginInfo, Event};
use async_trait::async_trait;

pub struct CustomPlannerPlugin;

#[async_trait]
impl Plugin for CustomPlannerPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Custom Planner".to_string(),
            version: "1.0.0".to_string(),
            description: "Custom task planning algorithm".to_string(),
            hooks: vec!["on_task_planning".to_string()],
        }
    }

    fn hooks(&self) -> Vec<String> {
        vec!["on_task_planning".to_string()]
    }

    async fn execute(&self, hook: &str, args: serde_json::Value) -> Result<serde_json::Value> {
        if hook == "on_task_planning" {
            let tasks = args["tasks"].as_array().unwrap();
            
            // Custom planning logic
            let mut assignments = serde_json::Map::new();
            
            for task in tasks {
                let task_id = task["id"].as_str().unwrap();
                // Your custom algorithm here
                assignments.insert(task_id.to_string(), json!("custom-agent"));
            }

            return Ok(serde_json::json!({
                "assignments": assignments,
                "algorithm": "custom"
            }));
        }

        Ok(serde_json::Value::Null)
    }

    async fn on_event(&self, event: Event) {}

    fn clone_box(&self) -> Box<dyn Plugin + Send + Sync> {
        Box::new(self.clone())
    }
}

impl Clone for CustomPlannerPlugin {
    fn clone(&self) -> Self {
        Self
    }
}
```

### Monitoring Plugin

```rust
use plugin_system::{Plugin, PluginInfo, Event};
use async_trait::async_trait;
use std::sync::atomic::{AtomicU64, Ordering};

pub struct MonitoringPlugin {
    task_count: AtomicU64,
    completed_count: AtomicU64,
}

impl MonitoringPlugin {
    pub fn new() -> Self {
        Self {
            task_count: AtomicU64::new(0),
            completed_count: AtomicU64::new(0),
        }
    }
}

#[async_trait]
impl Plugin for MonitoringPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Monitoring".to_string(),
            version: "1.0.0".to_string(),
            description: "Task monitoring and metrics".to_string(),
            hooks: vec![
                "on_task_created".to_string(),
                "on_task_completed".to_string(),
            ],
        }
    }

    fn hooks(&self) -> Vec<String> {
        vec![
            "on_task_created".to_string(),
            "on_task_completed".to_string(),
        ]
    }

    async fn execute(&self, hook: &str, _args: serde_json::Value) -> Result<serde_json::Value> {
        match hook {
            "on_task_created" => {
                self.task_count.fetch_add(1, Ordering::SeqCst);
            }
            "on_task_completed" => {
                self.completed_count.fetch_add(1, Ordering::SeqCst);
            }
            _ => {}
        }

        Ok(serde_json::json!({
            "total_tasks": self.task_count.load(Ordering::SeqCst),
            "completed": self.completed_count.load(Ordering::SeqCst),
            "pending": self.task_count.load(Ordering::SeqCst) - self.completed_count.load(Ordering::SeqCst)
        }))
    }

    async fn on_event(&self, event: Event) {}

    fn clone_box(&self) -> Box<dyn Plugin + Send + Sync> {
        Box::new(self.clone())
    }
}

impl Clone for MonitoringPlugin {
    fn clone(&self) -> Self {
        Self {
            task_count: AtomicU64::new(self.task_count.load(Ordering::SeqCst)),
            completed_count: AtomicU64::new(self.completed_count.load(Ordering::SeqCst)),
        }
    }
}
```

## Building Plugins

### Build Command

```bash
# Build plugin
cargo build --release

# Plugin will be at: target/release/libmy_plugin.so
```

### Install Plugin

```bash
# Copy to plugins directory
cp target/release/libmy_plugin.so ./plugins/

# Or specify custom directory
export SDK_PLUGINS_DIR=/path/to/plugins
```

## Configuration

### Plugin Configuration File

```json
{
  "plugins": {
    "task_logger": {
      "enabled": true,
      "log_level": "info",
      "output_file": "/var/log/sdk/tasks.log"
    },
    "custom_planner": {
      "enabled": true,
      "algorithm": "priority-based",
      "max_tasks": 100
    }
  }
}
```

## Hot-Reload

### Enable Auto-Reload

```rust
use plugin_system::{PluginManager, PluginManagerConfig};
use std::path::PathBuf;

let config = PluginManagerConfig {
    plugins_dir: PathBuf::from("./plugins"),
    auto_reload: true,
    reload_interval_secs: 5,
    enabled_plugins: vec![],
    max_plugins: 100,
};

let manager = PluginManager::new(config);
manager.start_watch().await?;
```

### Manual Reload

```bash
# Using CLI
sdk plugin reload plugin-id

# Or through API
curl -X POST http://localhost:3000/api/plugins/plugin-id/reload
```

## Debugging

### Enable Plugin Logging

```bash
RUST_LOG=plugin_system=debug sdk
```

### Plugin Error Handling

```rust
async fn execute(&self, hook: &str, args: serde_json::Value) -> Result<serde_json::Value> {
    match hook {
        "on_task_created" => {
            // Validate input
            if args["task_id"].is_null() {
                return Err(anyhow::anyhow!("Missing task_id"));
            }
            
            // Your logic here
            Ok(serde_json::json!({"status": "ok"}))
        }
        _ => Ok(serde_json::Value::Null),
    }
}
```

## Best Practices

1. **Keep plugins small** - Single responsibility
2. **Handle errors gracefully** - Don't crash the host
3. **Use async carefully** - Avoid blocking operations
4. **Clean up resources** - Implement `on_unload` hook
5. **Version your plugins** - Track compatibility
6. **Test thoroughly** - Unit and integration tests
7. **Document hooks** - Clear API documentation

## Next Steps

- [Plugin API Reference](./PLUGIN_API_REFERENCE.md)
- [Plugin Examples](./PLUGIN_EXAMPLES.md)
- [Troubleshooting](./PLUGIN_TROUBLESHOOTING.md)

---

*Last Updated: 2026-03-27*
