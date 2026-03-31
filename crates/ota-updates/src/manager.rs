//! Update manager.

use crate::error::Error;
use crate::package::{Package, PackageId, Version};
use crate::transport::UpdateTransport;
use dashmap::DashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Manages OTA updates for all packages.
pub struct UpdateManager {
    /// Installed packages.
    installed: Arc<DashMap<PackageId, Version>>,
    /// Transport for fetching updates.
    transport: Arc<dyn UpdateTransport + Send + Sync>,
    /// Directory where packages are stored.
    store_dir: PathBuf,
}

impl UpdateManager {
    /// Create a new update manager.
    pub fn new(
        transport: Arc<dyn UpdateTransport + Send + Sync>,
        store_dir: PathBuf,
    ) -> Result<Self, Error> {
        std::fs::create_dir_all(&store_dir)?;
        Ok(Self {
            installed: Arc::new(DashMap::new()),
            transport,
            store_dir,
        })
    }

    /// Register an installed package.
    pub fn register_installed(&self, id: PackageId, version: Version) {
        self.installed.insert(id, version);
    }

    /// Check for updates for a package.
    pub async fn check_for_updates(
        &self,
        id: &PackageId,
    ) -> Result<Option<Package>, Error> {
        let current = self.installed.get(id).map(|v| v.clone());
        let latest = self.transport.fetch_latest(id).await?;
        match (current, latest) {
            (Some(current_version), Some(latest_package)) => {
                if latest_package.version > current_version {
                    Ok(Some(latest_package))
                } else {
                    Ok(None)
                }
            }
            (None, Some(latest_package)) => Ok(Some(latest_package)),
            _ => Ok(None),
        }
    }

    /// Apply an update package.
    pub async fn apply_update(&self, package: Package) -> Result<(), Error> {
        // Validate package.
        package.validate()?;

        // Download payload.
        let payload = self.transport.download_package(&package).await?;

        // Store payload.
        let filename = format!("{}-{}.tar.gz", package.id, package.version.to_string());
        let path = self.store_dir.join(filename);
        tokio::fs::write(&path, payload).await?;

        // Apply package.
        package.apply(self.store_dir.clone()).await?;

        // Update installed version.
        self.installed
            .insert(package.id.clone(), package.version.clone());

        // Audit log.
        audit::AuditLogger::default()
            .log(audit::event::AuditEvent::new(
                audit::event::EventType::ConfigurationChange,
                audit::event::Severity::Info,
                None,
                None,
                format!("Applied OTA update for {} to {}", package.id, package.version.to_string()),
                serde_json::json!({ "package": package.id, "version": package.version.to_string() }),
            ))
            .await?;

        Ok(())
    }

    /// Rollback to previous version (if supported).
    pub async fn rollback(&self, id: &PackageId) -> Result<(), Error> {
        // TODO: implement rollback logic
        tracing::warn!("Rollback not yet implemented for {}", id);
        Ok(())
    }
}