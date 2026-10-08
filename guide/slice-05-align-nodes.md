# Slice 5: Node matching

**Goal.** Port `align_nodes.py`. `get_candidate_matches` takes the query `Q` and a crop of `G`,
with the anchor of each at position `0`. It returns every **pose**: a way to give each node of
`Q` its own node of the crop, such that the distances to the anchors agree within `tol`. Slices
6 and 7 align and score each pose.

**Outcome.** `cargo test align` reports `24 passed`, and `cargo run` finds the poses around
every node of a road graph and says how long it took.

**New ideas.** A table as `Vec<Vec<bool>>`; recursion with `&mut` parameters, in place of a
Python generator; why a closure cannot call itself; `contains`; `match` on a tuple, with `|`
and `_` patterns; `&=`; `min_by_key`; `let ... else`; `extend`; `flatten` and `count`.

**Starting point.** Slice 4 is done, and `cargo test` reports `51 passed`.

Notes about your files:

- Your component method is `cc_containing`, and the tree field is `kd_tree`. This slice does not
  use either directly.
- Your `query_radius` is private (`fn`, not `pub fn`). That is fine: nothing outside
  `spatial_index.rs` needs it.
- Your `src/main.rs` is still the Slice 1 version (Slice 4, Step 13 was not done). Step 11 of
  this slice replaces the whole file, so you do not need to do Slice 4, Step 13 first.

---

## The design

### What the Python code does

`get_candidate_matches(Q, G, Q_anchor, G_anchor, tol)` works in five steps:

1. **Anchor mask.** Put the anchor of `Q` on the anchor of `G`. Turn `Q` around that point. Each
   node of `Q` draws a circle, and a band of width `2 * tol` around that circle is an
   *annulus*. The mask has one row per node of `Q` and one column per node of `G`, and a cell
   is `True` if that `G` node is in the annulus of that `Q` node. In numbers: the distance of
   the `Q` node to its anchor and the distance of the `G` node to its anchor differ by at most
   `tol`.
2. **Early exits.** If `Q` has more nodes than `G`, or a row of the mask is all `False`, there
   is no pose.
3. **Alignment node.** Take the node of `Q` (not the anchor) with the fewest `True` cells in
   its row. On a tie, take the first one.
4. **For each `G` node in the row of the alignment node:** make a second annulus mask around
   the alignment pair, `&` it with the anchor mask, and split the result by **handedness**.
   The anchor and the alignment node give an axis in `Q` and an axis in `G`. A node of `Q` on
   the left of its axis can match a node of `G` on the left of its axis in a rotation, and a
   node on the right in a reflection. A node on an axis goes into both masks.
5. **Candidates.** For each of the two masks, list every **injective assignment**: one column
   per row, with no column used twice. Each assignment is a pose.

### The Rust version

| Python | Rust |
|---|---|
| `Q_arr`, `G_arr` (numpy arrays) | `q.coords()`, `g.coords()` (`&[Point]`) |
| a boolean array `(len(Q), len(G))` | `type Mask = Vec<Vec<bool>>` |
| a pose `dict[int, int]` from Q id to G id | `type Pose = Vec<usize>`: `pose[i]` is the crop position for Q position `i` |
| `_cross_signs` returns `+1/-1/0` | `sides` returns `Vec<Side>`, with `side_of` from Slice 2 |
| `_split_mask_by_handedness(mask, Q_arr, G_arr, 4 indices)` | `split_by_handedness(mask, q_sides, g_sides)` |
| `_make_candidates(masks, Q_ids, G_ids)` | `candidates(mask)`, called once per mask |
| `_injective_assignments`, a recursive generator | a recursive function that pushes into a `Vec` |

Some choices need an explanation.

**A pose holds positions, not ids (decision D7).** Python returns `{Q id: G id}`, and
`align_and_score` then calls `G_nodes.index(...)` to find each row again. A `Vec<usize>` with
one crop position per Q position needs no lookup. Slice 6 indexes `crop.coords()` with it
directly. The ids are only needed when the result is printed (Slice 8).

**A mask is a `Vec<Vec<bool>>`.** It reads like the numpy array: `mask[i][j]`, and a test can
write `vec![vec![true, false], vec![false, true]]`. The masks are small (a query has 3 to 20
nodes, and a mean crop has about 100), so the extra allocation per row does not matter yet.
Slice 9 measures it.

**The split takes sides, not coordinates.** The handedness of a pair depends only on the side
of each node about its own axis. So `sides` computes a `Vec<Side>` once per axis, and the split
only compares them. `side_of` and `Side::flipped` already exist and are tested (Slice 2). The
Python tests of `_cross_signs` are the geometry tests of Slice 2, so this slice does not port
them again.

**One-node query.** With one node, `Q` has no alignment node. Python then uses the anchor as
its own alignment node and returns poses from a degenerate axis. The Rust version returns no
poses. This is a second planned difference from Python. A one-node query is not in the data.

### The tests

The Python tests are in `tests/test_align_nodes.py`.

| Python test | Rust test |
|---|---|
| `test_output_shape` | `annulus_mask_has_a_row_per_q_node_and_a_column_per_g_node` |
| `test_matches_brute_force` | `annulus_mask_matches_brute_force` |
| `test_translation_invariant` | `annulus_mask_ignores_a_shift_of_q` |
| `test_anchor_row_matches_near_g_anchor` | `annulus_mask_anchor_row_is_the_g_nodes_near_the_g_anchor` |
| `test_boundary_is_inclusive` | `annulus_mask_boundary_is_inclusive` |
| `test_tol_zero_requires_exact_distance` | `annulus_mask_with_tol_zero_needs_equal_distances` |
| `test_nonzero_anchor_indices` | `annulus_mask_with_other_reference_nodes` |
| `test_empty_Q_arr_raises`, `test_empty_G_arr_raises` | Not ported. A reference index into an empty slice is a bug, and it panics (D3). A graph from a file or a crop always has a node |
| `TestMakeCandidates` (7 tests) | 6 tests on `candidates`. `test_combines_every_mask_in_the_tuple` is covered by `finds_a_reflected_pose` |
| `TestCrossSigns` (5 tests) | The `side_of` tests of Slice 2, plus `sides_about_the_axis` |
| `TestSplitMaskByHandedness` (5 tests) | 4 tests. `test_outputs_keep_input_shape` is covered by the others: they index every cell |
| `TestGetCandidateMatches` (6 tests) | 5 tests, plus `single_node_query_has_no_pose`. `test_returns_node_ids_not_indices` is not needed, because a pose holds positions |

---

## Steps

### 1. The module and `Graph::coords`

- [ ] Add a getter inside `impl Graph` in `src/graph.rs`, after `neighbors`:

  ```rust
      /// The coordinates of every node, in position order.
      pub fn coords(&self) -> &[Point] {
          &self.coords
      }
  ```

  This lends the whole coordinate list as a slice, as `neighbors` lends one adjacency list
  (Slice 3, Step 7). The mask code works on `&[Point]`, like the Python code works on arrays.

- [ ] Make `src/align_nodes.rs`:

  ```rust
  use crate::geometry::Point;

  /// One row per node of Q, one column per node of G.
  pub type Mask = Vec<Vec<bool>>;

  /// A candidate match: `pose[i]` is the position in G of the node matched to Q position `i`.
  pub type Pose = Vec<usize>;
  ```

  Both are type aliases (Slice 1, Step 3). `Mask` and `Vec<Vec<bool>>` are the same type, so
  every `Vec` method works on a `Mask`. The alias only gives the type a name that says what it
  holds.

  Why "pose" and not "match"? `match` is a keyword in Rust, so it cannot be a variable name.

- [ ] Add `pub mod align_nodes;` to `src/lib.rs`, above `pub mod geometry;`.

- [ ] Run `cargo build`. Expected: it builds. You can get a warning about the unused import
  `Point`.

### 2. The annulus mask tests

- [ ] Add a stub to `src/align_nodes.rs`:

  ```rust
  /// `mask[i][j]` is true when node `i` of Q and node `j` of G are at the same distance
  /// from their reference nodes, within `tol`.
  fn annulus_mask(q: &[Point], g: &[Point], q_ref: usize, g_ref: usize, tol: f64) -> Mask {
      todo!()
  }
  ```

  It is private: only `get_candidate_matches` calls it. The tests can still call it, because
  `mod tests` is inside this module and `use super::*` brings in private items too.

  `q_ref` and `g_ref` are "reference" nodes, not anchors. The function is called with the
  anchors and again with the alignment nodes.

- [ ] Add the test module at the end of the file:

  ```rust
  #[cfg(test)]
  mod tests {
      use super::*;

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

      /// The annulus mask, found with two plain loops and `hypot`.
      fn brute_force_mask(q: &[Point], g: &[Point], q_ref: usize, g_ref: usize, tol: f64) -> Mask {
          let mut mask = vec![vec![false; g.len()]; q.len()];
          for i in 0..q.len() {
              let d1 = (q[i].x - q[q_ref].x).hypot(q[i].y - q[q_ref].y);
              for j in 0..g.len() {
                  let d2 = (g[j].x - g[g_ref].x).hypot(g[j].y - g[g_ref].y);
                  mask[i][j] = (d1 - d2).abs() <= tol;
              }
          }
          mask
      }
  }
  ```

  1. A `const` cannot call `Point::new`, because `new` is a normal function. It can use the
     struct literal `Point { x, y }`, because the fields are `pub`. Your geometry tests do the
     same with `A` and `B`.
  2. `vec![vec![false; g.len()]; q.len()]` is `vec![x; n]` (Slice 4, Step 10) twice: `q.len()`
     copies of a row of `g.len()` `false` values. It is `np.zeros((len(Q), len(G)), bool)`.
     `vec![x; n]` clones `x` for each copy, so every row is its own `Vec`. (The Python trap
     `[[False] * m] * n`, where every row is the same list, cannot happen here.)
  3. `brute_force_mask` uses `hypot`, and the code under test will use `Point::dist`. The two
     are different code, so they check each other, as in the Python test.

- [ ] Add the seven tests inside `mod tests`, after `brute_force_mask`:

  ```rust
      #[test]
      fn annulus_mask_has_a_row_per_q_node_and_a_column_per_g_node() {
          let mask = annulus_mask(&Q, &G, 0, 0, 0.5);

          assert_eq!(mask.len(), 3);
          assert!(mask.iter().all(|row| row.len() == 5));
      }

      #[test]
      fn annulus_mask_matches_brute_force() {
          let mask = annulus_mask(&Q, &G, 0, 0, 0.5);

          // every cell agrees with the plain loops
          assert_eq!(mask, brute_force_mask(&Q, &G, 0, 0, 0.5));
      }

      #[test]
      fn annulus_mask_ignores_a_shift_of_q() {
          let shifted: Vec<Point> = Q.iter().map(|&p| p + Point::new(50.0, -20.0)).collect();

          let mask = annulus_mask(&shifted, &G, 0, 0, 0.5);

          // only distances to the reference node count, not positions
          assert_eq!(mask, annulus_mask(&Q, &G, 0, 0, 0.5));
      }

      #[test]
      fn annulus_mask_anchor_row_is_the_g_nodes_near_the_g_anchor() {
          let mask = annulus_mask(&Q, &G, 0, 0, 0.5);

          // Q[0] is at distance 0, and only G[0] is within 0.5 of distance 0
          assert_eq!(mask[0], vec![true, false, false, false, false]);
      }

      #[test]
      fn annulus_mask_boundary_is_inclusive() {
          // Q[1] is at distance 3 and G[3] at distance 2: the difference is exactly 1
          let mask = annulus_mask(&Q, &G, 0, 0, 1.0);

          assert!(mask[1][3]);
      }

      #[test]
      fn annulus_mask_with_tol_zero_needs_equal_distances() {
          let mask = annulus_mask(&Q, &G, 0, 0, 0.0);

          assert_eq!(mask, brute_force_mask(&Q, &G, 0, 0, 0.0));
          // the columns at distance 2 and 100 match no node of Q
          assert!(mask.iter().all(|row| !row[3] && !row[4]));
      }

      #[test]
      fn annulus_mask_with_other_reference_nodes() {
          // Q[1] and G[1] as the references
          let mask = annulus_mask(&Q, &G, 1, 1, 0.0);

          assert_eq!(mask, brute_force_mask(&Q, &G, 1, 1, 0.0));
          // each reference node is at distance 0 from itself
          assert!(mask[1][1]);
      }
  ```

  1. `annulus_mask(&Q, &G, ...)`: `&Q` borrows the array, and Rust turns a `&[Point; 3]` into
     a `&[Point]` by itself. This is why a function that takes a slice accepts an array, a
     `Vec` and part of a `Vec`.
  2. `assert_eq!(mask, ...)` compares two `Vec<Vec<bool>>` cell by cell. In numpy, `==` gives
     an array, and you need `np.array_equal`. In Rust, `==` on two `Vec`s gives one `bool`.

- [ ] Run `cargo test align`. Expected: `0 passed; 7 failed`, all with `not yet implemented`.

### 3. Write `annulus_mask`

**Try first.** Compute the distance of each `G` node to `g[g_ref]` once, into a `Vec<f64>`.
Then build one row for each `Q` node: its distance to `q[q_ref]`, compared with each `G`
distance. Use `Point::dist` and two nested `map`/`collect`s.

**Suggested code.**

```rust
fn annulus_mask(q: &[Point], g: &[Point], q_ref: usize, g_ref: usize, tol: f64) -> Mask {
    let g_dists: Vec<f64> = g.iter().map(|&p| p.dist(g[g_ref])).collect();

    q.iter()
        .map(|&p| {
            let q_dist = p.dist(q[q_ref]);
            g_dists
                .iter()
                .map(|&g_dist| (q_dist - g_dist).abs() <= tol)
                .collect()
        })
        .collect()
}
```

**Explanation.**

1. `g_dists` is computed before the loop over `Q`. Inside the loop it would be computed again
   for every row. numpy does the same with `G_dists[None, :]`: one row of distances, used by
   every row of `Q`.
2. `|&p|` copies each `Point` out of the reference (`Point` is `Copy`, Slice 2, Step 2).
   `g[g_ref]` is a copy too. This is cheap: two `f64`s.
3. The inner `collect` builds one `Vec<bool>` (a row), and the outer `collect` builds the
   `Vec<Vec<bool>>`. The return type `Mask` tells both `collect`s what to build. This is the
   shape of `subgraph` in Slice 4, Step 3.
4. `<= tol` keeps a difference of exactly `tol`, like the Python `<=`.
5. Is this as fast as numpy? Yes, or faster. numpy makes three temporary arrays here
   (`Q_dists[:, None] - G_dists[None, :]`, then `abs`, then `<=`). The loop makes no
   temporary at all (decision D4).

- [ ] Run `cargo test align`. Expected: `7 passed; 0 failed`.

### 4. The `candidates` tests

`candidates(mask)` lists every pose that one mask allows. A pose takes one `True` column from
each row, and no column twice.

- [ ] Add a stub after `annulus_mask`:

  ```rust
  /// Every pose that `mask` allows.
  fn candidates(mask: &Mask) -> Vec<Pose> {
      todo!()
  }
  ```

- [ ] Add six tests at the end of `mod tests`:

  ```rust
      #[test]
      fn one_way_to_assign() {
          let mask = vec![vec![true, false], vec![false, true]];

          assert_eq!(candidates(&mask), vec![vec![0, 1]]);
      }

      #[test]
      fn every_injective_assignment_is_found() {
          let mask = vec![vec![true, true], vec![true, true]];

          // both one-to-one assignments, and nothing else
          assert_eq!(candidates(&mask), vec![vec![0, 1], vec![1, 0]]);
      }

      #[test]
      fn a_column_is_not_used_twice() {
          // both rows can only use column 0
          let mask = vec![vec![true, false], vec![true, false]];

          assert!(candidates(&mask).is_empty());
      }

      #[test]
      fn a_row_with_no_match_gives_nothing() {
          let mask = vec![vec![false, false], vec![true, false]];

          assert!(candidates(&mask).is_empty());
      }

      #[test]
      fn more_columns_than_rows() {
          let mask = vec![vec![true, true, false], vec![false, true, true]];

          // every pick of two different columns, in row order
          assert_eq!(candidates(&mask), vec![vec![0, 1], vec![0, 2], vec![1, 2]]);
      }

      #[test]
      fn one_row_gives_one_pose_per_match() {
          let mask = vec![vec![true, false, true]];

          assert_eq!(candidates(&mask), vec![vec![0], vec![2]]);
      }
  ```

  The Python tests compare ids, and turn the result into a `set` when the order is not fixed.
  Here a pose holds positions, and the order is fixed: the search tries the columns of each row
  from left to right. So the tests compare the `Vec` directly.

- [ ] Run `cargo test align`. Expected: `7 passed; 6 failed`.

### 5. Write `candidates`

`candidates` has two parts. First it turns each row into the list of its `True` columns (the
Python `np.flatnonzero(row)`). Then it lists the injective assignments of those lists.

**The Python version is a recursive generator.** `backtrack(i)` tries each column of row `i`,
and calls itself for row `i + 1`. When `i` gets to the last row, it `yield`s the pose. Rust (in
the stable language) has no generators, and a closure cannot call itself, because a closure has
no name that its own body can use. So the Rust version is a normal function that calls itself,
and it pushes each finished pose into a `Vec` that the caller gives it.

**Try first.** Write two functions:

```rust
/// Every way to pick one column from each row, with no column used twice.
fn injective_assignments(row_matches: &[Vec<usize>]) -> Vec<Pose>

/// Add to `found` every assignment that starts with `pose`.
fn extend_assignments(row_matches: &[Vec<usize>], pose: &mut Pose, found: &mut Vec<Pose>)
```

The first makes an empty `pose` and an empty `found`, calls the second, and returns `found`.
The second is the Python `backtrack`. It needs no `i`, because the next row is `pose.len()`.

**Suggested code.** Start without the "no column twice" check, and let the tests show why it is
needed.

```rust
/// Every pose that `mask` allows.
fn candidates(mask: &Mask) -> Vec<Pose> {
    let row_matches: Vec<Vec<usize>> = mask
        .iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .filter(|&(_, &hit)| hit)
                .map(|(j, _)| j)
                .collect()
        })
        .collect();
    injective_assignments(&row_matches)
}

/// Every way to pick one column from each row, with no column used twice.
fn injective_assignments(row_matches: &[Vec<usize>]) -> Vec<Pose> {
    let mut found = Vec::new();
    let mut pose = Vec::with_capacity(row_matches.len());
    extend_assignments(row_matches, &mut pose, &mut found);
    found
}

/// Add to `found` every assignment that starts with `pose`.
fn extend_assignments(row_matches: &[Vec<usize>], pose: &mut Pose, found: &mut Vec<Pose>) {
    let row = pose.len();
    if row == row_matches.len() {
        found.push(pose.clone());
        return;
    }

    for &j in &row_matches[row] {
        pose.push(j);
        extend_assignments(row_matches, pose, found);
        pose.pop();
    }
}
```

**Explanation.**

1. In `candidates`, `enumerate` gives `(column, &cell)` pairs, and `filter` lends each pair
   again. So the pattern `|&(_, &hit)|` takes the pair out of the outer `&`, ignores the
   column with `_`, and copies the `bool` out of the inner `&`. Then `map` keeps only the
   column. This is `filter_map` in two steps; both forms are common.
2. `injective_assignments` owns `found` and `pose`, and lends both to `extend_assignments` with
   `&mut` (Slice 3, Step 3). There is one `pose` for the whole search. Each level pushes a
   column, lets the deeper levels work, and pops it again. This is the `combo.append` /
   `combo.pop()` of the Python code.
3. Inside `extend_assignments`, `pose` is already a `&mut Pose`. To pass it down, you write
   `pose`, not `&mut pose`. Rust lends it on for the length of the call (a **reborrow**), and
   `pose` is usable again after the call returns. So the `pose.pop()` on the next line is
   allowed.
4. `found.push(pose.clone())` stores a copy. `pose` itself keeps changing, so `found` cannot
   hold `pose`. In Python, `tuple(combo)` makes the copy for the same reason.
5. `for &j in &row_matches[row]`: `&row_matches[row]` borrows the row, the loop gives
   `&usize` items, and `&j` copies each one out.

- [ ] Run `cargo test align`. Expected: `10 passed; 3 failed`. One of the failures:

  ```text
  ---- align_nodes::tests::every_injective_assignment_is_found stdout ----
  assertion `left == right` failed
    left: [[0, 0], [0, 1], [1, 0], [1, 1]]
   right: [[0, 1], [1, 0]]
  ```

  This version is `itertools.product(*row_matches)`: every combination, also those that use
  column 0 twice. `a_column_is_not_used_twice` and `more_columns_than_rows` fail for the same
  reason.

- [ ] Add the check at the top of the `for` loop in `extend_assignments`:

  ```rust
      for &j in &row_matches[row] {
          if pose.contains(&j) {
              continue;
          }
          pose.push(j);
          extend_assignments(row_matches, pose, found);
          pose.pop();
      }
  ```

  `contains` looks at each value of `pose`, like `j in combo` on a Python list. Python keeps a
  separate `used` set for this. Here `pose` has at most 20 values (one per query node), and a
  scan of 20 numbers is faster than a hash lookup. It also means there is no second structure
  to keep in step with `pose`.

  Because the check is at each level, a branch that uses a column twice stops at once. It does
  not build the rest of the pose first. This is the point of the backtracking version, as the
  Python docstring says.

- [ ] Add an early exit at the top of `candidates`, and its helper after `candidates`:

  ```rust
      if !every_row_has_a_match(mask) {
          return Vec::new();
      }
  ```

  ```rust
  fn every_row_has_a_match(mask: &Mask) -> bool {
      mask.iter().all(|row| row.iter().any(|&hit| hit))
  }
  ```

  This is the Python `mask.any(axis=1).all()`. The search would find nothing for a mask with an
  empty row anyway, but only after it tries every combination of the rows before it.
  `get_candidate_matches` uses the helper too.

- [ ] Run `cargo test align`. Expected: `13 passed; 0 failed`.

### 6. The handedness tests

- [ ] Change the first line of `src/align_nodes.rs` to:

  ```rust
  use crate::geometry::{COLLINEAR_TOL, Point, Side, side_of};
  ```

- [ ] Add two stubs after `every_row_has_a_match`:

  ```rust
  /// The side of every point about the line from `points[from]` to `points[to]`.
  fn sides(points: &[Point], from: usize, to: usize) -> Vec<Side> {
      todo!()
  }

  /// Split `mask` into (rotation, reflection) by the side of each node about its axis.
  fn split_by_handedness(mask: &Mask, q_sides: &[Side], g_sides: &[Side]) -> (Mask, Mask) {
      todo!()
  }
  ```

  The split returns two masks as a tuple (Slice 3, Step 4), like the Python function.

- [ ] Add the shared data, a helper and five tests at the end of `mod tests`:

  ```rust
      // The axis from node 0 to node 1 points along +x. Node 2 is on its left, node 3 on
      // its right. G has the same shape, far away.
      const Q_HANDED: [Point; 4] = [
          Point { x: 0.0, y: 0.0 },
          Point { x: 2.0, y: 0.0 },
          Point { x: 1.0, y: 1.0 },
          Point { x: 1.0, y: -1.0 },
      ];
      const G_HANDED: [Point; 4] = [
          Point { x: 100.0, y: 100.0 },
          Point { x: 102.0, y: 100.0 },
          Point { x: 101.0, y: 101.0 },
          Point { x: 101.0, y: 99.0 },
      ];

      /// Split a full 4 x 4 mask by the sides of `Q_HANDED` and `G_HANDED`.
      fn split_full_mask() -> (Mask, Mask) {
          let full = vec![vec![true; 4]; 4];
          let q_sides = sides(&Q_HANDED, 0, 1);
          let g_sides = sides(&G_HANDED, 0, 1);
          split_by_handedness(&full, &q_sides, &g_sides)
      }

      #[test]
      fn sides_about_the_axis() {
          let result = sides(&Q_HANDED, 0, 1);

          // the two ends of the axis are on it
          assert_eq!(result, vec![Side::On, Side::On, Side::Left, Side::Right]);
      }

      #[test]
      fn same_side_to_rotation_opposite_side_to_reflection() {
          let (rotation, reflection) = split_full_mask();

          // the left node of Q
          assert!(rotation[2][2] && !rotation[2][3]);
          assert!(reflection[2][3] && !reflection[2][2]);
          // the right node of Q
          assert!(rotation[3][3] && !rotation[3][2]);
          assert!(reflection[3][2] && !reflection[3][3]);
      }

      #[test]
      fn nodes_on_the_axis_go_to_both_masks() {
          let (rotation, reflection) = split_full_mask();

          for k in 0..4 {
              for on_axis in [0, 1] {
                  assert!(rotation[on_axis][k] && reflection[on_axis][k]);
                  assert!(rotation[k][on_axis] && reflection[k][on_axis]);
              }
          }
      }

      #[test]
      fn the_masks_cover_the_input_and_overlap_only_on_the_axis() {
          let (rotation, reflection) = split_full_mask();

          for i in 0..4 {
              for j in 0..4 {
                  // every cell of the full mask is in one of the masks
                  assert!(rotation[i][j] || reflection[i][j]);
                  // a cell is in both only if one of its nodes is on the axis
                  let on_axis = i < 2 || j < 2;
                  assert_eq!(rotation[i][j] && reflection[i][j], on_axis);
              }
          }
      }

      #[test]
      fn the_split_never_adds_a_match() {
          let mut mask = vec![vec![false; 4]; 4];
          mask[2][2] = true;
          let q_sides = sides(&Q_HANDED, 0, 1);
          let g_sides = sides(&G_HANDED, 0, 1);

          let (rotation, reflection) = split_by_handedness(&mask, &q_sides, &g_sides);

          let count = |m: &Mask| m.iter().flatten().filter(|&&hit| hit).count();
          assert!(rotation[2][2]);
          assert_eq!(count(&rotation), 1);
          assert_eq!(count(&reflection), 0);
      }
  ```

  1. `let (rotation, reflection) = split_full_mask();` takes the tuple apart, as in Python.
  2. `for on_axis in [0, 1]` loops over an array literal, like `for x in (0, 1)` in Python.
  3. In the last test, `flatten` turns the rows into one stream of cells (like
     `itertools.chain.from_iterable`), and `count` counts the items that are left. Together
     they are the numpy `mask.sum()`. The closure has a type on its parameter, `m: &Mask`,
     because nothing else tells the compiler what `m` is.

- [ ] Run `cargo test align`. Expected: `13 passed; 5 failed`.

### 7. Write `sides` and `split_by_handedness`

**Try first.** `sides` is one `map` over the points with `side_of(p, points[from], points[to],
COLLINEAR_TOL)`. For the split, start with a copy of `mask` for each result, and loop over all
cells: a cell stays `true` in `rotation` if the two nodes "have the same hand", and in
`reflection` if they have opposite hands. Write the rule as a small function
`same_hand(q: Side, g: Side) -> bool`.

**Suggested code.** Start with the obvious rule, "the same side":

```rust
/// The side of every point about the line from `points[from]` to `points[to]`.
fn sides(points: &[Point], from: usize, to: usize) -> Vec<Side> {
    points
        .iter()
        .map(|&p| side_of(p, points[from], points[to], COLLINEAR_TOL))
        .collect()
}

/// Can a node on side `q` of the Q axis match a node on side `g` of the G axis,
/// without a reflection?
fn same_hand(q: Side, g: Side) -> bool {
    q == g
}

/// Split `mask` into (rotation, reflection) by the side of each node about its axis.
fn split_by_handedness(mask: &Mask, q_sides: &[Side], g_sides: &[Side]) -> (Mask, Mask) {
    let mut rotation = mask.clone();
    let mut reflection = mask.clone();

    for (i, &q_side) in q_sides.iter().enumerate() {
        for (j, &g_side) in g_sides.iter().enumerate() {
            rotation[i][j] &= same_hand(q_side, g_side);
            reflection[i][j] &= same_hand(q_side, g_side.flipped());
        }
    }
    (rotation, reflection)
}
```

**Explanation.**

1. `mask.clone()` copies the whole table, every row (Slice 2, Step 3). The function borrows
   `mask` and returns two new masks, so it needs two copies.
2. `rotation[i][j] &= x` is `rotation[i][j] = rotation[i][j] & x`. On `bool`, `&` is "and",
   like the numpy `mask & same`. A cell can only go from `true` to `false`, so the split never
   adds a match.
3. A reflection is a rotation of the mirror image. Mirror `G` and every side of `G` flips. So
   "opposite hands" is `same_hand(q_side, g_side.flipped())`, and the rule is written once.
   `flipped` keeps `On` as `On` (Slice 2).

- [ ] Run `cargo test align`. Expected: `16 passed; 2 failed`.

  ```text
  ---- align_nodes::tests::nodes_on_the_axis_go_to_both_masks stdout ----
  assertion failed: rotation[on_axis][k] && reflection[on_axis][k]
  ---- align_nodes::tests::the_masks_cover_the_input_and_overlap_only_on_the_axis stdout ----
  assertion failed: rotation[i][j] || reflection[i][j]
  ```

  A node on the axis has no hand. The Python code multiplies the signs, and `0 * x` is `0`,
  which passes both `>= 0` and `<= 0`. With `q == g`, an `On` node only matches another `On`
  node, so the cell of an `On` node and a `Left` node is in neither mask.

- [ ] Replace `same_hand`:

  ```rust
  fn same_hand(q: Side, g: Side) -> bool {
      match (q, g) {
          (Side::On, _) | (_, Side::On) => true,
          _ => q == g,
      }
  }
  ```

  1. `match (q, g)` matches a tuple, so one arm can look at both values. Python 3.10's
     `match (q, g): case (On, _) | (_, On):` is the same idea.
  2. `_` matches any value. `|` joins two patterns into one arm: "`q` is `On`, or `g` is `On`".
  3. The last arm `_` catches every other case, so the `match` covers all nine pairs, and the
     compiler is satisfied (Slice 2, Step 14).

- [ ] Run `cargo test align`. Expected: `18 passed; 0 failed`.

### 8. The `get_candidate_matches` tests

- [ ] Add the stub at the top of the file, after the two type aliases. It is the only public
  function of the module, so it goes first. Also add `use crate::graph::Graph;`.

  ```rust
  /// Candidate poses that put `q_anchor` of Q on `g_anchor` of G.
  pub fn get_candidate_matches(
      q: &Graph,
      g: &Graph,
      q_anchor: usize,
      g_anchor: usize,
      tol: f64,
  ) -> Vec<Pose> {
      todo!()
  }
  ```

  In the search, both anchors are position `0` (Slice 4). The parameters stay, so that the
  function says which nodes it aligns, and so that it is the same as the Python function.

- [ ] Add to the `use` lines of `mod tests`:

  ```rust
      use crate::io::NodeId;
      use crate::test_fixtures::file;
  ```

- [ ] Add the data, three helpers and six tests at the end of `mod tests`:

  ```rust
      // A 3-4-5 triangle. Its three distances are all different, so it has only one
      // correct pose. The ids are not positions.
      const TRIANGLE: [(NodeId, f64, f64); 3] = [(10, 0.0, 0.0), (20, 3.0, 0.0), (30, 0.0, 4.0)];

      fn graph(nodes: &[(NodeId, f64, f64)]) -> Graph {
          Graph::from_file(&file(nodes, &[])).unwrap()
      }

      /// The ids of a pose, as (Q id, G id) pairs.
      fn id_pairs(q: &Graph, g: &Graph, pose: &Pose) -> Vec<(NodeId, NodeId)> {
          pose.iter()
              .enumerate()
              .map(|(i, &j)| (q.id(i), g.id(j)))
              .collect()
      }

      /// True if one of `poses` maps the Q ids to the G ids as in `expected`.
      fn has_pose(q: &Graph, g: &Graph, poses: &[Pose], expected: &[(NodeId, NodeId)]) -> bool {
          poses.iter().any(|pose| id_pairs(q, g, pose) == expected)
      }

      #[test]
      fn finds_the_true_pose() {
          // the triangle, moved far away, and one more node that fits nowhere
          let q = graph(&TRIANGLE);
          let g = graph(&[
              (101, 100.0, 100.0),
              (102, 103.0, 100.0),
              (103, 100.0, 104.0),
              (104, 500.0, 500.0),
          ]);

          let poses = get_candidate_matches(&q, &g, 0, 0, 0.5);

          assert!(has_pose(&q, &g, &poses, &[(10, 101), (20, 102), (30, 103)]));
          // node 104 (position 3) is in no pose
          assert!(poses.iter().all(|pose| !pose.contains(&3)));
      }

      #[test]
      fn finds_a_rotated_pose() {
          // the triangle turned by 90 degrees: (x, y) -> (-y, x)
          let q = graph(&TRIANGLE);
          let g = graph(&[(101, 100.0, 100.0), (102, 100.0, 103.0), (103, 96.0, 100.0)]);

          let poses = get_candidate_matches(&q, &g, 0, 0, 0.5);

          assert!(has_pose(&q, &g, &poses, &[(10, 101), (20, 102), (30, 103)]));
      }

      #[test]
      fn finds_a_reflected_pose() {
          // the triangle mirrored: (x, y) -> (x, -y)
          let q = graph(&TRIANGLE);
          let g = graph(&[(101, 100.0, 100.0), (102, 103.0, 100.0), (103, 100.0, 96.0)]);

          let poses = get_candidate_matches(&q, &g, 0, 0, 0.5);

          assert!(has_pose(&q, &g, &poses, &[(10, 101), (20, 102), (30, 103)]));
      }

      #[test]
      fn query_larger_than_target_has_no_pose() {
          let q = graph(&TRIANGLE);
          let g = graph(&[(101, 100.0, 100.0), (102, 103.0, 100.0)]);

          let poses = get_candidate_matches(&q, &g, 0, 0, 0.5);

          assert!(poses.is_empty());
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

          assert!(poses.is_empty());
      }

      #[test]
      fn single_node_query_has_no_pose() {
          let q = graph(&[(10, 0.0, 0.0)]);
          let g = graph(&[(101, 100.0, 100.0)]);

          let poses = get_candidate_matches(&q, &g, 0, 0, 0.5);

          assert!(poses.is_empty());
      }
  ```

  1. The graphs have no edges. Node matching only uses coordinates, as the Python docstring
     says ("This ignores topology").
  2. `id_pairs` turns a pose back into ids, so the tests can state the expected pose with the
     same ids as the Python tests. Position `i` of the pose is Q position `i`, so `enumerate`
     gives the Q side of each pair.
  3. `id_pairs(...) == expected` compares a `Vec<(NodeId, NodeId)>` with a
     `&[(NodeId, NodeId)]`. Rust allows `==` between a `Vec` and a slice of the same item
     type.

- [ ] Run `cargo test align`. Expected: `18 passed; 6 failed`.

### 9. Write `get_candidate_matches`

**Try first.** Follow "What the Python code does", steps 1 to 5. You need one more helper:

```rust
/// The cells that are true in both masks.
fn and(a: &Mask, b: &Mask) -> Mask
```

Two iterator methods help with step 3. `count()` counts the items of an iterator.
`min_by_key(f)` returns the item with the smallest `f(item)`, as an `Option` (it is `None` for
an empty iterator). If several items have the same smallest key, it returns the **first** one,
the same as `np.argmin`. (`max_by_key` returns the **last** one. Check the docs when a tie
matters.)

**Suggested code.** Add `and` after `every_row_has_a_match`:

```rust
/// The cells that are true in both masks.
fn and(a: &Mask, b: &Mask) -> Mask {
    a.iter()
        .zip(b)
        .map(|(row_a, row_b)| row_a.iter().zip(row_b).map(|(&x, &y)| x && y).collect())
        .collect()
}
```

`zip` (Slice 3, Step 10) walks two rows together. `zip(b)` takes `b` itself: a `&Vec` can be
iterated, and gives references. The name `and` is allowed, because Rust has no `and` keyword
(it uses `&&`).

Then replace the stub of `get_candidate_matches`:

```rust
pub fn get_candidate_matches(
    q: &Graph,
    g: &Graph,
    q_anchor: usize,
    g_anchor: usize,
    tol: f64,
) -> Vec<Pose> {
    let q_points = q.coords();
    let g_points = g.coords();

    // every node of Q needs its own node of G
    if q_points.len() > g_points.len() {
        return Vec::new();
    }

    let anchor_mask = annulus_mask(q_points, g_points, q_anchor, g_anchor, tol);
    if !every_row_has_a_match(&anchor_mask) {
        return Vec::new();
    }

    // the node of Q, other than the anchor, with the fewest matches
    let counts: Vec<usize> = anchor_mask
        .iter()
        .map(|row| row.iter().filter(|&&hit| hit).count())
        .collect();
    let Some(q_align) = (0..q_points.len())
        .filter(|&i| i != q_anchor)
        .min_by_key(|&i| counts[i])
    else {
        return Vec::new();
    };
    let q_sides = sides(q_points, q_anchor, q_align);

    let mut poses = Vec::new();
    for g_align in 0..g_points.len() {
        if !anchor_mask[q_align][g_align] {
            continue;
        }

        let align_mask = annulus_mask(q_points, g_points, q_align, g_align, tol);
        let both = and(&anchor_mask, &align_mask);
        let g_sides = sides(g_points, g_anchor, g_align);
        let (rotation, reflection) = split_by_handedness(&both, &q_sides, &g_sides);

        poses.extend(candidates(&rotation));
        poses.extend(candidates(&reflection));
    }
    poses
}
```

**Explanation.**

1. Python sets the count of the anchor to `inf` so that `argmin` skips it. Here `filter` drops
   the anchor before `min_by_key` looks at the counts. No sentinel value is needed, and the
   counts can stay `usize`.
2. `let Some(q_align) = ... else { return Vec::new(); };` is **`let ... else`**. If the value
   matches the pattern, `q_align` is set and the function goes on. If not, the `else` block
   runs, and it must leave the function (with `return`, or a panic). Here the `else` runs only
   for a one-node query: after the anchor is dropped, there is nothing left (see "The design").
   It is shorter than a `match` with two arms, and `q_align` is a plain `usize` afterwards.
3. `q_sides` is computed once, before the loop. The axis of `Q` does not change. The axis of
   `G` changes with `g_align`, so `g_sides` is computed inside the loop.
4. The loop goes over every column, and `continue`s past the ones that are `false` in the row
   of the alignment node. This is the Python `np.where(anchor_mask[Q_alignment_idx])[0]`.
5. `poses.extend(...)` appends every item of another collection, like `list.extend`.
6. Rotation poses come before reflection poses for each `g_align`, as in Python. If a pose is in
   both masks (every node on the axis), it is in the list twice, as in Python.

- [ ] Run `cargo test align`. Expected: `24 passed; 0 failed`.
- [ ] Run `cargo test`. Expected: `75 passed; 0 failed`.

### 10. Look at it

- [ ] Add a temporary test at the end of `mod tests`:

  ```rust
      #[test]
      fn look() {
          let q = graph(&TRIANGLE);
          let g = graph(&[(101, 100.0, 100.0), (102, 103.0, 100.0), (103, 100.0, 96.0)]);

          let anchor_mask = annulus_mask(q.coords(), g.coords(), 0, 0, 0.5);
          let q_sides = sides(q.coords(), 0, 1);
          let g_sides = sides(g.coords(), 0, 1);
          let (rotation, reflection) = split_by_handedness(&anchor_mask, &q_sides, &g_sides);

          println!("sides: Q {q_sides:?}, G {g_sides:?}");
          println!("anchor: {anchor_mask:?}");
          println!("rotation: {rotation:?}");
          println!("reflection: {reflection:?}");
          println!("poses: {:?}", get_candidate_matches(&q, &g, 0, 0, 0.5));
      }
  ```

  This is the mirrored triangle of `finds_a_reflected_pose`. For this graph, the mask around the
  alignment pair is the same as the anchor mask, so the test splits the anchor mask directly.

- [ ] Run `cargo test look -- --nocapture` (Slice 4, Step 12). Expected:

  ```text
  sides: Q [On, On, Left], G [On, On, Right]
  anchor: [[true, false, false], [false, true, false], [false, false, true]]
  rotation: [[true, false, false], [false, true, false], [false, false, false]]
  reflection: [[true, false, false], [false, true, false], [false, false, true]]
  poses: [[0, 1, 2]]
  ```

  Node `30` of `Q` is on the left of its axis, and node `103` of `G` is on the right. So the
  rotation mask loses that cell, and its last row is empty: no rotation pose. The reflection mask
  keeps it, and gives the one pose `[0, 1, 2]`, which is `10 → 101, 20 → 102, 30 → 103`. The
  first two rows are in both masks, because those nodes are on the axis.

- [ ] Delete the `look` test.

### 11. Use it from `main`

The program crops `G` around every node, as in Slice 4, and now also finds the poses in each
crop.

**Read this before you run it.** The number of poses for one anchor can be very large. It is
about the product of the row sizes of the mask, so it grows very fast with the number of query
nodes and with the crop size. Every pose is stored in a `Vec` before it is counted, so a large
count uses a lot of memory. The Python code has the same behaviour (it also builds the full
list). Run it first on `gr00049` (10 query nodes, crops of about 13 nodes). Keep Task Manager
open, and stop the program with `Ctrl+C` if the memory use climbs. Do not run `gr00048` (19
query nodes, crops of about 100 nodes) or `gr00047` until Slice 9, which limits this.

- [ ] Replace all of `src/main.rs`:

  ```rust
  use std::error::Error;
  use std::time::Instant;

  use gsearch::align_nodes::get_candidate_matches;
  use gsearch::graph::Graph;
  use gsearch::io::read_graph_file;
  use gsearch::spatial_index::SpatialIndex;

  /// The default tolerance of the Python CLI. Slice 8 makes it an argument.
  const TOL: f64 = 10.0;

  fn main() -> Result<(), Box<dyn Error>> {
      let usage = "usage: gsearch <query.json> <graph.json>";
      let mut args = std::env::args().skip(1);
      let query_path = args.next().ok_or(usage)?;
      let graph_path = args.next().ok_or(usage)?;

      let query_file = read_graph_file(&query_path)?;
      let query = Graph::from_file(&query_file)?;
      let radius = query.radius_from(0) + TOL;
      println!(
          "{}: {} nodes, crop radius {:.4}",
          query_file.graph_id,
          query.node_count(),
          radius,
      );

      let graph_file = read_graph_file(&graph_path)?;
      let index = SpatialIndex::new(Graph::from_file(&graph_file)?);
      let graph = index.graph();
      println!(
          "{}: {} nodes, {} edges",
          graph_file.graph_id,
          graph.node_count(),
          graph.edge_count(),
      );

      let start = Instant::now();
      let mut total = 0;
      let mut anchors_with_a_pose = 0;
      let mut most = 0;
      for anchor in 0..graph.node_count() {
          let crop = index.make_crop(anchor, radius);
          let poses = get_candidate_matches(&query, &crop, 0, 0, TOL);
          total += poses.len();
          if !poses.is_empty() {
              anchors_with_a_pose += 1;
          }
          most = most.max(poses.len());
      }
      eprintln!("match every node: {:.3} s", start.elapsed().as_secs_f64());
      println!("anchors with a pose: {anchors_with_a_pose}");
      println!("poses: {total} in total, {most} at most for one anchor");
      Ok(())
  }
  ```

  1. Slice 4, Step 13 explains the argument code, `ok_or(usage)?` and the casts. This version
     does not keep the graph in its own variable: `Graph::from_file(...)?` goes straight into
     `SpatialIndex::new`, so there is no moved name left to use by mistake.
  2. `get_candidate_matches(&query, &crop, 0, 0, TOL)`: both anchors are position `0`. The crop
     radius is `Q_radius + tol`, and the match tolerance is `tol`, as in `run_gsearch`.
  3. `most.max(poses.len())` is the larger of two numbers. `max` is a method on `usize`.
  4. `poses` is dropped at the end of each loop step, so only one anchor's poses are in memory
     at a time.

- [ ] Run, in release mode (Slice 0, Step 7):

  ```powershell
  cargo run --release -- data\roads\queries\gr00049_q.json data\roads\graphs\gr00049.json
  ```

  Expected: the first two lines are the same as in Slice 4 (`gr00049_q: 10 nodes, crop radius
  68.0499` and `gr00049: 1128 nodes, 1205 edges`). Write down the other three numbers.

### 12. Compare with Python

The total number of poses does not depend on the order of the nodes in a crop, so it must be the
same in both languages. From the `gsearch` folder, with the same memory warning as Step 11:

```powershell
cd ..\gsearch
$check = @'
import sys, time
from gsearch.align_nodes import get_candidate_matches
from gsearch.core import _get_radius
from gsearch.io import load_graph
from gsearch.spatial_graph_index import SpatialGraphIndex

q, g = load_graph(sys.argv[1]), load_graph(sys.argv[2])
q_anchor = next(iter(q))
radius = _get_radius(q, q_anchor) + 10.0
index = SpatialGraphIndex(g)
start = time.perf_counter()
total = with_pose = most = 0
for n in g:
    k = len(get_candidate_matches(q, index.make_crop(n, radius), q_anchor, n, 10.0))
    total += k
    with_pose += k > 0
    most = max(most, k)
print(f"match every node: {time.perf_counter() - start:.3f} s")
print(f"anchors with a pose: {with_pose}")
print(f"poses: {total} in total, {most} at most for one anchor")
'@
$check | uv run python - ..\gsearch_rs\data\roads\queries\gr00049_q.json ..\gsearch_rs\data\roads\graphs\gr00049.json
cd ..\gsearch_rs
```

- [ ] Check that "anchors with a pose" and "poses" are the same as in Rust, and compare the
  times. If the counts differ, tell Claude: the first place to look is a crop at the edge of the
  radius (Slice 4), and then a distance difference at exactly `tol`.

### 13. Format, lint, commit

- [ ] Run `cargo fmt`, then `cargo clippy --all-targets`. Expected: no warnings.
- [ ] Run `cargo test`. Expected: `75 passed; 0 failed`.
- [ ] Commit:

  ```powershell
  git add -A
  git commit -m "Slice 5: annulus masks, handedness split, candidate poses"
  ```

---

## What comes next

Slice 6 ports `_align_graph`: for one pose, the rotation (or reflection) and translation that
put the query nodes closest to their matched crop nodes. In 2D this has a closed form, so no
matrix crate is needed (decision D4). It reads `crop.coords()[pose[i]]` for each query node `i`.

Slice 9 comes back to two things in this slice:

- The masks are `Vec<Vec<bool>>`, with one allocation per row, and `and` and the split make new
  masks for every alignment node.
- All poses are stored before they are used. A search that hands each pose to the next step as
  it is found, and stops early on a bad pose, would use far less memory.
