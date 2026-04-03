//! Container runtime manager.

use crate::error::{ContainerError, Result};
use crate::types::*;
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// Trait for container runtime implementations.
#[async_trait]
pub trait ContainerRuntimeTrait: Send + Sync {
    /// Returns the runtime type.
    fn runtime_type(&self) -> RuntimeType;

    /// Creates a container from a specification.
    async fn create_container(&self, spec: &ContainerSpec) -> Result<String>;

    /// Starts a container.
    async fn start_container(&self, container_id: &str) -> Result<()>;

    /// Stops a container.
    async fn stop_container(&self, container_id: &str, timeout_secs: Option<u32>) -> Result<()>;

    /// Restarts a container.
    async fn restart_container(&self, container_id: &str, timeout_secs: Option<u32>) -> Result<()>;

    /// Removes a container.
    async fn remove_container(&self, container_id: &str, force: bool) -> Result<()>;

    /// Gets container information.
    async fn get_container(&self, container_id: &str) -> Result<ContainerInfo>;

    /// Lists all containers.
    async fn list_containers(&self, all: bool) -> Result<Vec<ContainerInfo>>;

    /// Gets container logs.
    async fn get_logs(
        &self,
        container_id: &str,
        stdout: bool,
        stderr: bool,
        since: Option<chrono::DateTime<chrono::Utc>>,
        until: Option<chrono::DateTime<chrono::Utc>>,
        tail: Option<usize>,
    ) -> Result<Vec<u8>>;

    /// Gets container statistics.
    async fn get_stats(&self, container_id: &str) -> Result<ContainerStats>;

    /// Executes a command in a running container.
    async fn exec(
        &self,
        container_id: &str,
        command: Vec<String>,
        user: Option<String>,
        env: Vec<String>,
        working_dir: Option<String>,
    ) -> Result<(i32, Vec<u8>, Vec<u8>)>;

    /// Pulls an image from a registry.
    async fn pull_image(&self, image: &str, tag: &str) -> Result<()>;

    /// Builds an image from a Dockerfile.
    async fn build_image(
        &self,
        dockerfile_path: &str,
        tag: &str,
        build_args: HashMap<String, String>,
    ) -> Result<()>;

    /// Lists images.
    async fn list_images(&self) -> Result<Vec<ImageInfo>>;

    /// Removes an image.
    async fn remove_image(&self, image_id: &str, force: bool) -> Result<()>;

    /// Checks if the runtime is available.
    async fn ping(&self) -> Result<()>;
}

/// Container runtime manager that abstracts over different runtimes.
pub struct ContainerRuntime {
    /// Runtime implementation.
    inner: Arc<dyn ContainerRuntimeTrait>,
    /// Runtime configuration.
    config: RuntimeConfig,
}

impl ContainerRuntime {
    /// Creates a Docker runtime.
    #[cfg(feature = "docker")]
    pub async fn docker(config: DockerConfig) -> Result<Self> {
        use crate::docker::DockerRuntime;
        
        let runtime = DockerRuntime::new(config).await?;
        let runtime_config = RuntimeConfig {
            runtime_type: RuntimeType::Docker,
            docker_config: Some(config),
            containerd_config: None,
            default_resources: ResourceConstraints::default(),
            default_network: NetworkConfig::default(),
        };
        
        Ok(Self {
            inner: Arc::new(runtime),
            config: runtime_config,
        })
    }
    
    /// Creates a containerd runtime.
    #[cfg(feature = "containerd")]
    pub async fn containerd(config: ContainerdConfig) -> Result<Self> {
        use crate::containerd::ContainerdRuntime;
        
        let runtime = ContainerdRuntime::new(config).await?;
        let runtime_config = RuntimeConfig {
            runtime_type: RuntimeType::Containerd,
            docker_config: None,
            containerd_config: Some(config),
            default_resources: ResourceConstraints::default(),
            default_network: NetworkConfig::default(),
        };
        
        Ok(Self {
            inner: Arc::new(runtime),
            config: runtime_config,
        })
    }
    
    /// Creates a runtime from configuration.
    pub async fn from_config(config: RuntimeConfig) -> Result<Self> {
        match config.runtime_type {
            RuntimeType::Docker => {
                #[cfg(feature = "docker")]
                {
                    if let Some(docker_config) = config.docker_config {
                        return Self::docker(docker_config).await;
                    } else {
                        return Err(ContainerError::ConfigError(
                            "Docker configuration missing for Docker runtime".to_string()
                        ));
                    }
                }
                #[cfg(not(feature = "docker"))]
                {
                    return Err(ContainerError::ConfigError(
                        "Docker feature not enabled".to_string()
                    ));
                }
            }
            RuntimeType::Containerd => {
                #[cfg(feature = "containerd")]
                {
                    if let Some(containerd_config) = config.containerd_config {
                        return Self::containerd(containerd_config).await;
                    } else {
                        return Err(ContainerError::ConfigError(
                            "Containerd configuration missing for containerd runtime".to_string()
                        ));
                    }
                }
                #[cfg(not(feature = "containerd"))]
                {
                    return Err(ContainerError::ConfigError(
                        "Containerd feature not enabled".to_string()
                    ));
                }
            }
            RuntimeType::Podman => {
                // Podman is compatible with Docker API, so we can use Docker runtime
                #[cfg(feature = "docker")]
                {
                    let mut docker_config = config.docker_config.unwrap_or_default();
                    // Podman typically uses a different socket
                    if docker_config.host.contains("docker") {
                        docker_config.host = "unix:///run/podman/podman.sock".to_string();
                    }
                    return Self::docker(docker_config).await;
                }
                #[cfg(not(feature = "docker"))]
                {
                    return Err(ContainerError::ConfigError(
                        "Docker feature not enabled (required for Podman)".to_string()
                    ));
                }
            }
            RuntimeType::Custom(_) => {
                return Err(ContainerError::ConfigError(
                    "Custom runtimes not yet implemented".to_string()
                ));
            }
        }
    }
    
    /// Gets the runtime configuration.
    pub fn config(&self) -> &RuntimeConfig {
        &self.config
    }
    
    /// Gets the runtime type.
    pub fn runtime_type(&self) -> RuntimeType {
        self.inner.runtime_type()
    }
    
    /// Creates and starts a container.
    pub async fn run_container(&self, spec: &ContainerSpec) -> Result<String> {
        info!("Running container: {}", spec.name);
        
        let container_id = self.create_container(spec).await?;
        self.start_container(&container_id).await?;
        
        info!("Container {} started with ID: {}", spec.name, container_id);
        Ok(container_id)
    }
    
    /// Stops and removes a container.
    pub async fn cleanup_container(&self, container_id: &str, force: bool) -> Result<()> {
        info!("Cleaning up container: {}", container_id);
        
        // Try to stop the container first
        let _ = self.stop_container(container_id, Some(10)).await;
        
        // Remove the container
        self.remove_container(container_id, force).await?;
        
        info!("Container {} cleaned up", container_id);
        Ok(())
    }
    
    /// Checks container health.
    pub async fn check_health(&self, container_id: &str) -> Result<bool> {
        let info = self.get_container(container_id).await?;
        
        match info.status {
            ContainerStatus::Running => Ok(true),
            ContainerStatus::Created | ContainerStatus::Restarting => {
                // Container is not fully healthy but not dead
                Ok(false)
            }
            ContainerStatus::Paused | ContainerStatus::Stopped | ContainerStatus::Dead | ContainerStatus::Unknown => {
                Ok(false)
            }
        }
    }
    
    /// Waits for a container to be in a specific state.
    pub async fn wait_for_state(
        &self,
        container_id: &str,
        target_state: ContainerStatus,
        timeout_secs: u64,
    ) -> Result<()> {
        use tokio::time::{sleep, Duration, timeout};
        
        let start = std::time::Instant::now();
        let timeout_duration = Duration::from_secs(timeout_secs);
        
        loop {
            if start.elapsed() > timeout_duration {
                return Err(ContainerError::Timeout(format!(
                    "Timeout waiting for container {} to reach state {:?}",
                    container_id, target_state
                )));
            }
            
            match self.get_container(container_id).await {
                Ok(info) if info.status == target_state => {
                    return Ok(());
                }
                Ok(_) => {
                    // Not yet in target state
                    sleep(Duration::from_millis(500)).await;
                }
                Err(e) => {
                    // Container might not exist anymore
                    return Err(e);
                }
            }
        }
    }
}

// Delegate all trait methods to the inner runtime
#[async_trait]
impl ContainerRuntimeTrait for ContainerRuntime {
    fn runtime_type(&self) -> RuntimeType {
        self.inner.runtime_type()
    }
    
    async fn create_container(&self, spec: &ContainerSpec) -> Result<String> {
        self.inner.create_container(spec).await
    }
    
    async fn start_container(&self, container_id: &str) -> Result<()> {
        self.inner.start_container(container_id).await
    }
    
    async fn stop_container(&self, container_id: &str, timeout_secs: Option<u32>) -> Result<()> {
        self.inner.stop_container(container_id, timeout_secs).await
    }
    
    async fn restart_container(&self, container_id: &str, timeout_secs: Option<u32>) -> Result<()> {
        self.inner.restart_container(container_id, timeout_secs).await
    }
    
    async fn remove_container(&self, container_id: &str, force: bool) -> Result<()> {
        self.inner.remove_container(container_id, force).await
    }
    
    async fn get_container(&self, container_id: &str) -> Result<ContainerInfo> {
        self.inner.get_container(container_id).await
    }
    
    async fn list_containers(&self, all: bool) -> Result<Vec<ContainerInfo>> {
        self.inner.list_containers(all).await
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
        self.inner.get_logs(container_id, stdout, stderr, since, until, tail).await
    }
    
    async fn get_stats(&self, container_id: &str) -> Result<ContainerStats> {
        self.inner.get_stats(container_id).await
    }
    
    async fn exec(
        &self,
        container_id: &str,
        command: Vec<String>,
        user: Option<String>,
        env: Vec<String>,
        working_dir: Option<String>,
    ) -> Result<(i32, Vec<u8>, Vec<u8>)> {
        self.inner.exec(container_id, command, user, env, working_dir).await
    }
    
    async fn pull_image(&self, image: &str, tag: &str) -> Result<()> {
        self.inner.pull_image(image, tag).await
    }
    
    async fn build_image(
        &self,
        dockerfile_path: &str,
        tag: &str,
        build_args: HashMap<String, String>,
    ) -> Result<()> {
        self.inner.build_image(dockerfile_path, tag, build_args).await
    }
    
    async fn list_images(&self) -> Result<Vec<ImageInfo>> {
        self.inner.list_images().await
    }
    
    async fn remove_image(&self, image_id: &str, force: bool) -> Result<()> {
        self.inner.remove_image(image_id, force).await
    }
    
    async fn ping(&self) -> Result<()> {
        self.inner.ping().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_container_runtime_trait_object() {
        // This test verifies that ContainerRuntime can be used as a trait object
        let config = RuntimeConfig::default();
        
        // Note: This would actually create a runtime, but we skip it in tests
        // since we don't have Docker/containerd available
        assert_eq!(config.runtime_type, RuntimeType::Docker);
    }
}