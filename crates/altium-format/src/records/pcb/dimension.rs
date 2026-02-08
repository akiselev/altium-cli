//! PCB dimension annotation record type.
//!
//! Dimensions are measurement annotations placed on PCB documents.
//! They can show linear, angular, radial, or other measurements
//! between points or objects on the board.

use crate::records::pcb::DimensionKind;
use crate::types::{Coord, Layer, ParameterCollection};

/// Arrow position for dimension lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ArrowPosition {
    /// Arrows inside dimension lines.
    #[default]
    Inside,
    /// Arrows outside dimension lines.
    Outside,
}

impl ArrowPosition {
    /// Parse from string value.
    pub fn parse(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "OUTSIDE" => ArrowPosition::Outside,
            _ => ArrowPosition::Inside,
        }
    }

    /// Convert to string value.
    pub fn as_str(&self) -> &'static str {
        match self {
            ArrowPosition::Inside => "Inside",
            ArrowPosition::Outside => "Outside",
        }
    }
}

/// Text position for dimension annotations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextPosition {
    /// Automatic text position.
    #[default]
    Auto,
    /// Manual text position.
    Manual,
}

impl TextPosition {
    /// Parse from string value.
    pub fn parse(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "MANUAL" => TextPosition::Manual,
            _ => TextPosition::Auto,
        }
    }

    /// Convert to string value.
    pub fn as_str(&self) -> &'static str {
        match self {
            TextPosition::Auto => "Auto",
            TextPosition::Manual => "Manual",
        }
    }
}

/// A reference point for a dimension annotation.
///
/// Dimensions reference specific objects and anchor points on the board.
#[derive(Debug, Clone, Default)]
pub struct DimensionReference {
    /// Index of the primitive being referenced.
    pub prim: i32,
    /// Object ID of the referenced object.
    pub object_id: i32,
    /// Name/description of the referenced object (e.g., "BoardOutline").
    pub object_string: String,
    /// X coordinate of the reference point.
    pub point_x: Coord,
    /// Y coordinate of the reference point.
    pub point_y: Coord,
    /// Anchor point index on the referenced object.
    pub anchor: i32,
}

/// PCB dimension annotation record.
///
/// Dimensions show measurements on the PCB. They consist of dimension lines,
/// extension lines, arrows, and text labels. They can reference board objects
/// like board outline edges, pads, or specific coordinates.
///
/// In PcbDoc files, dimensions are stored in the `Dimensions6/Data` stream
/// with a 2-byte header `[u8 version][u8 flags]` before each parameter block.
#[derive(Debug, Clone, Default)]
pub struct PcbDimension {
    /// Kind of dimension (Linear, Angular, Radial, Leader, etc.).
    pub dimension_kind: DimensionKind,
    /// Layer the dimension is on.
    pub layer: Layer,
    /// Layer name for the dimension.
    pub dimension_layer: String,
    /// Whether the dimension is locked for editing.
    pub dimension_locked: bool,

    // Geometry
    /// First reference X coordinate.
    pub x1: Coord,
    /// First reference Y coordinate.
    pub y1: Coord,
    /// Second reference X coordinate.
    pub x2: Coord,
    /// Second reference Y coordinate.
    pub y2: Coord,
    /// Low extent X.
    pub lx: Coord,
    /// Low extent Y.
    pub ly: Coord,
    /// High extent X.
    pub hx: Coord,
    /// High extent Y.
    pub hy: Coord,
    /// Height/offset of dimension line from measured feature.
    pub height: Coord,
    /// Overall angle of the dimension in degrees.
    pub angle: f64,

    // Line properties
    /// Width of the dimension line.
    pub line_width: Coord,
    /// Style of the dimension (e.g., "None").
    pub style: String,
    /// Font for dimension text (e.g., "DEFAULT").
    pub font: String,

    // Text properties
    /// Text anchor X coordinate.
    pub text_x: Coord,
    /// Text anchor Y coordinate.
    pub text_y: Coord,
    /// Height of the text.
    pub text_height: Coord,
    /// Width/stroke width of the text.
    pub text_width: Coord,
    /// Line width for text stroke.
    pub text_line_width: Coord,
    /// Position mode for the text (Auto or Manual).
    pub text_position: TextPosition,
    /// Gap between text and dimension line.
    pub text_gap: Coord,
    /// Text format string (e.g., "10mil").
    pub text_format: String,
    /// Dimension unit for display (e.g., "Mils", "Millimeters").
    pub text_dimension_unit: String,
    /// Number of decimal places.
    pub text_precision: i32,
    /// Prefix string for dimension text.
    pub text_prefix: String,
    /// Suffix string for dimension text.
    pub text_suffix: String,

    // Secondary text position (for the actual rendered text)
    /// Rendered text X coordinate.
    pub text1_x: Coord,
    /// Rendered text Y coordinate.
    pub text1_y: Coord,
    /// Rendered text angle.
    pub text1_angle: f64,
    /// Whether rendered text is mirrored.
    pub text1_mirror: bool,

    // Font properties
    /// Whether to use TrueType fonts.
    pub use_tt_fonts: bool,
    /// Whether text is bold.
    pub bold: bool,
    /// Whether text is italic.
    pub italic: bool,
    /// TrueType font name (e.g., "Arial").
    pub font_name: String,

    // Arrow properties
    /// Size of arrows.
    pub arrow_size: Coord,
    /// Line width of arrows.
    pub arrow_line_width: Coord,
    /// Length of arrow heads.
    pub arrow_length: Coord,
    /// Position of arrows (Inside or Outside).
    pub arrow_position: ArrowPosition,

    // Extension line properties
    /// Offset of extension lines from reference points.
    pub extension_offset: Coord,
    /// Width of extension lines.
    pub extension_line_width: Coord,
    /// Gap between reference point and start of extension line.
    pub extension_pick_gap: Coord,

    // Reference points
    /// Reference objects that the dimension measures between.
    pub references: Vec<DimensionReference>,

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

impl PcbDimension {
    /// Parse a dimension from parameters.
    pub fn from_params(params: &ParameterCollection) -> Self {
        let references_count = params
            .get("REFERENCES_COUNT")
            .map(|v| v.as_int_or(0))
            .unwrap_or(0);

        let mut references = Vec::new();
        for i in 0..references_count {
            let reference = DimensionReference {
                prim: params
                    .get(&format!("REFERENCE{}PRIM", i))
                    .map(|v| v.as_int_or(0))
                    .unwrap_or(0),
                object_id: params
                    .get(&format!("REFERENCE{}OBJECTID", i))
                    .map(|v| v.as_int_or(0))
                    .unwrap_or(0),
                object_string: params
                    .get(&format!("REFERENCE{}OBJECTSTRING", i))
                    .map(|v| v.as_str().to_string())
                    .unwrap_or_default(),
                point_x: params
                    .get(&format!("REFERENCE{}POINTX", i))
                    .and_then(|v| v.as_coord().ok())
                    .unwrap_or_default(),
                point_y: params
                    .get(&format!("REFERENCE{}POINTY", i))
                    .and_then(|v| v.as_coord().ok())
                    .unwrap_or_default(),
                anchor: params
                    .get(&format!("REFERENCE{}ANCHOR", i))
                    .map(|v| v.as_int_or(0))
                    .unwrap_or(0),
            };
            references.push(reference);
        }

        PcbDimension {
            dimension_kind: params
                .get("DIMENSIONKIND")
                .map(|v| DimensionKind::from_byte(v.as_int_or(0) as u8))
                .unwrap_or_default(),
            layer: params
                .get("LAYER")
                .map(|v| v.as_layer())
                .unwrap_or_default(),
            dimension_layer: params
                .get("DIMENSIONLAYER")
                .map(|v| v.as_str().to_string())
                .unwrap_or_default(),
            dimension_locked: params
                .get("DIMENSIONLOCKED")
                .map(|v| v.as_bool_or(false))
                .unwrap_or(false),
            x1: params
                .get("X1")
                .and_then(|v| v.as_coord().ok())
                .unwrap_or_default(),
            y1: params
                .get("Y1")
                .and_then(|v| v.as_coord().ok())
                .unwrap_or_default(),
            x2: params
                .get("X2")
                .and_then(|v| v.as_coord().ok())
                .unwrap_or_default(),
            y2: params
                .get("Y2")
                .and_then(|v| v.as_coord().ok())
                .unwrap_or_default(),
            lx: params
                .get("LX")
                .and_then(|v| v.as_coord().ok())
                .unwrap_or_default(),
            ly: params
                .get("LY")
                .and_then(|v| v.as_coord().ok())
                .unwrap_or_default(),
            hx: params
                .get("HX")
                .and_then(|v| v.as_coord().ok())
                .unwrap_or_default(),
            hy: params
                .get("HY")
                .and_then(|v| v.as_coord().ok())
                .unwrap_or_default(),
            height: params
                .get("HEIGHT")
                .and_then(|v| v.as_coord().ok())
                .unwrap_or_default(),
            angle: params
                .get("ANGLE")
                .map(|v| v.as_double_or(0.0))
                .unwrap_or(0.0),
            line_width: params
                .get("LINEWIDTH")
                .and_then(|v| v.as_coord().ok())
                .unwrap_or_default(),
            style: params
                .get("STYLE")
                .map(|v| v.as_str().to_string())
                .unwrap_or_default(),
            font: params
                .get("FONT")
                .map(|v| v.as_str().to_string())
                .unwrap_or_default(),
            text_x: params
                .get("TEXTX")
                .and_then(|v| v.as_coord().ok())
                .unwrap_or_default(),
            text_y: params
                .get("TEXTY")
                .and_then(|v| v.as_coord().ok())
                .unwrap_or_default(),
            text_height: params
                .get("TEXTHEIGHT")
                .and_then(|v| v.as_coord().ok())
                .unwrap_or_default(),
            text_width: params
                .get("TEXTWIDTH")
                .and_then(|v| v.as_coord().ok())
                .unwrap_or_default(),
            text_line_width: params
                .get("TEXTLINEWIDTH")
                .and_then(|v| v.as_coord().ok())
                .unwrap_or_default(),
            text_position: params
                .get("TEXTPOSITION")
                .map(|v| TextPosition::parse(v.as_str()))
                .unwrap_or_default(),
            text_gap: params
                .get("TEXTGAP")
                .and_then(|v| v.as_coord().ok())
                .unwrap_or_default(),
            text_format: params
                .get("TEXTFORMAT")
                .map(|v| v.as_str().to_string())
                .unwrap_or_default(),
            text_dimension_unit: params
                .get("TEXTDIMENSIONUNIT")
                .map(|v| v.as_str().to_string())
                .unwrap_or_default(),
            text_precision: params
                .get("TEXTPRECISION")
                .map(|v| v.as_int_or(2))
                .unwrap_or(2),
            text_prefix: params
                .get("TEXTPREFIX")
                .map(|v| v.as_str().to_string())
                .unwrap_or_default(),
            text_suffix: params
                .get("TEXTSUFFIX")
                .map(|v| v.as_str().to_string())
                .unwrap_or_default(),
            text1_x: params
                .get("TEXT1X")
                .and_then(|v| v.as_coord().ok())
                .unwrap_or_default(),
            text1_y: params
                .get("TEXT1Y")
                .and_then(|v| v.as_coord().ok())
                .unwrap_or_default(),
            text1_angle: params
                .get("TEXT1ANGLE")
                .map(|v| v.as_double_or(0.0))
                .unwrap_or(0.0),
            text1_mirror: params
                .get("TEXT1MIRROR")
                .map(|v| v.as_bool_or(false))
                .unwrap_or(false),
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
            arrow_size: params
                .get("ARROWSIZE")
                .and_then(|v| v.as_coord().ok())
                .unwrap_or_default(),
            arrow_line_width: params
                .get("ARROWLINEWIDTH")
                .and_then(|v| v.as_coord().ok())
                .unwrap_or_default(),
            arrow_length: params
                .get("ARROWLENGTH")
                .and_then(|v| v.as_coord().ok())
                .unwrap_or_default(),
            arrow_position: params
                .get("ARROWPOSITION")
                .map(|v| ArrowPosition::parse(v.as_str()))
                .unwrap_or_default(),
            extension_offset: params
                .get("EXTENSIONOFFSET")
                .and_then(|v| v.as_coord().ok())
                .unwrap_or_default(),
            extension_line_width: params
                .get("EXTENSIONLINEWIDTH")
                .and_then(|v| v.as_coord().ok())
                .unwrap_or_default(),
            extension_pick_gap: params
                .get("EXTENSIONPICKGAP")
                .and_then(|v| v.as_coord().ok())
                .unwrap_or_default(),
            references,
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

        params.add_int("OBJECTID", 13);
        params.add_int("DIMENSIONKIND", self.dimension_kind.to_byte() as i32);
        params.add("LAYER", &self.layer.to_string());
        if !self.dimension_layer.is_empty() {
            params.add("DIMENSIONLAYER", &self.dimension_layer);
        }
        params.add_bool("DIMENSIONLOCKED", self.dimension_locked);

        params.add_coord("X1", self.x1);
        params.add_coord("Y1", self.y1);
        params.add_coord("X2", self.x2);
        params.add_coord("Y2", self.y2);
        params.add_coord("LX", self.lx);
        params.add_coord("LY", self.ly);
        params.add_coord("HX", self.hx);
        params.add_coord("HY", self.hy);
        params.add_coord("HEIGHT", self.height);
        params.add_double("ANGLE", self.angle, 14);

        params.add_coord("LINEWIDTH", self.line_width);
        if !self.style.is_empty() {
            params.add("STYLE", &self.style);
        }
        if !self.font.is_empty() {
            params.add("FONT", &self.font);
        }

        params.add_coord("TEXTX", self.text_x);
        params.add_coord("TEXTY", self.text_y);
        params.add_coord("TEXTHEIGHT", self.text_height);
        params.add_coord("TEXTWIDTH", self.text_width);
        params.add_coord("TEXTLINEWIDTH", self.text_line_width);
        params.add("TEXTPOSITION", self.text_position.as_str());
        params.add_coord("TEXTGAP", self.text_gap);
        if !self.text_format.is_empty() {
            params.add("TEXTFORMAT", &self.text_format);
        }
        if !self.text_dimension_unit.is_empty() {
            params.add("TEXTDIMENSIONUNIT", &self.text_dimension_unit);
        }
        params.add_int("TEXTPRECISION", self.text_precision);
        params.add("TEXTPREFIX", &self.text_prefix);
        params.add("TEXTSUFFIX", &self.text_suffix);

        params.add_coord("TEXT1X", self.text1_x);
        params.add_coord("TEXT1Y", self.text1_y);
        params.add_double("TEXT1ANGLE", self.text1_angle, 14);
        params.add_bool("TEXT1MIRROR", self.text1_mirror);

        params.add_bool("USETTFONTS", self.use_tt_fonts);
        params.add_bool("BOLD", self.bold);
        params.add_bool("ITALIC", self.italic);
        if !self.font_name.is_empty() {
            params.add("FONTNAME", &self.font_name);
        }

        params.add_coord("ARROWSIZE", self.arrow_size);
        params.add_coord("ARROWLINEWIDTH", self.arrow_line_width);
        params.add_coord("ARROWLENGTH", self.arrow_length);
        params.add("ARROWPOSITION", self.arrow_position.as_str());

        params.add_coord("EXTENSIONOFFSET", self.extension_offset);
        params.add_coord("EXTENSIONLINEWIDTH", self.extension_line_width);
        params.add_coord("EXTENSIONPICKGAP", self.extension_pick_gap);

        // Write references
        params.add_int("REFERENCES_COUNT", self.references.len() as i32);
        for (i, r) in self.references.iter().enumerate() {
            params.add_int(&format!("REFERENCE{}PRIM", i), r.prim);
            params.add_int(&format!("REFERENCE{}OBJECTID", i), r.object_id);
            params.add(&format!("REFERENCE{}OBJECTSTRING", i), &r.object_string);
            params.add_coord(&format!("REFERENCE{}POINTX", i), r.point_x);
            params.add_coord(&format!("REFERENCE{}POINTY", i), r.point_y);
            params.add_int(&format!("REFERENCE{}ANCHOR", i), r.anchor);
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

    /// Get the number of reference points.
    pub fn reference_count(&self) -> usize {
        self.references.len()
    }
}
