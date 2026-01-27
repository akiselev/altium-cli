//! SchEllipse - Schematic ellipse (Record 8).

use crate::error::Result;
use crate::traits::{FromParams, ToParams};
use crate::types::{Coord, CoordRect, ParameterCollection, UnknownFields};
use altium_format_derive::AltiumRecord;

use super::{LineWidth, SchGraphicalBase, SchPrimitive};

/// Schematic ellipse primitive.
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
    #[altium(param = "LINEWIDTH", default)]
    pub line_width: LineWidth,

    /// Whether the ellipse is solid (filled).
    #[altium(param = "ISSOLID", default)]
    pub is_solid: bool,

    /// Whether the fill is transparent.
    #[altium(param = "TRANSPARENT", default)]
    pub transparent: bool,

    /// Unknown parameters (preserved for non-destructive editing).
    #[altium(unknown)]
    pub unknown_params: UnknownFields,
}

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
            Coord::from_raw(self.graphical.location_x - self.radius_x),
            Coord::from_raw(self.graphical.location_y - self.radius_y),
            Coord::from_raw(self.graphical.location_x + self.radius_x),
            Coord::from_raw(self.graphical.location_y + self.radius_y),
        )
    }
}
