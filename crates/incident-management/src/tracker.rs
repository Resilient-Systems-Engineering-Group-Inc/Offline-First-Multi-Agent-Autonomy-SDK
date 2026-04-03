//! Incident tracking and state management.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use dashmap::DashMap;

use crate::error::{IncidentError, Result};
use crate::model::{Incident, IncidentId, IncidentStatus};

/// Incident tracker that stores and manages incidents.
pub struct IncidentTracker {
    incidents: Arc<DashMap<IncidentId, Incident>>,
    status_history: Arc<RwLock<HashMap<IncidentId, Vec<IncidentStatus>>>>,
}

impl IncidentTracker {
    /// Create a new incident tracker.
    pub fn new() -> Self {
        Self {
            incidents: Arc::new(DashMap::new()),
            status_history: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a new incident.
    pub async fn add_incident(&self, incident: Incident) -> IncidentId {
        let id = incident.id;
        self.incidents.insert(id, incident);
        self.status_history.write().await.insert(id, vec![IncidentStatus::New]);
        id
    }

    /// Get an incident by ID.
    pub fn get_incident(&self, id: IncidentId) -> Option<Incident> {
        self.incidents.get(&id).map(|entry| entry.clone())
    }

    /// Update an incident.
    pub async fn update_incident(&self, id: IncidentId, mut incident: Incident) -> Result<()> {
        if !self.incidents.contains_key(&id) {
            return Err(IncidentError::NotFound(format!("incident {}", id)));
        }
        incident.id = id; // ensure ID matches
        self.incidents.insert(id, incident);
        Ok(())
    }

    /// Update the status of an incident.
    pub async fn update_status(&self, id: IncidentId, status: IncidentStatus) -> Result<()> {
        let mut incident = self.incidents.get_mut(&id)
            .ok_or_else(|| IncidentError::NotFound(format!("incident {}", id)))?;
        incident.update_status(status);
        // Record in history
        let mut history = self.status_history.write().await;
        history.entry(id).or_insert_with(Vec::new).push(status);
        Ok(())
    }

    /// List all incidents.
    pub fn list_incidents(&self) -> Vec<Incident> {
        self.incidents.iter().map(|entry| entry.clone()).collect()
    }

    /// List incidents with a specific status.
    pub fn list_by_status(&self, status: IncidentStatus) -> Vec<Incident> {
        self.incidents.iter()
            .filter(|entry| entry.status == status)
            .map(|entry| entry.clone())
            .collect()
    }

    /// List incidents with severity >= given severity.
    pub fn list_by_severity(&self, min_severity: crate::model::IncidentSeverity) -> Vec<Incident> {
        self.incidents.iter()
            .filter(|entry| entry.severity >= min_severity)
            .map(|entry| entry.clone())
            .collect()
    }

    /// Delete an incident (e.g., false positive).
    pub async fn delete_incident(&self, id: IncidentId) -> Result<()> {
        self.incidents.remove(&id);
        self.status_history.write().await.remove(&id);
        Ok(())
    }

    /// Get status history of an incident.
    pub async fn get_status_history(&self, id: IncidentId) -> Option<Vec<IncidentStatus>> {
        let history = self.status_history.read().await;
        history.get(&id).cloned()
    }
}

impl Default for IncidentTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{IncidentSeverity, IncidentSource};

    #[tokio::test]
    async fn test_add_and_get() {
        let tracker = IncidentTracker::new();
        let incident = crate::model::Incident::new(
            "test",
            "test",
            IncidentSeverity::Info,
            IncidentSource::Custom("test".to_string()),
        );
        let id = tracker.add_incident(incident).await;
        let retrieved = tracker.get_incident(id).unwrap();
        assert_eq!(retrieved.title, "test");
    }

    #[tokio::test]
    async fn test_update_status() {
        let tracker = IncidentTracker::new();
        let incident = crate::model::Incident::new(
            "test",
            "test",
            IncidentSeverity::Info,
            IncidentSource::Custom("test".to_string()),
        );
        let id = tracker.add_incident(incident).await;
        tracker.update_status(id, IncidentStatus::Acknowledged).await.unwrap();
        let incident = tracker.get_incident(id).unwrap();
        assert_eq!(incident.status, IncidentStatus::Acknowledged);
    }
}