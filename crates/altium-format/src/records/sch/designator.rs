//! SchDesignator - Component designator (Record 34).
//!
//! A designator is a specialized parameter that displays the component reference.
//!
//! **DEPRECATED**: Use `v2::fields::DesignatorData` with `v2::serializer::format_v5` instead.

use crate::error::Result;
use crate::types::{CoordRect, ParameterCollection, UnknownFields};
use altium_format_derive::AltiumRecord;

use super::{SchParameter, SchPrimitive};

/// Schematic designator primitive - displays component reference designator.
///
/// This is essentially a specialized SchParameter with record type 34.
///
/// **DEPRECATED**: Use `v2::fields::DesignatorData` instead.
#[deprecated(note = "Use v2::fields::DesignatorData")]
#[derive(Debug, Clone, Default, AltiumRecord)]
#[altium(record_id = 34, format = "params")]
pub struct SchDesignator {
    /// Base parameter data (includes label, name, read-only state).
    #[altium(flatten)]
    pub param: SchParameter,

    /// Unknown parameters (preserved for non-destructive editing).
    #[altium(unknown)]
    pub unknown_params: UnknownFields,
}

impl SchDesignator {
    /// Get the designator text.
    pub fn text(&self) -> &str {
        self.param.value()
    }
}

#[allow(deprecated)]
impl SchPrimitive for SchDesignator {
    const RECORD_ID: i32 = 34;

    fn location(&self) -> Option<crate::types::CoordPoint> {
        Some(crate::types::CoordPoint::from_raw(
            self.param.label.graphical.location_x,
            self.param.label.graphical.location_y,
        ))
    }

    fn record_type_name(&self) -> &'static str {
        "Designator"
    }

    fn get_property(&self, name: &str) -> Option<String> {
        match name {
            "NAME" => Some(self.param.name.clone()),
            "TEXT" => Some(self.param.label.text.clone()),
            _ => None,
        }
    }

    fn import_from_params(_params: &ParameterCollection) -> Result<Self> {
        unimplemented!(
            "V1 SchDesignator::import_from_params is deprecated. \
            Use v2::fields::DesignatorData with v2::serializer::format_v5 instead."
        )
    }

    fn export_to_params(&self) -> ParameterCollection {
        unimplemented!(
            "V1 SchDesignator::export_to_params is deprecated. \
            Use v2::fields::DesignatorData with v2::serializer::format_v5 instead."
        )
    }

    fn owner_index(&self) -> i32 {
        self.param.owner_index()
    }

    fn calculate_bounds(&self) -> CoordRect {
        self.param.calculate_bounds()
    }
}
