//! SchEllipse - Schematic ellipse (Record 8).
//!
//! **DEPRECATED**: Use `v2::fields::EllipseData` with `v2::serializer::format_v5` instead.

use crate::error::Result;
use crate::types::{Coord, CoordRect, ParameterCollection, UnknownFields};
use altium_format_derive::AltiumRecord;

use super::{LineWidth, SchGraphicalBase, SchPrimitive};

/// Schematic ellipse primitive.
///
/// **DEPRECATED**: Use `v2::fields::EllipseData` instead.
#[deprecated(note = "Use v2::fields::EllipseData")]
#[derive(Debug, Clone, Default, AltiumRecord)]
#[altium(record_id = 8, format = "params")]
pub struct SchEllipse {
    /// Graphical base (location = center, color).
    #[altium(flatten)]
    pub graphical: SchGraphicalBase,

    /// X radius (raw coord units).
    #[altium(param = "RADIUS", frac = "RADIUS_FRAC")]
    pub radius_x: i32,

    /// Y radius (raw coord units).
    #[altium(param = "SECONDARYRADIUS", frac = "SECONDARYRADIUS_FRAC")]
    pub radius_y: i32,

    /// Line width.
    #[altium(param = "LINEWIDTH", default, skip_default)]
    pub line_width: LineWidth,

    /// Whether the ellipse is solid (filled).
    #[altium(param = "ISSOLID", default, skip_default)]
    pub is_solid: bool,

    /// Whether the fill is transparent.
    #[altium(param = "TRANSPARENT", default, skip_default)]
    pub transparent: bool,

    /// Unknown parameters (preserved for non-destructive editing).
    #[altium(unknown)]
    pub unknown_params: UnknownFields,
}

#[allow(deprecated)]
impl SchPrimitive for SchEllipse {
    const RECORD_ID: i32 = 8;

    fn location(&self) -> Option<crate::types::CoordPoint> {
        Some(crate::types::CoordPoint::from_raw(
            self.graphical.location_x,
            self.graphical.location_y,
        ))
    }

    fn record_type_name(&self) -> &'static str {
        "Ellipse"
    }

    fn import_from_params(_params: &ParameterCollection) -> Result<Self> {
        unimplemented!(
            "V1 SchEllipse::import_from_params is deprecated. \
            Use v2::fields::EllipseData with v2::serializer::format_v5 instead."
        )
    }

    fn export_to_params(&self) -> ParameterCollection {
        unimplemented!(
            "V1 SchEllipse::export_to_params is deprecated. \
            Use v2::fields::EllipseData with v2::serializer::format_v5 instead."
        )
    }

    fn owner_index(&self) -> i32 {
        self.graphical.base.owner_index
    }

    fn calculate_bounds(&self) -> CoordRect {
        CoordRect::from_points(
            Coord::from_raw(self.graphical.location_x - self.radius_x),
            Coord::from_raw(self.graphical.location_y - self.radius_y),
            Coord::from_raw(self.graphical.location_x + self.radius_x),
            Coord::from_raw(self.graphical.location_y + self.radius_y),
        )
    }
}
