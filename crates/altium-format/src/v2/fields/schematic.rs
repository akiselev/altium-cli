//! Schematic connectivity record data structs.

use crate::v2::types::*;
use super::GraphicalObjectBase;

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

/// Probe record data — from `ExportProbe`/`ImportProbe`.
#[derive(Clone, Debug, Default)]
pub struct ProbeData {
    pub graphical: GraphicalObjectBase,
    pub location_x: i32,
    pub location_y: i32,
    pub color: u32,
    pub orientation: RotationBy90,
    pub name: String,
    pub unique_id: String,
}

/// NoERC record data — from `ExportNoERC`/`ImportNoERC`.
#[derive(Clone, Debug, Default)]
pub struct NoERCData {
    pub graphical: GraphicalObjectBase,
    pub location_x: i32,
    pub location_y: i32,
    pub color: u32,
    pub orientation: RotationBy90,
    pub symbol: NoERCSymbol,
    pub is_active: bool,
    pub suppress_all: bool,
    pub unique_id: String,
}

/// Symbol record data — from `ExportSymbol`/`ImportSymbol`.
#[derive(Clone, Debug, Default)]
pub struct SymbolData {
    pub graphical: GraphicalObjectBase,
    pub symbol: IeeeSymbol,
    pub location_x: i32,
    pub location_y: i32,
    pub scale_factor: i32,
    pub orientation: RotationBy90,
    pub line_width: Size,
    pub color: u32,
    pub is_mirrored: bool,
}

/// Note record data — from `ExportNote`/`ImportNote`.
#[derive(Clone, Debug, Default)]
pub struct NoteData {
    pub graphical: GraphicalObjectBase,
    pub location_x: i32,
    pub location_y: i32,
    pub corner_x: i32,
    pub corner_y: i32,
    pub line_width: Size,
    pub color: u32,
    pub area_color: u32,
    pub text_color: u32,
    pub font_id: i32,
    pub is_solid: bool,
    pub show_border: bool,
    pub alignment: HorizontalAlign,
    pub word_wrap: bool,
    pub clip_to_rect: bool,
    pub text: String,
    pub text_margin: i32,
    pub collapsed: bool,
    pub author: String,
    pub unique_id: String,
}

/// TextFrame record data — from `ExportTextFrame`/`ImportTextFrame`.
#[derive(Clone, Debug, Default)]
pub struct TextFrameData {
    pub graphical: GraphicalObjectBase,
    pub location_x: i32,
    pub location_y: i32,
    pub corner_x: i32,
    pub corner_y: i32,
    pub line_width: Size,
    pub color: u32,
    pub area_color: u32,
    pub text_color: u32,
    pub font_id: i32,
    pub is_solid: bool,
    pub show_border: bool,
    pub alignment: HorizontalAlign,
    pub word_wrap: bool,
    pub clip_to_rect: bool,
    pub text: String,
    pub text_margin: i32,
    pub unique_id: String,
}

/// BusEntry record data — from `ExportBusEntry`/`ImportBusEntry`.
#[derive(Clone, Debug, Default)]
pub struct BusEntryData {
    pub graphical: GraphicalObjectBase,
    pub unique_id: String,
    pub location_x: i32,
    pub location_y: i32,
    pub corner_x: i32,
    pub corner_y: i32,
    pub line_width: Size,
    pub color: u32,
}

/// SignalHarness record data — from `ExportSignalHarness`/`ImportSignalHarness`.
#[derive(Clone, Debug, Default)]
pub struct SignalHarnessData {
    pub graphical: GraphicalObjectBase,
    pub line_width: Size,
    pub color: u32,
    pub underline_color: u32,
    pub vertices: Vec<(i32, i32)>,
    pub unique_id: String,
    pub assigned_interface: String,
    pub assigned_interface_signal: String,
}
