//! Kernel abstraction for device‑side computation.

use crate::error::{Result, AccelerationError};
use async_trait::async_trait;
use std::sync::Arc;

/// A kernel (compute shader, CUDA kernel, etc.) that can be executed on a device.
#[async_trait]
pub trait Kernel: Send + Sync {
    /// Returns the kernel name (entry point).
    fn name(&self) -> &str;

    /// Returns the source code (if available).
    fn source(&self) -> Option<&str>;

    /// Returns the backend‑specific handle.
    fn handle(&self) -> Option<&dyn std::any::Any>;

    /// Sets kernel arguments (backend‑specific).
    async fn set_args(&self, args: &[&dyn std::any::Any]) -> Result<()>;
}

/// A simple kernel that stores source and a dummy handle.
pub struct SimpleKernel {
    name: String,
    source: String,
    handle: Option<Arc<dyn std::any::Any + Send + Sync>>,
}

impl SimpleKernel {
    /// Creates a new kernel with source code.
    pub fn new(name: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            source: source.into(),
            handle: None,
        }
    }

    /// Attaches a backend‑specific handle.
    pub fn with_handle(mut self, handle: Arc<dyn std::any::Any + Send + Sync>) -> Self {
        self.handle = Some(handle);
        self
    }
}

#[async_trait]
impl Kernel for SimpleKernel {
    fn name(&self) -> &str {
        &self.name
    }

    fn source(&self) -> Option<&str> {
        Some(&self.source)
    }

    fn handle(&self) -> Option<&dyn std::any::Any> {
        self.handle.as_ref().map(|h| h.as_ref())
    }

    async fn set_args(&self, _args: &[&dyn std::any::Any]) -> Result<()> {
        // Dummy implementation
        Ok(())
    }
}

/// Kernel manager that caches compiled kernels.
pub struct KernelManager {
    kernels: std::collections::HashMap<String, Arc<dyn Kernel>>,
}

impl KernelManager {
    /// Creates a new empty kernel manager.
    pub fn new() -> Self {
        Self {
            kernels: std::collections::HashMap::new(),
        }
    }

    /// Adds a kernel to the cache.
    pub fn add(&mut self, kernel: Arc<dyn Kernel>) {
        self.kernels.insert(kernel.name().to_string(), kernel);
    }

    /// Retrieves a kernel by name.
    pub fn get(&self, name: &str) -> Option<Arc<dyn Kernel>> {
        self.kernels.get(name).cloned()
    }

    /// Removes a kernel from the cache.
    pub fn remove(&mut self, name: &str) -> Option<Arc<dyn Kernel>> {
        self.kernels.remove(name)
    }

    /// Returns all kernel names.
    pub fn names(&self) -> Vec<String> {
        self.kernels.keys().cloned().collect()
    }
}

impl Default for KernelManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_kernel() {
        let kernel = SimpleKernel::new("add", "kernel void add(...) {}");
        assert_eq!(kernel.name(), "add");
        assert!(kernel.source().is_some());
    }

    #[tokio::test]
    async fn test_kernel_manager() {
        let mut manager = KernelManager::new();
        let kernel = Arc::new(SimpleKernel::new("test", "source"));
        manager.add(kernel);
        assert!(manager.get("test").is_some());
        assert_eq!(manager.names(), vec!["test"]);
    }
}