//! Finding the best paths in the target for every edge in the query.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use ordered_float::OrderedFloat;

use crate::geometry::{Transform, along_line, dist_to_line, signed_dist_to_line};
use crate::graph::Graph;

/// The routes of all the query edges through the crop.
#[derive(Debug, Clone, PartialEq)]
pub struct EdgeRoutes {
    pub cost: f64,
    pub paths: Vec<Vec<usize>>,
}

/// Calculates the area between a line segment and a line.
///
/// d1 and d2 are the signed distances from the segment endpoints to the line. This helps calculate area
/// when it crosses the line. len is the length of the segment along the line.
fn step_area(d1: f64, d2: f64, len: f64) -> f64 {
    if d1 * d2 < 0.0 {
        len * (d1 * d1 + d2 * d2) / (2.0 * (d1.abs() + d2.abs()))
    } else {
        len * (d1.abs() + d2.abs()) / 2.0
    }
}

/// Finds the minimum-deviation path in `g` from `source` to `target`.
///
/// - Ignores nodes that are mode than `tol` from the line ab
/// - Edges are weighted by the area between them and the line ab
/// - Uses Dijkstra's under the hood
fn best_path(
    g: &Graph,
    a: crate::geometry::Point,
    b: crate::geometry::Point,
    source: usize,
    target: usize,
    tol: f64,
) -> Option<(f64, Vec<usize>)> {
    let in_corridor = |i: usize| dist_to_line(g.coord(i), a, b) <= tol;
    if !in_corridor(source) || !in_corridor(target) {
        return None;
    }

    let n = g.node_count();
    let mut best = vec![f64::INFINITY; n];
    let mut prev = vec![usize::MAX; n];
    let mut heap = BinaryHeap::new();
    best[source] = 0.0;
    heap.push(Reverse((OrderedFloat(0.0), source)));

    while let Some(Reverse((OrderedFloat(cost), u))) = heap.pop() {
        if u == target {
            let mut path = vec![target];
            while *path.last().unwrap() != source {
                path.push(prev[*path.last().unwrap()]);
            }
            path.reverse();
            return Some((cost, path));
        }
        if cost > best[u] {
            continue; // an older, worse entry
        }

        let pu = g.coord(u);
        let (du, tu) = (signed_dist_to_line(pu, a, b), along_line(pu, a, b));
        for &v in g.neighbors(u) {
            let pv = g.coord(v);
            let dv = signed_dist_to_line(pv, a, b);
            if dv.abs() > tol {
                continue;
            }
            let len = (along_line(pv, a, b) - tu).abs();
            let next = cost + step_area(du, dv, len);
            if next < best[v] {
                best[v] = next;
                prev[v] = u;
                heap.push(Reverse((OrderedFloat(next), v)));
            }
        }
    }
    None
}

/// Finds the best path in `g` for every edge of `q`.
///
/// Returns the total cost and the paths.
pub fn find_paths(
    q: &Graph,
    g: &Graph,
    mapping: &[usize],
    transform: &Transform,
    tol: f64,
) -> Option<EdgeRoutes> {
    let mut cost = 0.0;
    let mut paths = Vec::new();
    for (u, v) in q.edges() {
        let a = transform.apply(q.coord(u));
        let b = transform.apply(q.coord(v));
        let (c, path) = best_path(g, a, b, mapping[u], mapping[v], tol)?;
        cost += c;
        paths.push(path);
    }
    Some(EdgeRoutes { cost, paths })
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
        find_paths(q, g, &mapping, &Transform::IDENTITY, tol)
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
    fn detour_cost_follows_the_trapezoid_rule() {
        // the only route bows out through 13 at (2, 2)
        let g = graph(
            &[(10, 0.0, 0.0), (13, 2.0, 2.0), (12, 4.0, 0.0)],
            &[(10, 13), (13, 12)],
        );

        let routes = route(&q_edge(), &g, &SOURCE_TARGET, 10.0).unwrap();

        // two steps, each 0.5 * (0 + 2) * 2 (the length along the line is 2, not 2 * sqrt(2))
        assert_close(routes.cost, 4.0);
        assert_eq!(ids(&g, &routes.paths[0]), vec![10, 13, 12]);
    }

    #[test]
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

        // 4 for the first edge, and 2 for the second
        assert_close(routes.cost, 6.0);
    }

    #[test]
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

    #[test]
    fn a_step_across_the_line_is_two_triangles() {
        // from (0, 2) to (4, -2): two triangles, each 0.5 * 2 * 2
        let g = graph(&[(10, 0.0, 2.0), (12, 4.0, -2.0)], &[(10, 12)]);

        let routes = route(&q_edge(), &g, &SOURCE_TARGET, 10.0).unwrap();

        assert_close(routes.cost, 4.0);
    }

    #[test]
    fn a_crossing_step_with_unequal_ends() {
        // from (0, 3) to (4, -1): it crosses at x = 3. Triangles: 0.5 * 3 * 3 and 0.5 * 1 * 1
        let g = graph(&[(10, 0.0, 3.0), (12, 4.0, -1.0)], &[(10, 12)]);

        let routes = route(&q_edge(), &g, &SOURCE_TARGET, 10.0).unwrap();

        assert_close(routes.cost, 5.0);
    }

    #[test]
    fn a_step_on_one_side_is_a_trapezoid() {
        // from (0, 1) to (4, 3): 0.5 * (1 + 3) * 4
        let g = graph(&[(10, 0.0, 1.0), (12, 4.0, 3.0)], &[(10, 12)]);

        let routes = route(&q_edge(), &g, &SOURCE_TARGET, 10.0).unwrap();

        assert_close(routes.cost, 8.0);
    }

    #[test]
    fn a_step_across_the_line_costs_less_than_the_trapezoid_of_the_distances() {
        // a naive 0.5 * (2 + 2) * 4 = 8 would be wrong; the true area is 4
        let g = graph(&[(10, 0.0, 2.0), (12, 4.0, -2.0)], &[(10, 12)]);

        let routes = route(&q_edge(), &g, &SOURCE_TARGET, 10.0).unwrap();

        assert!(routes.cost < 8.0 - 1e-9);
    }

    #[test]
    fn a_step_across_the_line_gives_the_same_cost_for_a_rotated_query() {
        // the query edge points along +y; G is the same picture turned by 90 degrees
        let q = graph(&[(0, 0.0, 0.0), (1, 0.0, 4.0)], &[(0, 1)]);
        let g = graph(&[(10, 2.0, 0.0), (12, -2.0, 4.0)], &[(10, 12)]);

        let routes = route(&q, &g, &SOURCE_TARGET, 10.0).unwrap();

        assert_close(routes.cost, 4.0);
    }

    #[test]
    fn a_step_perpendicular_to_the_line_is_free() {
        // (0, 0) -> (0, 2) has no length along the line, so no area
        let g = graph(
            &[(10, 0.0, 0.0), (13, 0.0, 2.0), (12, 4.0, 0.0)],
            &[(10, 13), (13, 12), (10, 12)],
        );

        let routes = route(&q_edge(), &g, &SOURCE_TARGET, 10.0).unwrap();

        // both routes are valid; the direct edge has area 0 and so has the first step of the other
        assert_close(routes.cost, 0.0);
    }

    #[test]
    fn a_source_outside_the_corridor_gives_none() {
        let g = graph(&[(10, 0.0, 5.0), (12, 4.0, 0.0)], &[(10, 12)]);

        assert_eq!(route(&q_edge(), &g, &SOURCE_TARGET, 3.0), None);
    }

    #[test]
    fn the_transform_moves_the_line() {
        // shift Q up by 5, so the line is y = 5
        let g = graph(&[(10, 0.0, 5.0), (12, 4.0, 5.0)], &[(10, 12)]);
        let mapping = [g.index_of(10).unwrap(), g.index_of(12).unwrap()];
        let shift = Transform {
            t: crate::geometry::Point::new(0.0, 5.0),
            ..Transform::IDENTITY
        };

        let routes = find_paths(&q_edge(), &g, &mapping, &shift, 0.1).unwrap();

        assert_close(routes.cost, 0.0);
    }
}
