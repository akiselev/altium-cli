//! SchPolyline - Schematic polyline (Record 6).
//!
//! **DEPRECATED**: Use `v2::fields::PolylineData` with `v2::serializer::format_v5` instead.

use crate::error::Result;
use crate::types::{Coord, CoordRect, ParameterCollection, UnknownFields};
use altium_format_derive::AltiumRecord;

use super::{LineStyle, LineWidth, SchGraphicalBase, SchPrimitive};

/// Schematic polyline primitive.
///
/// **DEPRECATED**: Use `v2::fields::PolylineData` instead.
#[deprecated(note = "Use v2::fields::PolylineData")]
#[derive(Debug, Clone, Default, AltiumRecord)]
#[altium(record_id = 6, format = "params")]
pub struct SchPolyline {
    /// Graphical base (location, color).
    #[altium(flatten)]
    pub graphical: SchGraphicalBase,

    /// Line width.
    #[altium(param = "LINEWIDTH", default, skip_default)]
    pub line_width: LineWidth,

    /// Line style.
    #[altium(param = "LINESTYLE", default, skip_default)]
    pub line_style: LineStyle,

    /// Whether it's solid (filled - mainly for polygon compatibility).
    #[altium(param = "ISSOLID", default, skip_default)]
    pub is_solid: bool,

    /// Vertices as (x, y) raw coord pairs.
    #[altium(
        indexed_coords,
        prefix_x = "X",
        prefix_y = "Y",
        count = "LOCATIONCOUNT"
    )]
    pub vertices: Vec<(i32, i32)>,

    /// Unknown parameters (preserved for non-destructive editing).
    #[altium(unknown)]
    pub unknown_params: UnknownFields,
}

// Provide polygon field for compatibility with existing code
impl SchPolyline {
    /// Get a polygon-like view for compatibility.
    pub fn polygon(&self) -> PolygonView<'_> {
        PolygonView { polyline: self }
    }
}

/// View into polyline's polygon-like data.
pub struct PolygonView<'a> {
    polyline: &'a SchPolyline,
}

impl<'a> PolygonView<'a> {
    pub fn vertices(&self) -> &[(i32, i32)] {
        &self.polyline.vertices
    }

    pub fn graphical(&self) -> &SchGraphicalBase {
        &self.polyline.graphical
    }
}

#[allow(deprecated)]
impl SchPrimitive for SchPolyline {
    const RECORD_ID: i32 = 6;

    fn location(&self) -> Option<crate::types::CoordPoint> {
        Some(crate::types::CoordPoint::from_raw(
            self.graphical.location_x,
            self.graphical.location_y,
        ))
    }

    fn record_type_name(&self) -> &'static str {
        "Polyline"
    }

    fn import_from_params(_params: &ParameterCollection) -> Result<Self> {
        unimplemented!(
            "V1 SchPolyline::import_from_params is deprecated. \
            Use v2::fields::PolylineData with v2::serializer::format_v5 instead."
        )
    }

    fn export_to_params(&self) -> ParameterCollection {
        unimplemented!(
            "V1 SchPolyline::export_to_params is deprecated. \
            Use v2::fields::PolylineData with v2::serializer::format_v5 instead."
        )
    }

    fn owner_index(&self) -> i32 {
        self.graphical.base.owner_index
    }

    fn calculate_bounds(&self) -> CoordRect {
        let Some(&(first_x, first_y)) = self.vertices.first() else {
            return CoordRect::empty();
        };

        let (min_x, max_x, min_y, max_y) = self.vertices.iter().skip(1).fold(
            (first_x, first_x, first_y, first_y),
            |(min_x, max_x, min_y, max_y), &(x, y)| {
                (min_x.min(x), max_x.max(x), min_y.min(y), max_y.max(y))
            },
        );

        CoordRect::from_points(
            Coord::from_raw(min_x),
            Coord::from_raw(min_y),
            Coord::from_raw(max_x),
            Coord::from_raw(max_y),
        )
    }
}
