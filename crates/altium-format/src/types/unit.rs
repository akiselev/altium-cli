//! Unit types for Altium coordinate conversions.

use crate::error::{AltiumError, Result};
use crate::types::Coord;
use std::fmt;

/// Measurement units supported by Altium.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum Unit {
    /// Mils (thousandths of an inch).
    #[default]
    Mil,
    /// Millimeters.
    Millimeter,
    /// Inches.
    Inch,
    /// Centimeters.
    Centimeter,
    /// DXP default units (10 mils).
    DxpDefault,
    /// Meters.
    Meter,
}

impl Unit {
    /// Returns the suffix string for this unit.
    pub fn suffix(self) -> &'static str {
        match self {
            Unit::Mil => "mil",
            Unit::Millimeter => "mm",
            Unit::Inch => "in",
            Unit::Centimeter => "cm",
            Unit::DxpDefault => "",
            Unit::Meter => "m",
        }
    }

    /// Returns the display name for this unit.
    pub fn name(self) -> &'static str {
        match self {
            Unit::Mil => "Mils",
            Unit::Millimeter => "Millimeters",
            Unit::Inch => "Inches",
            Unit::Centimeter => "Centimeters",
            Unit::DxpDefault => "Dxp Defaults",
            Unit::Meter => "Meters",
        }
    }

    /// Returns true if this is a metric unit.
    pub fn is_metric(self) -> bool {
        matches!(self, Unit::Millimeter | Unit::Centimeter | Unit::Meter)
    }

    /// Converts a value in this unit to a Coord.
    pub fn to_coord(self, value: f64) -> Coord {
        match self {
            Unit::Mil => Coord::from_mils(value),
            Unit::Millimeter => Coord::from_mms(value),
            Unit::Inch => Coord::from_inches(value),
            Unit::Centimeter => Coord::from_cms(value),
            Unit::DxpDefault => Coord::from_dxp(value),
            Unit::Meter => Coord::from_meters(value),
        }
    }

    /// Converts a Coord to a value in this unit.
    pub fn from_coord(self, coord: Coord) -> f64 {
        match self {
            Unit::Mil => coord.to_mils(),
            Unit::Millimeter => coord.to_mms(),
            Unit::Inch => coord.to_inches(),
            Unit::Centimeter => coord.to_cms(),
            Unit::DxpDefault => coord.to_dxp(),
            Unit::Meter => coord.to_meters(),
        }
    }

    /// All available units.
    pub const ALL: &'static [Unit] = &[
        Unit::Mil,
        Unit::Millimeter,
        Unit::Inch,
        Unit::Centimeter,
        Unit::DxpDefault,
        Unit::Meter,
    ];

    /// Parses a string with a unit suffix and returns the coordinate and unit.
    ///
    /// Examples: "100mil", "2.54mm", "1in"
    pub fn parse_with_unit(s: &str) -> Result<(Coord, Unit)> {
        let s = s.trim();

        // Try each unit suffix (longest first to avoid partial matches)
        for &unit in &[
            Unit::Millimeter,
            Unit::Centimeter,
            Unit::DxpDefault,
            Unit::Mil,
            Unit::Inch,
            Unit::Meter,
        ] {
            let suffix = unit.suffix();
            if suffix.is_empty() {
                continue; // Skip DxpDefault for suffix matching
            }
            if let Some(num_str) = s.strip_suffix(suffix) {
                let value: f64 = num_str.trim().parse().map_err(|_| {
                    AltiumError::InvalidCoordinate(format!("Invalid number: {}", num_str))
                })?;
                return Ok((unit.to_coord(value), unit));
            }
        }

        // If no suffix, try as a plain number (DxpDefault)
        if let Ok(value) = s.parse::<f64>() {
            return Ok((Unit::DxpDefault.to_coord(value), Unit::DxpDefault));
        }

        Err(AltiumError::InvalidCoordinate(format!(
            "Cannot parse coordinate: {}",
            s
        )))
    }

    /// Formats a coordinate with this unit.
    pub fn format_coord(self, coord: Coord) -> String {
        let value = self.from_coord(coord);
        format!("{:.3}{}", value, self.suffix())
    }
}

impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl TryFrom<i32> for Unit {
    type Error = AltiumError;

    fn try_from(value: i32) -> Result<Self> {
        match value {
            0 => Ok(Unit::Mil),
            1 => Ok(Unit::Millimeter),
            2 => Ok(Unit::Inch),
            3 => Ok(Unit::Centimeter),
            4 => Ok(Unit::DxpDefault),
            5 => Ok(Unit::Meter),
            _ => Err(AltiumError::InvalidUnit(format!(
                "Unknown unit value: {}",
                value
            ))),
        }
    }
}

impl From<Unit> for i32 {
    fn from(unit: Unit) -> Self {
        match unit {
            Unit::Mil => 0,
            Unit::Millimeter => 1,
            Unit::Inch => 2,
            Unit::Centimeter => 3,
            Unit::DxpDefault => 4,
            Unit::Meter => 5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unit_conversions() {
        let coord = Unit::Mil.to_coord(100.0);
        assert!((Unit::Mil.from_coord(coord) - 100.0).abs() < 0.001);

        let coord = Unit::Millimeter.to_coord(2.54);
        assert!((Unit::Mil.from_coord(coord) - 100.0).abs() < 0.1);
    }

    #[test]
    fn test_parse_with_unit() {
        let (coord, unit) = Unit::parse_with_unit("100mil").unwrap();
        assert_eq!(unit, Unit::Mil);
        assert!((coord.to_mils() - 100.0).abs() < 0.001);

        let (coord, unit) = Unit::parse_with_unit("2.54mm").unwrap();
        assert_eq!(unit, Unit::Millimeter);
        assert!((coord.to_mils() - 100.0).abs() < 0.1);
    }
}
