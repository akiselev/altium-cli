//! SchJunction - Schematic junction (Record 29).

use crate::error::Result;
use crate::traits::{FromParams, ToParams};
use crate::types::{Coord, CoordRect, ParameterCollection, UnknownFields};
use altium_format_derive::AltiumRecord;

use super::{SchGraphicalBase, SchPrimitive};

/// Schematic junction primitive - a dot at wire intersections.
#[derive(Debug, Clone, Default, AltiumRecord)]
#[altium(record_id = 29, format = "params")]
pub struct SchJunction {
    /// Graphical base (location, color).
    #[altium(flatten)]
    pub graphical: SchGraphicalBase,

    /// Unknown parameters (preserved for non-destructive editing).
    #[altium(unknown)]
    pub unknown_params: UnknownFields,
}

impl SchPrimitive for SchJunction {
    const RECORD_ID: i32 = 29;

    fn location(&self) -> Option<crate::types::CoordPoint> {
        Some(crate::types::CoordPoint::from_raw(
            self.graphical.location_x,
            self.graphical.location_y,
        ))
    }

    fn record_type_name(&self) -> &'static str {
        "Junction"
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
        // Junction is a small dot, approximate with small size
        let size = 5000; // 0.5 mil
        CoordRect::from_points(
            Coord::from_raw(self.graphical.location_x - size),
            Coord::from_raw(self.graphical.location_y - size),
            Coord::from_raw(self.graphical.location_x + size),
            Coord::from_raw(self.graphical.location_y + size),
        )
    }
}
