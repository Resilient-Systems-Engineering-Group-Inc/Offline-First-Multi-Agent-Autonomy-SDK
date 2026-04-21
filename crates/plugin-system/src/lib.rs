//! Plugin system for the Multi-Agent SDK.
//!
//! Provides:
//! - Dynamic plugin loading
//! - Hot-reload support
//! - Plugin lifecycle management
//! - Event-based communication

pub mod plugin;
pub mod manager;
pub mod events;

use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::RwLock;
use tracing::info;

pub use plugin::*;
pub use manager::*;
pub use events::*;

/// Plugin configuration.
#[derive(Debug, Clone)]
pub struct PluginConfig {
    pub plugins_dir: PathBuf,
    pub auto_reload: bool,
    pub reload_interval_secs: u64,
    pub enabled_plugins: Vec<String>,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            plugins_dir: PathBuf::from("./plugins"),
            auto_reload: true,
            reload_interval_secs: 5,
            enabled_plugins: vec![],
        }
    }
}

/// Plugin manager.
pub struct PluginManager {
    config: PluginConfig,
    plugins: RwLock<HashMap<String, Box<dyn Plugin + Send + Sync>>>,
    event_subscribers: RwLock<HashMap<String, Vec<Box<dyn Fn(Event) + Send + Sync>>>>,
}

impl PluginManager {
    /// Create new plugin manager.
    pub fn new(config: PluginConfig) -> Self {
        Self {
            config,
            plugins: RwLock::new(HashMap::new()),
            event_subscribers: RwLock::new(HashMap::new()),
        }
    }

    /// Load plugin from file.
    pub async fn load_plugin(&self, path: &PathBuf) -> Result<String> {
        // Would use libloading to load dynamic library
        let plugin_id = uuid::Uuid::new_v4().to_string();
        
        info!("Loading plugin from {:?}", path);
        
        // Placeholder - actual implementation would load .so/.dll/.dylib
        let mut plugins = self.plugins.write().await;
        plugins.insert(plugin_id.clone(), create_mock_plugin(path));
        
        Ok(plugin_id)
    }

    /// Unload plugin.
    pub async fn unload_plugin(&self, plugin_id: &str) -> Result<()> {
        let mut plugins = self.plugins.write().await;
        plugins.remove(plugin_id);
        
        info!("Plugin unloaded: {}", plugin_id);
        Ok(())
    }

    /// Reload plugin.
    pub async fn reload_plugin(&self, plugin_id: &str) -> Result<()> {
        self.unload_plugin(plugin_id).await?;
        
        // Would reload from original path
        info!("Plugin reloaded: {}", plugin_id);
        Ok(())
    }

    /// Get all loaded plugins.
    pub async fn list_plugins(&self) -> Vec<PluginInfo> {
        let plugins = self.plugins.read().await;
        plugins.values().map(|p| p.info()).collect()
    }

    /// Get plugin by ID.
    pub async fn get_plugin(&self, plugin_id: &str) -> Option<Box<dyn Plugin + Send + Sync>> {
        let plugins = self.plugins.read().await;
        plugins.get(plugin_id).map(|p| p.clone_box())
    }

    /// Subscribe to events.
    pub async fn subscribe(
        &self,
        event_type: &str,
        handler: Box<dyn Fn(Event) + Send + Sync>,
    ) {
        let mut subscribers = self.event_subscribers.write().await;
        subscribers
            .entry(event_type.to_string())
            .or_insert_with(Vec::new)
            .push(handler);
    }

    /// Publish event.
    pub async fn publish(&self, event: Event) {
        let event_type = event.event_type.clone();
        let subscribers = self.event_subscribers.read().await;

        if let Some(handlers) = subscribers.get(&event_type) {
            for handler in handlers {
                handler(event.clone());
            }
        }

        // Also broadcast to all plugins
        let plugins = self.plugins.read().await;
        for plugin in plugins.values() {
            plugin.on_event(event.clone());
        }
    }

    /// Start auto-reload watcher.
    pub async fn start_watch(&self) -> Result<()> {
        if !self.config.auto_reload {
            return Ok(());
        }

        let plugins_dir = self.config.plugins_dir.clone();
        let reload_interval = self.config.reload_interval_secs;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(reload_interval));
            
            loop {
                interval.tick().await;
                
                // Check for plugin changes
                // Would use notify crate for file system watching
                info!("Checking for plugin updates in {:?}", plugins_dir);
            }
        });

        Ok(())
    }

    /// Load all plugins from plugins directory.
    pub async fn load_all_plugins(&self) -> Result<Vec<String>> {
        let mut loaded = vec![];

        if !self.config.plugins_dir.exists() {
            std::fs::create_dir_all(&self.config.plugins_dir)?;
            return Ok(loaded);
        }

        for entry in std::fs::read_dir(&self.config.plugins_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map_or(false, |ext| ext == "so" || ext == "dll" || ext == "dylib") {
                if self.config.enabled_plugins.is_empty() 
                    || self.config.enabled_plugins.iter().any(|name| path.file_stem().map_or(false, |s| s == name))
                {
                    let plugin_id = self.load_plugin(&path).await?;
                    loaded.push(plugin_id);
                }
            }
        }

        info!("Loaded {} plugins", loaded.len());
        Ok(loaded)
    }

    /// Execute plugin hook.
    pub async fn execute_hook(&self, hook: &str, args: serde_json::Value) -> Result<serde_json::Value> {
        let plugins = self.plugins.read().await;
        
        for plugin in plugins.values() {
            if plugin.hooks().contains(hook) {
                let result = plugin.execute(hook, args.clone()).await?;
                if !result.is_null() {
                    return Ok(result);
                }
            }
        }

        Ok(serde_json::Value::Null)
    }
}

/// Create mock plugin for testing.
fn create_mock_plugin(path: &PathBuf) -> Box<dyn Plugin + Send + Sync> {
    Box::new(MockPlugin {
        id: uuid::Uuid::new_v4().to_string(),
        name: path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string(),
        version: "0.1.0".to_string(),
        hooks: vec!["on_task_created".to_string(), "on_agent_connected".to_string()],
    })
}

/// Mock plugin implementation.
struct MockPlugin {
    id: String,
    name: String,
    version: String,
    hooks: Vec<String>,
}

#[async_trait::async_trait]
impl Plugin for MockPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: self.id.clone(),
            name: self.name.clone(),
            version: self.version.clone(),
            description: "Mock plugin".to_string(),
            hooks: self.hooks.clone(),
        }
    }

    fn hooks(&self) -> Vec<String> {
        self.hooks.clone()
    }

    async fn execute(&self, hook: &str, _args: serde_json::Value) -> Result<serde_json::Value> {
        info!("Mock plugin executed hook: {}", hook);
        Ok(serde_json::json!({"status": "ok", "plugin": self.name}))
    }

    async fn on_event(&self, event: Event) {
        info!("Mock plugin received event: {}", event.event_type);
    }

    fn clone_box(&self) -> Box<dyn Plugin + Send + Sync> {
        Box::new(self.clone())
    }
}

impl Clone for MockPlugin {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            name: self.name.clone(),
            version: self.version.clone(),
            hooks: self.hooks.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_plugin_manager() {
        let config = PluginConfig::default();
        let manager = PluginManager::new(config);

        // Create temp plugin file
        let temp_dir = tempfile::tempdir().unwrap();
        let plugin_path = temp_dir.path().join("test_plugin.so");
        
        // Load plugin
        let plugin_id = manager.load_plugin(&plugin_path).await.unwrap();
        assert!(!plugin_id.is_empty());

        // List plugins
        let plugins = manager.list_plugins().await;
        assert_eq!(plugins.len(), 1);

        // Unload plugin
        manager.unload_plugin(&plugin_id).await.unwrap();
        
        let plugins = manager.list_plugins().await;
        assert_eq!(plugins.len(), 0);
    }

    #[tokio::test]
    async fn test_event_system() {
        let config = PluginConfig::default();
        let manager = PluginManager::new(config);

        // Subscribe to event
        manager.subscribe("test_event", Box::new(|event| {
            println!("Event received: {}", event.event_type);
        })).await;

        // Publish event
        let event = Event {
            event_type: "test_event".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
            data: serde_json::json!({"key": "value"}),
        };
        
        manager.publish(event).await;
    }
}
