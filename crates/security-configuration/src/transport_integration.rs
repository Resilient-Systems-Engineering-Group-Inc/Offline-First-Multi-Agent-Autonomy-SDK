//! Integration with mesh transport for security configuration distribution.
//!
//! This module allows security configurations to be distributed across the mesh network,
//! enabling consistent security policies across all agents.
//!
//! Requires the `mesh-transport` feature.

use crate::config::SecurityConfig;
use crate::error::{Result, SecurityConfigError};
use crate::manager::SecurityConfigManager;
use async_trait::async_trait;
use futures::stream::BoxStream;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Trait for distributing security configurations over a transport.
#[async_trait]
pub trait ConfigDistributor: Send + Sync {
    /// Broadcasts the current configuration to all peers.
    async fn broadcast_config(&self, config: &SecurityConfig) -> Result<()>;

    /// Requests the latest configuration from a specific peer.
    async fn request_config(&self, peer_id: u64) -> Result<SecurityConfig>;

    /// Returns a stream of incoming configuration updates.
    fn config_updates(&self) -> BoxStream<'static, SecurityConfig>;
}

/// Mesh‑based distributor that uses the mesh transport.
pub struct MeshConfigDistributor {
    // In a real implementation you would hold a reference to a mesh transport.
    // For now we just stub the methods.
}

impl MeshConfigDistributor {
    /// Creates a new mesh distributor.
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl ConfigDistributor for MeshConfigDistributor {
    async fn broadcast_config(&self, _config: &SecurityConfig) -> Result<()> {
        tracing::info!("[MeshConfigDistributor] Broadcasting configuration (stub)");
        Ok(())
    }

    async fn request_config(&self, _peer_id: u64) -> Result<SecurityConfig> {
        tracing::info!("[MeshConfigDistributor] Requesting configuration from peer (stub)");
        Err(SecurityConfigError::Integration(
            "Mesh distributor not fully implemented".to_string(),
        ))
    }

    fn config_updates(&self) -> BoxStream<'static, SecurityConfig> {
        futures::stream::empty().boxed()
    }
}

/// Manager that combines a security config manager with distribution capabilities.
pub struct DistributedSecurityConfigManager {
    manager: Arc<RwLock<SecurityConfigManager>>,
    distributor: Arc<dyn ConfigDistributor>,
}

impl DistributedSecurityConfigManager {
    /// Creates a new distributed manager.
    pub fn new(
        manager: SecurityConfigManager,
        distributor: Arc<dyn ConfigDistributor>,
    ) -> Self {
        Self {
            manager: Arc::new(RwLock::new(manager)),
            distributor,
        }
    }

    /// Returns a reference to the inner manager.
    pub async fn get_manager(&self) -> Arc<RwLock<SecurityConfigManager>> {
        self.manager.clone()
    }

    /// Synchronizes the local configuration with the network.
    ///
    /// This method requests configurations from a quorum of peers, merges them,
    /// and updates the local configuration if a newer version is found.
    pub async fn sync_with_network(&self) -> Result<()> {
        tracing::info!("Syncing security configuration with network");
        // Stub implementation
        Ok(())
    }

    /// Publishes the local configuration to the network.
    pub async fn publish_local_config(&self) -> Result<()> {
        let manager = self.manager.read().await;
        let config = manager.get_config().await;
        self.distributor.broadcast_config(&config).await
    }

    /// Starts a background task that listens for configuration updates.
    pub async fn start_update_listener(&self) -> Result<()> {
        let manager = self.manager.clone();
        let updates = self.distributor.config_updates();
        tokio::spawn(async move {
            // In a real implementation you would process each incoming config.
            // For now we just log.
            tracing::info!("Security config update listener started (stub)");
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mesh_distributor_stub() {
        let distributor = MeshConfigDistributor::new();
        let config = SecurityConfig::default();
        assert!(distributor.broadcast_config(&config).await.is_ok());
        assert!(distributor.request_config(1).await.is_err());
        let updates = distributor.config_updates();
        assert_eq!(updates.count().await, 0);
    }
}