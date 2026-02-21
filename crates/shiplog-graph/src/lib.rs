//! Graph data structure utilities for shiplog.
//!
//! Provides basic graph data structures for representing nodes, edges,
//! and adjacency lists.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// A node identifier in a graph.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub String);

impl NodeId {
    /// Create a new node ID from a string.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// An edge between two nodes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Edge {
    /// Source node ID
    pub source: NodeId,
    /// Target node ID
    pub target: NodeId,
    /// Optional edge weight
    pub weight: Option<f64>,
}

impl Edge {
    /// Create a new edge from source to target.
    pub fn new(source: NodeId, target: NodeId) -> Self {
        Self {
            source,
            target,
            weight: None,
        }
    }

    /// Create a weighted edge.
    pub fn with_weight(source: NodeId, target: NodeId, weight: f64) -> Self {
        Self {
            source,
            target,
            weight: Some(weight),
        }
    }
}

/// A directed graph data structure.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Graph {
    /// Adjacency list representation
    adjacency: HashMap<NodeId, HashSet<NodeId>>,
    /// All nodes in the graph
    nodes: HashSet<NodeId>,
    /// All edges in the graph
    edges: Vec<Edge>,
}

impl Graph {
    /// Create a new empty graph.
    pub fn new() -> Self {
        Self {
            adjacency: HashMap::new(),
            nodes: HashSet::new(),
            edges: Vec::new(),
        }
    }

    /// Add a node to the graph.
    pub fn add_node(&mut self, node: NodeId) {
        self.nodes.insert(node.clone());
        self.adjacency.entry(node).or_default();
    }

    /// Add an edge to the graph.
    pub fn add_edge(&mut self, edge: Edge) {
        self.add_node(edge.source.clone());
        self.add_node(edge.target.clone());
        self.edges.push(edge.clone());
        if let Some(neighbors) = self.adjacency.get_mut(&edge.source) {
            neighbors.insert(edge.target);
        }
    }

    /// Check if a node exists in the graph.
    pub fn contains_node(&self, node: &NodeId) -> bool {
        self.nodes.contains(node)
    }

    /// Get all nodes in the graph.
    pub fn nodes(&self) -> impl Iterator<Item = &NodeId> {
        self.nodes.iter()
    }

    /// Get all edges in the graph.
    pub fn edges(&self) -> impl Iterator<Item = &Edge> {
        self.edges.iter()
    }

    /// Get neighbors of a node.
    pub fn neighbors(&self, node: &NodeId) -> Vec<NodeId> {
        self.adjacency
            .get(node)
            .map(|n| n.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Get the number of nodes.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get the number of edges.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Check if there's an edge from source to target.
    pub fn has_edge(&self, source: &NodeId, target: &NodeId) -> bool {
        self.adjacency
            .get(source)
            .map(|n| n.contains(target))
            .unwrap_or(false)
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self("".to_string())
    }
}

use std::fmt;

/// A directed acyclic graph (DAG).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Dag {
    graph: Graph,
}

impl Dag {
    /// Create a new empty DAG.
    pub fn new() -> Self {
        Self {
            graph: Graph::new(),
        }
    }

    /// Add a node to the DAG.
    pub fn add_node(&mut self, node: NodeId) {
        self.graph.add_node(node);
    }

    /// Add an edge to the DAG.
    pub fn add_edge(&mut self, edge: Edge) {
        self.graph.add_edge(edge);
    }

    /// Get all nodes in the DAG.
    pub fn nodes(&self) -> impl Iterator<Item = &NodeId> {
        self.graph.nodes()
    }

    /// Get all edges in the DAG.
    pub fn edges(&self) -> impl Iterator<Item = &Edge> {
        self.graph.edges()
    }

    /// Get the number of nodes.
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Get the number of edges.
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_id_creation() {
        let id = NodeId::new("node1");
        assert_eq!(id.0, "node1");
    }

    #[test]
    fn node_id_display() {
        let id = NodeId::new("node1");
        assert_eq!(format!("{}", id), "node1");
    }

    #[test]
    fn edge_creation() {
        let edge = Edge::new(NodeId::new("a"), NodeId::new("b"));
        assert_eq!(edge.source.0, "a");
        assert_eq!(edge.target.0, "b");
        assert_eq!(edge.weight, None);
    }

    #[test]
    fn weighted_edge() {
        let edge = Edge::with_weight(NodeId::new("a"), NodeId::new("b"), 1.5);
        assert_eq!(edge.weight, Some(1.5));
    }

    #[test]
    fn graph_add_node() {
        let mut graph = Graph::new();
        graph.add_node(NodeId::new("node1"));
        assert!(graph.contains_node(&NodeId::new("node1")));
    }

    #[test]
    fn graph_add_edge() {
        let mut graph = Graph::new();
        graph.add_edge(Edge::new(NodeId::new("a"), NodeId::new("b")));
        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 1);
    }

    #[test]
    fn graph_neighbors() {
        let mut graph = Graph::new();
        graph.add_edge(Edge::new(NodeId::new("a"), NodeId::new("b")));
        graph.add_edge(Edge::new(NodeId::new("a"), NodeId::new("c")));

        let neighbors = graph.neighbors(&NodeId::new("a"));
        assert_eq!(neighbors.len(), 2);
    }

    #[test]
    fn graph_has_edge() {
        let mut graph = Graph::new();
        graph.add_edge(Edge::new(NodeId::new("a"), NodeId::new("b")));

        assert!(graph.has_edge(&NodeId::new("a"), &NodeId::new("b")));
        assert!(!graph.has_edge(&NodeId::new("b"), &NodeId::new("a")));
    }

    #[test]
    fn dag_basic() {
        let mut dag = Dag::new();
        dag.add_node(NodeId::new("a"));
        dag.add_node(NodeId::new("b"));
        dag.add_edge(Edge::new(NodeId::new("a"), NodeId::new("b")));

        assert_eq!(dag.node_count(), 2);
        assert_eq!(dag.edge_count(), 1);
    }

    #[test]
    fn graph_default() {
        let graph: Graph = Graph::default();
        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn dag_default() {
        let dag: Dag = Dag::default();
        assert_eq!(dag.node_count(), 0);
        assert_eq!(dag.edge_count(), 0);
    }
}
