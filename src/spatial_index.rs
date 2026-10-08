use kiddo::ImmutableKdTree;
use kiddo::SquaredEuclidean;

use crate::geometry::Point;
use crate::graph::Graph;

/// A spatial index for a graph
pub struct SpatialIndex {
    graph: Graph,
    kd_tree: ImmutableKdTree<f64, 2>,
}

impl SpatialIndex {
    pub fn new(graph: Graph) -> Self {
        let points: Vec<[f64; 2]> = (0..graph.node_count())
            .map(|i| {
                let p = graph.coord(i);
                [p.x, p.y]
            })
            .collect();

        let kd_tree =
            ImmutableKdTree::new_from_slice(&points).expect("Input graph had more than 2^32 nodes");

        SpatialIndex { graph, kd_tree }
    }

    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    /// Gets a list of nodes within a radius of a point
    fn query_radius(&self, point: Point, radius: f64) -> Vec<(usize, f64)> {
        let hits = self
            .kd_tree
            .query(&[point.x, point.y])
            .within::<SquaredEuclidean<f64>>(radius.powi(2))
            .execute();

        hits.iter()
            .map(|hit| (hit.item as usize, hit.distance.sqrt()))
            .collect()
    }

    /// Gets the subgraph made up of nodes within a radius of a point
    pub fn make_crop(&self, anchor: usize, radius: f64) -> Graph {
        let hits = self.query_radius(self.graph.coord(anchor), radius);
        let keep: Vec<usize> = hits.iter().map(|&(i, _)| i).collect();
        let circle = self.graph.subgraph(&keep);

        // translate anchor index from graph -> circle
        let anchor = circle.index_of(self.graph.id(anchor)).unwrap();

        circle.cc_containing(anchor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::NodeId;
    use crate::test_fixtures::{COORDS, assert_close, file, fixture};

    fn index() -> SpatialIndex {
        SpatialIndex::new(fixture())
    }

    /// The file ids of the positions in a query result.
    fn ids_of(index: &SpatialIndex, hits: &[(usize, f64)]) -> Vec<NodeId> {
        hits.iter().map(|&(i, _)| index.graph().id(i)).collect()
    }

    /// The ids within `radius` of `point`, found by checking every node.
    fn brute_force_within(point: Point, radius: f64) -> Vec<NodeId> {
        let mut ids: Vec<NodeId> = COORDS
            .iter()
            .filter(|&&(_, x, y)| Point::new(x, y).dist(point) <= radius)
            .map(|&(id, _, _)| id)
            .collect();
        ids.sort();
        ids
    }

    /// The edges of a graph as sorted pairs of file ids.
    fn edge_ids(graph: &Graph) -> Vec<(NodeId, NodeId)> {
        let mut edges = Vec::new();
        for i in 0..graph.node_count() {
            for &j in graph.neighbors(i) {
                if i < j {
                    let (a, b) = (graph.id(i), graph.id(j));
                    edges.push((a.min(b), a.max(b)));
                }
            }
        }
        edges.sort();
        edges
    }

    fn crop_ids(crop: &Graph) -> Vec<NodeId> {
        (0..crop.node_count()).map(|i| crop.id(i)).collect()
    }

    #[test]
    fn query_nodes_finds_themselves() {
        let index = index();
        let graph = index.graph();

        for i in 0..graph.node_count() {
            let hits = index.query_radius(graph.coord(i), 0.0);

            // tree item i is graph position i
            assert!(hits.iter().any(|&(j, _)| j == i));
        }
    }

    #[test]
    fn query_radius_distances_match_coordinates() {
        let index = index();
        let point = Point::new(1.0, 1.0);

        let hits = index.query_radius(point, 25.0);

        // distances are correct, and paired with correct nodes
        assert!(!hits.is_empty());
        for (i, dist) in hits {
            assert_close(dist, index.graph().coord(i).dist(point));
        }
    }

    #[test]
    fn query_radius_sorts_by_distance() {
        let index = index();

        let hits = index.query_radius(Point::new(0.0, 0.0), 5.0);

        // nearest first, not in position order
        assert_eq!(ids_of(&index, &hits), vec![50, 17, 2, 8]);
        let dists: Vec<f64> = hits.iter().map(|&(_, d)| d).collect();
        assert_eq!(dists, vec![0.0, 3.0, 4.0, 5.0]);
    }

    #[test]
    fn query_radius_returns_exactly_nodes_within_radius() {
        let index = index();
        let point = Point::new(1.0, 2.0);

        let hits = index.query_radius(point, 6.0);

        // the same ids as a check of every node, and no repeats
        let mut ids = ids_of(&index, &hits);
        ids.sort();
        assert_eq!(ids, brute_force_within(point, 6.0));
    }

    #[test]
    fn query_radius_boundary_is_inclusive() {
        let index = index();

        let hits = index.query_radius(Point::new(0.0, 0.0), 5.0);

        // node 8 is at exactly 5.0, and it is in the result
        let eight = index.graph().index_of(8).unwrap();
        let dist = hits.iter().find(|&&(i, _)| i == eight).unwrap().1;
        assert_close(dist, 5.0);
    }

    #[test]
    fn query_radius_at_node_coords_includes_that_node() {
        let index = index();

        let hits = index.query_radius(Point::new(0.0, 3.0), 1.0);

        // the node under the point comes first, at distance zero
        assert_eq!(ids_of(&index, &hits)[0], 17);
        assert_close(hits[0].1, 0.0);
    }

    #[test]
    fn query_radius_returns_coincident_nodes() {
        let index = index();

        let hits = index.query_radius(Point::new(20.0, 20.0), 0.5);

        // both nodes at (20, 20), in file order
        assert_eq!(ids_of(&index, &hits), vec![33, 7]);
    }

    #[test]
    fn query_radius_with_no_hits_is_empty() {
        let index = index();

        let hits = index.query_radius(Point::new(1000.0, 1000.0), 1.0);

        assert!(hits.is_empty());
    }

    #[test]
    fn crop_has_the_nodes_within_the_radius() {
        let index = index();

        let crop = index.make_crop(0, 5.0);

        // the same ids as a check of every node, anchor included
        let mut ids = crop_ids(&crop);
        ids.sort();
        assert_eq!(ids, brute_force_within(Point::new(0.0, 0.0), 5.0));
    }

    #[test]
    fn crop_starts_with_the_anchor() {
        let index = index();
        let graph = index.graph();

        for anchor in 0..graph.node_count() {
            let crop = index.make_crop(anchor, 5.0);

            // the anchor is position 0 of every crop
            assert_eq!(crop.id(0), graph.id(anchor));
        }
    }

    #[test]
    fn crop_with_radius_zero_is_the_anchor() {
        let index = index();

        let crop = index.make_crop(0, 0.0);

        // only node 50 is at distance zero from itself
        assert_eq!(crop_ids(&crop), vec![50]);
        assert_eq!(crop.edge_count(), 0);
    }

    #[test]
    fn crop_with_radius_zero_keeps_coincident_nodes() {
        let index = index();
        let anchor = index.graph().index_of(33).unwrap();

        let crop = index.make_crop(anchor, 0.0);

        // node 7 is at the same point as node 33, and they share an edge
        assert_eq!(crop_ids(&crop), vec![33, 7]);
        assert_eq!(edge_ids(&crop), vec![(7, 33)]);
    }

    #[test]
    fn crop_keeps_only_the_anchor_component() {
        // node 100 is within the radius, but no edge connects it to the anchor
        let nodes = [(1, 0.0, 0.0), (2, 1.0, 0.0), (100, 2.0, 0.0)];
        let graph = Graph::from_file(&file(&nodes, &[(1, 2)])).unwrap();
        let index = SpatialIndex::new(graph);

        let crop = index.make_crop(0, 5.0);

        assert_eq!(crop_ids(&crop), vec![1, 2]);
    }
}
