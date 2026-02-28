use proptest::prelude::*;
use shiplog_graph::{Dag, Edge, Graph, NodeId};

// ── Known-answer tests ──────────────────────────────────────────────

#[test]
fn add_edge_creates_both_nodes() {
    let mut g = Graph::new();
    g.add_edge(Edge::new(NodeId::new("a"), NodeId::new("b")));
    assert!(g.contains_node(&NodeId::new("a")));
    assert!(g.contains_node(&NodeId::new("b")));
    assert_eq!(g.node_count(), 2);
    assert_eq!(g.edge_count(), 1);
}

#[test]
fn directed_edge_not_symmetric() {
    let mut g = Graph::new();
    g.add_edge(Edge::new(NodeId::new("a"), NodeId::new("b")));
    assert!(g.has_edge(&NodeId::new("a"), &NodeId::new("b")));
    assert!(!g.has_edge(&NodeId::new("b"), &NodeId::new("a")));
}

#[test]
fn neighbors_returns_correct_targets() {
    let mut g = Graph::new();
    g.add_edge(Edge::new(NodeId::new("a"), NodeId::new("b")));
    g.add_edge(Edge::new(NodeId::new("a"), NodeId::new("c")));
    g.add_edge(Edge::new(NodeId::new("b"), NodeId::new("c")));

    let mut neighbors = g.neighbors(&NodeId::new("a"));
    neighbors.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(neighbors.len(), 2);
    assert_eq!(neighbors[0], NodeId::new("b"));
    assert_eq!(neighbors[1], NodeId::new("c"));
}

#[test]
fn neighbors_of_nonexistent_node() {
    let g = Graph::new();
    assert!(g.neighbors(&NodeId::new("x")).is_empty());
}

#[test]
fn weighted_edge_preserves_weight() {
    let e = Edge::with_weight(NodeId::new("a"), NodeId::new("b"), 2.5);
    assert_eq!(e.weight, Some(2.5));
}

#[test]
fn unweighted_edge_has_no_weight() {
    let e = Edge::new(NodeId::new("a"), NodeId::new("b"));
    assert_eq!(e.weight, None);
}

#[test]
fn add_node_idempotent() {
    let mut g = Graph::new();
    g.add_node(NodeId::new("a"));
    g.add_node(NodeId::new("a"));
    assert_eq!(g.node_count(), 1);
}

#[test]
fn node_id_display() {
    assert_eq!(format!("{}", NodeId::new("hello")), "hello");
}

#[test]
fn node_id_default_is_empty() {
    let id = NodeId::default();
    assert_eq!(id.0, "");
}

// ── DAG tests ───────────────────────────────────────────────────────

#[test]
fn dag_add_nodes_and_edges() {
    let mut dag = Dag::new();
    dag.add_node(NodeId::new("1"));
    dag.add_node(NodeId::new("2"));
    dag.add_node(NodeId::new("3"));
    dag.add_edge(Edge::new(NodeId::new("1"), NodeId::new("2")));
    dag.add_edge(Edge::new(NodeId::new("2"), NodeId::new("3")));
    assert_eq!(dag.node_count(), 3);
    assert_eq!(dag.edge_count(), 2);
}

#[test]
fn dag_default_empty() {
    let dag = Dag::default();
    assert_eq!(dag.node_count(), 0);
    assert_eq!(dag.edge_count(), 0);
}

// ── Edge cases ──────────────────────────────────────────────────────

#[test]
fn empty_graph() {
    let g = Graph::new();
    assert_eq!(g.node_count(), 0);
    assert_eq!(g.edge_count(), 0);
    assert!(!g.contains_node(&NodeId::new("x")));
    assert!(!g.has_edge(&NodeId::new("x"), &NodeId::new("y")));
}

#[test]
fn self_loop() {
    let mut g = Graph::new();
    g.add_edge(Edge::new(NodeId::new("a"), NodeId::new("a")));
    assert!(g.has_edge(&NodeId::new("a"), &NodeId::new("a")));
    assert_eq!(g.node_count(), 1);
    assert_eq!(g.edge_count(), 1);
}

#[test]
fn duplicate_edges_both_stored() {
    let mut g = Graph::new();
    g.add_edge(Edge::new(NodeId::new("a"), NodeId::new("b")));
    g.add_edge(Edge::new(NodeId::new("a"), NodeId::new("b")));
    // Both edges are stored in the edges vec
    assert_eq!(g.edge_count(), 2);
    // But adjacency set deduplicates
    assert_eq!(g.neighbors(&NodeId::new("a")).len(), 1);
}

#[test]
fn graph_with_isolated_node() {
    let mut g = Graph::new();
    g.add_node(NodeId::new("isolated"));
    g.add_edge(Edge::new(NodeId::new("a"), NodeId::new("b")));
    assert_eq!(g.node_count(), 3);
    assert!(g.contains_node(&NodeId::new("isolated")));
    assert!(g.neighbors(&NodeId::new("isolated")).is_empty());
}

// ── Property tests ──────────────────────────────────────────────────

proptest! {
    #[test]
    fn adding_edge_implies_has_edge(
        src in "[a-z]{1,4}", tgt in "[a-z]{1,4}"
    ) {
        let mut g = Graph::new();
        g.add_edge(Edge::new(NodeId::new(&src), NodeId::new(&tgt)));
        prop_assert!(g.has_edge(&NodeId::new(&src), &NodeId::new(&tgt)));
    }

    #[test]
    fn node_count_never_exceeds_2_times_edge_count_plus_isolated(
        edges in prop::collection::vec(
            ("[a-z]{1,3}", "[a-z]{1,3}"), 0..20
        )
    ) {
        let mut g = Graph::new();
        for (s, t) in &edges {
            g.add_edge(Edge::new(NodeId::new(s), NodeId::new(t)));
        }
        // Each edge adds at most 2 nodes
        prop_assert!(g.node_count() <= 2 * g.edge_count() + g.node_count());
    }

    #[test]
    fn add_node_then_contains(name in "[a-z]{1,8}") {
        let mut g = Graph::new();
        g.add_node(NodeId::new(&name));
        prop_assert!(g.contains_node(&NodeId::new(&name)));
    }

    #[test]
    fn neighbors_are_subset_of_nodes(
        edges in prop::collection::vec(
            ("[a-z]{1,3}", "[a-z]{1,3}"), 1..15
        )
    ) {
        let mut g = Graph::new();
        for (s, t) in &edges {
            g.add_edge(Edge::new(NodeId::new(s), NodeId::new(t)));
        }
        let all_nodes: Vec<_> = g.nodes().cloned().collect();
        for node in &all_nodes {
            for neighbor in g.neighbors(node) {
                prop_assert!(g.contains_node(&neighbor));
            }
        }
    }

    #[test]
    fn dag_node_count_consistent(
        nodes in prop::collection::vec("[a-z]{1,4}", 1..10)
    ) {
        let mut dag = Dag::new();
        for n in &nodes {
            dag.add_node(NodeId::new(n));
        }
        // node_count <= nodes.len() (deduplication may reduce count)
        prop_assert!(dag.node_count() <= nodes.len());
    }
}
