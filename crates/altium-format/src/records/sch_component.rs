//! Schematic component record (RECORD=1).

use super::enums::*;
use crate::coord::SchCoord;
use crate::newtypes::{Designator, LibReference, UniqueId};
use altium_format_derive::altium_record;

/// Schematic component record -- RECORD=1.
///
/// Represents a component instance placed on a schematic sheet.
/// Fields map to C# `SchDataComponent` / `FileFormatV5.ExportComponent`.
#[altium_record(kind = "sch", record_id = 1, codec = "params")]
pub struct SchComponentRecord {
    // --- Identification ---
    #[altium(key = "LibReference")]
    lib_reference: LibReference,

    #[altium(key = "ComponentDescription")]
    component_description: String,

    #[altium(key = "PartCount")]
    part_count: i16,

    #[altium(key = "DisplayModeCount")]
    display_mode_count: u8,

    // --- Base object fields (flattened from GraphicalObjectBase -> DataObjectBase) ---
    #[altium(key = "OwnerIndex")]
    owner_index: i32,

    #[altium(key = "OwnerPartId")]
    owner_part_id: i16,

    #[altium(key = "OwnerPartDisplayMode")]
    owner_part_display_mode: u8,

    #[altium(key = "IndexInSheet")]
    index_in_sheet: i32,

    #[altium(key = "IsNotAccesible")]
    is_not_accessible: bool,

    #[altium(key = "GraphicallyLocked")]
    graphically_locked: bool,

    // --- Position ---
    #[altium(key = "Location.X")]
    location_x: SchCoord,

    #[altium(key = "Location.Y")]
    location_y: SchCoord,

    // --- Component properties ---
    #[altium(key = "DisplayMode")]
    display_mode: u8,

    #[altium(key = "IsMirrored")]
    is_mirrored: bool,

    #[altium(key = "Orientation")]
    orientation: RotationBy90,

    #[altium(key = "CurrentPartId")]
    current_part_id: i16,

    #[altium(key = "ShowHiddenFields")]
    show_hidden_fields: bool,

    #[altium(key = "ShowHiddenPins")]
    show_hidden_pins: bool,

    // --- Library references ---
    #[altium(key = "LibraryPath")]
    library_path: String,

    #[altium(key = "SourceLibraryName")]
    source_library_name: String,

    #[altium(key = "DatabaseTableName")]
    database_table_name: String,

    #[altium(key = "SheetPartFileName")]
    sheet_part_file_name: String,

    #[altium(key = "TargetFileName")]
    target_file_name: String,

    #[altium(key = "UniqueID")]
    unique_id: UniqueId,

    // --- Colors ---
    #[altium(key = "AreaColor")]
    area_color: u32,

    #[altium(key = "Color")]
    color: u32,

    #[altium(key = "PinColor")]
    pin_color: u32,

    #[altium(key = "OverideColors")]
    overide_colors: bool,

    // --- Flags ---
    #[altium(key = "DisplayFieldNames")]
    display_field_names: bool,

    #[altium(key = "DesignatorLocked")]
    designator_locked: bool,

    #[altium(key = "PartIDLocked", emit = "with_default")]
    part_id_locked: bool,

    #[altium(key = "PinsMoveable")]
    pins_moveable: bool,

    // --- Alias list ---
    #[altium(key = "AliasList")]
    alias_list: String,

    // --- Library name usage ---
    #[altium(key = "NotUseLibraryName")]
    not_use_library_name: bool,

    #[altium(key = "NotUseDBTableName")]
    not_use_db_table_name: bool,

    // --- Design item ---
    #[altium(key = "DesignItemId")]
    design_item_id: String,

    // --- Vault/GUID ---
    #[altium(key = "VaultGUID")]
    vault_guid: String,

    #[altium(key = "ItemGUID")]
    item_guid: String,

    #[altium(key = "RevisionGUID")]
    revision_guid: String,

    #[altium(key = "SymbolVaultGUID")]
    symbol_vault_guid: String,

    #[altium(key = "SymbolItemGUID")]
    symbol_item_guid: String,

    #[altium(key = "SymbolRevisionGUID")]
    symbol_revision_guid: String,

    #[altium(key = "GenericComponentTemplateGUID")]
    generic_component_template_guid: String,

    // --- Part info ---
    #[altium(key = "HasOnlyCurrentPartInfo")]
    has_only_current_part_info: bool,

    #[altium(key = "AllPinCount")]
    all_pin_count: i16,

    #[altium(key = "KeyComponentUniqueId")]
    key_component_unique_id: String,

    // --- Component kind ---
    #[altium(key = "ComponentKind")]
    component_kind: ComponentKind,

    // --- Designator (present in SchDoc component instances) ---
    #[altium(key = "Designator")]
    designator: Designator,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backing_store::{ParamOrigin, RecordOrigin};

    #[test]
    fn roundtrip_component_getter() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=1|LibReference=Resistor|ComponentDescription=100k Resistor|PartCount=1|Location.X=100|Location.Y=200|UniqueID=ABCD1234|Color=128|",
        ));
        let rec = SchComponentRecord::from_origin(origin);
        assert_eq!(rec.lib_reference(), LibReference::from("Resistor"));
        assert_eq!(rec.component_description(), "100k Resistor");
        assert_eq!(rec.part_count(), 1);
        assert_eq!(rec.color(), 128);
        assert_eq!(rec.unique_id(), UniqueId::from("ABCD1234"));
    }

    #[test]
    fn roundtrip_component_setter() {
        let origin = RecordOrigin::Param(ParamOrigin::new(
            "|RECORD=1|LibReference=Resistor|PartCount=1|",
        ));
        let mut rec = SchComponentRecord::from_origin(origin);
        rec.set_lib_reference(LibReference::from("Capacitor"));
        assert_eq!(rec.lib_reference(), LibReference::from("Capacitor"));
        rec.set_part_count(2);
        assert_eq!(rec.part_count(), 2);
    }

    #[test]
    fn emit_policy_with_default_and_sparse() {
        let origin = RecordOrigin::Param(ParamOrigin::new("|RECORD=1|"));
        let mut rec = SchComponentRecord::from_origin(origin);

        // `part_id_locked` is emitted with explicit defaults.
        rec.set_part_id_locked(false);
        assert_eq!(rec.try_part_id_locked(), Some(false));

        // `designator_locked` remains sparse by default.
        rec.set_designator_locked(false);
        assert_eq!(rec.try_designator_locked(), None);
    }
}
