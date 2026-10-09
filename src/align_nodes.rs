use crate::geometry::Point;
use crate::graph::Graph;
use crate::io::NodeId;

/// One row per node of Q, one column per node of G.
pub type Matches = Vec<Vec<usize>>;

/// A candidate match: `pose[i]` is the position in G of the node matched to Q position `i`.
pub type Pose = Vec<usize>;

pub fn get_candidate_matches(
    q: &Graph,
    g: &Graph,
    q_anchor: usize,
    g_anchor: usize,
    tol: f64,
) -> Option<Vec<Pose>> {
    // Need at least 2 nodes for this to work
    if q.node_count() < 2 || g.node_count() < 2 {
        return None;
    }
    // Need at least enough nodes for 1-to-1 match
    if q.node_count() > g.node_count() {
        return None;
    }

    let q_coords = q.coords();
    let g_coords = g.coords();

    let matches = annulus_matches(q_coords, g_coords, q_anchor, g_anchor, tol)?;

    todo!()
}

/// True when two distances agree within `tol`. A difference of exactly `tol` counts.
fn within_tol(q_dist: f64, g_dist: f64, tol: f64) -> bool {
    (q_dist - g_dist).abs() <= tol
}

/// Finds potential matches (within tolerance) for q within g
///
/// `q-ref` - the index of the reference node in q
/// `g-ref` - the index of the reference node in g
///
/// Outputs a Matches vector with the same length as q. Each element is a vector of nodes indices in g
/// which are the same distance from g_ref as the corresponding point in q is from q_ref
fn annulus_matches(
    q: &[Point],
    g: &[Point],
    q_ref: usize,
    g_ref: usize,
    tol: f64,
) -> Option<Matches> {
    let mut matches = vec![Vec::new(); q.len()];
    let g_dists: Vec<f64> = g.iter().map(|&p| p.dist(g[g_ref])).collect();

    for (i, &p_q) in q.iter().enumerate() {
        let q_dist = p_q.dist(q[q_ref]);
        for (j, &g_dist) in g_dists.iter().enumerate() {
            if within_tol(q_dist, g_dist, tol) {
                matches[i].push(j);
            }
        }

        // If there are no matches for a given node
        if matches[i].is_empty() {
            return None;
        }
    }
    Some(matches)
}

fn pick_alignment_node(matches: &Matches, q_anchor: usize) -> usize {
    let candidates = &matches[q_anchor];
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures::file;

    // Q is near the origin and G is far away, as in the real data.
    //
    //   Q (rows)     distance from Q[0]      G (columns)    distance from G[0]
    //   (0, 0)        0                      (100, 100)       0
    //   (3, 0)        3                      (103, 100)       3   matches Q[1]
    //   (0, 4)        4                      (100, 104)       4   matches Q[2]
    //                                        (100, 102)       2
    //                                        (200, 100)     100   matches nothing
    const Q: [Point; 3] = [
        Point { x: 0.0, y: 0.0 },
        Point { x: 3.0, y: 0.0 },
        Point { x: 0.0, y: 4.0 },
    ];
    const G: [Point; 5] = [
        Point { x: 100.0, y: 100.0 },
        Point { x: 103.0, y: 100.0 },
        Point { x: 100.0, y: 104.0 },
        Point { x: 100.0, y: 102.0 },
        Point { x: 200.0, y: 100.0 },
    ];

    /// The annulus matches, found with two plain loops and `hypot`.
    fn brute_force(q: &[Point], g: &[Point], q_ref: usize, g_ref: usize, tol: f64) -> Matches {
        let mut matches = vec![Vec::new(); q.len()];
        for i in 0..q.len() {
            let d1 = (q[i].x - q[q_ref].x).hypot(q[i].y - q[q_ref].y);
            for j in 0..g.len() {
                let d2 = (g[j].x - g[g_ref].x).hypot(g[j].y - g[g_ref].y);
                if (d1 - d2).abs() <= tol {
                    matches[i].push(j);
                }
            }
        }
        matches
    }

    // A 3-4-5 triangle. Its three distances are all different, so it has only one
    // correct pose. The ids are not positions.
    const TRIANGLE: [(NodeId, f64, f64); 3] = [(10, 0.0, 0.0), (20, 3.0, 0.0), (30, 0.0, 4.0)];

    fn graph(nodes: &[(NodeId, f64, f64)]) -> Graph {
        Graph::from_file(&file(nodes, &[])).unwrap()
    }

    #[test]
    fn annulus_matches_has_a_row_per_q_node() {
        let matches = annulus_matches(&Q, &G, 0, 0, 0.5).unwrap();

        assert_eq!(matches.len(), 3);
        // every match is a position of G
        assert!(matches.iter().flatten().all(|&j| j < G.len()));
    }

    #[test]
    fn annulus_matches_agrees_with_brute_force() {
        let matches = annulus_matches(&Q, &G, 0, 0, 0.5).unwrap();

        assert_eq!(matches, brute_force(&Q, &G, 0, 0, 0.5));
    }

    #[test]
    fn annulus_matches_ignores_a_shift_of_q() {
        let shifted: Vec<Point> = Q.iter().map(|&p| p + Point::new(50.0, -20.0)).collect();

        let matches = annulus_matches(&shifted, &G, 0, 0, 0.5).unwrap();

        // only distances to the reference node count, not positions
        assert_eq!(matches, annulus_matches(&Q, &G, 0, 0, 0.5).unwrap());
    }

    #[test]
    fn annulus_matches_boundary_is_inclusive() {
        let matches = annulus_matches(&Q, &G, 0, 0, 1.0).unwrap();

        assert!(matches[1].contains(&3));
    }

    #[test]
    fn annulus_matches_with_tol_zero_needs_equal_distances() {
        let matches = annulus_matches(&Q, &G, 0, 0, 0.0).unwrap();

        assert_eq!(matches, brute_force(&Q, &G, 0, 0, 0.0));
        assert_eq!(matches, vec![vec![0], vec![1], vec![2]]);
    }

    #[test]
    fn annulus_matches_with_other_reference_nodes() {
        let matches = annulus_matches(&Q, &G, 1, 1, 0.0).unwrap();

        assert_eq!(matches, brute_force(&Q, &G, 1, 1, 0.0));
        // each reference node is at distance 0 from itself
        assert!(matches[1].contains(&1));
    }

    #[test]
    fn query_larger_than_target_has_no_pose() {
        let q = graph(&TRIANGLE);
        let g = graph(&[(101, 100.0, 100.0), (102, 103.0, 100.0)]);

        let poses = get_candidate_matches(&q, &g, 0, 0, 0.5);

        assert!(poses.is_none());
    }

    #[test]
    fn query_node_with_no_match_gives_no_pose() {
        // nothing in G is near distance 4 from the anchor
        let q = graph(&TRIANGLE);
        let g = graph(&[
            (101, 100.0, 100.0),
            (102, 103.0, 100.0),
            (103, 200.0, 100.0),
        ]);

        let poses = get_candidate_matches(&q, &g, 0, 0, 0.5);

        assert!(poses.is_none());
    }

    #[test]
    fn alignment_node_is_never_the_anchor() {
        // the anchor (row 1) has the fewest matches, but it cannot align with itself
        let matches = vec![vec![1, 2, 3], vec![0], vec![1, 2]];

        assert_eq!(pick_alignment_node(&matches, 1), 2);
    }
    #[test]
    fn single_node_query_has_no_pose() {
        let q = graph(&[(10, 0.0, 0.0)]);
        let g = graph(&[(101, 100.0, 100.0)]);

        let poses = get_candidate_matches(&q, &g, 0, 0, 0.5);

        assert!(poses.is_none());
    }
}
