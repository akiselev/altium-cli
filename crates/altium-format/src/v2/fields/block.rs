//! Functional block and schematic block record data structs.

use crate::v2::types::*;
use super::GraphicalObjectBase;
use super::primitives::RectangleData;
use super::schematic::WireData;
use super::sheet::SheetSymbolData;

/// FunctionalBlock — Rectangle + text fields.
#[derive(Clone, Debug, Default)]
pub struct FunctionalBlockData {
    pub rect: RectangleData,
    pub font_id: i32,
    pub text_color: u32,
    pub name: String,
    pub file_name: String,
}

/// FunctionalConnectionLine — Wire + instance label.
#[derive(Clone, Debug, Default)]
pub struct FunctionalConnectionLineData {
    pub wire: WireData,
    pub instance_label: String,
    pub designator_locked: bool,
}

/// SchematicBlock — from `ExportSchematicBlock`.
#[derive(Clone, Debug, Default)]
pub struct SchematicBlockData {
    pub graphical: GraphicalObjectBase,
    pub name: String,
    pub location_x: i32,
    pub location_y: i32,
    pub corner_x: i32,
    pub corner_y: i32,
    pub line_style: LineStyle,
    pub line_width: Size,
    pub orientation: RotationBy90,
    pub color: u32,
    pub area_color: u32,
    pub is_solid: bool,
    pub transparent: bool,
    pub unique_id: String,
    pub designator_locked: bool,
    pub design_item_id: String,
    pub source_library_name: String,
    pub power_objects_name_mappings: Vec<(String, String)>,
    pub rb_server_parameters: Vec<String>,
    pub reuse_block_info: ReuseBlockImplementationInfoData,
}

/// ReuseBlockImplementationInfo — from `ExportReuseBlockImplementationInfo`.
#[derive(Clone, Debug, Default)]
pub struct ReuseBlockImplementationInfoData {
    pub name: String,
    pub description: String,
    pub design_item_id: String,
    pub block_server_name: String,
    pub block_vault_guid: String,
    pub block_item_guid: String,
    pub block_item_revision_guid: String,
    pub sch_snippet_vault_guid: String,
    pub sch_snippet_item_guid: String,
    pub sch_snippet_item_revision_guid: String,
    pub pcb_snippet_vault_guid: String,
    pub pcb_snippet_item_guid: String,
    pub pcb_snippet_item_revision_guid: String,
    pub reuse_block_id: String,
    pub is_dissolved: bool,
    pub reuse_block_objects_ids: String,
    pub docs_file_names_mappings: Vec<(String, String)>,
    pub parameters: Vec<(String, String)>,
}

/// ReuseSheetSymbol — SheetSymbol + reuse block info.
#[derive(Clone, Debug, Default)]
pub struct ReuseSheetSymbolData {
    pub sheet_symbol: SheetSymbolData,
    pub name: String,
    pub designator_locked: bool,
    pub power_objects_name_mappings: Vec<(String, String)>,
    pub rb_server_parameters: Vec<String>,
    pub reuse_block_info: ReuseBlockImplementationInfoData,
}
