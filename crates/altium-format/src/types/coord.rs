//! Coordinate types for Altium internal coordinate system.
//!
//! Altium uses a fixed-point coordinate system where 10,000 units = 1 mil.
//! This module provides type-safe wrappers around these internal coordinates.

use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Sub};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::error::Result;
use crate::traits::{FromBinary, ToBinary};

/// Conversion factor: internal units per mil.
pub const INTERNAL_UNITS: f64 = 10000.0;

/// Internal coordinate value, stored as a fixed-point integer.
///
/// 10,000 internal units = 1 mil = 0.001 inch = 0.0254 mm
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Coord(i32);

impl Coord {
    /// Length of 1 inch as a coordinate value.
    pub const ONE_INCH: Coord = Coord((1000.0 * INTERNAL_UNITS) as i32);

    /// Zero coordinate.
    pub const ZERO: Coord = Coord(0);

    /// Creates a coordinate from mils (thousandths of an inch).
    #[inline]
    pub fn from_mils(mils: f64) -> Self {
        Coord((mils * INTERNAL_UNITS) as i32)
    }

    /// Converts this coordinate to mils.
    #[inline]
    pub fn to_mils(self) -> f64 {
        self.0 as f64 / INTERNAL_UNITS
    }

    /// Creates a coordinate from millimeters.
    #[inline]
    pub fn from_mms(mms: f64) -> Self {
        Self::from_mils(mms / 0.0254)
    }

    /// Converts this coordinate to millimeters.
    #[inline]
    pub fn to_mms(self) -> f64 {
        self.to_mils() * 0.0254
    }

    /// Creates a coordinate from inches.
    #[inline]
    pub fn from_inches(inches: f64) -> Self {
        Self::from_mils(inches * 1000.0)
    }

    /// Converts this coordinate to inches.
    #[inline]
    pub fn to_inches(self) -> f64 {
        self.to_mils() * 0.001
    }

    /// Creates a coordinate from centimeters.
    #[inline]
    pub fn from_cms(cms: f64) -> Self {
        Self::from_mms(cms * 10.0)
    }

    /// Converts this coordinate to centimeters.
    #[inline]
    pub fn to_cms(self) -> f64 {
        self.to_mms() * 0.1
    }

    /// Creates a coordinate from meters.
    #[inline]
    pub fn from_meters(meters: f64) -> Self {
        Self::from_mms(meters * 1000.0)
    }

    /// Converts this coordinate to meters.
    #[inline]
    pub fn to_meters(self) -> f64 {
        self.to_mms() * 0.001
    }

    /// Creates a coordinate from DXP default units (10 mils).
    #[inline]
    pub fn from_dxp(value: f64) -> Self {
        Self::from_mils(value * 10.0)
    }

    /// Converts this coordinate to DXP default units.
    #[inline]
    pub fn to_dxp(self) -> f64 {
        self.to_mils() / 10.0
    }

    /// Creates a coordinate from a raw i32 value.
    #[inline]
    pub const fn from_raw(value: i32) -> Self {
        Coord(value)
    }

    /// Gets the raw i32 value.
    #[inline]
    pub const fn to_raw(self) -> i32 {
        self.0
    }

    /// Returns the absolute value of this coordinate.
    #[inline]
    pub fn abs(self) -> Self {
        Coord(self.0.abs())
    }

    /// Returns the minimum of two coordinates.
    #[inline]
    pub fn min(self, other: Self) -> Self {
        Coord(self.0.min(other.0))
    }

    /// Returns the maximum of two coordinates.
    #[inline]
    pub fn max(self, other: Self) -> Self {
        Coord(self.0.max(other.0))
    }
}

impl fmt::Debug for Coord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Coord({:.3}mil)", self.to_mils())
    }
}

impl fmt::Display for Coord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.3}mil", self.to_mils())
    }
}

impl From<i32> for Coord {
    #[inline]
    fn from(value: i32) -> Self {
        Coord(value)
    }
}

impl From<Coord> for i32 {
    #[inline]
    fn from(coord: Coord) -> Self {
        coord.0
    }
}

impl Add for Coord {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Coord(self.0 + rhs.0)
    }
}

impl Sub for Coord {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Coord(self.0 - rhs.0)
    }
}

impl Mul<i32> for Coord {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: i32) -> Self {
        Coord(self.0 * rhs)
    }
}

impl Div<i32> for Coord {
    type Output = Self;
    #[inline]
    fn div(self, rhs: i32) -> Self {
        Coord(self.0 / rhs)
    }
}

impl Neg for Coord {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Coord(-self.0)
    }
}

/// 2D point with X and Y coordinates.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct CoordPoint {
    pub x: Coord,
    pub y: Coord,
}

impl CoordPoint {
    /// Zero point at origin.
    pub const ZERO: CoordPoint = CoordPoint {
        x: Coord::ZERO,
        y: Coord::ZERO,
    };

    /// Creates a new point.
    #[inline]
    pub const fn new(x: Coord, y: Coord) -> Self {
        CoordPoint { x, y }
    }

    /// Creates a point from mil coordinates.
    #[inline]
    pub fn from_mils(x: f64, y: f64) -> Self {
        CoordPoint {
            x: Coord::from_mils(x),
            y: Coord::from_mils(y),
        }
    }

    /// Creates a point from millimeter coordinates.
    #[inline]
    pub fn from_mms(x: f64, y: f64) -> Self {
        CoordPoint {
            x: Coord::from_mms(x),
            y: Coord::from_mms(y),
        }
    }

    /// Creates a point from raw i32 values.
    #[inline]
    pub const fn from_raw(x: i32, y: i32) -> Self {
        CoordPoint {
            x: Coord::from_raw(x),
            y: Coord::from_raw(y),
        }
    }

    /// Translates this point by the given offset.
    #[inline]
    pub fn translate(self, dx: Coord, dy: Coord) -> Self {
        CoordPoint {
            x: self.x + dx,
            y: self.y + dy,
        }
    }

    /// Rotates this point around an anchor point by the given angle in degrees.
    pub fn rotate(self, anchor: CoordPoint, angle_degrees: f64) -> Self {
        let angle_radians = -angle_degrees * std::f64::consts::PI / 180.0;
        let cos_angle = angle_radians.cos();
        let sin_angle = angle_radians.sin();

        let local_x = (self.x.0 - anchor.x.0) as f64;
        let local_y = (self.y.0 - anchor.y.0) as f64;

        let rotated_x = local_x * cos_angle + local_y * sin_angle;
        let rotated_y = local_y * cos_angle - local_x * sin_angle;

        CoordPoint {
            x: Coord(anchor.x.0 + rotated_x as i32),
            y: Coord(anchor.y.0 + rotated_y as i32),
        }
    }
}

impl fmt::Debug for CoordPoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CoordPoint({:?}, {:?})", self.x, self.y)
    }
}

impl fmt::Display for CoordPoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "X:{} Y:{}", self.x, self.y)
    }
}

impl Add for CoordPoint {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        CoordPoint {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Sub for CoordPoint {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        CoordPoint {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl FromBinary for Coord {
    fn read_from<R: std::io::Read>(reader: &mut R) -> Result<Self> {
        Ok(Coord::from_raw(reader.read_i32::<LittleEndian>()?))
    }
}

impl ToBinary for Coord {
    fn write_to<W: std::io::Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_i32::<LittleEndian>(self.to_raw())?;
        Ok(())
    }

    fn binary_size(&self) -> usize {
        4
    }
}

impl FromBinary for CoordPoint {
    fn read_from<R: std::io::Read>(reader: &mut R) -> Result<Self> {
        let x = reader.read_i32::<LittleEndian>()?;
        let y = reader.read_i32::<LittleEndian>()?;
        Ok(CoordPoint::from_raw(x, y))
    }
}

impl ToBinary for CoordPoint {
    fn write_to<W: std::io::Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_i32::<LittleEndian>(self.x.to_raw())?;
        writer.write_i32::<LittleEndian>(self.y.to_raw())?;
        Ok(())
    }

    fn binary_size(&self) -> usize {
        8
    }
}

/// 3D point with X, Y, and Z coordinates.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct CoordPoint3D {
    pub x: Coord,
    pub y: Coord,
    pub z: Coord,
}

impl CoordPoint3D {
    /// Zero point at origin.
    pub const ZERO: CoordPoint3D = CoordPoint3D {
        x: Coord::ZERO,
        y: Coord::ZERO,
        z: Coord::ZERO,
    };

    /// Creates a new 3D point.
    #[inline]
    pub const fn new(x: Coord, y: Coord, z: Coord) -> Self {
        CoordPoint3D { x, y, z }
    }

    /// Creates a 3D point from raw i32 values.
    #[inline]
    pub const fn from_raw(x: i32, y: i32, z: i32) -> Self {
        CoordPoint3D {
            x: Coord::from_raw(x),
            y: Coord::from_raw(y),
            z: Coord::from_raw(z),
        }
    }

    /// Converts to a 2D point, discarding the Z coordinate.
    #[inline]
    pub const fn to_2d(self) -> CoordPoint {
        CoordPoint {
            x: self.x,
            y: self.y,
        }
    }
}

impl fmt::Debug for CoordPoint3D {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CoordPoint3D({:?}, {:?}, {:?})", self.x, self.y, self.z)
    }
}

impl fmt::Display for CoordPoint3D {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "X:{} Y:{} Z:{}", self.x, self.y, self.z)
    }
}

/// Rectangle defined by two corner points.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct CoordRect {
    /// Bottom-left corner (minimum x, minimum y).
    pub location1: CoordPoint,
    /// Top-right corner (maximum x, maximum y).
    pub location2: CoordPoint,
}

impl CoordRect {
    /// Empty rectangle at origin.
    pub const EMPTY: CoordRect = CoordRect {
        location1: CoordPoint::ZERO,
        location2: CoordPoint::ZERO,
    };

    /// Creates a rectangle from raw i32 values (x, y, width, height).
    #[inline]
    pub fn from_raw(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self::new(
            CoordPoint::from_raw(x, y),
            CoordPoint::from_raw(x + width, y + height),
        )
    }

    /// Creates a rectangle from two corner points without normalizing.
    #[inline]
    pub fn from_corners(p1: CoordPoint, p2: CoordPoint) -> Self {
        Self::new(p1, p2)
    }

    /// Creates a rectangle from two corner points.
    /// The points are normalized so location1 has the minimum values.
    pub fn new(p1: CoordPoint, p2: CoordPoint) -> Self {
        CoordRect {
            location1: CoordPoint {
                x: p1.x.min(p2.x),
                y: p1.y.min(p2.y),
            },
            location2: CoordPoint {
                x: p1.x.max(p2.x),
                y: p1.y.max(p2.y),
            },
        }
    }

    /// Creates a rectangle from position and size.
    pub fn from_xywh(x: Coord, y: Coord, width: Coord, height: Coord) -> Self {
        Self::new(
            CoordPoint::new(x, y),
            CoordPoint::new(x + width, y + height),
        )
    }

    /// Creates an empty rectangle at origin.
    #[inline]
    pub const fn empty() -> Self {
        Self::EMPTY
    }

    /// Creates a rectangle from four coordinate values.
    pub fn from_points(x1: Coord, y1: Coord, x2: Coord, y2: Coord) -> Self {
        Self::new(CoordPoint::new(x1, y1), CoordPoint::new(x2, y2))
    }

    /// Returns the width of the rectangle.
    #[inline]
    pub fn width(self) -> Coord {
        self.location2.x - self.location1.x
    }

    /// Returns the height of the rectangle.
    #[inline]
    pub fn height(self) -> Coord {
        self.location2.y - self.location1.y
    }

    /// Returns true if the rectangle is empty (zero width and height).
    #[inline]
    pub fn is_empty(self) -> bool {
        self.width().0 == 0 && self.height().0 == 0
    }

    /// Returns the center point of the rectangle.
    #[inline]
    pub fn center(self) -> CoordPoint {
        CoordPoint {
            x: Coord((self.location1.x.0 + self.location2.x.0) / 2),
            y: Coord((self.location1.y.0 + self.location2.y.0) / 2),
        }
    }

    /// Returns true if the rectangle contains the given point.
    #[inline]
    pub fn contains(self, point: CoordPoint) -> bool {
        self.location1.x <= point.x
            && point.x <= self.location2.x
            && self.location1.y <= point.y
            && point.y <= self.location2.y
    }

    /// Returns true if this rectangle intersects with another.
    #[inline]
    pub fn intersects(self, other: CoordRect) -> bool {
        self.location1.x <= other.location2.x
            && self.location2.x >= other.location1.x
            && self.location1.y <= other.location2.y
            && self.location2.y >= other.location1.y
    }

    /// Returns the four corner points of the rectangle.
    pub fn corners(self) -> [CoordPoint; 4] {
        [
            self.location1,
            CoordPoint::new(self.location2.x, self.location1.y),
            self.location2,
            CoordPoint::new(self.location1.x, self.location2.y),
        ]
    }

    /// Returns the union of two rectangles.
    pub fn union(self, other: CoordRect) -> CoordRect {
        if self.is_empty() {
            return other;
        }
        if other.is_empty() {
            return self;
        }
        CoordRect {
            location1: CoordPoint {
                x: self.location1.x.min(other.location1.x),
                y: self.location1.y.min(other.location1.y),
            },
            location2: CoordPoint {
                x: self.location2.x.max(other.location2.x),
                y: self.location2.y.max(other.location2.y),
            },
        }
    }

    /// Creates a bounding box from an iterator of rectangles.
    pub fn union_all(rects: impl IntoIterator<Item = CoordRect>) -> CoordRect {
        rects
            .into_iter()
            .fold(CoordRect::EMPTY, |acc, r| acc.union(r))
    }
}

impl fmt::Debug for CoordRect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CoordRect({:?} - {:?})", self.location1, self.location2)
    }
}

impl fmt::Display for CoordRect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({} {})", self.location1, self.location2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coord_conversions() {
        let c = Coord::from_mils(100.0);
        assert_eq!(c.to_raw(), 1_000_000);
        assert!((c.to_mils() - 100.0).abs() < 0.001);

        let c = Coord::from_mms(2.54);
        assert!((c.to_mils() - 100.0).abs() < 0.1);
    }

    #[test]
    fn test_coord_point() {
        let p1 = CoordPoint::from_mils(100.0, 200.0);
        let p2 = CoordPoint::from_mils(50.0, 50.0);
        let sum = p1 + p2;
        assert!((sum.x.to_mils() - 150.0).abs() < 0.001);
        assert!((sum.y.to_mils() - 250.0).abs() < 0.001);
    }

    #[test]
    fn test_coord_rect() {
        let r = CoordRect::from_xywh(
            Coord::from_mils(10.0),
            Coord::from_mils(20.0),
            Coord::from_mils(100.0),
            Coord::from_mils(50.0),
        );
        assert!((r.width().to_mils() - 100.0).abs() < 0.001);
        assert!((r.height().to_mils() - 50.0).abs() < 0.001);
        assert!(r.contains(CoordPoint::from_mils(50.0, 40.0)));
        assert!(!r.contains(CoordPoint::from_mils(0.0, 0.0)));
    }
}
