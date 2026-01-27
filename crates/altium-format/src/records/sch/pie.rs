//! SchPie - Schematic pie/wedge shape (Record 9).

use crate::error::Result;
use crate::traits::{FromParams, ToParams};
use crate::types::{Coord, CoordRect, ParameterCollection, UnknownFields};
use altium_format_derive::AltiumRecord;

use super::{LineWidth, SchGraphicalBase, SchPrimitive};

/// Schematic pie/wedge primitive.
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
