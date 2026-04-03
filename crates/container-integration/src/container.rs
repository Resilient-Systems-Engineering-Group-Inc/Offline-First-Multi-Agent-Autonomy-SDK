//! Container management utilities.

use crate::error::{ContainerError, Result};
use crate::types::*;
use std::collections::HashMap;
use tracing::{debug, info};

/// Container manager utilities.
pub struct ContainerManager;

impl ContainerManager {
    /// Creates a new container manager.
    pub fn new() -> Self {
        Self
    }
    
    /// Validates a container specification.
    pub fn validate_spec(&self, spec: &ContainerSpec) -> Result<()> {
        // Check name
        if spec.name.is_empty() {
            return Err(ContainerError::InvalidArgument("Container name cannot be empty".to_string()));
        }
        
        // Check image
        if spec.image.is_empty() {
            return Err(ContainerError::InvalidArgument("Container image cannot be empty".to_string()));
        }
        
        // Check command/args
        if let Some(cmd) = &spec.command {
            if cmd.is_empty() {
                return Err(ContainerError::InvalidArgument("Command cannot be empty if specified".to_string()));
            }
        }
        
        // Check environment variables format
        for env_var in &spec.env {
            if !env_var.contains('=') {
                return Err(ContainerError::InvalidArgument(
                    format!("Environment variable must contain '=': {}", env_var)
                ));
            }
        }
        
        Ok(())
    }
    
    /// Generates container environment variables.
    pub fn generate_env_vars(&self, base_vars: &[String], extra_vars: &HashMap<String, String>) -> Vec<String> {
        let mut env_vars = base_vars.to_vec();
        
        for (key, value) in extra_vars {
            env_vars.push(format!("{}={}", key, value));
        }
        
        env_vars
    }
    
    /// Generates container labels.
    pub fn generate_labels(&self, base_labels: &HashMap<String, String>, extra_labels: &HashMap<String, String>) -> HashMap<String, String> {
        let mut labels = base_labels.clone();
        
        for (key, value) in extra_labels {
            labels.insert(key.clone(), value.clone());
        }
        
        // Add timestamp label
        labels.insert("org.agent-sdk.created".to_string(), chrono::Utc::now().to_rfc3339());
        
        labels
    }
    
    /// Calculates resource requirements for a container.
    pub fn calculate_resources(&self, spec: &ContainerSpec, default_resources: &ResourceConstraints) -> ResourceConstraints {
        // In a real implementation, this would analyze the container spec
        // and calculate appropriate resource limits
        // For now, we'll just return the default resources
        
        default_resources.clone()
    }
    
    /// Creates a mock container info for testing.
    pub fn create_mock_container(&self, spec: &ContainerSpec, container_id: &str) -> ContainerInfo {
        ContainerInfo {
            id: container_id.to_string(),
            name: spec.name.clone(),
            image: spec.image.clone(),
            status: ContainerStatus::Created,
            created: chrono::Utc::now(),
            started: None,
            finished: None,
            exit_code: None,
            labels: spec.labels.clone(),
            annotations: spec.annotations.clone(),
        }
    }
    
    /// Checks if a container is healthy based on its status.
    pub fn is_healthy(&self, container_info: &ContainerInfo) -> bool {
        match container_info.status {
            ContainerStatus::Running => true,
            ContainerStatus::Created | ContainerStatus::Restarting => {
                // Container is starting up, not fully healthy yet
                false
            }
            ContainerStatus::Paused | ContainerStatus::Stopped | ContainerStatus::Dead | ContainerStatus::Unknown => {
                false
            }
        }
    }
    
    /// Gets the container uptime.
    pub fn uptime(&self, container_info: &ContainerInfo) -> Option<std::time::Duration> {
        if let Some(started) = container_info.started {
            let now = chrono::Utc::now();
            let duration = now - started;
            
            // Convert chrono::Duration to std::time::Duration
            match duration.to_std() {
                Ok(duration) => Some(duration),
                Err(_) => None,
            }
        } else {
            None
        }
    }
    
    /// Formats container status for display.
    pub fn format_status(&self, container_info: &ContainerInfo) -> String {
        match container_info.status {
            ContainerStatus::Created => "Created".to_string(),
            ContainerStatus::Running => {
                if let Some(uptime) = self.uptime(container_info) {
                    format!("Running ({} seconds)", uptime.as_secs())
                } else {
                    "Running".to_string()
                }
            }
            ContainerStatus::Paused => "Paused".to_string(),
            ContainerStatus::Restarting => "Restarting".to_string(),
            ContainerStatus::Stopped => {
                if let Some(exit_code) = container_info.exit_code {
                    format!("Stopped (exit code: {})", exit_code)
                } else {
                    "Stopped".to_string()
                }
            }
            ContainerStatus::Dead => "Dead".to_string(),
            ContainerStatus::Unknown => "Unknown".to_string(),
        }
    }
    
    /// Generates a container summary.
    pub fn generate_summary(&self, container_info: &ContainerInfo) -> HashMap<String, String> {
        let mut summary = HashMap::new();
        
        summary.insert("id".to_string(), container_info.id.clone());
        summary.insert("name".to_string(), container_info.name.clone());
        summary.insert("image".to_string(), container_info.image.clone());
        summary.insert("status".to_string(), self.format_status(container_info));
        summary.insert("created".to_string(), container_info.created.to_rfc3339());
        
        if let Some(started) = container_info.started {
            summary.insert("started".to_string(), started.to_rfc3339());
        }
        
        if let Some(finished) = container_info.finished {
            summary.insert("finished".to_string(), finished.to_rfc3339());
        }
        
        if let Some(exit_code) = container_info.exit_code {
            summary.insert("exit_code".to_string(), exit_code.to_string());
        }
        
        summary
    }
    
    /// Validates resource constraints.
    pub fn validate_resources(&self, resources: &ResourceConstraints) -> Result<()> {
        // Check CPU shares
        if let Some(cpu_shares) = resources.cpu_shares {
            if cpu_shares == 0 {
                return Err(ContainerError::ResourceError("CPU shares cannot be 0".to_string()));
            }
        }
        
        // Check CPU quota
        if let Some(cpu_quota) = resources.cpu_quota {
            if cpu_quota < 1000 && cpu_quota != -1 {
                return Err(ContainerError::ResourceError("CPU quota must be at least 1000 microseconds or -1 for no limit".to_string()));
            }
        }
        
        // Check CPU period
        if let Some(cpu_period) = resources.cpu_period {
            if cpu_period < 1000 {
                return Err(ContainerError::ResourceError("CPU period must be at least 1000 microseconds".to_string()));
            }
        }
        
        // Check CPUs
        if let Some(cpus) = resources.cpus {
            if cpus <= 0.0 {
                return Err(ContainerError::ResourceError("CPUs must be positive".to_string()));
            }
        }
        
        // Check memory
        if let Some(memory) = resources.memory {
            if memory < 4 * 1024 * 1024 { // 4 MB minimum
                return Err(ContainerError::ResourceError("Memory must be at least 4 MB".to_string()));
            }
        }
        
        // Check memory swap
        if let Some(memory_swap) = resources.memory_swap {
            if memory_swap != -1 && memory_swap < 0 {
                return Err(ContainerError::ResourceError("Memory swap must be -1 or non-negative".to_string()));
            }
        }
        
        Ok(())
    }
}

impl Default for ContainerManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_validate_spec() {
        let manager = ContainerManager::new();
        
        // Valid spec
        let spec = ContainerSpec {
            name: "test".to_string(),
            image: "alpine:latest".to_string(),
            ..Default::default()
        };
        assert!(manager.validate_spec(&spec).is_ok());
        
        // Invalid: empty name
        let spec = ContainerSpec {
            name: "".to_string(),
            image: "alpine:latest".to_string(),
            ..Default::default()
        };
        assert!(manager.validate_spec(&spec).is_err());
        
        // Invalid: empty image
        let spec = ContainerSpec {
            name: "test".to_string(),
            image: "".to_string(),
            ..Default::default()
        };
        assert!(manager.validate_spec(&spec).is_err());
        
        // Invalid: empty command
        let spec = ContainerSpec {
            name: "test".to_string(),
            image: "alpine:latest".to_string(),
            command: Some(vec![]),
            ..Default::default()
        };
        assert!(manager.validate_spec(&spec).is_err());
        
        // Invalid: malformed env var
        let spec = ContainerSpec {
            name: "test".to_string(),
            image: "alpine:latest".to_string(),
            env: vec!["MALFORMED".to_string()], // Missing =
            ..Default::default()
        };
        assert!(manager.validate_spec(&spec).is_err());
    }
    
    #[test]
    fn test_generate_env_vars() {
        let manager = ContainerManager::new();
        
        let base_vars = vec![
            "PATH=/usr/bin".to_string(),
            "HOME=/root".to_string(),
        ];
        
        let mut extra_vars = HashMap::new();
        extra_vars.insert("AGENT_ID".to_string(), "123".to_string());
        extra_vars.insert("DEBUG".to_string(), "true".to_string());
        
        let env_vars = manager.generate_env_vars(&base_vars, &extra_vars);
        
        assert_eq!(env_vars.len(), 4);
        assert!(env_vars.contains(&"PATH=/usr/bin".to_string()));
        assert!(env_vars.contains(&"HOME=/root".to_string()));
        assert!(env_vars.contains(&"AGENT_ID=123".to_string()));
        assert!(env_vars.contains(&"DEBUG=true".to_string()));
    }
    
    #[test]
    fn test_generate_labels() {
        let manager = ContainerManager::new();
        
        let mut base_labels = HashMap::new();
        base_labels.insert("app".to_string(), "agent".to_string());
        
        let mut extra_labels = HashMap::new();
        extra_labels.insert("version".to_string(), "1.0".to_string());
        
        let labels = manager.generate_labels(&base_labels, &extra_labels);
        
        assert!(labels.contains_key("app"));
        assert_eq!(labels.get("app"), Some(&"agent".to_string()));
        assert!(labels.contains_key("version"));
        assert_eq!(labels.get("version"), Some(&"1.0".to_string()));
        assert!(labels.contains_key("org.agent-sdk.created"));
    }
    
    #[test]
    fn test_is_healthy() {
        let manager = ContainerManager::new();
        
        let mut container = ContainerInfo {
            id: "test".to_string(),
            name: "test".to_string(),
            image: "alpine".to_string(),
            status: ContainerStatus::Running,
            created: chrono::Utc::now(),
            started: Some(chrono::Utc::now()),
            finished: None,
            exit_code: None,
            labels: HashMap::new(),
            annotations: HashMap::new(),
        };
        
        assert!(manager.is_healthy(&container));
        
        container.status = ContainerStatus::Stopped;
        assert!(!manager.is_healthy(&container));
        
        container.status = ContainerStatus::Created;
        assert!(!manager.is_healthy(&container));
    }
    
    #[test]
    fn test_validate_resources() {
        let manager = ContainerManager::new();
        
        // Valid resources
        let resources = ResourceConstraints {
            cpu_shares: Some(1024),
            memory: Some(128 * 1024 * 1024), // 128 MB
            ..Default::default()
        };
        assert!(manager.validate_resources(&resources).is_ok());
        
        // Invalid: CPU shares = 0
        let resources = ResourceConstraints {
            cpu_shares: Some(0),
            ..Default::default()
        };
        assert!(manager.validate_resources(&resources).is_err());
        
        // Invalid: memory too small
        let resources = ResourceConstraints {
            memory: Some(2 * 1024 * 1024), // 2 MB
            ..Default::default()
        };
        assert!(manager.validate_resources(&resources).is_err());
    }
}