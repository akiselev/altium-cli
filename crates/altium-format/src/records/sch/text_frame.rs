//! SchTextFrame - Schematic text frame (Record 28) and variant (Record 209).

use crate::error::Result;
use crate::traits::{FromParams, ToParams};
use crate::types::{Coord, CoordRect, ParameterCollection, UnknownFields};
use altium_derive::AltiumRecord;

use super::{LineWidth, SchGraphicalBase, SchPrimitive, TextOrientations};

fn text_frame_bounds(location_x: i32, location_y: i32, corner_x: i32, corner_y: i32) -> CoordRect {
    CoordRect::from_points(
        Coord::from_raw(location_x),
        Coord::from_raw(location_y),
        Coord::from_raw(corner_x),
        Coord::from_raw(corner_y),
    )
}

/// Schematic text frame primitive.
#[derive(Debug, Clone, Default, AltiumRecord)]
#[altium(record_id = 28, format = "params")]
pub struct SchTextFrame {
    /// Graphical base (location = lower-left, color).
    #[altium(flatten)]
    pub graphical: SchGraphicalBase,

    /// Corner point X (opposite corner).
    #[altium(param = "CORNER.X", frac = "CORNER.X_FRAC")]
    pub corner_x: i32,

    /// Corner point Y (opposite corner).
    #[altium(param = "CORNER.Y", frac = "CORNER.Y_FRAC")]
    pub corner_y: i32,

    /// Line width.
    #[altium(param = "LINEWIDTH", default)]
    pub line_width: LineWidth,

    /// Whether the frame is solid (filled).
    #[altium(param = "ISSOLID", default)]
    pub is_solid: bool,

    /// Whether the fill is transparent.
    #[altium(param = "TRANSPARENT", default)]
    pub transparent: bool,

    /// Font ID (references fonts in the document).
    #[altium(param = "FONTID", default)]
    pub font_id: i32,

    /// Text alignment.
    #[altium(param = "ALIGNMENT", default)]
    pub alignment: i32,

    /// Whether to wrap words.
    #[altium(param = "WORDWRAP", default)]
    pub word_wrap: bool,

    /// Whether to clip text to the frame.
    #[altium(param = "CLIPTORECT", default)]
    pub clip_to_rect: bool,

    /// Text color.
    #[altium(param = "TEXTCOLOR", default)]
    pub text_color: i32,

    /// Text contents.
    #[altium(param = "TEXT", default)]
    pub text: String,

    /// Text orientation.
    #[altium(param = "ORIENTATION", default)]
    pub orientation: TextOrientations,

    /// Text margin (raw coord units).
    #[altium(param = "TEXTMARGIN", frac = "TEXTMARGIN_FRAC")]
    pub text_margin: i32,

    /// Whether to show the border.
    #[altium(param = "SHOWBORDER", default)]
    pub show_border: bool,

    /// Unknown parameters (preserved for non-destructive editing).
    #[altium(unknown)]
    pub unknown_params: UnknownFields,
}

impl SchPrimitive for SchTextFrame {
    const RECORD_ID: i32 = 28;

    fn location(&self) -> Option<crate::types::CoordPoint> {
        Some(crate::types::CoordPoint::from_raw(
            self.graphical.location_x,
            self.graphical.location_y,
        ))
    }

    fn record_type_name(&self) -> &'static str {
        "TextFrame"
    }

    fn get_property(&self, name: &str) -> Option<String> {
        match name {
            "TEXT" => Some(self.text.clone()),
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
        self.graphical.base.owner_index
    }

    fn calculate_bounds(&self) -> CoordRect {
        text_frame_bounds(
            self.graphical.location_x,
            self.graphical.location_y,
            self.corner_x,
            self.corner_y,
        )
    }
}

/// Schematic text frame variant (Record 209).
#[derive(Debug, Clone, Default, AltiumRecord)]
#[altium(record_id = 209, format = "params")]
pub struct SchTextFrameVariant {
    /// Graphical base (location = lower-left, color).
    #[altium(flatten)]
    pub graphical: SchGraphicalBase,

    /// Corner point X (opposite corner).
    #[altium(param = "CORNER.X", frac = "CORNER.X_FRAC")]
    pub corner_x: i32,

    /// Corner point Y (opposite corner).
    #[altium(param = "CORNER.Y", frac = "CORNER.Y_FRAC")]
    pub corner_y: i32,

    /// Line width.
    #[altium(param = "LINEWIDTH", default)]
    pub line_width: LineWidth,

    /// Whether the frame is solid (filled).
    #[altium(param = "ISSOLID", default)]
    pub is_solid: bool,

    /// Whether the fill is transparent.
    #[altium(param = "TRANSPARENT", default)]
    pub transparent: bool,

    /// Font ID (references fonts in the document).
    #[altium(param = "FONTID", default)]
    pub font_id: i32,

    /// Text alignment.
    #[altium(param = "ALIGNMENT", default)]
    pub alignment: i32,

    /// Whether to wrap words.
    #[altium(param = "WORDWRAP", default)]
    pub word_wrap: bool,

    /// Whether to clip text to the frame.
    #[altium(param = "CLIPTORECT", default)]
    pub clip_to_rect: bool,

    /// Text color.
    #[altium(param = "TEXTCOLOR", default)]
    pub text_color: i32,

    /// Text contents.
    #[altium(param = "TEXT", default)]
    pub text: String,

    /// Text orientation.
    #[altium(param = "ORIENTATION", default)]
    pub orientation: TextOrientations,

    /// Text margin (raw coord units).
    #[altium(param = "TEXTMARGIN", frac = "TEXTMARGIN_FRAC")]
    pub text_margin: i32,

    /// Whether to show the border.
    #[altium(param = "SHOWBORDER", default)]
    pub show_border: bool,

    /// Unknown parameters (preserved for non-destructive editing).
    #[altium(unknown)]
    pub unknown_params: UnknownFields,
}

impl SchPrimitive for SchTextFrameVariant {
    const RECORD_ID: i32 = 209;

    fn location(&self) -> Option<crate::types::CoordPoint> {
        Some(crate::types::CoordPoint::from_raw(
            self.graphical.location_x,
            self.graphical.location_y,
        ))
    }

    fn record_type_name(&self) -> &'static str {
        "TextFrameVariant"
    }

    fn get_property(&self, name: &str) -> Option<String> {
        match name {
            "TEXT" => Some(self.text.clone()),
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
        self.graphical.base.owner_index
    }

    fn calculate_bounds(&self) -> CoordRect {
        text_frame_bounds(
            self.graphical.location_x,
            self.graphical.location_y,
            self.corner_x,
            self.corner_y,
        )
    }
}
