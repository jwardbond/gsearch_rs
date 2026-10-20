# Slice 6: The rigid transform (with a placeholder fit)

**Goal.** Port the first half of `align_and_score`: move the query `Q` onto the crop with a
**transform**, and measure how far the moved query nodes are from their mapped crop nodes.
You write the type, the way to apply it, and the node score. You do **not** write the fit
(`_align_graph`, the Procrustes step) in this slice. `fit` is a **placeholder** that returns the
identity transform, so that Slices 7 and 8 run from start to end. The slice gives you tests for
the real `fit`, marked `#[ignore]`, so that you can write it later.

**Outcome.** `cargo test procrustes` reports `8 passed; 0 failed; 4 ignored`. The command
`cargo test procrustes -- --ignored` reports `1 passed; 3 failed`: those are the tests that wait
for your `fit`.

**New ideas.** An array of arrays (`[[f64; 2]; 2]`); an associated `const` inside an `impl`;
`pub` fields; `sin_cos` and `to_radians`; `#[ignore]` and `cargo test -- --ignored`; a parameter
that starts with `_` (why the compiler does not warn); a placeholder that is marked so that it
cannot be forgotten.

**Starting point.** Slice 5 is done: `get_candidate_matches` works and its tests pass. Notes
about your files, from when I last read them:

- Your type is still called `Pose`. The glossary now says **mapping** (see `GLOSSARY.md`).
  Rename it: `pub type Mapping = Vec<usize>;`, and in the same file `poses` to `mappings`.
  Slice 5 is already changed in the same way. This slice and the next ones use `Mapping`.
- `get_candidate_matches` ended with `todo!()` after the line that calls `pick_alignment_node`.
  Finish Slice 5, Steps 5 to 8, first. Slice 5, Step 5 picks the alignment node with the
  **fewest** matches (as Python does), and your `pick_alignment_node` picks the node that is
  **farthest** from the anchor. These are two different rules, and they can give different
  mappings. Slice 8 compares your mapping counts with Python, and the counts agree only with the
  Slice 5 rule.
- `generate_candidates_is_injective` and `generate_candidates_finds_all_permutations` failed.
  The loop in `extend_poses` does not skip a column that is already in the mapping. Add
  `if mapping.contains(&j) { continue; }` at the top of the loop body. Slice 5, Step 6 explains
  it.
- Run `cargo test`, and write down how many tests pass. Slice 6 adds 12 tests to that number.

---

## The design

### What the Python code does

`align_and_score` first calls `_align_graph(Q_arr, G_arr_matched)`. Both arrays have one row per
node of `Q`, and row `i` of `G_arr_matched` is the crop node that is mapped to row `i` of `Q_arr`.

1. Subtract the mean of each array (the centroid).
2. `H = G_centered.T @ Q_centered`, a 2 by 2 matrix.
3. `U, _, Vt = svd(H)`, then `R = Vt.T @ U.T`. This is the best **orthogonal** matrix, so a
   reflection (`det R == -1`) is allowed. The matches that `get_candidate_matches` puts in the
   reflection table need this.
4. `t = mu_G - mu_Q @ R`, and `Q_aligned = Q_arr @ R + t`.

Then `node_score = np.linalg.norm(Q_aligned - G_arr_matched)`. The Frobenius norm of the
difference is the square root of the sum of **all** squared coordinate differences.

### What the Rust code looks like

| Python | Rust |
|---|---|
| `R` (2 by 2 array) and `t` (length 2) | `Transform { r: [[f64; 2]; 2], t: Point }` |
| `Q_arr @ R + t` | `transform.apply_all(&points)` |
| `np.linalg.norm(A - B)` | `node_score(&moved, &target)` |
| `_align_graph(Q_arr, G_arr)` | `fit(&q_points, &g_points) -> Transform` (placeholder) |

**A point is a row vector.** Python writes `Q_arr @ R`, so each point is a row, and `R` acts from
the right. The Rust `apply` keeps this: `p` moves to `p * r + t`. If you keep this rule, your
`fit` can follow the Python code line by line.

**The placeholder (a decision for this guide).** `fit` returns `Transform::IDENTITY`. The
transform then moves nothing, and `node_score` measures the distance between the original `Q`
coordinates and the crop. `Q` is near the origin and `G` is far away, so these scores are large
and have no meaning. A pipeline with a placeholder cannot rank results, but it can count
anchors and mappings, and Slice 9 profiles exactly that.

To make sure that nobody mistakes a placeholder score for a real one, the module has a constant,
`IS_PLACEHOLDER`. The CLI (Slice 8) reads it and prints a warning. When you write `fit`, set it
to `false`.

**Notes for when you write `fit`.** The decision D4 says: no matrix crate. In 2D, the best
rotation has a closed form with `atan2` of two sums over the centred points (a dot-product sum
and a cross-product sum). The best reflection is the same problem with `Q` mirrored (negate
`y`). Fit both, move `Q` with each, and keep the one with the smaller `node_score`. You can also
write the 2 by 2 SVD by hand. Remember the row-vector rule above. The four ignored tests say
what must be true.

---

## Steps

### 1. The module and the `Transform` type

- [ ] Create `src/procrustes.rs`:

  ```rust
  //! The rigid transform from a query to its mapped nodes (`_align_graph` in Python).

  use crate::geometry::Point;

  /// True while `fit` is a placeholder. Set it to `false` when you write `fit`.
  pub const IS_PLACEHOLDER: bool = true;

  /// A rotation or reflection `r` and a translation `t`.
  ///
  /// Like `Q @ R + t` in Python, a point is a row vector: `p` moves to `p * r + t`.
  #[derive(Debug, Clone, Copy, PartialEq)]
  pub struct Transform {
      pub r: [[f64; 2]; 2],
      pub t: Point,
  }

  impl Transform {
      /// Moves nothing.
      pub const IDENTITY: Transform = Transform {
          r: [[1.0, 0.0], [0.0, 1.0]],
          t: Point { x: 0.0, y: 0.0 },
      };
  }
  ```

  1. `[[f64; 2]; 2]` is an array of two arrays of two numbers. A `[f64; 2]` came in Slice 4,
     Step 5. An array has a fixed size that is part of its type, and it lives inline (no heap),
     so a `Transform` is 48 bytes and is `Copy`. `r[0][1]` is `R[0, 1]` in numpy.
  2. `const IDENTITY` inside `impl Transform` is an **associated constant**. You write it as
     `Transform::IDENTITY`, like `Transform.IDENTITY` as a class attribute in Python. A `const`
     is computed when the program is compiled, so its fields must be simple values. You wrote
     struct literals in a `const` in Slice 5, Step 2. `Point { x: 0.0, y: 0.0 }` is allowed
     here because the fields of `Point` are `pub`. (`Point::new` would also work in a `const`
     only if it were a `const fn`.)
  3. `pub` fields: a `Transform` has no rule to keep (any `r` and `t` are valid), so there is
     no reason to hide the fields. The private fields of `Graph` (Slice 3, Step 2) protect a
     rule. This type has none.
  4. `PartialEq` compares two `Transform`s field by field. It works for `f64` fields, and it
     is the reason for `PartialEq` and not `Eq` (Slice 2, Step 14).

- [ ] Add the module to `src/lib.rs`: `pub mod procrustes;`.
- [ ] Run `cargo build`. Expected: it finishes without an error. Warnings about unused items
  are normal for now.

### 2. `apply`

**Try first.** Write `pub fn apply(&self, p: Point) -> Point`. It uses the rule
`p * r + t` with a row vector `p = (x, y)`:

```text
x' = x * r[0][0] + y * r[1][0] + t.x
y' = x * r[0][1] + y * r[1][1] + t.y
```

First write the tests, and watch them fail.

- [ ] Add `apply` as a stub inside `impl Transform`, after `IDENTITY`:

  ```rust
      pub fn apply(&self, p: Point) -> Point {
          todo!()
      }
  ```

- [ ] Add a test module at the end of `src/procrustes.rs`:

  ```rust
  #[cfg(test)]
  mod tests {
      use super::*;
      use crate::test_fixtures::assert_close;

      fn assert_point_close(actual: Point, expected: Point) {
          assert_close(actual.x, expected.x);
          assert_close(actual.y, expected.y);
      }

      #[test]
      fn identity_moves_nothing() {
          let p = Point::new(3.0, -2.0);

          assert_eq!(Transform::IDENTITY.apply(p), p);
      }

      #[test]
      fn apply_multiplies_a_row_vector_then_shifts() {
          let t = Transform {
              r: [[0.0, -1.0], [1.0, 0.0]],
              t: Point::new(10.0, 20.0),
          };

          let moved = t.apply(Point::new(3.0, 0.0));

          // (3, 0) * R is (0, -3), and the shift adds (10, 20)
          assert_point_close(moved, Point::new(10.0, 17.0));
      }

      #[test]
      fn apply_uses_both_rows_of_r() {
          let t = Transform {
              r: [[0.0, -1.0], [1.0, 0.0]],
              t: Point::new(0.0, 0.0),
          };

          let moved = t.apply(Point::new(0.0, 4.0));

          // (0, 4) * R is (4, 0)
          assert_point_close(moved, Point::new(4.0, 0.0));
      }
  }
  ```

  1. `assert_close` is in your shared `test_fixtures` module (Slice 4, Step 1). It compares two
     `f64` with a small margin, because `==` on floats breaks on rounding.
  2. The tests use a matrix that is not symmetric. A symmetric matrix would hide a mistake
     where you use `r[0][1]` and `r[1][0]` the wrong way round. The second test uses a point
     on the `y` axis, so a wrong row also shows.

- [ ] Run `cargo test procrustes`. Expected: `0 passed; 3 failed`. Each failure says
  `not yet implemented` (that is the message of `todo!()`).

- [ ] Replace the body of `apply`:

  ```rust
      pub fn apply(&self, p: Point) -> Point {
          Point::new(
              p.x * self.r[0][0] + p.y * self.r[1][0] + self.t.x,
              p.x * self.r[0][1] + p.y * self.r[1][1] + self.t.y,
          )
      }
  ```

  `&self` borrows the transform and does not move it (Slice 2, Step 4). `Point` is `Copy`, so
  `p` is a copy, and the caller still has its point.

- [ ] Run `cargo test procrustes`. Expected: `3 passed; 0 failed`.

### 3. `apply_all`

`apply_all` moves every point of a slice. It is the `Q_arr @ R + t` of the Python code.

- [ ] Add a test inside `mod tests`, after the three tests, and a `TRIANGLE` constant at the top
  of `mod tests` (after `assert_point_close`):

  ```rust
      /// A 3-4-5 triangle: all three distances differ, so only one rigid transform fits it.
      const TRIANGLE: [Point; 3] = [
          Point { x: 0.0, y: 0.0 },
          Point { x: 3.0, y: 0.0 },
          Point { x: 0.0, y: 4.0 },
      ];
  ```

  ```rust
      #[test]
      fn apply_all_keeps_the_order() {
          let t = Transform {
              r: [[1.0, 0.0], [0.0, 1.0]],
              t: Point::new(1.0, 1.0),
          };

          let moved = t.apply_all(&TRIANGLE);

          assert_eq!(moved.len(), 3);
          assert_point_close(moved[0], Point::new(1.0, 1.0));
          assert_point_close(moved[2], Point::new(1.0, 5.0));
      }
  ```

- [ ] Add the method to `impl Transform`, after `apply`:

  ```rust
      pub fn apply_all(&self, points: &[Point]) -> Vec<Point> {
          points.iter().map(|&p| self.apply(p)).collect()
      }
  ```

  1. `&[Point]` is a slice (Slice 3, Step 7). The caller can pass a `Vec<Point>`, an array, or
     `graph.coords()` without a copy.
  2. `|&p|` takes the reference that `iter()` gives and copies the `Point` out of it, as in
     Slice 5, Step 7. The closure uses `self`. A closure can use the variables around it, so
     `self` does not need to be passed (Slice 3, Step 8).

- [ ] Run `cargo test procrustes`. Expected: `4 passed; 0 failed`.

### 4. `node_score`

**Try first.** Write `pub fn node_score(moved: &[Point], target: &[Point]) -> f64`. It is
`np.linalg.norm(moved - target)`: the square root of the sum of the squared offsets. Not a mean,
and not squared. Use `zip`, `map`, `sum::<f64>()` and `sqrt` (Slice 3, Steps 8 and 10).

- [ ] Add a stub before `mod tests`:

  ```rust
  /// The length of the vector of all the offsets between `moved[i]` and `target[i]`.
  ///
  /// This is `np.linalg.norm(moved - target)`: it is not squared, and it is not a mean.
  pub fn node_score(moved: &[Point], target: &[Point]) -> f64 {
      todo!()
  }
  ```

- [ ] Add three tests to `mod tests`:

  ```rust
      #[test]
      fn node_score_is_zero_for_equal_points() {
          assert_close(node_score(&TRIANGLE, &TRIANGLE), 0.0);
      }

      #[test]
      fn node_score_is_the_norm_of_all_offsets() {
          let target = [Point::new(3.0, 0.0), Point::new(0.0, 4.0)];
          let moved = [Point::new(0.0, 0.0), Point::new(0.0, 0.0)];

          // offsets of length 3 and 4 give sqrt(3^2 + 4^2)
          assert_close(node_score(&moved, &target), 5.0);
      }

      #[test]
      fn node_score_is_not_a_mean() {
          let target = [Point::new(3.0, 0.0), Point::new(0.0, 3.0)];
          let moved = [Point::new(0.0, 0.0), Point::new(0.0, 0.0)];

          // two offsets of length 3 give sqrt(18), not 3
          assert_close(node_score(&moved, &target), 18.0_f64.sqrt());
      }
  ```

- [ ] Run `cargo test procrustes`. Expected: `4 passed; 3 failed`.

- [ ] Replace the body of `node_score`:

  ```rust
      moved
          .iter()
          .zip(target)
          .map(|(&a, &b)| (a - b).dot(a - b))
          .sum::<f64>()
          .sqrt()
  ```

  1. `zip(target)` pairs the two slices item by item, and stops at the shorter one. `target`
     is a `&[Point]`, and `zip` takes anything that can be iterated, so you do not write
     `target.iter()`.
  2. `|(&a, &b)|` takes the pair apart and copies both points in the same pattern.
  3. `(a - b).dot(a - b)` is the squared length of the offset. It uses `Sub` for `Point`
     (Slice 2, Step 7) and your `dot`. `norm` would take a square root for each point, and
     then you would square it again.
  4. `sum::<f64>()` adds all the squares. The type must be given here (Slice 3, Step 8). A
     single `sqrt` at the end gives the Frobenius norm.

- [ ] Run `cargo test procrustes`. Expected: `7 passed; 0 failed`.

### 5. The placeholder `fit`, and the tests for the real one

- [ ] Add after `node_score`:

  ```rust
  /// PLACEHOLDER. The transform that moves `q[i]` closest to `g[i]`, over all `i`.
  pub fn fit(_q: &[Point], _g: &[Point]) -> Transform {
      Transform::IDENTITY
  }
  ```

  A parameter whose name starts with `_` tells the compiler that it is not used on purpose.
  Without the `_`, you get a warning, `unused variable: q`. When you write `fit`, remove the
  `_` and the placeholder line, and set `IS_PLACEHOLDER` to `false`.

- [ ] Add these tests at the end of `mod tests`. They are the Python `TestAlignGraph`, without
  the tests that Rust does not need (the shape of the result, and "does not change its input":
  the type `Transform` fixes the first, and `&[Point]` cannot be changed, so the borrow rules
  give the second):

  ```rust
      /// The transform that the Python tests use: a 30 degree turn, then a shift.
      fn known() -> Transform {
          let (s, c) = 30.0_f64.to_radians().sin_cos();
          Transform {
              r: [[c, -s], [s, c]],
              t: Point::new(100.0, 250.0),
          }
      }

      #[test]
      fn placeholder_fit_gives_the_identity() {
          let g = known().apply_all(&TRIANGLE);

          assert_eq!(fit(&TRIANGLE, &g), Transform::IDENTITY);
      }

      // The tests below are the target for your `fit`. They are ignored until you write it.
      // Run them with `cargo test procrustes -- --ignored`.

      #[test]
      #[ignore = "fit is a placeholder"]
      fn fit_recovers_a_known_transform() {
          let g = known().apply_all(&TRIANGLE);

          let t = fit(&TRIANGLE, &g);

          // the exact transform comes back from the paired points
          let expected = known();
          for i in 0..2 {
              for j in 0..2 {
                  assert_close(t.r[i][j], expected.r[i][j]);
              }
          }
          assert_point_close(t.t, expected.t);
      }

      #[test]
      #[ignore = "fit is a placeholder"]
      fn fit_moves_q_onto_g() {
          let g = known().apply_all(&TRIANGLE);

          let moved = fit(&TRIANGLE, &g).apply_all(&TRIANGLE);

          // every moved point lands on its pair
          for (m, g) in moved.iter().zip(&g) {
              assert_point_close(*m, *g);
          }
      }

      #[test]
      #[ignore = "fit is a placeholder"]
      fn fit_recovers_a_reflection() {
          // flip the y axis of Q, then apply the known transform
          let mirrored: Vec<Point> = TRIANGLE.iter().map(|p| Point::new(p.x, -p.y)).collect();
          let g = known().apply_all(&mirrored);

          let t = fit(&TRIANGLE, &g);

          // reflections are allowed, so the mirrored G still fits exactly
          for (m, g) in t.apply_all(&TRIANGLE).iter().zip(&g) {
              assert_point_close(*m, *g);
          }
          // and r is a reflection: its determinant is -1
          let det = t.r[0][0] * t.r[1][1] - t.r[0][1] * t.r[1][0];
          assert_close(det, -1.0);
      }

      #[test]
      #[ignore = "fit is a placeholder"]
      fn fit_is_the_identity_when_q_equals_g() {
          let t = fit(&TRIANGLE, &TRIANGLE);

          // nothing to turn and nothing to shift
          assert_eq!(t.r, [[1.0, 0.0], [0.0, 1.0]]);
          assert_point_close(t.t, Point::new(0.0, 0.0));
      }
  ```

  1. `to_radians` and `sin_cos` are methods on `f64`. `sin_cos` returns the pair `(sin, cos)`
     from one call. `30.0_f64` gives the literal its type, so that Rust knows which `f64`
     method to call.
  2. `#[ignore = "..."]` stops `cargo test` from running a test. It still compiles, so it
     cannot go out of date without you seeing it. The text is shown in the output.
  3. `cargo test -- --ignored` runs **only** the ignored tests. `-- --include-ignored` runs
     all tests. The first `--` ends the arguments for `cargo` and starts the arguments for the
     test program.
  4. `placeholder_fit_gives_the_identity` is the one test that **must be deleted** when you
     write `fit`. It checks the placeholder, and a real `fit` fails it.
  5. In `fit_recovers_a_reflection`, the test does not compare `r` with a fixed matrix. A
     reflection fits in more than one way when `Q` has a symmetry, and the triangle is chosen
     to have none. The test checks what matters: the moved points land on `G`, and `det r` is
     `-1`. The Python test does the same.
  6. `fit_is_the_identity_when_q_equals_g` passes with the placeholder, and it will pass with a
     good `fit`. It is in the ignored group because it passes for the wrong reason now.

- [ ] Run `cargo test procrustes`. Expected: `8 passed; 0 failed; 4 ignored`.
- [ ] Run `cargo test procrustes -- --ignored`. Expected: `1 passed; 3 failed`. The test that
  passes is `fit_is_the_identity_when_q_equals_g`. The failure of `fit_recovers_a_known_transform`
  reads `expected 0.8660254037844387, got 1`: that is `cos 30` against the `1` of the identity.

### 6. Look at it

- [ ] Run `cargo test`. Expected: the number that you wrote down at the start, plus 12 tests
  (8 passed and 4 ignored), and `0 failed`.
- [ ] Run `cargo clippy --all-targets`. Expected: no warning from `procrustes.rs`.

### 7. Format, lint, commit

- [ ] Run `cargo fmt; cargo clippy --all-targets`.
- [ ] Commit:

  ```powershell
  git add src guide GLOSSARY.md
  git commit -m "feat: transform, node score, placeholder fit"
  ```

---

## What comes next

Slice 7 uses `Transform` and `node_score` in `align_and_score`. It also adds the second
placeholder, `find_paths`, and the code that puts an `Alignment` together.

**When you write `fit` yourself**, do it in this order: remove the `_` from the parameters; delete
`placeholder_fit_gives_the_identity`; run `cargo test procrustes -- --ignored` and make the four
tests pass one by one; set `IS_PLACEHOLDER` to `false`; remove the `#[ignore]` lines. The tests
`a_moved_copy_scores_zero` and `perfect_alignment_costs_zero` of Slice 7 need your `fit` and your
`find_paths` together.
