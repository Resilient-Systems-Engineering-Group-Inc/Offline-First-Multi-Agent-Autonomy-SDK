//! Memory buffer abstraction for device memory.

use crate::error::{Result, AccelerationError};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Memory buffer allocated on a device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryBuffer {
    /// Unique identifier for this buffer.
    pub id: String,
    /// Size in bytes.
    pub size: usize,
    /// Device‑specific handle (opaque).
    #[serde(skip)]
    pub handle: Option<Arc<dyn std::any::Any + Send + Sync>>,
    /// Whether the buffer is currently mapped to host memory.
    pub mapped: bool,
    /// Memory type (host, device, unified).
    pub memory_type: MemoryType,
}

/// Type of memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryType {
    /// Host memory (CPU‑accessible).
    Host,
    /// Device memory (GPU/TPU/FPGA).
    Device,
    /// Unified memory (accessible by both host and device).
    Unified,
    /// Pinned host memory (fast DMA).
    Pinned,
}

impl MemoryBuffer {
    /// Creates a new memory buffer description (without actual allocation).
    pub fn new(size: usize, memory_type: MemoryType) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            size,
            handle: None,
            mapped: false,
            memory_type,
        }
    }

    /// Attaches a device‑specific handle.
    pub fn with_handle(mut self, handle: Arc<dyn std::any::Any + Send + Sync>) -> Self {
        self.handle = Some(handle);
        self
    }

    /// Returns whether the buffer has a valid handle.
    pub fn is_allocated(&self) -> bool {
        self.handle.is_some()
    }

    /// Returns a reference to the handle if present.
    pub fn handle(&self) -> Option<&Arc<dyn std::any::Any + Send + Sync>> {
        self.handle.as_ref()
    }
}

/// Memory manager that tracks allocated buffers.
pub struct MemoryManager {
    buffers: Vec<MemoryBuffer>,
    total_allocated: usize,
    max_memory: Option<usize>,
}

impl MemoryManager {
    /// Creates a new memory manager with an optional limit.
    pub fn new(max_memory: Option<usize>) -> Self {
        Self {
            buffers: Vec::new(),
            total_allocated: 0,
            max_memory,
        }
    }

    /// Registers a buffer as allocated.
    pub fn register(&mut self, buffer: MemoryBuffer) -> Result<()> {
        if let Some(max) = self.max_memory {
            if self.total_allocated + buffer.size > max {
                return Err(AccelerationError::OutOfMemory(format!(
                    "Exceeds limit {} bytes",
                    max
                )));
            }
        }
        self.total_allocated += buffer.size;
        self.buffers.push(buffer);
        Ok(())
    }

    /// Unregisters a buffer (e.g., after deallocation).
    pub fn unregister(&mut self, buffer_id: &str) -> Option<MemoryBuffer> {
        if let Some(pos) = self.buffers.iter().position(|b| b.id == buffer_id) {
            let buffer = self.buffers.remove(pos);
            self.total_allocated -= buffer.size;
            Some(buffer)
        } else {
            None
        }
    }

    /// Returns all registered buffers.
    pub fn buffers(&self) -> &[MemoryBuffer] {
        &self.buffers
    }

    /// Returns total allocated memory in bytes.
    pub fn total_allocated(&self) -> usize {
        self.total_allocated
    }

    /// Returns available memory (if a limit is set).
    pub fn available_memory(&self) -> Option<usize> {
        self.max_memory.map(|max| max.saturating_sub(self.total_allocated))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_buffer() {
        let buffer = MemoryBuffer::new(1024, MemoryType::Device);
        assert_eq!(buffer.size, 1024);
        assert!(!buffer.is_allocated());
    }

    #[test]
    fn test_memory_manager() {
        let mut manager = MemoryManager::new(Some(2048));
        let buffer = MemoryBuffer::new(512, MemoryType::Device);
        manager.register(buffer).unwrap();
        assert_eq!(manager.total_allocated(), 512);
        assert_eq!(manager.available_memory(), Some(1536));

        let buffer2 = MemoryBuffer::new(2048, MemoryType::Device);
        assert!(manager.register(buffer2).is_err()); // exceeds limit
    }
}