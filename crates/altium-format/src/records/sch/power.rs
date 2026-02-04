//! SchPowerObject - Schematic power object (Record 17).
//!
//! **DEPRECATED**: Use `v2::fields::PowerObjectData` with `v2::serializer::format_v5` instead.

use crate::error::Result;
use crate::types::{Coord, CoordRect, ParameterCollection, UnknownFields};
use altium_format_derive::AltiumRecord;

use super::{PowerObjectStyle, SchGraphicalBase, SchPrimitive, TextOrientations};

/// Schematic power object primitive (power/ground symbols).
///
/// **DEPRECATED**: Use `v2::fields::PowerObjectData` instead.
#[deprecated(note = "Use v2::fields::PowerObjectData")]
#[derive(Debug, Clone, Default, AltiumRecord)]
#[altium(record_id = 17, format = "params")]
pub struct SchPowerObject {
    /// Graphical base (location, color).
    #[altium(flatten)]
    pub graphical: SchGraphicalBase,

    /// Power object style.
    #[altium(param = "STYLE", default)]
    pub style: PowerObjectStyle,

    /// Orientation.
    #[altium(param = "ORIENTATION", default)]
    pub orientation: TextOrientations,

    /// Net/text.
    #[altium(param = "TEXT", default)]
    pub text: String,

    /// Whether to show net name.
    #[altium(param = "SHOWNETNAME", default)]
    pub show_net_name: bool,

    /// Font ID.
    #[altium(param = "FONTID", default)]
    pub font_id: i32,

    /// Unknown parameters (preserved for non-destructive editing).
    #[altium(unknown)]
    pub unknown_params: UnknownFields,
}

#[allow(deprecated)]
impl SchPrimitive for SchPowerObject {
    const RECORD_ID: i32 = 17;

    fn location(&self) -> Option<crate::types::CoordPoint> {
        Some(crate::types::CoordPoint::from_raw(
            self.graphical.location_x,
            self.graphical.location_y,
        ))
    }

    fn record_type_name(&self) -> &'static str {
        "PowerObject"
    }

    fn get_property(&self, name: &str) -> Option<String> {
        match name {
            "TEXT" => Some(self.text.clone()),
            _ => None,
        }
    }

    fn import_from_params(_params: &ParameterCollection) -> Result<Self> {
        unimplemented!(
            "V1 SchPowerObject::import_from_params is deprecated. \
            Use v2::fields::PowerObjectData with v2::serializer::format_v5 instead."
        )
    }

    fn export_to_params(&self) -> ParameterCollection {
        unimplemented!(
            "V1 SchPowerObject::export_to_params is deprecated. \
            Use v2::fields::PowerObjectData with v2::serializer::format_v5 instead."
        )
    }

    fn owner_index(&self) -> i32 {
        self.graphical.base.owner_index
    }

    fn calculate_bounds(&self) -> CoordRect {
        // Power objects have fixed-size symbols
        let size = 50000; // 5 mil
        CoordRect::from_points(
            Coord::from_raw(self.graphical.location_x - size),
            Coord::from_raw(self.graphical.location_y - size),
            Coord::from_raw(self.graphical.location_x + size),
            Coord::from_raw(self.graphical.location_y + size),
        )
    }
}
