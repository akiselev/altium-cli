//! SchJunction - Schematic junction (Record 29).
//!
//! **DEPRECATED**: Use `v2::fields::JunctionData` with `v2::serializer::format_v5` instead.

use crate::error::Result;
use crate::types::{Coord, CoordRect, ParameterCollection, UnknownFields};
use altium_format_derive::AltiumRecord;

use super::{SchGraphicalBase, SchPrimitive};

/// Schematic junction primitive - a dot at wire intersections.
///
/// **DEPRECATED**: Use `v2::fields::JunctionData` instead.
#[deprecated(note = "Use v2::fields::JunctionData")]
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

#[allow(deprecated)]
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

    fn import_from_params(_params: &ParameterCollection) -> Result<Self> {
        unimplemented!(
            "V1 SchJunction::import_from_params is deprecated. \
            Use v2::fields::JunctionData with v2::serializer::format_v5 instead."
        )
    }

    fn export_to_params(&self) -> ParameterCollection {
        unimplemented!(
            "V1 SchJunction::export_to_params is deprecated. \
            Use v2::fields::JunctionData with v2::serializer::format_v5 instead."
        )
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
