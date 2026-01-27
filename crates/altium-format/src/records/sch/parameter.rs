//! SchParameter - Schematic parameter (Record 41).

use crate::error::Result;
use crate::traits::{FromParams, ToParams};
use crate::types::{CoordRect, ParameterCollection, UnknownFields};
use altium_derive::AltiumRecord;

use super::{SchLabel, SchPrimitive};

/// Schematic parameter primitive - named parameter text.
#[derive(Debug, Clone, Default, AltiumRecord)]
#[altium(record_id = 41, format = "params")]
pub struct SchParameter {
    /// Base label data (includes graphical base, text, orientation, etc).
    #[altium(flatten)]
    pub label: SchLabel,

    /// Parameter name.
    #[altium(param = "NAME", default)]
    pub name: String,

    /// Whether to read-only autoposition.
    #[altium(param = "READONLYSTATE", default)]
    pub read_only_state: i32,

    /// Unknown parameters (preserved for non-destructive editing).
    #[altium(unknown)]
    pub unknown_params: UnknownFields,
}

impl SchParameter {
    /// Get the parameter name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the parameter value (the text).
    pub fn value(&self) -> &str {
        &self.label.text
    }
}

impl SchPrimitive for SchParameter {
    const RECORD_ID: i32 = 41;

    fn location(&self) -> Option<crate::types::CoordPoint> {
        Some(crate::types::CoordPoint::from_raw(
            self.label.graphical.location_x,
            self.label.graphical.location_y,
        ))
    }

    fn record_type_name(&self) -> &'static str {
        "Parameter"
    }

    fn get_property(&self, name: &str) -> Option<String> {
        match name {
            "NAME" => Some(self.name.clone()),
            "TEXT" => Some(self.label.text.clone()),
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
        self.label.graphical.base.owner_index
    }

    fn calculate_bounds(&self) -> CoordRect {
        self.label.calculate_bounds()
    }
}
