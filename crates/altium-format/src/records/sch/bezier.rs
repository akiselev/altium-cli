//! SchBezier - Schematic bezier curve (Record 5).
//!
//! **DEPRECATED**: Use `v2::fields::BezierData` with `v2::serializer::format_v5` instead.

use crate::error::Result;
use crate::types::{Coord, CoordRect, ParameterCollection, UnknownFields};
use altium_format_derive::AltiumRecord;

use super::{LineWidth, SchGraphicalBase, SchPrimitive};

/// Schematic bezier curve primitive.
/// Vertices are interpreted as bezier control points.
///
/// **DEPRECATED**: Use `v2::fields::BezierData` instead.
#[deprecated(note = "Use v2::fields::BezierData")]
#[derive(Debug, Clone, Default, AltiumRecord)]
#[altium(record_id = 5, format = "params")]
pub struct SchBezier {
    /// Graphical base (location, color).
    #[altium(flatten)]
    pub graphical: SchGraphicalBase,

    /// Line width.
    #[altium(param = "LINEWIDTH", default, skip_default)]
    pub line_width: LineWidth,

    /// Control points as (x, y) raw coord pairs.
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
impl SchBezier {
    /// Get a polygon-like view for compatibility.
    pub fn polygon(&self) -> BezierPolygonView<'_> {
        BezierPolygonView { bezier: self }
    }
}

/// View into bezier's polygon-like data.
pub struct BezierPolygonView<'a> {
    bezier: &'a SchBezier,
}

impl<'a> BezierPolygonView<'a> {
    pub fn vertices(&self) -> &[(i32, i32)] {
        &self.bezier.vertices
    }

    pub fn graphical(&self) -> &SchGraphicalBase {
        &self.bezier.graphical
    }
}

#[allow(deprecated)]
impl SchPrimitive for SchBezier {
    const RECORD_ID: i32 = 5;

    fn location(&self) -> Option<crate::types::CoordPoint> {
        Some(crate::types::CoordPoint::from_raw(
            self.graphical.location_x,
            self.graphical.location_y,
        ))
    }

    fn record_type_name(&self) -> &'static str {
        "Bezier"
    }

    fn import_from_params(_params: &ParameterCollection) -> Result<Self> {
        unimplemented!(
            "V1 SchBezier::import_from_params is deprecated. \
            Use v2::fields::BezierData with v2::serializer::format_v5 instead."
        )
    }

    fn export_to_params(&self) -> ParameterCollection {
        unimplemented!(
            "V1 SchBezier::export_to_params is deprecated. \
            Use v2::fields::BezierData with v2::serializer::format_v5 instead."
        )
    }

    fn owner_index(&self) -> i32 {
        self.graphical.base.owner_index
    }

    fn calculate_bounds(&self) -> CoordRect {
        // Approximate bounds from control points
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
