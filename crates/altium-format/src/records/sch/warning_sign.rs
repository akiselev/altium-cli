//! SchWarningSign - Schematic warning sign (Record 43).

use crate::error::Result;
use crate::traits::{FromParams, ToParams};
use crate::types::{Coord, CoordRect, ParameterCollection, UnknownFields};
use altium_format_derive::AltiumRecord;

use super::{SchGraphicalBase, SchPrimitive, TextOrientations};

/// Schematic warning sign primitive.
#[derive(Debug, Clone, Default, AltiumRecord)]
#[altium(record_id = 43, format = "params")]
pub struct SchWarningSign {
    /// Graphical base (location, color).
    #[altium(flatten)]
    pub graphical: SchGraphicalBase,

    /// Warning name (e.g., DIFFPAIR).
    #[altium(param = "NAME", default)]
    pub name: String,

    /// Orientation for the warning text.
    #[altium(param = "ORIENTATION", default)]
    pub orientation: TextOrientations,

    /// Unknown parameters (preserved for non-destructive editing).
    #[altium(unknown)]
    pub unknown_params: UnknownFields,
}

impl SchPrimitive for SchWarningSign {
    const RECORD_ID: i32 = 43;

    fn location(&self) -> Option<crate::types::CoordPoint> {
        Some(crate::types::CoordPoint::from_raw(
            self.graphical.location_x,
            self.graphical.location_y,
        ))
    }

    fn record_type_name(&self) -> &'static str {
        "WarningSign"
    }

    fn get_property(&self, name: &str) -> Option<String> {
        match name {
            "NAME" => Some(self.name.clone()),
            _ => None,
        }
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
            Coord::from_raw(self.graphical.location_x + 1),
            Coord::from_raw(self.graphical.location_y + 1),
        )
    }
}
