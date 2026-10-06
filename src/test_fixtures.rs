use crate::graph::Graph;
use crate::io::{EdgeRecord, GraphFile, NodeId, NodeRecord};

pub const COORDS: [(NodeId, f64, f64); 6] = [
    (50, 0.0, 0.0),
    (2, 4.0, 0.0),
    (17, 0.0, 3.0),
    (8, 3.0, 4.0),
    (33, 20.0, 20.0),
    (7, 20.0, 20.0),
];

pub const EDGES: [(NodeId, NodeId); 5] = [(50, 2), (50, 17), (17, 8), (2, 33), (33, 7)];

pub fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-9,
        "expected {expected}, got {actual}"
    );
}

/// Creates a graph file
pub fn file(nodes: &[(NodeId, f64, f64)], edges: &[(NodeId, NodeId)]) -> GraphFile {
    GraphFile {
        graph_id: "test".to_string(),
        nodes: nodes
            .iter()
            .map(|&(id, x, y)| NodeRecord { id, x, y })
            .collect(),
        edges: edges
            .iter()
            .map(|&(source, target)| EdgeRecord { source, target })
            .collect(),
    }
}

/// get index for node 50
pub fn anchor_50(graph: &Graph) -> usize {
    graph.index_of(50).unwrap()
}

/// Creates a graph
pub fn fixture() -> Graph {
    Graph::from_file(&file(&COORDS, &EDGES)).unwrap()
}
