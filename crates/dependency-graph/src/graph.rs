//! Core graph data structures and algorithms.

use petgraph::{
    graph::{DiGraph, NodeIndex},
    visit::{Dfs, EdgeRef},
    Direction,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::error::DependencyError;

/// Unique identifier for a node in the dependency graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(Uuid);

impl NodeId {
    /// Create a new random node ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create a node ID from a UUID.
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Get the underlying UUID.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Type of dependency edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeType {
    /// Hard dependency: target cannot start until source completes.
    Hard,
    /// Soft dependency: target can start but may wait for source.
    Soft,
    /// Data flow: source produces data consumed by target.
    Data,
    /// Control flow: source controls execution of target.
    Control,
}

/// Node in the dependency graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    /// Unique identifier.
    pub id: NodeId,
    /// Human‑readable label.
    pub label: String,
    /// Type of node (e.g., "agent", "task", "resource").
    pub node_type: String,
    /// Arbitrary metadata.
    pub metadata: serde_json::Value,
    /// Whether the node is active.
    pub active: bool,
}

impl Node {
    /// Create a new node.
    pub fn new(label: impl Into<String>, node_type: impl Into<String>) -> Self {
        Self {
            id: NodeId::new(),
            label: label.into(),
            node_type: node_type.into(),
            metadata: serde_json::Value::Null,
            active: true,
        }
    }

    /// Set metadata.
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Edge in the dependency graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    /// Source node ID.
    pub from: NodeId,
    /// Target node ID.
    pub to: NodeId,
    /// Type of dependency.
    pub edge_type: EdgeType,
    /// Optional weight (e.g., latency, priority).
    pub weight: f64,
    /// Arbitrary metadata.
    pub metadata: serde_json::Value,
}

impl Edge {
    /// Create a new edge.
    pub fn new(from: NodeId, to: NodeId, edge_type: EdgeType) -> Self {
        Self {
            from,
            to,
            edge_type,
            weight: 1.0,
            metadata: serde_json::Value::Null,
        }
    }

    /// Set weight.
    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }
}

/// Directed acyclic graph of dependencies.
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    /// Underlying petgraph DiGraph.
    inner: DiGraph<Node, Edge>,
    /// Mapping from NodeId to NodeIndex.
    index_map: HashMap<NodeId, NodeIndex>,
    /// Reverse mapping from NodeIndex to NodeId.
    reverse_map: HashMap<NodeIndex, NodeId>,
}

impl DependencyGraph {
    /// Create a new empty graph.
    pub fn new() -> Self {
        Self {
            inner: DiGraph::new(),
            index_map: HashMap::new(),
            reverse_map: HashMap::new(),
        }
    }

    /// Add a node to the graph.
    pub fn add_node(&mut self, node: Node) -> NodeId {
        let idx = self.inner.add_node(node.clone());
        self.index_map.insert(node.id, idx);
        self.reverse_map.insert(idx, node.id);
        node.id
    }

    /// Remove a node and all incident edges.
    pub fn remove_node(&mut self, node_id: NodeId) -> Result<(), DependencyError> {
        let idx = self
            .index_map
            .get(&node_id)
            .ok_or_else(|| DependencyError::node_not_found(node_id.to_string()))?;
        self.inner.remove_node(*idx);
        self.index_map.remove(&node_id);
        self.reverse_map.remove(idx);
        Ok(())
    }

    /// Add an edge between two nodes.
    pub fn add_edge(&mut self, edge: Edge) -> Result<(), DependencyError> {
        let from_idx = *self
            .index_map
            .get(&edge.from)
            .ok_or_else(|| DependencyError::node_not_found(edge.from.to_string()))?;
        let to_idx = *self
            .index_map
            .get(&edge.to)
            .ok_or_else(|| DependencyError::node_not_found(edge.to.to_string()))?;

        // Check for cycles before adding edge.
        let mut test_graph = self.inner.clone();
        test_graph.add_edge(from_idx, to_idx, edge.clone());
        if petgraph::algo::is_cyclic_directed(&test_graph) {
            return Err(DependencyError::cycle());
        }

        self.inner.add_edge(from_idx, to_idx, edge);
        Ok(())
    }

    /// Remove an edge.
    pub fn remove_edge(&mut self, from: NodeId, to: NodeId) -> Result<(), DependencyError> {
        let from_idx = *self
            .index_map
            .get(&from)
            .ok_or_else(|| DependencyError::node_not_found(from.to_string()))?;
        let to_idx = *self
            .index_map
            .get(&to)
            .ok_or_else(|| DependencyError::node_not_found(to.to_string()))?;

        let edge_idx = self
            .inner
            .find_edge(from_idx, to_idx)
            .ok_or_else(|| DependencyError::edge_not_found(from.to_string(), to.to_string()))?;
        self.inner.remove_edge(edge_idx);
        Ok(())
    }

    /// Get a node by ID.
    pub fn node(&self, node_id: NodeId) -> Option<&Node> {
        self.index_map
            .get(&node_id)
            .and_then(|idx| self.inner.node_weight(*idx))
    }

    /// Get all nodes.
    pub fn nodes(&self) -> Vec<&Node> {
        self.inner.node_weights().collect()
    }

    /// Get all edges.
    pub fn edges(&self) -> Vec<&Edge> {
        self.inner.edge_weights().collect()
    }

    /// Get predecessors of a node.
    pub fn predecessors(&self, node_id: NodeId) -> Vec<NodeId> {
        let idx = match self.index_map.get(&node_id) {
            Some(idx) => *idx,
            None => return Vec::new(),
        };
        self.inner
            .neighbors_directed(idx, Direction::Incoming)
            .filter_map(|pred_idx| self.reverse_map.get(&pred_idx).copied())
            .collect()
    }

    /// Get successors of a node.
    pub fn successors(&self, node_id: NodeId) -> Vec<NodeId> {
        let idx = match self.index_map.get(&node_id) {
            Some(idx) => *idx,
            None => return Vec::new(),
        };
        self.inner
            .neighbors_directed(idx, Direction::Outgoing)
            .filter_map(|succ_idx| self.reverse_map.get(&succ_idx).copied())
            .collect()
    }

    /// Perform topological sort of nodes.
    pub fn topological_sort(&self) -> Result<Vec<NodeId>, DependencyError> {
        match petgraph::algo::toposort(&self.inner, None) {
            Ok(order) => Ok(order
                .into_iter()
                .filter_map(|idx| self.reverse_map.get(&idx).copied())
                .collect()),
            Err(_) => Err(DependencyError::NotADag),
        }
    }

    /// Check if the graph contains a cycle.
    pub fn has_cycle(&self) -> bool {
        petgraph::algo::is_cyclic_directed(&self.inner)
    }

    /// Find all nodes that have no incoming edges (roots).
    pub fn roots(&self) -> Vec<NodeId> {
        self.inner
            .node_indices()
            .filter(|&idx| self.inner.neighbors_directed(idx, Direction::Incoming).count() == 0)
            .filter_map(|idx| self.reverse_map.get(&idx).copied())
            .collect()
    }

    /// Find all nodes that have no outgoing edges (leaves).
    pub fn leaves(&self) -> Vec<NodeId> {
        self.inner
            .node_indices()
            .filter(|&idx| self.inner.neighbors_directed(idx, Direction::Outgoing).count() == 0)
            .filter_map(|idx| self.reverse_map.get(&idx).copied())
            .collect()
    }

    /// Serialize the graph to JSON.
    pub fn to_json(&self) -> Result<String, DependencyError> {
        let nodes: Vec<&Node> = self.nodes();
        let edges: Vec<&Edge> = self.edges();
        let serializable = SerializableGraph { nodes, edges };
        serde_json::to_string(&serializable).map_err(Into::into)
    }

    /// Deserialize a graph from JSON.
    pub fn from_json(json: &str) -> Result<Self, DependencyError> {
        let serializable: SerializableGraph = serde_json::from_str(json)?;
        let mut graph = Self::new();
        for node in serializable.nodes {
            graph.add_node(node.clone());
        }
        for edge in serializable.edges {
            graph.add_edge(edge.clone())?;
        }
        Ok(graph)
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper struct for serialization.
#[derive(Serialize, Deserialize)]
struct SerializableGraph<'a> {
    nodes: Vec<&'a Node>,
    edges: Vec<&'a Edge>,
}