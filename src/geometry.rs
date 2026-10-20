use std::ops::{Add, Mul, Sub};

use serde::Serialize;

/// Below this value, the cross product of a point with a line will indicate that
/// the point is "on the line"
pub const COLLINEAR_TOL: f64 = 1e-9;

#[derive(Debug, Copy, Clone, PartialEq, Serialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    // I can probably take new out
    pub fn new(x: f64, y: f64) -> Point {
        Point { x, y }
    }

    pub fn dot(self, other: Point) -> f64 {
        self.x * other.x + self.y * other.y
    }

    pub fn cross(self, other: Point) -> f64 {
        self.x * other.y - self.y * other.x
    }

    pub fn norm(self) -> f64 {
        self.dot(self).sqrt()
    }

    pub fn dist(self, other: Point) -> f64 {
        (self - other).norm()
    }
}

impl Sub for Point {
    type Output = Point;
    fn sub(self, other: Point) -> Point {
        Point::new(self.x - other.x, self.y - other.y)
    }
}

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

/// A rigid transformation matrix.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Transform {
    pub r: [[f64; 2]; 2],
    pub t: Point,
}

impl Transform {
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

/// Defines what side of a line a point falls on
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
    On,
}

impl Side {
    pub fn flipped(self) -> Side {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
            Side::On => Side::On,
        }
    }

    /// Determines if two sides are of the same handedness.
    pub fn same_hand(self, other: Side) -> bool {
        match (self, other) {
            // If either is on the line, then the it is the same handedness.
            (Side::On, _) | (_, Side::On) => true,
            // Otherwise, they must be the same side to have the same handedness.
            _ => self == other,
        }
    }
}

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

/// Returns distance from p to the a->b segment
pub fn dist_to_segment(p: Point, a: Point, b: Point) -> f64 {
    let ab = b - a;
    let denom = ab.dot(ab);

    if denom == 0.0 {
        return p.dist(a);
    }
    let t = ((p - a).dot(ab) / denom).clamp(0.0, 1.0);
    let closest_point = a + ab * t;
    p.dist(closest_point)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-9,
            "expected {expected}, got {actual}"
        );
    }

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

    #[test]
    fn apply_all_keeps_the_order() {
        let t = Transform {
            r: [[1.0, 0.0], [0.0, 1.0]],
            t: Point::new(1.0, 1.0),
        };
        let points = [
            Point::new(0.0, 0.0),
            Point::new(3.0, 0.0),
            Point::new(0.0, 4.0),
        ];

        let moved = t.apply_all(&points);

        assert_eq!(moved.len(), 3);
        assert_point_close(moved[0], Point::new(1.0, 1.0));
        assert_point_close(moved[2], Point::new(1.0, 5.0));
    }

    #[test]
    fn same_hand_is_true_for_equal_sides_and_for_on() {
        assert!(Side::Left.same_hand(Side::Left));
        assert!(Side::Right.same_hand(Side::Right));
        assert!(Side::On.same_hand(Side::Left));
        assert!(Side::Right.same_hand(Side::On));
        assert!(Side::On.same_hand(Side::On));
    }

    #[test]
    fn same_hand_is_false_for_opposite_sides() {
        assert!(!Side::Left.same_hand(Side::Right));
        assert!(!Side::Right.same_hand(Side::Left));
    }

    #[test]
    fn dist_is_euclidean() {
        let p1 = Point::new(1.0, 1.0);
        let p2: Point = Point::new(4.0, 5.0);

        let d = p1.dist(p2);
        assert_close(d, 5.0)
    }

    #[test]
    fn add_is_correct() {
        let p1 = Point::new(-1.0, -1.0);
        let p2: Point = Point::new(4.0, 5.0);

        let sum = p1 + p2;

        assert_close(sum.x, 3.0);
        assert_close(sum.y, 4.0);
    }

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

    // The axis (0,0) -> (1,0) points along +x.
    const X1: Point = Point { x: 1.0, y: 0.0 };

    fn side(x: f64, y: f64) -> Side {
        side_of(Point::new(x, y), A, X1, COLLINEAR_TOL)
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

    #[test]
    fn reversing_axis_flips_side() {
        for p in [
            Point::new(0.0, 1.0),
            Point::new(0.0, -1.0),
            Point::new(2.0, 0.0),
        ] {
            let forward = side_of(p, A, X1, COLLINEAR_TOL);

            let reversed = side_of(p, X1, A, COLLINEAR_TOL);

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
}
