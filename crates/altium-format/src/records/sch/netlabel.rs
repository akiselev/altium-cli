//! SchNetLabel - Schematic net label (Record 25).
//!
//! **DEPRECATED**: Use `v2::fields::NetLabelData` with `v2::serializer::format_v5` instead.

use crate::error::Result;
use crate::types::{CoordRect, ParameterCollection, UnknownFields};
use altium_format_derive::AltiumRecord;

use super::{SchLabel, SchPrimitive};

/// Schematic net label primitive - extends label for net naming.
///
/// **DEPRECATED**: Use `v2::fields::NetLabelData` instead.
#[deprecated(note = "Use v2::fields::NetLabelData")]
#[derive(Debug, Clone, Default, AltiumRecord)]
#[altium(record_id = 25, format = "params")]
pub struct SchNetLabel {
    /// Base label data (includes graphical base, text, orientation, etc).
    #[altium(flatten)]
    pub label: SchLabel,

    /// Unknown parameters (preserved for non-destructive editing).
    #[altium(unknown)]
    pub unknown_params: UnknownFields,
}

#[allow(deprecated)]
impl SchPrimitive for SchNetLabel {
    const RECORD_ID: i32 = 25;

    fn location(&self) -> Option<crate::types::CoordPoint> {
        Some(crate::types::CoordPoint::from_raw(
            self.label.graphical.location_x,
            self.label.graphical.location_y,
        ))
    }

    fn record_type_name(&self) -> &'static str {
        "NetLabel"
    }

    fn get_property(&self, name: &str) -> Option<String> {
        match name {
            "TEXT" => Some(self.label.text.clone()),
            _ => None,
        }
    }

    fn import_from_params(_params: &ParameterCollection) -> Result<Self> {
        unimplemented!(
            "V1 SchNetLabel::import_from_params is deprecated. \
            Use v2::fields::NetLabelData with v2::serializer::format_v5 instead."
        )
    }

    fn export_to_params(&self) -> ParameterCollection {
        unimplemented!(
            "V1 SchNetLabel::export_to_params is deprecated. \
            Use v2::fields::NetLabelData with v2::serializer::format_v5 instead."
        )
    }

    fn owner_index(&self) -> i32 {
        self.label.graphical.base.owner_index
    }

    fn calculate_bounds(&self) -> CoordRect {
        self.label.calculate_bounds()
    }
}
