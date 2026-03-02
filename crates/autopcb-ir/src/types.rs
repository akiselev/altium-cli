//! Geometric primitives for the PCB IR — all values in millimeters.

use altium_format_types::CoordPoint;

/// A 2D point in millimeters.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct PointMm {
    pub x: f64,
    pub y: f64,
}

impl PointMm {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn distance_to(&self, other: &PointMm) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }

    pub fn from_coord_point(cp: &CoordPoint) -> Self {
        Self {
            x: cp.x.to_mms(),
            y: cp.y.to_mms(),
        }
    }
}

/// Axis-aligned bounding box in millimeters.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct BoundingBoxMm {
    pub min: PointMm,
    pub max: PointMm,
}

impl BoundingBoxMm {
    pub fn new(min: PointMm, max: PointMm) -> Self {
        Self { min, max }
    }

    pub fn from_points(points: &[PointMm]) -> Option<Self> {
        if points.is_empty() {
            return None;
        }
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        for p in points {
            min_x = min_x.min(p.x);
            min_y = min_y.min(p.y);
            max_x = max_x.max(p.x);
            max_y = max_y.max(p.y);
        }
        Some(Self {
            min: PointMm::new(min_x, min_y),
            max: PointMm::new(max_x, max_y),
        })
    }

    pub fn width(&self) -> f64 {
        self.max.x - self.min.x
    }

    pub fn height(&self) -> f64 {
        self.max.y - self.min.y
    }

    pub fn center(&self) -> PointMm {
        PointMm::new(
            (self.min.x + self.max.x) / 2.0,
            (self.min.y + self.max.y) / 2.0,
        )
    }

    pub fn contains(&self, p: &PointMm) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }

    /// Returns the smallest bounding box enclosing both `self` and `other`.
    pub fn union(&self, other: &BoundingBoxMm) -> BoundingBoxMm {
        BoundingBoxMm {
            min: PointMm::new(self.min.x.min(other.min.x), self.min.y.min(other.min.y)),
            max: PointMm::new(self.max.x.max(other.max.x), self.max.y.max(other.max.y)),
        }
    }

    /// Expand the bounding box by `margin` in all directions.
    pub fn expand(&self, margin: f64) -> BoundingBoxMm {
        BoundingBoxMm {
            min: PointMm::new(self.min.x - margin, self.min.y - margin),
            max: PointMm::new(self.max.x + margin, self.max.y + margin),
        }
    }
}

/// Which side of the board a component is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum BoardSide {
    Top,
    Bottom,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_distance() {
        let a = PointMm::new(0.0, 0.0);
        let b = PointMm::new(3.0, 4.0);
        assert!((a.distance_to(&b) - 5.0).abs() < 1e-10);
    }

    #[test]
    fn bbox_from_points_empty() {
        assert!(BoundingBoxMm::from_points(&[]).is_none());
    }

    #[test]
    fn bbox_arithmetic() {
        let bb = BoundingBoxMm::new(PointMm::new(1.0, 2.0), PointMm::new(5.0, 8.0));
        assert!((bb.width() - 4.0).abs() < 1e-10);
        assert!((bb.height() - 6.0).abs() < 1e-10);
        let c = bb.center();
        assert!((c.x - 3.0).abs() < 1e-10);
        assert!((c.y - 5.0).abs() < 1e-10);
        assert!(bb.contains(&PointMm::new(3.0, 5.0)));
        assert!(!bb.contains(&PointMm::new(0.0, 0.0)));
    }

    #[test]
    fn bbox_union() {
        let a = BoundingBoxMm::new(PointMm::new(0.0, 0.0), PointMm::new(2.0, 2.0));
        let b = BoundingBoxMm::new(PointMm::new(1.0, 1.0), PointMm::new(4.0, 3.0));
        let u = a.union(&b);
        assert!((u.min.x - 0.0).abs() < 1e-10);
        assert!((u.min.y - 0.0).abs() < 1e-10);
        assert!((u.max.x - 4.0).abs() < 1e-10);
        assert!((u.max.y - 3.0).abs() < 1e-10);
    }
}
