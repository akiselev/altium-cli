//! Format functions for Component record type.

use crate::error::Result;
use crate::v2::fields::component::ComponentData;
use crate::v2::serializer::SchSerializer;
use crate::v2::types::*;
use super::{export_graphical_object, import_graphical_object};

pub fn export_component(s: &mut dyn SchSerializer, comp: &ComponentData) -> Result<()> {
    s.export_dynamic_string(&comp.lib_reference, "LibReference")?;
    s.export_string(&comp.component_description, "ComponentDescription")?;
    s.export_short_int(comp.part_count as i32, "PartCount")?;
    s.export_byte(comp.display_mode_count, "DisplayModeCount")?;
    export_graphical_object(s, &comp.graphical)?;
    s.export_coord(comp.location_x, "Location.X")?;
    s.export_coord(comp.location_y, "Location.Y")?;
    s.export_display_mode(comp.display_mode, "DisplayMode")?;
    s.export_boolean(comp.is_mirrored, "IsMirrored")?;
    s.export_rotation_by90(comp.orientation, "Orientation")?;
    s.export_short_int(comp.current_part_id as i32, "CurrentPartId")?;
    s.export_boolean(comp.show_hidden_fields, "ShowHiddenFields")?;
    s.export_boolean(comp.show_hidden_pins, "ShowHiddenPins")?;
    s.export_dynamic_string(&comp.library_path, "LibraryPath")?;
    s.export_dynamic_string(&comp.source_library_name, "SourceLibraryName")?;
    s.export_dynamic_string(&comp.database_table_name, "DatabaseTableName")?;
    s.export_dynamic_string(&comp.sheet_part_file_name, "SheetPartFileName")?;
    s.export_dynamic_string(&comp.target_file_name, "TargetFileName")?;
    s.export_string(&comp.unique_id, "UniqueID")?;
    s.export_color(comp.area_color, "AreaColor")?;
    s.export_color(comp.color, "Color")?;
    s.export_color(comp.pin_color, "PinColor")?;
    s.export_boolean(comp.overide_colors, "OverideColors")?;
    s.export_boolean(comp.display_field_names, "DisplayFieldNames")?;
    s.export_boolean(comp.designator_locked, "DesignatorLocked")?;
    s.export_boolean_with_default(comp.part_id_locked, "PartIDLocked")?;
    s.export_boolean(comp.pins_moveable, "PinsMoveable")?;
    s.export_dynamic_string(&comp.alias_list, "AliasList")?;
    s.export_boolean(comp.not_use_library_name, "NotUseLibraryName")?;
    s.export_boolean(comp.not_use_db_table_name, "NotUseDBTableName")?;
    s.export_dynamic_string(&comp.design_item_id, "DesignItemId")?;
    s.export_dynamic_string(&comp.vault_guid, "VaultGUID")?;
    s.export_dynamic_string(&comp.item_guid, "ItemGUID")?;
    s.export_dynamic_string(&comp.revision_guid, "RevisionGUID")?;
    s.export_dynamic_string(&comp.symbol_vault_guid, "SymbolVaultGUID")?;
    s.export_dynamic_string(&comp.symbol_item_guid, "SymbolItemGUID")?;
    s.export_dynamic_string(&comp.symbol_revision_guid, "SymbolRevisionGUID")?;
    s.export_dynamic_string(&comp.generic_component_template_guid, "GenericComponentTemplateGUID")?;
    s.export_boolean(comp.has_only_current_part_info, "HasOnlyCurrentPartInfo")?;
    s.export_short_int(comp.all_pin_count as i32, "AllPinCount")?;
    s.export_dynamic_string(&comp.key_component_unique_id, "KeyComponentUniqueId")?;

    // ComponentKind version-aware export
    match comp.component_kind {
        ComponentKind::Jumper => {
            s.export_byte(0, "ComponentKind")?;
            s.export_byte(0, "ComponentKindVersion2")?;
            s.export_byte(6, "ComponentKindVersion3")?;
        }
        ComponentKind::StandardNoBOM => {
            s.export_byte(0, "ComponentKind")?;
            s.export_byte(comp.component_kind as u8, "ComponentKindVersion2")?;
        }
        _ => {
            s.export_byte(comp.component_kind as u8, "ComponentKind")?;
        }
    }

    for (i, name) in comp.custom_display_mode_names.iter().enumerate() {
        s.export_dynamic_string(name, &format!("CustomDisplayModeName{}", i))?;
    }

    Ok(())
}

/// Import component — from C# `FileFormatV5.ImportComponent` (lines 2790-2946).
pub fn import_component(s: &mut dyn SchSerializer, comp: &mut ComponentData) -> Result<()> {
    comp.lib_reference = s.import_dynamic_string("LibReference")?;
    comp.component_description = s.import_string("ComponentDescription")?;
    comp.part_count = s.import_short_int("PartCount")? as i16;
    comp.display_mode_count = s.import_byte("DisplayModeCount")?;
    import_graphical_object(s, &mut comp.graphical)?;
    comp.location_x = s.import_coord("Location.X")?;
    comp.location_y = s.import_coord("Location.Y")?;
    comp.display_mode = s.import_display_mode("DisplayMode")?;
    comp.is_mirrored = s.import_boolean("IsMirrored")?;
    comp.orientation = s.import_rotation_by90("Orientation")?;
    comp.current_part_id = s.import_short_int("CurrentPartId")? as i16;
    comp.show_hidden_fields = s.import_boolean("ShowHiddenFields")?;
    comp.show_hidden_pins = s.import_boolean("ShowHiddenPins")?;
    comp.library_path = s.import_dynamic_string("LibraryPath")?;
    comp.source_library_name = s.import_dynamic_string("SourceLibraryName")?;
    comp.database_table_name = s.import_dynamic_string("DatabaseTableName")?;
    comp.sheet_part_file_name = s.import_dynamic_string("SheetPartFileName")?;
    comp.target_file_name = s.import_dynamic_string("TargetFileName")?;
    comp.unique_id = s.import_string("UniqueID")?;
    comp.area_color = s.import_color("AreaColor")?;
    comp.color = s.import_color("Color")?;
    comp.pin_color = s.import_color("PinColor")?;
    comp.overide_colors = s.import_boolean("OverideColors")?;
    comp.display_field_names = s.import_boolean("DisplayFieldNames")?;
    comp.designator_locked = s.import_boolean("DesignatorLocked")?;
    comp.part_id_locked = s.import_boolean_with_default("PartIDLocked", comp.designator_locked)?;
    comp.pins_moveable = s.import_boolean("PinsMoveable")?;
    comp.alias_list = s.import_dynamic_string("AliasList")?;
    comp.not_use_library_name = s.import_boolean("NotUseLibraryName")?;
    comp.not_use_db_table_name = s.import_boolean("NotUseDBTableName")?;
    comp.design_item_id = s.import_dynamic_string("DesignItemId")?;

    // ComponentKind version-aware import
    let kind_v1 = s.import_byte("ComponentKind")?;
    let kind_v2 = s.import_byte("ComponentKindVersion2")?;
    let kind_v3 = s.import_byte("ComponentKindVersion3")?;
    let effective_v2 = if kind_v3 == 6 { kind_v3 } else { kind_v2 };
    comp.component_kind = if effective_v2 >= 5 {
        ComponentKind::from_u8(effective_v2).unwrap_or_default()
    } else {
        ComponentKind::from_u8(kind_v1).unwrap_or_default()
    };

    comp.vault_guid = s.import_dynamic_string("VaultGUID")?;
    comp.item_guid = s.import_dynamic_string("ItemGUID")?;
    comp.revision_guid = s.import_dynamic_string("RevisionGUID")?;
    comp.symbol_vault_guid = s.import_dynamic_string("SymbolVaultGUID")?;
    comp.symbol_item_guid = s.import_dynamic_string("SymbolItemGUID")?;
    comp.symbol_revision_guid = s.import_dynamic_string("SymbolRevisionGUID")?;
    comp.generic_component_template_guid = s.import_dynamic_string("GenericComponentTemplateGUID")?;
    comp.has_only_current_part_info = s.import_boolean("HasOnlyCurrentPartInfo")?;
    comp.all_pin_count = s.import_short_int("AllPinCount")? as i16;
    comp.key_component_unique_id = s.import_dynamic_string("KeyComponentUniqueId")?;

    comp.custom_display_mode_names.clear();
    for i in 0..comp.display_mode_count {
        let name = s.import_dynamic_string(&format!("CustomDisplayModeName{}", i))?;
        comp.custom_display_mode_names.push(name);
    }

    Ok(())
}
