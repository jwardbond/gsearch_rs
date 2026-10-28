use log::{debug, info};

use crate::alignment::{Alignment, align};
use crate::graph::Graph;
use crate::match_generation::get_candidate_matches;
use crate::spatial_index::SpatialIndex;

/// The default weight of the edge score. The Python CLI does not have an option for it.
pub const DEFAULT_WEIGHT: f64 = 0.5;

pub struct SearchParams {
    pub tol: f64,
    pub k: usize,
    pub weight: f64,
}

pub struct SearchOutcome {
    /// At most `k` alignments, best first.
    pub alignments: Vec<Alignment>,
    pub n_anchors: usize,
    pub n_mappings: usize,
}

/// The nodes of `g` to try as the anchor, with a score, best first.
///
/// For now every node has a score of 1.0. This is a placeholder for a future scoring function.
fn get_candidate_anchors(_q_anchor: usize, _q: &Graph, g: &Graph) -> Vec<(usize, f64)> {
    let mut anchors: Vec<(usize, f64)> = (0..g.node_count()).map(|j| (j, 1.0)).collect();
    anchors.sort_by(|a, b| b.1.total_cmp(&a.1));
    anchors
}

/// The `k` lowest scores in order
struct TopK {
    k: usize,
    items: Vec<Alignment>,
}

impl TopK {
    fn new(k: usize) -> TopK {
        TopK {
            k,
            items: Vec::with_capacity(k),
        }
    }

    /// Alignments with equal scores keep the order in which they came, as in Python's `sorted`.
    fn push(&mut self, alignment: Alignment) {
        let at = self.items.partition_point(|a| a.score <= alignment.score);

        // Return early if
        // - new is worse than all existing
        // - we already have k items
        if at >= self.k {
            return;
        }

        let bbox = alignment.matched.bbox();
        let overlaps: Vec<(usize, f64)> = self
            .items
            .iter()
            .enumerate()
            .map(|(idx, i)| (idx, i.matched.bbox().overlap(bbox)))
            .filter(|&(_, overlap)| overlap >= 0.5)
            .collect();

        // Return early if an overlapping item with a better score already exists
        if overlaps.iter().any(|&(idx, _)| idx < at) {
            return;
        }

        // Bump the most overlapping item, else the last item
        let overlap_idx = overlaps
            .into_iter()
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(idx, _)| idx);

        self.items.insert(at, alignment);

        if let Some(idx) = overlap_idx {
            self.items.remove(idx + 1);
        } else if self.items.len() > self.k {
            self.items.pop();
        }
    }
}

/// Searches `index.graph()` for `q`.
///
/// `on_anchor` is a callback used to report progress to watchers.
pub fn run_gsearch(
    q: &Graph,
    index: &SpatialIndex,
    params: &SearchParams,
    mut on_anchor: impl FnMut(usize, usize),
) -> SearchOutcome {
    let q_anchor = 0;
    let radius = q.radius_from(q_anchor) + params.tol;

    // Get all candidate anchors, sorted by score.
    let anchors = get_candidate_anchors(q_anchor, q, index.graph());
    let total = anchors.len();

    // Build the candidate mappings.
    let mut best = TopK::new(params.k);
    let mut n_mappings = 0;
    for (done, &(g_anchor, _)) in anchors.iter().enumerate() {
        let crop = index.make_crop(g_anchor, radius);

        // Get the candidate mappings for a pair of anchors.
        let Some(mappings) = get_candidate_matches(q, &crop, q_anchor, 0, params.tol) else {
            on_anchor(done + 1, total);
            continue;
        };
        debug!("anchor {g_anchor}: {} mappings", mappings.len());
        n_mappings += mappings.len();

        // Process each candidate mapping.
        for mapping in &mappings {
            if let Some(alignment) = align(q, &crop, mapping, params.tol, params.weight) {
                best.push(alignment);
            }
        }

        on_anchor(done + 1, total);
    }

    info!(
        "{total} anchors, {n_mappings} mappings, {} kept",
        best.items.len()
    );
    SearchOutcome {
        alignments: best.items,
        n_anchors: total,
        n_mappings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Transform;
    use crate::io::NodeId;
    use crate::test_fixtures::file;

    fn graph(nodes: &[(NodeId, f64, f64)], edges: &[(NodeId, NodeId)]) -> Graph {
        Graph::from_file(&file(nodes, edges)).unwrap()
    }

    fn alignment(score: f64, tag: NodeId) -> Alignment {
        Alignment {
            score,
            transform: Transform::IDENTITY,
            mapping: vec![(tag, tag)],
            matched: graph(&[(tag, 0.0, 0.0)], &[]),
        }
    }

    fn scores(top: &TopK) -> Vec<f64> {
        top.items.iter().map(|a| a.score).collect()
    }

    /// An alignment whose matched graph has the bbox (x, 0) to (x + w, h).
    fn alignment_at(score: f64, tag: NodeId, x: f64, w: f64, h: f64) -> Alignment {
        Alignment {
            score,
            transform: Transform::IDENTITY,
            mapping: vec![(tag, tag)],
            matched: graph(&[(tag, x, 0.0), (tag + 1000, x + w, h)], &[]),
        }
    }

    fn tags(top: &TopK) -> Vec<NodeId> {
        top.items.iter().map(|a| a.matched.id(0)).collect()
    }

    #[test]
    fn every_node_is_a_candidate_anchor() {
        let q = graph(&[(1, 0.0, 0.0)], &[]);
        let g = graph(&[(10, 0.0, 0.0), (11, 1.0, 0.0), (12, 2.0, 0.0)], &[]);

        let anchors = get_candidate_anchors(0, &q, &g);

        // all scores are 1.0, so the order of the nodes is kept
        assert_eq!(anchors, vec![(0, 1.0), (1, 1.0), (2, 1.0)]);
    }

    #[test]
    fn top_k_keeps_the_lowest_scores_in_order() {
        let mut top = TopK::new(2);

        for score in [5.0, 1.0, 3.0, 2.0] {
            top.push(alignment(score, 0));
        }

        assert_eq!(scores(&top), vec![1.0, 2.0]);
    }

    #[test]
    fn top_k_keeps_the_first_of_equal_scores() {
        let mut top = TopK::new(2);

        for tag in [1, 2, 3] {
            top.push(alignment(1.0, tag));
        }

        // the first two stay, as with a stable sort
        let tags: Vec<NodeId> = top.items.iter().map(|a| a.matched.id(0)).collect();
        assert_eq!(tags, vec![1, 2]);
    }

    #[test]
    fn top_k_with_k_zero_keeps_nothing() {
        let mut top = TopK::new(0);

        top.push(alignment(1.0, 0));

        assert!(top.items.is_empty());
    }

    #[test]
    fn top_k_with_fewer_than_k_keeps_all() {
        let mut top = TopK::new(5);

        top.push(alignment(2.0, 0));
        top.push(alignment(1.0, 0));

        assert_eq!(scores(&top), vec![1.0, 2.0]);
    }

    #[test]
    fn top_k_ignores_an_alignment_worse_than_a_full_list() {
        let mut top = TopK::new(2);

        top.push(alignment_at(1.0, 1, 0.0, 10.0, 10.0));
        top.push(alignment_at(2.0, 2, 100.0, 10.0, 10.0));
        top.push(alignment_at(3.0, 3, 200.0, 10.0, 10.0));

        assert_eq!(tags(&top), vec![1, 2]);
    }

    #[test]
    fn top_k_replaces_an_overlapping_worse_alignment() {
        let mut top = TopK::new(3);
        top.push(alignment_at(1.0, 1, 0.0, 10.0, 10.0));
        top.push(alignment_at(3.0, 2, 100.0, 10.0, 10.0));

        top.push(alignment_at(2.0, 3, 100.0, 10.0, 10.0));

        assert_eq!(tags(&top), vec![1, 3]);
    }

    #[test]
    fn top_k_drops_an_alignment_that_overlaps_a_better_one() {
        let mut top = TopK::new(3);
        top.push(alignment_at(1.0, 1, 0.0, 10.0, 10.0));
        top.push(alignment_at(3.0, 2, 100.0, 10.0, 10.0));

        top.push(alignment_at(2.0, 3, 0.0, 10.0, 10.0));

        assert_eq!(tags(&top), vec![1, 2]);
    }

    #[test]
    fn top_k_drops_an_alignment_that_overlaps_a_better_and_a_worse_one() {
        let mut top = TopK::new(3);
        // A and B overlap each other by 0.43, which is below 0.5
        top.push(alignment_at(1.0, 1, 0.0, 10.0, 10.0));
        top.push(alignment_at(3.0, 2, 4.0, 10.0, 10.0));

        // overlaps both by 0.67
        top.push(alignment_at(2.0, 3, 2.0, 10.0, 10.0));

        assert_eq!(tags(&top), vec![1, 2]);
    }

    #[test]
    fn top_k_replaces_the_most_overlapping_worse_alignment() {
        let mut top = TopK::new(4);
        // 1 and 2 overlap each other by 0.43, which is below 0.5
        top.push(alignment_at(3.0, 1, -3.0, 10.0, 10.0));
        top.push(alignment_at(4.0, 2, 1.0, 10.0, 10.0));

        // overlaps 1 by 0.54 and 2 by 0.82
        top.push(alignment_at(2.0, 3, 0.0, 10.0, 10.0));

        assert_eq!(tags(&top), vec![3, 1]);
    }

    #[test]
    fn top_k_replaces_an_overlapping_alignment_at_the_end_of_a_full_list() {
        let mut top = TopK::new(3);
        top.push(alignment_at(1.0, 1, 0.0, 10.0, 10.0));
        top.push(alignment_at(2.0, 2, 100.0, 10.0, 10.0));
        top.push(alignment_at(4.0, 3, 200.0, 10.0, 10.0));

        top.push(alignment_at(3.0, 4, 200.0, 10.0, 10.0));

        assert_eq!(tags(&top), vec![1, 2, 4]);
    }

    #[test]
    fn top_k_treats_an_overlap_of_exactly_one_half_as_overlapping() {
        let mut top = TopK::new(2);
        top.push(alignment_at(3.0, 1, 0.0, 10.0, 10.0));

        // lies inside the first box, with half of its area
        top.push(alignment_at(2.0, 2, 0.0, 10.0, 5.0));

        assert_eq!(tags(&top), vec![2]);
    }

    #[test]
    fn top_k_keeps_an_alignment_that_overlaps_below_one_half() {
        let mut top = TopK::new(2);
        top.push(alignment_at(3.0, 1, 0.0, 10.0, 10.0));

        // lies inside the first box, with 0.4 of its area
        top.push(alignment_at(2.0, 2, 0.0, 10.0, 4.0));

        assert_eq!(tags(&top), vec![2, 1]);
    }

    /// A 3-4-5 triangle as Q, and a G with one copy of it, far away, and one stray node.
    fn q_and_g() -> (Graph, Graph) {
        let q = graph(
            &[(1, 0.0, 0.0), (2, 3.0, 0.0), (3, 0.0, 4.0)],
            &[(1, 2), (1, 3)],
        );
        let g = graph(
            &[
                (11, 100.0, 100.0),
                (12, 103.0, 100.0),
                (13, 100.0, 104.0),
                (14, 300.0, 300.0),
            ],
            &[(11, 12), (11, 13)],
        );
        (q, g)
    }

    #[test]
    fn finds_the_copy_and_counts_the_work() {
        let (q, g) = q_and_g();
        let index = SpatialIndex::new(g);
        let params = SearchParams {
            tol: 0.5,
            k: 3,
            weight: DEFAULT_WEIGHT,
        };

        let outcome = run_gsearch(&q, &index, &params, |_, _| {});

        // 4 anchors are tried, and only the anchor 11 has a mapping: Q 1->11, 2->12, 3->13
        assert_eq!(outcome.n_anchors, 4);
        assert_eq!(outcome.n_mappings, 1);
        assert_eq!(outcome.alignments.len(), 1);
        assert_eq!(
            outcome.alignments[0].mapping,
            vec![(1, 11), (2, 12), (3, 13)]
        );
    }

    #[test]
    fn reports_progress_after_each_anchor() {
        let (q, g) = q_and_g();
        let index = SpatialIndex::new(g);
        let params = SearchParams {
            tol: 0.5,
            k: 1,
            weight: DEFAULT_WEIGHT,
        };
        let mut calls = Vec::new();

        run_gsearch(&q, &index, &params, |done, total| calls.push((done, total)));

        assert_eq!(calls, vec![(1, 4), (2, 4), (3, 4), (4, 4)]);
    }

    #[test]
    fn k_limits_the_alignments() {
        let (q, g) = q_and_g();
        let index = SpatialIndex::new(g);
        let params = SearchParams {
            tol: 0.5,
            k: 0,
            weight: DEFAULT_WEIGHT,
        };

        let outcome = run_gsearch(&q, &index, &params, |_, _| {});

        // the mapping is counted, but nothing is kept
        assert_eq!(outcome.n_mappings, 1);
        assert!(outcome.alignments.is_empty());
    }

    #[test]
    fn a_query_that_is_not_in_g_gives_nothing() {
        let (_, g) = q_and_g();
        let q = graph(&[(1, 0.0, 0.0), (2, 50.0, 0.0)], &[(1, 2)]);
        let index = SpatialIndex::new(g);
        let params = SearchParams {
            tol: 0.5,
            k: 1,
            weight: DEFAULT_WEIGHT,
        };

        let outcome = run_gsearch(&q, &index, &params, |_, _| {});

        assert_eq!(outcome.n_mappings, 0);
        assert!(outcome.alignments.is_empty());
    }
}
