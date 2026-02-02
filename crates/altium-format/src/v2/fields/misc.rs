//! Miscellaneous record data structs.

use crate::v2::types::*;
use super::{DataObjectBase, GraphicalObjectBase, RectangularEntryContainerBase};
use super::schematic::{LabelData, TextFrameData, PowerData};
use super::parameter::ParameterData;
use super::sheet::SheetData;

/// ErrorMarker record data — from `ExportErrorMarker`/`ImportErrorMarker`.
#[derive(Clone, Debug, Default)]
pub struct ErrorMarkerData {
    pub graphical: GraphicalObjectBase,
    pub location_x: i32,
    pub location_y: i32,
    pub color: u32,
}

/// ClipBoard record data — from `ExportClipBoard`/`ImportClipBoard`.
#[derive(Clone, Debug, Default)]
pub struct ClipBoardData {
    pub graphical: GraphicalObjectBase,
    pub location_x: i32,
    pub location_y: i32,
}

/// RTFLink record data — from `ExportRTFLink`/`ImportRTFLink`.
#[derive(Clone, Debug, Default)]
pub struct RTFLinkData {
    pub container: RectangularEntryContainerBase,
    pub file_name_rtf: String,
    pub collapsed: bool,
}

/// RichTextDocument record data — from `ExportRichTextDocument`/`ImportRichTextDocument`.
#[derive(Clone, Debug, Default)]
pub struct RichTextDocumentData {
    pub graphical: GraphicalObjectBase,
    pub location_x: i32,
    pub location_y: i32,
    pub corner_x: i32,
    pub corner_y: i32,
    pub line_width: Size,
    pub color: u32,
    pub area_color: u32,
    pub is_solid: bool,
    pub show_border: bool,
    pub rtf_stream: Vec<u8>,
}

/// CompileMask record data — from `ExportCompileMask`/`ImportCompileMask`.
#[derive(Clone, Debug, Default)]
pub struct CompileMaskData {
    pub graphical: GraphicalObjectBase,
    pub unique_id: String,
    pub location_x: i32,
    pub location_y: i32,
    pub corner_x: i32,
    pub corner_y: i32,
    pub color: u32,
    pub area_color: u32,
    pub collapsed: bool,
    pub line_width: Size,
}

/// Blanket record data — from `ExportBlanket`/`ImportBlanket`.
#[derive(Clone, Debug, Default)]
pub struct BlanketData {
    pub graphical: GraphicalObjectBase,
    pub location_x: i32,
    pub location_y: i32,
    pub corner_x: i32,
    pub corner_y: i32,
    pub line_width: Size,
    pub color: u32,
    pub area_color: u32,
    pub collapsed: bool,
    pub line_style: LineStyle,
    pub vertices: Vec<(i32, i32)>,
    pub unique_id: String,
}

/// LineView record data — from `ExportLineView`.
#[derive(Clone, Debug, Default)]
pub struct LineViewData {
    pub graphical: GraphicalObjectBase,
    /// Vec of (x1, y1, x2, y2) coordinate rects.
    pub locations: Vec<(i32, i32, i32, i32)>,
    pub orientation: RotationBy90,
}

/// ObjectDefinition — from `ExportObjectDefinition`.
#[derive(Clone, Debug, Default)]
pub struct ObjectDefinitionData {
    pub object_definition_id: String,
    pub object_definition_hash: String,
    pub database_table_name: String,
    pub design_item_id: String,
    pub item_guid: String,
    pub library_path: String,
    pub lib_reference: String,
    pub revision_guid: String,
    pub source_library_name: String,
    pub target_file_name: String,
    pub not_use_db_table_name: bool,
    pub not_use_library_name: bool,
    pub vault_guid: String,
    pub base: DataObjectBase,
}

/// AssociatedObjects — DataObject base + type byte.
#[derive(Clone, Debug, Default)]
pub struct AssociatedObjectsData {
    pub base: DataObjectBase,
    pub associated_object_type: u8,
}

// ============================================================================
// Type aliases for delegate record types
// ============================================================================

/// Hyperlink — delegates to Label.
pub type HyperlinkData = LabelData;

/// CrossSheetConnector — delegates to Power.
pub type CrossSheetConnectorData = PowerData;

/// TaskHolder — empty graphical object.
pub type TaskHolderData = GraphicalObjectBase;

/// ImageParameter — delegates to Parameter.
pub type ImageParameterData = ParameterData;

/// FunctionalTextFrame — delegates to TextFrame.
pub type FunctionalTextFrameData = TextFrameData;

/// ElectronicsSystemDesignDocument — delegates to Sheet.
pub type ElectronicsSystemDesignDocumentData = SheetData;
