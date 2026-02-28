//! Public API types for SchLib documents.
//!
//! These types provide a clean, domain-typed interface for querying and mutating
//! schematic library components. They abstract away internal format details like
//! `SchRecord`, `owner_index` linking, sidecar streams, and CFB structure.

use altium_format_types::color::Color;
use altium_format_types::common::{ComponentKind, RotationBy90};
use altium_format_types::coord::{Coord, CoordPoint};
use altium_format_types::sch::{
    HorizontalAlign, IeeeSymbol, LineShape, LineStyle, ParameterReadOnlyState,
    ParameterType, PenWidth, PinElectricalType, StdLogicState, TextJustification,
};

use crate::param_value::SchAngle;

// ── Component ────────────────────────────────────────────────────────────────

/// A schematic library component.
///
/// Identified by `lib_reference` (natural key, unique within a SchLib).
/// Contains all child records: pins, parameters, footprint maps, and graphics.
#[derive(Debug, Clone)]
pub struct Component {
    pub lib_reference: String,
    /// Designator prefix (e.g. "U?", "R?"). Extracted from the RECORD=34 Designator record.
    pub designator: Option<String>,
    pub description: Option<String>,
    pub component_kind: Option<ComponentKind>,
    pub part_count: i32,
    pub show_hidden_pins: bool,

    pub pins: Vec<Pin>,
    pub parameters: Vec<Parameter>,
    pub footprints: Vec<FootprintMap>,
    pub graphics: Vec<Graphic>,
    pub aliases: Vec<String>,
}

// ── Pin ──────────────────────────────────────────────────────────────────────

/// A schematic pin.
///
/// Identified by `designator` (natural key, unique within a Component).
/// All sidecar fields (from PinTextData, PinWideText, etc.) are merged in.
#[derive(Debug, Clone)]
pub struct Pin {
    pub designator: String,
    pub name: String,
    pub electrical: PinElectricalType,
    pub location: CoordPoint,
    pub length: Coord,
    pub orientation: RotationBy90,
    pub is_hidden: bool,
    pub hidden_net_name: String,
    pub owner_part_id: i32,

    // Display control
    pub show_name: bool,
    pub show_designator: bool,

    // IEEE symbols
    pub symbol_inner_edge: IeeeSymbol,
    pub symbol_outer_edge: IeeeSymbol,
    pub symbol_inside: IeeeSymbol,
    pub symbol_outside: IeeeSymbol,

    // Sidecar fields
    pub swap_id_pin: String,
    pub swap_id_part: String,
    pub swap_id_pair: String,
    pub default_value: String,
    pub pin_package_length: String,
    pub propagation_delay: String,
    pub pin_symbol_line_width: Option<i32>,
    pub name_text_data: Option<PinTextPositioning>,
    pub designator_text_data: Option<PinTextPositioning>,

    // Misc
    pub description: String,
    pub formal_type: StdLogicState,
    pub spice_pin_name: String,
    pub unique_id: String,
    pub color: Color,
    pub is_not_accessible: bool,
    pub graphically_locked: bool,
    pub owner_part_display_mode: u8,
}

/// Text positioning override data for a pin's name or designator label.
#[derive(Debug, Clone)]
pub struct PinTextPositioning {
    pub position_mode_custom: bool,
    pub rotation_anchor_component: bool,
    pub rotation_relative: RotationBy90,
    pub font_mode_custom: bool,
    pub custom_position_margin: Option<Coord>,
    pub custom_font_id: Option<i16>,
    pub custom_color: Option<Color>,
}

// ── Parameter ────────────────────────────────────────────────────────────────

/// A schematic parameter (RECORD=41).
///
/// Identified by `name` (natural key, unique within a Component).
/// The special "Comment" parameter is included in the parameters list.
/// The "Designator" parameter (RECORD=34) is extracted to `Component.designator`.
#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub text: String,
    pub is_hidden: bool,
    pub read_only: ParameterReadOnlyState,
    pub location: CoordPoint,
    pub orientation: RotationBy90,
    pub color: Color,
    pub font_id: i32,

    pub justification: TextJustification,
    pub is_mirrored: bool,
    pub show_name: bool,
    pub unique_id: String,
    pub not_auto_position: bool,
    pub param_type: ParameterType,
    pub description: String,
}

// ── FootprintMap ─────────────────────────────────────────────────────────────

/// A footprint implementation mapping (from RECORD=44→45→46→47 chain).
///
/// Identified by `model_name` (natural key, unique within a Component).
/// Only PCBLIB-type implementations are exposed.
#[derive(Debug, Clone)]
pub struct FootprintMap {
    pub model_name: String,
    pub description: String,
    pub is_current: bool,
    pub pin_pad_maps: Vec<PinPadMap>,
}

/// A single pin-to-pad mapping within a footprint implementation.
#[derive(Debug, Clone)]
pub struct PinPadMap {
    pub pin: String,
    pub pad: String,
}

// ── Graphic ──────────────────────────────────────────────────────────────────

/// A graphical primitive within a component.
///
/// Each variant corresponds to a schematic record type and carries all fields
/// from that record (minus format-internal fields like `owner_index`).
///
/// Natural key: `unique_id` (accessed via `Graphic::unique_id()`).
/// Note: `PieGraphic` has no unique_id in the Altium format.
#[derive(Debug, Clone)]
pub enum Graphic {
    Line(LineGraphic),
    Rectangle(RectangleGraphic),
    RoundRectangle(RoundRectangleGraphic),
    Arc(ArcGraphic),
    EllipticalArc(EllipticalArcGraphic),
    Ellipse(EllipseGraphic),
    Pie(PieGraphic),
    Polyline(PolylineGraphic),
    Polygon(PolygonGraphic),
    Bezier(BezierGraphic),
    Image(ImageGraphic),
    Label(LabelGraphic),
    TextFrame(TextFrameGraphic),
}

impl Graphic {
    /// Returns the unique ID, if the variant has one.
    ///
    /// `PieGraphic` has no unique_id in the Altium format, so returns `None`.
    pub fn unique_id(&self) -> Option<&str> {
        match self {
            Graphic::Line(g) => Some(&g.unique_id),
            Graphic::Rectangle(g) => Some(&g.unique_id),
            Graphic::RoundRectangle(g) => Some(&g.unique_id),
            Graphic::Arc(g) => Some(&g.unique_id),
            Graphic::EllipticalArc(g) => Some(&g.unique_id),
            Graphic::Ellipse(g) => Some(&g.unique_id),
            Graphic::Pie(_) => None,
            Graphic::Polyline(g) => Some(&g.unique_id),
            Graphic::Polygon(g) => Some(&g.unique_id),
            Graphic::Bezier(g) => Some(&g.unique_id),
            Graphic::Image(g) => Some(&g.unique_id),
            Graphic::Label(g) => Some(&g.unique_id),
            Graphic::TextFrame(g) => Some(&g.unique_id),
        }
    }

    /// Returns the owner part ID for this graphic.
    pub fn owner_part_id(&self) -> i32 {
        match self {
            Graphic::Line(g) => g.owner_part_id,
            Graphic::Rectangle(g) => g.owner_part_id,
            Graphic::RoundRectangle(g) => g.owner_part_id,
            Graphic::Arc(g) => g.owner_part_id,
            Graphic::EllipticalArc(g) => g.owner_part_id,
            Graphic::Ellipse(g) => g.owner_part_id,
            Graphic::Pie(g) => g.owner_part_id,
            Graphic::Polyline(g) => g.owner_part_id,
            Graphic::Polygon(g) => g.owner_part_id,
            Graphic::Bezier(g) => g.owner_part_id,
            Graphic::Image(g) => g.owner_part_id,
            Graphic::Label(g) => g.owner_part_id,
            Graphic::TextFrame(g) => g.owner_part_id,
        }
    }
}

// ── Graphic Variant Structs ──────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct LineGraphic {
    pub unique_id: String,
    pub owner_part_id: i32,
    pub location: CoordPoint,
    pub corner: CoordPoint,
    pub line_width: PenWidth,
    pub line_style: LineStyle,
    pub color: Color,
}

#[derive(Debug, Clone)]
pub struct RectangleGraphic {
    pub unique_id: String,
    pub owner_part_id: i32,
    pub location: CoordPoint,
    pub corner: CoordPoint,
    pub line_width: PenWidth,
    pub line_style: LineStyle,
    pub color: Color,
    pub area_color: Color,
    pub is_solid: bool,
    pub transparent: bool,
}

#[derive(Debug, Clone)]
pub struct RoundRectangleGraphic {
    pub unique_id: String,
    pub owner_part_id: i32,
    pub location: CoordPoint,
    pub corner: CoordPoint,
    pub corner_x_radius: Coord,
    pub corner_y_radius: Coord,
    pub line_width: PenWidth,
    pub color: Color,
    pub area_color: Color,
    pub is_solid: bool,
}

#[derive(Debug, Clone)]
pub struct ArcGraphic {
    pub unique_id: String,
    pub owner_part_id: i32,
    pub location: CoordPoint,
    pub radius: Coord,
    pub start_angle: SchAngle,
    pub end_angle: Option<SchAngle>,
    pub line_width: PenWidth,
    pub color: Color,
}

#[derive(Debug, Clone)]
pub struct EllipticalArcGraphic {
    pub unique_id: String,
    pub owner_part_id: i32,
    pub location: CoordPoint,
    pub radius: Coord,
    pub secondary_radius: Coord,
    pub start_angle: SchAngle,
    pub end_angle: Option<SchAngle>,
    pub line_width: PenWidth,
    pub color: Color,
}

#[derive(Debug, Clone)]
pub struct EllipseGraphic {
    pub unique_id: String,
    pub owner_part_id: i32,
    pub location: CoordPoint,
    pub radius: Coord,
    pub secondary_radius: Coord,
    pub line_width: PenWidth,
    pub color: Color,
    pub area_color: Color,
    pub is_solid: bool,
    pub transparent: bool,
}

#[derive(Debug, Clone)]
pub struct PieGraphic {
    pub owner_part_id: i32,
    pub location: CoordPoint,
    pub radius: Coord,
    pub start_angle: SchAngle,
    pub end_angle: Option<SchAngle>,
    pub line_width: PenWidth,
    pub color: Color,
    pub area_color: Color,
    pub is_solid: bool,
}

#[derive(Debug, Clone)]
pub struct PolylineGraphic {
    pub unique_id: String,
    pub owner_part_id: i32,
    pub vertices: Vec<CoordPoint>,
    pub line_width: PenWidth,
    pub line_style: LineStyle,
    pub start_line_shape: LineShape,
    pub end_line_shape: LineShape,
    pub line_shape_size: PenWidth,
    pub color: Color,
}

#[derive(Debug, Clone)]
pub struct PolygonGraphic {
    pub unique_id: String,
    pub owner_part_id: i32,
    pub vertices: Vec<CoordPoint>,
    pub line_width: PenWidth,
    pub color: Color,
    pub area_color: Color,
    pub is_solid: bool,
    pub transparent: bool,
}

#[derive(Debug, Clone)]
pub struct BezierGraphic {
    pub unique_id: String,
    pub owner_part_id: i32,
    pub vertices: Vec<CoordPoint>,
    pub line_width: PenWidth,
    pub color: Color,
}

#[derive(Debug, Clone)]
pub struct ImageGraphic {
    pub unique_id: String,
    pub owner_part_id: i32,
    pub location: CoordPoint,
    pub corner: CoordPoint,
    pub orientation: RotationBy90,
    pub line_width: PenWidth,
    pub color: Color,
    pub is_solid: bool,
    pub keep_aspect: bool,
    pub embed_image: bool,
    pub file_name: String,
}

#[derive(Debug, Clone)]
pub struct LabelGraphic {
    pub unique_id: String,
    pub owner_part_id: i32,
    pub location: CoordPoint,
    pub orientation: RotationBy90,
    pub justification: TextJustification,
    pub color: Color,
    pub font_id: i32,
    pub text: String,
    pub is_mirrored: bool,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct TextFrameGraphic {
    pub unique_id: String,
    pub owner_part_id: i32,
    pub location: CoordPoint,
    pub corner: CoordPoint,
    pub line_width: PenWidth,
    pub color: Color,
    pub area_color: Color,
    pub text_color: Color,
    pub font_id: i32,
    pub is_solid: bool,
    pub show_border: bool,
    pub alignment: HorizontalAlign,
    pub word_wrap: bool,
    pub clip_to_rect: bool,
    pub text: String,
    pub text_margin: Coord,
    pub transparent: bool,
}
