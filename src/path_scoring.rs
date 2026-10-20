//! The best path in the crop for each edge of the query (`_find_paths` in Python).

use crate::geometry::Point;
use crate::graph::Graph;

/// True while `find_paths` is a placeholder. Set it to `false` when you write `find_paths`.
pub const IS_PLACEHOLDER: bool = true;

/// The routes of all the query edges through the crop.
#[derive(Debug, Clone, PartialEq)]
pub struct EdgeRoutes {
    /// The sum of the costs of the paths.
    pub cost: f64,
    /// `paths[e]` is the path for edge `e` of `q.edges()`: crop positions, from the node
    /// that is mapped to its first end to the node that is mapped to its second end.
    pub paths: Vec<Vec<usize>>,
}

/// PLACEHOLDER. Finds the best path in `g` for every edge of `q`.
///
/// - `mapping[i]` is the position in `g` of the node of `q` at position `i`.
/// - `moved_q[i]` is the moved position of the node of `q` at position `i`.
///
/// For the edge from `u` to `v`, the corridor is the set of nodes of `g` that are within `tol`
/// of the **line** through `moved_q[u]` and `moved_q[v]`. It is the line, not the segment, so a
/// node past the end of the segment is not penalised for that. A path may use only nodes of
/// the corridor. The cost of a path is the sum, over its steps, of the average of the two end
/// distances to the line times the length of the step (the trapezoid rule). The best path has
/// the lowest cost.
///
/// Returns `None` if any edge has no path.
pub fn find_paths(
    _q: &Graph,
    _g: &Graph,
    _mapping: &[usize],
    _moved_q: &[Point],
    _tol: f64,
) -> Option<EdgeRoutes> {
    Some(EdgeRoutes {
        cost: 0.0,
        paths: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::NodeId;
    use crate::test_fixtures::{assert_close, file};

    fn graph(nodes: &[(NodeId, f64, f64)], edges: &[(NodeId, NodeId)]) -> Graph {
        Graph::from_file(&file(nodes, edges)).unwrap()
    }

    /// Runs `find_paths` with Q and G in the same frame, so Q does not move.
    /// `ids[i]` is the id in `g` of the node at position `i` of `q`.
    fn route(q: &Graph, g: &Graph, ids: &[NodeId], tol: f64) -> Option<EdgeRoutes> {
        let mapping: Vec<usize> = ids.iter().map(|&id| g.index_of(id).unwrap()).collect();
        find_paths(q, g, &mapping, q.coords(), tol)
    }

    /// The ids along a path.
    fn ids(g: &Graph, path: &[usize]) -> Vec<NodeId> {
        path.iter().map(|&i| g.id(i)).collect()
    }

    /// One query edge along the x axis, from (0, 0) to (4, 0).
    fn q_edge() -> Graph {
        graph(&[(0, 0.0, 0.0), (1, 4.0, 0.0)], &[(0, 1)])
    }

    const SOURCE_TARGET: [NodeId; 2] = [10, 12];

    #[test]
    fn placeholder_gives_cost_zero_and_no_paths() {
        let g = graph(&[(10, 0.0, 0.0), (12, 4.0, 0.0)], &[(10, 12)]);

        let routes = route(&q_edge(), &g, &SOURCE_TARGET, 10.0).unwrap();

        assert_eq!(routes.cost, 0.0);
        assert!(routes.paths.is_empty());
    }

    // The tests below are the target for your `find_paths`. They are ignored until you write it.
    // Run them with `cargo test path_scoring -- --ignored`.

    #[test]
    #[ignore = "find_paths is a placeholder"]
    fn has_one_path_per_query_edge() {
        // a query path 0-1-2 along a straight chain in G
        let q = graph(
            &[(0, 0.0, 0.0), (1, 4.0, 0.0), (2, 8.0, 0.0)],
            &[(0, 1), (1, 2)],
        );
        let g = graph(
            &[
                (10, 0.0, 0.0),
                (11, 2.0, 0.0),
                (12, 4.0, 0.0),
                (13, 6.0, 0.0),
                (14, 8.0, 0.0),
            ],
            &[(10, 11), (11, 12), (12, 13), (13, 14)],
        );

        let routes = route(&q, &g, &[10, 12, 14], 10.0).unwrap();

        // `paths[e]` follows the order of `q.edges()`
        assert_eq!(routes.paths.len(), 2);
        assert_eq!(ids(&g, &routes.paths[0]), vec![10, 11, 12]);
        assert_eq!(ids(&g, &routes.paths[1]), vec![12, 13, 14]);
    }

    #[test]
    #[ignore = "find_paths is a placeholder"]
    fn perfect_alignment_costs_zero() {
        let g = graph(
            &[(10, 0.0, 0.0), (11, 2.0, 0.0), (12, 4.0, 0.0)],
            &[(10, 11), (11, 12)],
        );

        let routes = route(&q_edge(), &g, &SOURCE_TARGET, 10.0).unwrap();

        // every node of the path is on the line, so the area is zero
        assert_close(routes.cost, 0.0);
        assert_eq!(ids(&g, &routes.paths[0]), vec![10, 11, 12]);
    }

    #[test]
    #[ignore = "find_paths is a placeholder"]
    fn picks_the_path_with_the_lowest_area() {
        // the straight route 10-11-12 and a detour through 13 at (2, 3)
        let g = graph(
            &[
                (10, 0.0, 0.0),
                (11, 2.0, 0.0),
                (12, 4.0, 0.0),
                (13, 2.0, 3.0),
            ],
            &[(10, 11), (11, 12), (10, 13), (13, 12)],
        );

        let routes = route(&q_edge(), &g, &SOURCE_TARGET, 10.0).unwrap();

        // the straight route has zero area and wins
        assert_eq!(ids(&g, &routes.paths[0]), vec![10, 11, 12]);
    }

    #[test]
    #[ignore = "find_paths is a placeholder"]
    fn detour_cost_follows_the_trapezoid_rule() {
        // the only route bows out through 13 at (2, 2)
        let g = graph(
            &[(10, 0.0, 0.0), (13, 2.0, 2.0), (12, 4.0, 0.0)],
            &[(10, 13), (13, 12)],
        );

        let routes = route(&q_edge(), &g, &SOURCE_TARGET, 10.0).unwrap();

        // two trapezoids, each 0.5 * (0 + 2) * (2 * sqrt(2))
        assert_close(routes.cost, 4.0 * 2.0_f64.sqrt());
        assert_eq!(ids(&g, &routes.paths[0]), vec![10, 13, 12]);
    }

    #[test]
    #[ignore = "find_paths is a placeholder"]
    fn cost_is_the_sum_over_the_edges() {
        let q = graph(
            &[(0, 0.0, 0.0), (1, 4.0, 0.0), (2, 8.0, 0.0)],
            &[(0, 1), (1, 2)],
        );
        // each query edge has its own detour
        let g = graph(
            &[
                (10, 0.0, 0.0),
                (13, 2.0, 2.0),
                (12, 4.0, 0.0),
                (15, 6.0, 1.0),
                (14, 8.0, 0.0),
            ],
            &[(10, 13), (13, 12), (12, 15), (15, 14)],
        );

        let routes = route(&q, &g, &[10, 12, 14], 10.0).unwrap();

        // 4 * sqrt(2) for the first edge, and sqrt(5) for the second
        assert_close(routes.cost, 4.0 * 2.0_f64.sqrt() + 5.0_f64.sqrt());
    }

    #[test]
    #[ignore = "find_paths is a placeholder"]
    fn no_path_gives_none() {
        // 10 and 12 are in different components
        let g = graph(
            &[(10, 0.0, 0.0), (12, 4.0, 0.0), (14, 1.0, 0.0)],
            &[(10, 14)],
        );

        let routes = route(&q_edge(), &g, &SOURCE_TARGET, 10.0);

        assert_eq!(routes, None);
    }

    #[test]
    #[ignore = "find_paths is a placeholder"]
    fn a_path_outside_the_corridor_gives_none() {
        // the only route goes through 13 at (2, 20), far from the line
        let g = graph(
            &[(10, 0.0, 0.0), (13, 2.0, 20.0), (12, 4.0, 0.0)],
            &[(10, 13), (13, 12)],
        );

        let routes = route(&q_edge(), &g, &SOURCE_TARGET, 10.0);

        assert_eq!(routes, None);
    }

    #[test]
    #[ignore = "find_paths is a placeholder"]
    fn a_node_exactly_at_tol_is_in_the_corridor() {
        let g = graph(
            &[(10, 0.0, 0.0), (13, 2.0, 3.0), (12, 4.0, 0.0)],
            &[(10, 13), (13, 12)],
        );

        let routes = route(&q_edge(), &g, &SOURCE_TARGET, 3.0).unwrap();

        // a distance equal to tol passes: only a larger distance is out
        assert_eq!(ids(&g, &routes.paths[0]), vec![10, 13, 12]);
    }

    #[test]
    #[ignore = "find_paths is a placeholder"]
    fn a_node_past_the_end_is_free_on_a_line() {
        // 13 at (6, 0) is 2 from the segment, but it is on the line through the segment
        let g = graph(
            &[(10, 0.0, 0.0), (13, 6.0, 0.0), (12, 4.0, 0.0)],
            &[(10, 13), (13, 12)],
        );

        let routes = route(&q_edge(), &g, &SOURCE_TARGET, 1.0).unwrap();

        // with the segment, 13 is out of the corridor (2 > 1) and the answer would be `None`
        assert_close(routes.cost, 0.0);
        assert_eq!(ids(&g, &routes.paths[0]), vec![10, 13, 12]);
    }
}
