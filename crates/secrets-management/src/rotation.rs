//! Automatic secret rotation scheduler.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, mpsc};
use tokio::time::{sleep, interval};
use chrono::{DateTime, Utc};

use crate::error::{SecretsError, Result};
use crate::manager::SecretsManager;
use crate::model::Secret;

/// Rotation job configuration.
#[derive(Debug, Clone)]
pub struct RotationJob {
    /// Secret ID to rotate.
    pub secret_id: String,
    
    /// Rotation interval (seconds).
    pub interval_secs: u64,
    
    /// Last rotation timestamp.
    pub last_rotated: Option<DateTime<Utc>>,
    
    /// Next scheduled rotation.
    pub next_rotation: DateTime<Utc>,
    
    /// Rotation function.
    pub rotation_fn: Arc<dyn Fn() -> String + Send + Sync>,
    
    /// Whether the job is enabled.
    pub enabled: bool,
}

impl RotationJob {
    /// Create a new rotation job.
    pub fn new(
        secret_id: impl Into<String>,
        interval_secs: u64,
        rotation_fn: impl Fn() -> String + Send + Sync + 'static,
    ) -> Self {
        let now = Utc::now();
        Self {
            secret_id: secret_id.into(),
            interval_secs,
            last_rotated: None,
            next_rotation: now + chrono::Duration::seconds(interval_secs as i64),
            rotation_fn: Arc::new(rotation_fn),
            enabled: true,
        }
    }
    
    /// Check if the job is due for rotation.
    pub fn is_due(&self) -> bool {
        Utc::now() >= self.next_rotation
    }
    
    /// Update schedule after rotation.
    pub fn mark_rotated(&mut self) {
        let now = Utc::now();
        self.last_rotated = Some(now);
        self.next_rotation = now + chrono::Duration::seconds(self.interval_secs as i64);
    }
}

/// Rotation scheduler for automatic secret rotation.
#[derive(Debug)]
pub struct RotationScheduler {
    jobs: RwLock<HashMap<String, RotationJob>>,
    manager: Arc<dyn SecretsManagerTrait>,
    running: RwLock<bool>,
    task_handle: RwLock<Option<tokio::task::JoinHandle<()>>>,
}

/// Trait for secrets manager (for dependency injection).
#[async_trait::async_trait]
pub trait SecretsManagerTrait: Send + Sync {
    async fn rotate(&self, id: &str, new_value: String) -> Result<()>;
    async fn get(&self, id: &str) -> Result<Secret>;
}

#[async_trait::async_trait]
impl<B: crate::backend::Backend> SecretsManagerTrait for crate::manager::SecretsManager<B> {
    async fn rotate(&self, id: &str, new_value: String) -> Result<()> {
        self.rotate(id, new_value).await
    }
    
    async fn get(&self, id: &str) -> Result<Secret> {
        self.get(id).await
    }
}

impl RotationScheduler {
    /// Create a new rotation scheduler.
    pub fn new(manager: Arc<dyn SecretsManagerTrait>) -> Self {
        Self {
            jobs: RwLock::new(HashMap::new()),
            manager,
            running: RwLock::new(false),
            task_handle: RwLock::new(None),
        }
    }
    
    /// Add a rotation job.
    pub async fn add_job(&self, job: RotationJob) {
        let mut jobs = self.jobs.write().await;
        jobs.insert(job.secret_id.clone(), job);
    }
    
    /// Remove a rotation job.
    pub async fn remove_job(&self, secret_id: &str) {
        let mut jobs = self.jobs.write().await;
        jobs.remove(secret_id);
    }
    
    /// Enable/disable a job.
    pub async fn set_job_enabled(&self, secret_id: &str, enabled: bool) {
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.get_mut(secret_id) {
            job.enabled = enabled;
        }
    }
    
    /// Get all jobs.
    pub async fn jobs(&self) -> Vec<RotationJob> {
        let jobs = self.jobs.read().await;
        jobs.values().cloned().collect()
    }
    
    /// Start the scheduler.
    pub async fn start(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(SecretsError::Rotation("scheduler already running".into()));
        }
        
        *running = true;
        
        // Clone Arc for the task
        let scheduler = Arc::new(self.clone());
        
        let handle = tokio::spawn(async move {
            scheduler.run_loop().await;
        });
        
        let mut task_handle = self.task_handle.write().await;
        *task_handle = Some(handle);
        
        Ok(())
    }
    
    /// Stop the scheduler.
    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().await;
        *running = false;
        
        let mut task_handle = self.task_handle.write().await;
        if let Some(handle) = task_handle.take() {
            handle.abort();
        }
        
        Ok(())
    }
    
    /// Check if scheduler is running.
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }
    
    /// Run a single rotation cycle.
    pub async fn run_cycle(&self) -> Result<Vec<(String, Result<()>)>> {
        let mut results = Vec::new();
        let mut jobs = self.jobs.write().await;
        
        for (secret_id, job) in jobs.iter_mut() {
            if !job.enabled || !job.is_due() {
                continue;
            }
            
            log::info!("Rotating secret {}", secret_id);
            
            // Generate new value
            let new_value = (job.rotation_fn)();
            
            // Perform rotation
            let result = self.manager.rotate(secret_id, new_value).await;
            
            if result.is_ok() {
                job.mark_rotated();
                log::info!("Successfully rotated secret {}", secret_id);
            } else {
                log::error!("Failed to rotate secret {}: {:?}", secret_id, result);
            }
            
            results.push((secret_id.clone(), result));
        }
        
        Ok(results)
    }
    
    /// Main scheduler loop.
    async fn run_loop(self: Arc<Self>) {
        let mut interval = interval(Duration::from_secs(60)); // Check every minute
        
        while *self.running.read().await {
            interval.tick().await;
            
            match self.run_cycle().await {
                Ok(results) => {
                    if !results.is_empty() {
                        log::debug!("Rotation cycle completed: {} secrets processed", results.len());
                    }
                }
                Err(e) => {
                    log::error!("Rotation cycle failed: {}", e);
                }
            }
        }
        
        log::info!("Rotation scheduler stopped");
    }
    
    /// Manually trigger rotation of a secret.
    pub async fn trigger_rotation(&self, secret_id: &str) -> Result<()> {
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.get_mut(secret_id) {
            if !job.enabled {
                return Err(SecretsError::Rotation(
                    format!("job for secret {} is disabled", secret_id)
                ));
            }
            
            let new_value = (job.rotation_fn)();
            self.manager.rotate(secret_id, new_value).await?;
            job.mark_rotated();
            
            Ok(())
        } else {
            Err(SecretsError::NotFound(
                format!("rotation job for secret {} not found", secret_id)
            ))
        }
    }
    
    /// Get next rotation time for a secret.
    pub async fn next_rotation(&self, secret_id: &str) -> Option<DateTime<Utc>> {
        let jobs = self.jobs.read().await;
        jobs.get(secret_id).map(|job| job.next_rotation)
    }
}

impl Clone for RotationScheduler {
    fn clone(&self) -> Self {
        Self {
            jobs: RwLock::new(HashMap::new()), // Empty clone
            manager: self.manager.clone(),
            running: RwLock::new(false),
            task_handle: RwLock::new(None),
        }
    }
}

/// Built‑in rotation functions.
pub mod rotation_functions {
    use rand::{Rng, rngs::OsRng};
    use uuid::Uuid;
    
    /// Generate a random alphanumeric string.
    pub fn random_alphanumeric() -> String {
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                                abcdefghijklmnopqrstuvwxyz\
                                0123456789";
        const LEN: usize = 32;
        
        let mut rng = OsRng;
        (0..LEN)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }
    
    /// Generate a random UUID.
    pub fn random_uuid() -> String {
        Uuid::new_v4().to_string()
    }
    
    /// Generate a random hexadecimal string.
    pub fn random_hex() -> String {
        let mut rng = OsRng;
        let bytes: [u8; 16] = rng.gen();
        hex::encode(bytes)
    }
    
    /// Generate a timestamp‑based secret.
    pub fn timestamp_based() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        format!("secret_{}", timestamp)
    }
}