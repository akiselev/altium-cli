//! SchPie - Schematic pie/wedge shape (Record 9).
//!
//! **DEPRECATED**: Use `v2::fields::PieData` with `v2::serializer::format_v5` instead.

use crate::error::Result;
use crate::types::{Coord, CoordRect, ParameterCollection, UnknownFields};
use altium_format_derive::AltiumRecord;

use super::{LineWidth, SchGraphicalBase, SchPrimitive};

/// Schematic pie/wedge primitive.
///
/// **DEPRECATED**: Use `v2::fields::PieData` instead.
#[deprecated(note = "Use v2::fields::PieData")]
#[derive(Debug, Clone, Default, AltiumRecord)]
#[altium(record_id = 9, format = "params")]
pub struct SchPie {
    /// Graphical base (location = center, color).
    #[altium(flatten)]
    pub graphical: SchGraphicalBase,

    /// Primary radius (raw coord units).
    #[altium(param = "RADIUS", frac = "RADIUS_FRAC")]
    pub radius: i32,

    /// Secondary radius (raw coord units).
    #[altium(param = "SECONDARYRADIUS", frac = "SECONDARYRADIUS_FRAC")]
    pub secondary_radius: i32,

    /// Start angle in degrees.
    #[altium(param = "STARTANGLE", default)]
    pub start_angle: f64,

    /// End angle in degrees.
    #[altium(param = "ENDANGLE", default)]
    pub end_angle: f64,

    /// Line width.
    #[altium(param = "LINEWIDTH", default)]
    pub line_width: LineWidth,

    /// Whether the pie is solid (filled).
    #[altium(param = "ISSOLID", default)]
    pub is_solid: bool,

    /// Unknown parameters (preserved for non-destructive editing).
    #[altium(unknown)]
    pub unknown_params: UnknownFields,
}

#[allow(deprecated)]
impl SchPrimitive for SchPie {
    const RECORD_ID: i32 = 9;

    fn location(&self) -> Option<crate::types::CoordPoint> {
        Some(crate::types::CoordPoint::from_raw(
            self.graphical.location_x,
            self.graphical.location_y,
        ))
    }

    fn record_type_name(&self) -> &'static str {
        "Pie"
    }

    fn import_from_params(_params: &ParameterCollection) -> Result<Self> {
        unimplemented!(
            "V1 SchPie::import_from_params is deprecated. \
            Use v2::fields::PieData with v2::serializer::format_v5 instead."
        )
    }

    fn export_to_params(&self) -> ParameterCollection {
        unimplemented!(
            "V1 SchPie::export_to_params is deprecated. \
            Use v2::fields::PieData with v2::serializer::format_v5 instead."
        )
    }

    fn owner_index(&self) -> i32 {
        self.graphical.base.owner_index
    }

    fn calculate_bounds(&self) -> CoordRect {
        let secondary = if self.secondary_radius == 0 {
            self.radius
        } else {
            self.secondary_radius
        };

        CoordRect::from_points(
            Coord::from_raw(self.graphical.location_x - self.radius),
            Coord::from_raw(self.graphical.location_y - secondary),
            Coord::from_raw(self.graphical.location_x + self.radius),
            Coord::from_raw(self.graphical.location_y + secondary),
        )
    }
}
