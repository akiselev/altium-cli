use std::path::PathBuf;

use altium_format_types::{
    Color, ComponentKind, Coord, CoordPoint, PadShape, PadStackMode, PinElectricalType,
    PlaneConnectionStyle, RotationBy90, V6Layer,
};

// ── SchLib ──────────────────────────────────────────────────────────────────

pub struct SchLibSpec {
    pub components: Vec<ComponentSpec>,
}

pub struct ComponentSpec {
    pub lib_reference: String,
    pub designator: Option<String>,
    pub description: Option<String>,
    pub component_kind: Option<ComponentKind>,
    pub part_count: Option<i32>,
    pub show_hidden_pins: Option<bool>,

    pub pins: Vec<PinSpec>,
    pub parameters: Vec<ParameterSpec>,
    pub aliases: Vec<String>,
    pub footprints: Vec<FootprintMapSpec>,
    pub graphics: Vec<GraphicSpec>,

    pub parts: Vec<PartSpec>,
}

pub struct PartSpec {
    pub part_number: i32,
    pub pins: Vec<PinSpec>,
    pub graphics: Vec<GraphicSpec>,
}

pub struct PinSpec {
    pub designator: String,
    pub name: Option<String>,
    pub electrical: Option<PinElectricalType>,
    pub length: Option<Coord>,
    pub location: CoordPoint,
    pub orientation: RotationBy90,
    pub is_hidden: Option<bool>,
    pub hidden_net_name: Option<String>,
    pub owner_part_id: i32,
}

pub struct ParameterSpec {
    pub name: String,
    pub text: String,
    pub is_hidden: Option<bool>,
}

pub struct FootprintMapSpec {
    pub model_name: String,
    pub maps: Vec<PinPadMap>,
    pub source: Option<PathBuf>,
}

pub struct PinPadMap {
    pub pin: String,
    pub pad: String,
}

// ── PcbLib ──────────────────────────────────────────────────────────────────

pub struct PcbLibSpec {
    pub footprints: Vec<FootprintSpec>,
}

pub struct FootprintSpec {
    pub display_name: String,
    pub description: Option<String>,
    pub height: Option<Coord>,
    pub pattern: Option<String>,

    pub pads: Vec<PadSpec>,
    pub graphics: Vec<PcbGraphicSpec>,
}

pub struct PadSpec {
    pub pad_name: String,
    pub at: CoordPoint,
    pub shape: Option<PadShape>,
    pub x_size: Option<Coord>,
    pub y_size: Option<Coord>,
    pub rotation: Option<f64>,
    pub hole_size: Option<Coord>,
    pub is_plated: Option<bool>,
    pub layer: Option<V6Layer>,
    pub pad_mode: Option<PadStackMode>,
    pub solder_mask_expansion: Option<Coord>,
    pub paste_mask_expansion: Option<Coord>,
    pub plane_connection: Option<PlaneConnectionStyle>,
    pub relief_conductor_width: Option<Coord>,
    pub relief_entries: Option<i32>,
    pub relief_air_gap: Option<Coord>,
}

// ── Graphics (Schematic) ─────────────────────────────────────────────────────

pub struct GraphicSpec {
    pub unique_id: String,
    pub graphic_type: GraphicType,
    pub properties: GraphicProperties,
}

#[derive(Debug, Clone)]
pub enum GraphicType {
    Line,
    Rectangle,
    Arc,
    EllipticalArc,
    Ellipse,
    Polyline,
    Polygon,
    Bezier,
    Pie,
    RoundRectangle,
    Label,
    TextFrame,
    Image,
}

pub struct GraphicProperties {
    // Box types (rectangle, round_rectangle, text_frame, image)
    pub from: Option<CoordPoint>,
    pub to: Option<CoordPoint>,
    pub is_solid: Option<bool>,
    pub corner_x_radius: Option<Coord>,
    pub corner_y_radius: Option<Coord>,

    // Center+radius types (arc, ellipse, pie)
    pub center: Option<CoordPoint>,
    pub radius: Option<Coord>,
    pub secondary_radius: Option<Coord>,
    pub start_angle: Option<f64>,
    pub end_angle: Option<f64>,

    // Vertex-list types (polyline, polygon, bezier)
    pub points: Option<Vec<CoordPoint>>,

    // Common visual
    pub color: Option<Color>,
    pub area_color: Option<Color>,
    pub line_width: Option<Coord>,

    // Text (label, text_frame)
    pub text: Option<String>,
    pub font_id: Option<i32>,
    pub at: Option<CoordPoint>,

    // Image
    pub file_name: Option<String>,
    pub image_data: Option<Vec<u8>>,

    // PCB-specific
    pub layer: Option<V6Layer>,
    pub width: Option<Coord>,
    pub closed: Option<bool>,
    pub show_border: Option<bool>,
}

// ── Graphics (PCB) ───────────────────────────────────────────────────────────

pub struct PcbGraphicSpec {
    pub unique_id: String,
    pub graphic_type: PcbGraphicType,
    pub properties: PcbGraphicProperties,
}

pub enum PcbGraphicType {
    Track,
    Arc,
    Fill,
    Region,
    Text,
    Via,
    ComponentBody,
    Polyline,
}

pub struct PcbGraphicProperties {
    pub layer: Option<V6Layer>,
    pub width: Option<Coord>,
    pub from: Option<CoordPoint>,
    pub to: Option<CoordPoint>,
    pub center: Option<CoordPoint>,
    pub radius: Option<Coord>,
    pub start_angle: Option<f64>,
    pub end_angle: Option<f64>,
    pub points: Option<Vec<CoordPoint>>,
    pub text: Option<String>,
    pub at: Option<CoordPoint>,
    pub rotation: Option<f64>,
    pub hole_size: Option<Coord>,
    pub diameter: Option<Coord>,
    pub is_solid: Option<bool>,
}

// ── Domain / SpecModel ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecDomain {
    SchLib,
    PcbLib,
}

pub enum SpecModel {
    SchLib(SchLibSpec),
    PcbLib(PcbLibSpec),
}
