# Finish the three placeholders

**Goal.** The rest of the port is in `src/`. Three parts are left for you to write:

1. the Procrustes fit (`procrustes::fit`);
2. the distance from a node to the **line** through a moved query edge (`geometry::dist_to_line`);
3. Dijkstra with the corridor and the area cost (`path_scoring::find_paths`).

Each part has a placeholder now, and tests that tell you when it is correct. After this guide the
search gives real scores.

**Outcome.** `cargo test` reports `116 passed; 0 failed; 0 ignored`. The six small graphs of Part 4
give a `lowest score` below `1e-6`, and the warning about placeholders is gone.

**New ideas.** `atan2`; `fold`; a closure that gives a pair; `BinaryHeap` and why a `BinaryHeap`
needs `Ord`; an `Ord` that you write yourself (`Ordering`, `total_cmp`); `while let` with a queue;
`let ... else`; `f64::INFINITY`; `vec![None; n]`.

**Starting point.** Everything else is done. `cargo test` reports `90 passed; 0 failed; 16 ignored`.
(Your count can be a little different if you added tests.) The 16 ignored tests are your work list:
`cargo test -- --ignored` shows them. Read `GLOSSARY.md` for the words (mapping, transform,
corridor, alignment). The three parts do not depend on each other, but Part 3 needs Part 2.

The same rules as before: write the **test first** when this guide says so, watch it fail, then
write the code. A `todo!()` body makes a test fail with `not yet implemented`.

---

## Part 1: The Procrustes fit

### What it must do

`fit(q, g)` gets two lists of points of the same length. It returns the `Transform` that moves
`q[i]` closest to `g[i]`, for all `i` at once. "Closest" means that the sum of the squared
distances is smallest. A transform is a 2 by 2 matrix `r` and a shift `t`. A point is a row
vector, so `p` moves to `p * r + t` (Slice 6). `r` must be **orthogonal**: a rotation, or a
reflection. Python gets it from an SVD. In 2D there is a shorter way (decision D4: no matrix crate).

**The idea, in four lines.**

1. Subtract the mean (the centroid) from both lists. This removes `t` from the problem.
2. For centred points, the best rotation turns `q` by the angle `phi`, where
   `phi = atan2(S_cross, S_dot)`. `S_dot` is the sum of `a . b`, and `S_cross` is the sum of
   `a x b` (the `dot` and `cross` of `Point`, Slice 2), over all pairs `(a, b)` of
   `(q[i], g[i])`. This is the angle that makes the pairs point in the same direction, on average.
3. A reflection is the same problem for the **mirror** of `q` (negate `y` of every `q` point). Fit
   both, and keep the one with the smaller `node_score`. Python's SVD gives the better of the two by
   itself, and this is the same choice.
4. The shift is `t = mu_g - mu_q * r`, so that the centroid of `q` lands on the centroid of `g`.

With the row-vector rule, a rotation by `phi` is `r = [[c, s], [-s, c]]` (`c = cos phi`,
`s = sin phi`), and the reflection is `r = [[c, s], [s, -c]]`. If you want to check these two
matrices, apply them to `(1, 0)` and `(0, 1)` by hand.

### 1.1 The tests that exist

- [ ] Open `src/procrustes.rs`. At the end of `mod tests` there are four tests with
  `#[ignore = "fit is a placeholder"]`. Run `cargo test procrustes -- --ignored`.
  Expected: `1 passed; 3 failed`.
- [ ] Delete the test `placeholder_fit_gives_the_identity`. It checks the placeholder, and a real
  `fit` fails it.

### 1.2 `centroid`

**Try first.** Write `fn centroid(points: &[Point]) -> Point`: the mean of the points. You have
`Add` and `Mul<f64>` for `Point`, and `fold` (Slice 3, Step 12).

- [ ] Add a stub before `fit`, and a test to `mod tests`:

  ```rust
  /// The mean of the points.
  fn centroid(points: &[Point]) -> Point {
      todo!()
  }
  ```

  ```rust
      #[test]
      fn centroid_is_the_mean_of_the_points() {
          let c = centroid(&TRIANGLE);

          assert_point_close(c, Point::new(1.0, 4.0 / 3.0));
      }
  ```

- [ ] Run `cargo test centroid`. Expected: the test fails with `not yet implemented`.
- [ ] Replace the body:

  ```rust
      let sum = points.iter().fold(Point::new(0.0, 0.0), |acc, &p| acc + p);
      sum * (1.0 / points.len() as f64)
  ```

  1. `fold(start, |acc, item| ...)` goes over the items with a running value. It is
     `functools.reduce`. The start is the point `(0, 0)`, and the closure adds each point.
  2. `points.len() as f64` turns the length (`usize`) into a float (Slice 4, Step 7). `Point` has
     `Mul<f64>` but not `Div`, so the code multiplies by `1.0 / n`.
  3. An empty list gives `0.0 / 0.0 = NaN`. `fit` is never called with an empty list: a
     mapping has at least one node.

- [ ] Run `cargo test centroid`. Expected: `1 passed`.

### 1.3 `best_matrix`

This function does steps 2 and 3 of the idea, for **one** choice (`mirror` or not).

**Try first.** `fn best_matrix(q: &[Point], g: &[Point], mirror: bool) -> [[f64; 2]; 2]`. Both lists
are already centred. Loop over the pairs, add up `dot` and `cross` (with `q` mirrored if `mirror`),
get `phi`, and return the matrix of the idea above. You can write it without a test of its own:
Part 1.4 tests it through `fit`.

**Suggested code.**

- [ ] Add before `fit` (after `centroid`):

  ```rust
  /// The matrix `r` (a rotation, or a reflection if `mirror`) that moves the centred points
  /// `q` closest to the centred points `g`. A point is a row vector: `p` goes to `p * r`.
  fn best_matrix(q: &[Point], g: &[Point], mirror: bool) -> [[f64; 2]; 2] {
      let mut dot = 0.0;
      let mut cross = 0.0;
      for (&a, &b) in q.iter().zip(g) {
          // a reflection is a mirror of Q (y -> -y) followed by a rotation
          let a = if mirror { Point::new(a.x, -a.y) } else { a };
          dot += a.dot(b);
          cross += a.cross(b);
      }
      let (s, c) = cross.atan2(dot).sin_cos();
      if mirror {
          [[c, s], [s, -c]]
      } else {
          [[c, s], [-s, c]]
      }
  }
  ```

  1. `cross.atan2(dot)` is `math.atan2(cross, dot)`: the angle of the vector `(dot, cross)`. Rust
     writes it as a method of the first argument (`y.atan2(x)`).
  2. `sin_cos()` gives both values from one call (Slice 6, Step 5).
  3. `let a = if mirror { ... } else { a };` shadows `a` (Slice 4, Step 13). `if` is an expression
     here, as in `x if c else y` of Python.
  4. When `q` and `g` have no turn between them, `dot > 0` and `cross == 0`, so `atan2` is `0`,
     `c = 1`, `s = 0`, and the matrix is the identity. The test `fit_is_the_identity_when_q_equals_g`
     needs exactly this.

### 1.4 `fit`

**Try first.** Replace the placeholder. Centre both lists, call `best_matrix` for `mirror = false`
and `true`, build the two `Transform`s (with the shift of idea step 4), and return the one with the
smaller `node_score`. On a tie, return the rotation.

**Suggested code.** Replace the whole placeholder `fit` (the doc comment and the function):

```rust
/// The transform that moves `q[i]` closest to `g[i]`, over all `i`. Reflections are allowed.
pub fn fit(q: &[Point], g: &[Point]) -> Transform {
    let (mu_q, mu_g) = (centroid(q), centroid(g));
    let q_centred: Vec<Point> = q.iter().map(|&p| p - mu_q).collect();
    let g_centred: Vec<Point> = g.iter().map(|&p| p - mu_g).collect();

    let candidate = |mirror: bool| {
        let r = best_matrix(&q_centred, &g_centred, mirror);
        let turn = Transform {
            r,
            t: Point::new(0.0, 0.0),
        };
        let t = mu_g - turn.apply(mu_q);
        let transform = Transform { r, t };
        let score = node_score(&transform.apply_all(q), g);
        (score, transform)
    };
    let (rotation_score, rotation) = candidate(false);
    let (reflection_score, reflection) = candidate(true);

    // on a tie (all points on a line) prefer the rotation
    if rotation_score <= reflection_score {
        rotation
    } else {
        reflection
    }
}
```

**Explanation.**

1. The `_` prefix is gone from the parameters, because they are used now (Slice 6, Step 5).
2. `candidate` is a closure that captures `q_centred`, `g_centred`, `mu_q`, `mu_g`, `q` and `g`. It
   runs the same steps twice, and the two calls differ in one argument. A closure is the right
   size for this: a separate function would need six parameters.
3. `turn` is the transform with the matrix and **no** shift. `turn.apply(mu_q)` is `mu_q * r`. The
   shift is the difference between where the centroid of `g` is and where `r` sends the centroid of
   `q`. This is `t = mu_G - mu_Q @ R` of the Python code.
4. The score of a candidate is measured on the **original** points (`q`, `g`), with the full
   transform. It is the same number that `align_and_score` uses, so the choice is the right one for
   the final score.
5. The closure returns a pair `(score, transform)`, and the `let (a, b) = ...` lines take it
   apart. Compare `(score, transform)` with a tuple in Python.
6. When all the points of `q` are on one line, the rotation and the reflection fit equally well.
   `<=` chooses the rotation. Python's SVD makes its own choice there, and none of the tests need it.

- [ ] Remove the line `#[ignore = "fit is a placeholder"]` from the four tests, and the two comment
  lines above the first one. Set `IS_PLACEHOLDER` to `false`, and change its doc comment to
  `/// True while `fit` is a placeholder.`
- [ ] Run `cargo test procrustes::`. Expected: `12 passed; 0 failed; 0 ignored`.

  If `fit_recovers_a_known_transform` fails, check the two matrices of Part 1 on `(1, 0)` and
  `(0, 1)` first. If only `fit_recovers_a_reflection` fails, the mirror is on the wrong side: the
  mirror is of `q` (before the rotation), and the matrix is `[[c, s], [s, -c]]`.

- [ ] Run `cargo clippy --all-targets`. Expected: no warning.
- [ ] Commit: `git add src; git commit -m "feat: procrustes fit"`.

---

## Part 2: Distance to a line

### What it must do

`dist_to_line(p, a, b)` is the distance from `p` to the **infinite line** through `a` and `b`. The
Python code (and `dist_to_segment` of Slice 2) uses the **segment**: past the end `b`, the distance
is the distance to `b`. A line has no ends, so a node past the end of the edge, on the line, has
distance `0` (decision D10).

The cross product of `b - a` and `p - a` is the area of the parallelogram that they span. The area
is the base times the height, and the base is `|b - a|`. So the height (the distance) is
`|cross| / |b - a|`. If `a == b`, there is no line, and the function returns the distance to `a`
(as `dist_to_segment` does).

### 2.1 Tests first

- [ ] Add a stub in `src/geometry.rs`, after `dist_to_segment`:

  ```rust
  /// Returns distance from p to the infinite line through a and b.
  /// If a and b are the same point, there is no line: it returns the distance to a.
  pub fn dist_to_line(p: Point, a: Point, b: Point) -> f64 {
      todo!()
  }
  ```

- [ ] Add these tests at the end of `mod tests` in `geometry.rs`. (`assert_close` is already in
  this module.)

  ```rust
      #[test]
      fn dist_to_line_is_the_perpendicular_offset() {
          // the line is the x axis
          let (a, b) = (Point::new(0.0, 0.0), Point::new(4.0, 0.0));

          assert_close(dist_to_line(Point::new(2.0, 3.0), a, b), 3.0);
          assert_close(dist_to_line(Point::new(1.0, -2.0), a, b), 2.0);
      }

      #[test]
      fn dist_to_line_ignores_the_ends_of_the_segment() {
          let (a, b) = (Point::new(0.0, 0.0), Point::new(4.0, 0.0));

          // past b and past a, on the line: distance 0 (to the segment it would be 2 and 3)
          assert_close(dist_to_line(Point::new(6.0, 0.0), a, b), 0.0);
          assert_close(dist_to_line(Point::new(-3.0, 0.0), a, b), 0.0);
          // past b and off the line: only the offset counts (to the segment it would be 5)
          assert_close(dist_to_line(Point::new(7.0, 4.0), a, b), 4.0);
      }

      #[test]
      fn dist_to_line_of_a_slanted_line() {
          // the line y = x, and the point (2, 0): distance sqrt(2)
          let d = dist_to_line(
              Point::new(2.0, 0.0),
              Point::new(0.0, 0.0),
              Point::new(5.0, 5.0),
          );

          assert_close(d, 2.0_f64.sqrt());
      }

      #[test]
      fn dist_to_line_does_not_depend_on_the_direction() {
          let (a, b) = (Point::new(0.0, 1.0), Point::new(3.0, 5.0));
          let p = Point::new(4.0, 0.0);

          assert_close(dist_to_line(p, a, b), dist_to_line(p, b, a));
      }

      #[test]
      fn dist_to_line_of_a_single_point_is_the_point_distance() {
          let a = Point::new(1.0, 1.0);

          assert_close(dist_to_line(Point::new(4.0, 5.0), a, a), 5.0);
      }
  ```

- [ ] Run `cargo test dist_to_line`. Expected: `0 passed; 5 failed`, each with `not yet implemented`.

### 2.2 The code

**Try first.** Four lines. Use `norm` and `cross` (Slice 2) and `abs`.

- [ ] Replace the body:

  ```rust
      let ab = b - a;
      let len = ab.norm();
      if len == 0.0 {
          return p.dist(a);
      }
      ab.cross(p - a).abs() / len
  ```

  1. `len == 0.0` is an exact test on purpose: it is true only when `a` and `b` are the same
     point. A line with a very short `ab` still has a direction, and the division is fine.
  2. `return` early, then the normal case as the last expression (Slice 1, Step 9).
  3. `abs()` is needed because the cross product has a sign: it says on which **side** of the
     line the point is (that is what `side_of` uses). The distance has no side.

- [ ] Run `cargo test dist_to_line`. Expected: `5 passed`.
- [ ] Commit: `git add src; git commit -m "feat: dist_to_line"`.

---

## Part 3: `find_paths`

### What it must do

Read the doc comment of `find_paths` in `src/path_scoring.rs`, and the nine ignored tests below it.
In short: for each edge `(u, v)` of `q.edges()`,

1. `d[i]` is the distance of crop node `i` to the line through `moved_q[u]` and `moved_q[v]`;
2. a **step** between two neighbouring crop nodes is allowed only if both have `d <= tol`
   (the corridor), and it costs `0.5 * (d_a + d_b) * length`, the area of a trapezoid;
3. the cheapest path from `mapping[u]` to `mapping[v]` is found (Dijkstra), and its cost is added;
4. if one edge has no path, the answer is `None` for the whole mapping.

You build it from the bottom: the cost of one step, then the queue item, then the search, then the
loop over the edges.

### 3.1 The cost of a step

**Try first.** `fn step_cost(d_a: f64, d_b: f64, len: f64, tol: f64) -> Option<f64>`. `None` if
either distance is larger than `tol`, else `Some(trapezoid area)`. It is a separate function so
that the rule has a name and a test, and so that the search reads easily.

- [ ] Add before the doc comment of `find_paths` (put the `use` lines of 3.2 at the top now, to avoid
  a later edit, or add them when the compiler asks):

  ```rust
  /// The cost of one step of a path between two nodes of the crop, or `None` if the step is not
  /// allowed. `d_a` and `d_b` are the distances of the two ends to the line, and `len` is the length
  /// of the step. The step is allowed when both ends are within `tol` of the line.
  fn step_cost(d_a: f64, d_b: f64, len: f64, tol: f64) -> Option<f64> {
      todo!()
  }
  ```

- [ ] Add these tests in `mod tests`, after `const SOURCE_TARGET`:

  ```rust
      #[test]
      fn step_cost_is_a_trapezoid() {
          // ends at distance 0 and 2 from the line, a step of length 4: 0.5 * (0 + 2) * 4
          assert_eq!(step_cost(0.0, 2.0, 4.0, 10.0), Some(4.0));
      }

      #[test]
      fn step_cost_is_none_when_an_end_is_outside_the_corridor() {
          assert_eq!(step_cost(0.0, 3.0, 4.0, 2.0), None);
          assert_eq!(step_cost(3.0, 0.0, 4.0, 2.0), None);
      }

      #[test]
      fn step_cost_allows_an_end_exactly_at_tol() {
          assert_eq!(step_cost(2.0, 2.0, 1.0, 2.0), Some(2.0));
      }
  ```

- [ ] Run `cargo test step_cost`. Expected: `0 passed; 3 failed`.
- [ ] Replace the body:

  ```rust
      if d_a > tol || d_b > tol {
          return None;
      }
      Some(0.5 * (d_a + d_b) * len)
  ```

  The test is `>`, so a distance equal to `tol` is inside. Python's gate is the same (`da > tol`).
  The trapezoid rule on unsigned distances counts some extra area when a path crosses the line. Python
  does the same, so the numbers stay comparable.

- [ ] Run `cargo test step_cost`. Expected: `3 passed`.

### 3.2 The queue item

Dijkstra's algorithm keeps a queue of nodes that it has reached. It always takes the one with the
**lowest** cost so far. Rust's `BinaryHeap` is a priority queue, and it gives the **largest** item. And
a `BinaryHeap<T>` needs `T: Ord` (a total order), and `f64` is not `Ord` because of `NaN` (Slice 3,
Step 11). So you write a small type with an `Ord` that you choose: compare the costs with
`total_cmp`, and compare them **the other way round**, so that the lowest cost is the largest item.

**Try first.** `struct State { cost: f64, node: usize }`. Write the four traits that `Ord` needs:
`PartialEq`, `Eq`, `PartialOrd`, `Ord`. Only `cmp` has a real body; the others use it. (A derive
would compare the fields in order, and would not compile for `f64`.)

**Suggested code.** Add the two `use` lines at the top of `path_scoring.rs`, with the others:

```rust
use std::cmp::Ordering;
use std::collections::BinaryHeap;
```

and this before `step_cost`:

```rust
/// A node in the queue of Dijkstra's algorithm. `BinaryHeap` takes the **largest** item, so the
/// order is reversed: the item with the lowest cost is the largest.
struct State {
    cost: f64,
    node: usize,
}

impl PartialEq for State {
    fn eq(&self, other: &State) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for State {}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &State) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for State {
    fn cmp(&self, other: &State) -> Ordering {
        other.cost.total_cmp(&self.cost)
    }
}
```

**Explanation.**

1. `Ordering` is an enum with `Less`, `Equal` and `Greater` (Slice 2, Step 13 explains enums). `cmp`
   returns one of them.
2. `other.cost.total_cmp(&self.cost)` has `other` and `self` in the **reverse** order of the normal
   `self.cost.total_cmp(&other.cost)`. This one line makes the max-heap act as a min-heap.
3. `eq`, `partial_cmp` and `Eq` all follow from `cmp`, so the four traits agree. That is a rule of
   the language: if they disagree, the heap can behave wrongly. `impl Eq for State {}` has an empty
   body: `Eq` only promises that `==` is a full equivalence, and `total_cmp` makes it so.
4. `State` compares only `cost`. Two states with the same cost and different nodes are "equal", and
   the heap may give either one first. That is fine for Dijkstra.
5. This is the usual way to do it. You can also wrap a number in `std::cmp::Reverse`, but `Reverse`
   still needs `Ord` on the inside, so for `f64` you would write the wrapper anyway.

- [ ] Run `cargo build`. Expected: no error. (Warnings that `State` and `step_cost` are not used
  are normal for now.)

### 3.3 The search

**Try first.**

```rust
fn cheapest_path(
    g: &Graph,
    from: usize,
    to: usize,
    d: &[f64],
    tol: f64,
) -> Option<(Vec<usize>, f64)>
```

It returns the path (crop positions, from `from` to `to`) and its cost, or `None`. `d[i]` is the
distance of node `i` to the line. The steps:

1. `best[i]` is the lowest cost found so far to reach node `i`. It starts at infinity, and `0` for
   `from`.
2. `previous[i]` is the node before `i` on the best path found so far. It starts at `None`.
3. Push `from` into the queue. Then, while the queue is not empty, pop the cheapest `(cost, node)`.
4. If `node == to`, build the path by following `previous` back to `from`, reverse it, and return.
5. If `cost > best[node]`, this is an old entry (a cheaper way was found after it was pushed): skip it.
6. Otherwise look at each neighbour: if `step_cost` allows the step and `cost + step < best[next]`,
   update `best` and `previous`, and push the neighbour.
7. If the queue is empty, there is no path: `None`.

Write the tests first.

- [ ] Add a stub after `State`'s impls (before `step_cost`):

  ```rust
  /// The cheapest path from `from` to `to` in `g`, and its cost. `d[i]` is the distance of node `i`
  /// to the line. `None` if there is no path.
  fn cheapest_path(
      g: &Graph,
      from: usize,
      to: usize,
      d: &[f64],
      tol: f64,
  ) -> Option<(Vec<usize>, f64)> {
      todo!()
  }
  ```

- [ ] Add these in `mod tests`, after the `step_cost` tests:

  ```rust
      /// Nodes 0, 1, 2 on the x axis, 3 above node 1. Routes: 0-1-2, and 0-3-2.
      fn diamond() -> Graph {
          graph(
              &[(0, 0.0, 0.0), (1, 2.0, 0.0), (2, 4.0, 0.0), (3, 2.0, 3.0)],
              &[(0, 1), (1, 2), (0, 3), (3, 2)],
          )
      }

      #[test]
      fn cheapest_path_takes_the_lower_cost_route() {
          let g = diamond();
          // distances to the x axis
          let d = [0.0, 0.0, 0.0, 3.0];

          let (path, cost) = cheapest_path(&g, 0, 2, &d, 10.0).unwrap();

          assert_eq!(path, vec![0, 1, 2]);
          assert_close(cost, 0.0);
      }

      #[test]
      fn cheapest_path_avoids_nodes_outside_the_corridor() {
          let g = diamond();
          // node 1 is too far from the line now
          let d = [0.0, 20.0, 0.0, 3.0];

          let (path, cost) = cheapest_path(&g, 0, 2, &d, 10.0).unwrap();

          // 0.5 * (0 + 3) * sqrt(13) for each of the two steps
          assert_eq!(path, vec![0, 3, 2]);
          assert_close(cost, 3.0 * 13.0_f64.sqrt());
      }

      #[test]
      fn cheapest_path_is_none_when_the_target_cannot_be_reached() {
          let g = diamond();
          // both middle nodes are outside the corridor
          let d = [0.0, 20.0, 0.0, 20.0];

          assert_eq!(cheapest_path(&g, 0, 2, &d, 10.0), None);
      }
  ```

- [ ] Run `cargo test cheapest_path`. Expected: `0 passed; 3 failed`.

**Suggested code.** In three pieces. The start:

```rust
    let mut best = vec![f64::INFINITY; g.node_count()];
    let mut previous: Vec<Option<usize>> = vec![None; g.node_count()];
    let mut queue = BinaryHeap::new();

    best[from] = 0.0;
    queue.push(State {
        cost: 0.0,
        node: from,
    });
```

1. `f64::INFINITY` is `float("inf")`. Any real cost is lower.
2. `vec![None; n]` makes `n` copies of `None`. It needs `Clone` on the item, and `Option<usize>` has
   it. The type annotation says what the `None`s are.
3. `BinaryHeap::new()` has no type here. It gets `State` from the first `push` (Slice 5, Step 6).

The loop:

```rust
    while let Some(State { cost, node }) = queue.pop() {
        if node == to {
            let mut path = vec![to];
            let mut at = to;
            while let Some(before) = previous[at] {
                path.push(before);
                at = before;
            }
            path.reverse();
            return Some((path, cost));
        }
        // an old entry: a cheaper way to this node was found after it was pushed
        if cost > best[node] {
            continue;
        }
        for &next in g.neighbors(node) {
            let len = g.coord(node).dist(g.coord(next));
            let Some(step) = step_cost(d[node], d[next], len, tol) else {
                continue;
            };
            if cost + step < best[next] {
                best[next] = cost + step;
                previous[next] = Some(node);
                queue.push(State {
                    cost: cost + step,
                    node: next,
                });
            }
        }
    }
    None
```

4. `while let Some(x) = queue.pop()` repeats until `pop` gives `None` (the queue is empty). It is
   `while queue:` plus `x = heappop(queue)` in Python (Slice 4, Step 10 has the same shape with a
   stack). `State { cost, node }` in the pattern takes the two fields out.
5. The first `if` is the end. The queue gives nodes in order of cost, so the first time that `to` comes
   out, its cost is the lowest possible. Dijkstra needs no more work than this.
6. `while let Some(before) = previous[at]` follows the links back. `previous[from]` is `None`, so the
   loop stops at the start. `path` was built from the end to the start, so `reverse()` fixes it.
   `previous[at]` is an `Option<usize>`, and `usize` is `Copy`, so the pattern copies it out.
7. The "old entry" check is needed because the queue cannot change an item that is already in it.
   The code pushes a **new** entry when it finds a cheaper way, and the old one is skipped when it
   comes out.
8. `let Some(step) = ... else { continue; };` is **let-else**: take the value out of the `Some`, or
   run the `else` block, which must leave the block (`continue`, `return`, ...). It avoids a nesting
   level. It is the guard `if step is None: continue` followed by using `step`.
9. `g.neighbors(node)` is the slice of the adjacency list (Slice 3). The loop borrows `g`, and nothing
   changes `g`, so the borrow is fine.

- [ ] Put the three pieces in the body, in order. (The start, the `while let` loop with its final
  `None`.)
- [ ] Run `cargo test cheapest_path`. Expected: `3 passed`.

### 3.4 `find_paths`

**Try first.** The loop of the four steps at the start of Part 3. Use `q.edges()`, `dist_to_line` and
`cheapest_path`. Collect the distances with one `map` over `g.coords()`. Use `?` to return `None` when an
edge has no path (Slice 7, Step 3c).

**Suggested code.** Replace the placeholder (keep the doc comment, but change its first line to
`/// Finds the best path in `g` for every edge of `q`.`), and add the import of `dist_to_line`:

```rust
use crate::geometry::{Point, dist_to_line};
```

```rust
pub fn find_paths(
    q: &Graph,
    g: &Graph,
    mapping: &[usize],
    moved_q: &[Point],
    tol: f64,
) -> Option<EdgeRoutes> {
    let mut cost = 0.0;
    let mut paths = Vec::new();

    for (u, v) in q.edges() {
        let d: Vec<f64> = g
            .coords()
            .iter()
            .map(|&p| dist_to_line(p, moved_q[u], moved_q[v]))
            .collect();

        let (path, path_cost) = cheapest_path(g, mapping[u], mapping[v], &d, tol)?;
        cost += path_cost;
        paths.push(path);
    }
    Some(EdgeRoutes { cost, paths })
}
```

1. The parameters have no `_` now. This is the end of the placeholder.
2. `moved_q[u]` and `moved_q[v]` are the two ends of the moved edge. `u` and `v` are positions in `q`,
   and `mapping[u]` is the crop node that `u` is mapped to. Two different tables, two different
   jobs.
3. `?` on the `Option` returns `None` from `find_paths` at the first edge without a path. The
   other edges are not tried: a mapping with one bad edge is rejected.
4. The `d` vector has one number for each node of the **whole crop**, for each query edge, for each
   mapping. This is correct and simple, and it is a good thing to look at in the profile (Slice 9):
   most of the crop is usually far from the line, and the search never looks at it.
5. `paths` has one path for each edge, in the order of `q.edges()`, as the doc comment of
   `EdgeRoutes` says.

- [ ] Delete the test `placeholder_gives_cost_zero_and_no_paths`. Remove the lines
  `#[ignore = "find_paths is a placeholder"]` from the nine tests, and the two comment lines above the
  first. Set `IS_PLACEHOLDER` to `false`, and change its doc comment to
  `/// True while `find_paths` is a placeholder.`
- [ ] Run `cargo test path_scoring::`. Expected: `15 passed; 0 failed; 0 ignored` (3 for `step_cost`,
  3 for `cheapest_path`, 9 for `find_paths`).

  If `a_node_past_the_end_is_free_on_a_line` fails but the others pass, `find_paths` uses the segment
  distance somewhere. If `detour_cost_follows_the_trapezoid_rule` fails, look at `step_cost` and the
  length of the step.

- [ ] Commit: `git add src; git commit -m "feat: find_paths with dijkstra and the corridor"`.

---

## Part 4: Everything together

- [ ] In `src/alignment.rs`, remove the three lines `#[ignore = "fit and find_paths are placeholders"]`
  and the two comment lines above the first one.
- [ ] Run `cargo test`. Expected: `116 passed; 0 failed; 0 ignored`.
- [ ] Run `cargo clippy --all-targets`. Expected: no warning.
- [ ] Run the six small graphs. In PowerShell, from the repo folder:

  ```powershell
  foreach ($id in "gg00041", "gd00040", "go00038", "go00028", "gg00016", "gd00032") {
      cargo run --release -q -- ..\gsearch\data\db\queries\${id}_q.json ..\gsearch\data\db\graphs\$id.json -k 3 2>$null
  }
  ```

  Expected (the times differ). There is no `warning` line any more:

  ```text
  gg00041_q in gg00041: 78 anchors, 1 mappings, 1 alignments kept (0.000 s)
  lowest score: 0
  gd00040_q in gd00040: 98 anchors, 2 mappings, 2 alignments kept (0.004 s)
  lowest score: 0.0000000000486044298426844
  go00038_q in go00038: 100 anchors, 4 mappings, 2 alignments kept (0.014 s)
  lowest score: 0.00000000045928002558352944
  go00028_q in go00028: 103 anchors, 2 mappings, 1 alignments kept (0.012 s)
  lowest score: 0.00000000006537983576915065
  gg00016_q in gg00016: 113 anchors, 1 mappings, 1 alignments kept (0.001 s)
  lowest score: 0.00000000007842531231752922
  gd00032_q in gd00032: 115 anchors, 1 mappings, 1 alignments kept (0.001 s)
  lowest score: 0.000000000020491267535862764
  ```

  Each query was cut out of its own graph, so the best alignment is an exact copy: every score is
  below `1e-6`. The last digits can differ on your machine (they are rounding noise near zero).
  `alignments kept` is smaller than `mappings` when a mapping has an edge with no path: for
  `go00038`, 2 of the 4 mappings are rejected.

- [ ] Run `cargo run --release -q -- ..\gsearch\data\db\queries\gg00041_q.json ..\gsearch\data\db\graphs\gg00041.json -o out.json`,
  and look at the file (`python -c "import json; print(json.load(open('out.json'))['results'][0])"`).
  Expected: `transform` is no longer the identity, `edges` is no longer empty, and
  `placeholder_scoring` is `false`.

### Compare with Python

Python scores the same queries too. The lowest scores are tiny in both, so the check is that both
are close to zero. From the `gsearch` folder:

```powershell
cd ..\gsearch
foreach ($id in "gg00041", "gd00040", "go00038") {
    uv run python -c "import sys; from gsearch.cli import gsearch; sys.argv = ['gsearch', 'data/db/queries/${id}_q.json', 'data/db/graphs/$id.json', '-k', '3']; gsearch()" 2>&1 | Select-String "Lowest"
}
cd ..\gsearch_rs
```

Expected: `1.84e-11`, `1.26e-10` and `4.12e-10`. Python and Rust differ in the last digits, because the SVD
and the closed form round differently. Do not compare them to more than about `1e-6`. The scores of
a real, noisy query will also differ for a second reason: D10 (Python measures to the segment, this
port to the line).

### Update the README, and profile again

- [ ] In `guide/README.md`, change the status of rows 6 and 7 to `done`, and delete the section
  "Finishing the placeholders" (or leave it as a record).
- [ ] Do Slice 9 again with the real `find_paths`. The profile that it gave with the placeholder is
  out of date: Dijkstra and the `d` vector of Part 3.4 are new. Use `-e 8` on `gr00049` first (21 400
  mappings), and `-e 9` only when you have a baseline. Do not run `-e 10`.
- [ ] Commit: `git add src guide; git commit -m "feat: real scoring, placeholders removed"`.
