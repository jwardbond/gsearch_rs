# Slice 7: Path scoring (with a placeholder `find_paths`)

**Goal.** Port the rest of `align_edges.py`. For one mapping, `align_and_score` moves `Q` onto the
crop (Slice 6), routes every edge of `Q` through the crop, and returns an `Alignment`: the
score, the transform, and the part of the crop that matches. You write the `Alignment` type,
the way to put its parts together, and the score. You do **not** write `find_paths` in this
slice (Dijkstra, the corridor and the area cost). It is a **placeholder** with a written spec and
ignored tests, as `fit` was in Slice 6.

**Outcome.** `cargo test alignment::` reports `8 passed; 0 failed; 3 ignored`, and
`cargo test path_scoring::` reports `1 passed; 0 failed; 9 ignored`. The 12 ignored tests (with
the 4 of Slice 6, 16 in all) are what remains for you.

**New ideas.** Iterating over the edges of a graph; `BTreeSet` (a sorted set); `windows(2)` for
the steps of a path; `min` and `max` on numbers; `?` on an `Option` inside a function that
returns an `Option`; `Vec<_>`; `into_iter` (consume a collection); a result type that holds
ids, and why; `assert_eq!` against `None`.

**Starting point.** Slice 6 is done: `cargo test procrustes` reports `8 passed; 4 ignored`. The
type `Mapping` is `Vec<usize>` (see the note in Slice 6).

---

## The design

### What the Python code does

`align_and_score(Q, G_crop, match, tol, weight)`:

1. Build the arrays of coordinates, and `_align_graph` (Slice 6) gives `R`, `t` and the moved `Q`.
2. `node_score = np.linalg.norm(Q_aligned - G_matched)`.
3. `_find_paths` goes through the edges of `Q`. For an edge `(u, v)`, it measures the distance
   from every node of the crop to the **segment** between the moved `u` and `v`. A step of a
   path from `a` to `b` is allowed only if both ends are within `tol` of the segment. Its cost is
   `0.5 * (d_a + d_b) * length(a, b)`: the area of a trapezoid. `nx.single_source_dijkstra` finds
   the cheapest path from `match[u]` to `match[v]`. If there is none, the whole mapping is
   rejected (`None`).
4. The nodes of the result are the matched nodes and all path nodes. The edges are the steps
   of the paths.
5. `score = node_score + weight * edge_score`.

### What changes in Rust

| Python | Rust |
|---|---|
| `match: dict[int, int]` | `mapping: &[usize]`, `mapping[i]` is the crop position of Q position `i` |
| `Q.edges` | `q.edges()` (new, Step 1): each edge once, as `(i, j)` with `i < j` |
| `_find_paths` returns `(edge_score, {edge: path})` or `None` | `find_paths` returns `Option<EdgeRoutes>`; `paths[e]` is the path of edge `e` of `q.edges()` |
| a result tuple with an `nx.Graph` | an `Alignment` struct that holds ids |
| the `weight` docstring and the code disagree | the code wins: `node_score + weight * edge_score` (README) |

**D10. The distance is to the line, not to the segment (decided).** Python measures the distance
from a node to the **segment** of the moved edge. This port measures it to the infinite **line**
through the two moved end points. A node that is beyond the end of the segment, but on the line,
is then free: it is in the corridor, and it adds nothing to the area. This is a planned
difference from Python, so scores and some rejections will not be the same as in Python. It is
in the ignored test `a_node_past_the_end_is_free_on_a_line`, which fails if you use the segment.

**An `Alignment` holds ids, not positions.** A position only has a meaning inside one crop (D7).
The search (Slice 8) drops each crop after the work on its anchor, but it keeps the best
alignments until the end. So `align_and_score` turns positions into ids before it returns.

---

## Steps

### 1. `Graph::edges`

`find_paths` needs the edges of `Q`. Python has `Q.edges`. Your `Graph` keeps adjacency lists
(`adj[i]` holds the neighbours of `i`, in both directions), so an edge is in two lists. The
method gives each edge once.

**Try first.** Write `pub fn edges(&self) -> Vec<(usize, usize)>`. Keep a pair `(i, j)` only when
`i < j`. The result is in increasing order of `i`, then `j`, because the lists are sorted
(Slice 3, Step 8).

- [ ] Add a stub in `impl Graph` in `src/graph.rs`, before `distance_from_node`:

  ```rust
      /// Every edge once, as `(i, j)` with `i < j`, in increasing order.
      pub fn edges(&self) -> Vec<(usize, usize)> {
          todo!()
      }
  ```

- [ ] Add a test at the end of `mod tests` in `graph.rs`:

  ```rust
      #[test]
      fn edges_lists_each_edge_once_in_order() {
          let graph = fixture();

          let edges = graph.edges();

          // positions follow the file order: 50 is 0, 2 is 1, 17 is 2, 8 is 3, 33 is 4, 7 is 5
          assert_eq!(edges, vec![(0, 1), (0, 2), (1, 4), (2, 3), (4, 5)]);
          // the list is as long as the edge count
          assert_eq!(edges.len(), graph.edge_count());
      }
  ```

- [ ] Run `cargo test edges_lists`. Expected: the test fails with `not yet implemented`.
- [ ] Replace the body of `edges`:

  ```rust
          let mut edges = Vec::with_capacity(self.edge_count());
          for (i, neighbors) in self.adj.iter().enumerate() {
              for &j in neighbors {
                  if i < j {
                      edges.push((i, j));
                  }
              }
          }
          edges
  ```

  1. `Vec::with_capacity(self.edge_count())` reserves the right size at once (Slice 3, Step 6).
  2. `for (i, neighbors) in self.adj.iter().enumerate()` gives the position and the list.
  3. The test `i < j` keeps the copy of each edge that is found from its smaller end.
  4. The method builds a new `Vec` on every call. `find_paths` calls it once for each mapping. If
     the profile in Slice 9 says that this matters, an iterator that gives the edges one by one
     (no `Vec`) is the fix.

- [ ] Run `cargo test edges_lists`. Expected: `1 passed; 0 failed`.

### 2. The placeholder `find_paths`

- [ ] Create `src/path_scoring.rs`:

  ```rust
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
  ```

  `Vec<Vec<usize>>` is a list of paths, and a path is a list of crop positions. The type
  `EdgeRoutes` is a struct and not a tuple `(f64, Vec<...>)`, so that a caller writes
  `routes.cost` and not `routes.0`. The derive `PartialEq` lets a test write
  `assert_eq!(routes, None)`.

- [ ] Add the function:

  ```rust
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
  ```

  1. The placeholder says that every edge routes at no cost, and it gives no path. The
     assembly in Step 3 then keeps only the mapped nodes, and `edges` is empty.
  2. The function takes `moved_q` (the positions) and not the `Transform`. It needs only
     where the nodes are, and so it does not depend on `procrustes`.
  3. `mapping: &[usize]`: a `&Vec<usize>` (what `&mapping` is) turns into a `&[usize]` by
     itself at a call. Slice 3, Step 7 explains slices.

- [ ] Add `pub mod path_scoring;` to `src/lib.rs`.

- [ ] Add the tests. First the helpers and the placeholder test:

  ```rust
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
  ```

  The helpers hide the bookkeeping: the tests use ids (`10`, `12`) because they are easier to
  read, and `route` turns them into positions. `Q` and `G` share one frame in these tests, so
  `moved_q` is just `q.coords()`.

- [ ] Add the nine ported tests. They are the Python `TestFindPaths`, with the same numbers.
  Each is `#[ignore]` until you write `find_paths`. Put them after the placeholder test, and
  close `mod tests` with a last `}`:

  ```rust
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
  ```

  1. Eight of these tests are the Python tests. The Python test about the result type
     (`test_returns_score_and_paths_dict`) is not needed in Rust, because the type says it.
     `a_node_past_the_end_is_free_on_a_line` is new: it tests decision D10.
  2. I ran the nine tests against a throw-away implementation (Dijkstra with a binary heap and
     the distance to the line), and all pass. With the distance to the **segment**, the last
     test fails and the other eight pass. So the tests agree with each other.
  3. `assert_eq!(routes, None)` compares an `Option<EdgeRoutes>` with `None`. It needs `Debug`
     and `PartialEq` on `EdgeRoutes`, from the derive in the first piece.
  4. The tests in one query edge ask for one path, so `paths[0]` is the path. The test with two
     edges is the only one that checks the order of `paths`.

- [ ] Run `cargo test path_scoring::`. Expected: `1 passed; 0 failed; 9 ignored`.
- [ ] Run `cargo test path_scoring:: -- --ignored`. Expected: `0 passed; 9 failed`. Look at one
  failure. `no_path_gives_none` reads `left: Some(EdgeRoutes { cost: 0.0, paths: [] })`,
  `right: None`.

**Notes for when you write `find_paths`.** For each `(u, v)` in `q.edges()`:

1. Let `a = moved_q[u]` and `b = moved_q[v]`. The distance of a point `p` to the line is
   `|(b - a) x (p - a)| / |b - a|` (the cross product of Slice 2). If `a == b` the line is not
   defined: use the distance to the point `a`, as the Python code does.
2. A step from crop node `m` to crop node `n` is allowed when `d[m] <= tol` and `d[n] <= tol`.
   Its cost is `0.5 * (d[m] + d[n]) * |m - n|`.
3. Find the cheapest path from `mapping[u]` to `mapping[v]`. This is Dijkstra, and you write it
   (D5). `f64` is not `Ord` (Slice 3, Step 11), so a `BinaryHeap` needs a small wrapper type
   with `total_cmp`, and `std::cmp::Reverse` (or a reversed `cmp`) turns the max-heap into a
   min-heap. The pieces are explained at the point you need them, so ask for them then.
4. If `mapping[u]` or `mapping[v]` is not reachable, the answer for the whole mapping is `None`.
5. The trapezoid rule on unsigned distances counts a little extra area when a path crosses the
   line. The Python code does the same. Keep it, so that results stay comparable.

### 3. `Alignment` and `align_and_score`

The code in this step is yours to write in full. It uses `fit` and `find_paths` through their
signatures, so it works with the placeholders now, and it keeps working when you replace them.

- [ ] Create `src/alignment.rs`, and add `pub mod alignment;` to `src/lib.rs`:

  ```rust
  //! Aligns and scores one mapping (`align_and_score` in Python).

  use crate::io::NodeId;
  use crate::procrustes::Transform;

  /// A mapping with its transform, its score and the part of the crop that realises it.
  ///
  /// It holds ids, not positions: a position only has a meaning inside one crop, and the crop
  /// is dropped before the search ends.
  #[derive(Debug, Clone, PartialEq)]
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
  ```

  The Python function returns a tuple of five items (`score, R, t, subgraph, match`). A struct
  gives each item a name. `Transform` holds `R` and `t`. The graph `subgraph` is two lists,
  because nothing in the program needs a `Graph` here, and ids are all the output needs.
  Step 3c explains why the ids are sorted.

#### 3a. `combine`

Only the edge score has a weight (the README: the code wins, not the docstring).

- [ ] Add a stub before the tests, and the first test module:

  ```rust
  /// The score of an alignment. As in the Python code, only the edge score has a weight.
  fn combine(node_score: f64, edge_score: f64, weight: f64) -> f64 {
      todo!()
  }

  #[cfg(test)]
  mod tests {
      use super::*;
      use crate::test_fixtures::assert_close;

      #[test]
      fn combine_weights_only_the_edge_score() {
          assert_close(combine(2.0, 4.0, 0.5), 4.0);
          assert_close(combine(2.0, 4.0, 0.0), 2.0);
          assert_close(combine(2.0, 4.0, 1.0), 6.0);
      }
  }
  ```

- [ ] Run `cargo test combine`. Expected: the test fails with `not yet implemented`.
- [ ] Replace the body: `node_score + weight * edge_score`. A function ends with an expression
  and no `;` (Slice 1, Step 9).
- [ ] Run `cargo test combine`. Expected: `1 passed; 0 failed`.

A function this small has its own name and test because the formula is the place where the
Python docstring and the Python code disagree. If you change your mind, one line changes, and
the test says what the formula was.

#### 3b. `assemble`

`assemble` makes the node list and the edge list of the alignment. The input is the mapping and
the paths from `find_paths`. The Python code builds two sets, and the sets remove repeats. A
node is in the result once, even if two paths use it. An edge is in the result once, even if
two paths use it in opposite directions.

**Try first.** Write `fn assemble(mapping: &[usize], paths: &[Vec<usize>]) -> (Vec<usize>, Vec<(usize, usize)>)`.
A `HashSet` works, but its order is random, and a result with a random order is hard to test and
to compare. `std::collections::BTreeSet` keeps its items sorted.

- [ ] Add a stub after `combine`, with its imports at the top of the file:

  ```rust
  use std::collections::BTreeSet;
  ```

  ```rust
  /// The crop positions of the mapped nodes and of all the path nodes, and the path edges.
  /// Nothing is in the lists twice. Both are in increasing order of position.
  fn assemble(mapping: &[usize], paths: &[Vec<usize>]) -> (Vec<usize>, Vec<(usize, usize)>) {
      todo!()
  }
  ```

- [ ] Add three tests to `mod tests`:

  ```rust
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
  ```

- [ ] Run `cargo test assemble`. Expected: `0 passed; 3 failed`.
- [ ] Replace the body:

  ```rust
      let mut nodes: BTreeSet<usize> = mapping.iter().copied().collect();
      let mut edges = BTreeSet::new();

      for path in paths {
          nodes.extend(path);
          for step in path.windows(2) {
              edges.insert((step[0].min(step[1]), step[0].max(step[1])));
          }
      }
      (nodes.into_iter().collect(), edges.into_iter().collect())
  ```

  1. `collect()` into a `BTreeSet<usize>` builds the sorted set from the iterator. It is like
     `set(mapping)` in Python, but the items are kept in order. The type is written on the
     variable, so `collect` knows what to build (Slice 4, Step 3).
  2. `nodes.extend(path)` adds all the items of `path`. `path` is a `&Vec<usize>`, and a set of
     `usize` can take the references (it copies the numbers), as `extend` did in Slice 5,
     Step 7.
  3. `path.windows(2)` gives every pair of neighbours, as `pairwise(path)` does in Python
     (Slice 3, Step 10). `step` is a `&[usize]` of length 2.
  4. `a.min(b)` and `a.max(b)` are methods on numbers. They put the smaller id first, so the
     edge `(5, 4)` and the edge `(4, 5)` are one item. Python's `nx.Graph` does the same,
     because its edges have no direction.
  5. The type of `edges` is found from `insert`, so it needs no annotation.
  6. `into_iter()` on a set gives the items by value and **uses up** the set, which is what we
     want at the end. `collect()` makes a `Vec` of them. The tuple on the last line has the type
     that the signature says, so the two `collect()` calls know their types.

- [ ] Run `cargo test assemble`. Expected: `3 passed; 0 failed`.

#### 3c. `align_and_score`

**Try first.** The function has five steps (see "What the Python code does"). Write them in
order, and call the functions that you have: `fit`, `Transform::apply_all`, `node_score`,
`find_paths`, `assemble`, `combine`. The signature is:

```rust
pub fn align_and_score(
    q: &Graph,
    crop: &Graph,
    mapping: &[usize],
    tol: f64,
    weight: f64,
) -> Option<Alignment>
```

**Suggested code.** First the head of the function (the node part):

```rust
    let targets: Vec<_> = mapping.iter().map(|&j| crop.coord(j)).collect();

    let transform = fit(q.coords(), &targets);
    let moved = transform.apply_all(q.coords());
    let node_cost = node_score(&moved, &targets);
```

1. `targets[i]` is the position of the crop node that is mapped to Q position `i`. It is
   `G_arr_matched` in Python, and the lookup `G_nodes.index(match[q])` is not needed, because
   a mapping holds positions (D7).
2. `Vec<_>` means "a `Vec`, and you find its item type". The type comes from the closure
   (`crop.coord` returns a `Point`). You can write `Vec<Point>` if you prefer.
3. The first line of `fit` that uses `q.coords()` is a `&[Point]` already, so it needs no
   `&`. `&targets` borrows the `Vec` as a slice.

Then the edge part. This is where the `?` is:

```rust
    let routes = find_paths(q, crop, mapping, &moved, tol)?;

    let (nodes, edges) = assemble(mapping, &routes.paths);
```

4. `?` on an `Option` (Slice 5, Step 4): if `find_paths` returns `None`, `align_and_score`
   returns `None` at once. This is `if result is None: return None` in Python. The function
   returns `Option<Alignment>`, so the `?` is allowed.

Last, the result. It turns positions into ids, and sorts them:

```rust
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
```

5. `assemble` returns positions in increasing order of **position**. The ids of a crop do not
   follow the positions (the ids are the file ids, D7). The doc comment of `Alignment` promises
   increasing **id** order, so the code sorts after the change to ids. `sort_unstable` (Slice 3,
   Step 8) is enough: equal ids cannot occur, so the order of equal items does not matter.
6. For an edge, the smaller id must be first, and the smaller position need not have the smaller
   id. So the pair is built again with `min` and `max` on the ids.
7. `nodes` and `edges` shadow the earlier `nodes` and `edges` (Slice 4, Step 13). The old ones
   (positions) are used up by `into_iter()`, so no name is left that points at the wrong data.
8. `transform,` and `nodes,` are short for `transform: transform,` (a field with the same name
   as a variable, Slice 1, Step 3).
9. The mapping holds `(q.id(i), crop.id(j))`: the file id of Q position `i`, and the file id of
   the crop node.

- [ ] Add the imports at the top of the file:

  ```rust
  use crate::graph::Graph;
  use crate::path_scoring::find_paths;
  use crate::procrustes::{fit, node_score};
  ```

- [ ] Add the function (with a `todo!()` body first, if you want to see the tests fail), and
  these tests to `mod tests`. Add `use crate::test_fixtures::{assert_close, file};` in place of
  the `assert_close` import, and the `graph` helper:

  ```rust
      fn graph(nodes: &[(NodeId, f64, f64)], edges: &[(NodeId, NodeId)]) -> Graph {
          Graph::from_file(&file(nodes, edges)).unwrap()
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
      fn nodes_are_sorted_by_id_not_by_position() {
          let q = graph(&[(1, 0.0, 0.0), (2, 3.0, 0.0)], &[(1, 2)]);
          // the ids fall while the positions rise
          let g = graph(&[(13, 0.0, 0.0), (12, 3.0, 0.0), (11, 9.0, 9.0)], &[(13, 12)]);

          let alignment = align_and_score(&q, &g, &[0, 1], 1.0, 0.5).unwrap();

          assert_eq!(alignment.nodes, vec![12, 13]);
      }

      #[test]
      fn a_mapping_to_other_positions_is_reported_by_id() {
          let (q, g) = q_and_copy();

          // a different mapping: Q's nodes 1 and 2 go to G's 12 and 11
          let alignment = align_and_score(&q, &g, &[1, 0, 2], 100.0, 0.5).unwrap();

          assert_eq!(alignment.mapping, vec![(1, 12), (2, 11), (3, 13)]);
          assert_eq!(alignment.nodes, vec![11, 12, 13]);
      }
  ```

  1. These four tests pass with the placeholders. In `a_copy_in_the_same_frame_scores_zero`,
     `Q` and `G` are in the same frame, so the identity is the right transform, and the
     placeholder `find_paths` has no cost. When you write the real `fit` and `find_paths`,
     the tests keep passing.
  2. `a_mapping_to_other_positions_is_reported_by_id` uses a large `tol`, so that a real
     `find_paths` still finds paths for the crossed mapping.

- [ ] Run `cargo test alignment::` (the two colons keep out `pick_alignment_node` and
  `perfect_alignment_costs_zero`, which also have `alignment` in their names). Expected:
  `8 passed; 0 failed`: one test of `combine`, three of `assemble` and four of
  `align_and_score`. If you started from the `todo!()` body, the four new tests first fail with
  `not yet implemented`.

- [ ] Add three more tests that wait for your `fit` and `find_paths`:

  ```rust
      // The tests below need your `fit` and your `find_paths`.
      // Run them with `cargo test alignment:: -- --ignored`.

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
  ```

  `assert_eq!(alignment, None)` needs `PartialEq` on `Alignment`. It is in the derive.
  `a_moved_copy_scores_zero` is the real check of Slice 6 and this slice together: `fit` finds
  the shift, and the area is zero.

- [ ] Run `cargo test alignment::`. Expected: `8 passed; 0 failed; 3 ignored`.
- [ ] Run `cargo test alignment:: -- --ignored`. Expected: `0 passed; 3 failed`.

### 4. Look at it

- [ ] Run `cargo test`. Expected: `0 failed`. Compared with the end of Slice 6, `passed` is 10
  higher (1 for `edges`, 1 in `path_scoring`, 8 in `alignment`) and `ignored` is 12 higher
  (9 and 3), so `ignored` is `16` in all (the 4 of Slice 6 and the 12 of this slice).
- [ ] Run `cargo test -- --ignored`. Expected: `1 passed; 15 failed`. The one that passes is
  `fit_is_the_identity_when_q_equals_g` (Slice 6). The other 15 are your work list.

### 5. Format, lint, commit

- [ ] Run `cargo fmt; cargo clippy --all-targets`. Expected: no warning from the new files.
- [ ] Commit:

  ```powershell
  git add src guide GLOSSARY.md
  git commit -m "feat: alignment, assemble and scoring with a placeholder find_paths"
  ```

---

## What comes next

Slice 8 loops over the candidate anchors, calls `get_candidate_matches` and `align_and_score`
for each mapping, keeps the `k` best alignments, and wraps it all in a CLI with JSON output.
