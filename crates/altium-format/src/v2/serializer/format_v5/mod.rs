//! Format V5 export/import functions — ported from `FileFormatV5.cs`.
//!
//! Each record type has an `export_*` and `import_*` function pair that
//! serializes/deserializes the record's fields via the `SchSerializer` trait.
//!
//! Field order, names, and types match the C# code exactly.

pub mod block;
pub mod component;
pub mod harness;
pub mod implementation;
pub mod misc;
pub mod parameter;
pub mod pin;
pub mod primitives;
pub mod schematic;
pub mod sheet;

// Re-export all format functions for convenience
pub use block::*;
pub use component::*;
pub use harness::*;
pub use implementation::*;
pub use misc::*;
pub use parameter::*;
pub use pin::*;
pub use primitives::*;
pub use schematic::*;
pub use sheet::*;

use crate::error::Result;
use crate::v2::fields::*;
use crate::v2::serializer::SchSerializer;

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
// Vertex helpers
// ============================================================================

/// Export vertex list — Altium uses LocationCount + Location.X_n/Location.Y_n pattern.
pub(crate) fn export_vertices(s: &mut dyn SchSerializer, verts: &[(i32, i32)]) -> Result<()> {
    s.export_short_int(verts.len() as i32, "LocationCount")?;
    for (i, (x, y)) in verts.iter().enumerate() {
        let n = i + 1;
        s.export_coord(*x, &format!("X{}", n))?;
        s.export_coord(*y, &format!("Y{}", n))?;
    }
    Ok(())
}

/// Import vertex list.
pub(crate) fn import_vertices(s: &mut dyn SchSerializer) -> Result<Vec<(i32, i32)>> {
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
// Connected objects helpers
// ============================================================================

/// Helper: export a list of connected object unique IDs.
pub(crate) fn export_connected_objects(s: &mut dyn SchSerializer, items: &[String], count_name: &str, item_name: &str) -> Result<()> {
    s.export_long_int(items.len() as i32, count_name)?;
    for (i, uid) in items.iter().enumerate() {
        s.export_dynamic_string(uid, &format!("{}{}", item_name, i))?;
    }
    Ok(())
}

/// Helper: import a list of connected object unique IDs.
pub(crate) fn import_connected_objects(s: &mut dyn SchSerializer, count_name: &str, item_name: &str) -> Result<Vec<String>> {
    let count = s.import_long_int(count_name)?;
    let mut items = Vec::with_capacity(count as usize);
    for i in 0..count {
        items.push(s.import_dynamic_string(&format!("{}{}", item_name, i))?);
    }
    Ok(items)
}

/// Helper: export library component fields (VaultGUID, ItemGUID, etc.).
pub(crate) fn export_library_component(s: &mut dyn SchSerializer, vault_guid: &str, item_guid: &str, revision_guid: &str, design_item_id: &str, source_library_name: &str, library_path: &str, lib_reference: &str, not_use_library_name: bool, database_table_name: &str) -> Result<()> {
    s.export_dynamic_string(vault_guid, "VaultGUID")?;
    s.export_dynamic_string(item_guid, "ItemGUID")?;
    s.export_dynamic_string(revision_guid, "RevisionGUID")?;
    s.export_dynamic_string(design_item_id, "DesignItemId")?;
    s.export_dynamic_string(source_library_name, "SourceLibraryName")?;
    s.export_dynamic_string(library_path, "LibraryPath")?;
    s.export_dynamic_string(lib_reference, "LibReference")?;
    s.export_boolean(not_use_library_name, "NotUseLibraryName")?;
    s.export_dynamic_string(database_table_name, "DatabaseTableName")?;
    Ok(())
}

/// Helper: import library component fields (VaultGUID, ItemGUID, etc.).
pub(crate) fn import_library_component(s: &mut dyn SchSerializer, vault_guid: &mut String, item_guid: &mut String, revision_guid: &mut String, design_item_id: &mut String, source_library_name: &mut String, library_path: &mut String, lib_reference: &mut String, not_use_library_name: &mut bool, database_table_name: &mut String) -> Result<()> {
    *vault_guid = s.import_dynamic_string("VaultGUID")?;
    *item_guid = s.import_dynamic_string("ItemGUID")?;
    *revision_guid = s.import_dynamic_string("RevisionGUID")?;
    *design_item_id = s.import_dynamic_string("DesignItemId")?;
    *source_library_name = s.import_dynamic_string("SourceLibraryName")?;
    *library_path = s.import_dynamic_string("LibraryPath")?;
    *lib_reference = s.import_dynamic_string("LibReference")?;
    let not_use = s.import_boolean("NotUseLibraryName")?;
    *not_use_library_name = not_use;
    *database_table_name = s.import_dynamic_string("DatabaseTableName")?;
    Ok(())
}

// ============================================================================
// String map/list helpers
// ============================================================================

pub(crate) fn export_string_map(s: &mut dyn SchSerializer, map: &[(String, String)], count_name: &str, key_prefix: &str, val_prefix: &str) -> Result<()> {
    s.export_long_int(map.len() as i32, count_name)?;
    for (i, (k, v)) in map.iter().enumerate() {
        s.export_dynamic_string(k, &format!("{}{}", key_prefix, i))?;
        s.export_dynamic_string(v, &format!("{}{}", val_prefix, i))?;
    }
    Ok(())
}

pub(crate) fn import_string_map(s: &mut dyn SchSerializer, count_name: &str, key_prefix: &str, val_prefix: &str) -> Result<Vec<(String, String)>> {
    let count = s.import_long_int(count_name)?;
    let mut map = Vec::with_capacity(count as usize);
    for i in 0..count {
        let k = s.import_dynamic_string(&format!("{}{}", key_prefix, i))?;
        let v = s.import_dynamic_string(&format!("{}{}", val_prefix, i))?;
        map.push((k, v));
    }
    Ok(map)
}

pub(crate) fn export_string_list(s: &mut dyn SchSerializer, list: &[String], count_name: &str, item_prefix: &str) -> Result<()> {
    s.export_long_int(list.len() as i32, count_name)?;
    for (i, item) in list.iter().enumerate() {
        s.export_dynamic_string(item, &format!("{}{}", item_prefix, i))?;
    }
    Ok(())
}

pub(crate) fn import_string_list(s: &mut dyn SchSerializer, count_name: &str, item_prefix: &str) -> Result<Vec<String>> {
    let count = s.import_long_int(count_name)?;
    let mut list = Vec::with_capacity(count as usize);
    for i in 0..count {
        list.push(s.import_dynamic_string(&format!("{}{}", item_prefix, i))?);
    }
    Ok(list)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2::fields::pin::PinData;
    use crate::v2::fields::component::ComponentData;
    use crate::v2::serializer::ascii::AsciiSerializer;
    use crate::v2::types::*;

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
