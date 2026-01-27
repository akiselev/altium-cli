//! SchDesignator - Component designator (Record 34).
//!
//! A designator is a specialized parameter that displays the component reference.

use crate::error::Result;
use crate::traits::{FromParams, ToParams};
use crate::types::{CoordRect, ParameterCollection, UnknownFields};
use altium_format_derive::AltiumRecord;

use super::{SchParameter, SchPrimitive};

/// Schematic designator primitive - displays component reference designator.
///
/// This is essentially a specialized SchParameter with record type 34.
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

    fn import_from_params(params: &ParameterCollection) -> Result<Self> {
        let mut result = Self::from_params(params)?;
        // Designators typically have read-only state of 1
        if result.param.read_only_state == 0 {
            result.param.read_only_state = 1;
        }
        Ok(result)
    }

    fn export_to_params(&self) -> ParameterCollection {
        self.to_params()
    }

    fn owner_index(&self) -> i32 {
        self.param.owner_index()
    }

    fn calculate_bounds(&self) -> CoordRect {
        self.param.calculate_bounds()
    }
}
