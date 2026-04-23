//! Distributed locking with Redis.

use crate::store::CacheStore;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Distributed lock.
pub struct DistributedLock {
    key: String,
    value: String,
    ttl_secs: u64,
    store: *const CacheStore,
}

unsafe impl Send for DistributedLock {}
unsafe impl Sync for DistributedLock {}

impl DistributedLock {
    /// Acquire a distributed lock.
    pub async fn acquire(store: &CacheStore, key: &str, ttl_secs: u64) -> Result<Self> {
        let value = Uuid::new_v4().to_string();
        
        // Try to set with NX (only if not exists)
        let success = try_acquire_lock(store, key, &value, ttl_secs).await?;
        
        if success {
            Ok(Self {
                key: key.to_string(),
                value,
                ttl_secs,
                store: store as *const CacheStore,
            })
        } else {
            Err(anyhow::anyhow!("Failed to acquire lock: {}", key))
        }
    }

    /// Release the lock.
    pub async fn release(&self) -> Result<()> {
        let store = unsafe { &*self.store };
        
        // Only release if we still own the lock
        let current_value: Option<String> = store.get(&self.key).await?;
        
        if current_value.as_ref() == Some(&self.value) {
            store.delete(&self.key).await?;
        }

        Ok(())
    }

    /// Extend lock TTL.
    pub async fn extend(&self, ttl_secs: u64) -> Result<()> {
        let store = unsafe { &*self.store };
        
        // Only extend if we still own the lock
        let current_value: Option<String> = store.get(&self.key).await?;
        
        if current_value.as_ref() == Some(&self.value) {
            store.refresh_ttl(&self.key, ttl_secs).await?;
        }

        Ok(())
    }
}

impl Drop for DistributedLock {
    fn drop(&mut self) {
        // Try to release lock on drop (best effort)
        let store = unsafe { &*self.store };
        let key = self.key.clone();
        let value = self.value.clone();

        tokio::spawn(async move {
            let current_value: Option<String> = store.get(&key).await.unwrap_or(None);
            if current_value.as_ref() == Some(&value) {
                let _ = store.delete(&key).await;
            }
        });
    }
}

async fn try_acquire_lock(store: &CacheStore, key: &str, value: &str, ttl_secs: u64) -> Result<bool> {
    // Try to set with NX option
    // This is a simplified implementation - would use Redis SET NX in production
    
    let exists = store.exists(key).await?;
    
    if !exists {
        store.set(key, &value.to_string(), ttl_secs).await?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Lock guard for automatic release.
pub struct LockGuard {
    lock: Option<DistributedLock>,
}

impl LockGuard {
    pub fn new(lock: DistributedLock) -> Self {
        Self {
            lock: Some(lock),
        }
    }

    pub fn release(mut self) -> Result<()> {
        if let Some(lock) = self.lock.take() {
            futures::executor::block_on(lock.release())?;
        }
        Ok(())
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        if let Some(lock) = self.lock.take() {
            tokio::spawn(async move {
                let _ = lock.release().await;
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CacheConfig, CacheManager};

    #[tokio::test]
    async fn test_distributed_lock() {
        let config = CacheConfig {
            cache_type: crate::CacheType::InMemory,
            ..Default::default()
        };
        let manager = CacheManager::new(config);
        manager.initialize().await.unwrap();

        // Acquire lock
        let lock = manager.acquire_lock("test-lock", 10).await.unwrap();
        
        // Try to acquire same lock again (should fail)
        let result = manager.acquire_lock("test-lock", 10).await;
        assert!(result.is_err());

        // Release lock
        drop(lock);
        
        // Small delay for async cleanup
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Now should be able to acquire
        let lock2 = manager.acquire_lock("test-lock", 10).await;
        assert!(lock2.is_ok());
    }
}
