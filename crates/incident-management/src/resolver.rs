//! Incident resolution and automation.

use std::sync::Arc;
use tokio::sync::Mutex;

use crate::error::{IncidentError, Result};
use crate::model::{Incident, IncidentId, IncidentStatus};
use crate::tracker::IncidentTracker;

/// Trait for automated incident resolvers.
#[async_trait::async_trait]
pub trait IncidentResolver: Send + Sync {
    /// Attempt to resolve an incident automatically.
    /// Returns true if resolution was successful.
    async fn resolve(&self, incident: &Incident) -> Result<bool>;

    /// Get resolver name.
    fn name(&self) -> &str;
}

/// Simple resolver that marks incidents as resolved after a timeout.
pub struct TimeoutResolver {
    timeout_seconds: u64,
}

impl TimeoutResolver {
    /// Create a new timeout resolver.
    pub fn new(timeout_seconds: u64) -> Self {
        Self { timeout_seconds }
    }
}

#[async_trait::async_trait]
impl IncidentResolver for TimeoutResolver {
    async fn resolve(&self, incident: &Incident) -> Result<bool> {
        // In a real implementation, you would check if the incident has been
        // in a certain status for longer than timeout.
        // For now, we just return false.
        Ok(false)
    }

    fn name(&self) -> &str {
        "timeout"
    }
}

/// Resolver that runs a script or command.
pub struct ScriptResolver {
    script_path: String,
}

impl ScriptResolver {
    /// Create a new script resolver.
    pub fn new(script_path: impl Into<String>) -> Self {
        Self {
            script_path: script_path.into(),
        }
    }
}

#[async_trait::async_trait]
impl IncidentResolver for ScriptResolver {
    async fn resolve(&self, _incident: &Incident) -> Result<bool> {
        // In a real implementation, execute the script.
        Ok(false)
    }

    fn name(&self) -> &str {
        "script"
    }
}

/// Composite resolver that tries multiple resolvers in order.
pub struct CompositeResolver {
    resolvers: Vec<Arc<dyn IncidentResolver>>,
}

impl CompositeResolver {
    /// Create a new composite resolver.
    pub fn new() -> Self {
        Self {
            resolvers: Vec::new(),
        }
    }

    /// Add a resolver.
    pub fn add_resolver(&mut self, resolver: Arc<dyn IncidentResolver>) {
        self.resolvers.push(resolver);
    }

    /// Try to resolve an incident using all resolvers.
    pub async fn try_resolve(&self, incident: &Incident) -> Result<bool> {
        for resolver in &self.resolvers {
            if resolver.resolve(incident).await? {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

impl Default for CompositeResolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Incident resolution manager.
pub struct IncidentResolverManager {
    tracker: Arc<IncidentTracker>,
    resolver: Arc<CompositeResolver>,
    auto_resolve_enabled: bool,
}

impl IncidentResolverManager {
    /// Create a new resolution manager.
    pub fn new(tracker: Arc<IncidentTracker>, resolver: Arc<CompositeResolver>) -> Self {
        Self {
            tracker,
            resolver,
            auto_resolve_enabled: true,
        }
    }

    /// Enable or disable auto‑resolution.
    pub fn set_auto_resolve(&mut self, enabled: bool) {
        self.auto_resolve_enabled = enabled;
    }

    /// Process unresolved incidents and attempt auto‑resolution.
    pub async fn process(&self) -> Result<Vec<IncidentId>> {
        if !self.auto_resolve_enabled {
            return Ok(Vec::new());
        }

        let unresolved = self.tracker.list_by_status(IncidentStatus::New)
            .into_iter()
            .chain(self.tracker.list_by_status(IncidentStatus::Investigating))
            .chain(self.tracker.list_by_status(IncidentStatus::Resolving))
            .collect::<Vec<_>>();

        let mut resolved_ids = Vec::new();
        for incident in unresolved {
            if self.resolver.try_resolve(&incident).await? {
                // Mark incident as resolved
                self.tracker.update_status(incident.id, IncidentStatus::Resolved).await?;
                resolved_ids.push(incident.id);
            }
        }

        Ok(resolved_ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{IncidentSeverity, IncidentSource};

    #[tokio::test]
    async fn test_timeout_resolver() {
        let resolver = TimeoutResolver::new(60);
        let incident = Incident::new(
            "test",
            "test",
            IncidentSeverity::Info,
            IncidentSource::Custom("test".to_string()),
        );
        let result = resolver.resolve(&incident).await.unwrap();
        assert!(!result);
    }
}