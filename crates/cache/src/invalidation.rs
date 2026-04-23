//! Cache invalidation strategies.

use serde::{Deserialize, Serialize};

/// Invalidation strategy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InvalidationStrategy {
    /// Time-based invalidation (TTL)
    TTL,
    /// Manual invalidation
    Manual,
    /// Pattern-based invalidation
    Pattern,
}

/// Cache invalidation manager.
pub struct InvalidationManager {
    strategy: InvalidationStrategy,
    pending_invalidations: tokio::sync::Mutex<Vec<String>>,
}

impl InvalidationManager {
    pub fn new(strategy: InvalidationStrategy) -> Self {
        Self {
            strategy,
            pending_invalidations: tokio::sync::Mutex::new(Vec::new()),
        }
    }

    pub fn get_strategy(&self) -> &InvalidationStrategy {
        &self.strategy
    }

    pub async fn schedule_invalidation(&self, key: &str) {
        if matches!(self.strategy, InvalidationStrategy::Manual) {
            let mut pending = self.pending_invalidations.lock().await;
            pending.push(key.to_string());
        }
    }

    pub async fn get_pending(&self) -> Vec<String> {
        let mut pending = self.pending_invalidations.lock().await;
        std::mem::take(&mut *pending)
    }

    pub async fn clear_pending(&self) {
        let mut pending = self.pending_invalidations.lock().await;
        pending.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_invalidation_manager() {
        let manager = InvalidationManager::new(InvalidationStrategy::Manual);

        manager.schedule_invalidation("key1").await;
        manager.schedule_invalidation("key2").await;

        let pending = manager.get_pending().await;
        assert_eq!(pending.len(), 2);
        assert!(pending.contains(&"key1".to_string()));
        assert!(pending.contains(&"key2".to_string()));

        manager.clear_pending().await;
        assert!(manager.get_pending().await.is_empty());
    }
}
