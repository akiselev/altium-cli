//! Core types for Altium file parsing.
//!
//! This module contains fundamental types used throughout the library:
//! - Coordinate types (Coord, CoordPoint, CoordRect)
//! - Unit types for measurement conversions
//! - Layer definitions for PCB layers
//! - Color type for Altium colors
//! - Parameter collection for key-value storage
//! - Unknown fields for non-destructive editing

mod color;
mod coord;
mod layer;
mod mask_expansion;
mod parameters;
mod unit;
mod unknown;

pub use color::Color;
pub use coord::{Coord, CoordPoint, CoordPoint3D, CoordRect};
pub use layer::Layer;
pub use mask_expansion::MaskExpansion;
pub use parameters::{ParameterCollection, ParameterValue};
pub use unit::Unit;
pub use unknown::UnknownFields;

/// Combine DXP integer and fractional parts into a raw coordinate value.
///
/// Altium stores coordinates as integer + fractional parts where:
/// - integer is the main value
/// - frac is the fractional part (0-9999)
/// - raw = integer * 10000 + frac
#[inline]
pub fn dxp_frac_to_coord(integer: i32, frac: i32) -> i32 {
    integer * 10000 + frac
}

/// Split a raw coordinate value into DXP integer and fractional parts.
#[inline]
pub fn coord_to_dxp_frac(raw: i32) -> (i32, i32) {
    (raw / 10000, raw % 10000)
}
