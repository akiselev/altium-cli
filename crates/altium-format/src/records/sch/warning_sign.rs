//! SchWarningSign - Schematic warning sign (Record 43).
//!
//! **DEPRECATED**: Use `v2::fields::WarningSignData` with `v2::serializer::format_v5` instead.

use crate::error::Result;
use crate::types::{Coord, CoordRect, ParameterCollection, UnknownFields};
use altium_format_derive::AltiumRecord;

use super::{SchGraphicalBase, SchPrimitive, TextOrientations};

/// Schematic warning sign primitive.
///
/// **DEPRECATED**: Use `v2::fields::WarningSignData` instead.
#[deprecated(note = "Use v2::fields::WarningSignData")]
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

#[allow(deprecated)]
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

    fn import_from_params(_params: &ParameterCollection) -> Result<Self> {
        unimplemented!(
            "V1 SchWarningSign::import_from_params is deprecated. \
            Use v2::fields::WarningSignData with v2::serializer::format_v5 instead."
        )
    }

    fn export_to_params(&self) -> ParameterCollection {
        unimplemented!(
            "V1 SchWarningSign::export_to_params is deprecated. \
            Use v2::fields::WarningSignData with v2::serializer::format_v5 instead."
        )
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
