//! Solves for the rigid transform that best aligns two sets of points using Procrustes analysis.

use crate::geometry::{Point, Transform};

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

/// Calculates the centroid of a set of points.
fn centroid(points: &[Point]) -> Point {
    let sum = points.iter().fold(Point::new(0.0, 0.0), |acc, &p| acc + p);
    sum * (1.0 / points.len() as f64)
}

/// Aligns to sets of points using procrustes analysis.
///
/// Uses the SVD based algorithm from Arun et al. 1987 (https://ieeexplore.ieee.org/abstract/document/4767965)
pub fn fit(q: &[Point], g: &[Point]) -> Transform {
    let mu_q = centroid(q);
    let mu_g = centroid(g);

    let mut m = [[0.0; 2]; 2];
    for (&a, &b) in q.iter().zip(g) {
        let (a, b) = (a - mu_q, b - mu_g);
        m[0][0] += a.x * b.x;
        m[0][1] += a.x * b.y;
        m[1][0] += a.y * b.x;
        m[1][1] += a.y * b.y;
    }

    let det = m[0][0] * m[1][1] - m[0][1] * m[1][0];

    let rotation = det >= 0.0;

    let (x, y) = if rotation {
        (m[0][0] + m[1][1], m[1][0] - m[0][1])
    } else {
        (m[0][0] - m[1][1], m[0][1] + m[1][0])
    };

    let norm = x.hypot(y);
    let r = if norm == 0.0 {
        Transform::IDENTITY.r
    } else {
        let (c, s) = (x / norm, y / norm);
        if rotation {
            [[c, -s], [s, c]]
        } else {
            [[c, s], [s, -c]]
        }
    };

    // `mu_q` moves to `mu_g`
    let turn = Transform {
        r,
        t: Point::new(0.0, 0.0),
    };
    Transform {
        r,
        t: mu_g - turn.apply(mu_q),
    }
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
    fn fit_moves_q_onto_g() {
        let g = known().apply_all(&TRIANGLE);

        let moved = fit(&TRIANGLE, &g).apply_all(&TRIANGLE);

        // every moved point lands on its pair
        for (m, g) in moved.iter().zip(&g) {
            assert_point_close(*m, *g);
        }
    }

    #[test]
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
    fn fit_of_one_point_is_a_shift() {
        let t = fit(&[Point::new(1.0, 2.0)], &[Point::new(11.0, 22.0)]);

        // one point has no direction to turn, so r is the identity
        assert_eq!(t.r, [[1.0, 0.0], [0.0, 1.0]]);
        assert_point_close(t.t, Point::new(10.0, 20.0));
    }

    #[test]
    fn fit_is_the_identity_when_q_equals_g() {
        let t = fit(&TRIANGLE, &TRIANGLE);

        // nothing to turn and nothing to shift
        assert_eq!(t.r, [[1.0, 0.0], [0.0, 1.0]]);
        assert_point_close(t.t, Point::new(0.0, 0.0));
    }
}
