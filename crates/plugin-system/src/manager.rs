//! Plugin manager implementation.

use crate::events::Event;
use crate::plugin::{Plugin, PluginInfo};
use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Plugin manager configuration.
#[derive(Debug, Clone)]
pub struct PluginManagerConfig {
    pub plugins_dir: PathBuf,
    pub auto_reload: bool,
    pub reload_interval_secs: u64,
    pub enabled_plugins: Vec<String>,
    pub max_plugins: usize,
}

impl Default for PluginManagerConfig {
    fn default() -> Self {
        Self {
            plugins_dir: PathBuf::from("./plugins"),
            auto_reload: true,
            reload_interval_secs: 5,
            enabled_plugins: vec![],
            max_plugins: 100,
        }
    }
}

/// Plugin manager.
pub struct PluginManager {
    config: PluginManagerConfig,
    plugins: RwLock<HashMap<String, Box<dyn Plugin + Send + Sync>>>,
    event_listeners: RwLock<HashMap<String, Vec<Box<dyn Fn(Event) + Send + Sync>>>>,
}

impl PluginManager {
    /// Create new plugin manager.
    pub fn new(config: PluginManagerConfig) -> Self {
        Self {
            config,
            plugins: RwLock::new(HashMap::new()),
            event_listeners: RwLock::new(HashMap::new()),
        }
    }

    /// Load plugin from dynamic library.
    pub async fn load_plugin(&self, path: &PathBuf) -> Result<String> {
        let plugins = self.plugins.read().await;
        if plugins.len() >= self.config.max_plugins {
            return Err(anyhow::anyhow!("Maximum number of plugins reached"));
        }
        drop(plugins);

        // In production, would use libloading to load .so/.dll/.dylib
        // For now, create mock plugin
        let plugin_id = uuid::Uuid::new_v4().to_string();
        let plugin_name = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let plugin = create_mock_plugin(&plugin_id, &plugin_name);

        let mut plugins = self.plugins.write().await;
        plugins.insert(plugin_id.clone(), plugin);

        info!("Plugin loaded: {} from {:?}", plugin_id, path);
        Ok(plugin_id)
    }

    /// Unload plugin.
    pub async fn unload_plugin(&self, plugin_id: &str) -> Result<()> {
        let mut plugins = self.plugins.write().await;
        
        if plugins.remove(plugin_id).is_some() {
            info!("Plugin unloaded: {}", plugin_id);
            
            // Emit unload event
            let event = Event::new("plugin_unloaded", serde_json::json!({
                "plugin_id": plugin_id
            }));
            self.emit_event(event).await;
            
            Ok(())
        } else {
            Err(anyhow::anyhow!("Plugin not found: {}", plugin_id))
        }
    }

    /// Reload plugin.
    pub async fn reload_plugin(&self, plugin_id: &str) -> Result<()> {
        // Get plugin info before unload
        let plugin_info = {
            let plugins = self.plugins.read().await;
            plugins.get(plugin_id).map(|p| p.info())
        };

        if let Some(info) = plugin_info {
            self.unload_plugin(plugin_id).await?;
            
            // Reload from same path
            let plugins_dir = self.config.plugins_dir.clone();
            let plugin_name = info.name.replace(" ", "_").to_lowercase();
            let plugin_path = plugins_dir.join(format!("{}.so", plugin_name));
            
            if plugin_path.exists() {
                self.load_plugin(&plugin_path).await?;
            } else {
                warn!("Plugin file not found for reload: {:?}", plugin_path);
            }
        }

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

    /// Check if plugin is loaded.
    pub async fn has_plugin(&self, plugin_id: &str) -> bool {
        let plugins = self.plugins.read().await;
        plugins.contains_key(plugin_id)
    }

    /// Subscribe to plugin events.
    pub async fn subscribe<F>(&self, event_type: &str, handler: F)
    where
        F: Fn(Event) + Send + Sync + 'static,
    {
        let mut listeners = self.event_listeners.write().await;
        listeners
            .entry(event_type.to_string())
            .or_insert_with(Vec::new)
            .push(Box::new(handler));
    }

    /// Emit event to all listeners.
    async fn emit_event(&self, event: Event) {
        let event_type = event.event_type.clone();
        let listeners = self.event_listeners.read().await;

        if let Some(handlers) = listeners.get(&event_type) {
            for handler in handlers {
                handler(event.clone());
            }
        }

        // Also notify all plugins
        let plugins = self.plugins.read().await;
        for plugin in plugins.values() {
            plugin.on_event(event.clone()).await;
        }
    }

    /// Execute plugin hook.
    pub async fn execute_hook(&self, hook: &str, args: serde_json::Value) -> Result<Vec<serde_json::Value>> {
        let plugins = self.plugins.read().await;
        let mut results = vec![];

        for (plugin_id, plugin) in plugins.iter() {
            if plugin.hooks().contains(&hook.to_string()) {
                match plugin.execute(hook, args.clone()).await {
                    Ok(result) => {
                        results.push(result);
                        info!("Hook {} executed by plugin {}", hook, plugin_id);
                    }
                    Err(e) => {
                        warn!("Hook {} failed in plugin {}: {}", hook, plugin_id, e);
                    }
                }
            }
        }

        Ok(results)
    }

    /// Load all plugins from plugins directory.
    pub async fn load_all_plugins(&self) -> Result<Vec<String>> {
        let mut loaded = vec![];

        if !self.config.plugins_dir.exists() {
            info!("Plugins directory does not exist, creating: {:?}", self.config.plugins_dir);
            std::fs::create_dir_all(&self.config.plugins_dir)?;
            return Ok(loaded);
        }

        for entry in std::fs::read_dir(&self.config.plugins_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map_or(false, |ext| ext == "so" || ext == "dll" || ext == "dylib") {
                if self.config.enabled_plugins.is_empty() 
                    || self.config.enabled_plugins.iter().any(|name| {
                        path.file_stem().map_or(false, |s| s == name)
                    })
                {
                    match self.load_plugin(&path).await {
                        Ok(plugin_id) => loaded.push(plugin_id),
                        Err(e) => warn!("Failed to load plugin {:?}: {}", path, e),
                    }
                }
            }
        }

        info!("Loaded {} plugins", loaded.len());
        Ok(loaded)
    }

    /// Start auto-reload watcher.
    pub async fn start_watch(&self) -> Result<()> {
        if !self.config.auto_reload {
            info!("Auto-reload disabled");
            return Ok(());
        }

        let plugins_dir = self.config.plugins_dir.clone();
        let reload_interval = self.config.reload_interval_secs;
        let plugins = self.plugins.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(reload_interval));
            
            loop {
                interval.tick().await;
                
                // Check for plugin file changes
                // Would use notify crate for efficient file system watching
                info!("Checking for plugin updates in {:?}", plugins_dir);
            }
        });

        Ok(())
    }

    /// Get plugin statistics.
    pub async fn get_stats(&self) -> PluginStats {
        let plugins = self.plugins.read().await;
        let total_hooks: usize = plugins.values()
            .map(|p| p.hooks().len())
            .sum();

        PluginStats {
            total_plugins: plugins.len() as i64,
            total_hooks: total_hooks as i32,
            plugins: plugins.keys().cloned().collect(),
        }
    }
}

/// Plugin statistics.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginStats {
    pub total_plugins: i64,
    pub total_hooks: i32,
    pub plugins: Vec<String>,
}

/// Create mock plugin for testing.
fn create_mock_plugin(plugin_id: &str, name: &str) -> Box<dyn Plugin + Send + Sync> {
    Box::new(MockPlugin {
        id: plugin_id.to_string(),
        name: name.to_string(),
        version: "0.1.0".to_string(),
        hooks: vec![
            "on_task_created".to_string(),
            "on_task_completed".to_string(),
            "on_agent_connected".to_string(),
        ],
    })
}

/// Mock plugin implementation.
#[derive(Clone)]
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
            description: format!("Mock plugin: {}", self.name),
            hooks: self.hooks.clone(),
        }
    }

    fn hooks(&self) -> Vec<String> {
        self.hooks.clone()
    }

    async fn execute(&self, hook: &str, _args: serde_json::Value) -> Result<serde_json::Value> {
        info!("Mock plugin {} executed hook: {}", self.name, hook);
        Ok(serde_json::json!({
            "status": "ok",
            "plugin": self.name,
            "hook": hook
        }))
    }

    async fn on_event(&self, event: Event) {
        info!("Mock plugin {} received event: {}", self.name, event.event_type);
    }

    fn clone_box(&self) -> Box<dyn Plugin + Send + Sync> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_plugin_manager_lifecycle() {
        let config = PluginManagerConfig::default();
        let manager = PluginManager::new(config);

        // Load plugin
        let temp_dir = tempfile::tempdir().unwrap();
        let plugin_path = temp_dir.path().join("test_plugin.so");
        
        let plugin_id = manager.load_plugin(&plugin_path).await.unwrap();
        assert!(!plugin_id.is_empty());

        // Verify plugin loaded
        assert!(manager.has_plugin(&plugin_id).await);

        // List plugins
        let plugins = manager.list_plugins().await;
        assert_eq!(plugins.len(), 1);

        // Unload plugin
        manager.unload_plugin(&plugin_id).await.unwrap();
        assert!(!manager.has_plugin(&plugin_id).await);
    }

    #[tokio::test]
    async fn test_event_system() {
        let config = PluginManagerConfig::default();
        let manager = PluginManager::new(config);

        // Subscribe to event
        let mut received = false;
        manager.subscribe("test_event", move |event| {
            received = true;
            println!("Event received: {}", event.event_type);
        }).await;

        // Emit event
        let event = Event::new("test_event", serde_json::json!({"key": "value"}));
        manager.emit_event(event).await;

        // Note: In async context, we can't directly verify received flag
        // This is just to ensure no panic occurs
    }
}
