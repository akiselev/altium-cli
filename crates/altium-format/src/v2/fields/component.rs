//! Component record data — ported from `SchDataComponent.cs` and `FileFormatV5.ExportComponent`.
//!
//! Component has 47+ fields.

use super::GraphicalObjectBase;
use crate::v2::types::*;

/// Component data matching C# `SchDataComponent` (47+ fields).
///
/// Fields are ordered to match the C# `ExportComponent` serialization order.
#[derive(Clone, Debug, Default)]
pub struct ComponentData {
    // --- Identification (before graphical base) ---
    pub lib_reference: String,
    pub component_description: String,
    pub part_count: i16,
    pub display_mode_count: u8,

    // --- Graphical base (ExportGraphicalObject -> ExportDataObject) ---
    pub graphical: GraphicalObjectBase,

    // --- Position ---
    pub location_x: i32,
    pub location_y: i32,

    // --- Component properties ---
    pub display_mode: u8,
    pub is_mirrored: bool,
    pub orientation: RotationBy90,
    pub current_part_id: i16,
    pub show_hidden_fields: bool,
    pub show_hidden_pins: bool,

    // --- Library references ---
    pub library_path: String,
    pub source_library_name: String,
    pub database_table_name: String,
    pub sheet_part_file_name: String,
    pub target_file_name: String,
    pub unique_id: String,

    // --- Colors ---
    pub area_color: u32,
    pub color: u32,
    pub pin_color: u32,
    pub overide_colors: bool,

    // --- Flags ---
    pub display_field_names: bool,
    pub designator_locked: bool,
    pub part_id_locked: bool,
    pub pins_moveable: bool,

    // --- Alias list ---
    pub alias_list: String,

    // --- Library name usage ---
    pub not_use_library_name: bool,
    pub not_use_db_table_name: bool,

    // --- Design item ---
    pub design_item_id: String,

    // --- Vault/GUID ---
    pub vault_guid: String,
    pub item_guid: String,
    pub revision_guid: String,
    pub symbol_vault_guid: String,
    pub symbol_item_guid: String,
    pub symbol_revision_guid: String,
    pub generic_component_template_guid: String,

    // --- Part info ---
    pub has_only_current_part_info: bool,
    pub all_pin_count: i16,
    pub key_component_unique_id: String,

    // --- Component kind (version-aware) ---
    pub component_kind: ComponentKind,

    // --- Custom display mode names ---
    pub custom_display_mode_names: Vec<String>,
}
