# Slice 5: Node matching

**Goal.** Port `align_nodes.py`. `get_candidate_matches` takes the query `Q` and a crop of `G`,
with the anchor of each at position `0`. It returns every **pose**: a way to give each node of
`Q` its own node of the crop, such that the distances to the anchors agree within `tol`. Slices
6 and 7 align and score each pose.

**Outcome.** `cargo test align` reports `31 passed`, and `cargo run` finds the poses around
every node of a road graph and says how long it took.

**New ideas.** A table of matches as `Vec<Vec<usize>>`, in place of a boolean mask; `flatten`;
a slice of part of an array (`&g[..2]`); why a private function that only tests call is "never
used"; `?` on an `Option`; `todo!` with a message; `min_by_key`; recursion with `&mut`
parameters, in place of a Python generator; why a closure cannot call itself; a parameter that
takes any closure (`impl Fn`); `copied` on an iterator; `extend`; `chain`; `match` on a tuple,
with `|` and `_` patterns; `bool::then_some`; `unwrap_or_default`.

**Starting point.** Slice 4 is done, and `cargo test` reports `51 passed`.

Notes about your files:

- Your component method is `cc_containing`, and the tree field is `kd_tree`. This slice does not
  use either directly.
- Your `src/main.rs` is still the Slice 1 version (Slice 4, Step 13 was not done). Step 10 of
  this slice replaces the whole file, so you do not need to do Slice 4, Step 13 first.
- Steps 1 to 3 are done, and most of Step 4. This version of the slice follows your return
  types: `annulus_matches` returns `Option<Matches>`, and `get_candidate_matches` returns
  `Option<Vec<Pose>>`. It also uses your names: `within_tol`, `q_coords`, `g_coords`, and
  `matches` for the anchor table.
- `use crate::io::NodeId;` and `use crate::test_fixtures::file;` are at the top of your file.
  Only the tests use them, and `test_fixtures` exists only in a test build (`#[cfg(test)]` in
  `lib.rs`). So `cargo build` fails with `E0432: unresolved import crate::test_fixtures`. Step 4
  moves them into `mod tests`.
- Your two Step 4 tests call `poses.is_empty()`, but `poses` is now an `Option`, so they do not
  compile. Step 4 changes them to `is_none()`.
- Your file is ahead of Step 3. Do the two fixes above first (Step 4: move the two `use` lines,
  and change `is_empty()` to `is_none()`). Then the file builds, and you can continue at the end
  of Step 3.
- The doc comment on your `Matches` still says "one column per node of G". That was the mask.
  You can change it to the comment in Step 1.

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

Steps 1 and 4 together are a **triangulation**. A G node that is right for Q node `i` must be
on two rings: one around the anchor (step 1), and one around the alignment node (step 4). Two
rings cross at two points, one on each side of the axis. The handedness split picks the side.

### The Rust version

| Python | Rust |
|---|---|
| `Q_arr`, `G_arr` (numpy arrays) | `q.coords()`, `g.coords()` (`&[Point]`) |
| a boolean array `(len(Q), len(G))` | `type Matches = Vec<Vec<usize>>`: `matches[i]` lists the G positions that Q node `i` can match |
| `_make_annulus_mask`, then `anchor_mask.any(axis=1).all()` | `annulus_matches`, with the test in `within_tol`. It returns `None` if a row is empty |
| `counts[Q_anchor_idx] = np.inf; np.argmin(counts)` | `alignment_node(&matches, q_anchor)`, which returns an `Option<usize>` |
| `anchor_mask & align_mask` | `filter_matches(&matches, closure)`: keep the pairs that are also on the second ring |
| a pose `dict[int, int]` from Q id to G id | `type Pose = Vec<usize>`: `pose[i]` is the crop position for Q position `i` |
| `_cross_signs` returns `+1/-1/0` | `sides` returns `Vec<Side>`, with `side_of` from Slice 2 |
| `_split_mask_by_handedness(mask, Q_arr, G_arr, 4 indices)` | `split_by_handedness(matches, q_sides, g_sides)` |
| `_make_candidates(masks, Q_ids, G_ids)` | `candidates(matches)`, called once for each of the two tables |
| `_injective_assignments`, a recursive generator | a recursive function that pushes into a `Vec` |
| `return []` (an early exit) | `return None`, or `?` on an `Option` |

Some choices need an explanation.

**The order of the steps.** `get_candidate_matches` is the function that joins all the parts.
You write it first, in Step 4, with only its first part. Each later step adds one part to it,
in the order that the data goes through it:

| Step | New part | What `get_candidate_matches` can do after the step |
|---|---|---|
| 4 | the size check, ring 1 | Return `None` (no pose) when Q is too large, or a node of Q has no match |
| 5 | `alignment_node` | Pick the second node of the triangulation |
| 6 | `candidates` | (Not used yet. It is tested alone, on tables you write by hand.) |
| 7 | the loop, ring 2, `filter_matches` | Return poses. Some of them are wrong: they mix the two sides of the axis |
| 8 | `sides`, `split_by_handedness` | Return only correct poses |

Until a part exists, the function ends in `todo!("...")`. The message says which part comes
next.

**`None` means "no pose" (your choice).** `get_candidate_matches` returns
`Option<Vec<Pose>>`. `None` means that no pose is possible, and `Some(poses)` always holds at
least one pose. There is only one form of "no pose", so a caller cannot treat two forms
differently by mistake. Python returns `[]`.

The function can find "no pose" in four places: Q is larger than the crop, a node of Q has no
match on ring 1, Q has no alignment node, or the search runs and finds nothing. The first three
are one `return None` or one `?` each. The last one is the final line of the function (Step 7).

An empty `Vec` would work as well. Nothing in this project needs one choice more than the
other. `Option` lets each early exit be one `?`.

**A pose holds positions, not ids (decision D7).** Python returns `{Q id: G id}`, and
`align_and_score` then calls `G_nodes.index(...)` to find each row again. A `Vec<usize>` with
one crop position per Q position needs no lookup. Slice 6 indexes `crop.coords()` with it
directly. The ids are only needed when the result is printed (Slice 8).

**Lists of matches, not a boolean mask (decision D4).** The Python code uses a full
`(len(Q), len(G))` table of `True`/`False` because numpy can only be fast on whole arrays. Rust
has no such limit: a plain loop is fast. So each row keeps only the columns that match, in
increasing order:

- The rows are short. An annulus row has a few matches out of about 100 crop nodes. A list
  holds only those few, and the search does not scan the other columns again and again.
- The search needs lists anyway. The Python `_injective_assignments` first turns each row into
  its `True` columns (`np.flatnonzero`).
- "The number of `True` cells in a row" becomes `matches[i].len()`, and "a row is all
  `False`" becomes `matches[i].is_empty()`.
- The second ring and the split by handedness only remove pairs. So they become one operation,
  `filter_matches`, that keeps the pairs that pass a test. It only tests the pairs that are
  already in the anchor lists. Python computes a full second table, and then `&`s it.
- A test can write a table as `vec![vec![0], vec![1, 2]]`.

The columns stay in increasing order, as `np.flatnonzero` gives them, so the poses come out in
the same order as in Python.

**The split takes sides, not coordinates.** The handedness of a pair depends only on the side
of each node about its own axis. So `sides` computes a `Vec<Side>` once per axis, and the split
only compares them. `side_of` and `Side::flipped` already exist and are tested (Slice 2). The
Python tests of `_cross_signs` are the geometry tests of Slice 2, so this slice does not port
them again.

**One-node query.** With one node, `Q` has no alignment node. Python then uses the anchor as
its own alignment node and returns poses from a degenerate axis. The Rust version returns
`None`. This is a second planned difference from Python. A one-node query is not in the data.

### The tests

The Python tests are in `tests/test_align_nodes.py`.

| Python test | Rust test |
|---|---|
| `test_output_shape` | `annulus_matches_has_a_row_per_q_node`. A list has no fixed width, so the test checks that every match is a position of G |
| `test_matches_brute_force` | `annulus_matches_agrees_with_brute_force` |
| `test_translation_invariant` | `annulus_matches_ignores_a_shift_of_q` |
| `test_anchor_row_matches_near_g_anchor` | Not ported. `annulus_matches_agrees_with_brute_force` checks the same row |
| `test_boundary_is_inclusive` | `annulus_matches_boundary_is_inclusive` |
| `test_tol_zero_requires_exact_distance` | `annulus_matches_with_tol_zero_needs_equal_distances` |
| `test_nonzero_anchor_indices` | `annulus_matches_with_other_reference_nodes` |
| `test_empty_Q_arr_raises`, `test_empty_G_arr_raises` | Not ported. A reference index into an empty slice is a bug, and it panics (D3). A graph from a file or a crop always has a node |
| (none) | `annulus_matches_is_none_if_a_q_node_has_no_match`, for the new `None` case |
| (none) | 4 tests on `alignment_node`. Python has no function for this, so it has no test |
| `TestMakeCandidates` (7 tests) | 6 tests on `candidates`. `test_combines_every_mask_in_the_tuple` is covered by `finds_a_reflected_pose` |
| (none) | `filter_keeps_only_the_pairs_that_pass`, for the new helper |
| `TestCrossSigns` (5 tests) | The `side_of` tests of Slice 2, plus `sides_about_the_axis` |
| `TestSplitMaskByHandedness` (5 tests) | 4 tests. `test_outputs_keep_input_shape` is covered by the others: they look at every row |
| `TestGetCandidateMatches` (6 tests) | 5 tests, plus `single_node_query_has_no_pose`, `mixed_hands_are_not_a_pose` and `only_mixed_hands_gives_no_pose`. `test_returns_node_ids_not_indices` is not needed, because a pose holds positions |

---

## Steps

### 1. The module and `Graph::coords`

- [x] Add a getter inside `impl Graph` in `src/graph.rs`, after `neighbors`:

  ```rust
      /// The coordinates of every node, in position order.
      pub fn coords(&self) -> &[Point] {
          &self.coords
      }
  ```

  This lends the whole coordinate list as a slice, as `neighbors` lends one adjacency list
  (Slice 3, Step 7). The matching code works on `&[Point]`, like the Python code works on
  arrays.

- [x] Make `src/align_nodes.rs`:

  ```rust
  use crate::geometry::Point;

  /// `matches[i]` lists the positions in G that node `i` of Q can match, in increasing order.
  pub type Matches = Vec<Vec<usize>>;

  /// A candidate match: `pose[i]` is the position in G of the node matched to Q position `i`.
  pub type Pose = Vec<usize>;
  ```

  Both are type aliases (Slice 1, Step 3). `Matches`, `Pose` and their `Vec` types are the same
  types, so every `Vec` method works on them. The alias only gives the type a name that says
  what it holds. (`Matches` and `Vec<Pose>` are even the same type, `Vec<Vec<usize>>`. The
  names keep them apart for the reader, not for the compiler.)

  Why "pose" and not "match"? `match` is a keyword in Rust, so it cannot be a variable name.

- [x] Add `pub mod align_nodes;` to `src/lib.rs`, above `pub mod geometry;`.

- [x] Run `cargo build`. Expected: it builds. You can get a warning about the unused import
  `Point`.

### 2. The annulus tests

- [x] Add a stub to `src/align_nodes.rs`:

  ```rust
  /// `matches[i]` holds every node `j` of G that is at the same distance from `g[g_ref]` as
  /// node `i` of Q is from `q[q_ref]`, within `tol`. `None` if a node of Q has no match.
  fn annulus_matches(
      q: &[Point],
      g: &[Point],
      q_ref: usize,
      g_ref: usize,
      tol: f64,
  ) -> Option<Matches> {
      todo!()
  }
  ```

  It is private: only `get_candidate_matches` calls it. The tests can still call it, because
  `mod tests` is inside this module and `use super::*` brings in private items too.

  `q_ref` and `g_ref` are "reference" nodes, not anchors. The Python function is called with
  the anchors and again with the alignment nodes. The Rust code calls it only with the anchors
  (Step 4), but the tests use other reference nodes too, as in Python.

  It returns `None` when a node of Q has no match. Then no pose is possible, and there is no
  reason to look at the other nodes. This is the Python `anchor_mask.any(axis=1).all()` check,
  done inside the function.

- [x] Add the test module at the end of the file:

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
  }
  ```

  1. A `const` cannot call `Point::new`, because `new` is a normal function. It can use the
     struct literal `Point { x, y }`, because the fields are `pub`. Your geometry tests do the
     same with `A` and `B`.
  2. `vec![Vec::new(); q.len()]` is `vec![x; n]` (Slice 4, Step 10): `q.len()` empty rows. The
     compiler gets the item type of each row from the return type `Matches`. `vec![x; n]`
     clones `x` for each copy, so every row is its own `Vec`. (The Python trap
     `[[]] * n`, where every row is the same list, cannot happen here.)
  3. The loop over `j` goes up, so each row is in increasing order, like `np.flatnonzero`.
  4. `brute_force` uses `hypot`, and the code under test will use `Point::dist`. The two are
     different code, so they check each other, as in the Python test. `brute_force` returns a
     plain `Matches`, with empty rows if there are any. The tests below only use it where every
     row has a match.

- [x] Add six tests inside `mod tests`, after `brute_force`:

  ```rust
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
          // Q[1] is at distance 3 and G[3] at distance 2: the difference is exactly 1
          let matches = annulus_matches(&Q, &G, 0, 0, 1.0).unwrap();

          assert!(matches[1].contains(&3));
      }

      #[test]
      fn annulus_matches_with_tol_zero_needs_equal_distances() {
          let matches = annulus_matches(&Q, &G, 0, 0, 0.0).unwrap();

          assert_eq!(matches, brute_force(&Q, &G, 0, 0, 0.0));
          // G[3] (distance 2) and G[4] (distance 100) match no node of Q
          assert_eq!(matches, vec![vec![0], vec![1], vec![2]]);
      }

      #[test]
      fn annulus_matches_with_other_reference_nodes() {
          // Q[1] and G[1] as the references
          let matches = annulus_matches(&Q, &G, 1, 1, 0.0).unwrap();

          assert_eq!(matches, brute_force(&Q, &G, 1, 1, 0.0));
          // each reference node is at distance 0 from itself
          assert!(matches[1].contains(&1));
      }
  ```

  1. `annulus_matches(&Q, &G, ...)`: `&Q` borrows the array, and Rust turns a `&[Point; 3]`
     into a `&[Point]` by itself. This is why a function that takes a slice accepts an array, a
     `Vec` and part of a `Vec`.
  2. `.unwrap()` takes the `Matches` out of the `Some` (Slice 1, Step 7). In these tests every
     node of Q has a match, so a `None` is a bug, and the panic shows it.
  3. `flatten` turns the rows into one stream of values, like
     `itertools.chain.from_iterable`. Here it checks every match of every row in one `all`.
  4. `assert_eq!(matches, ...)` compares two `Vec<Vec<usize>>` row by row and value by value.
     In numpy, `==` gives an array, and you need `np.array_equal`. In Rust, `==` on two `Vec`s
     gives one `bool`.
  5. `contains(&3)` looks for a value in a `Vec`, like `3 in row` in Python. It takes a
     reference, so you write `&3`.

- [x] Run `cargo test align`. Expected: `0 passed; 6 failed`, all with `not yet implemented`.

### 3. Write `annulus_matches`

**Try first.** Compute the distance of each `G` node to `g[g_ref]` once, into a `Vec<f64>`.
Then fill one row for each `Q` node: its distance to `q[q_ref]`, and every column `j` whose
`G` distance agrees within `tol`. Put the comparison in a small function
`within_tol(q_dist: f64, g_dist: f64, tol: f64) -> bool`, because Step 7 uses it again. If a
row is empty when it is complete, return `None`.

**Suggested code.** This is your version.

```rust
/// True when two distances agree within `tol`. A difference of exactly `tol` counts.
fn within_tol(q_dist: f64, g_dist: f64, tol: f64) -> bool {
    (q_dist - g_dist).abs() <= tol
}

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

        // a node of Q with no match: no pose is possible
        if matches[i].is_empty() {
            return None;
        }
    }
    Some(matches)
}
```

**Explanation.**

1. `g_dists` is computed before the loop over `Q`. Inside the loop it would be computed again
   for every row. numpy does the same with `G_dists[None, :]`: one row of distances, used by
   every row of `Q`.
2. `&p_q` and `&g_dist` copy each value out of the reference that the loop gives (`Point` is
   `Copy`, Slice 2, Step 2). This is cheap: two `f64`s, or one.
3. The check for an empty row is inside the loop over `Q`. So the function stops at the first
   node of Q with no match, and does not compute the rows after it.
4. `<= tol` keeps a difference of exactly `tol`, like the Python `<=`.
5. Is this as fast as numpy? Yes, or faster. numpy makes three temporary tables here
   (`Q_dists[:, None] - G_dists[None, :]`, then `abs`, then `<=`). The loop makes only the
   short rows that it returns (decision D4).

- [x] Run `cargo test align`. Expected: `6 passed; 0 failed`.

- [ ] Add a test for the `None` case at the end of `mod tests`:

  ```rust
      #[test]
      fn annulus_matches_is_none_if_a_q_node_has_no_match() {
          // without G[2], nothing in G is at distance 4 from G[0]
          let matches = annulus_matches(&Q, &G[..2], 0, 0, 0.5);

          assert!(matches.is_none());
      }
  ```

  `&G[..2]` is a slice of the first two items of `G`, like `G[:2]` in Python. It does not copy:
  it lends part of the array. (`[2..]` is "from 2 to the end", and `[1..3]` is "1 and 2".) An
  index out of range panics, where Python returns a shorter list.

- [ ] Run `cargo test align`. Expected: `7 passed; 0 failed`. (You have the two Step 4 tests
  too, so you see `9 passed`.)

`cargo build` gives two warnings here, if nothing calls `annulus_matches` yet:
`function within_tol is never used` and `function annulus_matches is never used`. Both
functions are private, and the only caller is `mod tests`. `#[cfg(test)]` removes `mod tests`
from a normal build, so in that build nothing calls them. Step 4 removes the warnings: a `pub`
function calls them.

### 4. Start `get_candidate_matches`: the early exits

The first version of `get_candidate_matches` does steps 1 and 2 of "What the Python code
does": the size check, and ring 1 with its empty-row check. Then it stops. Most of this step is
already in your file.

- [ ] Move `use crate::io::NodeId;` and `use crate::test_fixtures::file;` from the top of the
  file into `mod tests`, below `use super::*;`. Keep `use crate::graph::Graph;` at the top:
  `get_candidate_matches` uses it.

- [x] Add the main function after the two type aliases. It is the only public function of the
  module, so it goes first:

  ```rust
  /// Candidate poses that put `q_anchor` of Q on `g_anchor` of G. `None` if the search stops
  /// before it starts: no pose is possible.
  pub fn get_candidate_matches(
      q: &Graph,
      g: &Graph,
      q_anchor: usize,
      g_anchor: usize,
      tol: f64,
  ) -> Option<Vec<Pose>> {
      todo!()
  }
  ```

  In the search, both anchors are position `0` (Slice 4). The parameters stay, so that the
  function says which nodes it aligns, and so that it is the same as the Python function.

- [ ] Add the data and a helper at the end of `mod tests` (you have `graph` already):

  ```rust
      // A 3-4-5 triangle. Its three distances are all different, so it has only one
      // correct pose. The ids are not positions.
      const TRIANGLE: [(NodeId, f64, f64); 3] = [(10, 0.0, 0.0), (20, 3.0, 0.0), (30, 0.0, 4.0)];

      fn graph(nodes: &[(NodeId, f64, f64)]) -> Graph {
          Graph::from_file(&file(nodes, &[])).unwrap()
      }
  ```

- [ ] Change the two tests to check for `None`:

  ```rust
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
  ```

  The graphs have no edges. Node matching only uses coordinates, as the Python docstring says
  ("This ignores topology").

**Try first.** Write the body up to ring 1. Return `None` if Q has more nodes than G. Then call
`annulus_matches`, and use `?` to return `None` if it returns `None`. End the body with
`todo!("pick the alignment node")`.

**Suggested code.** This is your version, with the `todo!` message.

```rust
pub fn get_candidate_matches(
    q: &Graph,
    g: &Graph,
    q_anchor: usize,
    g_anchor: usize,
    tol: f64,
) -> Option<Vec<Pose>> {
    // every node of Q needs its own node of G
    if q.node_count() > g.node_count() {
        return None;
    }

    let q_coords = q.coords();
    let g_coords = g.coords();

    // ring 1: the G nodes at the right distance from the G anchor
    let matches = annulus_matches(q_coords, g_coords, q_anchor, g_anchor, tol)?;

    todo!("pick the alignment node")
}
```

**Explanation.**

1. **`?` on an `Option`.** Slice 1, Step 13 used `?` on a `Result`: on an `Err`, return it from
   the function at once. On an `Option`, `?` does the same with `None`: if
   `annulus_matches(...)` is `None`, `get_candidate_matches` returns `None`. If it is
   `Some(m)`, the expression gives `m`. So `matches` is a plain `Matches`, not an `Option`.
2. `?` on an `Option` only works in a function that returns an `Option`. (`?` on a `Result`
   needs a function that returns a `Result`.) The compiler tells you if they do not agree.
3. `todo!("...")` takes a message, like `raise NotImplementedError("...")`. A test that gets
   to it fails with `not yet implemented: pick the alignment node`. Both tests here return
   before it.
4. `q.coords()` lends the coordinates as a slice. `annulus_matches` takes `&[Point]`, so
   `q_coords` goes in as it is, with no `&`.

- [ ] Run `cargo test align`. Expected: `9 passed; 0 failed`.
- [ ] Run `cargo build`. Expected: it builds, with one warning: `unused variable: matches`.
  Step 5 uses it. The `never used` warnings are gone: `get_candidate_matches` is `pub`, so code
  outside the module can call it, and it calls the other two functions.

### 5. The alignment node

Step 3 of "What the Python code does": the node of Q, other than the anchor, with the fewest
matches. Its row is the shortest, so the loop of Step 7 runs the fewest times. It goes in its
own function, so that the tests can check the choice directly.

- [ ] Add a stub after `annulus_matches`:

  ```rust
  /// The node of Q, other than the anchor, with the fewest matches. The first one on a tie.
  /// `None` if Q has no node other than the anchor.
  fn alignment_node(matches: &Matches, q_anchor: usize) -> Option<usize> {
      todo!()
  }
  ```

- [ ] Add five tests at the end of `mod tests`:

  ```rust
      #[test]
      fn alignment_node_has_the_fewest_matches() {
          let matches = vec![vec![0], vec![1, 2, 3], vec![1, 2], vec![3, 4, 5]];

          assert_eq!(alignment_node(&matches, 0), Some(2));
      }

      #[test]
      fn alignment_node_is_the_first_on_a_tie() {
          let matches = vec![vec![0], vec![1, 2], vec![3, 4]];

          assert_eq!(alignment_node(&matches, 0), Some(1));
      }

      #[test]
      fn alignment_node_is_never_the_anchor() {
          // the anchor (row 1) has the fewest matches, but it cannot align with itself
          let matches = vec![vec![1, 2, 3], vec![0], vec![1, 2]];

          assert_eq!(alignment_node(&matches, 1), Some(2));
      }

      #[test]
      fn one_node_has_no_alignment_node() {
          let matches = vec![vec![0]];

          assert_eq!(alignment_node(&matches, 0), None);
      }

      #[test]
      fn single_node_query_has_no_pose() {
          let q = graph(&[(10, 0.0, 0.0)]);
          let g = graph(&[(101, 100.0, 100.0)]);

          let poses = get_candidate_matches(&q, &g, 0, 0, 0.5);

          assert!(poses.is_none());
      }
  ```

- [ ] Run `cargo test align`. Expected: `9 passed; 5 failed`. The last test fails with
  `not yet implemented: pick the alignment node`.

**Try first.** Go over the row numbers `0..matches.len()`, drop the anchor, and take the row
number with the shortest row. One iterator method does the last part: `min_by_key(f)` returns
the item with the smallest `f(item)`, as an `Option` (it is `None` for an empty iterator). If
several items have the same smallest key, it returns the **first** one, the same as
`np.argmin`. (`max_by_key` returns the **last** one. Check the docs when a tie matters.)

Then replace the `todo!` in `get_candidate_matches`. Use `?` again, and end with
`todo!("triangulate with the alignment node")`.

**Suggested code.**

```rust
fn alignment_node(matches: &Matches, q_anchor: usize) -> Option<usize> {
    (0..matches.len())
        .filter(|&i| i != q_anchor)
        .min_by_key(|&i| matches[i].len())
}
```

In `get_candidate_matches`, replace `todo!("pick the alignment node")` with:

```rust
    let q_align = alignment_node(&matches, q_anchor)?;

    todo!("triangulate with the alignment node")
```

**Explanation.**

1. Python sets the count of the anchor to `inf` so that `argmin` skips it. Here `filter` drops
   the anchor before `min_by_key` looks at the counts. No sentinel value is needed, and the
   counts are plain `usize` lengths. There is no table of counts: `len()` of a `Vec` is stored,
   so it costs nothing to ask for it again.
2. `min_by_key` already returns an `Option`, so `alignment_node` returns it as it is. There is
   no `if` for the one-node case: after the anchor is dropped, the iterator is empty, and
   `min_by_key` gives `None` by itself.
3. In `get_candidate_matches`, the `?` turns that `None` into the `None` of the whole function.
   This is the one-node query of "The design". Your choice of `Option` as the return type makes
   this one character.

- [ ] Run `cargo test align`. Expected: `14 passed; 0 failed`. `cargo build` warns that
  `q_align` is not used. Step 7 uses it.

### 6. `candidates`: from a table to poses

Step 5 of "What the Python code does". `candidates(matches)` lists every pose that one table
allows: one column from each row, and no column twice. It is the last part of the pipeline, but
it does not need the parts before it. Its tests use small tables that you write by hand. Step 7
connects it.

- [ ] Add a stub after `alignment_node`:

  ```rust
  /// Every pose that `matches` allows: one column from each row, with no column used twice.
  fn candidates(matches: &Matches) -> Vec<Pose> {
      todo!()
  }
  ```

  It returns a `Vec`, not an `Option`. The tables that it gets come from the second ring and the
  handedness split (Steps 7 and 8), and those can make a row empty. An empty row gives no pose,
  and that is an empty `Vec`. `get_candidate_matches` collects the poses of many tables, so an
  empty `Vec` is the useful answer here.

- [ ] Add six tests at the end of `mod tests`:

  ```rust
      #[test]
      fn one_way_to_assign() {
          let matches = vec![vec![0], vec![1]];

          assert_eq!(candidates(&matches), vec![vec![0, 1]]);
      }

      #[test]
      fn every_injective_assignment_is_found() {
          let matches = vec![vec![0, 1], vec![0, 1]];

          // both one-to-one assignments, and nothing else
          assert_eq!(candidates(&matches), vec![vec![0, 1], vec![1, 0]]);
      }

      #[test]
      fn a_column_is_not_used_twice() {
          // both rows can only use column 0
          let matches = vec![vec![0], vec![0]];

          assert!(candidates(&matches).is_empty());
      }

      #[test]
      fn a_row_with_no_match_gives_nothing() {
          let matches = vec![vec![], vec![0]];

          assert!(candidates(&matches).is_empty());
      }

      #[test]
      fn more_columns_than_rows() {
          let matches = vec![vec![0, 1], vec![1, 2]];

          // every pick of two different columns, in row order
          assert_eq!(candidates(&matches), vec![vec![0, 1], vec![0, 2], vec![1, 2]]);
      }

      #[test]
      fn one_row_gives_one_pose_per_match() {
          let matches = vec![vec![0, 2]];

          assert_eq!(candidates(&matches), vec![vec![0], vec![2]]);
      }
  ```

  1. In `a_row_with_no_match_gives_nothing`, `vec![]` alone has no item type. The compiler gets
     it from the next row, `vec![0]`, and from the call `candidates(&matches)`, which needs a
     `&Vec<Vec<usize>>`. So the integer `0` is a `usize` too.
  2. The Python tests compare ids, and turn the result into a `set` when the order is not fixed.
     Here a pose holds positions, and the order is fixed: the search tries the columns of each
     row in list order. So the tests compare the `Vec` directly.

- [ ] Run `cargo test align`. Expected: `14 passed; 6 failed`.

**How the search works.** `pose` is a list that the search fills from left to right: `pose[0]`
is the G node for Q node 0, `pose[1]` for Q node 1, and so on.

1. The next row to fill is `pose.len()`.
2. Try each column of that row, one at a time: push it onto `pose`, fill the remaining rows
   (the function calls itself), then pop it, so that the next column can take its place.
3. When `pose` has one value for each row, the pose is complete. Copy it into `found`.

For `[[0, 1], [0, 1]]`, with the "no column twice" check (each indent is one more call):

```text
pose = []          row 0: try 0, then 1
  pose = [0]       row 1: 0 is used, skip. try 1
    pose = [0, 1]  complete -> found = [[0, 1]]
  pose = [1]       row 1: try 0
    pose = [1, 0]  complete -> found = [[0, 1], [1, 0]]
                   1 is used, skip
```

**The Python version is a recursive generator.** `backtrack(i)` does the same, and `yield`s
each complete pose. Rust (in the stable language) has no generators, and a closure cannot call
itself, because a closure has no name that its own body can use. So the Rust version is a
normal function that calls itself, and it pushes each complete pose into a `Vec` that the
caller gives it.

**Try first.** Write a second function:

```rust
/// Add to `found` every pose that starts with `pose`.
fn extend_poses(matches: &Matches, pose: &mut Pose, found: &mut Vec<Pose>)
```

`candidates` makes an empty `pose` and an empty `found`, calls `extend_poses`, and returns
`found`. `extend_poses` is the Python `backtrack`. It needs no `i`, because the next row is
`pose.len()`.

**Suggested code.** Start without the "no column twice" check, and let the tests show why it is
needed.

```rust
fn candidates(matches: &Matches) -> Vec<Pose> {
    let mut found = Vec::new();
    let mut pose = Vec::with_capacity(matches.len());
    extend_poses(matches, &mut pose, &mut found);
    found
}

/// Add to `found` every pose that starts with `pose`.
fn extend_poses(matches: &Matches, pose: &mut Pose, found: &mut Vec<Pose>) {
    let row = pose.len();
    if row == matches.len() {
        found.push(pose.clone());
        return;
    }

    for &j in &matches[row] {
        pose.push(j);
        extend_poses(matches, pose, found);
        pose.pop();
    }
}
```

**Explanation.**

1. `candidates` owns `found` and `pose`, and lends both to `extend_poses` with `&mut` (Slice 3,
   Step 3). There is one `pose` for the whole search. Each call pushes a column, lets the
   deeper calls work, and pops it again. So when a call returns, `pose` is the same as when the
   call started. This is the `combo.append` / `combo.pop()` of the Python code.
2. Inside `extend_poses`, `pose` is already a `&mut Pose`. To pass it down, you write `pose`,
   not `&mut pose`. Rust lends it on for the length of the call (a **reborrow**), and `pose` is
   usable again after the call returns. So the `pose.pop()` on the next line is allowed.
3. `found.push(pose.clone())` stores a copy. `pose` itself keeps changing, so `found` cannot
   hold `pose`. In Python, `tuple(combo)` makes the copy for the same reason.
4. `for &j in &matches[row]`: `&matches[row]` borrows the row, the loop gives `&usize` items,
   and `&j` copies each one out.

- [ ] Run `cargo test align`. Expected: `17 passed; 3 failed`. One of the failures:

  ```text
  ---- align_nodes::tests::every_injective_assignment_is_found stdout ----
  assertion `left == right` failed
    left: [[0, 0], [0, 1], [1, 0], [1, 1]]
   right: [[0, 1], [1, 0]]
  ```

  This version is `itertools.product(*row_matches)`: every combination, also those that use
  column 0 twice. `a_column_is_not_used_twice` and `more_columns_than_rows` fail for the same
  reason.

- [ ] Add the check at the top of the `for` loop in `extend_poses`:

  ```rust
      for &j in &matches[row] {
          if pose.contains(&j) {
              continue;
          }
          pose.push(j);
          extend_poses(matches, pose, found);
          pose.pop();
      }
  ```

  Python keeps a separate `used` set for this. Here `pose` has at most 20 values (one per query
  node), and a scan of 20 numbers is faster than a hash lookup. It also means there is no second
  structure to keep in step with `pose`.

  Because the check is at each level, a branch that uses a column twice stops at once. It does
  not build the rest of the pose first. This is the point of the backtracking version, as the
  Python docstring says.

- [ ] Add an early exit at the top of `candidates`:

  ```rust
      if matches.iter().any(|row| row.is_empty()) {
          return Vec::new();
      }
  ```

  This is the Python `if not mask.any(axis=1).all(): continue` in `_make_candidates`. The
  search would find nothing for a table with an empty row anyway, but only after it tries every
  combination of the rows before it.

- [ ] Run `cargo test align`. Expected: `20 passed; 0 failed`. `cargo build` warns that
  `candidates` is never used. Step 7 uses it.

### 7. The second ring: the first poses

Step 4 of "What the Python code does", without the handedness split. For each G node that the
alignment node can match, keep only the anchor matches that are also on the second ring, around
the alignment pair. Then give that table to `candidates`.

The second ring only removes pairs from the anchor table. So it is a filter: "keep pair
`(i, j)` if this test is true". The handedness split in Step 8 is a filter too, with a
different test. So one function does both, and takes the test as a closure.

- [ ] Add a stub after `candidates` and `extend_poses`:

  ```rust
  /// Keep the pair (Q node `i`, G node `j`) of `matches` only when `keep(i, j)` is true.
  fn filter_matches(matches: &Matches, keep: impl Fn(usize, usize) -> bool) -> Matches {
      todo!()
  }
  ```

  1. `keep: impl Fn(usize, usize) -> bool` means "any closure (or function) that takes two
     `usize` values and returns a `bool`". It is the Python type hint
     `Callable[[int, int], bool]`. Each closure has its own type, and that type has no name
     you can write. `impl Fn` lets `filter_matches` accept any of them.
  2. The compiler makes a separate copy of `filter_matches` for each closure you pass to it, as
     it does for a generic type (Slice 4, Step 5). So the call to `keep` costs the same as code
     written in place. A Python callback costs a function call for every pair.
  3. `Fn` is one of three closure traits. `Fn` only reads what it captures, so it can be called
     many times, also through a shared reference. (`FnMut` can change what it captures, and
     `FnOnce` can be called only once.) A filter only reads, so `Fn` is correct.
  4. It returns a plain `Matches`, not an `Option`, for the same reason as `candidates`: an
     empty row here is a normal result, and `candidates` handles it.

- [ ] Add two helpers and four tests at the end of `mod tests`:

  ```rust
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
      fn filter_keeps_only_the_pairs_that_pass() {
          let matches = vec![vec![0, 1, 2], vec![0, 1, 2]];

          // keep the pairs where the column is larger than the row
          let kept = filter_matches(&matches, |i, j| j > i);

          assert_eq!(kept, vec![vec![1, 2], vec![2]]);
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

          let poses = get_candidate_matches(&q, &g, 0, 0, 0.5).unwrap();

          assert!(has_pose(&q, &g, &poses, &[(10, 101), (20, 102), (30, 103)]));
          // node 104 (position 3) is in no pose
          assert!(poses.iter().all(|pose| !pose.contains(&3)));
      }

      #[test]
      fn finds_a_rotated_pose() {
          // the triangle turned by 90 degrees: (x, y) -> (-y, x)
          let q = graph(&TRIANGLE);
          let g = graph(&[(101, 100.0, 100.0), (102, 100.0, 103.0), (103, 96.0, 100.0)]);

          let poses = get_candidate_matches(&q, &g, 0, 0, 0.5).unwrap();

          assert!(has_pose(&q, &g, &poses, &[(10, 101), (20, 102), (30, 103)]));
      }

      #[test]
      fn finds_a_reflected_pose() {
          // the triangle mirrored: (x, y) -> (x, -y)
          let q = graph(&TRIANGLE);
          let g = graph(&[(101, 100.0, 100.0), (102, 103.0, 100.0), (103, 100.0, 96.0)]);

          let poses = get_candidate_matches(&q, &g, 0, 0, 0.5).unwrap();

          assert!(has_pose(&q, &g, &poses, &[(10, 101), (20, 102), (30, 103)]));
      }
  ```

  1. `filter_matches(&matches, |i, j| j > i)`: the closure is the `keep` argument. The
     compiler gets the types of `i` and `j` from `impl Fn(usize, usize) -> bool`, so the
     closure needs no type hints.
  2. `id_pairs` turns a pose back into ids, so the tests can state the expected pose with the
     same ids as the Python tests. Position `i` of the pose is Q position `i`, so `enumerate`
     gives the Q side of each pair.
  3. `id_pairs(...) == expected` compares a `Vec<(NodeId, NodeId)>` with a
     `&[(NodeId, NodeId)]`. Rust allows `==` between a `Vec` and a slice of the same item
     type.
  4. `has_pose` takes `&[Pose]`, and `&poses` is a `&Vec<Pose>`. Rust turns one into the other
     by itself, as with an array (Step 2).

- [ ] Run `cargo test align`. Expected: `20 passed; 4 failed`. The three `finds_` tests fail
  with `not yet implemented: triangulate with the alignment node`.

**Try first.** Write `filter_matches`: a `map` over the rows (with `enumerate`, to know `i`),
and a `filter` inside each row. Then replace the `todo!` in `get_candidate_matches` with a loop
over the row of the alignment node in `matches`. For each `g_align` in it, call
`filter_matches` on `matches` with a closure that calls `within_tol` on two distances: Q node
`i` to `q_coords[q_align]`, and G node `j` to `g_coords[g_align]`. Give the result to
`candidates`, and collect every pose. Return `None` if there is no pose, and `Some(poses)` if
there is.

**Suggested code.**

```rust
fn filter_matches(matches: &Matches, keep: impl Fn(usize, usize) -> bool) -> Matches {
    matches
        .iter()
        .enumerate()
        .map(|(i, row)| row.iter().copied().filter(|&j| keep(i, j)).collect())
        .collect()
}
```

In `get_candidate_matches`, replace `todo!("triangulate with the alignment node")` with:

```rust
    let mut poses = Vec::new();
    for &g_align in &matches[q_align] {
        // ring 2: the anchor matches that are also at the right distance from the alignment node
        let both = filter_matches(&matches, |i, j| {
            within_tol(
                q_coords[i].dist(q_coords[q_align]),
                g_coords[j].dist(g_coords[g_align]),
                tol,
            )
        });

        poses.extend(candidates(&both));
    }
    (!poses.is_empty()).then_some(poses)
```

**Explanation.**

1. In `filter_matches`, `row.iter()` gives `&usize` items. `copied()` turns them into `usize`
   values, as `Option::copied` does for one value (Slice 3, Step 6). Then the row of the result
   holds `usize`, not references into `matches`.
2. The inner closure `|&j| keep(i, j)` captures `keep` and `i` from around it (Slice 3,
   Step 8). A closure inside a closure is fine. `keep` is borrowed, not moved, because calling
   an `Fn` only needs a shared reference.
3. The loop goes over the row of the alignment node directly. This is the Python
   `np.where(anchor_mask[Q_alignment_idx])[0]`, with no work to find the `True` cells.
4. The closure in the loop captures `q_coords`, `g_coords`, `q_align`, `g_align` and `tol`. It
   computes a distance only for the pairs that are in `matches`. Python computes the full
   second mask, for every pair, and then drops most of it with `&`. The rule "within `tol`,
   inclusive" is in `within_tol`, so the two rings cannot use different rules.
5. `poses.extend(...)` appends every item of another collection, like `list.extend`.
6. `(!poses.is_empty()).then_some(poses)` is the last expression, so it is the return value
   (Slice 1, Step 9). `then_some` is a method on `bool`: `true.then_some(x)` is `Some(x)`, and
   `false.then_some(x)` is `None`. So an empty search gives `None`, the same as an early exit
   (see "The design"). In Python, it is `poses if poses else None`.

- [ ] Run `cargo test align`. Expected: `24 passed; 0 failed`.

All the tests pass, but the function is not correct yet. Two rings cross at **two** points, one
on each side of the axis from the anchor to the alignment node. So a node of Q can match its
mirror image in G. With a triangle, all of a pose is mirrored together, and that is a correct
reflection. With four nodes, one node can be mirrored and another not. No rotation or
reflection does that. Step 8 shows it with a test, and fixes it.

### 8. Handedness: the correct poses

- [ ] Add a test at the end of `mod tests`:

  ```rust
      #[test]
      fn mixed_hands_are_not_a_pose() {
          // A (30) and B (40) are both on the left of the axis from 10 to 20.
          // G has the same shape, plus 105: the mirror image of B (104) about the axis.
          let q = graph(&[(10, 0.0, 0.0), (20, 4.0, 0.0), (30, 1.0, 1.0), (40, 3.0, 1.0)]);
          let g = graph(&[
              (101, 100.0, 100.0),
              (102, 104.0, 100.0),
              (103, 101.0, 101.0),
              (104, 103.0, 101.0),
              (105, 103.0, 99.0),
          ]);

          let poses = get_candidate_matches(&q, &g, 0, 0, 0.5);

          // [0, 1, 2, 4] would put A on the left and B on the right
          assert_eq!(poses, Some(vec![vec![0, 1, 2, 3]]));
      }

      #[test]
      fn only_mixed_hands_gives_no_pose() {
          // the same Q, and G with A on the left but B only on the right
          let q = graph(&[(10, 0.0, 0.0), (20, 4.0, 0.0), (30, 1.0, 1.0), (40, 3.0, 1.0)]);
          let g = graph(&[
              (101, 100.0, 100.0),
              (102, 104.0, 100.0),
              (103, 101.0, 101.0),
              (104, 103.0, 99.0),
          ]);

          let poses = get_candidate_matches(&q, &g, 0, 0, 0.5);

          // the search runs, and finds nothing: that is None too
          assert_eq!(poses, None);
      }
  ```

  `assert_eq!` can compare two `Option`s directly. `Some(a) == Some(b)` is true when `a == b`.

- [ ] Run `cargo test align`. Expected: `24 passed; 2 failed`:

  ```text
  ---- align_nodes::tests::mixed_hands_are_not_a_pose stdout ----
  assertion `left == right` failed
    left: Some([[0, 1, 2, 3], [0, 1, 2, 4]])
   right: Some([[0, 1, 2, 3]])
  ---- align_nodes::tests::only_mixed_hands_gives_no_pose stdout ----
  assertion `left == right` failed
    left: Some([[0, 1, 2, 3]])
   right: None
  ```

  In the first test, node `105` is on both rings of B: at the same distance from the anchor and
  from the alignment node as `104`. Only the side of the axis is different. The second test has
  only the mirror image of B, so its one pose is wrong.

- [ ] Change the first line of `src/align_nodes.rs` to:

  ```rust
  use crate::geometry::{COLLINEAR_TOL, Point, Side, side_of};
  ```

- [ ] Add two stubs after `filter_matches`:

  ```rust
  /// The side of every point about the line from `points[from]` to `points[to]`.
  fn sides(points: &[Point], from: usize, to: usize) -> Vec<Side> {
      todo!()
  }

  /// Split `matches` into (rotation, reflection) by the side of each node about its axis.
  fn split_by_handedness(
      matches: &Matches,
      q_sides: &[Side],
      g_sides: &[Side],
  ) -> (Matches, Matches) {
      todo!()
  }
  ```

  The split returns two tables as a tuple (Slice 3, Step 4), like the Python function.
  `cargo fmt` puts the parameters on separate lines, because the line is too long.

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

      /// Split "every node matches every node" by the sides of `Q_HANDED` and `G_HANDED`.
      fn split_full() -> (Matches, Matches) {
          let full = vec![vec![0, 1, 2, 3]; 4];
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
          let (rotation, reflection) = split_full();

          // the left node of Q: the two axis nodes, and the left node of G
          assert_eq!(rotation[2], vec![0, 1, 2]);
          assert_eq!(reflection[2], vec![0, 1, 3]);
          // the right node of Q
          assert_eq!(rotation[3], vec![0, 1, 3]);
          assert_eq!(reflection[3], vec![0, 1, 2]);
      }

      #[test]
      fn nodes_on_the_axis_go_to_both_tables() {
          let (rotation, reflection) = split_full();

          // an axis node of Q keeps every match
          for on_axis in [0, 1] {
              assert_eq!(rotation[on_axis], vec![0, 1, 2, 3]);
              assert_eq!(reflection[on_axis], vec![0, 1, 2, 3]);
          }
          // every node of Q keeps the axis nodes of G
          for row in rotation.iter().chain(&reflection) {
              assert!(row.contains(&0) && row.contains(&1));
          }
      }

      #[test]
      fn the_tables_cover_the_input_and_overlap_only_on_the_axis() {
          let (rotation, reflection) = split_full();

          for i in 0..4 {
              for j in 0..4 {
                  let in_rotation = rotation[i].contains(&j);
                  let in_reflection = reflection[i].contains(&j);
                  // every pair of the full table is in one of the tables
                  assert!(in_rotation || in_reflection);
                  // a pair is in both only if one of its nodes is on the axis
                  let on_axis = i < 2 || j < 2;
                  assert_eq!(in_rotation && in_reflection, on_axis);
              }
          }
      }

      #[test]
      fn the_split_never_adds_a_match() {
          // only the left node of Q matches the left node of G
          let matches = vec![vec![], vec![], vec![2], vec![]];
          let q_sides = sides(&Q_HANDED, 0, 1);
          let g_sides = sides(&G_HANDED, 0, 1);

          let (rotation, reflection) = split_by_handedness(&matches, &q_sides, &g_sides);

          assert_eq!(rotation, matches);
          assert!(reflection.iter().all(|row| row.is_empty()));
      }
  ```

  1. `let (rotation, reflection) = split_full();` takes the tuple apart, as in Python.
  2. `for on_axis in [0, 1]` loops over an array literal, like `for x in (0, 1)` in Python.
  3. `rotation.iter().chain(&reflection)` goes over the rows of `rotation`, then the rows of
     `reflection`, like `itertools.chain`. `chain` takes anything that can be iterated, and a
     `&Vec` gives references, the same items as `rotation.iter()`.

- [ ] Run `cargo test align`. Expected: `24 passed; 7 failed`.

**Try first.** `sides` is one `map` over the points with `side_of(p, points[from], points[to],
COLLINEAR_TOL)`. The split is two calls to `filter_matches`: a pair stays in `rotation` if the
two nodes "have the same hand", and in `reflection` if they have opposite hands. Write the rule
as a small function `same_hand(q: Side, g: Side) -> bool`.

**Suggested code.** Start with the obvious rule, "the same side":

```rust
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

fn split_by_handedness(
    matches: &Matches,
    q_sides: &[Side],
    g_sides: &[Side],
) -> (Matches, Matches) {
    let rotation = filter_matches(matches, |i, j| same_hand(q_sides[i], g_sides[j]));
    let reflection = filter_matches(matches, |i, j| same_hand(q_sides[i], g_sides[j].flipped()));
    (rotation, reflection)
}
```

**Explanation.**

1. The closures capture `q_sides` and `g_sides`. They are already references (`&[Side]`), so
   the capture costs nothing.
2. A pair can only be dropped, never added, so the split never adds a match. In numpy,
   `mask & same` does the same, but on every cell of the full table.
3. A reflection is a rotation of the mirror image. Mirror `G` and every side of `G` flips. So
   "opposite hands" is `same_hand(q_side, g_side.flipped())`, and the rule is written once.
   `flipped` keeps `On` as `On` (Slice 2).

- [ ] Run `cargo test align`. Expected: `26 passed; 5 failed`. Two of the failures:

  ```text
  ---- align_nodes::tests::nodes_on_the_axis_go_to_both_tables stdout ----
  assertion `left == right` failed
    left: [0, 1]
   right: [0, 1, 2, 3]
  ---- align_nodes::tests::same_side_to_rotation_opposite_side_to_reflection stdout ----
  assertion `left == right` failed
    left: [2]
   right: [0, 1, 2]
  ```

  A node on the axis has no hand. The Python code multiplies the signs, and `0 * x` is `0`,
  which passes both `>= 0` and `<= 0`. With `q == g`, an `On` node only matches another `On`
  node, so the pair of an `On` node and a `Left` node is in neither table.
  `the_tables_cover_the_input_and_overlap_only_on_the_axis` fails for the same reason.
  `mixed_hands_are_not_a_pose` and `only_mixed_hands_gives_no_pose` still fail, because
  `get_candidate_matches` does not call the split yet.

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

- [ ] Run `cargo test align`. Expected: `29 passed; 2 failed`.

**Connect the split.** In `get_candidate_matches`, compute the sides of Q once, after the
alignment node, and the sides of G inside the loop. Split `both`, and get the poses of each
table.

**Suggested code.** The full function:

```rust
pub fn get_candidate_matches(
    q: &Graph,
    g: &Graph,
    q_anchor: usize,
    g_anchor: usize,
    tol: f64,
) -> Option<Vec<Pose>> {
    // every node of Q needs its own node of G
    if q.node_count() > g.node_count() {
        return None;
    }

    let q_coords = q.coords();
    let g_coords = g.coords();

    // ring 1: the G nodes at the right distance from the G anchor
    let matches = annulus_matches(q_coords, g_coords, q_anchor, g_anchor, tol)?;

    let q_align = alignment_node(&matches, q_anchor)?;
    let q_sides = sides(q_coords, q_anchor, q_align);

    let mut poses = Vec::new();
    for &g_align in &matches[q_align] {
        // ring 2: the anchor matches that are also at the right distance from the alignment node
        let both = filter_matches(&matches, |i, j| {
            within_tol(
                q_coords[i].dist(q_coords[q_align]),
                g_coords[j].dist(g_coords[g_align]),
                tol,
            )
        });

        // the two rings cross on both sides of the axis: keep one side per table
        let g_sides = sides(g_coords, g_anchor, g_align);
        let (rotation, reflection) = split_by_handedness(&both, &q_sides, &g_sides);

        poses.extend(candidates(&rotation));
        poses.extend(candidates(&reflection));
    }
    (!poses.is_empty()).then_some(poses)
}
```

**Explanation.**

1. `q_sides` is computed once, before the loop. The axis of `Q` does not change. The axis of
   `G` changes with `g_align`, so `g_sides` is computed inside the loop.
2. Rotation poses come before reflection poses for each `g_align`, as in Python. If a pose is in
   both tables (every node on the axis), it is in the list twice, as in Python.
3. In `mixed_hands_are_not_a_pose`, the rotation table keeps `104` for B and drops `105`. The
   reflection table keeps `105` for B, but it has no match for A (A is on the left in Q, and G
   has no node on the right at A's distances). An empty row gives no pose. So the only pose is
   `[0, 1, 2, 3]`.
4. In `only_mixed_hands_gives_no_pose`, the rotation table has no match for B, and the
   reflection table has no match for A. Both give no pose, so `poses` is empty, and the last
   line gives `None`.

- [ ] Run `cargo test align`. Expected: `31 passed; 0 failed`.
- [ ] Run `cargo test`. Expected: `82 passed; 0 failed`.

### 9. Look at it

- [ ] Add a temporary test at the end of `mod tests`:

  ```rust
      #[test]
      fn look() {
          let q = graph(&TRIANGLE);
          let g = graph(&[(101, 100.0, 100.0), (102, 103.0, 100.0), (103, 100.0, 96.0)]);

          let matches = annulus_matches(q.coords(), g.coords(), 0, 0, 0.5).unwrap();
          let q_sides = sides(q.coords(), 0, 1);
          let g_sides = sides(g.coords(), 0, 1);
          let (rotation, reflection) = split_by_handedness(&matches, &q_sides, &g_sides);

          println!("sides: Q {q_sides:?}, G {g_sides:?}");
          println!("anchor: {matches:?}");
          println!("rotation: {rotation:?}");
          println!("reflection: {reflection:?}");
          println!("poses: {:?}", get_candidate_matches(&q, &g, 0, 0, 0.5));
      }
  ```

  This is the mirrored triangle of `finds_a_reflected_pose`. For this graph, the second ring
  keeps every anchor match, so the test splits the anchor table directly.

- [ ] Run `cargo test look -- --nocapture` (Slice 4, Step 12). Expected:

  ```text
  sides: Q [On, On, Left], G [On, On, Right]
  anchor: [[0], [1], [2]]
  rotation: [[0], [1], []]
  reflection: [[0], [1], [2]]
  poses: Some([[0, 1, 2]])
  ```

  Node `30` of `Q` is on the left of its axis, and node `103` of `G` is on the right. So the
  rotation table loses that pair, and its last row is empty: no rotation pose. The reflection
  table keeps it, and gives the one pose `[0, 1, 2]`, which is `10 → 101, 20 → 102, 30 → 103`.
  The first two rows are in both tables, because those nodes are on the axis.

- [ ] Delete the `look` test.

### 10. Use it from `main`

The program crops `G` around every node, as in Slice 4, and now also finds the poses in each
crop.

**Read this before you run it.** The number of poses for one anchor can be very large. It is
about the product of the row lengths of the table, so it grows very fast with the number of
query nodes and with the crop size. Every pose is stored in a `Vec` before it is counted, so a
large count uses a lot of memory. The Python code has the same behaviour (it also builds the
full list). Run it first on `gr00049` (10 query nodes, crops of about 13 nodes). Keep Task
Manager open, and stop the program with `Ctrl+C` if the memory use climbs. Do not run `gr00048`
(19 query nodes, crops of about 100 nodes) or `gr00047` until Slice 9, which limits this.

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
          let poses = get_candidate_matches(&query, &crop, 0, 0, TOL).unwrap_or_default();
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
  3. `unwrap_or_default()` gives the value in a `Some`, or the **default** value of the type
     for a `None`. The default `Vec` is empty. So "no pose" becomes an empty `Vec`, and the
     loop can count it with the others. It is `poses or []` in Python.
  4. `most.max(poses.len())` is the larger of two numbers. `max` is a method on `usize`.
  5. `poses` is dropped at the end of each loop step, so only one anchor's poses are in memory
     at a time.

- [ ] Run, in release mode (Slice 0, Step 7):

  ```powershell
  cargo run --release -- data\roads\queries\gr00049_q.json data\roads\graphs\gr00049.json
  ```

  Expected: the first two lines are the same as in Slice 4 (`gr00049_q: 10 nodes, crop radius
  68.0499` and `gr00049: 1128 nodes, 1205 edges`). Write down the other three numbers.

### 11. Compare with Python

The total number of poses does not depend on the order of the nodes in a crop, so it must be the
same in both languages. From the `gsearch` folder, with the same memory warning as Step 10:

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

### 12. Format, lint, commit

- [ ] Run `cargo fmt`, then `cargo clippy --all-targets`. Expected: no warnings.
- [ ] Run `cargo test`. Expected: `82 passed; 0 failed`.
- [ ] Commit:

  ```powershell
  git add -A
  git commit -m "Slice 5: annulus matches, handedness split, candidate poses"
  ```

---

## What comes next

Slice 6 ports `_align_graph`: for one pose, the rotation (or reflection) and translation that
put the query nodes closest to their matched crop nodes. In 2D this has a closed form, so no
matrix crate is needed (decision D4). It reads `crop.coords()[pose[i]]` for each query node `i`.

Slice 9 comes back to two things in this slice:

- `filter_matches` makes three new tables (`both`, `rotation`, `reflection`) for every
  alignment node. A search that tests each pair while it recurses would make none.
- All poses are stored before they are used. A search that hands each pose to the next step as
  it is found, and stops early on a bad pose, would use far less memory.
