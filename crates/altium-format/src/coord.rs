//! V2 Coordinate system for Altium schematic and PCB formats.
//!
//! Altium uses two different internal coordinate scales:
//! - **Schematic**: 100,000 internal units per mil (confirmed from decompiled C#
//!   `SchDataSerializerBinary.Export_Coord` which divides by 100,000).
//! - **PCB**: 10,000 internal units per mil.
//!
//! This module provides type-safe wrappers via the [`AltiumCoord`] trait,
//! with [`SchCoord`] and [`PcbCoord`] as the concrete implementations.

use std::fmt;
use std::marker::PhantomData;
use std::ops::{Add, Div, Mul, Neg, Sub};

use crate::error::{AltiumError, Result};

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// AltiumCoord trait
// ---------------------------------------------------------------------------

/// Shared interface for Altium coordinate types.
///
/// Each coordinate type has a fixed number of internal units per mil, allowing
/// type-safe conversions between physical units and the raw integer representation.
pub trait AltiumCoord: Copy + Sized + PartialEq + Eq + PartialOrd + Ord + std::fmt::Debug {
    /// Number of raw internal units per mil (thousandth of an inch).
    const UNITS_PER_MIL: i32;

    /// Creates a coordinate from a raw internal-unit value.
    fn from_raw(raw: i32) -> Self;

    /// Returns the raw internal-unit value.
    fn to_raw(self) -> i32;

    /// Creates a coordinate from mils (thousandths of an inch).
    fn from_mils(mils: f64) -> Self {
        Self::from_raw((mils * Self::UNITS_PER_MIL as f64) as i32)
    }

    /// Converts to mils.
    fn to_mils(self) -> f64 {
        self.to_raw() as f64 / Self::UNITS_PER_MIL as f64
    }

    /// Creates a coordinate from millimeters.
    fn from_mm(mm: f64) -> Self {
        Self::from_mils(mm / 0.0254)
    }

    /// Converts to millimeters.
    fn to_mm(self) -> f64 {
        self.to_mils() * 0.0254
    }
}

// ---------------------------------------------------------------------------
// impl_coord_ops! macro
// ---------------------------------------------------------------------------

/// Implements arithmetic operators (Add, Sub, Neg, Mul<i32>, Div<i32>) for
/// a coordinate newtype wrapping an `i32`.
macro_rules! impl_coord_ops {
    ($T:ident) => {
        impl Add for $T {
            type Output = Self;
            #[inline]
            fn add(self, rhs: Self) -> Self {
                $T(self.0 + rhs.0)
            }
        }

        impl Sub for $T {
            type Output = Self;
            #[inline]
            fn sub(self, rhs: Self) -> Self {
                $T(self.0 - rhs.0)
            }
        }

        impl Neg for $T {
            type Output = Self;
            #[inline]
            fn neg(self) -> Self {
                $T(-self.0)
            }
        }

        impl Mul<i32> for $T {
            type Output = Self;
            #[inline]
            fn mul(self, rhs: i32) -> Self {
                $T(self.0 * rhs)
            }
        }

        impl Div<i32> for $T {
            type Output = Self;
            #[inline]
            fn div(self, rhs: i32) -> Self {
                $T(self.0 / rhs)
            }
        }
    };
}

// ---------------------------------------------------------------------------
// SchCoord -- 100,000 units/mil
// ---------------------------------------------------------------------------

/// Schematic coordinate with 100,000 internal units per mil.
///
/// In the ASCII/parameter format, coordinates are stored as an integer part
/// (raw / 10,000) and a fractional part (raw % 10,000), referred to as
/// "DXP parts". In the binary format, they are split into a whole-mil `i16`
/// (raw / 100,000) and a fractional `i32` remainder, used for the Data and
/// PinFrac streams respectively.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct SchCoord(pub(crate) i32);

impl SchCoord {
    /// Zero coordinate.
    pub const ZERO: SchCoord = SchCoord(0);

    /// Splits into DXP parameter parts (integer, fractional).
    ///
    /// The DXP parameter format stores coordinates as two keys:
    /// - `KEY` = integer part (raw / 10,000)
    /// - `KEY_FRAC` = fractional part (raw % 10,000)
    ///
    /// This allows reconstructing the full coordinate: `integer * 10_000 + frac`.
    #[inline]
    pub fn to_dxp_parts(self) -> (i32, i32) {
        (self.0 / 10_000, self.0 % 10_000)
    }

    /// Reconstructs from DXP parameter parts (integer + fractional).
    #[inline]
    pub fn from_dxp_parts(integer: i32, frac: i32) -> Self {
        SchCoord(integer * 10_000 + frac)
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
    pub fn to_binary_parts(self) -> Result<(i16, i32)> {
        let whole = self.0 / 100_000;
        let frac = self.0 - 100_000 * whole;
        let whole_i16 = i16::try_from(whole).map_err(|_| {
            AltiumError::InvalidCoordinate(format!(
                "coordinate {} mils overflows i16 binary representation",
                whole
            ))
        })?;
        Ok((whole_i16, frac))
    }

    /// Reconstructs from binary parts (whole mils `i16` + fractional remainder `i32`).
    #[inline]
    pub fn from_binary_parts(whole: i16, frac: i32) -> Self {
        SchCoord(whole as i32 * 100_000 + frac)
    }

    /// Returns the absolute value.
    #[inline]
    pub fn abs(self) -> Self {
        SchCoord(self.0.abs())
    }

    /// Returns the minimum of two coordinates.
    #[inline]
    pub fn min(self, other: Self) -> Self {
        SchCoord(self.0.min(other.0))
    }

    /// Returns the maximum of two coordinates.
    #[inline]
    pub fn max(self, other: Self) -> Self {
        SchCoord(self.0.max(other.0))
    }
}

impl AltiumCoord for SchCoord {
    const UNITS_PER_MIL: i32 = 100_000;

    #[inline]
    fn from_raw(raw: i32) -> Self {
        SchCoord(raw)
    }

    #[inline]
    fn to_raw(self) -> i32 {
        self.0
    }
}

impl fmt::Debug for SchCoord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SchCoord({} = {:.3}mil)", self.0, self.to_mils())
    }
}

impl fmt::Display for SchCoord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.3}mil", self.to_mils())
    }
}

impl Serialize for SchCoord {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_i32(self.0)
    }
}

impl<'de> Deserialize<'de> for SchCoord {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        i32::deserialize(deserializer).map(SchCoord)
    }
}

impl_coord_ops!(SchCoord);

// ---------------------------------------------------------------------------
// PcbCoord -- 10,000 units/mil
// ---------------------------------------------------------------------------

/// PCB coordinate with 10,000 internal units per mil.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct PcbCoord(pub(crate) i32);

impl PcbCoord {
    /// Zero coordinate.
    pub const ZERO: PcbCoord = PcbCoord(0);

    /// Returns the absolute value.
    #[inline]
    pub fn abs(self) -> Self {
        PcbCoord(self.0.abs())
    }

    /// Returns the minimum of two coordinates.
    #[inline]
    pub fn min(self, other: Self) -> Self {
        PcbCoord(self.0.min(other.0))
    }

    /// Returns the maximum of two coordinates.
    #[inline]
    pub fn max(self, other: Self) -> Self {
        PcbCoord(self.0.max(other.0))
    }
}

impl AltiumCoord for PcbCoord {
    const UNITS_PER_MIL: i32 = 10_000;

    #[inline]
    fn from_raw(raw: i32) -> Self {
        PcbCoord(raw)
    }

    #[inline]
    fn to_raw(self) -> i32 {
        self.0
    }
}

impl fmt::Debug for PcbCoord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PcbCoord({} = {:.3}mil)", self.0, self.to_mils())
    }
}

impl fmt::Display for PcbCoord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.3}mil", self.to_mils())
    }
}

impl Serialize for PcbCoord {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_i32(self.0)
    }
}

impl<'de> Deserialize<'de> for PcbCoord {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        i32::deserialize(deserializer).map(PcbCoord)
    }
}

impl_coord_ops!(PcbCoord);

// ---------------------------------------------------------------------------
// Point<C> -- generic 2D point
// ---------------------------------------------------------------------------

/// A 2D point parameterized by coordinate type.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Point<C: AltiumCoord> {
    pub x: C,
    pub y: C,
}

/// Schematic point alias.
pub type SchPoint = Point<SchCoord>;

/// PCB point alias.
pub type PcbPoint = Point<PcbCoord>;

impl<C: AltiumCoord> Point<C> {
    /// Creates a new point.
    #[inline]
    pub fn new(x: C, y: C) -> Self {
        Point { x, y }
    }

    /// Creates a point from mil values.
    #[inline]
    pub fn from_mils(x: f64, y: f64) -> Self {
        Point {
            x: C::from_mils(x),
            y: C::from_mils(y),
        }
    }

    /// Creates a point from millimeter values.
    #[inline]
    pub fn from_mm(x: f64, y: f64) -> Self {
        Point {
            x: C::from_mm(x),
            y: C::from_mm(y),
        }
    }

    /// Creates a point from raw internal-unit values.
    #[inline]
    pub fn from_raw(x: i32, y: i32) -> Self {
        Point {
            x: C::from_raw(x),
            y: C::from_raw(y),
        }
    }
}

impl<C: AltiumCoord> Add for Point<C> {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Point {
            x: C::from_raw(self.x.to_raw() + rhs.x.to_raw()),
            y: C::from_raw(self.y.to_raw() + rhs.y.to_raw()),
        }
    }
}

impl<C: AltiumCoord> Sub for Point<C> {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Point {
            x: C::from_raw(self.x.to_raw() - rhs.x.to_raw()),
            y: C::from_raw(self.y.to_raw() - rhs.y.to_raw()),
        }
    }
}

impl<C: AltiumCoord> fmt::Debug for Point<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Point({:?}, {:?})", self.x, self.y)
    }
}

impl<C: AltiumCoord> fmt::Display for Point<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "({:.3}mil, {:.3}mil)",
            self.x.to_mils(),
            self.y.to_mils()
        )
    }
}

// ---------------------------------------------------------------------------
// Rect<C> -- generic axis-aligned rectangle
// ---------------------------------------------------------------------------

/// An axis-aligned rectangle parameterized by coordinate type.
///
/// `min` holds the corner with the smallest x and y values; `max` holds
/// the corner with the largest.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Rect<C: AltiumCoord> {
    pub min: Point<C>,
    pub max: Point<C>,
}

/// Schematic rectangle alias.
pub type SchRect = Rect<SchCoord>;

/// PCB rectangle alias.
pub type PcbRect = Rect<PcbCoord>;

impl<C: AltiumCoord> Rect<C> {
    /// Creates a rectangle from two corner points, normalizing so `min <= max`.
    pub fn new(p1: Point<C>, p2: Point<C>) -> Self {
        Rect {
            min: Point {
                x: if p1.x.to_raw() < p2.x.to_raw() {
                    p1.x
                } else {
                    p2.x
                },
                y: if p1.y.to_raw() < p2.y.to_raw() {
                    p1.y
                } else {
                    p2.y
                },
            },
            max: Point {
                x: if p1.x.to_raw() > p2.x.to_raw() {
                    p1.x
                } else {
                    p2.x
                },
                y: if p1.y.to_raw() > p2.y.to_raw() {
                    p1.y
                } else {
                    p2.y
                },
            },
        }
    }

    /// Creates a rectangle from four raw coordinate values.
    pub fn from_raw(x1: i32, y1: i32, x2: i32, y2: i32) -> Self {
        Self::new(Point::from_raw(x1, y1), Point::from_raw(x2, y2))
    }

    /// Returns the width (max.x - min.x).
    #[inline]
    pub fn width(self) -> C {
        C::from_raw(self.max.x.to_raw() - self.min.x.to_raw())
    }

    /// Returns the height (max.y - min.y).
    #[inline]
    pub fn height(self) -> C {
        C::from_raw(self.max.y.to_raw() - self.min.y.to_raw())
    }

    /// Returns the center point.
    #[inline]
    pub fn center(self) -> Point<C> {
        Point {
            x: C::from_raw((self.min.x.to_raw() + self.max.x.to_raw()) / 2),
            y: C::from_raw((self.min.y.to_raw() + self.max.y.to_raw()) / 2),
        }
    }

    /// Returns `true` if the rectangle contains the given point (inclusive bounds).
    #[inline]
    pub fn contains(self, point: Point<C>) -> bool {
        self.min.x <= point.x
            && point.x <= self.max.x
            && self.min.y <= point.y
            && point.y <= self.max.y
    }

    /// Returns `true` if this rectangle intersects with another (inclusive bounds).
    #[inline]
    pub fn intersects(self, other: Rect<C>) -> bool {
        self.min.x.to_raw() <= other.max.x.to_raw()
            && self.max.x.to_raw() >= other.min.x.to_raw()
            && self.min.y.to_raw() <= other.max.y.to_raw()
            && self.max.y.to_raw() >= other.min.y.to_raw()
    }
}

impl<C: AltiumCoord> fmt::Debug for Rect<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Rect({:?} .. {:?})", self.min, self.max)
    }
}

// ---------------------------------------------------------------------------
// Measurement<U> -- unit-tagged floating-point value
// ---------------------------------------------------------------------------

/// Marker trait for physical unit types.
pub trait Unit {
    /// Number of mils per one of this unit.
    const MILS_PER_UNIT: f64;
    /// Short abbreviation for display (e.g. "mm", "mil").
    const ABBREVIATION: &'static str;
}

/// Millimeters unit marker.
pub struct Millimeters;

impl Unit for Millimeters {
    // 1 mm = 1/0.0254 mils ~ 39.3701 mils
    const MILS_PER_UNIT: f64 = 1.0 / 0.0254;
    const ABBREVIATION: &'static str = "mm";
}

/// Mils (thousandths of an inch) unit marker.
pub struct Mils;

impl Unit for Mils {
    const MILS_PER_UNIT: f64 = 1.0;
    const ABBREVIATION: &'static str = "mil";
}

/// Inches unit marker.
pub struct Inches;

impl Unit for Inches {
    const MILS_PER_UNIT: f64 = 1000.0;
    const ABBREVIATION: &'static str = "in";
}

/// A physical measurement in a specific unit.
///
/// This is used at the CLI/DTO boundary for user-facing values; it converts
/// to concrete coordinate types via `From` impls.
pub struct Measurement<U: Unit>(pub f64, PhantomData<U>);

/// Millimeter measurement alias.
pub type Mm = Measurement<Millimeters>;

/// Mil measurement alias.
pub type Mil = Measurement<Mils>;

impl<U: Unit> Measurement<U> {
    /// Creates a new measurement.
    #[inline]
    pub fn new(value: f64) -> Self {
        Measurement(value, PhantomData)
    }

    /// Returns the value in mils.
    #[inline]
    pub fn to_mils(self) -> f64 {
        self.0 * U::MILS_PER_UNIT
    }
}

impl<U: Unit> fmt::Debug for Measurement<U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Measurement({} {})", self.0, U::ABBREVIATION)
    }
}

impl<U: Unit> fmt::Display for Measurement<U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.0, U::ABBREVIATION)
    }
}

impl<U: Unit> Clone for Measurement<U> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<U: Unit> Copy for Measurement<U> {}

impl<U: Unit> From<Measurement<U>> for SchCoord {
    fn from(m: Measurement<U>) -> Self {
        SchCoord::from_mils(m.to_mils())
    }
}

impl<U: Unit> From<Measurement<U>> for PcbCoord {
    fn from(m: Measurement<U>) -> Self {
        PcbCoord::from_mils(m.to_mils())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coord_from_mils() {
        // SchCoord: 1 mil = 100,000 raw units
        let sc = SchCoord::from_mils(1.0);
        assert_eq!(sc.to_raw(), 100_000);

        // PcbCoord: 1 mil = 10,000 raw units
        let pc = PcbCoord::from_mils(1.0);
        assert_eq!(pc.to_raw(), 10_000);
    }

    #[test]
    fn coord_round_trip_mils() {
        let sc = SchCoord::from_mils(42.5);
        assert!((sc.to_mils() - 42.5).abs() < 1e-6);

        let pc = PcbCoord::from_mils(42.5);
        assert!((pc.to_mils() - 42.5).abs() < 1e-6);
    }

    #[test]
    fn sch_coord_dxp_parts() {
        // 3.5 mils at 100K units/mil = 350,000 raw
        let c = SchCoord::from_raw(350_000);
        let (integer, frac) = c.to_dxp_parts();
        // DXP: raw / 10_000 = 35, raw % 10_000 = 0
        assert_eq!(integer, 35);
        assert_eq!(frac, 0);
        assert_eq!(SchCoord::from_dxp_parts(integer, frac), c);

        // Test a value with a non-zero fractional DXP part
        let c2 = SchCoord::from_raw(354_321);
        let (integer2, frac2) = c2.to_dxp_parts();
        assert_eq!(integer2, 35);
        assert_eq!(frac2, 4_321);
        assert_eq!(SchCoord::from_dxp_parts(integer2, frac2), c2);

        // Negative value
        let c3 = SchCoord::from_raw(-250_000);
        let (integer3, frac3) = c3.to_dxp_parts();
        assert_eq!(integer3, -25);
        assert_eq!(frac3, 0);
        assert_eq!(SchCoord::from_dxp_parts(integer3, frac3), c3);
    }

    #[test]
    fn sch_coord_binary_parts() {
        // Positive value with fractional part
        let c = SchCoord::from_raw(350_123);
        let (whole, frac) = c.to_binary_parts().unwrap();
        assert_eq!(whole, 3);
        assert_eq!(frac, 50_123);
        assert_eq!(SchCoord::from_binary_parts(whole, frac), c);

        // Negative value
        let c2 = SchCoord::from_raw(-250_000);
        let (whole2, frac2) = c2.to_binary_parts().unwrap();
        assert_eq!(whole2, -2);
        assert_eq!(frac2, -50_000);
        assert_eq!(SchCoord::from_binary_parts(whole2, frac2), c2);

        // Exact mil boundary
        let c3 = SchCoord::from_mils(100.0);
        let (whole3, frac3) = c3.to_binary_parts().unwrap();
        assert_eq!(whole3, 100);
        assert_eq!(frac3, 0);
    }

    #[test]
    fn point_arithmetic() {
        let p1 = SchPoint::from_mils(10.0, 20.0);
        let p2 = SchPoint::from_mils(5.0, 8.0);

        let sum = p1 + p2;
        assert!((sum.x.to_mils() - 15.0).abs() < 1e-6);
        assert!((sum.y.to_mils() - 28.0).abs() < 1e-6);

        let diff = p1 - p2;
        assert!((diff.x.to_mils() - 5.0).abs() < 1e-6);
        assert!((diff.y.to_mils() - 12.0).abs() < 1e-6);
    }

    #[test]
    fn rect_contains() {
        let r = SchRect::new(
            SchPoint::from_mils(10.0, 20.0),
            SchPoint::from_mils(100.0, 80.0),
        );

        // Inside
        assert!(r.contains(SchPoint::from_mils(50.0, 50.0)));
        // On boundary
        assert!(r.contains(SchPoint::from_mils(10.0, 20.0)));
        assert!(r.contains(SchPoint::from_mils(100.0, 80.0)));
        // Outside
        assert!(!r.contains(SchPoint::from_mils(5.0, 50.0)));
        assert!(!r.contains(SchPoint::from_mils(50.0, 90.0)));

        // Width / height
        assert!((r.width().to_mils() - 90.0).abs() < 1e-6);
        assert!((r.height().to_mils() - 60.0).abs() < 1e-6);

        // Center
        let center = r.center();
        assert!((center.x.to_mils() - 55.0).abs() < 1e-6);
        assert!((center.y.to_mils() - 50.0).abs() < 1e-6);

        // Intersects
        let r2 = SchRect::new(
            SchPoint::from_mils(90.0, 70.0),
            SchPoint::from_mils(200.0, 200.0),
        );
        assert!(r.intersects(r2));

        let r3 = SchRect::new(
            SchPoint::from_mils(200.0, 200.0),
            SchPoint::from_mils(300.0, 300.0),
        );
        assert!(!r.intersects(r3));
    }

    #[test]
    fn measurement_conversion() {
        // 1 mm -> SchCoord
        let m = Mm::new(1.0);
        let sc: SchCoord = m.into();
        // 1 mm = 1/0.0254 mils ~ 39.3701 mils
        assert!((sc.to_mils() - (1.0 / 0.0254)).abs() < 0.1);

        // 1 mm -> PcbCoord
        let pc: PcbCoord = Mm::new(1.0).into();
        assert!((pc.to_mils() - (1.0 / 0.0254)).abs() < 0.1);

        // 100 mil -> SchCoord
        let mil_m = Mil::new(100.0);
        let sc2: SchCoord = mil_m.into();
        assert_eq!(sc2.to_raw(), 100_000 * 100);

        // 1 inch -> PcbCoord
        let inch_m = Measurement::<Inches>::new(1.0);
        let pc2: PcbCoord = inch_m.into();
        assert!((pc2.to_mils() - 1000.0).abs() < 1e-6);
    }

    #[test]
    fn coord_arithmetic_ops() {
        let a = SchCoord::from_mils(10.0);
        let b = SchCoord::from_mils(3.0);

        assert_eq!((a + b).to_raw(), SchCoord::from_mils(13.0).to_raw());
        assert_eq!((a - b).to_raw(), SchCoord::from_mils(7.0).to_raw());
        assert_eq!((-a).to_raw(), SchCoord::from_mils(-10.0).to_raw());
        assert_eq!((a * 2).to_raw(), SchCoord::from_mils(20.0).to_raw());
        assert_eq!((a / 2).to_raw(), SchCoord::from_mils(5.0).to_raw());

        // Same for PcbCoord
        let c = PcbCoord::from_mils(10.0);
        let d = PcbCoord::from_mils(3.0);

        assert_eq!((c + d).to_raw(), PcbCoord::from_mils(13.0).to_raw());
        assert_eq!((c - d).to_raw(), PcbCoord::from_mils(7.0).to_raw());
        assert_eq!((-c).to_raw(), PcbCoord::from_mils(-10.0).to_raw());
        assert_eq!((c * 2).to_raw(), PcbCoord::from_mils(20.0).to_raw());
        assert_eq!((c / 2).to_raw(), PcbCoord::from_mils(5.0).to_raw());
    }

    #[test]
    fn serde_roundtrip() {
        let sc = SchCoord::from_raw(12345);
        let json = serde_json::to_string(&sc).unwrap();
        assert_eq!(json, "12345");
        let sc2: SchCoord = serde_json::from_str(&json).unwrap();
        assert_eq!(sc, sc2);

        let pc = PcbCoord::from_raw(-99999);
        let json2 = serde_json::to_string(&pc).unwrap();
        assert_eq!(json2, "-99999");
        let pc2: PcbCoord = serde_json::from_str(&json2).unwrap();
        assert_eq!(pc, pc2);
    }

    #[test]
    fn mm_round_trip() {
        let sc = SchCoord::from_mm(2.54);
        // 2.54 mm = 100 mils
        assert!((sc.to_mils() - 100.0).abs() < 0.1);
        assert!((sc.to_mm() - 2.54).abs() < 0.01);

        let pc = PcbCoord::from_mm(2.54);
        assert!((pc.to_mils() - 100.0).abs() < 0.1);
        assert!((pc.to_mm() - 2.54).abs() < 0.01);
    }
}
