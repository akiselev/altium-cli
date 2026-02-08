//! PCB coordinate annotation record type.
//!
//! Coordinates are annotation objects placed on PCB documents to mark
//! specific positions. They display X/Y coordinates at a given location.
//!
//! Note: The binary format for Coordinates6/Data in PcbDoc files has not
//! been fully verified from real data, as all available test files have
//! empty Coordinates6/Data streams. The field set is derived from
//! PcbApi_QueryCoordinate analysis and follows the same parameter format
//! pattern as other PCB primitive types.

use crate::types::{Coord, Layer, ParameterCollection};

/// PCB coordinate annotation record.
///
/// Coordinate annotations display the X/Y position at a specific location
/// on the board. They are similar to dimensions but show absolute coordinates
/// rather than distances between two points.
///
/// In PcbDoc files, coordinates are stored in the `Coordinates6/Data` stream.
/// The exact binary framing (whether it has the 2-byte header like Dimensions6
/// or uses plain parameter blocks like Components6) has not been confirmed
/// from real data.
#[derive(Debug, Clone, Default)]
pub struct PcbCoordinate {
    /// Layer the coordinate annotation is on.
    pub layer: Layer,
    /// X coordinate of the annotation.
    pub x: Coord,
    /// Y coordinate of the annotation.
    pub y: Coord,
    /// Angle of the annotation in degrees.
    pub angle: f64,

    // Text properties
    /// Height of the text.
    pub text_height: Coord,
    /// Width/stroke width of the text.
    pub text_width: Coord,
    /// Whether to use TrueType fonts.
    pub use_tt_fonts: bool,
    /// Whether text is bold.
    pub bold: bool,
    /// Whether text is italic.
    pub italic: bool,
    /// TrueType font name.
    pub font_name: String,

    // Common primitive flags
    /// Whether selected.
    pub selection: bool,
    /// Whether locked.
    pub locked: bool,
    /// Whether this is a polygon outline.
    pub polygon_outline: bool,
    /// Whether user-routed.
    pub user_routed: bool,
    /// Whether this is a keepout region.
    pub keepout: bool,
    /// Union index (for grouping).
    pub union_index: i32,
    /// Whether primitives are locked.
    pub primitive_lock: bool,
    /// Whether there is a DRC error.
    pub drc_error: bool,
    /// Unique save index.
    pub v_index_for_save: i32,

    /// All parameters for round-tripping.
    pub params: ParameterCollection,
}

impl PcbCoordinate {
    /// Parse a coordinate from parameters.
    pub fn from_params(params: &ParameterCollection) -> Self {
        PcbCoordinate {
            layer: params
                .get("LAYER")
                .map(|v| v.as_layer())
                .unwrap_or_default(),
            x: params
                .get("X")
                .and_then(|v| v.as_coord().ok())
                .unwrap_or_default(),
            y: params
                .get("Y")
                .and_then(|v| v.as_coord().ok())
                .unwrap_or_default(),
            angle: params
                .get("ANGLE")
                .map(|v| v.as_double_or(0.0))
                .unwrap_or(0.0),
            text_height: params
                .get("TEXTHEIGHT")
                .and_then(|v| v.as_coord().ok())
                .unwrap_or_default(),
            text_width: params
                .get("TEXTWIDTH")
                .and_then(|v| v.as_coord().ok())
                .unwrap_or_default(),
            use_tt_fonts: params
                .get("USETTFONTS")
                .map(|v| v.as_bool_or(false))
                .unwrap_or(false),
            bold: params
                .get("BOLD")
                .map(|v| v.as_bool_or(false))
                .unwrap_or(false),
            italic: params
                .get("ITALIC")
                .map(|v| v.as_bool_or(false))
                .unwrap_or(false),
            font_name: params
                .get("FONTNAME")
                .map(|v| v.as_str().to_string())
                .unwrap_or_default(),
            selection: params
                .get("SELECTION")
                .map(|v| v.as_bool_or(false))
                .unwrap_or(false),
            locked: params
                .get("LOCKED")
                .map(|v| v.as_bool_or(false))
                .unwrap_or(false),
            polygon_outline: params
                .get("POLYGONOUTLINE")
                .map(|v| v.as_bool_or(false))
                .unwrap_or(false),
            user_routed: params
                .get("USERROUTED")
                .map(|v| v.as_bool_or(true))
                .unwrap_or(true),
            keepout: params
                .get("KEEPOUT")
                .map(|v| v.as_bool_or(false))
                .unwrap_or(false),
            union_index: params
                .get("UNIONINDEX")
                .map(|v| v.as_int_or(0))
                .unwrap_or(0),
            primitive_lock: params
                .get("PRIMITIVELOCK")
                .map(|v| v.as_bool_or(false))
                .unwrap_or(false),
            drc_error: params
                .get("DRCERROR")
                .map(|v| v.as_bool_or(false))
                .unwrap_or(false),
            v_index_for_save: params
                .get("VINDEXFORSAVE")
                .map(|v| v.as_int_or(0))
                .unwrap_or(0),
            params: params.clone(),
        }
    }

    /// Export to parameters.
    pub fn to_params(&self) -> ParameterCollection {
        let mut params = self.params.clone();

        params.add_int("OBJECTID", 14);
        params.add("LAYER", &self.layer.to_string());
        params.add_coord("X", self.x);
        params.add_coord("Y", self.y);
        params.add_double("ANGLE", self.angle, 14);

        params.add_coord("TEXTHEIGHT", self.text_height);
        params.add_coord("TEXTWIDTH", self.text_width);
        params.add_bool("USETTFONTS", self.use_tt_fonts);
        params.add_bool("BOLD", self.bold);
        params.add_bool("ITALIC", self.italic);
        if !self.font_name.is_empty() {
            params.add("FONTNAME", &self.font_name);
        }

        params.add_bool("SELECTION", self.selection);
        params.add_bool("LOCKED", self.locked);
        params.add_bool("POLYGONOUTLINE", self.polygon_outline);
        params.add_bool("USERROUTED", self.user_routed);
        params.add_bool("KEEPOUT", self.keepout);
        params.add_int("UNIONINDEX", self.union_index);
        params.add_bool("PRIMITIVELOCK", self.primitive_lock);
        params.add_bool("DRCERROR", self.drc_error);
        params.add_int("VINDEXFORSAVE", self.v_index_for_save);

        params
    }
}
