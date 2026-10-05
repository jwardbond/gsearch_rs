use crate::geometry::Point;
use crate::io::{GraphFile, NodeId};

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

pub struct Graph {
    ids: Vec<NodeId>,
    coords: Vec<Point>,
    adj: Vec<Vec<usize>>,
    index_of: HashMap<NodeId, usize>,
}

/// Undirected graph with 2D coordinates for each node.
impl Graph {
    pub fn from_file(file: &GraphFile) -> Result<Graph, GraphError> {
        let n = file.nodes.len();
        let mut ids = Vec::with_capacity(n);
        let mut coords = Vec::with_capacity(n);
        let mut index_of = HashMap::with_capacity(n);

        for (i, node) in file.nodes.iter().enumerate() {
            if index_of.insert(node.id, i).is_some() {
                return Err(GraphError::DuplicateNode(node.id));
            }

            ids.push(node.id);
            coords.push(Point::new(node.x, node.y));
        }

        let lookup = |id| {
            index_of
                .get(&id)
                .copied()
                .ok_or(GraphError::UnknownNode(id))
        };

        let mut adj = vec![Vec::new(); n];
        for edge in &file.edges {
            let source = lookup(edge.source)?;
            let target = lookup(edge.target)?;

            if source != target {
                adj[source].push(target);
                adj[target].push(source);
            }
        }

        for n in &mut adj {
            n.sort_unstable();
            n.dedup();
        }

        Ok(Graph {
            ids,
            coords,
            adj,
            index_of,
        })
    }

    pub fn node_count(&self) -> usize {
        self.ids.len()
    }

    pub fn edge_count(&self) -> usize {
        self.adj.iter().map(Vec::len).sum::<usize>() / 2
    }

    pub fn id(&self, i: usize) -> NodeId {
        self.ids[i]
    }

    pub fn coord(&self, i: usize) -> Point {
        self.coords[i]
    }

    pub fn index_of(&self, id: NodeId) -> Option<usize> {
        self.index_of.get(&id).copied()
    }

    pub fn neighbors(&self, i: usize) -> &[usize] {
        &self.adj[i]
    }

    pub fn distance_from_node(&self, i: usize) -> Vec<(usize, f64)> {
        todo!()
    }

    pub fn radius_from(&self, i: usize) -> f64 {
        todo!()
    }
}

#[derive(Debug, PartialEq)]
pub enum GraphError {
    DuplicateNode(NodeId),
    UnknownNode(NodeId),
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GraphError::DuplicateNode(id) => write!(f, "node if {id} appears twice"),
            GraphError::UnknownNode(id) => {
                write!(f, "an edge uses node id {id}, which has no node")
            }
        }
    }
}

impl Error for GraphError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::{EdgeRecord, NodeRecord};

    const COORDS: [(NodeId, f64, f64); 6] = [
        (50, 0.0, 0.0),
        (2, 4.0, 0.0),
        (17, 0.0, 3.0),
        (8, 3.0, 4.0),
        (33, 20.0, 20.0),
        (7, 20.0, 20.0),
    ];

    const EDGES: [(NodeId, NodeId); 5] = [(50, 2), (50, 17), (17, 8), (2, 33), (33, 7)];

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-9,
            "expected {expected}, got {actual}"
        );
    }

    /// Creates a graph file
    fn file(nodes: &[(NodeId, f64, f64)], edges: &[(NodeId, NodeId)]) -> GraphFile {
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
    fn anchor_50(graph: &Graph) -> usize {
        graph.index_of(50).unwrap()
    }

    /// Creates a graph
    fn fixture() -> Graph {
        Graph::from_file(&file(&COORDS, &EDGES)).unwrap()
    }

    #[test]
    fn counts_nodes() {
        let graph = fixture();

        assert_eq!(graph.node_count(), 6);
    }

    #[test]
    fn nodes_keep_file_order() {
        let graph = fixture();

        // position i is the i-th node of the file
        assert_eq!(graph.id(0), 50);
        assert_eq!(graph.id(3), 8);
        assert_eq!(graph.coord(3), Point::new(3.0, 4.0));
    }

    #[test]
    fn index_of_is_the_inverse_of_id() {
        let graph = fixture();

        // every id maps back to its own position
        for i in 0..graph.node_count() {
            assert_eq!(graph.index_of(graph.id(i)), Some(i));
        }
        // an id that is not in the file has no position
        assert_eq!(graph.index_of(999), None);
    }

    #[test]
    fn rejects_repeated_node_id() {
        let nodes = [(1, 0.0, 0.0), (2, 1.0, 0.0), (1, 5.0, 5.0)];

        let result = Graph::from_file(&file(&nodes, &[]));

        // the error says which id was repeated
        assert_eq!(result.err(), Some(GraphError::DuplicateNode(1)));
    }

    #[test]
    fn counts_edges() {
        let graph = fixture();

        assert_eq!(graph.edge_count(), 5);
    }

    #[test]
    fn neighbors_are_symmetric_and_sorted() {
        let graph = fixture();

        // each edge shows in the lists of both ends, in position order
        assert_eq!(graph.neighbors(0), &[1, 2]);
        assert_eq!(graph.neighbors(1), &[0, 4]);
        assert_eq!(graph.neighbors(5), &[4]);
    }

    #[test]
    fn repeated_edges_count_once() {
        let nodes = [(1, 0.0, 0.0), (2, 1.0, 0.0), (3, 2.0, 0.0)];
        let edges = [(1, 2), (1, 3), (2, 1)];

        let graph = Graph::from_file(&file(&nodes, &edges)).unwrap();

        // (2, 1) is the first edge written backwards
        assert_eq!(graph.edge_count(), 2);
        assert_eq!(graph.neighbors(0), &[1, 2]);
    }

    #[test]
    fn self_loops_are_dropped() {
        let nodes = [(1, 0.0, 0.0), (2, 1.0, 0.0)];
        let edges = [(1, 1), (1, 2)];

        let graph = Graph::from_file(&file(&nodes, &edges)).unwrap();

        // only the edge between two different nodes stays
        assert_eq!(graph.edge_count(), 1);
        assert_eq!(graph.neighbors(0), &[1]);
    }

    #[test]
    fn rejects_edge_to_missing_node() {
        let nodes = [(1, 0.0, 0.0), (2, 1.0, 0.0)];
        let edges = [(1, 2), (2, 3)];

        let result = Graph::from_file(&file(&nodes, &edges));

        // the error says which id has no node
        assert_eq!(result.err(), Some(GraphError::UnknownNode(3)));
    }

    #[test]
    fn distance_excludes_anchor() {
        let graph = fixture();
        let anchor = anchor_50(&graph);

        let result = graph.distance_from_node(anchor);

        // the anchor is not among its own distances
        assert!(result.iter().all(|&(i, _)| i != anchor));
    }

    #[test]
    fn distance_covers_all_other_nodes() {
        let graph = fixture();
        let anchor = anchor_50(&graph);

        let result = graph.distance_from_node(anchor);

        // every other node appears exactly once
        let mut ids: Vec<NodeId> = result.iter().map(|&(i, _)| graph.id(i)).collect();
        ids.sort();
        assert_eq!(ids, vec![2, 7, 8, 17, 33]);
    }

    #[test]
    fn distances_match_coordinates() {
        let graph = fixture();
        let anchor = anchor_50(&graph);

        let result = graph.distance_from_node(anchor);

        // each distance matches its node (anchor 50 is at the origin)
        for (i, dist) in result {
            let (_, x, y) = COORDS[i];
            assert_close(dist, x.hypot(y));
        }
    }
}
