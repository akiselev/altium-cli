//! SchRectangle - Schematic rectangle (Record 14).

use crate::error::Result;
use crate::traits::{FromParams, ToParams};
use crate::types::{Coord, CoordRect, ParameterCollection, UnknownFields};
use altium_format_derive::AltiumRecord;

use super::{LineWidth, SchGraphicalBase, SchPrimitive};

/// Schematic rectangle primitive.
#[derive(Debug, Clone, Default, AltiumRecord)]
#[altium(record_id = 14, format = "params")]
pub struct SchRectangle {
    /// Graphical base (location = one corner, color).
    #[altium(flatten)]
    pub graphical: SchGraphicalBase,

    /// Corner point X (opposite corner).
    #[altium(param = "CORNER.X", frac = "CORNER.X_FRAC")]
    pub corner_x: i32,

    /// Corner point Y (opposite corner).
    #[altium(param = "CORNER.Y", frac = "CORNER.Y_FRAC")]
    pub corner_y: i32,

    /// Line width.
    #[altium(param = "LINEWIDTH", default)]
    pub line_width: LineWidth,

    /// Whether the rectangle is solid (filled).
    #[altium(param = "ISSOLID", default)]
    pub is_solid: bool,

    /// Whether the fill is transparent.
    #[altium(param = "TRANSPARENT", default)]
    pub transparent: bool,

    /// Unknown parameters (preserved for non-destructive editing).
    #[altium(unknown)]
    pub unknown_params: UnknownFields,
}

impl SchPrimitive for SchRectangle {
    const RECORD_ID: i32 = 14;

    fn location(&self) -> Option<crate::types::CoordPoint> {
        Some(crate::types::CoordPoint::from_raw(
            self.graphical.location_x,
            self.graphical.location_y,
        ))
    }

    fn record_type_name(&self) -> &'static str {
        "Rectangle"
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
        CoordRect::from_points(
            Coord::from_raw(self.graphical.location_x),
            Coord::from_raw(self.graphical.location_y),
            Coord::from_raw(self.corner_x),
            Coord::from_raw(self.corner_y),
        )
    }
}
