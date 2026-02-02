//! PCB coordinate system: 10,000 internal units per mil.
//!
//! Confirmed from Altium SDK: `InternalUnits = 10000`, `k1Mil = 10000`.
//! 1 internal unit = 0.1 microinch = 2.54 nanometers.
//!
//! This is DISTINCT from SchLib's V2Coord which uses 100,000 units/mil.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Sub};

/// Conversion factor: 10,000 internal units per mil.
pub const PCB_UNITS_PER_MIL: f64 = 10_000.0;

/// Nanometers per internal unit (2.54 nm).
pub const NM_PER_UNIT: f64 = 2.54;

/// PCB coordinate value using 10K units/mil scale.
///
/// All PCB binary fields (X, Y, Width, Radius, etc.) use this coordinate system.
/// Stored as `i32` in little-endian format in binary records.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PcbCoord(pub(crate) i32);

impl PcbCoord {
    pub const ZERO: PcbCoord = PcbCoord(0);

    /// Maximum coordinate (99999 mils = ~2540mm).
    pub const MAX: PcbCoord = PcbCoord(999_990_000);

    /// Creates from raw internal units (as stored in binary files).
    #[inline]
    pub const fn from_raw(value: i32) -> Self {
        PcbCoord(value)
    }

    /// Returns the raw internal unit value.
    #[inline]
    pub const fn to_raw(self) -> i32 {
        self.0
    }

    /// Creates from mils (thousandths of an inch).
    #[inline]
    pub fn from_mils(mils: f64) -> Self {
        PcbCoord((mils * PCB_UNITS_PER_MIL) as i32)
    }

    /// Converts to mils.
    #[inline]
    pub fn to_mils(self) -> f64 {
        self.0 as f64 / PCB_UNITS_PER_MIL
    }

    /// Creates from millimeters.
    #[inline]
    pub fn from_mms(mms: f64) -> Self {
        Self::from_mils(mms / 0.0254)
    }

    /// Converts to millimeters.
    #[inline]
    pub fn to_mms(self) -> f64 {
        self.to_mils() * 0.0254
    }

    /// Creates from nanometers.
    #[inline]
    pub fn from_nm(nm: f64) -> Self {
        PcbCoord((nm / NM_PER_UNIT) as i32)
    }

    /// Converts to nanometers.
    #[inline]
    pub fn to_nm(self) -> f64 {
        self.0 as f64 * NM_PER_UNIT
    }

    /// Absolute value.
    #[inline]
    pub fn abs(self) -> Self {
        PcbCoord(self.0.abs())
    }

    #[inline]
    pub fn min(self, other: Self) -> Self {
        PcbCoord(self.0.min(other.0))
    }

    #[inline]
    pub fn max(self, other: Self) -> Self {
        PcbCoord(self.0.max(other.0))
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

impl Add for PcbCoord {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        PcbCoord(self.0 + rhs.0)
    }
}

impl Sub for PcbCoord {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        PcbCoord(self.0 - rhs.0)
    }
}

impl Neg for PcbCoord {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        PcbCoord(-self.0)
    }
}

impl Mul<i32> for PcbCoord {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: i32) -> Self {
        PcbCoord(self.0 * rhs)
    }
}

impl Div<i32> for PcbCoord {
    type Output = Self;
    #[inline]
    fn div(self, rhs: i32) -> Self {
        PcbCoord(self.0 / rhs)
    }
}

/// A 2D point using PCB coordinates.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug, Serialize, Deserialize)]
pub struct PcbPoint {
    pub x: PcbCoord,
    pub y: PcbCoord,
}

impl PcbPoint {
    pub const ORIGIN: PcbPoint = PcbPoint {
        x: PcbCoord::ZERO,
        y: PcbCoord::ZERO,
    };

    pub fn new(x: PcbCoord, y: PcbCoord) -> Self {
        PcbPoint { x, y }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coord_from_mils() {
        let c = PcbCoord::from_mils(1.0);
        assert_eq!(c.to_raw(), 10_000);
    }

    #[test]
    fn coord_round_trip_mils() {
        let c = PcbCoord::from_mils(42.5);
        assert!((c.to_mils() - 42.5).abs() < 1e-6);
    }

    #[test]
    fn coord_to_nm() {
        let c = PcbCoord::from_raw(10_000);
        assert!((c.to_nm() - 25_400.0).abs() < 1e-6);
    }

    #[test]
    fn coord_from_nm() {
        let c = PcbCoord::from_nm(25_400.0);
        assert_eq!(c.to_raw(), 10_000);
    }

    #[test]
    fn coord_max() {
        assert!((PcbCoord::MAX.to_mils() - 99999.0).abs() < 1e-6);
    }
}
