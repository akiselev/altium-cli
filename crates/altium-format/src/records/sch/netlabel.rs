//! SchNetLabel - Schematic net label (Record 25).

use crate::error::Result;
use crate::traits::{FromParams, ToParams};
use crate::types::{CoordRect, ParameterCollection, UnknownFields};
use altium_derive::AltiumRecord;

use super::{SchLabel, SchPrimitive};

/// Schematic net label primitive - extends label for net naming.
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
