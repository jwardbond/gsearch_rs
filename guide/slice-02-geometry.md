# Slice 2: Geometry

**Goal.** Port the small geometry functions that the matching code uses: Euclidean distance
(`core._euclidean_distance`), distance from a point to a segment (`align_edges._dist_to_segment`),
and the side of an axis that a point is on (`align_nodes._cross_signs`). They all use a new
`Point` type. This slice does not depend on decision D5.

**Outcome.** `cargo test geometry` runs the ported pytest cases and reports `10 passed`.
`cargo doc --open` shows your `Point` and `Side` types in a browser.

**New ideas.** `Copy`, `Clone` and moves; `impl` blocks and methods; associated functions
(`Point::new`); traits and operator overloading (`Sub`, `Add`, `Mul`); iterators with `map` and
`collect`; closures; `const`; why Rust has no default arguments; enums and `match`; `Eq` vs
`PartialEq`.

**Starting point.** Slice 1 is done, and `cargo test` shows `2 passed`.

---

## Why one point at a time?

The Python functions take an `(N, 2)` array and return `N` results, because a Python loop
over `N` points is slow and numpy is fast. In Rust, a loop is compiled to machine code, so it
is as fast as numpy and needs no temporary arrays (decision D4). The Rust functions therefore
take **one** point. When you need `N` results, you loop over the points or use an iterator
(Step 10). The functions also become simpler, because each one does one calculation.

---

## Steps

### 1. Add the module

- [ ] In `src/lib.rs`, add a line, so the file is:

  ```rust
  pub mod geometry;
  pub mod io;
  ```

- [ ] Make an empty file `src/geometry.rs`.

### 2. The `Point` type

**Try first.** In `src/geometry.rs`, write a public struct `Point` with public `x` and `y`
fields of type `f64`. Derive `Debug` as in Slice 1, Step 4.

**Suggested code.**

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}
```

There are three new derives.

1. `PartialEq` lets you compare two points with `==`. Tests need it for `assert_eq!`.
2. `Clone` adds a `.clone()` method that makes a full copy, like `copy.deepcopy`.
3. `Copy` says that a plain copy of the bytes is a correct copy. With `Copy`, `let b = a;`
   copies the point, and both `a` and `b` stay usable. `Copy` needs `Clone` too, so you
   always derive both together.

Why does `Copy` matter? Step 3 shows what happens without it.

### 3. Experiment: moves

- [ ] Temporarily remove `Clone, Copy, ` from the derive line, so it is
  `#[derive(Debug, PartialEq)]`.

- [ ] Add this temporary test at the end of `src/geometry.rs`:

  ```rust
  #[test]
  fn experiment() {
      let a = Point { x: 1.0, y: 2.0 };
      let b = a;
      println!("{a:?} {b:?}");
  }
  ```

- [ ] Run `cargo test`. Expected: the build fails.

  ```text
  error[E0382]: borrow of moved value: `a`
    |
    |     let a = Point { x: 1.0, y: 2.0 };
    |         - move occurs because `a` has type `Point`, which does not implement the `Copy` trait
    |     let b = a;
    |             - value moved here
    |     println!("{a:?} {b:?}");
    |                ^ value borrowed here after move
  ```

This is the main rule of ownership. In Python, `b = a` gives the same object a second name.
In Rust, `b = a` **moves** the value: `b` is now the owner, and you cannot use `a` again. Then
each value always has exactly one owner, and Rust knows exactly when to free it, without a
garbage collector.

A move is the right default for a type like `GraphFile`. Its `Vec`s own large blocks of
memory, and a copy would be expensive. You make that copy with `.clone()`, and the call
shows in the code. A `Point` is only two floats (16 bytes), so a copy costs nothing. That is
why we mark it `Copy`.

- [ ] Put `Clone, Copy, ` back in the derive line, and run `cargo test` again. Expected: it
  builds, and `3 passed`.
- [ ] Delete the `experiment` test.

### 4. Methods

In Rust, methods are not written inside the struct. They go in a separate `impl` block.

- [ ] Add this after the struct:

  ```rust
  impl Point {
      pub fn new(x: f64, y: f64) -> Point {
          Point { x, y }
      }

      pub fn dot(self, other: Point) -> f64 {
          self.x * other.x + self.y * other.y
      }

      pub fn cross(self, other: Point) -> f64 {
          self.x * other.y - self.y * other.x
      }
  }
  ```

1. `new` has no `self`, so it is an **associated function**, like a `@classmethod` or
   `@staticmethod`. You call it as `Point::new(1.0, 2.0)`. Rust has no constructors, and
   `new` is only a naming convention.
2. `Point { x, y }` is short for `Point { x: x, y: y }`, because the variable names are the
   same as the field names.
3. `dot(self, ...)` is a method, called as `a.dot(b)`. In Python, `self` is always a
   reference to the object. In Rust, you choose. `self` takes the value (here it is a copy,
   because `Point` is `Copy`), `&self` borrows it, and `&mut self` borrows it and can change
   it. For small `Copy` types, `self` is normal and is the fastest option.
4. `cross` is the z part of the 3D cross product. It is positive when `other` points
   counter-clockwise from `self`. `_cross_signs` uses the same expression.

### 5. Stubs and the first test

- [ ] Add two more methods inside the `impl Point` block:

  ```rust
      pub fn norm(self) -> f64 {
          todo!()
      }

      pub fn dist(self, other: Point) -> f64 {
          todo!()
      }
  ```

- [ ] Add the test module at the end of the file:

  ```rust
  #[cfg(test)]
  mod tests {
      use super::*;

      fn assert_close(actual: f64, expected: f64) {
          assert!(
              (actual - expected).abs() < 1e-9,
              "expected {expected}, got {actual}"
          );
      }

      #[test]
      fn dist_is_euclidean() {
          let d = Point::new(1.0, 1.0).dist(Point::new(4.0, 5.0));

          // a 3-4-5 triangle
          assert_close(d, 5.0);
      }
  }
  ```

`assert_close` replaces `np.allclose`. Float results must not be compared with `==`. A test
module can contain helper functions without `#[test]`, like a plain function in a pytest
file. `assert!` takes extra arguments after the condition, used as a format string for the
failure message. Floats have methods, so you write `x.abs()`, not `abs(x)`.

### 6. Watch it fail

- [ ] Run `cargo test`. Expected: `2 passed; 1 failed`, and the failure is
  `not yet implemented`. You also see warnings about unused variables in the stubs.

### 7. Subtraction with a trait

`dist` is "the length of `self - other`". To use `-` on points, `Point` must implement the
`Sub` **trait**.

- [ ] Add this line at the top of the file:

  ```rust
  use std::ops::Sub;
  ```

- [ ] Add this after the `impl Point` block:

  ```rust
  impl Sub for Point {
      type Output = Point;

      fn sub(self, other: Point) -> Point {
          Point::new(self.x - other.x, self.y - other.y)
      }
  }
  ```

A **trait** is a set of methods that a type can promise to have. It is like a
`typing.Protocol`, but you implement it with an explicit `impl Trait for Type` block.
Operators are traits: `a - b` calls `Sub::sub(a, b)`, as Python's `a - b` calls
`a.__sub__(b)`. `type Output = Point;` says what type `a - b` gives. It does not have to be
the same as the inputs.

You already used traits: `Debug`, `Clone`, `Copy` and `Deserialize` are traits, and
`#[derive]` writes their `impl` blocks for you. `Box<dyn Error>` in Slice 1 means "a value of
some type that implements the `Error` trait".

- [ ] Now replace the two stubs:

  ```rust
      pub fn norm(self) -> f64 {
          self.dot(self).sqrt()
      }

      pub fn dist(self, other: Point) -> f64 {
          (self - other).norm()
      }
  ```

`self.dot(self)` uses `self` twice. That is allowed only because `Point` is `Copy`. Without
`Copy`, the first use would move it. `norm` is `sqrt(x² + y²)`, the same calculation as the
Python `((x1 - x2) ** 2 + (y1 - y2) ** 2) ** 0.5`.

### 8. Watch it pass

- [ ] Run `cargo test`. Expected: `3 passed`, with no warnings.

### 9. Add and scale

`dist_to_segment` needs `a + ab * t`: a point plus a point, and a point times a number.

**Try first.** Implement `Add` for `Point` (it is the same pattern as `Sub`), and `Mul<f64>`
for `Point`, so that `point * 2.0` scales both coordinates.

**Suggested code.** Change the `use` line to `use std::ops::{Add, Mul, Sub};` and add:

```rust
impl Add for Point {
    type Output = Point;

    fn add(self, other: Point) -> Point {
        Point::new(self.x + other.x, self.y + other.y)
    }
}

impl Mul<f64> for Point {
    type Output = Point;

    fn mul(self, k: f64) -> Point {
        Point::new(self.x * k, self.y * k)
    }
}
```

`Mul<f64>` means "multiply a `Point` by an `f64`". The `<f64>` is a **generic parameter** of
the trait: `Mul` is written once for every pair of types, and you choose the pair. `Sub` and
`Add` also have this parameter, but it is the same type as `Self` by default, so you do not
have to write it. With this impl, `point * 2.0` works but `2.0 * point` does not. That would
need a second impl, `Mul<Point> for f64`, and we do not need it.

- [ ] Add a stub below the `impl` blocks (not inside one, because it is a plain function):

  ```rust
  /// Distance from `p` to the closest point on the segment `a`-`b`.
  pub fn dist_to_segment(p: Point, a: Point, b: Point) -> f64 {
      todo!()
  }
  ```

### 10. Port the `TestDistToSegment` cases

These are the cases from `tests/test_align_edges.py::TestDistToSegment`. The first Python test
checks that one distance comes back for each row. In Rust, that is about how you call the
function, so the test shows the standard way to do it.

- [ ] Add these inside `mod tests`, after `dist_is_euclidean`:

  ```rust
      // A horizontal segment on the x-axis.
      const A: Point = Point { x: 0.0, y: 0.0 };
      const B: Point = Point { x: 4.0, y: 0.0 };

      #[test]
      fn perpendicular_distance_when_foot_is_inside() {
          let points = [
              Point::new(2.0, 3.0),
              Point::new(1.0, -2.0),
              Point::new(3.0, 0.5),
          ];

          let dists: Vec<f64> = points.iter().map(|&p| dist_to_segment(p, A, B)).collect();

          // one distance per point, each the perpendicular offset
          assert_eq!(dists.len(), 3);
          assert_close(dists[0], 3.0);
          assert_close(dists[1], 2.0);
          assert_close(dists[2], 0.5);
      }
  ```

1. `const A: Point = Point { ... }` is a compile-time constant. Its value cannot come from a
   function call such as `Point::new`, so it uses the struct literal.
2. `[a, b, c]` is an **array**. Its length is fixed and is part of its type (`[Point; 3]`). A
   `Vec` can grow.
3. `points.iter().map(...).collect()` is the Rust way to do `[f(p) for p in points]`.
   `.iter()` gives an **iterator** that lends each element. `.map(...)` changes each element.
   `.collect()` puts the results into a collection. Iterators are lazy, like Python
   generators: nothing runs until `collect` asks for the values. The compiler then makes one
   tight loop with no temporary lists.
4. `|&p| dist_to_segment(p, A, B)` is a **closure**, like `lambda p: dist_to_segment(p, A, B)`.
   `.iter()` gives references (`&Point`), and `|&p|` takes the value out of the
   reference, so `p` is a `Point`. That works because `Point` is `Copy`.
5. `let dists: Vec<f64>` tells `collect` which collection to build. `collect` can build many
   types, so it needs to know which one.

- [ ] Add the other three cases:

  ```rust
      #[test]
      fn clamps_to_nearest_endpoint_beyond_the_ends() {
          // past b, past a, then diagonally past b
          assert_close(dist_to_segment(Point::new(6.0, 0.0), A, B), 2.0);
          assert_close(dist_to_segment(Point::new(-3.0, 0.0), A, B), 3.0);
          assert_close(dist_to_segment(Point::new(7.0, 4.0), A, B), 5.0);
      }

      #[test]
      fn zero_on_the_segment() {
          // both endpoints and an interior point
          assert_close(dist_to_segment(A, A, B), 0.0);
          assert_close(dist_to_segment(B, A, B), 0.0);
          assert_close(dist_to_segment(Point::new(2.0, 0.0), A, B), 0.0);
      }

      #[test]
      fn degenerate_segment_uses_point_distance() {
          let c = Point::new(1.0, 1.0);

          // a == b, so this is plain point-to-point distance
          assert_close(dist_to_segment(Point::new(1.0, 1.0), c, c), 0.0);
          assert_close(dist_to_segment(Point::new(4.0, 5.0), c, c), 5.0);
          assert_close(dist_to_segment(Point::new(1.0, 4.0), c, c), 3.0);
      }
  ```

### 11. Watch them fail

- [ ] Run `cargo test`. Expected: `3 passed; 4 failed`. All four failures are
  `not yet implemented`.

### 12. Write `dist_to_segment`

**Try first.** Port the Python function for one point:

```python
AB = pB - pA
denom = AB @ AB
if denom == 0.0:
    return np.linalg.norm(coords - pA, axis=1)
t = np.clip((coords - pA) @ AB / denom, 0.0, 1.0)
proj = pA + t[:, None] * AB
return np.linalg.norm(coords - proj, axis=1)
```

You need `dot`, `dist`, `-`, `+`, `*`, and the float method `clamp(min, max)`.

**Suggested code.**

```rust
pub fn dist_to_segment(p: Point, a: Point, b: Point) -> f64 {
    let ab = b - a;
    let denom = ab.dot(ab);
    if denom == 0.0 {
        return p.dist(a);
    }
    let t = ((p - a).dot(ab) / denom).clamp(0.0, 1.0);
    let foot = a + ab * t;
    p.dist(foot)
}
```

`return` exists in Rust, and it is used for an early exit, as here. The normal result is the
last expression, `p.dist(foot)`, as in Slice 1, Step 9. `if` needs no brackets around the
condition, but it always needs `{ }`. Comparing with `== 0.0` is correct here because the
Python code does the same thing: a segment of exactly zero length.

- [ ] Run `cargo test`. Expected: `7 passed`.

### 13. An enum for the side

`_cross_signs` returns `+1.0`, `-1.0` or `0.0`, and `_split_mask_by_handedness` multiplies
those signs. In Rust, a value that is always one of a fixed set of cases is an **enum**.

- [ ] Add this after `dist_to_segment`:

  ```rust
  /// Below this absolute cross product, a point counts as on the axis.
  pub const COLLINEAR_ATOL: f64 = 1e-9;

  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub enum Side {
      Left,
      Right,
      On,
  }

  /// Which side of the directed line `a`->`b` the point `p` is on.
  pub fn side_of(p: Point, a: Point, b: Point, atol: f64) -> Side {
      todo!()
  }
  ```

1. `enum Side` has three **variants**. A `Side` value is always exactly one of them, and you
   write it as `Side::Left`. A Python `enum.Enum` is similar. Rust enums can also hold data in
   each variant: `Option` and `Result` are enums, with `Some(value)` and `Err(error)`.
2. With an enum, a sign of `0.5` or `NaN` cannot exist. The type also says what the value means:
   `Side::Left` is clearer than `1.0`. In Slice 5 you use `match` to say which pairs of sides
   are compatible, instead of the product trick.
3. `Eq` says that `==` is a full equivalence: every value equals itself. `Point` cannot be
   `Eq`, because `f64` is not. `NaN == NaN` is false, so `f64` only has `PartialEq`. `Side`
   has no floats, so it can be `Eq`.
4. Python has `atol: float = 1e-9`. **Rust has no default arguments.** Rust prefers that every
   call shows all of its inputs. The constant gives the usual value a name, and calls pass
   `COLLINEAR_ATOL` explicitly. For a function with many optional settings, Rust code uses a
   settings struct instead. You will see one in Slice 8.

### 14. Flip a side with `match`

Reversing the axis swaps left and right. The test in Step 15 needs this, and Slice 5 needs it
to check reflections.

- [ ] Add this after the enum:

  ```rust
  impl Side {
      pub fn flipped(self) -> Side {
          match self {
              Side::Left => Side::Right,
              Side::Right => Side::Left,
              Side::On => Side::On,
          }
      }
  }
  ```

`match` is like a Python `match` statement, but it is an **expression**: it gives a value,
which is the return value here. It must also be **exhaustive**. Delete the `Side::On` line and
run `cargo build`: the error is `E0004: non-exhaustive patterns: Side::On not covered`. Put the
line back. If you add a fourth variant later, the compiler finds every `match` that you must
update.

### 15. Port the `TestCrossSigns` cases

- [ ] Add these at the end of `mod tests`:

  ```rust
      // The axis (0,0) -> (1,0) points along +x.
      const X1: Point = Point { x: 1.0, y: 0.0 };

      fn side(x: f64, y: f64) -> Side {
          side_of(Point::new(x, y), A, X1, COLLINEAR_ATOL)
      }

      #[test]
      fn left_right_and_on_axis() {
          // above is left (counter-clockwise), below is right
          assert_eq!(side(0.0, 1.0), Side::Left);
          assert_eq!(side(0.0, -1.0), Side::Right);
          // ahead of the segment, on the line
          assert_eq!(side(2.0, 0.0), Side::On);
      }

      #[test]
      fn axis_endpoints_are_on() {
          assert_eq!(side(0.0, 0.0), Side::On);
          assert_eq!(side(1.0, 0.0), Side::On);
      }

      #[test]
      fn collinear_beyond_segment_is_on() {
          // the side is about the infinite line, not the segment
          assert_eq!(side(2.0, 0.0), Side::On);
          assert_eq!(side(-3.0, 0.0), Side::On);
      }
  ```

  `side` is a test helper that keeps each assert on one line.

- [ ] Add the last two:

  ```rust
      #[test]
      fn reversing_axis_flips_side() {
          for p in [
              Point::new(0.0, 1.0),
              Point::new(0.0, -1.0),
              Point::new(2.0, 0.0),
          ] {
              let forward = side_of(p, A, X1, COLLINEAR_ATOL);

              let reversed = side_of(p, X1, A, COLLINEAR_ATOL);

              // left and right swap; on stays on
              assert_eq!(reversed, forward.flipped());
          }
      }

      #[test]
      fn atol_snaps_near_axis_to_on() {
          // just off the line, inside a generous atol
          assert_eq!(side_of(Point::new(0.5, 0.05), A, X1, 0.1), Side::On);
          // clearly off the line keeps its side
          assert_eq!(side_of(Point::new(0.5, 0.5), A, X1, 0.1), Side::Left);
      }
  ```

`for p in [...]` loops over an array directly, like a Python `for` loop over a list.

### 16. Watch them fail

- [ ] Run `cargo test`. Expected: `7 passed; 5 failed`. All five failures are
  `not yet implemented`.

### 17. Write `side_of`

**Try first.** The Python code computes `cross = axis × (p - a)` with `axis = b - a`. It sets
values with `|cross| < atol` to 0, and then takes the sign. Return a `Side` instead of a sign.
Use an `if` / `else if` / `else` chain as an expression.

**Suggested code.**

```rust
pub fn side_of(p: Point, a: Point, b: Point, atol: f64) -> Side {
    let cross = (b - a).cross(p - a);
    if cross.abs() < atol {
        Side::On
    } else if cross > 0.0 {
        Side::Left
    } else {
        Side::Right
    }
}
```

An `if` is also an expression. Each branch gives a `Side`, and the whole `if` is the return
value. This is like Python's `x if cond else y`, but with full blocks. Every branch must give
the same type.

### 18. Watch them pass

- [ ] Run `cargo test`. Expected: `12 passed`.
- [ ] Run only the geometry tests:

  ```powershell
  cargo test geometry
  ```

  Expected: `10 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out`. The argument filters
  by test path (`geometry::tests::...`), like `pytest -k`.

### 19. Look at it

- [ ] Run:

  ```powershell
  cargo doc --no-deps --open
  ```

  Expected: a browser opens the documentation for the `gsearch` crate. Click `geometry` and then
  `Point`. Your `///` comments are there, together with the traits that `Point` implements,
  including `Sub`, `Add`, `Mul<f64>`, and the derived traits.

### 20. Format, lint, commit

- [ ] Run `cargo fmt`, then `cargo clippy --all-targets`. Expected: no warnings.
- [ ] Commit:

  ```powershell
  git add -A
  git commit -m "Slice 2: geometry primitives"
  ```

---

## What comes next

Slice 3 builds the graph type, so decide D5 first (see the end of Slice 1). Then ask Claude
to write Slice 3. Slices 5 to 7 use `Point`, `dist_to_segment`, `side_of` and `Side::flipped`
from this slice.
