//! SchLabel - Schematic label (Record 4).
//!
//! **DEPRECATED**: Use `v2::fields::LabelData` with `v2::serializer::format_v5` instead.

use crate::error::Result;
use crate::types::{Coord, CoordRect, ParameterCollection, UnknownFields};
use altium_format_derive::AltiumRecord;

use super::{SchGraphicalBase, SchPrimitive, TextJustification, TextOrientations};

/// Schematic label primitive.
///
/// **DEPRECATED**: Use `v2::fields::LabelData` instead.
#[deprecated(note = "Use v2::fields::LabelData")]
#[derive(Debug, Clone, Default, AltiumRecord)]
#[altium(record_id = 4, format = "params")]
pub struct SchLabel {
    /// Graphical base (location, color).
    #[altium(flatten)]
    pub graphical: SchGraphicalBase,

    /// Text orientation.
    #[altium(param = "ORIENTATION", default, skip_default)]
    pub orientation: TextOrientations,

    /// Text justification.
    #[altium(param = "JUSTIFICATION", default, skip_default)]
    pub justification: TextJustification,

    /// Font ID (references fonts in the document).
    #[altium(param = "FONTID", default)]
    pub font_id: i32,

    /// Label text.
    #[altium(param = "TEXT", default, skip_default)]
    pub text: String,

    /// Whether the text is mirrored.
    #[altium(param = "ISMIRRORED", default, skip_default)]
    pub is_mirrored: bool,

    /// Whether the label is hidden.
    #[altium(param = "ISHIDDEN", default, skip_default)]
    pub is_hidden: bool,

    /// Unknown parameters (preserved for non-destructive editing).
    #[altium(unknown)]
    pub unknown_params: UnknownFields,
}

impl SchLabel {
    /// Returns the display text (for subclass override).
    pub fn display_text(&self) -> &str {
        &self.text
    }
}

#[allow(deprecated)]
impl SchPrimitive for SchLabel {
    const RECORD_ID: i32 = 4;

    fn location(&self) -> Option<crate::types::CoordPoint> {
        Some(crate::types::CoordPoint::from_raw(
            self.graphical.location_x,
            self.graphical.location_y,
        ))
    }

    fn record_type_name(&self) -> &'static str {
        "Label"
    }

    fn get_property(&self, name: &str) -> Option<String> {
        match name {
            "TEXT" => Some(self.text.clone()),
            _ => None,
        }
    }

    fn import_from_params(_params: &ParameterCollection) -> Result<Self> {
        unimplemented!(
            "V1 SchLabel::import_from_params is deprecated. \
            Use v2::fields::LabelData with v2::serializer::format_v5 instead."
        )
    }

    fn export_to_params(&self) -> ParameterCollection {
        unimplemented!(
            "V1 SchLabel::export_to_params is deprecated. \
            Use v2::fields::LabelData with v2::serializer::format_v5 instead."
        )
    }

    fn owner_index(&self) -> i32 {
        self.graphical.base.owner_index
    }

    fn calculate_bounds(&self) -> CoordRect {
        // Text bounds depend on font metrics, return point bounds for now
        CoordRect::from_points(
            Coord::from_raw(self.graphical.location_x),
            Coord::from_raw(self.graphical.location_y),
            Coord::from_raw(self.graphical.location_x + 1),
            Coord::from_raw(self.graphical.location_y + 1),
        )
    }
}
