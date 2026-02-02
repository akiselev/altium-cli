//! Sheet/document record data structs.

use crate::v2::types::*;
use super::{GraphicalObjectBase, RectangularEntryContainerBase, BasicEntryObjectBase};

/// Sheet record data — from `ExportSheet`/`ImportSheet` (via ExportDocument).
#[derive(Clone, Debug, Default)]
pub struct SheetData {
    // Font table (simplified — stored as raw count + font entries)
    pub font_id_count: i32,
    pub fonts: Vec<FontEntry>,
    // Document fields
    pub use_mbcs: bool,
    pub is_boc: bool,
    pub hot_spot_grid_on: bool,
    pub hot_spot_grid_size: i32,
    pub sheet_style: u8,
    pub system_font: i32,
    pub document_border_style: u8,
    pub workspace_orientation: u8,
    pub border_on: bool,
    pub title_block_on: bool,
    pub sheet_number_space_size: i32,
    pub color: u32,
    pub area_color: u32,
    pub snap_grid_on: bool,
    pub snap_grid_size: i32,
    pub visible_grid_on: bool,
    pub visible_grid_size: i32,
    pub custom_x: i32,
    pub custom_y: i32,
    pub use_custom_sheet: bool,
    pub show_hidden_pins: bool,
    pub reference_zones_on: bool,
    pub custom_x_zones: i32,
    pub custom_y_zones: i32,
    pub custom_margin_width: i32,
    pub show_template_graphics: bool,
    pub template_file_name: String,
    pub display_unit: u8,
    pub reference_zone_style: u8,
    pub always_show_cd: bool,
    // Vault/GUID fields
    pub release_vault_guid: String,
    pub release_item_guid: String,
    pub item_revision_guid: String,
    pub props_vault_guid: String,
    pub props_revision_guid: String,
    pub file_version_info: String,
    pub template_vault_guid: String,
    pub template_item_guid: String,
    pub template_revision_guid: String,
}

/// Library record data — from `ExportLibrary`/`ImportLibrary`.
///
/// Similar to SheetData but has a different field set:
/// - Missing: HotSpotGrid, SystemFont, CustomZones, CustomMarginWidth,
///   ShowTemplateGraphics, TemplateFileName, ReferenceZoneStyle,
///   ReleaseItemGUID, ItemRevisionGUID, PropsVaultGUID, PropsRevisionGUID, FileVersionInfo
/// - Extra: Description, FolderGUID, LifeCycleDefinitionGUID, RevisionNamingSchemeGUID
#[derive(Clone, Debug, Default)]
pub struct LibraryData {
    // Font table
    pub font_id_count: i32,
    pub fonts: Vec<FontEntry>,
    // Library fields
    pub use_mbcs: bool,
    pub is_boc: bool,
    pub description: String,
    pub document_border_style: u8,
    pub sheet_style: u8,
    pub workspace_orientation: u8,
    pub border_on: bool,
    pub title_block_on: bool,
    pub sheet_number_space_size: i32,
    pub color: u32,
    pub area_color: u32,
    pub snap_grid_on: bool,
    pub snap_grid_size: i32,
    pub visible_grid_on: bool,
    pub visible_grid_size: i32,
    pub custom_x: i32,
    pub custom_y: i32,
    pub use_custom_sheet: bool,
    pub show_hidden_pins: bool,
    pub reference_zones_on: bool,
    pub display_unit: u8,
    pub always_show_cd: bool,
    pub release_vault_guid: String,
    pub folder_guid: String,
    pub life_cycle_definition_guid: String,
    pub revision_naming_scheme_guid: String,
}

/// Font table entry for Sheet.
#[derive(Clone, Debug, Default)]
pub struct FontEntry {
    pub font_name: String,
    pub size: i32,
    pub rotation: i32,
    pub italic: bool,
    pub bold: bool,
    pub underline: bool,
    pub strike_out: bool,
}

/// SheetSymbol record data — from `ExportSheetSymbol`/`ImportSheetSymbol`.
#[derive(Clone, Debug, Default)]
pub struct SheetSymbolData {
    pub container: RectangularEntryContainerBase,
    pub is_solid: bool,
    pub show_hidden_fields: bool,
    pub unique_id: String,
    pub symbol_type: String,
    pub design_item_id: String,
    pub source_library_name: String,
    pub vault_guid: String,
    pub item_guid: String,
    pub revision_guid: String,
    pub revision_name: String,
}

/// SheetEntry record data — from `ExportSheetEntry`/`ImportSheetEntry`.
#[derive(Clone, Debug, Default)]
pub struct SheetEntryData {
    pub entry: BasicEntryObjectBase,
    pub io_type: PortIO,
    pub style: PortArrowStyle,
    pub arrow_kind: String,
}

/// SheetName record data — from `ExportSheetName`/`ImportSheetName`.
#[derive(Clone, Debug, Default)]
pub struct SheetNameData {
    pub graphical: GraphicalObjectBase,
    pub location_x: i32,
    pub location_y: i32,
    pub orientation: RotationBy90,
    pub justification: TextJustification,
    pub color: u32,
    pub font_id: i32,
    pub is_hidden: bool,
    pub text: String,
    pub is_mirrored: bool,
    pub auto_position: bool,
    pub text_horz_anchor: TextHorzAnchor,
    pub text_vert_anchor: TextVertAnchor,
    pub unique_id: String,
}

/// SheetFileName record data — from `ExportSheetFileName`/`ImportSheetFileName`.
#[derive(Clone, Debug, Default)]
pub struct SheetFileNameData {
    pub graphical: GraphicalObjectBase,
    pub location_x: i32,
    pub location_y: i32,
    pub orientation: RotationBy90,
    pub justification: TextJustification,
    pub color: u32,
    pub font_id: i32,
    pub is_hidden: bool,
    pub text: String,
    pub is_mirrored: bool,
    pub auto_position: bool,
    pub text_horz_anchor: TextHorzAnchor,
    pub text_vert_anchor: TextVertAnchor,
    pub unique_id: String,
}

/// Template record data — from `ExportTemplate`/`ImportTemplate`.
#[derive(Clone, Debug, Default)]
pub struct TemplateData {
    pub graphical: GraphicalObjectBase,
    pub file_name: String,
}

/// HarnessConnectorType record data — from `ExportHarnessConnectorType`/`ImportHarnessConnectorType`.
#[derive(Clone, Debug, Default)]
pub struct HarnessConnectorTypeData {
    pub graphical: GraphicalObjectBase,
    pub location_x: i32,
    pub location_y: i32,
    pub orientation: RotationBy90,
    pub justification: TextJustification,
    pub color: u32,
    pub font_id: i32,
    pub is_hidden: bool,
    pub text: String,
    pub is_mirrored: bool,
    pub auto_position: bool,
    pub text_horz_anchor: TextHorzAnchor,
    pub text_vert_anchor: TextVertAnchor,
    pub unique_id: String,
}
