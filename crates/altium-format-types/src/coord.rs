use std::fmt;
use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

/// Fixed-point coordinate: 10,000 internal units = 1 mil (0.001 inch).
///
/// 1 mil = 10,000 units
/// 1 mm  = ~393,701 units
/// 1 inch = 10,000,000 units
/// Range: approximately +/- 214,748 mils = +/- 5,454 mm
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Coord(i32);

impl Coord {
    pub const ZERO: Self = Self(0);
    pub const ONE_MIL: Self = Self(10_000);
    pub const ONE_INCH: Self = Self(10_000_000);
    /// Nearest integer approximation: 1 mm = 39370.0787... mils * 10000 = 393700.787...
    /// Altium uses 393701.
    pub const ONE_MM: Self = Self(393_701);

    /// Units per mil (the fundamental resolution).
    pub const UNITS_PER_MIL: i32 = 10_000;
    /// DXP base unit for schematic parameter encoding.
    /// Each "DXP unit" (the integer part of LOCATION.X etc.) represents 10 mils.
    /// Source: Rt_Schematic.Consts.cBaseUnit = 100000.
    pub const DXP_BASE_UNIT: i32 = 100_000;

    pub fn from_mils(mils: i32) -> Self {
        Self(mils.checked_mul(10_000).expect("Coord::from_mils overflow"))
    }

    pub fn from_mms(mm: f64) -> Self {
        Self((mm * 393_700.787_401_574_8) as i32)
    }

    pub fn to_mils(self) -> f64 {
        self.0 as f64 / 10_000.0
    }

    pub fn to_mms(self) -> f64 {
        self.0 as f64 / 393_700.787_401_574_8
    }

    pub fn from_internal(raw: i32) -> Self {
        Self(raw)
    }

    pub fn to_internal(self) -> i32 {
        self.0
    }

    /// Alias for `from_internal`.
    pub fn new(raw: i32) -> Self {
        Self(raw)
    }

    /// Alias for `to_internal`.
    pub fn raw(self) -> i32 {
        self.0
    }

    pub fn abs(self) -> Self {
        Self(self.0.abs())
    }

    /// Construct from DXP integer + fractional parts.
    /// Each DXP integer unit = DXP_BASE_UNIT internal units (100,000 = 10 mils).
    /// The `frac` component stores the sub-unit remainder in internal units.
    pub fn from_dxp_frac(integer: i32, frac: i32) -> Self {
        Self(integer * Self::DXP_BASE_UNIT + frac)
    }

    /// Split into DXP integer + fractional parts using Euclidean division.
    /// Returns (integer_part, fractional_remainder) where 0 <= frac < DXP_BASE_UNIT.
    pub fn to_dxp_frac(self) -> (i32, i32) {
        (self.0.div_euclid(Self::DXP_BASE_UNIT), self.0.rem_euclid(Self::DXP_BASE_UNIT))
    }

    pub fn min(self, other: Self) -> Self {
        Self(self.0.min(other.0))
    }

    pub fn max(self, other: Self) -> Self {
        Self(self.0.max(other.0))
    }
}

impl Add for Coord {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl Sub for Coord {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0)
    }
}

impl Neg for Coord {
    type Output = Self;
    fn neg(self) -> Self {
        Self(-self.0)
    }
}

impl Mul<i32> for Coord {
    type Output = Self;
    fn mul(self, rhs: i32) -> Self {
        Self(self.0 * rhs)
    }
}

impl Div<i32> for Coord {
    type Output = Self;
    fn div(self, rhs: i32) -> Self {
        Self(self.0 / rhs)
    }
}

impl AddAssign for Coord {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl SubAssign for Coord {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

impl fmt::Display for Coord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.4} mils", self.to_mils())
    }
}

/// 2D point with X and Y coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct CoordPoint {
    pub x: Coord,
    pub y: Coord,
}

impl CoordPoint {
    pub fn new(x: Coord, y: Coord) -> Self {
        Self { x, y }
    }

    pub fn zero() -> Self {
        Self { x: Coord::ZERO, y: Coord::ZERO }
    }
}

impl fmt::Display for CoordPoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

/// Axis-aligned bounding box defined by min and max corners.
/// Invariant: min.x <= max.x && min.y <= max.y (enforced at construction).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BoundingBox {
    min: CoordPoint,
    max: CoordPoint,
}

impl BoundingBox {
    /// Construct a BoundingBox, panicking if min > max in either axis.
    pub fn new(min: CoordPoint, max: CoordPoint) -> Self {
        assert!(min.x <= max.x, "BoundingBox: min.x > max.x");
        assert!(min.y <= max.y, "BoundingBox: min.y > max.y");
        Self { min, max }
    }

    pub fn min(&self) -> CoordPoint {
        self.min
    }

    pub fn max(&self) -> CoordPoint {
        self.max
    }

    pub fn width(&self) -> Coord {
        self.max.x - self.min.x
    }

    pub fn height(&self) -> Coord {
        self.max.y - self.min.y
    }

    pub fn contains_point(&self, p: CoordPoint) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }

    pub fn union(&self, other: &Self) -> Self {
        let min = CoordPoint {
            x: if self.min.x <= other.min.x { self.min.x } else { other.min.x },
            y: if self.min.y <= other.min.y { self.min.y } else { other.min.y },
        };
        let max = CoordPoint {
            x: if self.max.x >= other.max.x { self.max.x } else { other.max.x },
            y: if self.max.y >= other.max.y { self.max.y } else { other.max.y },
        };
        Self { min, max }
    }

    /// Construct from two arbitrary points, auto-normalizing.
    pub fn from_points(a: CoordPoint, b: CoordPoint) -> Self {
        Self {
            min: CoordPoint {
                x: a.x.min(b.x),
                y: a.y.min(b.y),
            },
            max: CoordPoint {
                x: a.x.max(b.x),
                y: a.y.max(b.y),
            },
        }
    }

    /// Single-point bounding box.
    pub fn from_point(p: CoordPoint) -> Self {
        Self { min: p, max: p }
    }

    /// Bounding box from an iterator of points. Returns None if empty.
    pub fn from_iter(mut iter: impl Iterator<Item = CoordPoint>) -> Option<Self> {
        let first = iter.next()?;
        let mut bbox = Self::from_point(first);
        for p in iter {
            bbox = bbox.union_point(p);
        }
        Some(bbox)
    }

    /// Center point.
    pub fn center(&self) -> CoordPoint {
        CoordPoint {
            x: Coord::from_internal((self.min.x.to_internal() + self.max.x.to_internal()) / 2),
            y: Coord::from_internal((self.min.y.to_internal() + self.max.y.to_internal()) / 2),
        }
    }

    /// Expand bounding box by a margin on all sides.
    pub fn expand(&self, margin: Coord) -> Self {
        Self {
            min: CoordPoint {
                x: self.min.x - margin,
                y: self.min.y - margin,
            },
            max: CoordPoint {
                x: self.max.x + margin,
                y: self.max.y + margin,
            },
        }
    }

    /// Extend bounding box to include a point.
    pub fn union_point(&self, p: CoordPoint) -> Self {
        Self {
            min: CoordPoint {
                x: self.min.x.min(p.x),
                y: self.min.y.min(p.y),
            },
            max: CoordPoint {
                x: self.max.x.max(p.x),
                y: self.max.y.max(p.y),
            },
        }
    }
}
