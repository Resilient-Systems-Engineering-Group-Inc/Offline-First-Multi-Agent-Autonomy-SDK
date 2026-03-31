//! Dynamic configuration watching and hot‑reload.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::fs;
use tokio::sync::{mpsc, RwLock};
use tokio::time;
use tracing::{info, warn, error};

use crate::error::Error;
use crate::loader::{Loader, FileFormat};
use crate::manager::ConfigurationManager;
use crate::schema::Configuration;

/// Event emitted when configuration changes.
#[derive(Debug, Clone)]
pub enum ConfigEvent {
    /// Configuration file was updated.
    Updated(PathBuf),
    /// Configuration file was deleted.
    Deleted(PathBuf),
    /// Error occurred while watching.
    Error(String),
}

/// Watcher configuration.
#[derive(Debug, Clone)]
pub struct WatcherConfig {
    /// Polling interval in seconds (if using polling).
    pub poll_interval_secs: u64,
    /// Whether to use inotify (or similar) if available.
    pub use_inotify: bool,
    /// Maximum retries on error.
    pub max_retries: u32,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            poll_interval_secs: 5,
            use_inotify: true,
            max_retries: 3,
        }
    }
}

/// Watches configuration files for changes and triggers reload.
pub struct ConfigWatcher {
    config: WatcherConfig,
    loader: Loader,
    manager: Arc<RwLock<ConfigurationManager>>,
    event_tx: mpsc::UnboundedSender<ConfigEvent>,
    /// Map from path to last known modification time.
    last_mtimes: Arc<RwLock<HashMap<PathBuf, std::time::SystemTime>>>,
}

impl ConfigWatcher {
    /// Create a new config watcher.
    pub fn new(
        config: WatcherConfig,
        loader: Loader,
        manager: Arc<RwLock<ConfigurationManager>>,
    ) -> (Self, mpsc::UnboundedReceiver<ConfigEvent>) {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let watcher = Self {
            config,
            loader,
            manager,
            event_tx,
            last_mtimes: Arc::new(RwLock::new(HashMap::new())),
        };
        (watcher, event_rx)
    }

    /// Start watching a configuration file.
    pub async fn watch(&self, path: impl AsRef<Path>) -> Result<(), Error> {
        let path = path.as_ref().to_path_buf();
        if !path.exists() {
            return Err(Error::NotFound(path.to_string_lossy().to_string()));
        }

        // Record initial mtime.
        let metadata = fs::metadata(&path).await?;
        let mtime = metadata.modified()?;
        self.last_mtimes.write().await.insert(path.clone(), mtime);

        info!("Started watching configuration file: {}", path.display());
        Ok(())
    }

    /// Start the watcher loop (polling).
    pub async fn start_polling(&self) -> Result<(), Error> {
        info!("Starting configuration watcher (polling)");
        let mut interval = time::interval(Duration::from_secs(self.config.poll_interval_secs));

        loop {
            interval.tick().await;
            if let Err(e) = self.check_all().await {
                error!("Error while checking config files: {}", e);
            }
        }
    }

    /// Check all watched files for changes.
    async fn check_all(&self) -> Result<(), Error> {
        let paths: Vec<PathBuf> = {
            let mtimes = self.last_mtimes.read().await;
            mtimes.keys().cloned().collect()
        };

        for path in paths {
            match self.check_file(&path).await {
                Ok(changed) => {
                    if changed {
                        info!("Configuration file changed: {}", path.display());
                        self.event_tx
                            .send(ConfigEvent::Updated(path.clone()))
                            .map_err(|e| Error::Other(e.to_string()))?;
                        self.reload_file(&path).await?;
                    }
                }
                Err(e) => {
                    if e.is::<std::io::Error>() {
                        // File may have been deleted.
                        warn!("File may have been deleted: {}", path.display());
                        self.event_tx
                            .send(ConfigEvent::Deleted(path.clone()))
                            .map_err(|e| Error::Other(e.to_string()))?;
                        self.last_mtimes.write().await.remove(&path);
                    } else {
                        error!("Error checking file {}: {}", path.display(), e);
                    }
                }
            }
        }
        Ok(())
    }

    /// Check if a file has changed.
    async fn check_file(&self, path: &Path) -> Result<bool, Error> {
        let metadata = fs::metadata(path).await?;
        let mtime = metadata.modified()?;

        let mut last_mtimes = self.last_mtimes.write().await;
        let last = last_mtimes.get(path).copied();

        if let Some(last) = last {
            if mtime > last {
                last_mtimes.insert(path.to_path_buf(), mtime);
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            last_mtimes.insert(path.to_path_buf(), mtime);
            Ok(true)
        }
    }

    /// Reload a configuration file and update the manager.
    async fn reload_file(&self, path: &Path) -> Result<(), Error> {
        let config = self.loader.load_file(path).await?;
        let mut manager = self.manager.write().await;
        manager.update(config)?;
        info!("Configuration reloaded from {}", path.display());
        Ok(())
    }
}

/// Dynamic configuration subscriber.
pub struct ConfigSubscriber {
    event_rx: mpsc::UnboundedReceiver<ConfigEvent>,
    callbacks: Vec<Box<dyn Fn(&ConfigEvent) + Send + Sync>>,
}

impl ConfigSubscriber {
    /// Create a new subscriber.
    pub fn new(event_rx: mpsc::UnboundedReceiver<ConfigEvent>) -> Self {
        Self {
            event_rx,
            callbacks: Vec::new(),
        }
    }

    /// Register a callback to be called on configuration events.
    pub fn on_event<F>(&mut self, callback: F)
    where
        F: Fn(&ConfigEvent) + Send + Sync + 'static,
    {
        self.callbacks.push(Box::new(callback));
    }

    /// Start listening for events.
    pub async fn start(mut self) {
        while let Some(event) = self.event_rx.recv().await {
            for callback in &self.callbacks {
                callback(&event);
            }
        }
    }
}