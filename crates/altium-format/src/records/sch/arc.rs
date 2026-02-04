//! SchArc - Schematic arc (Record 12) and SchEllipticalArc (Record 11).
//!
//! **DEPRECATED**: Use `v2::fields::ArcData` with `v2::serializer::format_v5` instead.

use crate::error::Result;
use crate::types::{Coord, CoordRect, ParameterCollection, UnknownFields};
use altium_format_derive::AltiumRecord;

use super::{LineWidth, SchGraphicalBase, SchPrimitive};

/// Schematic elliptical arc primitive (Record 11).
///
/// **DEPRECATED**: Use `v2::fields::EllipticalArcData` instead.
#[deprecated(note = "Use v2::fields::EllipticalArcData")]
///
/// This is identical to SchArc but specifically has a secondary radius
/// for drawing elliptical arc segments.
#[derive(Debug, Clone, Default, AltiumRecord)]
#[altium(record_id = 11, format = "params")]
pub struct SchEllipticalArc {
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
    #[altium(param = "STARTANGLE", default, skip_default)]
    pub start_angle: f64,

    /// End angle in degrees.
    #[altium(param = "ENDANGLE", default, skip_default)]
    pub end_angle: f64,

    /// Line width.
    #[altium(param = "LINEWIDTH", default, skip_default)]
    pub line_width: LineWidth,

    /// Unknown parameters (preserved for non-destructive editing).
    #[altium(unknown)]
    pub unknown_params: UnknownFields,
}

#[allow(deprecated)]
impl SchPrimitive for SchEllipticalArc {
    const RECORD_ID: i32 = 11;

    fn location(&self) -> Option<crate::types::CoordPoint> {
        Some(crate::types::CoordPoint::from_raw(
            self.graphical.location_x,
            self.graphical.location_y,
        ))
    }

    fn record_type_name(&self) -> &'static str {
        "EllipticalArc"
    }

    fn import_from_params(_params: &ParameterCollection) -> Result<Self> {
        unimplemented!(
            "V1 SchEllipticalArc::import_from_params is deprecated. \
            Use v2::fields::EllipticalArcData with v2::serializer::format_v5 instead."
        )
    }

    fn export_to_params(&self) -> ParameterCollection {
        unimplemented!(
            "V1 SchEllipticalArc::export_to_params is deprecated. \
            Use v2::fields::EllipticalArcData with v2::serializer::format_v5 instead."
        )
    }

    fn owner_index(&self) -> i32 {
        self.graphical.base.owner_index
    }

    fn calculate_bounds(&self) -> CoordRect {
        // Approximate bounds as bounding box of full ellipse
        CoordRect::from_points(
            Coord::from_raw(self.graphical.location_x - self.radius),
            Coord::from_raw(self.graphical.location_y - self.secondary_radius),
            Coord::from_raw(self.graphical.location_x + self.radius),
            Coord::from_raw(self.graphical.location_y + self.secondary_radius),
        )
    }
}

/// Schematic arc primitive.
///
/// **DEPRECATED**: Use `v2::fields::ArcData` instead.
#[deprecated(note = "Use v2::fields::ArcData")]
#[derive(Debug, Clone, Default, AltiumRecord)]
#[altium(record_id = 12, format = "params")]
pub struct SchArc {
    /// Graphical base (location = center, color).
    #[altium(flatten)]
    pub graphical: SchGraphicalBase,

    /// Radius (raw coord units).
    #[altium(param = "RADIUS", frac = "RADIUS_FRAC")]
    pub radius: i32,

    /// Secondary radius (for elliptical arcs, omitted when equal to radius).
    #[altium(param = "SECONDARYRADIUS", frac = "SECONDARYRADIUS_FRAC", skip_default)]
    pub secondary_radius: i32,

    /// Start angle in degrees.
    #[altium(param = "STARTANGLE", default, skip_default)]
    pub start_angle: f64,

    /// End angle in degrees.
    #[altium(param = "ENDANGLE", default, skip_default)]
    pub end_angle: f64,

    /// Line width.
    #[altium(param = "LINEWIDTH", default, skip_default)]
    pub line_width: LineWidth,

    /// Unknown parameters (preserved for non-destructive editing).
    #[altium(unknown)]
    pub unknown_params: UnknownFields,
}

#[allow(deprecated)]
impl SchPrimitive for SchArc {
    const RECORD_ID: i32 = 12;

    fn location(&self) -> Option<crate::types::CoordPoint> {
        Some(crate::types::CoordPoint::from_raw(
            self.graphical.location_x,
            self.graphical.location_y,
        ))
    }

    fn record_type_name(&self) -> &'static str {
        "Arc"
    }

    fn import_from_params(_params: &ParameterCollection) -> Result<Self> {
        unimplemented!(
            "V1 SchArc::import_from_params is deprecated. \
            Use v2::fields::ArcData with v2::serializer::format_v5 instead."
        )
    }

    fn export_to_params(&self) -> ParameterCollection {
        unimplemented!(
            "V1 SchArc::export_to_params is deprecated. \
            Use v2::fields::ArcData with v2::serializer::format_v5 instead."
        )
    }

    fn owner_index(&self) -> i32 {
        self.graphical.base.owner_index
    }

    fn calculate_bounds(&self) -> CoordRect {
        // Approximate bounds as bounding box of full circle
        let r = self.radius.max(self.secondary_radius);
        CoordRect::from_points(
            Coord::from_raw(self.graphical.location_x - r),
            Coord::from_raw(self.graphical.location_y - r),
            Coord::from_raw(self.graphical.location_x + r),
            Coord::from_raw(self.graphical.location_y + r),
        )
    }
}
