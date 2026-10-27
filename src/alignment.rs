//! Aligns and scores one mapping

use std::collections::BTreeSet;

use serde::Serialize;

use crate::geometry::{Point, Transform};
use crate::graph::Graph;
use crate::io::NodeId;
use crate::path_scoring::find_paths;
use crate::procrustes::{fit, node_score};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Alignment {
    pub score: f64,
    pub transform: Transform,
    pub mapping: Vec<(NodeId, NodeId)>,
    pub matched: Graph,
}

/// Aligns `q` onto the crop with `mapping`, routes each edge of `q` through the crop, and scores
/// the result.
///
/// `None` if some edge has no path.
///
/// `score = node_score + weight * edge_score`.
pub fn align(
    q: &Graph,
    crop: &Graph,
    mapping: &[usize],
    tol: f64,
    weight: f64,
) -> Option<Alignment> {
    // Align Q with G using Procrustes.
    let targets: Vec<Point> = mapping.iter().map(|&j| crop.coord(j)).collect();
    let transform = fit(q.coords(), &targets);
    let moved = transform.apply_all(q.coords());
    let node_cost = node_score(&moved, &targets);

    // Find the paths in G most aligned with each edge in Q
    let routes = find_paths(q, crop, mapping, &transform, tol)?;

    // Construct the output.
    let matched = crop.subgraph(&path_edges(&routes.paths));
    Some(Alignment {
        score: score(node_cost, routes.cost, weight),
        transform,
        mapping: mapping
            .iter()
            .enumerate()
            .map(|(i, &j)| (q.id(i), crop.id(j)))
            .collect(),
        matched,
    })
}

fn score(node_score: f64, edge_score: f64, weight: f64) -> f64 {
    node_score + weight * edge_score
}

/// The crop edges on the paths, each as `(smaller position, larger position)`. Nothing is in the
/// list twice. It is in increasing order.
fn path_edges(paths: &[Vec<usize>]) -> Vec<(usize, usize)> {
    let mut edges = BTreeSet::new();

    for path in paths {
        for step in path.windows(2) {
            edges.insert((step[0].min(step[1]), step[0].max(step[1])));
        }
    }
    edges.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures::{assert_close, file};

    fn graph(nodes: &[(NodeId, f64, f64)], edges: &[(NodeId, NodeId)]) -> Graph {
        Graph::from_file(&file(nodes, edges)).unwrap()
    }

    fn node_ids(g: &Graph) -> Vec<NodeId> {
        (0..g.node_count()).map(|i| g.id(i)).collect()
    }

    fn edge_ids(g: &Graph) -> Vec<(NodeId, NodeId)> {
        g.edges()
            .into_iter()
            .map(|(i, j)| (g.id(i), g.id(j)))
            .collect()
    }

    #[test]
    fn score_weights_only_the_edge_score() {
        assert_close(score(2.0, 4.0, 0.5), 4.0);
        assert_close(score(2.0, 4.0, 0.0), 2.0);
        assert_close(score(2.0, 4.0, 1.0), 6.0);
    }

    #[test]
    fn path_edges_is_empty_when_there_are_no_paths() {
        assert!(path_edges(&[]).is_empty());
    }

    #[test]
    fn path_edges_lists_each_edge_once_smaller_end_first() {
        // the step 4-5 is used in both directions
        let paths = vec![vec![3, 4, 5], vec![5, 4, 7]];

        let edges = path_edges(&paths);

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

        let alignment = align(&q, &g, &[0, 1, 2], 1.0, 0.5).unwrap();

        // nothing has to move, and there is no area
        assert_close(alignment.score, 0.0);
    }

    #[test]
    fn the_mapping_holds_ids_in_the_order_of_q() {
        let (q, g) = q_and_copy();

        let alignment = align(&q, &g, &[0, 1, 2], 1.0, 0.5).unwrap();

        assert_eq!(alignment.mapping, vec![(1, 11), (2, 12), (3, 13)]);
    }

    #[test]
    fn a_mapping_to_other_positions_is_reported_by_id() {
        let (q, g) = q_and_copy();

        // a different mapping: Q's nodes 1 and 2 go to G's 12 and 11
        let alignment = align(&q, &g, &[1, 0, 2], 100.0, 0.5).unwrap();

        assert_eq!(alignment.mapping, vec![(1, 12), (2, 11), (3, 13)]);
        assert_eq!(node_ids(&alignment.matched), vec![11, 12, 13]);
    }

    #[test]
    fn nodes_are_sorted_by_id_not_by_position() {
        let q = graph(&[(1, 0.0, 0.0), (2, 3.0, 0.0)], &[(1, 2)]);
        // the ids fall while the positions rise
        let g = graph(
            &[(13, 0.0, 0.0), (12, 3.0, 0.0), (11, 9.0, 9.0)],
            &[(13, 12)],
        );

        let alignment = align(&q, &g, &[0, 1], 1.0, 0.5).unwrap();

        assert_eq!(node_ids(&alignment.matched), vec![12, 13]);
    }

    // The tests below need your `fit` and your `find_paths`.
    // Run them with `cargo test alignment -- --ignored`.

    #[test]
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

        let alignment = align(&q, &g, &[0, 1, 2], 1.0, 0.5).unwrap();

        assert_close(alignment.score, 0.0);
    }

    #[test]
    fn the_edges_are_the_g_edges_on_the_paths() {
        let (q, g) = q_and_copy();

        let alignment = align(&q, &g, &[0, 1, 2], 1.0, 0.5).unwrap();

        assert_eq!(edge_ids(&alignment.matched), vec![(11, 12), (12, 13)]);
    }

    #[test]
    fn a_query_edge_without_a_path_gives_none() {
        let q = graph(&[(1, 0.0, 0.0), (2, 4.0, 0.0)], &[(1, 2)]);
        // the two mapped nodes are not connected
        let g = graph(&[(11, 0.0, 0.0), (12, 4.0, 0.0)], &[]);

        let alignment = align(&q, &g, &[0, 1], 1.0, 0.5);

        assert_eq!(alignment, None);
    }
}
