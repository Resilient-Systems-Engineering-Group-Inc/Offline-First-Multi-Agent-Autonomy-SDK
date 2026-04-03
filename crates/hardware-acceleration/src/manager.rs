//! High‑level manager for hardware acceleration.

use crate::accelerator::{Accelerator, AcceleratorBackend, GenericAccelerator};
use crate::backend::BackendRegistry;
use crate::device::{AccelerationBackend as BackendType, Device, DeviceType};
use crate::error::{Result, AccelerationError};
use crate::task::{AccelerationTask, TaskScheduler, TaskStatus};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

/// Manages hardware accelerators and schedules acceleration tasks.
pub struct AccelerationManager {
    /// Registry of available backends.
    backend_registry: Arc<BackendRegistry>,
    /// Active accelerators per device.
    accelerators: RwLock<HashMap<String, Arc<dyn Accelerator>>>,
    /// Task scheduler.
    scheduler: Arc<Mutex<TaskScheduler>>,
    /// Configuration.
    config: AccelerationManagerConfig,
}

/// Configuration for the acceleration manager.
#[derive(Debug, Clone)]
pub struct AccelerationManagerConfig {
    /// Whether to automatically discover backends on startup.
    pub auto_discover: bool,
    /// Preferred backends in order of priority.
    pub preferred_backends: Vec<BackendType>,
    /// Maximum number of concurrent tasks per accelerator.
    pub max_concurrent_tasks: usize,
    /// Enable task scheduling.
    pub enable_scheduling: bool,
}

impl Default for AccelerationManagerConfig {
    fn default() -> Self {
        Self {
            auto_discover: true,
            preferred_backends: vec![
                BackendType::Cuda,
                BackendType::OpenCl,
                BackendType::Wgpu,
                BackendType::TfLite,
                BackendType::Torch,
                BackendType::Onnx,
            ],
            max_concurrent_tasks: 4,
            enable_scheduling: true,
        }
    }
}

impl AccelerationManager {
    /// Creates a new acceleration manager with default configuration.
    pub async fn new() -> Result<Self> {
        Self::with_config(AccelerationManagerConfig::default()).await
    }

    /// Creates a new acceleration manager with custom configuration.
    pub async fn with_config(config: AccelerationManagerConfig) -> Result<Self> {
        let backend_registry = Arc::new(BackendRegistry::new());
        let mut manager = Self {
            backend_registry: backend_registry.clone(),
            accelerators: RwLock::new(HashMap::new()),
            scheduler: Arc::new(Mutex::new(TaskScheduler::new())),
            config,
        };

        if manager.config.auto_discover {
            backend_registry.discover().await?;
        }

        // Initialize accelerators for preferred backends
        for backend_type in &manager.config.preferred_backends {
            if let Some(backend) = backend_registry.get(*backend_type) {
                let _ = manager.initialize_backend(backend).await;
            }
        }

        Ok(manager)
    }

    /// Initializes accelerators for a given backend.
    async fn initialize_backend(&self, backend: Arc<dyn AcceleratorBackend>) -> Result<()> {
        let devices = backend.enumerate_devices().await?;
        for device in devices {
            if device.available {
                match backend.create_accelerator(&device).await {
                    Ok(accelerator) => {
                        let mut accelerators = self.accelerators.write().await;
                        accelerators.insert(device.id.clone(), accelerator);
                        tracing::info!(
                            "Initialized accelerator: {} ({})",
                            device.name,
                            device.backend
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to create accelerator for {}: {}",
                            device.name,
                            e
                        );
                    }
                }
            }
        }
        Ok(())
    }

    /// Returns a list of available accelerators.
    pub async fn available_accelerators(&self) -> Vec<Arc<dyn Accelerator>> {
        let accelerators = self.accelerators.read().await;
        accelerators.values().cloned().collect()
    }

    /// Returns accelerators of a specific device type.
    pub async fn accelerators_by_type(&self, device_type: DeviceType) -> Vec<Arc<dyn Accelerator>> {
        let accelerators = self.accelerators.read().await;
        accelerators
            .values()
            .filter(|acc| acc.device().device_type == device_type)
            .cloned()
            .collect()
    }

    /// Returns accelerators using a specific backend.
    pub async fn accelerators_by_backend(&self, backend: BackendType) -> Vec<Arc<dyn Accelerator>> {
        let accelerators = self.accelerators.read().await;
        accelerators
            .values()
            .filter(|acc| acc.device().backend == backend)
            .cloned()
            .collect()
    }

    /// Submits a task for execution.
    pub async fn submit_task(&self, task: AccelerationTask) -> Result<String> {
        let mut scheduler = self.scheduler.lock().await;
        scheduler.submit(task).await
    }

    /// Returns the status of a task.
    pub async fn task_status(&self, task_id: &str) -> Option<TaskStatus> {
        let scheduler = self.scheduler.lock().await;
        scheduler.task_status(task_id).await
    }

    /// Cancels a task.
    pub async fn cancel_task(&self, task_id: &str) -> Result<()> {
        let mut scheduler = self.scheduler.lock().await;
        scheduler.cancel(task_id).await
    }

    /// Waits for a task to complete and returns its result.
    pub async fn wait_for_task(&self, task_id: &str) -> Result<Vec<u8>> {
        let mut scheduler = self.scheduler.lock().await;
        scheduler.wait(task_id).await
    }

    /// Runs the scheduler loop (should be spawned as a background task).
    pub async fn run_scheduler(&self) -> Result<()> {
        if !self.config.enable_scheduling {
            return Ok(());
        }

        let scheduler = self.scheduler.clone();
        let accelerators = self.accelerators.read().await;
        let accelerators_list: Vec<Arc<dyn Accelerator>> = accelerators.values().cloned().collect();
        drop(accelerators);

        let mut scheduler = scheduler.lock().await;
        scheduler.run(&accelerators_list).await
    }

    /// Adds a new accelerator manually.
    pub async fn add_accelerator(&self, accelerator: Arc<dyn Accelerator>) {
        let mut accelerators = self.accelerators.write().await;
        accelerators.insert(accelerator.device().id.clone(), accelerator);
    }

    /// Removes an accelerator by device ID.
    pub async fn remove_accelerator(&self, device_id: &str) -> Option<Arc<dyn Accelerator>> {
        let mut accelerators = self.accelerators.write().await;
        accelerators.remove(device_id)
    }

    /// Returns the backend registry.
    pub fn backend_registry(&self) -> Arc<BackendRegistry> {
        self.backend_registry.clone()
    }
}

/// Convenience function to create a manager with default settings.
pub async fn create_manager() -> Result<AccelerationManager> {
    AccelerationManager::new().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accelerator::DummyBackend;
    use crate::task::{AccelerationTask, TaskType};

    #[tokio::test]
    async fn test_manager_creation() {
        let manager = AccelerationManager::new().await;
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_submit_dummy_task() {
        let manager = AccelerationManager::new().await.unwrap();
        let task = AccelerationTask {
            id: "test_task".to_string(),
            task_type: TaskType::Inference,
            data: vec![1, 2, 3],
            ..Default::default()
        };
        let task_id = manager.submit_task(task).await;
        // Should succeed even without real accelerators (dummy backend)
        assert!(task_id.is_ok());
    }

    #[tokio::test]
    async fn test_accelerators_list() {
        let manager = AccelerationManager::new().await.unwrap();
        let accelerators = manager.available_accelerators().await;
        // At least dummy accelerator should be present
        assert!(!accelerators.is_empty());
    }
}