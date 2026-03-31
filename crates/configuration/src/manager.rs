//! Configuration manager with hot‑reload capabilities.

use crate::error::Error;
use crate::loader::Loader;
use crate::schema::Configuration;
use crate::validator::Validator;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Manages configuration state and hot‑reload.
#[derive(Debug)]
pub struct ConfigurationManager {
    config: Arc<RwLock<Configuration>>,
    path: String,
    loader: Loader,
    validator: Validator,
}

impl ConfigurationManager {
    /// Create a new configuration manager by loading from a file.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let loader = Loader::new();
        let validator = Validator::new();
        let config = loader.load(&path)?;
        validator.validate(&config)?;
        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            path: path.as_ref().to_string_lossy().into_owned(),
            loader,
            validator,
        })
    }

    /// Get a read guard to the current configuration.
    pub async fn get(&self) -> tokio::sync::RwLockReadGuard<'_, Configuration> {
        self.config.read().await
    }

    /// Reload configuration from disk.
    pub async fn reload(&self) -> Result<(), Error> {
        let new_config = self.loader.load(&self.path)?;
        self.validator.validate(&new_config)?;
        *self.config.write().await = new_config;
        Ok(())
    }

    /// Start a background task that watches the configuration file for changes
    /// and automatically reloads (requires `watch` feature).
    #[cfg(feature = "watch")]
    pub async fn start_watcher(&self) -> Result<(), Error> {
        use notify::{RecommendedWatcher, RecursiveMode, Watcher};
        use std::sync::mpsc;
        use std::time::Duration;
        use tokio::time::sleep;

        let (tx, rx) = mpsc::channel();
        let mut watcher: RecommendedWatcher = Watcher::new(tx, Duration::from_secs(2))
            .map_err(|e| Error::Watch(format!("Failed to create watcher: {}", e)))?;

        watcher
            .watch(&self.path, RecursiveMode::NonRecursive)
            .map_err(|e| Error::Watch(format!("Failed to watch file: {}", e)))?;

        let config = self.config.clone();
        let loader = self.loader.clone();
        let validator = self.validator.clone();
        let path = self.path.clone();

        tokio::spawn(async move {
            while let Ok(event) = rx.recv() {
                // Debounce: wait a bit before reloading.
                sleep(Duration::from_millis(500)).await;
                match loader.load(&path) {
                    Ok(new_config) => {
                        if let Err(e) = validator.validate(&new_config) {
                            tracing::error!("Configuration validation failed after change: {}", e);
                            continue;
                        }
                        *config.write().await = new_config;
                        tracing::info!("Configuration reloaded from {}", path);
                    }
                    Err(e) => tracing::error!("Failed to reload configuration: {}", e),
                }
            }
        });

        Ok(())
    }
}