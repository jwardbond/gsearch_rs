//! Aligns and scores one mapping (`align_and_score` in Python).

use std::collections::BTreeSet;

use serde::Serialize;

use crate::graph::Graph;
use crate::io::NodeId;
use crate::path_scoring::find_paths;
use crate::procrustes::{Transform, fit, node_score};

/// A mapping with its transform, its score and the part of the crop that realises it.
///
/// It holds ids, not positions: a position only has a meaning inside one crop, and the crop
/// is dropped before the search ends.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Alignment {
    /// Lower is better.
    pub score: f64,
    pub transform: Transform,
    /// `(id in Q, id in G)` for every node of Q, in the order of the positions of Q.
    pub mapping: Vec<(NodeId, NodeId)>,
    /// The nodes of G in the mapping or on a path, in increasing id order.
    pub nodes: Vec<NodeId>,
    /// The edges of G on the paths, each as `(smaller id, larger id)`, in increasing order.
    pub edges: Vec<(NodeId, NodeId)>,
}

/// The score of an alignment. As in the Python code, only the edge score has a weight.
fn combine(node_score: f64, edge_score: f64, weight: f64) -> f64 {
    node_score + weight * edge_score
}

/// The crop positions of the mapped nodes and of all the path nodes, and the path edges.
/// Nothing is in the lists twice. Both are in increasing order of position.
fn assemble(mapping: &[usize], paths: &[Vec<usize>]) -> (Vec<usize>, Vec<(usize, usize)>) {
    let mut nodes: BTreeSet<usize> = mapping.iter().copied().collect();
    let mut edges = BTreeSet::new();

    for path in paths {
        nodes.extend(path);
        for step in path.windows(2) {
            edges.insert((step[0].min(step[1]), step[0].max(step[1])));
        }
    }
    (nodes.into_iter().collect(), edges.into_iter().collect())
}

/// Aligns `q` onto the crop with `mapping`, routes each edge of `q` through the crop, and scores
/// the result. `None` if some edge has no path.
///
/// `score = node_score + weight * edge_score`.
pub fn align_and_score(
    q: &Graph,
    crop: &Graph,
    mapping: &[usize],
    tol: f64,
    weight: f64,
) -> Option<Alignment> {
    let targets: Vec<_> = mapping.iter().map(|&j| crop.coord(j)).collect();

    let transform = fit(q.coords(), &targets);
    let moved = transform.apply_all(q.coords());
    let node_cost = node_score(&moved, &targets);

    let routes = find_paths(q, crop, mapping, &moved, tol)?;

    let (nodes, edges) = assemble(mapping, &routes.paths);

    let mut nodes: Vec<NodeId> = nodes.into_iter().map(|j| crop.id(j)).collect();
    nodes.sort_unstable();
    let mut edges: Vec<(NodeId, NodeId)> = edges
        .into_iter()
        .map(|(a, b)| (crop.id(a).min(crop.id(b)), crop.id(a).max(crop.id(b))))
        .collect();
    edges.sort_unstable();

    Some(Alignment {
        score: combine(node_cost, routes.cost, weight),
        transform,
        mapping: mapping
            .iter()
            .enumerate()
            .map(|(i, &j)| (q.id(i), crop.id(j)))
            .collect(),
        nodes,
        edges,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures::{assert_close, file};

    fn graph(nodes: &[(NodeId, f64, f64)], edges: &[(NodeId, NodeId)]) -> Graph {
        Graph::from_file(&file(nodes, edges)).unwrap()
    }

    #[test]
    fn combine_weights_only_the_edge_score() {
        assert_close(combine(2.0, 4.0, 0.5), 4.0);
        assert_close(combine(2.0, 4.0, 0.0), 2.0);
        assert_close(combine(2.0, 4.0, 1.0), 6.0);
    }

    #[test]
    fn assemble_keeps_the_mapped_nodes_when_there_are_no_paths() {
        let (nodes, edges) = assemble(&[7, 3, 5], &[]);

        // sorted, and no edges
        assert_eq!(nodes, vec![3, 5, 7]);
        assert!(edges.is_empty());
    }

    #[test]
    fn assemble_adds_path_nodes_once() {
        // the two paths share node 4
        let paths = vec![vec![3, 4, 5], vec![5, 4, 7]];

        let (nodes, _) = assemble(&[3, 5, 7], &paths);

        assert_eq!(nodes, vec![3, 4, 5, 7]);
    }

    #[test]
    fn assemble_lists_each_edge_once_smaller_end_first() {
        // the step 4-5 is used in both directions
        let paths = vec![vec![3, 4, 5], vec![5, 4, 7]];

        let (_, edges) = assemble(&[3, 5, 7], &paths);

        assert_eq!(edges, vec![(3, 4), (4, 5), (4, 7)]);
    }

    /// Q and a copy of Q in the same frame, with other ids. Q has the edges 1-2 and 2-3.
    fn q_and_copy() -> (Graph, Graph) {
        let q = graph(
            &[(1, 0.0, 0.0), (2, 3.0, 0.0), (3, 0.0, 4.0)],
            &[(1, 2), (2, 3)],
        );
        let g = graph(
            &[(11, 0.0, 0.0), (12, 3.0, 0.0), (13, 0.0, 4.0)],
            &[(11, 12), (12, 13)],
        );
        (q, g)
    }

    #[test]
    fn a_copy_in_the_same_frame_scores_zero() {
        let (q, g) = q_and_copy();

        let alignment = align_and_score(&q, &g, &[0, 1, 2], 1.0, 0.5).unwrap();

        // nothing has to move, and there is no area
        assert_close(alignment.score, 0.0);
    }

    #[test]
    fn the_mapping_holds_ids_in_the_order_of_q() {
        let (q, g) = q_and_copy();

        let alignment = align_and_score(&q, &g, &[0, 1, 2], 1.0, 0.5).unwrap();

        assert_eq!(alignment.mapping, vec![(1, 11), (2, 12), (3, 13)]);
    }

    #[test]
    fn a_mapping_to_other_positions_is_reported_by_id() {
        let (q, g) = q_and_copy();

        // a different mapping: Q's nodes 1 and 2 go to G's 12 and 11
        let alignment = align_and_score(&q, &g, &[1, 0, 2], 100.0, 0.5).unwrap();

        assert_eq!(alignment.mapping, vec![(1, 12), (2, 11), (3, 13)]);
        assert_eq!(alignment.nodes, vec![11, 12, 13]);
    }

    #[test]
    fn nodes_are_sorted_by_id_not_by_position() {
        let q = graph(&[(1, 0.0, 0.0), (2, 3.0, 0.0)], &[(1, 2)]);
        // the ids fall while the positions rise
        let g = graph(
            &[(13, 0.0, 0.0), (12, 3.0, 0.0), (11, 9.0, 9.0)],
            &[(13, 12)],
        );

        let alignment = align_and_score(&q, &g, &[0, 1], 1.0, 0.5).unwrap();

        assert_eq!(alignment.nodes, vec![12, 13]);
    }

    // The tests below need your `fit` and your `find_paths`.
    // Run them with `cargo test alignment -- --ignored`.

    #[test]
    #[ignore = "fit and find_paths are placeholders"]
    fn a_moved_copy_scores_zero() {
        let q = graph(
            &[(1, 0.0, 0.0), (2, 3.0, 0.0), (3, 0.0, 4.0)],
            &[(1, 2), (2, 3)],
        );
        // the same shape, shifted by (100, 250)
        let g = graph(
            &[(11, 100.0, 250.0), (12, 103.0, 250.0), (13, 100.0, 254.0)],
            &[(11, 12), (12, 13)],
        );

        let alignment = align_and_score(&q, &g, &[0, 1, 2], 1.0, 0.5).unwrap();

        assert_close(alignment.score, 0.0);
    }

    #[test]
    #[ignore = "fit and find_paths are placeholders"]
    fn the_edges_are_the_g_edges_on_the_paths() {
        let (q, g) = q_and_copy();

        let alignment = align_and_score(&q, &g, &[0, 1, 2], 1.0, 0.5).unwrap();

        assert_eq!(alignment.edges, vec![(11, 12), (12, 13)]);
    }

    #[test]
    #[ignore = "fit and find_paths are placeholders"]
    fn a_query_edge_without_a_path_gives_none() {
        let q = graph(&[(1, 0.0, 0.0), (2, 4.0, 0.0)], &[(1, 2)]);
        // the two mapped nodes are not connected
        let g = graph(&[(11, 0.0, 0.0), (12, 4.0, 0.0)], &[]);

        let alignment = align_and_score(&q, &g, &[0, 1], 1.0, 0.5);

        assert_eq!(alignment, None);
    }
}
