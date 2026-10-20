//! The rigid transform from a query to its mapped nodes (`_align_graph` in Python).

use serde::Serialize;

use crate::geometry::Point;

/// True while `fit` is a placeholder. Set it to `false` when you write `fit`.
pub const IS_PLACEHOLDER: bool = true;

/// A rotation or reflection `r` and a translation `t`.
///
/// Like `Q @ R + t` in Python, a point is a row vector: `p` moves to `p * r + t`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
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

    pub fn apply(&self, p: Point) -> Point {
        Point::new(
            p.x * self.r[0][0] + p.y * self.r[1][0] + self.t.x,
            p.x * self.r[0][1] + p.y * self.r[1][1] + self.t.y,
        )
    }

    pub fn apply_all(&self, points: &[Point]) -> Vec<Point> {
        points.iter().map(|&p| self.apply(p)).collect()
    }
}

/// The length of the vector of all the offsets between `moved[i]` and `target[i]`.
///
/// This is `np.linalg.norm(moved - target)`: it is not squared, and it is not a mean.
pub fn node_score(moved: &[Point], target: &[Point]) -> f64 {
    moved
        .iter()
        .zip(target)
        .map(|(&a, &b)| (a - b).dot(a - b))
        .sum::<f64>()
        .sqrt()
}

/// PLACEHOLDER. The transform that moves `q[i]` closest to `g[i]`, over all `i`.
pub fn fit(_q: &[Point], _g: &[Point]) -> Transform {
    Transform::IDENTITY
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures::assert_close;

    fn assert_point_close(actual: Point, expected: Point) {
        assert_close(actual.x, expected.x);
        assert_close(actual.y, expected.y);
    }

    /// A 3-4-5 triangle: all three distances differ, so only one rigid transform fits it.
    const TRIANGLE: [Point; 3] = [
        Point { x: 0.0, y: 0.0 },
        Point { x: 3.0, y: 0.0 },
        Point { x: 0.0, y: 4.0 },
    ];

    /// The transform that the Python tests use: a 30 degree turn, then a shift.
    fn known() -> Transform {
        let (s, c) = 30.0_f64.to_radians().sin_cos();
        Transform {
            r: [[c, -s], [s, c]],
            t: Point::new(100.0, 250.0),
        }
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
}
