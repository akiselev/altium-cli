//! SchPolygon - Schematic polygon (Record 7).

use crate::error::Result;
use crate::traits::{FromParams, ToParams};
use crate::types::{Coord, CoordRect, ParameterCollection, UnknownFields};
use altium_derive::AltiumRecord;

use super::{LineWidth, SchGraphicalBase, SchPrimitive};

/// Schematic polygon primitive.
#[derive(Debug, Clone, Default, AltiumRecord)]
#[altium(record_id = 7, format = "params")]
pub struct SchPolygon {
    /// Graphical base (location, color).
    #[altium(flatten)]
    pub graphical: SchGraphicalBase,

    /// Whether the polygon is solid (filled).
    #[altium(param = "ISSOLID", default)]
    pub is_solid: bool,

    /// Line width.
    #[altium(param = "LINEWIDTH", default)]
    pub line_width: LineWidth,

    /// Vertices as (x, y) raw coord pairs.
    #[altium(
        indexed_coords,
        prefix_x = "X",
        prefix_y = "Y",
        count = "LOCATIONCOUNT"
    )]
    pub vertices: Vec<(i32, i32)>,

    /// Whether the fill is transparent.
    #[altium(param = "TRANSPARENT", default)]
    pub transparent: bool,

    /// Unknown parameters (preserved for non-destructive editing).
    #[altium(unknown)]
    pub unknown_params: UnknownFields,
}

impl SchPrimitive for SchPolygon {
    const RECORD_ID: i32 = 7;

    fn location(&self) -> Option<crate::types::CoordPoint> {
        Some(crate::types::CoordPoint::from_raw(
            self.graphical.location_x,
            self.graphical.location_y,
        ))
    }

    fn record_type_name(&self) -> &'static str {
        "Polygon"
    }

    fn import_from_params(params: &ParameterCollection) -> Result<Self> {
        Self::from_params(params)
    }

    fn export_to_params(&self) -> ParameterCollection {
        self.to_params()
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
