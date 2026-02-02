//! Per-record data structs and format functions.
//!
//! Each record type gets a struct and export/import function pair,
//! ported from `FileFormatV5.cs`.

pub mod component;
pub mod pin;

use crate::v2::types::*;

// ============================================================================
// Base object structs (shared across multiple record types)
// ============================================================================

/// Base data object fields — from `ExportDataObject`/`ImportDataObject`.
#[derive(Clone, Debug, Default)]
pub struct DataObjectBase {
    pub owner_index: i32,
    pub is_not_accessible: bool,
    pub owner_index_additional_list: bool,
    pub index_in_sheet: i32,
    pub ignore_on_load: bool,
    pub is_schematic_block_object: bool,
    pub unique_id_in_reuse_block: String,
}

/// Graphical object fields — from `ExportGraphicalObject`/`ImportGraphicalObject`.
///
/// Extends DataObjectBase.
#[derive(Clone, Debug, Default)]
pub struct GraphicalObjectBase {
    pub base: DataObjectBase,
    pub owner_part_id: i16,
    pub owner_part_display_mode: u8,
    pub selection_memory: u8,
    pub union_index: i32,
    pub graphically_locked: bool,
}

// ============================================================================
// Record data structs
// ============================================================================

/// Arc record data — from `ExportArc`/`ImportArc` (ObjectId::Arc = 12).
#[derive(Clone, Debug, Default)]
pub struct ArcData {
    pub graphical: GraphicalObjectBase,
    pub location_x: i32,
    pub location_y: i32,
    pub radius: i32,
    pub line_width: Size,
    pub start_angle: f64,
    pub end_angle: f64,
    pub color: u32,
    pub unique_id: String,
}

/// Line record data — from `ExportLine`/`ImportLine` (ObjectId::Line = 13).
#[derive(Clone, Debug, Default)]
pub struct LineData {
    pub graphical: GraphicalObjectBase,
    pub location_x: i32,
    pub location_y: i32,
    pub corner_x: i32,
    pub corner_y: i32,
    pub line_width: Size,
    pub line_style: LineStyle,
    pub color: u32,
    pub unique_id: String,
}

/// Rectangle record data — from `ExportRectangle`/`ImportRectangle` (ObjectId::Rectangle = 14).
#[derive(Clone, Debug, Default)]
pub struct RectangleData {
    pub graphical: GraphicalObjectBase,
    pub location_x: i32,
    pub location_y: i32,
    pub corner_x: i32,
    pub corner_y: i32,
    pub line_style: LineStyle,
    pub line_width: Size,
    pub color: u32,
    pub area_color: u32,
    pub is_solid: bool,
    pub transparent: bool,
    pub unique_id: String,
}

/// Ellipse record data — from `ExportEllipse`/`ImportEllipse` (ObjectId::Ellipse = 11).
#[derive(Clone, Debug, Default)]
pub struct EllipseData {
    pub graphical: GraphicalObjectBase,
    pub location_x: i32,
    pub location_y: i32,
    pub radius: i32,
    pub secondary_radius: i32,
    pub line_width: Size,
    pub color: u32,
    pub area_color: u32,
    pub is_solid: bool,
    pub transparent: bool,
    pub unique_id: String,
}

/// Polygon record data — from `ExportPolygon`/`ImportPolygon` (ObjectId::Polygon = 7).
#[derive(Clone, Debug, Default)]
pub struct PolygonData {
    pub graphical: GraphicalObjectBase,
    pub line_width: Size,
    pub color: u32,
    pub area_color: u32,
    pub is_solid: bool,
    pub transparent: bool,
    pub vertices: Vec<(i32, i32)>,
    pub unique_id: String,
}

/// Polyline record data — from `ExportPolyline`/`ImportPolyline` (ObjectId::Polyline = 6).
#[derive(Clone, Debug, Default)]
pub struct PolylineData {
    pub graphical: GraphicalObjectBase,
    pub line_width: Size,
    pub line_style: LineStyle,
    pub start_line_shape: LineShape,
    pub end_line_shape: LineShape,
    pub line_shape_size: Size,
    pub color: u32,
    pub vertices: Vec<(i32, i32)>,
    pub unique_id: String,
}

/// Bezier record data — from `ExportBezier`/`ImportBezier` (ObjectId::Bezier = 5).
#[derive(Clone, Debug, Default)]
pub struct BezierData {
    pub graphical: GraphicalObjectBase,
    pub line_width: Size,
    pub color: u32,
    pub vertices: Vec<(i32, i32)>,
    pub unique_id: String,
}

/// Junction record data — from `ExportJunction`/`ImportJunction` (ObjectId::Junction = 29).
#[derive(Clone, Debug, Default)]
pub struct JunctionData {
    pub graphical: GraphicalObjectBase,
    pub location_x: i32,
    pub location_y: i32,
    pub size: Size,
    pub color: u32,
    pub locked: bool,
    pub unique_id: String,
}

/// Label record data — from `ExportLabel`/`ImportLabel` (ObjectId::Label = 4).
#[derive(Clone, Debug, Default)]
pub struct LabelData {
    pub graphical: GraphicalObjectBase,
    pub location_x: i32,
    pub location_y: i32,
    pub orientation: RotationBy90,
    pub justification: TextJustification,
    pub color: u32,
    pub font_id: i32,
    pub text: String,
    pub is_mirrored: bool,
    pub url: String,
    pub unique_id: String,
}

/// Net label record data — from `ExportNetLabel`/`ImportNetLabel` (ObjectId::NetLabel = 25).
#[derive(Clone, Debug, Default)]
pub struct NetLabelData {
    pub graphical: GraphicalObjectBase,
    pub location_x: i32,
    pub location_y: i32,
    pub orientation: RotationBy90,
    pub justification: TextJustification,
    pub color: u32,
    pub font_id: i32,
    pub text: String,
    pub is_mirrored: bool,
    pub unique_id: String,
}

/// Wire record data — from `ExportWire`/`ImportWire` (ObjectId::Wire = 27).
#[derive(Clone, Debug, Default)]
pub struct WireData {
    pub graphical: GraphicalObjectBase,
    pub line_width: Size,
    pub color: u32,
    pub underline_color: u32,
    pub unique_id: String,
    pub assigned_interface: String,
    pub assigned_interface_signal: String,
    pub vertices: Vec<(i32, i32)>,
}

/// Bus record data — from `ExportBus`/`ImportBus` (ObjectId::Bus = 26).
#[derive(Clone, Debug, Default)]
pub struct BusData {
    pub graphical: GraphicalObjectBase,
    pub line_width: Size,
    pub color: u32,
    pub underline_color: u32,
    pub unique_id: String,
    pub assigned_interface: String,
    pub assigned_interface_signal: String,
    pub vertices: Vec<(i32, i32)>,
}

/// Port record data — from `ExportPort`/`ImportPort` (ObjectId::Port = 17).
#[derive(Clone, Debug, Default)]
pub struct PortData {
    pub graphical: GraphicalObjectBase,
    pub style: PortArrowStyle,
    pub io_type: PortIO,
    pub alignment: HorizontalAlign,
    pub width: i32,
    pub location_x: i32,
    pub location_y: i32,
    pub color: u32,
    pub font_id: i32,
    pub area_color: u32,
    pub text_color: u32,
    pub name: String,
    pub harness_type: String,
    pub unique_id: String,
    pub height: i32,
    pub border_width: Size,
    pub auto_size: bool,
    pub object_definition_id: String,
    pub show_net_name: bool,
}

/// Power record data — from `ExportPower`/`ImportPower` (ObjectId::PowerObject = 22).
#[derive(Clone, Debug, Default)]
pub struct PowerData {
    pub graphical: GraphicalObjectBase,
    pub style: PowerObjectStyle,
    pub show_net_name: bool,
    pub location_x: i32,
    pub location_y: i32,
    pub orientation: RotationBy90,
    pub color: u32,
    pub font_id: i32,
    pub text: String,
    pub is_cross_sheet_connector: bool,
    pub unique_id: String,
    pub object_definition_id: String,
}

/// Parameter record data — from `ExportParameter`/`ImportParameter` (ObjectId::Parameter = 41).
#[derive(Clone, Debug, Default)]
pub struct ParameterData {
    pub graphical: GraphicalObjectBase,
    pub location_x: i32,
    pub location_y: i32,
    pub orientation: RotationBy90,
    pub justification: TextJustification,
    pub color: u32,
    pub font_id: i32,
    pub is_hidden: bool,
    pub text: String,
    pub param_type: ParameterType,
    pub name: String,
    pub show_name: bool,
    pub read_only_state: ParameterReadOnlyState,
    pub unique_id: String,
    pub description: String,
    pub allow_library_synchronize: bool,
    pub allow_database_synchronize: bool,
    pub auto_position: bool,
    pub is_mirrored: bool,
    pub text_horz_anchor: TextHorzAnchor,
    pub text_vert_anchor: TextVertAnchor,
    pub is_image_parameter: bool,
}

/// Designator record data — from `ExportDesignator`/`ImportDesignator` (ObjectId::Designator = 34).
///
/// Extends ParameterData with auto-position override handling.
#[derive(Clone, Debug, Default)]
pub struct DesignatorData {
    pub param: ParameterData,
    pub override_not_auto_position: bool,
}

/// Image record data — from `ExportImage`/`ImportImage` (ObjectId::Image = 30).
#[derive(Clone, Debug, Default)]
pub struct ImageData {
    pub graphical: GraphicalObjectBase,
    pub location_x: i32,
    pub location_y: i32,
    pub corner_x: i32,
    pub corner_y: i32,
    pub orientation: RotationBy90,
    pub line_width: Size,
    pub color: u32,
    pub is_solid: bool,
    pub keep_aspect: bool,
    pub embed_image: bool,
    pub file_name: String,
    pub unique_id: String,
}
