//! V2 Coordinate system matching Altium's internal representation.
//!
//! Altium uses 100,000 internal units per mil (confirmed from decompiled C#
//! `SchDataSerializerBinary.Export_Coord` which divides by 100_000, and
//! `Import_Coord` which multiplies by 100_000).
//!
//! The v1 code incorrectly uses 10,000 units/mil.

use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Sub};

/// Conversion factor: 100,000 internal units per mil.
///
/// Confirmed from C# `SchDataSerializerBinary`:
/// - `Export_Coord`: `int num = argN / 100000; WriteShort((short)num, argName);`
/// - `Import_Coord`: `argN = value * 100000;`
pub const INTERNAL_UNITS: f64 = 100_000.0;

/// V2 coordinate value using the correct 100K units/mil scale.
///
/// In binary mode, coordinates are split into a whole-mil `i16` part
/// and a fractional `i32` remainder (stored in the PinFrac stream).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct V2Coord(pub(crate) i32);

impl V2Coord {
    /// Zero coordinate.
    pub const ZERO: V2Coord = V2Coord(0);

    /// Length of 1 inch (1000 mils).
    pub const ONE_INCH: V2Coord = V2Coord((1000.0 * INTERNAL_UNITS) as i32);

    /// Creates a coordinate from raw internal units.
    #[inline]
    pub const fn from_raw(value: i32) -> Self {
        V2Coord(value)
    }

    /// Returns the raw internal unit value.
    #[inline]
    pub const fn to_raw(self) -> i32 {
        self.0
    }

    /// Creates a coordinate from mils (thousandths of an inch).
    #[inline]
    pub fn from_mils(mils: f64) -> Self {
        V2Coord((mils * INTERNAL_UNITS) as i32)
    }

    /// Converts to mils.
    #[inline]
    pub fn to_mils(self) -> f64 {
        self.0 as f64 / INTERNAL_UNITS
    }

    /// Creates a coordinate from millimeters.
    #[inline]
    pub fn from_mms(mms: f64) -> Self {
        Self::from_mils(mms / 0.0254)
    }

    /// Converts to millimeters.
    #[inline]
    pub fn to_mms(self) -> f64 {
        self.to_mils() * 0.0254
    }

    /// Creates a coordinate from inches.
    #[inline]
    pub fn from_inches(inches: f64) -> Self {
        Self::from_mils(inches * 1000.0)
    }

    /// Converts to inches.
    #[inline]
    pub fn to_inches(self) -> f64 {
        self.to_mils() * 0.001
    }

    /// Splits into binary parts for the Altium binary format.
    ///
    /// Binary mode stores coordinates as a whole-mil `i16` in the Data stream,
    /// with the fractional remainder as `i32` in the PinFrac stream.
    ///
    /// From C# `SchDataSerializerBinary`:
    /// - `Export_Coord`: `int num = argN / 100000; WriteShort((short)num, ...)`
    /// - The fractional part is `argN - 100000 * whole`.
    #[inline]
    pub fn to_binary_parts(self) -> (i16, i32) {
        let whole = self.0 / 100_000;
        let frac = self.0 - 100_000 * whole;
        (whole as i16, frac)
    }

    /// Reconstructs from binary parts (whole mils + fractional remainder).
    #[inline]
    pub fn from_binary_parts(whole: i16, frac: i32) -> Self {
        V2Coord(whole as i32 * 100_000 + frac)
    }

    /// Absolute value.
    #[inline]
    pub fn abs(self) -> Self {
        V2Coord(self.0.abs())
    }

    /// Minimum of two coordinates.
    #[inline]
    pub fn min(self, other: Self) -> Self {
        V2Coord(self.0.min(other.0))
    }

    /// Maximum of two coordinates.
    #[inline]
    pub fn max(self, other: Self) -> Self {
        V2Coord(self.0.max(other.0))
    }
}

impl fmt::Debug for V2Coord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "V2Coord({} = {:.3}mil)", self.0, self.to_mils())
    }
}

impl fmt::Display for V2Coord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.3}mil", self.to_mils())
    }
}

impl Add for V2Coord {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        V2Coord(self.0 + rhs.0)
    }
}

impl Sub for V2Coord {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        V2Coord(self.0 - rhs.0)
    }
}

impl Neg for V2Coord {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        V2Coord(-self.0)
    }
}

impl Mul<i32> for V2Coord {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: i32) -> Self {
        V2Coord(self.0 * rhs)
    }
}

impl Div<i32> for V2Coord {
    type Output = Self;
    #[inline]
    fn div(self, rhs: i32) -> Self {
        V2Coord(self.0 / rhs)
    }
}

/// A 2D point using V2 coordinates.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
pub struct V2Point {
    pub x: V2Coord,
    pub y: V2Coord,
}

impl V2Point {
    pub const ORIGIN: V2Point = V2Point {
        x: V2Coord::ZERO,
        y: V2Coord::ZERO,
    };

    pub fn new(x: V2Coord, y: V2Coord) -> Self {
        V2Point { x, y }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coord_from_mils() {
        let c = V2Coord::from_mils(1.0);
        assert_eq!(c.to_raw(), 100_000);
    }

    #[test]
    fn coord_round_trip_mils() {
        let c = V2Coord::from_mils(42.5);
        assert!((c.to_mils() - 42.5).abs() < 1e-6);
    }

    #[test]
    fn coord_binary_parts_positive() {
        let c = V2Coord::from_raw(350_123); // 3.50123 mils
        let (whole, frac) = c.to_binary_parts();
        assert_eq!(whole, 3);
        assert_eq!(frac, 50_123);
        assert_eq!(V2Coord::from_binary_parts(whole, frac), c);
    }

    #[test]
    fn coord_binary_parts_negative() {
        let c = V2Coord::from_raw(-250_000); // -2.5 mils
        let (whole, frac) = c.to_binary_parts();
        assert_eq!(whole, -2);
        assert_eq!(frac, -50_000);
        assert_eq!(V2Coord::from_binary_parts(whole, frac), c);
    }

    #[test]
    fn coord_binary_parts_exact_mil() {
        let c = V2Coord::from_mils(100.0);
        let (whole, frac) = c.to_binary_parts();
        assert_eq!(whole, 100);
        assert_eq!(frac, 0);
    }

    #[test]
    fn one_inch() {
        assert_eq!(V2Coord::ONE_INCH.to_raw(), 100_000_000);
        assert!((V2Coord::ONE_INCH.to_mils() - 1000.0).abs() < 1e-6);
    }
}
