//! Format V5 export/import functions — ported from `FileFormatV5.cs`.
//!
//! Each record type has an `export_*` and `import_*` function pair that
//! serializes/deserializes the record's fields via the `SchSerializer` trait.
//!
//! Field order, names, and types match the C# code exactly.

use crate::error::Result;
use crate::v2::fields::*;
use crate::v2::fields::component::ComponentData;
use crate::v2::fields::pin::PinData;
use crate::v2::serializer::SchSerializer;
use crate::v2::types::*;

// ============================================================================
// Base object helpers
// ============================================================================

/// Export base data object fields — from C# `ExportDataObject`.
pub fn export_data_object(s: &mut dyn SchSerializer, obj: &DataObjectBase) -> Result<()> {
    s.export_long_int(obj.owner_index, "OwnerIndex")?;
    s.export_boolean(obj.is_not_accessible, "IsNotAccesible")?;
    s.export_boolean(obj.owner_index_additional_list, "OwnerIndexAdditionalList")?;
    s.export_long_int(obj.index_in_sheet, "IndexInSheet")?;
    if obj.ignore_on_load {
        s.export_boolean(true, "IgnoreOnLoad")?;
    }
    s.export_boolean(obj.is_schematic_block_object, "IsSchematicBlockObject")?;
    if !obj.unique_id_in_reuse_block.is_empty() {
        s.export_string(&obj.unique_id_in_reuse_block, "UniqueIDInReuseBlock")?;
    }
    Ok(())
}

/// Import base data object fields — from C# `ImportDataObject`.
pub fn import_data_object(s: &mut dyn SchSerializer, obj: &mut DataObjectBase) -> Result<()> {
    obj.owner_index = s.import_long_int("OwnerIndex")?;
    obj.is_not_accessible = s.import_boolean("IsNotAccesible")?;
    obj.owner_index_additional_list = s.import_boolean("OwnerIndexAdditionalList")?;
    obj.index_in_sheet = s.import_long_int("IndexInSheet")?;
    obj.ignore_on_load = s.import_boolean("IgnoreOnLoad")?;
    obj.is_schematic_block_object = s.import_boolean("IsSchematicBlockObject")?;
    obj.unique_id_in_reuse_block = s.import_string("UniqueIDInReuseBlock")?;
    Ok(())
}

/// Export graphical object fields — from C# `ExportGraphicalObject`.
pub fn export_graphical_object(s: &mut dyn SchSerializer, obj: &GraphicalObjectBase) -> Result<()> {
    export_data_object(s, &obj.base)?;
    s.export_short_int(obj.owner_part_id as i32, "OwnerPartId")?;
    s.export_byte(obj.owner_part_display_mode, "OwnerPartDisplayMode")?;
    s.export_byte(obj.selection_memory, "SelectionMemory")?;
    s.export_long_int(obj.union_index, "UnionIndex")?;
    s.export_boolean(obj.graphically_locked, "GraphicallyLocked")?;
    Ok(())
}

/// Import graphical object fields — from C# `ImportGraphicalObject`.
pub fn import_graphical_object(s: &mut dyn SchSerializer, obj: &mut GraphicalObjectBase) -> Result<()> {
    import_data_object(s, &mut obj.base)?;
    obj.owner_part_id = s.import_short_int("OwnerPartId")? as i16;
    obj.owner_part_display_mode = s.import_byte("OwnerPartDisplayMode")?;
    obj.selection_memory = s.import_byte("SelectionMemory")?;
    obj.union_index = s.import_long_int("UnionIndex")?;
    obj.graphically_locked = s.import_boolean("GraphicallyLocked")?;
    Ok(())
}

// ============================================================================
// Pin (ObjectId = 2) — most complex record type
// ============================================================================

/// Export pin — from C# `FileFormatV5.ExportPin` (lines 296-418).
pub fn export_pin(s: &mut dyn SchSerializer, pin: &PinData) -> Result<()> {
    s.export_long_int(pin.owner_index, "OwnerIndex")?;
    s.export_short_int(pin.owner_part_id as i32, "OwnerPartId")?;
    s.export_byte(pin.owner_part_display_mode, "OwnerPartDisplayMode")?;
    s.export_byte(pin.symbol_inner_edge as u8, "SymBol_InnerEdge")?;
    s.export_byte(pin.symbol_outer_edge as u8, "SymBol_OuterEdge")?;
    s.export_byte(pin.symbol_inner as u8, "SymBol_Inner")?;
    s.export_byte(pin.symbol_outer as u8, "SymBol_Outer")?;
    s.export_dynamic_string(&pin.description, "Description")?;
    s.export_byte(pin.formal_type as u8, "FormalType")?;
    s.export_pin_electrical(pin.electrical, "Electrical")?;

    // PinConglomerate — packed byte
    let mut conglom: u8 = pin.orientation as u8 & 0x03;
    if pin.is_hidden { conglom |= 0x04; }
    if pin.show_name { conglom |= 0x08; }
    if pin.show_designator { conglom |= 0x10; }
    if !pin.is_accessible { conglom |= 0x20; }
    if pin.graphically_locked { conglom |= 0x40; }
    if pin.owner_index_additional_list { conglom |= 0x80; }
    s.export_byte(conglom, "PinConglomerate")?;

    s.export_coord(pin.pin_length, "PinLength")?;
    s.export_coord(pin.location_x, "Location.X")?;
    s.export_coord(pin.location_y, "Location.Y")?;
    s.export_color(pin.color, "Color")?;
    s.export_dynamic_string(&pin.name, "Name")?;
    s.export_dynamic_string(&pin.designator, "Designator")?;
    s.export_string(&pin.swap_id_pin, "SwapIdPin")?;
    s.export_string(&pin.swap_id_part, "SwapIDPart")?;
    s.export_dynamic_string(&pin.default_value, "DefaultValue")?;
    s.export_ascii_only_string(&pin.swap_id_pair, "SwapIdPair")?;

    // Name customization (ASCII-only, conditional)
    if pin.name_position_mode == PinItemMode::Custom || pin.name_font_mode == PinItemMode::Custom {
        let mut b: u8 = 0;
        if pin.name_position_mode == PinItemMode::Custom {
            b |= 1;
            if pin.name_custom_rotation_anchor == PinTextRotationAnchor::Component {
                b |= 2;
            }
            b |= ((pin.name_custom_rotation_relative as u8) << 2) & 0x0C;
        }
        if pin.name_font_mode == PinItemMode::Custom {
            b |= 0x10;
        }
        s.export_ascii_only_byte(b, "PinName_PositionConglomerate")?;
        if pin.name_position_mode == PinItemMode::Custom {
            s.export_ascii_only_coord(pin.name_custom_position_margin, "Name_CustomPosition_Margin")?;
        }
        if pin.name_font_mode == PinItemMode::Custom {
            s.export_ascii_only_font_id(pin.name_custom_font_id, "Name_CustomFontID")?;
            s.export_ascii_only_color(pin.name_custom_color, "Name_CustomColor")?;
        }
    }

    // Designator customization (ASCII-only, conditional)
    if pin.designator_position_mode == PinItemMode::Custom || pin.designator_font_mode == PinItemMode::Custom {
        let mut b: u8 = 0;
        if pin.designator_position_mode == PinItemMode::Custom {
            b |= 1;
            if pin.designator_custom_rotation_anchor == PinTextRotationAnchor::Component {
                b |= 2;
            }
            b |= ((pin.designator_custom_rotation_relative as u8) << 2) & 0x0C;
        }
        if pin.designator_font_mode == PinItemMode::Custom {
            b |= 0x10;
        }
        s.export_ascii_only_byte(b, "PinDesignator_PositionConglomerate")?;
        if pin.designator_position_mode == PinItemMode::Custom {
            s.export_ascii_only_coord(pin.designator_custom_position_margin, "Designator_CustomPosition_Margin")?;
        }
        if pin.designator_font_mode == PinItemMode::Custom {
            s.export_ascii_only_font_id(pin.designator_custom_font_id, "Designator_CustomFontID")?;
            s.export_ascii_only_color(pin.designator_custom_color, "Designator_CustomColor")?;
        }
    }

    s.export_ascii_only_byte(pin.symbol_line_width as u8, "SymBol_LineWidth")?;
    s.export_ascii_only_coord(pin.pin_package_length, "PinPackageLength")?;
    s.export_ascii_only_double(pin.pin_propagation_delay, "PinPropagationDelay")?;

    if !pin.unique_id.is_empty() {
        s.export_dynamic_string(&pin.unique_id, "UniqueID")?;
    }

    s.export_ascii_only_boolean(pin.hide_pin_name_as_function, "HidePinNameAsFunction")?;
    s.export_ascii_only_string(&pin.pin_symbolic_name, "PinSymbolicName")?;
    s.export_ascii_only_boolean(pin.show_symbolic_name_as_function, "ShowPinSymbolicNameAsFunction")?;

    Ok(())
}

/// Import pin — from C# `FileFormatV5.ImportPin` (lines 420-588).
pub fn import_pin(s: &mut dyn SchSerializer, pin: &mut PinData) -> Result<()> {
    pin.owner_index = s.import_long_int("OwnerIndex")?;
    pin.owner_part_id = s.import_short_int("OwnerPartId")? as i16;
    pin.owner_part_display_mode = s.import_byte("OwnerPartDisplayMode")?;
    pin.symbol_inner_edge = IeeeSymbol::from_u8(s.import_byte("SymBol_InnerEdge")?);
    pin.symbol_outer_edge = IeeeSymbol::from_u8(s.import_byte("SymBol_OuterEdge")?);
    pin.symbol_inner = IeeeSymbol::from_u8(s.import_byte("SymBol_Inner")?);
    pin.symbol_outer = IeeeSymbol::from_u8(s.import_byte("SymBol_Outer")?);
    pin.description = s.import_dynamic_string("Description")?;
    pin.formal_type = StdLogicState::from_u8(s.import_byte("FormalType")?).unwrap_or_default();
    pin.electrical = s.import_pin_electrical("Electrical")?;

    // PinConglomerate — packed byte
    let conglom = s.import_byte("PinConglomerate")?;
    pin.orientation = RotationBy90::from_u8(conglom & 0x03).unwrap_or_default();
    pin.is_hidden = (conglom & 0x04) != 0;
    pin.show_name = (conglom & 0x08) != 0;
    pin.show_designator = (conglom & 0x10) != 0;
    pin.is_accessible = (conglom & 0x20) == 0; // inverted!
    pin.graphically_locked = false; // C# always sets false on import
    pin.owner_index_additional_list = (conglom & 0x80) != 0;

    pin.pin_length = s.import_coord("PinLength")?;
    pin.location_x = s.import_coord("Location.X")?;
    pin.location_y = s.import_coord("Location.Y")?;
    pin.color = s.import_color("Color")?;
    pin.name = s.import_dynamic_string("Name")?;
    pin.designator = s.import_dynamic_string("Designator")?;
    pin.swap_id_pin = s.import_string("SwapIdPin")?;
    pin.swap_id_part = s.import_dynamic_string("SwapIDPart")?;
    pin.default_value = s.import_dynamic_string("DefaultValue")?;
    pin.swap_id_pair = s.import_ascii_only_string("SwapIdPair")?;

    // Name position conglomerate (ASCII-only)
    let name_conglom = s.import_ascii_only_byte("PinName_PositionConglomerate")?;
    if (name_conglom & 1) != 0 {
        pin.name_position_mode = PinItemMode::Custom;
        pin.name_custom_rotation_anchor = if (name_conglom & 2) != 0 {
            PinTextRotationAnchor::Component
        } else {
            PinTextRotationAnchor::Pin
        };
        pin.name_custom_rotation_relative = RotationBy90::from_u8((name_conglom & 0x0C) >> 2).unwrap_or_default();
        pin.name_custom_position_margin = s.import_ascii_only_coord("Name_CustomPosition_Margin")?;
    } else {
        pin.name_position_mode = PinItemMode::Default;
    }
    if (name_conglom & 0x10) != 0 {
        pin.name_font_mode = PinItemMode::Custom;
        pin.name_custom_font_id = s.import_ascii_only_font_id("Name_CustomFontID")?;
        pin.name_custom_color = s.import_ascii_only_color("Name_CustomColor")?;
    } else {
        pin.name_font_mode = PinItemMode::Default;
    }

    // Designator position conglomerate (ASCII-only)
    let desig_conglom = s.import_ascii_only_byte("PinDesignator_PositionConglomerate")?;
    if (desig_conglom & 1) != 0 {
        pin.designator_position_mode = PinItemMode::Custom;
        pin.designator_custom_rotation_anchor = if (desig_conglom & 2) != 0 {
            PinTextRotationAnchor::Component
        } else {
            PinTextRotationAnchor::Pin
        };
        pin.designator_custom_rotation_relative = RotationBy90::from_u8((desig_conglom & 0x0C) >> 2).unwrap_or_default();
        pin.designator_custom_position_margin = s.import_ascii_only_coord("Designator_CustomPosition_Margin")?;
    } else {
        pin.designator_position_mode = PinItemMode::Default;
    }
    if (desig_conglom & 0x10) != 0 {
        pin.designator_font_mode = PinItemMode::Custom;
        pin.designator_custom_font_id = s.import_ascii_only_font_id("Designator_CustomFontID")?;
        pin.designator_custom_color = s.import_ascii_only_color("Designator_CustomColor")?;
    } else {
        pin.designator_font_mode = PinItemMode::Default;
    }

    pin.symbol_line_width = Size::from_u8(s.import_ascii_only_byte("SymBol_LineWidth")?).unwrap_or_default();
    pin.pin_package_length = s.import_ascii_only_coord("PinPackageLength")?;
    pin.pin_propagation_delay = s.import_ascii_only_double("PinPropagationDelay")?;
    pin.unique_id = s.import_dynamic_string("UniqueID")?;
    pin.hide_pin_name_as_function = s.import_ascii_only_boolean("HidePinNameAsFunction")?;
    pin.pin_symbolic_name = s.import_ascii_only_string("PinSymbolicName")?;
    pin.show_symbolic_name_as_function = s.import_ascii_only_boolean("ShowPinSymbolicNameAsFunction")?;

    Ok(())
}

// ============================================================================
// Component (ObjectId = 1)
// ============================================================================

/// Export component — from C# `FileFormatV5.ExportComponent` (lines 2722-2788).
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

// ============================================================================
// Arc (ObjectId = 12)
// ============================================================================

pub fn export_arc(s: &mut dyn SchSerializer, arc: &ArcData) -> Result<()> {
    export_graphical_object(s, &arc.graphical)?;
    s.export_coord(arc.location_x, "Location.X")?;
    s.export_coord(arc.location_y, "Location.Y")?;
    s.export_coord(arc.radius, "Radius")?;
    s.export_size(arc.line_width, "LineWidth")?;
    s.export_angle(arc.start_angle, "StartAngle")?;
    s.export_angle(arc.end_angle, "EndAngle")?;
    s.export_color(arc.color, "Color")?;
    s.export_dynamic_string(&arc.unique_id, "UniqueID")?;
    Ok(())
}

pub fn import_arc(s: &mut dyn SchSerializer, arc: &mut ArcData) -> Result<()> {
    import_graphical_object(s, &mut arc.graphical)?;
    arc.location_x = s.import_coord("Location.X")?;
    arc.location_y = s.import_coord("Location.Y")?;
    arc.radius = s.import_coord("Radius")?;
    arc.line_width = s.import_size("LineWidth")?;
    arc.start_angle = s.import_angle("StartAngle")?;
    arc.end_angle = s.import_angle("EndAngle")?;
    arc.color = s.import_color("Color")?;
    arc.unique_id = s.import_dynamic_string("UniqueID")?;
    Ok(())
}

// ============================================================================
// Ellipse (ObjectId = 11)
// ============================================================================

pub fn export_ellipse(s: &mut dyn SchSerializer, e: &EllipseData) -> Result<()> {
    export_graphical_object(s, &e.graphical)?;
    s.export_coord(e.location_x, "Location.X")?;
    s.export_coord(e.location_y, "Location.Y")?;
    s.export_coord(e.radius, "Radius")?;
    s.export_coord(e.secondary_radius, "SecondaryRadius")?;
    s.export_size(e.line_width, "LineWidth")?;
    s.export_color(e.color, "Color")?;
    s.export_color(e.area_color, "AreaColor")?;
    s.export_boolean(e.is_solid, "IsSolid")?;
    s.export_boolean(e.transparent, "Transparent")?;
    s.export_dynamic_string(&e.unique_id, "UniqueID")?;
    Ok(())
}

pub fn import_ellipse(s: &mut dyn SchSerializer, e: &mut EllipseData) -> Result<()> {
    import_graphical_object(s, &mut e.graphical)?;
    e.location_x = s.import_coord("Location.X")?;
    e.location_y = s.import_coord("Location.Y")?;
    e.radius = s.import_coord("Radius")?;
    e.secondary_radius = s.import_coord("SecondaryRadius")?;
    e.line_width = s.import_size("LineWidth")?;
    e.color = s.import_color("Color")?;
    e.area_color = s.import_color("AreaColor")?;
    e.is_solid = s.import_boolean("IsSolid")?;
    e.transparent = s.import_boolean("Transparent")?;
    e.unique_id = s.import_dynamic_string("UniqueID")?;
    Ok(())
}

// ============================================================================
// Line (ObjectId = 13)
// ============================================================================

pub fn export_line(s: &mut dyn SchSerializer, line: &LineData) -> Result<()> {
    export_graphical_object(s, &line.graphical)?;
    s.export_coord(line.location_x, "Location.X")?;
    s.export_coord(line.location_y, "Location.Y")?;
    s.export_coord(line.corner_x, "Corner.X")?;
    s.export_coord(line.corner_y, "Corner.Y")?;
    s.export_size(line.line_width, "LineWidth")?;
    s.export_line_style(line.line_style, "LineStyle")?;
    s.export_color(line.color, "Color")?;
    s.export_dynamic_string(&line.unique_id, "UniqueID")?;
    Ok(())
}

pub fn import_line(s: &mut dyn SchSerializer, line: &mut LineData) -> Result<()> {
    import_graphical_object(s, &mut line.graphical)?;
    line.location_x = s.import_coord("Location.X")?;
    line.location_y = s.import_coord("Location.Y")?;
    line.corner_x = s.import_coord("Corner.X")?;
    line.corner_y = s.import_coord("Corner.Y")?;
    line.line_width = s.import_size("LineWidth")?;
    line.line_style = s.import_line_style("LineStyle")?;
    line.color = s.import_color("Color")?;
    line.unique_id = s.import_dynamic_string("UniqueID")?;
    Ok(())
}

// ============================================================================
// Rectangle (ObjectId = 14)
// ============================================================================

pub fn export_rectangle(s: &mut dyn SchSerializer, rect: &RectangleData) -> Result<()> {
    export_graphical_object(s, &rect.graphical)?;
    s.export_coord(rect.location_x, "Location.X")?;
    s.export_coord(rect.location_y, "Location.Y")?;
    s.export_coord(rect.corner_x, "Corner.X")?;
    s.export_coord(rect.corner_y, "Corner.Y")?;
    s.export_line_style(rect.line_style, "LineStyleExt")?;
    s.export_size(rect.line_width, "LineWidth")?;
    s.export_color(rect.color, "Color")?;
    s.export_color(rect.area_color, "AreaColor")?;
    s.export_boolean(rect.is_solid, "IsSolid")?;
    s.export_boolean(rect.transparent, "Transparent")?;
    s.export_dynamic_string(&rect.unique_id, "UniqueID")?;
    Ok(())
}

pub fn import_rectangle(s: &mut dyn SchSerializer, rect: &mut RectangleData) -> Result<()> {
    import_graphical_object(s, &mut rect.graphical)?;
    rect.location_x = s.import_coord("Location.X")?;
    rect.location_y = s.import_coord("Location.Y")?;
    rect.corner_x = s.import_coord("Corner.X")?;
    rect.corner_y = s.import_coord("Corner.Y")?;
    rect.line_style = s.import_line_style("LineStyleExt")?;
    rect.line_width = s.import_size("LineWidth")?;
    rect.color = s.import_color("Color")?;
    rect.area_color = s.import_color("AreaColor")?;
    rect.is_solid = s.import_boolean("IsSolid")?;
    rect.transparent = s.import_boolean("Transparent")?;
    rect.unique_id = s.import_dynamic_string("UniqueID")?;
    Ok(())
}

// ============================================================================
// Bezier (ObjectId = 5)
// ============================================================================

pub fn export_bezier(s: &mut dyn SchSerializer, bez: &BezierData) -> Result<()> {
    export_graphical_object(s, &bez.graphical)?;
    s.export_size(bez.line_width, "LineWidth")?;
    s.export_color(bez.color, "Color")?;
    export_vertices(s, &bez.vertices)?;
    s.export_dynamic_string(&bez.unique_id, "UniqueID")?;
    Ok(())
}

pub fn import_bezier(s: &mut dyn SchSerializer, bez: &mut BezierData) -> Result<()> {
    import_graphical_object(s, &mut bez.graphical)?;
    bez.line_width = s.import_size("LineWidth")?;
    bez.color = s.import_color("Color")?;
    bez.vertices = import_vertices(s)?;
    bez.unique_id = s.import_dynamic_string("UniqueID")?;
    Ok(())
}

// ============================================================================
// Polyline (ObjectId = 6)
// ============================================================================

pub fn export_polyline(s: &mut dyn SchSerializer, pl: &PolylineData) -> Result<()> {
    export_graphical_object(s, &pl.graphical)?;
    s.export_size(pl.line_width, "LineWidth")?;
    s.export_line_style(pl.line_style, "LineStyle")?;
    s.export_line_shape(pl.start_line_shape, "StartLineShape")?;
    s.export_line_shape(pl.end_line_shape, "EndLineShape")?;
    s.export_size(pl.line_shape_size, "LineShapeSize")?;
    s.export_color(pl.color, "Color")?;
    export_vertices(s, &pl.vertices)?;
    s.export_dynamic_string(&pl.unique_id, "UniqueID")?;
    Ok(())
}

pub fn import_polyline(s: &mut dyn SchSerializer, pl: &mut PolylineData) -> Result<()> {
    import_graphical_object(s, &mut pl.graphical)?;
    pl.line_width = s.import_size("LineWidth")?;
    pl.line_style = s.import_line_style("LineStyle")?;
    pl.start_line_shape = s.import_line_shape("StartLineShape")?;
    pl.end_line_shape = s.import_line_shape("EndLineShape")?;
    pl.line_shape_size = s.import_size("LineShapeSize")?;
    pl.color = s.import_color("Color")?;
    pl.vertices = import_vertices(s)?;
    pl.unique_id = s.import_dynamic_string("UniqueID")?;
    Ok(())
}

// ============================================================================
// Polygon (ObjectId = 7)
// ============================================================================

pub fn export_polygon(s: &mut dyn SchSerializer, poly: &PolygonData) -> Result<()> {
    export_graphical_object(s, &poly.graphical)?;
    s.export_size(poly.line_width, "LineWidth")?;
    s.export_color(poly.color, "Color")?;
    s.export_color(poly.area_color, "AreaColor")?;
    s.export_boolean(poly.is_solid, "IsSolid")?;
    s.export_boolean(poly.transparent, "Transparent")?;
    export_vertices(s, &poly.vertices)?;
    s.export_dynamic_string(&poly.unique_id, "UniqueID")?;
    Ok(())
}

pub fn import_polygon(s: &mut dyn SchSerializer, poly: &mut PolygonData) -> Result<()> {
    import_graphical_object(s, &mut poly.graphical)?;
    poly.line_width = s.import_size("LineWidth")?;
    poly.color = s.import_color("Color")?;
    poly.area_color = s.import_color("AreaColor")?;
    poly.is_solid = s.import_boolean("IsSolid")?;
    poly.transparent = s.import_boolean("Transparent")?;
    poly.vertices = import_vertices(s)?;
    poly.unique_id = s.import_dynamic_string("UniqueID")?;
    Ok(())
}

// ============================================================================
// Junction (ObjectId = 29)
// ============================================================================

pub fn export_junction(s: &mut dyn SchSerializer, j: &JunctionData) -> Result<()> {
    export_graphical_object(s, &j.graphical)?;
    s.export_coord(j.location_x, "Location.X")?;
    s.export_coord(j.location_y, "Location.Y")?;
    s.export_size(j.size, "Size")?;
    s.export_color(j.color, "Color")?;
    s.export_boolean(j.locked, "Locked")?;
    s.export_dynamic_string(&j.unique_id, "UniqueID")?;
    Ok(())
}

pub fn import_junction(s: &mut dyn SchSerializer, j: &mut JunctionData) -> Result<()> {
    import_graphical_object(s, &mut j.graphical)?;
    j.location_x = s.import_coord("Location.X")?;
    j.location_y = s.import_coord("Location.Y")?;
    j.size = s.import_size("Size")?;
    j.color = s.import_color("Color")?;
    j.locked = s.import_boolean("Locked")?;
    j.unique_id = s.import_dynamic_string("UniqueID")?;
    Ok(())
}

// ============================================================================
// Label (ObjectId = 4)
// ============================================================================

pub fn export_label(s: &mut dyn SchSerializer, label: &LabelData) -> Result<()> {
    export_graphical_object(s, &label.graphical)?;
    s.export_coord(label.location_x, "Location.X")?;
    s.export_coord(label.location_y, "Location.Y")?;
    s.export_rotation_by90(label.orientation, "Orientation")?;
    s.export_text_justification(label.justification, "Justification")?;
    s.export_color(label.color, "Color")?;
    s.export_font_id(label.font_id, "FontID")?;
    s.export_dynamic_string(&label.text, "Text")?;
    s.export_boolean(label.is_mirrored, "IsMirrored")?;
    s.export_dynamic_string(&label.url, "URL")?;
    s.export_dynamic_string(&label.unique_id, "UniqueID")?;
    Ok(())
}

pub fn import_label(s: &mut dyn SchSerializer, label: &mut LabelData) -> Result<()> {
    import_graphical_object(s, &mut label.graphical)?;
    label.location_x = s.import_coord("Location.X")?;
    label.location_y = s.import_coord("Location.Y")?;
    label.orientation = s.import_rotation_by90("Orientation")?;
    label.justification = s.import_text_justification("Justification")?;
    label.color = s.import_color("Color")?;
    label.font_id = s.import_font_id("FontID")?;
    label.text = s.import_dynamic_string("Text")?;
    label.is_mirrored = s.import_boolean("IsMirrored")?;
    label.url = s.import_dynamic_string("URL")?;
    label.unique_id = s.import_dynamic_string("UniqueID")?;
    Ok(())
}

// ============================================================================
// NetLabel (ObjectId = 25)
// ============================================================================

pub fn export_net_label(s: &mut dyn SchSerializer, nl: &NetLabelData) -> Result<()> {
    export_graphical_object(s, &nl.graphical)?;
    s.export_coord(nl.location_x, "Location.X")?;
    s.export_coord(nl.location_y, "Location.Y")?;
    s.export_rotation_by90(nl.orientation, "Orientation")?;
    s.export_text_justification(nl.justification, "Justification")?;
    s.export_color(nl.color, "Color")?;
    s.export_font_id(nl.font_id, "FontID")?;
    s.export_dynamic_string(&nl.text, "Text")?;
    s.export_boolean(nl.is_mirrored, "IsMirrored")?;
    s.export_dynamic_string(&nl.unique_id, "UniqueID")?;
    Ok(())
}

pub fn import_net_label(s: &mut dyn SchSerializer, nl: &mut NetLabelData) -> Result<()> {
    import_graphical_object(s, &mut nl.graphical)?;
    nl.location_x = s.import_coord("Location.X")?;
    nl.location_y = s.import_coord("Location.Y")?;
    nl.orientation = s.import_rotation_by90("Orientation")?;
    nl.justification = s.import_text_justification("Justification")?;
    nl.color = s.import_color("Color")?;
    nl.font_id = s.import_font_id("FontID")?;
    nl.text = s.import_dynamic_string("Text")?;
    nl.is_mirrored = s.import_boolean("IsMirrored")?;
    nl.unique_id = s.import_dynamic_string("UniqueID")?;
    Ok(())
}

// ============================================================================
// Wire (ObjectId = 27)
// ============================================================================

pub fn export_wire(s: &mut dyn SchSerializer, wire: &WireData) -> Result<()> {
    export_graphical_object(s, &wire.graphical)?;
    s.export_size(wire.line_width, "LineWidth")?;
    s.export_color(wire.color, "Color")?;
    s.export_color(wire.underline_color, "UnderlineColor")?;
    s.export_dynamic_string(&wire.unique_id, "UniqueID")?;
    s.export_dynamic_string(&wire.assigned_interface, "AssignedInterface")?;
    s.export_dynamic_string(&wire.assigned_interface_signal, "AssignedInterfaceSignal")?;
    export_vertices(s, &wire.vertices)?;
    Ok(())
}

pub fn import_wire(s: &mut dyn SchSerializer, wire: &mut WireData) -> Result<()> {
    import_graphical_object(s, &mut wire.graphical)?;
    wire.line_width = s.import_size("LineWidth")?;
    wire.color = s.import_color("Color")?;
    wire.underline_color = s.import_color("UnderlineColor")?;
    wire.unique_id = s.import_dynamic_string("UniqueID")?;
    wire.vertices = import_vertices(s)?;
    wire.assigned_interface = s.import_dynamic_string("AssignedInterface")?;
    wire.assigned_interface_signal = s.import_dynamic_string("AssignedInterfaceSignal")?;
    Ok(())
}

// ============================================================================
// Bus (ObjectId = 26)
// ============================================================================

pub fn export_bus(s: &mut dyn SchSerializer, bus: &BusData) -> Result<()> {
    export_graphical_object(s, &bus.graphical)?;
    s.export_size(bus.line_width, "LineWidth")?;
    s.export_color(bus.color, "Color")?;
    s.export_color(bus.underline_color, "UnderlineColor")?;
    export_vertices(s, &bus.vertices)?;
    s.export_dynamic_string(&bus.unique_id, "UniqueID")?;
    s.export_dynamic_string(&bus.assigned_interface, "AssignedInterface")?;
    s.export_dynamic_string(&bus.assigned_interface_signal, "AssignedInterfaceSignal")?;
    Ok(())
}

pub fn import_bus(s: &mut dyn SchSerializer, bus: &mut BusData) -> Result<()> {
    import_graphical_object(s, &mut bus.graphical)?;
    bus.line_width = s.import_size("LineWidth")?;
    bus.color = s.import_color("Color")?;
    bus.underline_color = s.import_color("UnderlineColor")?;
    bus.vertices = import_vertices(s)?;
    bus.unique_id = s.import_dynamic_string("UniqueID")?;
    bus.assigned_interface = s.import_dynamic_string("AssignedInterface")?;
    bus.assigned_interface_signal = s.import_dynamic_string("AssignedInterfaceSignal")?;
    Ok(())
}

// ============================================================================
// Port (ObjectId = 17)
// ============================================================================

pub fn export_port(s: &mut dyn SchSerializer, port: &PortData) -> Result<()> {
    export_graphical_object(s, &port.graphical)?;
    s.export_port_arrow_style(port.style, "Style")?;
    s.export_port_io(port.io_type, "IOType")?;
    s.export_horizontal_align(port.alignment, "Alignment")?;
    s.export_coord(port.width, "Width")?;
    s.export_coord(port.location_x, "Location.X")?;
    s.export_coord(port.location_y, "Location.Y")?;
    s.export_color(port.color, "Color")?;
    s.export_font_id(port.font_id, "FontID")?;
    s.export_color(port.area_color, "AreaColor")?;
    s.export_color(port.text_color, "TextColor")?;
    s.export_dynamic_string(&port.name, "Name")?;
    s.export_dynamic_string(&port.harness_type, "HarnessType")?;
    s.export_dynamic_string(&port.unique_id, "UniqueID")?;
    s.export_coord(port.height, "Height")?;
    s.export_size(port.border_width, "BorderWidth")?;
    s.export_boolean(port.auto_size, "AutoSize")?;
    s.export_dynamic_string(&port.object_definition_id, "ObjectDefinitionId")?;
    s.export_boolean(!port.show_net_name, "PortNameIsHidden")?;
    Ok(())
}

pub fn import_port(s: &mut dyn SchSerializer, port: &mut PortData) -> Result<()> {
    import_graphical_object(s, &mut port.graphical)?;
    port.style = s.import_port_arrow_style("Style")?;
    port.io_type = s.import_port_io("IOType")?;
    port.alignment = s.import_horizontal_align("Alignment")?;
    port.width = s.import_coord("Width")?;
    port.location_x = s.import_coord("Location.X")?;
    port.location_y = s.import_coord("Location.Y")?;
    port.color = s.import_color("Color")?;
    port.area_color = s.import_color("AreaColor")?;
    port.text_color = s.import_color("TextColor")?;
    port.font_id = s.import_font_id("FontID")?;
    port.name = s.import_dynamic_string("Name")?;
    port.harness_type = s.import_dynamic_string("HarnessType")?;
    port.unique_id = s.import_dynamic_string("UniqueID")?;
    port.height = s.import_coord("Height")?;
    port.border_width = s.import_size("BorderWidth")?;
    port.auto_size = s.import_boolean("AutoSize")?;
    port.object_definition_id = s.import_dynamic_string("ObjectDefinitionId")?;
    let hidden = s.import_boolean("PortNameIsHidden")?;
    port.show_net_name = !hidden;
    Ok(())
}

// ============================================================================
// Power (ObjectId = 22)
// ============================================================================

pub fn export_power(s: &mut dyn SchSerializer, pwr: &PowerData) -> Result<()> {
    export_graphical_object(s, &pwr.graphical)?;
    s.export_power_object_style(pwr.style, "Style")?;
    s.export_boolean_with_default(pwr.show_net_name, "ShowNetName")?;
    s.export_coord(pwr.location_x, "Location.X")?;
    s.export_coord(pwr.location_y, "Location.Y")?;
    s.export_rotation_by90(pwr.orientation, "Orientation")?;
    s.export_color(pwr.color, "Color")?;
    if pwr.font_id != 0 {
        s.export_font_id(pwr.font_id, "FontID")?;
    }
    s.export_dynamic_string(&pwr.text, "Text")?;
    s.export_boolean(pwr.is_cross_sheet_connector, "IsCrossSheetConnector")?;
    s.export_dynamic_string(&pwr.unique_id, "UniqueID")?;
    s.export_dynamic_string(&pwr.object_definition_id, "ObjectDefinitionId")?;
    Ok(())
}

pub fn import_power(s: &mut dyn SchSerializer, pwr: &mut PowerData) -> Result<()> {
    import_graphical_object(s, &mut pwr.graphical)?;
    pwr.style = s.import_power_object_style("Style")?;
    pwr.show_net_name = s.import_boolean_with_default("ShowNetName", true)?;
    pwr.location_x = s.import_coord("Location.X")?;
    pwr.location_y = s.import_coord("Location.Y")?;
    pwr.orientation = s.import_rotation_by90("Orientation")?;
    pwr.color = s.import_color("Color")?;
    pwr.font_id = s.import_font_id("FontID")?;
    pwr.text = s.import_dynamic_string("Text")?;
    pwr.is_cross_sheet_connector = s.import_boolean("IsCrossSheetConnector")?;
    pwr.unique_id = s.import_dynamic_string("UniqueID")?;
    pwr.object_definition_id = s.import_dynamic_string("ObjectDefinitionId")?;
    Ok(())
}

// ============================================================================
// Parameter (ObjectId = 41)
// ============================================================================

pub fn export_parameter(s: &mut dyn SchSerializer, p: &ParameterData) -> Result<()> {
    export_graphical_object(s, &p.graphical)?;
    s.export_coord(p.location_x, "Location.X")?;
    s.export_coord(p.location_y, "Location.Y")?;
    s.export_rotation_by90(p.orientation, "Orientation")?;
    s.export_text_justification(p.justification, "Justification")?;
    s.export_color(p.color, "Color")?;
    s.export_font_id(p.font_id, "FontID")?;
    s.export_boolean(p.is_hidden, "IsHidden")?;
    s.export_dynamic_string(&p.text, "Text")?;
    s.export_parameter_kind(p.param_type, "ParamType")?;
    s.export_string(&p.name, "Name")?;
    s.export_boolean(p.show_name, "ShowName")?;
    s.export_parameter_read_only_state(p.read_only_state, "ReadOnlyState")?;
    s.export_dynamic_string(&p.unique_id, "UniqueID")?;
    s.export_dynamic_string(&p.description, "Description")?;
    s.export_boolean(!p.allow_library_synchronize, "NotAllowLibrarySynchronize")?;
    s.export_boolean(!p.allow_database_synchronize, "NotAllowDatabaseSynchronize")?;
    s.export_boolean(!p.auto_position, "NotAutoPosition")?;
    s.export_boolean(p.is_mirrored, "IsMirrored")?;
    s.export_text_horizontal_anchor(p.text_horz_anchor, "TextHorzAnchor")?;
    s.export_text_vertical_anchor(p.text_vert_anchor, "TextVertAnchor")?;
    s.export_boolean(p.is_image_parameter, "IsImageParameter")?;
    Ok(())
}

pub fn import_parameter(s: &mut dyn SchSerializer, p: &mut ParameterData) -> Result<()> {
    import_graphical_object(s, &mut p.graphical)?;
    p.location_x = s.import_coord("Location.X")?;
    p.location_y = s.import_coord("Location.Y")?;
    p.orientation = s.import_rotation_by90("Orientation")?;
    p.justification = s.import_text_justification("Justification")?;
    p.color = s.import_color("Color")?;
    p.font_id = s.import_font_id("FontID")?;
    p.is_hidden = s.import_boolean("IsHidden")?;
    p.text = s.import_dynamic_string("Text")?;
    p.param_type = s.import_parameter_kind("ParamType")?;
    p.name = s.import_string("Name")?;
    p.show_name = s.import_boolean("ShowName")?;
    p.read_only_state = s.import_parameter_read_only_state("ReadOnlyState")?;
    p.unique_id = s.import_dynamic_string("UniqueID")?;
    p.description = s.import_dynamic_string("Description")?;
    let not_allow_lib = s.import_boolean("NotAllowLibrarySynchronize")?;
    p.allow_library_synchronize = !not_allow_lib;
    let not_allow_db = s.import_boolean("NotAllowDatabaseSynchronize")?;
    p.allow_database_synchronize = !not_allow_db;
    let not_auto = s.import_boolean("NotAutoPosition")?;
    p.auto_position = !not_auto;
    p.is_mirrored = s.import_boolean("IsMirrored")?;
    p.text_horz_anchor = s.import_text_horizontal_anchor("TextHorzAnchor")?;
    p.text_vert_anchor = s.import_text_vertical_anchor("TextVertAnchor")?;
    p.is_image_parameter = s.import_boolean("IsImageParameter")?;
    Ok(())
}

// ============================================================================
// Designator (ObjectId = 34)
// ============================================================================

pub fn export_designator(s: &mut dyn SchSerializer, d: &DesignatorData) -> Result<()> {
    export_parameter(s, &d.param)?;
    if d.override_not_auto_position {
        s.export_boolean(true, "OverrideNotAutoPosition")?;
    }
    Ok(())
}

pub fn import_designator(s: &mut dyn SchSerializer, d: &mut DesignatorData) -> Result<()> {
    import_parameter(s, &mut d.param)?;
    d.override_not_auto_position = s.import_boolean("OverrideNotAutoPosition")?;
    if d.override_not_auto_position {
        d.param.auto_position = false;
    }
    Ok(())
}

// ============================================================================
// Image (ObjectId = 30)
// ============================================================================

pub fn export_image(s: &mut dyn SchSerializer, img: &ImageData) -> Result<()> {
    export_graphical_object(s, &img.graphical)?;
    s.export_coord(img.location_x, "Location.X")?;
    s.export_coord(img.location_y, "Location.Y")?;
    s.export_coord(img.corner_x, "Corner.X")?;
    s.export_coord(img.corner_y, "Corner.Y")?;
    s.export_rotation_by90(img.orientation, "Orientation")?;
    s.export_size(img.line_width, "LineWidth")?;
    s.export_color(img.color, "Color")?;
    s.export_boolean(img.is_solid, "IsSolid")?;
    s.export_boolean(img.keep_aspect, "KeepAspect")?;
    s.export_boolean(img.embed_image, "EmbedImage")?;
    s.export_dynamic_string(&img.file_name, "FileName")?;
    s.export_dynamic_string(&img.unique_id, "UniqueID")?;
    Ok(())
}

pub fn import_image(s: &mut dyn SchSerializer, img: &mut ImageData) -> Result<()> {
    import_graphical_object(s, &mut img.graphical)?;
    img.location_x = s.import_coord("Location.X")?;
    img.location_y = s.import_coord("Location.Y")?;
    img.corner_x = s.import_coord("Corner.X")?;
    img.corner_y = s.import_coord("Corner.Y")?;
    img.orientation = s.import_rotation_by90("Orientation")?;
    img.line_width = s.import_size("LineWidth")?;
    img.color = s.import_color("Color")?;
    img.is_solid = s.import_boolean("IsSolid")?;
    img.keep_aspect = s.import_boolean("KeepAspect")?;
    img.embed_image = s.import_boolean("EmbedImage")?;
    img.file_name = s.import_dynamic_string("FileName")?;
    img.unique_id = s.import_dynamic_string("UniqueID")?;
    Ok(())
}

// ============================================================================
// EllipticalArc (RECORD=8)
// ============================================================================

pub fn export_elliptical_arc(s: &mut dyn SchSerializer, ea: &EllipticalArcData) -> Result<()> {
    export_graphical_object(s, &ea.graphical)?;
    s.export_coord(ea.location_x, "Location.X")?;
    s.export_coord(ea.location_y, "Location.Y")?;
    s.export_coord(ea.radius, "Radius")?;
    s.export_coord(ea.secondary_radius, "SecondaryRadius")?;
    s.export_size(ea.line_width, "LineWidth")?;
    s.export_angle(ea.start_angle, "StartAngle")?;
    s.export_angle(ea.end_angle, "EndAngle")?;
    s.export_color(ea.color, "Color")?;
    s.export_dynamic_string(&ea.unique_id, "UniqueID")?;
    Ok(())
}

pub fn import_elliptical_arc(s: &mut dyn SchSerializer, ea: &mut EllipticalArcData) -> Result<()> {
    import_graphical_object(s, &mut ea.graphical)?;
    ea.location_x = s.import_coord("Location.X")?;
    ea.location_y = s.import_coord("Location.Y")?;
    ea.radius = s.import_coord("Radius")?;
    ea.secondary_radius = s.import_coord("SecondaryRadius")?;
    ea.line_width = s.import_size("LineWidth")?;
    ea.start_angle = s.import_angle("StartAngle")?;
    ea.end_angle = s.import_angle("EndAngle")?;
    ea.color = s.import_color("Color")?;
    ea.unique_id = s.import_dynamic_string("UniqueID")?;
    Ok(())
}

// ============================================================================
// Pie (RECORD=9)
// ============================================================================

pub fn export_pie(s: &mut dyn SchSerializer, pie: &PieData) -> Result<()> {
    export_graphical_object(s, &pie.graphical)?;
    s.export_coord(pie.location_x, "Location.X")?;
    s.export_coord(pie.location_y, "Location.Y")?;
    s.export_coord(pie.radius, "Radius")?;
    s.export_size(pie.line_width, "LineWidth")?;
    s.export_angle(pie.start_angle, "StartAngle")?;
    s.export_angle(pie.end_angle, "EndAngle")?;
    s.export_color(pie.color, "Color")?;
    s.export_color(pie.area_color, "AreaColor")?;
    s.export_boolean(pie.is_solid, "IsSolid")?;
    Ok(())
}

pub fn import_pie(s: &mut dyn SchSerializer, pie: &mut PieData) -> Result<()> {
    import_graphical_object(s, &mut pie.graphical)?;
    pie.location_x = s.import_coord("Location.X")?;
    pie.location_y = s.import_coord("Location.Y")?;
    pie.radius = s.import_coord("Radius")?;
    pie.line_width = s.import_size("LineWidth")?;
    pie.start_angle = s.import_angle("StartAngle")?;
    pie.end_angle = s.import_angle("EndAngle")?;
    pie.color = s.import_color("Color")?;
    pie.area_color = s.import_color("AreaColor")?;
    pie.is_solid = s.import_boolean("IsSolid")?;
    Ok(())
}

// ============================================================================
// Note (RECORD=2)
// ============================================================================

pub fn export_note(s: &mut dyn SchSerializer, n: &NoteData) -> Result<()> {
    export_graphical_object(s, &n.graphical)?;
    s.export_coord(n.location_x, "Location.X")?;
    s.export_coord(n.location_y, "Location.Y")?;
    s.export_coord(n.corner_x, "Corner.X")?;
    s.export_coord(n.corner_y, "Corner.Y")?;
    s.export_size(n.line_width, "LineWidth")?;
    s.export_color(n.color, "Color")?;
    s.export_color(n.area_color, "AreaColor")?;
    s.export_color(n.text_color, "TextColor")?;
    s.export_font_id(n.font_id, "FontID")?;
    s.export_boolean(n.is_solid, "IsSolid")?;
    s.export_boolean(n.show_border, "ShowBorder")?;
    s.export_horizontal_align(n.alignment, "Alignment")?;
    s.export_boolean(n.word_wrap, "WordWrap")?;
    s.export_boolean(n.clip_to_rect, "ClipToRect")?;
    s.export_text(&n.text, "Text")?;
    s.export_coord(n.text_margin, "TextMargin")?;
    s.export_boolean(n.collapsed, "Collapsed")?;
    s.export_dynamic_string(&n.author, "Author")?;
    s.export_dynamic_string(&n.unique_id, "UniqueID")?;
    Ok(())
}

pub fn import_note(s: &mut dyn SchSerializer, n: &mut NoteData) -> Result<()> {
    import_graphical_object(s, &mut n.graphical)?;
    n.location_x = s.import_coord("Location.X")?;
    n.location_y = s.import_coord("Location.Y")?;
    n.corner_x = s.import_coord("Corner.X")?;
    n.corner_y = s.import_coord("Corner.Y")?;
    n.line_width = s.import_size("LineWidth")?;
    n.color = s.import_color("Color")?;
    n.area_color = s.import_color("AreaColor")?;
    n.text_color = s.import_color("TextColor")?;
    n.font_id = s.import_font_id("FontID")?;
    n.is_solid = s.import_boolean("IsSolid")?;
    n.show_border = s.import_boolean("ShowBorder")?;
    n.alignment = s.import_horizontal_align("Alignment")?;
    n.word_wrap = s.import_boolean("WordWrap")?;
    n.clip_to_rect = s.import_boolean("ClipToRect")?;
    n.text = s.import_text("Text")?;
    n.text_margin = s.import_coord("TextMargin")?;
    n.collapsed = s.import_boolean("Collapsed")?;
    n.author = s.import_dynamic_string("Author")?;
    n.unique_id = s.import_dynamic_string("UniqueID")?;
    Ok(())
}

// ============================================================================
// TextFrame (RECORD=13)
// ============================================================================

pub fn export_text_frame(s: &mut dyn SchSerializer, tf: &TextFrameData) -> Result<()> {
    export_graphical_object(s, &tf.graphical)?;
    s.export_coord(tf.location_x, "Location.X")?;
    s.export_coord(tf.location_y, "Location.Y")?;
    s.export_coord(tf.corner_x, "Corner.X")?;
    s.export_coord(tf.corner_y, "Corner.Y")?;
    s.export_size(tf.line_width, "LineWidth")?;
    s.export_color(tf.color, "Color")?;
    s.export_color(tf.area_color, "AreaColor")?;
    s.export_color(tf.text_color, "TextColor")?;
    s.export_font_id(tf.font_id, "FontID")?;
    s.export_boolean(tf.is_solid, "IsSolid")?;
    s.export_boolean(tf.show_border, "ShowBorder")?;
    s.export_horizontal_align(tf.alignment, "Alignment")?;
    s.export_boolean(tf.word_wrap, "WordWrap")?;
    s.export_boolean(tf.clip_to_rect, "ClipToRect")?;
    s.export_text(&tf.text, "Text")?;
    s.export_coord(tf.text_margin, "TextMargin")?;
    s.export_dynamic_string(&tf.unique_id, "UniqueID")?;
    Ok(())
}

pub fn import_text_frame(s: &mut dyn SchSerializer, tf: &mut TextFrameData) -> Result<()> {
    import_graphical_object(s, &mut tf.graphical)?;
    tf.location_x = s.import_coord("Location.X")?;
    tf.location_y = s.import_coord("Location.Y")?;
    tf.corner_x = s.import_coord("Corner.X")?;
    tf.corner_y = s.import_coord("Corner.Y")?;
    tf.line_width = s.import_size("LineWidth")?;
    tf.color = s.import_color("Color")?;
    tf.area_color = s.import_color("AreaColor")?;
    tf.text_color = s.import_color("TextColor")?;
    tf.font_id = s.import_font_id("FontID")?;
    tf.is_solid = s.import_boolean("IsSolid")?;
    tf.show_border = s.import_boolean("ShowBorder")?;
    tf.alignment = s.import_horizontal_align("Alignment")?;
    tf.word_wrap = s.import_boolean("WordWrap")?;
    tf.clip_to_rect = s.import_boolean("ClipToRect")?;
    tf.text = s.import_text("Text")?;
    tf.text_margin = s.import_coord("TextMargin")?;
    tf.unique_id = s.import_dynamic_string("UniqueID")?;
    Ok(())
}

// ============================================================================
// BusEntry (RECORD=7)
// ============================================================================

pub fn export_bus_entry(s: &mut dyn SchSerializer, be: &BusEntryData) -> Result<()> {
    export_graphical_object(s, &be.graphical)?;
    s.export_dynamic_string(&be.unique_id, "UniqueID")?;
    s.export_coord(be.location_x, "Location.X")?;
    s.export_coord(be.location_y, "Location.Y")?;
    s.export_coord(be.corner_x, "Corner.X")?;
    s.export_coord(be.corner_y, "Corner.Y")?;
    s.export_size(be.line_width, "LineWidth")?;
    s.export_color(be.color, "Color")?;
    Ok(())
}

pub fn import_bus_entry(s: &mut dyn SchSerializer, be: &mut BusEntryData) -> Result<()> {
    import_graphical_object(s, &mut be.graphical)?;
    be.location_x = s.import_coord("Location.X")?;
    be.location_y = s.import_coord("Location.Y")?;
    be.corner_x = s.import_coord("Corner.X")?;
    be.corner_y = s.import_coord("Corner.Y")?;
    be.line_width = s.import_size("LineWidth")?;
    be.color = s.import_color("Color")?;
    be.unique_id = s.import_dynamic_string("UniqueID")?;
    Ok(())
}

// ============================================================================
// Rectangular entry container / Basic entry object helpers
// ============================================================================

pub fn export_rectangular_entry_container(s: &mut dyn SchSerializer, c: &RectangularEntryContainerBase) -> Result<()> {
    export_graphical_object(s, &c.graphical)?;
    s.export_coord(c.location_x, "Location.X")?;
    s.export_coord(c.location_y, "Location.Y")?;
    s.export_coord(c.x_size, "XSize")?;
    s.export_coord(c.y_size, "YSize")?;
    s.export_size(c.line_width, "LineWidth")?;
    s.export_color(c.color, "Color")?;
    s.export_color(c.area_color, "AreaColor")?;
    Ok(())
}

pub fn import_rectangular_entry_container(s: &mut dyn SchSerializer, c: &mut RectangularEntryContainerBase) -> Result<()> {
    import_graphical_object(s, &mut c.graphical)?;
    c.location_x = s.import_coord("Location.X")?;
    c.location_y = s.import_coord("Location.Y")?;
    c.x_size = s.import_coord("XSize")?;
    c.y_size = s.import_coord("YSize")?;
    c.line_width = s.import_size("LineWidth")?;
    c.color = s.import_color("Color")?;
    c.area_color = s.import_color("AreaColor")?;
    Ok(())
}

pub fn export_basic_entry_object(s: &mut dyn SchSerializer, e: &BasicEntryObjectBase) -> Result<()> {
    export_graphical_object(s, &e.graphical)?;
    s.export_left_right_side(e.side, "Side")?;
    s.export_coord(e.distance_from_top, "DistanceFromTop")?;
    s.export_color(e.color, "Color")?;
    s.export_color(e.area_color, "AreaColor")?;
    s.export_color(e.text_color, "TextColor")?;
    s.export_font_id(e.text_font_id, "TextFontID")?;
    s.export_dynamic_string(&e.text_style, "TextStyle")?;
    s.export_dynamic_string(&e.name, "Name")?;
    s.export_dynamic_string(&e.harness_type, "HarnessType")?;
    s.export_dynamic_string(&e.unique_id, "UniqueID")?;
    Ok(())
}

pub fn import_basic_entry_object(s: &mut dyn SchSerializer, e: &mut BasicEntryObjectBase) -> Result<()> {
    import_graphical_object(s, &mut e.graphical)?;
    e.side = s.import_left_right_side("Side")?;
    e.distance_from_top = s.import_coord("DistanceFromTop")?;
    e.color = s.import_color("Color")?;
    e.area_color = s.import_color("AreaColor")?;
    e.text_color = s.import_color("TextColor")?;
    e.text_font_id = s.import_font_id("TextFontID")?;
    e.text_style = s.import_dynamic_string("TextStyle")?;
    e.name = s.import_dynamic_string("Name")?;
    e.harness_type = s.import_dynamic_string("HarnessType")?;
    e.unique_id = s.import_dynamic_string("UniqueID")?;
    Ok(())
}

// ============================================================================
// SheetSymbol (RECORD=15)
// ============================================================================

pub fn export_sheet_symbol(s: &mut dyn SchSerializer, ss: &SheetSymbolData) -> Result<()> {
    export_rectangular_entry_container(s, &ss.container)?;
    s.export_boolean(ss.is_solid, "IsSolid")?;
    s.export_boolean(ss.show_hidden_fields, "ShowHiddenFields")?;
    s.export_string(&ss.unique_id, "UniqueID")?;
    s.export_dynamic_string(&ss.symbol_type, "SymbolType")?;
    s.export_dynamic_string(&ss.design_item_id, "DesignItemId")?;
    s.export_dynamic_string(&ss.source_library_name, "SourceLibraryName")?;
    s.export_dynamic_string(&ss.vault_guid, "VaultGUID")?;
    s.export_dynamic_string(&ss.item_guid, "ItemGUID")?;
    s.export_dynamic_string(&ss.revision_guid, "RevisionGUID")?;
    s.export_dynamic_string(&ss.revision_name, "RevisionName")?;
    Ok(())
}

pub fn import_sheet_symbol(s: &mut dyn SchSerializer, ss: &mut SheetSymbolData) -> Result<()> {
    import_rectangular_entry_container(s, &mut ss.container)?;
    ss.is_solid = s.import_boolean("IsSolid")?;
    ss.show_hidden_fields = s.import_boolean("ShowHiddenFields")?;
    ss.unique_id = s.import_string("UniqueID")?;
    ss.symbol_type = s.import_dynamic_string("SymbolType")?;
    ss.design_item_id = s.import_dynamic_string("DesignItemId")?;
    ss.source_library_name = s.import_dynamic_string("SourceLibraryName")?;
    ss.vault_guid = s.import_dynamic_string("VaultGUID")?;
    ss.item_guid = s.import_dynamic_string("ItemGUID")?;
    ss.revision_guid = s.import_dynamic_string("RevisionGUID")?;
    ss.revision_name = s.import_dynamic_string("RevisionName")?;
    Ok(())
}

// ============================================================================
// SheetEntry (RECORD=16)
// ============================================================================

pub fn export_sheet_entry(s: &mut dyn SchSerializer, se: &SheetEntryData) -> Result<()> {
    export_basic_entry_object(s, &se.entry)?;
    s.export_port_io(se.io_type, "IOType")?;
    s.export_port_arrow_style(se.style, "Style")?;
    s.export_dynamic_string(&se.arrow_kind, "ArrowKind")?;
    Ok(())
}

pub fn import_sheet_entry(s: &mut dyn SchSerializer, se: &mut SheetEntryData) -> Result<()> {
    import_basic_entry_object(s, &mut se.entry)?;
    se.io_type = s.import_port_io("IOType")?;
    se.style = s.import_port_arrow_style("Style")?;
    se.arrow_kind = s.import_dynamic_string("ArrowKind")?;
    Ok(())
}

// ============================================================================
// Probe (RECORD=3)
// ============================================================================

pub fn export_probe(s: &mut dyn SchSerializer, p: &ProbeData) -> Result<()> {
    export_graphical_object(s, &p.graphical)?;
    s.export_coord(p.location_x, "Location.X")?;
    s.export_coord(p.location_y, "Location.Y")?;
    s.export_color(p.color, "Color")?;
    s.export_rotation_by90(p.orientation, "Orientation")?;
    s.export_dynamic_string(&p.name, "Name")?;
    s.export_dynamic_string(&p.unique_id, "UniqueID")?;
    Ok(())
}

pub fn import_probe(s: &mut dyn SchSerializer, p: &mut ProbeData) -> Result<()> {
    import_graphical_object(s, &mut p.graphical)?;
    p.location_x = s.import_coord("Location.X")?;
    p.location_y = s.import_coord("Location.Y")?;
    p.color = s.import_color("Color")?;
    p.orientation = s.import_rotation_by90("Orientation")?;
    p.name = s.import_dynamic_string("Name")?;
    p.unique_id = s.import_dynamic_string("UniqueID")?;
    Ok(())
}

// ============================================================================
// NoERC (RECORD=24)
// ============================================================================

pub fn export_no_erc(s: &mut dyn SchSerializer, n: &NoERCData) -> Result<()> {
    export_graphical_object(s, &n.graphical)?;
    s.export_coord(n.location_x, "Location.X")?;
    s.export_coord(n.location_y, "Location.Y")?;
    s.export_color(n.color, "Color")?;
    s.export_rotation_by90(n.orientation, "Orientation")?;
    s.export_no_erc_symbol(n.symbol, "Symbol")?;
    s.export_boolean_with_default(n.is_active, "IsActive")?;
    s.export_boolean_with_default(n.suppress_all, "SuppressAll")?;
    s.export_dynamic_string(&n.unique_id, "UniqueID")?;
    Ok(())
}

pub fn import_no_erc(s: &mut dyn SchSerializer, n: &mut NoERCData) -> Result<()> {
    import_graphical_object(s, &mut n.graphical)?;
    n.location_x = s.import_coord("Location.X")?;
    n.location_y = s.import_coord("Location.Y")?;
    n.color = s.import_color("Color")?;
    n.orientation = s.import_rotation_by90("Orientation")?;
    n.symbol = s.import_no_erc_symbol("Symbol")?;
    n.is_active = s.import_boolean_with_default("IsActive", true)?;
    n.suppress_all = s.import_boolean_with_default("SuppressAll", true)?;
    n.unique_id = s.import_dynamic_string("UniqueID")?;
    Ok(())
}

// ============================================================================
// RoundRectangle (RECORD=10)
// ============================================================================

pub fn export_round_rectangle(s: &mut dyn SchSerializer, rr: &RoundRectangleData) -> Result<()> {
    export_graphical_object(s, &rr.graphical)?;
    s.export_coord(rr.location_x, "Location.X")?;
    s.export_coord(rr.location_y, "Location.Y")?;
    s.export_coord(rr.corner_x, "Corner.X")?;
    s.export_coord(rr.corner_y, "Corner.Y")?;
    s.export_coord(rr.corner_x_radius, "CornerXRadius")?;
    s.export_coord(rr.corner_y_radius, "CornerYRadius")?;
    s.export_size(rr.line_width, "LineWidth")?;
    s.export_color(rr.color, "Color")?;
    s.export_color(rr.area_color, "AreaColor")?;
    s.export_boolean(rr.is_solid, "IsSolid")?;
    s.export_dynamic_string(&rr.unique_id, "UniqueID")?;
    Ok(())
}

pub fn import_round_rectangle(s: &mut dyn SchSerializer, rr: &mut RoundRectangleData) -> Result<()> {
    import_graphical_object(s, &mut rr.graphical)?;
    rr.location_x = s.import_coord("Location.X")?;
    rr.location_y = s.import_coord("Location.Y")?;
    rr.corner_x = s.import_coord("Corner.X")?;
    rr.corner_y = s.import_coord("Corner.Y")?;
    rr.corner_x_radius = s.import_coord("CornerXRadius")?;
    rr.corner_y_radius = s.import_coord("CornerYRadius")?;
    rr.line_width = s.import_size("LineWidth")?;
    rr.color = s.import_color("Color")?;
    rr.area_color = s.import_color("AreaColor")?;
    rr.is_solid = s.import_boolean("IsSolid")?;
    rr.unique_id = s.import_dynamic_string("UniqueID")?;
    Ok(())
}

// ============================================================================
// CompileMask (RECORD=28)
// ============================================================================

pub fn export_compile_mask(s: &mut dyn SchSerializer, cm: &CompileMaskData) -> Result<()> {
    export_graphical_object(s, &cm.graphical)?;
    s.export_dynamic_string(&cm.unique_id, "UniqueID")?;
    s.export_coord(cm.location_x, "Location.X")?;
    s.export_coord(cm.location_y, "Location.Y")?;
    s.export_coord(cm.corner_x, "Corner.X")?;
    s.export_coord(cm.corner_y, "Corner.Y")?;
    s.export_color(cm.color, "Color")?;
    s.export_color(cm.area_color, "AreaColor")?;
    s.export_boolean(cm.collapsed, "Collapsed")?;
    s.export_size(cm.line_width, "LineWidth")?;
    Ok(())
}

pub fn import_compile_mask(s: &mut dyn SchSerializer, cm: &mut CompileMaskData) -> Result<()> {
    import_graphical_object(s, &mut cm.graphical)?;
    cm.location_x = s.import_coord("Location.X")?;
    cm.location_y = s.import_coord("Location.Y")?;
    cm.corner_x = s.import_coord("Corner.X")?;
    cm.corner_y = s.import_coord("Corner.Y")?;
    cm.color = s.import_color("Color")?;
    cm.area_color = s.import_color("AreaColor")?;
    cm.collapsed = s.import_boolean("Collapsed")?;
    cm.unique_id = s.import_dynamic_string("UniqueID")?;
    cm.line_width = s.import_size("LineWidth")?;
    Ok(())
}

// ============================================================================
// Blanket (RECORD=29)
// ============================================================================

pub fn export_blanket(s: &mut dyn SchSerializer, b: &BlanketData) -> Result<()> {
    export_graphical_object(s, &b.graphical)?;
    s.export_coord(b.location_x, "Location.X")?;
    s.export_coord(b.location_y, "Location.Y")?;
    s.export_coord(b.corner_x, "Corner.X")?;
    s.export_coord(b.corner_y, "Corner.Y")?;
    s.export_size(b.line_width, "LineWidth")?;
    s.export_color(b.color, "Color")?;
    s.export_color(b.area_color, "AreaColor")?;
    s.export_boolean(b.collapsed, "Collapsed")?;
    // Export clamped line_style for backward compat, then extended via ASCII-only
    let clamped = if (b.line_style as u8) > (LineStyle::DashDotted as u8) {
        LineStyle::Solid
    } else {
        b.line_style
    };
    s.export_line_style(clamped, "LineStyle")?;
    export_vertices(s, &b.vertices)?;
    // LineStyleExt — ASCII-only byte for extended line styles
    s.export_ascii_only_byte(b.line_style as u8, "LineStyleExt")?;
    s.export_dynamic_string(&b.unique_id, "UniqueID")?;
    Ok(())
}

pub fn import_blanket(s: &mut dyn SchSerializer, b: &mut BlanketData) -> Result<()> {
    import_graphical_object(s, &mut b.graphical)?;
    b.location_x = s.import_coord("Location.X")?;
    b.location_y = s.import_coord("Location.Y")?;
    b.corner_x = s.import_coord("Corner.X")?;
    b.corner_y = s.import_coord("Corner.Y")?;
    b.line_width = s.import_size("LineWidth")?;
    b.color = s.import_color("Color")?;
    b.area_color = s.import_color("AreaColor")?;
    b.collapsed = s.import_boolean("Collapsed")?;
    b.line_style = s.import_line_style("LineStyle")?;
    b.unique_id = s.import_dynamic_string("UniqueID")?;
    b.vertices = import_vertices(s)?;
    // LineStyleExt overrides if present and larger
    let ext = s.import_ascii_only_byte("LineStyleExt")?;
    if ext > b.line_style as u8 {
        b.line_style = LineStyle::from_u8(ext).unwrap_or(b.line_style);
    }
    Ok(())
}

// ============================================================================
// SignalHarness (RECORD=33)
// ============================================================================

pub fn export_signal_harness(s: &mut dyn SchSerializer, sh: &SignalHarnessData) -> Result<()> {
    export_graphical_object(s, &sh.graphical)?;
    s.export_size(sh.line_width, "LineWidth")?;
    s.export_color(sh.color, "Color")?;
    s.export_color(sh.underline_color, "UnderlineColor")?;
    export_vertices(s, &sh.vertices)?;
    s.export_dynamic_string(&sh.unique_id, "UniqueID")?;
    s.export_dynamic_string(&sh.assigned_interface, "AssignedInterface")?;
    s.export_dynamic_string(&sh.assigned_interface_signal, "AssignedInterfaceSignal")?;
    Ok(())
}

pub fn import_signal_harness(s: &mut dyn SchSerializer, sh: &mut SignalHarnessData) -> Result<()> {
    import_graphical_object(s, &mut sh.graphical)?;
    sh.line_width = s.import_size("LineWidth")?;
    sh.color = s.import_color("Color")?;
    sh.underline_color = s.import_color("UnderlineColor")?;
    sh.vertices = import_vertices(s)?;
    sh.unique_id = s.import_dynamic_string("UniqueID")?;
    sh.assigned_interface = s.import_dynamic_string("AssignedInterface")?;
    sh.assigned_interface_signal = s.import_dynamic_string("AssignedInterfaceSignal")?;
    Ok(())
}

// ============================================================================
// Symbol (RECORD=34)
// ============================================================================

pub fn export_symbol(s: &mut dyn SchSerializer, sym: &SymbolData) -> Result<()> {
    export_graphical_object(s, &sym.graphical)?;
    s.export_ieee_symbol(sym.symbol, "Symbol")?;
    s.export_coord(sym.location_x, "Location.X")?;
    s.export_coord(sym.location_y, "Location.Y")?;
    s.export_coord(sym.scale_factor, "ScaleFactor")?;
    s.export_rotation_by90(sym.orientation, "Orientation")?;
    s.export_size(sym.line_width, "LineWidth")?;
    s.export_color(sym.color, "Color")?;
    s.export_boolean(sym.is_mirrored, "Mirror")?;
    Ok(())
}

pub fn import_symbol(s: &mut dyn SchSerializer, sym: &mut SymbolData) -> Result<()> {
    import_graphical_object(s, &mut sym.graphical)?;
    sym.symbol = s.import_ieee_symbol("Symbol")?;
    sym.location_x = s.import_coord("Location.X")?;
    sym.location_y = s.import_coord("Location.Y")?;
    sym.scale_factor = s.import_coord("ScaleFactor")?;
    sym.orientation = s.import_rotation_by90("Orientation")?;
    sym.line_width = s.import_size("LineWidth")?;
    sym.color = s.import_color("Color")?;
    sym.is_mirrored = s.import_boolean("Mirror")?;
    Ok(())
}

// ============================================================================
// Implementation (RECORD=46)
// ============================================================================

pub fn export_implementation(s: &mut dyn SchSerializer, imp: &ImplementationData) -> Result<()> {
    export_data_object(s, &imp.base)?;
    s.export_dynamic_string(&imp.description, "Description")?;
    s.export_boolean(imp.use_component_library, "UseComponentLibrary")?;
    s.export_string(&imp.model_name, "ModelName")?;
    s.export_string(&imp.model_type, "ModelType")?;
    s.export_short_int(imp.datafile_links.len() as i32, "DatafileCount")?;
    s.export_dynamic_string(&imp.model_vault_guid, "ModelVaultGUID")?;
    s.export_dynamic_string(&imp.model_item_guid, "ModelItemGUID")?;
    s.export_dynamic_string(&imp.model_revision_guid, "ModelRevisionGUID")?;
    for (i, (location, entity, kind)) in imp.datafile_links.iter().enumerate() {
        let idx = i.to_string();
        s.export_dynamic_string(location, &format!("ModelDatafile{}", idx))?;
        s.export_dynamic_string(entity, &format!("ModelDatafileEntity{}", idx))?;
        s.export_dynamic_string(kind, &format!("ModelDatafileKind{}", idx))?;
    }
    s.export_boolean(imp.is_current, "IsCurrent")?;
    s.export_boolean(imp.use_component_library, "DatalinksLocked")?;
    s.export_boolean(imp.use_component_library, "DatabaseDatalinksLocked")?;
    s.export_boolean(imp.integrated_model, "IntegratedModel")?;
    s.export_boolean(imp.database_model, "DatabaseModel")?;
    s.export_dynamic_string(&imp.unique_id, "UniqueID")?;
    Ok(())
}

pub fn import_implementation(s: &mut dyn SchSerializer, imp: &mut ImplementationData) -> Result<()> {
    import_data_object(s, &mut imp.base)?;
    imp.description = s.import_dynamic_string("Description")?;
    let use_comp_lib = s.import_boolean("UseComponentLibrary")?;
    imp.model_name = s.import_string("ModelName")?;
    imp.model_type = s.import_string("ModelType")?;
    imp.is_current = s.import_boolean("IsCurrent")?;
    let datalinks_locked = s.import_boolean("DatalinksLocked")?;
    let db_datalinks_locked = s.import_boolean("DatabaseDatalinksLocked")?;
    imp.use_component_library = use_comp_lib || datalinks_locked || db_datalinks_locked;
    imp.integrated_model = s.import_boolean("IntegratedModel")?;
    imp.database_model = s.import_boolean("DatabaseModel")?;
    imp.model_vault_guid = s.import_dynamic_string("ModelVaultGUID")?;
    imp.model_item_guid = s.import_dynamic_string("ModelItemGUID")?;
    imp.model_revision_guid = s.import_dynamic_string("ModelRevisionGUID")?;
    imp.unique_id = s.import_dynamic_string("UniqueID")?;
    let count = s.import_short_int("DatafileCount")?;
    imp.datafile_links.clear();
    for i in 0..count {
        let idx = i.to_string();
        let location = s.import_dynamic_string(&format!("ModelDatafile{}", idx))?;
        let entity = s.import_dynamic_string(&format!("ModelDatafileEntity{}", idx))?;
        let kind = s.import_dynamic_string(&format!("ModelDatafileKind{}", idx))?;
        imp.datafile_links.push((location, entity, kind));
    }
    Ok(())
}

// ============================================================================
// ImplementationList (RECORD=47) — just graphical object wrapper
// ============================================================================

pub fn export_implementation_list(s: &mut dyn SchSerializer, il: &ImplementationListData) -> Result<()> {
    export_graphical_object(s, &il.graphical)
}

pub fn import_implementation_list(s: &mut dyn SchSerializer, il: &mut ImplementationListData) -> Result<()> {
    import_graphical_object(s, &mut il.graphical)
}

// ============================================================================
// ParameterList (RECORD=29) — just graphical object wrapper
// ============================================================================

pub fn export_parameter_list(s: &mut dyn SchSerializer, pl: &ParameterListData) -> Result<()> {
    export_graphical_object(s, &pl.graphical)
}

pub fn import_parameter_list(s: &mut dyn SchSerializer, pl: &mut ParameterListData) -> Result<()> {
    import_graphical_object(s, &mut pl.graphical)
}

// ============================================================================
// ParameterSet (RECORD=28)
// ============================================================================

pub fn export_parameter_set(s: &mut dyn SchSerializer, ps: &ParameterSetData) -> Result<()> {
    export_graphical_object(s, &ps.graphical)?;
    s.export_coord(ps.location_x, "Location.X")?;
    s.export_coord(ps.location_y, "Location.Y")?;
    s.export_color(ps.color, "Color")?;
    s.export_rotation_by90(ps.orientation, "Orientation")?;
    s.export_dynamic_string(&ps.name, "Name")?;
    s.export_parameter_set_style(ps.style, "Style")?;
    s.export_dynamic_string(&ps.unique_id, "UniqueID")?;
    Ok(())
}

pub fn import_parameter_set(s: &mut dyn SchSerializer, ps: &mut ParameterSetData) -> Result<()> {
    import_graphical_object(s, &mut ps.graphical)?;
    ps.location_x = s.import_coord("Location.X")?;
    ps.location_y = s.import_coord("Location.Y")?;
    ps.color = s.import_color("Color")?;
    ps.orientation = s.import_rotation_by90("Orientation")?;
    ps.name = s.import_dynamic_string("Name")?;
    ps.style = s.import_parameter_set_style("Style")?;
    ps.unique_id = s.import_dynamic_string("UniqueID")?;
    Ok(())
}

// ============================================================================
// MapDefiner (RECORD=44)
// ============================================================================

pub fn export_map_definer(s: &mut dyn SchSerializer, md: &MapDefinerData) -> Result<()> {
    export_data_object(s, &md.base)?;
    s.export_string(&md.designator_interface, "DesIntf")?;
    s.export_long_int(md.implementation_designators.len() as i32, "DesImpCount")?;
    for (i, des) in md.implementation_designators.iter().enumerate() {
        s.export_string(des, &format!("DesImp{}", i))?;
    }
    Ok(())
}

pub fn import_map_definer(s: &mut dyn SchSerializer, md: &mut MapDefinerData) -> Result<()> {
    import_data_object(s, &mut md.base)?;
    md.designator_interface = s.import_dynamic_string("DesIntf")?;
    let count = s.import_long_int("DesImpCount")?;
    md.implementation_designators.clear();
    for i in 0..count {
        let des = s.import_string(&format!("DesImp{}", i))?;
        md.implementation_designators.push(des);
    }
    Ok(())
}

// ============================================================================
// ImplementationMap (RECORD=45) — just data object wrapper
// ============================================================================

pub fn export_implementation_map(s: &mut dyn SchSerializer, im: &ImplementationMapData) -> Result<()> {
    export_data_object(s, &im.base)
}

pub fn import_implementation_map(s: &mut dyn SchSerializer, im: &mut ImplementationMapData) -> Result<()> {
    import_data_object(s, &mut im.base)
}

// ============================================================================
// Template (RECORD=42)
// ============================================================================

pub fn export_template(s: &mut dyn SchSerializer, t: &TemplateData) -> Result<()> {
    export_graphical_object(s, &t.graphical)?;
    s.export_string(&t.file_name, "FileName")?;
    Ok(())
}

pub fn import_template(s: &mut dyn SchSerializer, t: &mut TemplateData) -> Result<()> {
    import_graphical_object(s, &mut t.graphical)?;
    t.file_name = s.import_string("FileName")?;
    Ok(())
}

// ============================================================================
// HarnessConnectorType (RECORD=55)
// ============================================================================

pub fn export_harness_connector_type(s: &mut dyn SchSerializer, h: &HarnessConnectorTypeData) -> Result<()> {
    export_graphical_object(s, &h.graphical)?;
    s.export_coord(h.location_x, "Location.X")?;
    s.export_coord(h.location_y, "Location.Y")?;
    s.export_rotation_by90(h.orientation, "Orientation")?;
    s.export_text_justification(h.justification, "Justification")?;
    s.export_color(h.color, "Color")?;
    s.export_font_id(h.font_id, "FontID")?;
    s.export_boolean(h.is_hidden, "IsHidden")?;
    s.export_dynamic_string(&h.text, "Text")?;
    s.export_boolean(h.is_mirrored, "IsMirrored")?;
    s.export_boolean(!h.auto_position, "NotAutoPosition")?;
    s.export_text_horizontal_anchor(h.text_horz_anchor, "TextHorzAnchor")?;
    s.export_text_vertical_anchor(h.text_vert_anchor, "TextVertAnchor")?;
    s.export_dynamic_string(&h.unique_id, "UniqueID")?;
    Ok(())
}

pub fn import_harness_connector_type(s: &mut dyn SchSerializer, h: &mut HarnessConnectorTypeData) -> Result<()> {
    import_graphical_object(s, &mut h.graphical)?;
    h.location_x = s.import_coord("Location.X")?;
    h.location_y = s.import_coord("Location.Y")?;
    h.orientation = s.import_rotation_by90("Orientation")?;
    h.justification = s.import_text_justification("Justification")?;
    h.color = s.import_color("Color")?;
    h.font_id = s.import_font_id("FontID")?;
    h.is_hidden = s.import_boolean("IsHidden")?;
    h.text = s.import_dynamic_string("Text")?;
    h.is_mirrored = s.import_boolean("IsMirrored")?;
    let not_auto = s.import_boolean("NotAutoPosition")?;
    h.auto_position = !not_auto;
    h.text_horz_anchor = s.import_text_horizontal_anchor("TextHorzAnchor")?;
    h.text_vert_anchor = s.import_text_vertical_anchor("TextVertAnchor")?;
    h.unique_id = s.import_dynamic_string("UniqueID")?;
    Ok(())
}

// ============================================================================
// SheetName (RECORD=30)
// ============================================================================

pub fn export_sheet_name(s: &mut dyn SchSerializer, sn: &SheetNameData) -> Result<()> {
    export_graphical_object(s, &sn.graphical)?;
    s.export_coord(sn.location_x, "Location.X")?;
    s.export_coord(sn.location_y, "Location.Y")?;
    s.export_rotation_by90(sn.orientation, "Orientation")?;
    s.export_text_justification(sn.justification, "Justification")?;
    s.export_color(sn.color, "Color")?;
    s.export_font_id(sn.font_id, "FontID")?;
    s.export_boolean(sn.is_hidden, "IsHidden")?;
    s.export_dynamic_string(&sn.text, "Text")?;
    s.export_boolean(sn.is_mirrored, "IsMirrored")?;
    s.export_boolean(!sn.auto_position, "NotAutoPosition")?;
    s.export_text_horizontal_anchor(sn.text_horz_anchor, "TextHorzAnchor")?;
    s.export_text_vertical_anchor(sn.text_vert_anchor, "TextVertAnchor")?;
    s.export_dynamic_string(&sn.unique_id, "UniqueID")?;
    Ok(())
}

pub fn import_sheet_name(s: &mut dyn SchSerializer, sn: &mut SheetNameData) -> Result<()> {
    import_graphical_object(s, &mut sn.graphical)?;
    sn.location_x = s.import_coord("Location.X")?;
    sn.location_y = s.import_coord("Location.Y")?;
    sn.orientation = s.import_rotation_by90("Orientation")?;
    sn.justification = s.import_text_justification("Justification")?;
    sn.color = s.import_color("Color")?;
    sn.font_id = s.import_font_id("FontID")?;
    sn.is_hidden = s.import_boolean("IsHidden")?;
    sn.text = s.import_dynamic_string("Text")?;
    sn.is_mirrored = s.import_boolean("IsMirrored")?;
    let not_auto = s.import_boolean("NotAutoPosition")?;
    sn.auto_position = !not_auto;
    sn.text_horz_anchor = s.import_text_horizontal_anchor("TextHorzAnchor")?;
    sn.text_vert_anchor = s.import_text_vertical_anchor("TextVertAnchor")?;
    sn.unique_id = s.import_dynamic_string("UniqueID")?;
    Ok(())
}

// ============================================================================
// SheetFileName (RECORD=31)
// ============================================================================

pub fn export_sheet_file_name(s: &mut dyn SchSerializer, sf: &SheetFileNameData) -> Result<()> {
    export_graphical_object(s, &sf.graphical)?;
    s.export_coord(sf.location_x, "Location.X")?;
    s.export_coord(sf.location_y, "Location.Y")?;
    s.export_rotation_by90(sf.orientation, "Orientation")?;
    s.export_text_justification(sf.justification, "Justification")?;
    s.export_color(sf.color, "Color")?;
    s.export_font_id(sf.font_id, "FontID")?;
    s.export_boolean(sf.is_hidden, "IsHidden")?;
    s.export_dynamic_string(&sf.text, "Text")?;
    s.export_boolean(sf.is_mirrored, "IsMirrored")?;
    s.export_boolean(!sf.auto_position, "NotAutoPosition")?;
    s.export_text_horizontal_anchor(sf.text_horz_anchor, "TextHorzAnchor")?;
    s.export_text_vertical_anchor(sf.text_vert_anchor, "TextVertAnchor")?;
    s.export_dynamic_string(&sf.unique_id, "UniqueID")?;
    Ok(())
}

pub fn import_sheet_file_name(s: &mut dyn SchSerializer, sf: &mut SheetFileNameData) -> Result<()> {
    import_graphical_object(s, &mut sf.graphical)?;
    sf.location_x = s.import_coord("Location.X")?;
    sf.location_y = s.import_coord("Location.Y")?;
    sf.orientation = s.import_rotation_by90("Orientation")?;
    sf.justification = s.import_text_justification("Justification")?;
    sf.color = s.import_color("Color")?;
    sf.font_id = s.import_font_id("FontID")?;
    sf.is_hidden = s.import_boolean("IsHidden")?;
    sf.text = s.import_dynamic_string("Text")?;
    sf.is_mirrored = s.import_boolean("IsMirrored")?;
    let not_auto = s.import_boolean("NotAutoPosition")?;
    sf.auto_position = !not_auto;
    sf.text_horz_anchor = s.import_text_horizontal_anchor("TextHorzAnchor")?;
    sf.text_vert_anchor = s.import_text_vertical_anchor("TextVertAnchor")?;
    sf.unique_id = s.import_dynamic_string("UniqueID")?;
    Ok(())
}

// ============================================================================
// Vertex helpers
// ============================================================================

/// Export vertex list — Altium uses LocationCount + Location.X_n/Location.Y_n pattern.
fn export_vertices(s: &mut dyn SchSerializer, verts: &[(i32, i32)]) -> Result<()> {
    s.export_short_int(verts.len() as i32, "LocationCount")?;
    for (i, (x, y)) in verts.iter().enumerate() {
        let n = i + 1;
        s.export_coord(*x, &format!("X{}", n))?;
        s.export_coord(*y, &format!("Y{}", n))?;
    }
    Ok(())
}

/// Import vertex list.
fn import_vertices(s: &mut dyn SchSerializer) -> Result<Vec<(i32, i32)>> {
    let count = s.import_short_int("LocationCount")?;
    let mut verts = Vec::with_capacity(count as usize);
    for i in 0..count {
        let n = i + 1;
        let x = s.import_coord(&format!("X{}", n))?;
        let y = s.import_coord(&format!("Y{}", n))?;
        verts.push((x, y));
    }
    Ok(verts)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::serializer::ascii::AsciiSerializer;

    #[test]
    fn pin_round_trip_ascii() {
        let mut pin = PinData::default();
        pin.owner_index = 1;
        pin.owner_part_id = 1;
        pin.name = "VCC".to_string();
        pin.designator = "1".to_string();
        pin.electrical = PinElectrical::Passive;
        pin.pin_length = 200_000; // 2 mils
        pin.location_x = 500_000;
        pin.location_y = 300_000;
        pin.color = 0x00FF00;
        pin.is_hidden = false;
        pin.show_name = true;
        pin.show_designator = true;
        pin.is_accessible = true;
        pin.swap_id_pin = "0".to_string();
        pin.swap_id_part = "0".to_string();

        let mut w = AsciiSerializer::new_writer();
        export_pin(&mut w, &pin).unwrap();
        let params = w.to_param_string();

        let mut r = AsciiSerializer::from_params(&params);
        let mut pin2 = PinData::default();
        import_pin(&mut r, &mut pin2).unwrap();

        assert_eq!(pin2.owner_index, 1);
        assert_eq!(pin2.name, "VCC");
        assert_eq!(pin2.designator, "1");
        assert_eq!(pin2.electrical, PinElectrical::Passive);
        assert_eq!(pin2.pin_length, 200_000);
        assert_eq!(pin2.location_x, 500_000);
        assert_eq!(pin2.location_y, 300_000);
        assert_eq!(pin2.show_name, true);
        assert_eq!(pin2.show_designator, true);
        assert_eq!(pin2.is_accessible, true);
    }

    #[test]
    fn pin_conglomerate_packing() {
        let mut pin = PinData::default();
        pin.orientation = RotationBy90::Rotate90;
        pin.is_hidden = true;
        pin.show_name = true;
        pin.show_designator = false;
        pin.is_accessible = false;
        pin.owner_index_additional_list = true;

        let mut w = AsciiSerializer::new_writer();
        export_pin(&mut w, &pin).unwrap();
        let params = w.to_param_string();

        let mut r = AsciiSerializer::from_params(&params);
        let mut pin2 = PinData::default();
        import_pin(&mut r, &mut pin2).unwrap();

        assert_eq!(pin2.orientation, RotationBy90::Rotate90);
        assert!(pin2.is_hidden);
        assert!(pin2.show_name);
        assert!(!pin2.show_designator);
        assert!(!pin2.is_accessible);
        assert!(pin2.owner_index_additional_list);
    }

    #[test]
    fn component_round_trip_ascii() {
        let mut comp = ComponentData::default();
        comp.lib_reference = "LM358".to_string();
        comp.component_description = "Dual Op-Amp".to_string();
        comp.part_count = 2;
        comp.display_mode_count = 1;
        comp.location_x = 1_000_000;
        comp.location_y = 2_000_000;
        comp.orientation = RotationBy90::Rotate90;
        comp.unique_id = "ABC123".to_string();
        comp.component_kind = ComponentKind::Standard;

        let mut w = AsciiSerializer::new_writer();
        export_component(&mut w, &comp).unwrap();
        let params = w.to_param_string();

        let mut r = AsciiSerializer::from_params(&params);
        let mut comp2 = ComponentData::default();
        import_component(&mut r, &mut comp2).unwrap();

        assert_eq!(comp2.lib_reference, "LM358");
        assert_eq!(comp2.component_description, "Dual Op-Amp");
        assert_eq!(comp2.part_count, 2);
        assert_eq!(comp2.location_x, 1_000_000);
        assert_eq!(comp2.location_y, 2_000_000);
        assert_eq!(comp2.orientation, RotationBy90::Rotate90);
    }

    #[test]
    fn rectangle_round_trip_ascii() {
        let mut rect = RectangleData::default();
        rect.location_x = 100_000;
        rect.location_y = 200_000;
        rect.corner_x = 500_000;
        rect.corner_y = 400_000;
        rect.is_solid = true;
        rect.color = 0xFF0000;
        rect.area_color = 0x00FF00;

        let mut w = AsciiSerializer::new_writer();
        export_rectangle(&mut w, &rect).unwrap();
        let params = w.to_param_string();

        let mut r = AsciiSerializer::from_params(&params);
        let mut rect2 = RectangleData::default();
        import_rectangle(&mut r, &mut rect2).unwrap();

        assert_eq!(rect2.location_x, 100_000);
        assert_eq!(rect2.corner_x, 500_000);
        assert!(rect2.is_solid);
        assert_eq!(rect2.color, 0xFF0000);
    }

    #[test]
    fn parameter_inverted_booleans() {
        let mut p = ParameterData::default();
        p.allow_library_synchronize = true;
        p.allow_database_synchronize = false;
        p.auto_position = true;
        p.name = "Value".to_string();
        p.text = "100k".to_string();

        let mut w = AsciiSerializer::new_writer();
        export_parameter(&mut w, &p).unwrap();
        let params = w.to_param_string();

        // Inverted fields: Not* versions should be false/true
        assert!(!params.contains("NotAllowLibrarySynchronize=T"));
        assert!(params.contains("NotAllowDatabaseSynchronize=T"));
        assert!(!params.contains("NotAutoPosition=T"));

        let mut r = AsciiSerializer::from_params(&params);
        let mut p2 = ParameterData::default();
        import_parameter(&mut r, &mut p2).unwrap();

        assert!(p2.allow_library_synchronize);
        assert!(!p2.allow_database_synchronize);
        assert!(p2.auto_position);
    }

    #[test]
    fn note_round_trip_ascii() {
        let mut n = NoteData::default();
        n.location_x = 100_000;
        n.location_y = 200_000;
        n.corner_x = 500_000;
        n.corner_y = 400_000;
        n.text = "Hello World".to_string();
        n.author = "Test".to_string();
        n.font_id = 2;
        n.is_solid = true;
        n.word_wrap = true;

        let mut w = AsciiSerializer::new_writer();
        export_note(&mut w, &n).unwrap();
        let params = w.to_param_string();

        let mut r = AsciiSerializer::from_params(&params);
        let mut n2 = NoteData::default();
        import_note(&mut r, &mut n2).unwrap();

        assert_eq!(n2.location_x, 100_000);
        assert_eq!(n2.corner_x, 500_000);
        assert_eq!(n2.text, "Hello World");
        assert_eq!(n2.author, "Test");
        assert_eq!(n2.font_id, 2);
        assert!(n2.is_solid);
        assert!(n2.word_wrap);
    }

    #[test]
    fn sheet_symbol_round_trip_ascii() {
        let mut ss = SheetSymbolData::default();
        ss.container.location_x = 100_000;
        ss.container.location_y = 200_000;
        ss.container.x_size = 300_000;
        ss.container.y_size = 400_000;
        ss.is_solid = true;
        ss.unique_id = "ABC".to_string();
        ss.design_item_id = "Test.SchDoc".to_string();

        let mut w = AsciiSerializer::new_writer();
        export_sheet_symbol(&mut w, &ss).unwrap();
        let params = w.to_param_string();

        let mut r = AsciiSerializer::from_params(&params);
        let mut ss2 = SheetSymbolData::default();
        import_sheet_symbol(&mut r, &mut ss2).unwrap();

        assert_eq!(ss2.container.location_x, 100_000);
        assert_eq!(ss2.container.x_size, 300_000);
        assert!(ss2.is_solid);
        assert_eq!(ss2.unique_id, "ABC");
        assert_eq!(ss2.design_item_id, "Test.SchDoc");
    }

    #[test]
    fn implementation_round_trip_ascii() {
        let mut imp = ImplementationData::default();
        imp.description = "Footprint".to_string();
        imp.model_name = "SOIC8".to_string();
        imp.model_type = "PCBLIB".to_string();
        imp.is_current = true;
        imp.use_component_library = true;
        imp.datafile_links.push(("lib.PcbLib".to_string(), "SOIC8".to_string(), "PCBLib".to_string()));

        let mut w = AsciiSerializer::new_writer();
        export_implementation(&mut w, &imp).unwrap();
        let params = w.to_param_string();

        let mut r = AsciiSerializer::from_params(&params);
        let mut imp2 = ImplementationData::default();
        import_implementation(&mut r, &mut imp2).unwrap();

        assert_eq!(imp2.description, "Footprint");
        assert_eq!(imp2.model_name, "SOIC8");
        assert_eq!(imp2.model_type, "PCBLIB");
        assert!(imp2.is_current);
        assert!(imp2.use_component_library);
        assert_eq!(imp2.datafile_links.len(), 1);
        assert_eq!(imp2.datafile_links[0].0, "lib.PcbLib");
    }

    #[test]
    fn sheet_name_inverted_auto_position() {
        let mut sn = SheetNameData::default();
        sn.auto_position = true;
        sn.text = "Sheet1".to_string();

        let mut w = AsciiSerializer::new_writer();
        export_sheet_name(&mut w, &sn).unwrap();
        let params = w.to_param_string();

        // auto_position=true means NotAutoPosition=F (which is omitted by ASCII)
        assert!(!params.contains("NotAutoPosition=T"));

        let mut r = AsciiSerializer::from_params(&params);
        let mut sn2 = SheetNameData::default();
        import_sheet_name(&mut r, &mut sn2).unwrap();

        assert!(sn2.auto_position);
        assert_eq!(sn2.text, "Sheet1");
    }

    #[test]
    fn blanket_round_trip_with_vertices() {
        let mut b = BlanketData::default();
        b.location_x = 100_000;
        b.location_y = 200_000;
        b.corner_x = 500_000;
        b.corner_y = 400_000;
        b.collapsed = true;
        b.vertices = vec![(100_000, 200_000), (300_000, 400_000), (500_000, 200_000)];

        let mut w = AsciiSerializer::new_writer();
        export_blanket(&mut w, &b).unwrap();
        let params = w.to_param_string();

        let mut r = AsciiSerializer::from_params(&params);
        let mut b2 = BlanketData::default();
        import_blanket(&mut r, &mut b2).unwrap();

        assert_eq!(b2.location_x, 100_000);
        assert!(b2.collapsed);
        assert_eq!(b2.vertices.len(), 3);
        assert_eq!(b2.vertices[1], (300_000, 400_000));
    }
}
