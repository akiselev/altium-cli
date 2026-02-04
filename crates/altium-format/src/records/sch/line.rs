//! SchLine - Schematic line (Record 13).
//!
//! **DEPRECATED**: Use `v2::fields::LineData` with `v2::serializer::format_v5` instead.

use crate::error::Result;
use crate::types::{Coord, CoordRect, ParameterCollection, UnknownFields};
use altium_format_derive::AltiumRecord;

use super::{LineWidth, SchGraphicalBase, SchPrimitive};

/// Schematic line primitive.
///
/// **DEPRECATED**: Use `v2::fields::LineData` instead.
#[deprecated(note = "Use v2::fields::LineData")]
#[derive(Debug, Clone, Default, AltiumRecord)]
#[altium(record_id = 13, format = "params")]
pub struct SchLine {
    /// Graphical base (location = start point, color).
    #[altium(flatten)]
    pub graphical: SchGraphicalBase,

    /// End point X.
    #[altium(param = "CORNER.X", frac = "CORNER.X_FRAC")]
    pub corner_x: i32,

    /// End point Y.
    #[altium(param = "CORNER.Y", frac = "CORNER.Y_FRAC")]
    pub corner_y: i32,

    /// Line width.
    #[altium(param = "LINEWIDTH", default, skip_default)]
    pub line_width: LineWidth,

    /// Unknown parameters (preserved for non-destructive editing).
    #[altium(unknown)]
    pub unknown_params: UnknownFields,
}

#[allow(deprecated)]
impl SchPrimitive for SchLine {
    const RECORD_ID: i32 = 13;

    fn location(&self) -> Option<crate::types::CoordPoint> {
        Some(crate::types::CoordPoint::from_raw(
            self.graphical.location_x,
            self.graphical.location_y,
        ))
    }

    fn record_type_name(&self) -> &'static str {
        "Line"
    }

    fn import_from_params(_params: &ParameterCollection) -> Result<Self> {
        unimplemented!(
            "V1 SchLine::import_from_params is deprecated. \
            Use v2::fields::LineData with v2::serializer::format_v5 instead."
        )
    }

    fn export_to_params(&self) -> ParameterCollection {
        unimplemented!(
            "V1 SchLine::export_to_params is deprecated. \
            Use v2::fields::LineData with v2::serializer::format_v5 instead."
        )
    }

    fn owner_index(&self) -> i32 {
        self.graphical.base.owner_index
    }

    fn calculate_bounds(&self) -> CoordRect {
        CoordRect::from_points(
            Coord::from_raw(self.graphical.location_x),
            Coord::from_raw(self.graphical.location_y),
            Coord::from_raw(self.corner_x),
            Coord::from_raw(self.corner_y),
        )
    }
}
