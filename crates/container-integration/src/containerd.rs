//! Containerd runtime integration.

use crate::error::{ContainerError, Result};
use crate::types::*;
use crate::manager::ContainerRuntimeTrait;
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Containerd runtime implementation.
pub struct ContainerdRuntime {
    /// Containerd configuration.
    config: ContainerdConfig,
    /// In-memory container store for simulation.
    containers: Arc<RwLock<HashMap<String, ContainerInfo>>>,
    /// In-memory image store for simulation.
    images: Arc<RwLock<HashMap<String, ImageInfo>>>,
}

impl ContainerdRuntime {
    /// Creates a new containerd runtime.
    pub async fn new(config: ContainerdConfig) -> Result<Self> {
        info!("Creating containerd runtime with socket: {}", config.socket_path.display());
        
        // In a real implementation, this would create a containerd-client
        // For simulation, we'll just create the runtime without actual connection
        
        Ok(Self {
            config,
            containers: Arc::new(RwLock::new(HashMap::new())),
            images: Arc::new(RwLock::new(HashMap::new())),
        })
    }
    
    /// Simulates checking containerd availability.
    async fn check_daemon(&self) -> Result<()> {
        // In a real implementation, this would connect to containerd socket
        // For simulation, we'll always return success
        debug!("Checking containerd availability (simulated)");
        Ok(())
    }
    
    /// Generates a mock container ID.
    fn generate_container_id(&self) -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let id: u64 = rng.gen();
        format!("ctr-{:x}", id)
    }
    
    /// Generates a mock image ID.
    fn generate_image_id(&self) -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let id: u64 = rng.gen();
        format!("sha256:{:x}", id)
    }
    
    /// Converts container spec to OCI runtime spec (simplified).
    fn create_oci_spec(&self, spec: &ContainerSpec) -> Result<HashMap<String, serde_json::Value>> {
        // In a real implementation, this would create a proper OCI runtime spec
        // For simulation, we'll return a simple JSON structure
        
        let mut oci_spec = HashMap::new();
        
        oci_spec.insert("ociVersion".to_string(), serde_json::json!("1.0.2"));
        oci_spec.insert("process".to_string(), serde_json::json!({
            "terminal": spec.tty,
            "user": {
                "uid": 0,
                "gid": 0
            },
            "args": spec.command.clone().unwrap_or_else(|| vec!["sh".to_string()]),
            "env": spec.env,
            "cwd": spec.working_dir.clone().unwrap_or_else(|| "/".to_string()),
        }));
        oci_spec.insert("root".to_string(), serde_json::json!({
            "path": "rootfs",
            "readonly": false
        }));
        oci_spec.insert("hostname".to_string(), serde_json::json!(spec.hostname.clone().unwrap_or_else(|| "container".to_string())));
        
        Ok(oci_spec)
    }
}

#[async_trait]
impl ContainerRuntimeTrait for ContainerdRuntime {
    fn runtime_type(&self) -> RuntimeType {
        RuntimeType::Containerd
    }
    
    async fn create_container(&self, spec: &ContainerSpec) -> Result<String> {
        info!("Creating container in containerd: {}", spec.name);
        
        // Check daemon availability
        self.check_daemon().await?;
        
        // Generate container ID
        let container_id = self.generate_container_id();
        
        // Create OCI spec (simulated)
        let _oci_spec = self.create_oci_spec(spec)?;
        
        // Create container info
        let container_info = ContainerInfo {
            id: container_id.clone(),
            name: spec.name.clone(),
            image: spec.image.clone(),
            status: ContainerStatus::Created,
            created: chrono::Utc::now(),
            started: None,
            finished: None,
            exit_code: None,
            labels: spec.labels.clone(),
            annotations: spec.annotations.clone(),
        };
        
        // Store in memory
        let mut containers = self.containers.write().await;
        containers.insert(container_id.clone(), container_info);
        
        info!("Container {} created with ID: {}", spec.name, container_id);
        Ok(container_id)
    }
    
    async fn start_container(&self, container_id: &str) -> Result<()> {
        info!("Starting container in containerd: {}", container_id);
        
        // Check daemon availability
        self.check_daemon().await?;
        
        let mut containers = self.containers.write().await;
        
        if let Some(container) = containers.get_mut(container_id) {
            container.status = ContainerStatus::Running;
            container.started = Some(chrono::Utc::now());
            container.finished = None;
            container.exit_code = None;
            
            info!("Container {} started", container_id);
            Ok(())
        } else {
            Err(ContainerError::ContainerNotFound(container_id.to_string()))
        }
    }
    
    async fn stop_container(&self, container_id: &str, timeout_secs: Option<u32>) -> Result<()> {
        info!("Stopping container in containerd: {} (timeout: {:?}s)", container_id, timeout_secs);
        
        // Check daemon availability
        self.check_daemon().await?;
        
        let mut containers = self.containers.write().await;
        
        if let Some(container) = containers.get_mut(container_id) {
            container.status = ContainerStatus::Stopped;
            container.finished = Some(chrono::Utc::now());
            container.exit_code = Some(0); // Success
            
            info!("Container {} stopped", container_id);
            Ok(())
        } else {
            Err(ContainerError::ContainerNotFound(container_id.to_string()))
        }
    }
    
    async fn restart_container(&self, container_id: &str, timeout_secs: Option<u32>) -> Result<()> {
        info!("Restarting container in containerd: {} (timeout: {:?}s)", container_id, timeout_secs);
        
        // Stop the container
        self.stop_container(container_id, timeout_secs).await?;
        
        // Start it again
        self.start_container(container_id).await?;
        
        info!("Container {} restarted", container_id);
        Ok(())
    }
    
    async fn remove_container(&self, container_id: &str, force: bool) -> Result<()> {
        info!("Removing container from containerd: {} (force: {})", container_id, force);
        
        // Check daemon availability
        self.check_daemon().await?;
        
        let mut containers = self.containers.write().await;
        
        if containers.remove(container_id).is_some() {
            info!("Container {} removed", container_id);
            Ok(())
        } else {
            Err(ContainerError::ContainerNotFound(container_id.to_string()))
        }
    }
    
    async fn get_container(&self, container_id: &str) -> Result<ContainerInfo> {
        debug!("Getting container info from containerd: {}", container_id);
        
        let containers = self.containers.read().await;
        
        containers.get(container_id)
            .cloned()
            .ok_or_else(|| ContainerError::ContainerNotFound(container_id.to_string()))
    }
    
    async fn list_containers(&self, all: bool) -> Result<Vec<ContainerInfo>> {
        debug!("Listing containers from containerd (all: {})", all);
        
        let containers = self.containers.read().await;
        
        let mut result: Vec<ContainerInfo> = containers.values().cloned().collect();
        
        // If not showing all containers, filter out stopped ones
        if !all {
            result.retain(|c| c.status == ContainerStatus::Running || c.status == ContainerStatus::Created);
        }
        
        Ok(result)
    }
    
    async fn get_logs(
        &self,
        container_id: &str,
        stdout: bool,
        stderr: bool,
        since: Option<chrono::DateTime<chrono::Utc>>,
        until: Option<chrono::DateTime<chrono::Utc>>,
        tail: Option<usize>,
    ) -> Result<Vec<u8>> {
        info!("Getting logs for container from containerd: {} (stdout: {}, stderr: {})", 
            container_id, stdout, stderr);
        
        // Check if container exists
        let containers = self.containers.read().await;
        if !containers.contains_key(container_id) {
            return Err(ContainerError::ContainerNotFound(container_id.to_string()));
        }
        
        // Generate mock logs
        let log_lines = vec![
            format!("[{}] Container started via containerd", chrono::Utc::now().to_rfc3339()),
            format!("[{}] OCI runtime initialized", chrono::Utc::now().to_rfc3339()),
            format!("[{}] Agent process started", chrono::Utc::now().to_rfc3339()),
            format!("[{}] Connected to containerd shim", chrono::Utc::now().to_rfc3339()),
            format!("[{}] Health check passed", chrono::Utc::now().to_rfc3339()),
        ];
        
        // Apply tail filter
        let filtered_logs = if let Some(tail_count) = tail {
            log_lines.iter()
                .rev()
                .take(tail_count)
                .rev()
                .cloned()
                .collect::<Vec<_>>()
        } else {
            log_lines
        };
        
        // Join logs
        let logs = filtered_logs.join("\n");
        
        Ok(logs.into_bytes())
    }
    
    async fn get_stats(&self, container_id: &str) -> Result<ContainerStats> {
        debug!("Getting stats for container from containerd: {}", container_id);
        
        // Check if container exists
        let containers = self.containers.read().await;
        if !containers.contains_key(container_id) {
            return Err(ContainerError::ContainerNotFound(container_id.to_string()));
        }
        
        // Generate mock stats
        Ok(ContainerStats {
            cpu_usage: 300_000_000, // 0.3 CPU seconds
            memory_usage: 128 * 1024 * 1024, // 128 MB
            memory_limit: 512 * 1024 * 1024, // 512 MB
            network_rx: 768 * 1024, // 768 KB received
            network_tx: 256 * 1024, // 256 KB sent
            block_read: 0,
            block_write: 0,
            pids: 3,
            timestamp: chrono::Utc::now(),
        })
    }
    
    async fn exec(
        &self,
        container_id: &str,
        command: Vec<String>,
        user: Option<String>,
        env: Vec<String>,
        working_dir: Option<String>,
    ) -> Result<(i32, Vec<u8>, Vec<u8>)> {
        info!("Executing command in containerd container {}: {:?}", container_id, command);
        
        // Check if container exists
        let containers = self.containers.read().await;
        if !containers.contains_key(container_id) {
            return Err(ContainerError::ContainerNotFound(container_id.to_string()));
        }
        
        // Simulate command execution via containerd
        let exit_code = 0;
        let stdout = format!("Command executed via containerd: {:?}\n", command).into_bytes();
        let stderr = Vec::new();
        
        Ok((exit_code, stdout, stderr))
    }
    
    async fn pull_image(&self, image: &str, tag: &str) -> Result<()> {
        info!("Pulling image via containerd: {}:{}", image, tag);
        
        // Check daemon availability
        self.check_daemon().await?;
        
        // Generate image ID
        let image_id = self.generate_image_id();
        
        // Create image info
        let image_info = ImageInfo {
            id: image_id,
            repository: image.to_string(),
            tag: tag.to_string(),
            digest: None,
            created: chrono::Utc::now(),
            size: 200 * 1024 * 1024, // 200 MB
            labels: HashMap::new(),
        };
        
        // Store in memory
        let mut images = self.images.write().await;
        let key = format!("{}:{}", image, tag);
        images.insert(key, image_info);
        
        info!("Image {}:{} pulled via containerd", image, tag);
        Ok(())
    }
    
    async fn build_image(
        &self,
        dockerfile_path: &str,
        tag: &str,
        build_args: HashMap<String, String>,
    ) -> Result<()> {
        info!("Building image via containerd from {} with tag: {}", dockerfile_path, tag);
        
        // Check daemon availability
        self.check_daemon().await?;
        
        // Generate image ID
        let image_id = self.generate_image_id();
        
        // Create image info
        let image_info = ImageInfo {
            id: image_id,
            repository: "local".to_string(),
            tag: tag.to_string(),
            digest: None,
            created: chrono::Utc::now(),
            size: 400 * 1024 * 1024, // 400 MB
            labels: HashMap::new(),
        };
        
        // Store in memory
        let mut images = self.images.write().await;
        images.insert(tag.to_string(), image_info);
        
        info!("Image {} built via containerd", tag);
        Ok(())
    }
    
    async fn list_images(&self) -> Result<Vec<ImageInfo>> {
        debug!("Listing images from containerd");
        
        let images = self.images.read().await;
        Ok(images.values().cloned().collect())
    }
    
    async fn remove_image(&self, image_id: &str, force: bool) -> Result<()> {
        info!("Removing image from containerd: {} (force: {})", image_id, force);
        
        // Check daemon availability
        self.check_daemon().await?;
        
        let mut images = self.images.write().await;
        
        // Try to find image by ID or tag
        let mut found_key = None;
        for (key, image) in images.iter() {
            if image.id == image_id || key == image_id {
                found_key = Some(key.clone());
                break;
            }
        }
        
        if let Some(key) = found_key {
            images.remove(&key);
            info!("Image {} removed from containerd", image_id);
            Ok(())
        } else {
            Err(ContainerError::ImageNotFound(image_id.to_string()))
        }
    }
    
    async fn ping(&self) -> Result<()> {
        debug!("Pinging containerd");
        
        // Simulate ping
        self.check_daemon().await?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_containerd_runtime_creation() {
        let config = ContainerdConfig::default();
        let runtime = ContainerdRuntime::new(config).await;
        
        assert!(runtime.is_ok());
    }
    
    #[tokio::test]
    async fn test_create_container() {
        let config = ContainerdConfig::default();
        let runtime = ContainerdRuntime::new(config).await.unwrap();
        
        let spec = ContainerSpec {
            name: "test-container".to_string(),
            image: "alpine:latest".to_string(),
            ..Default::default()
        };
        
        let container_id = runtime.create_container(&spec).await;
        assert!(container_id.is_ok());
    }
    
    #[tokio::test]
    async fn test_containerd_specific_features() {
        let config = ContainerdConfig::default();
        let runtime = ContainerdRuntime::new(config).await.unwrap();
        
        // Test that it's actually containerd runtime
        assert_eq!(runtime.runtime_type(), RuntimeType::Containerd);
    }
}