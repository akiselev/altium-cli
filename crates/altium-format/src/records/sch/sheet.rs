//! SchSheetHeader - Schematic sheet header (Record 31).

use crate::error::Result;
use crate::traits::{FromParams, ToParams};
use crate::types::{CoordRect, ParameterCollection, UnknownFields};
use altium_format_derive::AltiumRecord;

use super::{SchPrimitive, SchPrimitiveBase};

/// Schematic sheet header primitive.
#[derive(Debug, Clone, Default, AltiumRecord)]
#[altium(record_id = 31, format = "params")]
pub struct SchSheetHeader {
    /// Base primitive fields.
    #[altium(flatten)]
    pub base: SchPrimitiveBase,

    /// Font ID count.
    #[altium(param = "FONTIDCOUNT", default)]
    pub font_id_count: i32,

    /// Sheet size (style).
    #[altium(param = "SHEETSTYLE", default)]
    pub sheet_size: i32,

    /// Custom X size.
    #[altium(param = "CUSTOMX", default)]
    pub custom_x: i32,

    /// Custom Y size.
    #[altium(param = "CUSTOMY", default)]
    pub custom_y: i32,

    /// Workspace orientation.
    #[altium(param = "WORKSPACEORIENTATION", default)]
    pub workspace_orientation: i32,

    /// Unknown parameters (preserved for non-destructive editing).
    #[altium(unknown)]
    pub unknown_params: UnknownFields,
}

impl SchPrimitive for SchSheetHeader {
    const RECORD_ID: i32 = 31;

    fn record_type_name(&self) -> &'static str {
        "SheetHeader"
    }

    fn import_from_params(params: &ParameterCollection) -> Result<Self> {
        Self::from_params(params)
    }

    fn export_to_params(&self) -> ParameterCollection {
        self.to_params()
    }

    fn owner_index(&self) -> i32 {
        self.base.owner_index
    }

    fn calculate_bounds(&self) -> CoordRect {
        CoordRect::empty()
    }
}
