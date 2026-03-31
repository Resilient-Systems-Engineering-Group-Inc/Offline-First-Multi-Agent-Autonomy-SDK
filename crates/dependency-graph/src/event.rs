//! Events emitted by the dependency graph scheduler.

use serde::{Deserialize, Serialize};
use crate::graph::NodeId;

/// Events that can occur during dependency graph scheduling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DependencyEvent {
    /// Scheduler has started.
    SchedulerStarted,
    /// Scheduler has finished.
    SchedulerFinished,
    /// A node has started execution.
    NodeStarted(NodeId),
    /// A node has completed successfully.
    NodeCompleted(NodeId),
    /// A node has failed.
    NodeFailed(NodeId, String),
    /// A node is blocked by dependencies.
    NodeBlocked(NodeId),
    /// A dependency edge was added.
    EdgeAdded(NodeId, NodeId),
    /// A dependency edge was removed.
    EdgeRemoved(NodeId, NodeId),
    /// A node was added to the graph.
    NodeAdded(NodeId),
    /// A node was removed from the graph.
    NodeRemoved(NodeId),
    /// The graph was updated (structural change).
    GraphUpdated,
}

impl DependencyEvent {
    /// Returns a human‑readable description of the event.
    pub fn description(&self) -> String {
        match self {
            Self::SchedulerStarted => "Scheduler started".to_string(),
            Self::SchedulerFinished => "Scheduler finished".to_string(),
            Self::NodeStarted(id) => format!("Node {} started", id),
            Self::NodeCompleted(id) => format!("Node {} completed", id),
            Self::NodeFailed(id, err) => format!("Node {} failed: {}", id, err),
            Self::NodeBlocked(id) => format!("Node {} blocked by dependencies", id),
            Self::EdgeAdded(from, to) => format!("Edge added from {} to {}", from, to),
            Self::EdgeRemoved(from, to) => format!("Edge removed from {} to {}", from, to),
            Self::NodeAdded(id) => format!("Node {} added", id),
            Self::NodeRemoved(id) => format!("Node {} removed", id),
            Self::GraphUpdated => "Graph updated".to_string(),
        }
    }

    /// Returns the event severity.
    pub fn severity(&self) -> EventSeverity {
        match self {
            Self::SchedulerStarted | Self::SchedulerFinished => EventSeverity::Info,
            Self::NodeStarted(_) | Self::NodeCompleted(_) | Self::NodeAdded(_) | Self::EdgeAdded(_, _) => EventSeverity::Info,
            Self::NodeBlocked(_) | Self::NodeRemoved(_) | Self::EdgeRemoved(_, _) | Self::GraphUpdated => EventSeverity::Warning,
            Self::NodeFailed(_, _) => EventSeverity::Error,
        }
    }
}

/// Severity of an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventSeverity {
    /// Informational event.
    Info,
    /// Warning event.
    Warning,
    /// Error event.
    Error,
}

impl std::fmt::Display for EventSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Info => write!(f, "INFO"),
            Self::Warning => write!(f, "WARNING"),
            Self::Error => write!(f, "ERROR"),
        }
    }
}