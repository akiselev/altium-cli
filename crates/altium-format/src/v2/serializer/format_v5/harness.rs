//! Format functions for harness record types.

use crate::error::Result;
use crate::v2::fields::harness::*;
use crate::v2::fields::misc::LineViewData;
use crate::v2::serializer::SchSerializer;
use super::{export_graphical_object, import_graphical_object, export_data_object, import_data_object, export_connected_objects, import_connected_objects, export_library_component, import_library_component};
use super::schematic::{export_power, import_power, export_wire, import_wire, export_net_label, import_net_label};
use super::primitives::{export_line, import_line, export_rectangle, import_rectangle};
use super::pin::{export_pin, import_pin};
use super::sheet::{export_sheet, import_sheet, export_basic_entry_object, import_basic_entry_object};

// ============================================================================
// HarnessEntry — delegates to BasicEntryObject
// ============================================================================

pub fn export_harness_entry(s: &mut dyn SchSerializer, he: &HarnessEntryData) -> Result<()> {
    export_basic_entry_object(s, he)
}

pub fn import_harness_entry(s: &mut dyn SchSerializer, he: &mut HarnessEntryData) -> Result<()> {
    import_basic_entry_object(s, he)
}

pub fn export_harness_splice(s: &mut dyn SchSerializer, hs: &HarnessSpliceData) -> Result<()> {
    export_graphical_object(s, &hs.graphical)?;
    s.export_byte(hs.style, "Style")?;
    s.export_boolean(hs.show_name, "ShowName")?;
    export_connected_objects(s, &hs.connected_wires, "ConnectedWiresUniqueIdsCount", "ConnectedWireUniqueId")?;
    s.export_dynamic_string(&hs.connected_inline_wire_unique_id, "ConnectedObjectUniqueId")?;
    s.export_coord(hs.location_x, "Location.X")?;
    s.export_coord(hs.location_y, "Location.Y")?;
    s.export_rotation_by90(hs.orientation, "Orientation")?;
    s.export_color(hs.color, "Color")?;
    s.export_color(hs.area_color, "AreaColor")?;
    s.export_color(hs.border_color, "BorderColor")?;
    if hs.font_id != 0 {
        s.export_font_id(hs.font_id, "FontID")?;
    }
    s.export_dynamic_string(&hs.text, "Text")?;
    s.export_dynamic_string(&hs.unique_id, "UniqueID")?;
    s.export_boolean(hs.designator_locked, "DesignatorLocked")?;
    Ok(())
}

pub fn import_harness_splice(s: &mut dyn SchSerializer, hs: &mut HarnessSpliceData) -> Result<()> {
    import_graphical_object(s, &mut hs.graphical)?;
    hs.style = s.import_byte("Style")?;
    hs.show_name = s.import_boolean("ShowName")?;
    hs.connected_wires = import_connected_objects(s, "ConnectedWiresUniqueIdsCount", "ConnectedWireUniqueId")?;
    hs.connected_inline_wire_unique_id = s.import_dynamic_string("ConnectedObjectUniqueId")?;
    hs.location_x = s.import_coord("Location.X")?;
    hs.location_y = s.import_coord("Location.Y")?;
    hs.orientation = s.import_rotation_by90("Orientation")?;
    hs.color = s.import_color("Color")?;
    hs.area_color = s.import_color("AreaColor")?;
    hs.border_color = s.import_color("BorderColor")?;
    hs.font_id = s.import_font_id("FontID")?;
    hs.text = s.import_dynamic_string("Text")?;
    hs.unique_id = s.import_dynamic_string("UniqueID")?;
    hs.designator_locked = s.import_boolean("DesignatorLocked")?;
    Ok(())
}

pub fn export_harness_no_connect(s: &mut dyn SchSerializer, hn: &HarnessNoConnectData) -> Result<()> {
    export_graphical_object(s, &hn.graphical)?;
    s.export_no_erc_symbol(hn.style, "Style")?;
    s.export_boolean(hn.show_name, "ShowName")?;
    export_connected_objects(s, &hn.connected_wires, "ConnectedWiresUniqueIdsCount", "ConnectedWireUniqueId")?;
    s.export_coord(hn.location_x, "Location.X")?;
    s.export_coord(hn.location_y, "Location.Y")?;
    s.export_rotation_by90(hn.orientation, "Orientation")?;
    s.export_color(hn.color, "Color")?;
    if hn.font_id != 0 {
        s.export_font_id(hn.font_id, "FontID")?;
    }
    s.export_dynamic_string(&hn.text, "Text")?;
    s.export_dynamic_string(&hn.unique_id, "UniqueID")?;
    Ok(())
}

pub fn import_harness_no_connect(s: &mut dyn SchSerializer, hn: &mut HarnessNoConnectData) -> Result<()> {
    import_graphical_object(s, &mut hn.graphical)?;
    hn.style = s.import_no_erc_symbol("Style")?;
    hn.show_name = s.import_boolean("ShowName")?;
    hn.connected_wires = import_connected_objects(s, "ConnectedWiresUniqueIdsCount", "ConnectedWireUniqueId")?;
    hn.location_x = s.import_coord("Location.X")?;
    hn.location_y = s.import_coord("Location.Y")?;
    hn.orientation = s.import_rotation_by90("Orientation")?;
    hn.color = s.import_color("Color")?;
    hn.font_id = s.import_font_id("FontID")?;
    hn.text = s.import_dynamic_string("Text")?;
    hn.unique_id = s.import_dynamic_string("UniqueID")?;
    Ok(())
}

pub fn export_harness_no_connect_data(s: &mut dyn SchSerializer, hnd: &HarnessNoConnectDataRecord) -> Result<()> {
    export_data_object(s, &hnd.base)?;
    s.export_dynamic_string(&hnd.designator, "Designator")?;
    export_connected_objects(s, &hnd.connected_wires, "ConnectedWiresUniqueIdsCount", "ConnectedWireUniqueId")?;
    s.export_dynamic_string(&hnd.unique_id, "UniqueID")?;
    Ok(())
}

pub fn import_harness_no_connect_data(s: &mut dyn SchSerializer, hnd: &mut HarnessNoConnectDataRecord) -> Result<()> {
    import_data_object(s, &mut hnd.base)?;
    hnd.designator = s.import_dynamic_string("Designator")?;
    hnd.connected_wires = import_connected_objects(s, "ConnectedWiresUniqueIdsCount", "ConnectedWireUniqueId")?;
    hnd.unique_id = s.import_dynamic_string("UniqueID")?;
    Ok(())
}

pub fn export_harness_splice_data(s: &mut dyn SchSerializer, hsd: &HarnessSpliceDataRecord) -> Result<()> {
    export_data_object(s, &hsd.base)?;
    s.export_dynamic_string(&hsd.designator, "Designator")?;
    export_connected_objects(s, &hsd.connected_wires, "ConnectedWiresUniqueIdsCount", "ConnectedWireUniqueId")?;
    s.export_dynamic_string(&hsd.connected_inline_wire_unique_id, "ConnectedObjectUniqueId")?;
    s.export_dynamic_string(&hsd.unique_id, "UniqueID")?;
    s.export_byte(hsd.style, "Style")?;
    Ok(())
}

pub fn import_harness_splice_data(s: &mut dyn SchSerializer, hsd: &mut HarnessSpliceDataRecord) -> Result<()> {
    import_data_object(s, &mut hsd.base)?;
    hsd.designator = s.import_dynamic_string("Designator")?;
    hsd.connected_wires = import_connected_objects(s, "ConnectedWiresUniqueIdsCount", "ConnectedWireUniqueId")?;
    hsd.connected_inline_wire_unique_id = s.import_dynamic_string("ConnectedObjectUniqueId")?;
    hsd.unique_id = s.import_dynamic_string("UniqueID")?;
    hsd.style = s.import_byte("Style")?;
    Ok(())
}

pub fn export_harness_twist_data(s: &mut dyn SchSerializer, htd: &HarnessTwistDataRecord) -> Result<()> {
    export_data_object(s, &htd.base)?;
    s.export_dynamic_string(&htd.designator, "Designator")?;
    export_connected_objects(s, &htd.connected_wires, "ConnectedWiresUniqueIdsCount", "ConnectedWireUniqueId")?;
    s.export_dynamic_string(&htd.unique_id, "UniqueID")?;
    Ok(())
}

pub fn import_harness_twist_data(s: &mut dyn SchSerializer, htd: &mut HarnessTwistDataRecord) -> Result<()> {
    import_data_object(s, &mut htd.base)?;
    htd.designator = s.import_dynamic_string("Designator")?;
    htd.connected_wires = import_connected_objects(s, "ConnectedWiresUniqueIdsCount", "ConnectedWireUniqueId")?;
    htd.unique_id = s.import_dynamic_string("UniqueID")?;
    Ok(())
}

pub fn export_harness_shield_data(s: &mut dyn SchSerializer, hsd: &HarnessShieldDataRecord) -> Result<()> {
    export_data_object(s, &hsd.base)?;
    s.export_dynamic_string(&hsd.designator, "Designator")?;
    export_connected_objects(s, &hsd.connected_wires, "ConnectedWiresUniqueIdsCount", "ConnectedWireUniqueId")?;
    export_connected_objects(s, &hsd.connected_pin_wires, "ConnectedPinWiresUniqueIdsCount", "ConnectedPinWireUniqueId")?;
    s.export_dynamic_string(&hsd.unique_id, "UniqueID")?;
    s.export_byte(hsd.style, "Style")?;
    s.export_dynamic_string(&hsd.comment, "Comment")?;
    s.export_byte(hsd.component_kind, "ComponentKind")?;
    Ok(())
}

pub fn import_harness_shield_data(s: &mut dyn SchSerializer, hsd: &mut HarnessShieldDataRecord) -> Result<()> {
    import_data_object(s, &mut hsd.base)?;
    hsd.designator = s.import_dynamic_string("Designator")?;
    hsd.connected_wires = import_connected_objects(s, "ConnectedWiresUniqueIdsCount", "ConnectedWireUniqueId")?;
    hsd.connected_pin_wires = import_connected_objects(s, "ConnectedPinWiresUniqueIdsCount", "ConnectedPinWireUniqueId")?;
    hsd.unique_id = s.import_dynamic_string("UniqueID")?;
    hsd.style = s.import_byte("Style")?;
    hsd.comment = s.import_dynamic_string("Comment")?;
    hsd.component_kind = s.import_byte("ComponentKind")?;
    Ok(())
}

pub fn export_harness_wire_data(s: &mut dyn SchSerializer, hwd: &HarnessWireDataRecord) -> Result<()> {
    export_data_object(s, &hwd.base)?;
    s.export_dynamic_string(&hwd.name, "Name")?;
    s.export_dynamic_string(&hwd.comment, "Comment")?;
    s.export_dynamic_string(&hwd.description, "Description")?;
    s.export_color(hwd.color, "Color")?;
    s.export_dynamic_string(&hwd.end_vertex1_connected_object_unique_id, "EndVertex1ConnectedObjectUniqueID")?;
    s.export_dynamic_string(&hwd.end_vertex2_connected_object_unique_id, "EndVertex2ConnectedObjectUniqueID")?;
    export_connected_objects(s, &hwd.connected_shields, "ConnectedShieldsUniqueIdsCount", "ConnectedShieldUniqueId")?;
    export_connected_objects(s, &hwd.connected_twists, "ConnectedTwistsUniqueIdsCount", "ConnectedTwistUniqueId")?;
    export_connected_objects(s, &hwd.connected_cables, "ConnectedCablesUniqueIdsCount", "ConnectedCableUniqueId")?;
    export_connected_objects(s, &hwd.connected_inline_splices, "ConnectedInlineSplicesUniqueIdsCount", "ConnectedInlineSpliceUniqueId")?;
    s.export_dynamic_string(&hwd.unique_id, "UniqueID")?;
    s.export_dynamic_string(&hwd.vault_guid, "VaultGUID")?;
    s.export_dynamic_string(&hwd.item_guid, "ItemGUID")?;
    s.export_dynamic_string(&hwd.revision_guid, "RevisionGUID")?;
    s.export_dynamic_string(&hwd.design_item_id, "DesignItemId")?;
    s.export_dynamic_string(&hwd.source_library_name, "SourceLibraryName")?;
    s.export_byte(hwd.component_kind, "ComponentKind")?;
    Ok(())
}

pub fn import_harness_wire_data(s: &mut dyn SchSerializer, hwd: &mut HarnessWireDataRecord) -> Result<()> {
    import_data_object(s, &mut hwd.base)?;
    hwd.name = s.import_dynamic_string("Name")?;
    hwd.comment = s.import_dynamic_string("Comment")?;
    hwd.description = s.import_dynamic_string("Description")?;
    hwd.color = s.import_color("Color")?;
    hwd.end_vertex1_connected_object_unique_id = s.import_dynamic_string("EndVertex1ConnectedObjectUniqueID")?;
    hwd.end_vertex2_connected_object_unique_id = s.import_dynamic_string("EndVertex2ConnectedObjectUniqueID")?;
    hwd.connected_shields = import_connected_objects(s, "ConnectedShieldsUniqueIdsCount", "ConnectedShieldUniqueId")?;
    hwd.connected_twists = import_connected_objects(s, "ConnectedTwistsUniqueIdsCount", "ConnectedTwistUniqueId")?;
    hwd.connected_cables = import_connected_objects(s, "ConnectedCablesUniqueIdsCount", "ConnectedCableUniqueId")?;
    hwd.connected_inline_splices = import_connected_objects(s, "ConnectedInlineSplicesUniqueIdsCount", "ConnectedInlineSpliceUniqueId")?;
    hwd.unique_id = s.import_dynamic_string("UniqueID")?;
    hwd.vault_guid = s.import_dynamic_string("VaultGUID")?;
    hwd.item_guid = s.import_dynamic_string("ItemGUID")?;
    hwd.revision_guid = s.import_dynamic_string("RevisionGUID")?;
    hwd.design_item_id = s.import_dynamic_string("DesignItemId")?;
    hwd.source_library_name = s.import_dynamic_string("SourceLibraryName")?;
    hwd.component_kind = s.import_byte("ComponentKind")?;
    Ok(())
}

pub fn export_harness_cable_data(s: &mut dyn SchSerializer, hcd: &HarnessCableDataRecord) -> Result<()> {
    export_data_object(s, &hcd.base)?;
    s.export_dynamic_string(&hcd.designator, "Designator")?;
    s.export_dynamic_string(&hcd.comment, "Comment")?;
    s.export_dynamic_string(&hcd.description, "Description")?;
    export_connected_objects(s, &hcd.connected_wires, "ConnectedWiresUniqueIdsCount", "ConnectedWireUniqueId")?;
    s.export_dynamic_string(&hcd.unique_id, "UniqueID")?;
    s.export_dynamic_string(&hcd.vault_guid, "VaultGUID")?;
    s.export_dynamic_string(&hcd.item_guid, "ItemGUID")?;
    s.export_dynamic_string(&hcd.revision_guid, "RevisionGUID")?;
    s.export_dynamic_string(&hcd.design_item_id, "DesignItemId")?;
    s.export_dynamic_string(&hcd.source_library_name, "SourceLibraryName")?;
    s.export_byte(hcd.component_kind, "ComponentKind")?;
    Ok(())
}

pub fn import_harness_cable_data(s: &mut dyn SchSerializer, hcd: &mut HarnessCableDataRecord) -> Result<()> {
    import_data_object(s, &mut hcd.base)?;
    hcd.designator = s.import_dynamic_string("Designator")?;
    hcd.comment = s.import_dynamic_string("Comment")?;
    hcd.description = s.import_dynamic_string("Description")?;
    hcd.connected_wires = import_connected_objects(s, "ConnectedWiresUniqueIdsCount", "ConnectedWireUniqueId")?;
    hcd.unique_id = s.import_dynamic_string("UniqueID")?;
    hcd.vault_guid = s.import_dynamic_string("VaultGUID")?;
    hcd.item_guid = s.import_dynamic_string("ItemGUID")?;
    hcd.revision_guid = s.import_dynamic_string("RevisionGUID")?;
    hcd.design_item_id = s.import_dynamic_string("DesignItemId")?;
    hcd.source_library_name = s.import_dynamic_string("SourceLibraryName")?;
    hcd.component_kind = s.import_byte("ComponentKind")?;
    Ok(())
}

pub fn export_harness_layout_connection_point(s: &mut dyn SchSerializer, hlcp: &HarnessLayoutConnectionPointData) -> Result<()> {
    export_graphical_object(s, &hlcp.graphical)?;
    s.export_byte(hlcp.style, "Style")?;
    export_connected_objects(s, &hlcp.connected_bundles, "ConnectedBundlesUniqueIdsCount", "ConnectedBundleUniqueId")?;
    s.export_coord(hlcp.location_x, "Location.X")?;
    s.export_coord(hlcp.location_y, "Location.Y")?;
    s.export_rotation_by90(hlcp.orientation, "Orientation")?;
    s.export_color(hlcp.color, "Color")?;
    s.export_color(hlcp.area_color, "AreaColor")?;
    s.export_color(hlcp.border_color, "BorderColor")?;
    if hlcp.font_id != 0 {
        s.export_font_id(hlcp.font_id, "FontID")?;
    }
    s.export_dynamic_string(&hlcp.text, "Text")?;
    s.export_dynamic_string(&hlcp.unique_id, "UniqueID")?;
    s.export_boolean(hlcp.show_name, "ShowName")?;
    s.export_boolean(hlcp.designator_locked, "DesignatorLocked")?;
    Ok(())
}

pub fn import_harness_layout_connection_point(s: &mut dyn SchSerializer, hlcp: &mut HarnessLayoutConnectionPointData) -> Result<()> {
    import_graphical_object(s, &mut hlcp.graphical)?;
    hlcp.style = s.import_byte("Style")?;
    hlcp.connected_bundles = import_connected_objects(s, "ConnectedBundlesUniqueIdsCount", "ConnectedBundleUniqueId")?;
    hlcp.location_x = s.import_coord("Location.X")?;
    hlcp.location_y = s.import_coord("Location.Y")?;
    hlcp.orientation = s.import_rotation_by90("Orientation")?;
    hlcp.color = s.import_color("Color")?;
    hlcp.area_color = s.import_color("AreaColor")?;
    hlcp.border_color = s.import_color("BorderColor")?;
    hlcp.font_id = s.import_font_id("FontID")?;
    hlcp.text = s.import_dynamic_string("Text")?;
    hlcp.unique_id = s.import_dynamic_string("UniqueID")?;
    hlcp.show_name = s.import_boolean("ShowName")?;
    hlcp.designator_locked = s.import_boolean("DesignatorLocked")?;
    Ok(())
}

pub fn export_harness_wire_break(s: &mut dyn SchSerializer, hwb: &HarnessWireBreakData) -> Result<()> {
    export_power(s, &hwb.power)?;
    s.export_dynamic_string(&hwb.connected_wire_unique_id, "ConnectedObjectUniqueId")?;
    s.export_color(hwb.secondary_color, "SecondaryColor")?;
    s.export_color(hwb.tertiary_color, "TertiaryColor")?;
    s.export_color(hwb.border_color, "BorderColor")?;
    s.export_dynamic_string(&hwb.primary_color_name, "PrimaryColorName")?;
    s.export_dynamic_string(&hwb.secondary_color_name, "SecondaryColorName")?;
    s.export_dynamic_string(&hwb.tertiary_color_name, "TertiaryColorName")?;
    s.export_dynamic_string(&hwb.border_color_name, "BorderColorName")?;
    s.export_dynamic_string(&hwb.vault_guid, "VaultGUID")?;
    s.export_dynamic_string(&hwb.item_guid, "ItemGUID")?;
    s.export_dynamic_string(&hwb.revision_guid, "RevisionGUID")?;
    s.export_dynamic_string(&hwb.design_item_id, "DesignItemId")?;
    s.export_dynamic_string(&hwb.source_library_name, "SourceLibraryName")?;
    Ok(())
}

pub fn import_harness_wire_break(s: &mut dyn SchSerializer, hwb: &mut HarnessWireBreakData) -> Result<()> {
    import_power(s, &mut hwb.power)?;
    hwb.connected_wire_unique_id = s.import_dynamic_string("ConnectedObjectUniqueId")?;
    hwb.secondary_color = s.import_color("SecondaryColor")?;
    hwb.tertiary_color = s.import_color("TertiaryColor")?;
    hwb.border_color = s.import_color("BorderColor")?;
    hwb.primary_color_name = s.import_dynamic_string("PrimaryColorName")?;
    hwb.secondary_color_name = s.import_dynamic_string("SecondaryColorName")?;
    hwb.tertiary_color_name = s.import_dynamic_string("TertiaryColorName")?;
    hwb.border_color_name = s.import_dynamic_string("BorderColorName")?;
    hwb.vault_guid = s.import_dynamic_string("VaultGUID")?;
    hwb.item_guid = s.import_dynamic_string("ItemGUID")?;
    hwb.revision_guid = s.import_dynamic_string("RevisionGUID")?;
    hwb.design_item_id = s.import_dynamic_string("DesignItemId")?;
    hwb.source_library_name = s.import_dynamic_string("SourceLibraryName")?;
    Ok(())
}

pub fn export_line_view(s: &mut dyn SchSerializer, lv: &LineViewData) -> Result<()> {
    export_graphical_object(s, &lv.graphical)?;
    s.export_short_int(lv.locations.len() as i32, "LocationCount")?;
    for (i, (x1, y1, x2, y2)) in lv.locations.iter().enumerate() {
        s.export_coord(*x1, &format!("X1_{}", i))?;
        s.export_coord(*y1, &format!("Y1_{}", i))?;
        s.export_coord(*x2, &format!("X2_{}", i))?;
        s.export_coord(*y2, &format!("Y2_{}", i))?;
    }
    s.export_rotation_by90(lv.orientation, "Orientation")?;
    Ok(())
}

pub fn import_line_view(s: &mut dyn SchSerializer, lv: &mut LineViewData) -> Result<()> {
    import_graphical_object(s, &mut lv.graphical)?;
    let count = s.import_short_int("LocationCount")?;
    lv.locations.clear();
    for i in 0..count {
        let x1 = s.import_coord(&format!("X1_{}", i))?;
        let y1 = s.import_coord(&format!("Y1_{}", i))?;
        let x2 = s.import_coord(&format!("X2_{}", i))?;
        let y2 = s.import_coord(&format!("Y2_{}", i))?;
        lv.locations.push((x1, y1, x2, y2));
    }
    lv.orientation = s.import_rotation_by90("Orientation")?;
    Ok(())
}

// ============================================================================
// HarnessWire — Wire base + harness-specific fields
// ============================================================================

pub fn export_harness_wire(s: &mut dyn SchSerializer, hw: &HarnessWireData) -> Result<()> {
    export_wire(s, &hw.wire)?;
    s.export_color(hw.secondary_color, "SecondaryColor")?;
    s.export_color(hw.tertiary_color, "TertiaryColor")?;
    s.export_color(hw.border_color, "BorderColor")?;
    s.export_dynamic_string(&hw.end_vertex1_connected_object_unique_id, "EndVertex1ConnectedObjectUniqueID")?;
    s.export_dynamic_string(&hw.end_vertex2_connected_object_unique_id, "EndVertex2ConnectedObjectUniqueID")?;
    export_connected_objects(s, &hw.connected_inline_splices, "ConnectedInlineSplicesUniqueIdsCount", "ConnectedInlineSpliceUniqueId")?;
    export_connected_objects(s, &hw.connected_wire_labels, "ConnectedWireLabelsUniqueIdsCount", "ConnectedWireLabelUniqueId")?;
    export_connected_objects(s, &hw.connected_shields, "ConnectedShieldsUniqueIdsCount", "ConnectedShieldUniqueId")?;
    export_connected_objects(s, &hw.connected_twists, "ConnectedTwistsUniqueIdsCount", "ConnectedTwistUniqueId")?;
    export_connected_objects(s, &hw.connected_cables, "ConnectedCablesUniqueIdsCount", "ConnectedCableUniqueId")?;
    export_library_component(s, &hw.vault_guid, &hw.item_guid, &hw.revision_guid, &hw.design_item_id, &hw.source_library_name, &hw.library_path, &hw.lib_reference, hw.not_use_library_name, &hw.database_table_name)?;
    s.export_boolean(hw.designator_locked, "DesignatorLocked")?;
    s.export_byte(hw.component_kind, "ComponentKind")?;
    Ok(())
}

pub fn import_harness_wire(s: &mut dyn SchSerializer, hw: &mut HarnessWireData) -> Result<()> {
    import_wire(s, &mut hw.wire)?;
    hw.secondary_color = s.import_color("SecondaryColor")?;
    hw.tertiary_color = s.import_color("TertiaryColor")?;
    hw.border_color = s.import_color("BorderColor")?;
    hw.end_vertex1_connected_object_unique_id = s.import_dynamic_string("EndVertex1ConnectedObjectUniqueID")?;
    hw.end_vertex2_connected_object_unique_id = s.import_dynamic_string("EndVertex2ConnectedObjectUniqueID")?;
    hw.connected_inline_splices = import_connected_objects(s, "ConnectedInlineSplicesUniqueIdsCount", "ConnectedInlineSpliceUniqueId")?;
    hw.connected_wire_labels = import_connected_objects(s, "ConnectedWireLabelsUniqueIdsCount", "ConnectedWireLabelUniqueId")?;
    hw.connected_shields = import_connected_objects(s, "ConnectedShieldsUniqueIdsCount", "ConnectedShieldUniqueId")?;
    hw.connected_twists = import_connected_objects(s, "ConnectedTwistsUniqueIdsCount", "ConnectedTwistUniqueId")?;
    hw.connected_cables = import_connected_objects(s, "ConnectedCablesUniqueIdsCount", "ConnectedCableUniqueId")?;
    import_library_component(s, &mut hw.vault_guid, &mut hw.item_guid, &mut hw.revision_guid, &mut hw.design_item_id, &mut hw.source_library_name, &mut hw.library_path, &mut hw.lib_reference, &mut hw.not_use_library_name, &mut hw.database_table_name)?;
    hw.designator_locked = s.import_boolean("DesignatorLocked")?;
    hw.component_kind = s.import_byte("ComponentKind")?;
    Ok(())
}

// ============================================================================
// HarnessBundle — Wire base + bundle-specific fields
// ============================================================================

pub fn export_harness_bundle(s: &mut dyn SchSerializer, hb: &HarnessBundleData) -> Result<()> {
    export_wire(s, &hb.wire)?;
    let compat_length = if hb.is_length_set_manually { hb.length } else { 0 };
    let compat_length_long = if hb.is_length_set_manually { hb.length_long } else { 0 };
    s.export_coord(compat_length, "Length")?;
    s.export_long(compat_length_long, "LengthLong")?;
    s.export_boolean(hb.is_length_set_manually, "IsLengthSetManually")?;
    s.export_dynamic_string(&hb.end_vertex1_connected_object_unique_id, "EndVertex1ConnectedObjectUniqueID")?;
    s.export_dynamic_string(&hb.end_vertex2_connected_object_unique_id, "EndVertex2ConnectedObjectUniqueID")?;
    s.export_boolean(hb.designator_locked, "DesignatorLocked")?;
    Ok(())
}

pub fn import_harness_bundle(s: &mut dyn SchSerializer, hb: &mut HarnessBundleData) -> Result<()> {
    import_wire(s, &mut hb.wire)?;
    hb.length = s.import_coord("Length")?;
    hb.length_long = s.import_long("LengthLong")?;
    if hb.length_long != 0 {
        // LengthLong overrides the old Length field
        hb.length = hb.length_long as i32;
    }
    hb.is_length_set_manually = s.import_boolean("IsLengthSetManually")?;
    hb.end_vertex1_connected_object_unique_id = s.import_dynamic_string("EndVertex1ConnectedObjectUniqueID")?;
    hb.end_vertex2_connected_object_unique_id = s.import_dynamic_string("EndVertex2ConnectedObjectUniqueID")?;
    hb.designator_locked = s.import_boolean("DesignatorLocked")?;
    Ok(())
}

// ============================================================================
// HarnessLogicalSignal — Line base + signal fields
// ============================================================================

pub fn export_harness_logical_signal(s: &mut dyn SchSerializer, hls: &HarnessLogicalSignalData) -> Result<()> {
    export_line(s, &hls.line)?;
    s.export_dynamic_string(&hls.connection1_comp, "HarnessLogicalSignalConnection1Comp")?;
    s.export_dynamic_string(&hls.connection1_pin, "HarnessLogicalSignalConnection1Pin")?;
    s.export_dynamic_string(&hls.connection2_comp, "HarnessLogicalSignalConnection2Comp")?;
    s.export_dynamic_string(&hls.connection2_pin, "HarnessLogicalSignalConnection2Pin")?;
    s.export_dynamic_string(&hls.name, "Name")?;
    s.export_dynamic_string(&hls.system_design_unique_id, "SystemDesignUniqueId")?;
    Ok(())
}

pub fn import_harness_logical_signal(s: &mut dyn SchSerializer, hls: &mut HarnessLogicalSignalData) -> Result<()> {
    import_line(s, &mut hls.line)?;
    hls.connection1_comp = s.import_dynamic_string("HarnessLogicalSignalConnection1Comp")?;
    hls.connection1_pin = s.import_dynamic_string("HarnessLogicalSignalConnection1Pin")?;
    hls.connection2_comp = s.import_dynamic_string("HarnessLogicalSignalConnection2Comp")?;
    hls.connection2_pin = s.import_dynamic_string("HarnessLogicalSignalConnection2Pin")?;
    hls.name = s.import_dynamic_string("Name")?;
    hls.system_design_unique_id = s.import_dynamic_string("SystemDesignUniqueId")?;
    Ok(())
}

// ============================================================================
// HarnessPin — Pin base + harness fields
// ============================================================================

pub fn export_harness_pin(s: &mut dyn SchSerializer, hp: &HarnessPinData) -> Result<()> {
    export_pin(s, &hp.pin)?;
    export_connected_objects(s, &hp.connected_wires, "ConnectedWiresUniqueIdsCount", "ConnectedWireUniqueId")?;
    s.export_dynamic_string(&hp.wiring_diagram_origin_unique_id, "WiringDiagramOriginUniqueId")?;
    Ok(())
}

pub fn import_harness_pin(s: &mut dyn SchSerializer, hp: &mut HarnessPinData) -> Result<()> {
    import_pin(s, &mut hp.pin)?;
    hp.connected_wires = import_connected_objects(s, "ConnectedWiresUniqueIdsCount", "ConnectedWireUniqueId")?;
    hp.wiring_diagram_origin_unique_id = s.import_dynamic_string("WiringDiagramOriginUniqueId")?;
    Ok(())
}

// ============================================================================
// HarnessWireLabel — NetLabel base + connected wire
// ============================================================================

pub fn export_harness_wire_label(s: &mut dyn SchSerializer, hwl: &HarnessWireLabelData) -> Result<()> {
    export_net_label(s, &hwl.net_label)?;
    s.export_dynamic_string(&hwl.connected_wire_unique_id, "ConnectedObjectUniqueId")?;
    Ok(())
}

pub fn import_harness_wire_label(s: &mut dyn SchSerializer, hwl: &mut HarnessWireLabelData) -> Result<()> {
    import_net_label(s, &mut hwl.net_label)?;
    hwl.connected_wire_unique_id = s.import_dynamic_string("ConnectedObjectUniqueId")?;
    Ok(())
}

// ============================================================================
// HarnessLayoutLabel — Label base (without text) + harness-specific fields
// ============================================================================

pub fn export_harness_layout_label(s: &mut dyn SchSerializer, hll: &HarnessLayoutLabelData) -> Result<()> {
    // Export label fields WITHOUT text (C# calls ExportLabel with includeText: false)
    export_graphical_object(s, &hll.label.graphical)?;
    s.export_coord(hll.label.location_x, "Location.X")?;
    s.export_coord(hll.label.location_y, "Location.Y")?;
    s.export_rotation_by90(hll.label.orientation, "Orientation")?;
    s.export_text_justification(hll.label.justification, "Justification")?;
    s.export_color(hll.label.color, "Color")?;
    s.export_font_id(hll.label.font_id, "FontID")?;
    // Text is NOT exported here — EncodedText is exported below as "Text"
    s.export_boolean(hll.label.is_mirrored, "IsMirrored")?;
    s.export_dynamic_string(&hll.label.url, "URL")?;
    s.export_dynamic_string(&hll.label.unique_id, "UniqueID")?;
    // Harness-specific fields
    s.export_horizontal_align(hll.alignment, "Alignment")?;
    s.export_color(hll.area_color, "AreaColor")?;
    s.export_color(hll.text_color, "TextColor")?;
    s.export_boolean(hll.show_only_first_line, "ShowOnlyFirstLine")?;
    s.export_dynamic_string(&hll.encoded_text, "Text")?;
    s.export_boolean(hll.designator_locked, "DesignatorLocked")?;
    s.export_dynamic_string(&hll.vault_guid, "VaultGUID")?;
    s.export_dynamic_string(&hll.item_guid, "ItemGUID")?;
    s.export_dynamic_string(&hll.revision_guid, "RevisionGUID")?;
    s.export_dynamic_string(&hll.design_item_id, "DesignItemId")?;
    s.export_dynamic_string(&hll.source_library_name, "SourceLibraryName")?;
    s.export_dynamic_string(&hll.library_path, "LibraryPath")?;
    s.export_dynamic_string(&hll.lib_reference, "LibReference")?;
    s.export_boolean(hll.not_use_library_name, "NotUseLibraryName")?;
    s.export_dynamic_string(&hll.database_table_name, "DatabaseTableName")?;
    s.export_byte(hll.component_kind, "ComponentKind")?;
    Ok(())
}

pub fn import_harness_layout_label(s: &mut dyn SchSerializer, hll: &mut HarnessLayoutLabelData) -> Result<()> {
    // Import label fields WITHOUT text
    import_graphical_object(s, &mut hll.label.graphical)?;
    hll.label.location_x = s.import_coord("Location.X")?;
    hll.label.location_y = s.import_coord("Location.Y")?;
    hll.label.orientation = s.import_rotation_by90("Orientation")?;
    hll.label.justification = s.import_text_justification("Justification")?;
    hll.label.color = s.import_color("Color")?;
    hll.label.font_id = s.import_font_id("FontID")?;
    // Text is NOT imported here — EncodedText is imported below from "Text"
    hll.label.is_mirrored = s.import_boolean("IsMirrored")?;
    hll.label.url = s.import_dynamic_string("URL")?;
    hll.label.unique_id = s.import_dynamic_string("UniqueID")?;
    // Harness-specific fields
    hll.alignment = s.import_horizontal_align("Alignment")?;
    hll.area_color = s.import_color("AreaColor")?;
    hll.text_color = s.import_color("TextColor")?;
    hll.show_only_first_line = s.import_boolean("ShowOnlyFirstLine")?;
    hll.encoded_text = s.import_dynamic_string("Text")?;
    hll.designator_locked = s.import_boolean("DesignatorLocked")?;
    hll.vault_guid = s.import_dynamic_string("VaultGUID")?;
    hll.item_guid = s.import_dynamic_string("ItemGUID")?;
    hll.revision_guid = s.import_dynamic_string("RevisionGUID")?;
    hll.design_item_id = s.import_dynamic_string("DesignItemId")?;
    hll.source_library_name = s.import_dynamic_string("SourceLibraryName")?;
    hll.library_path = s.import_dynamic_string("LibraryPath")?;
    hll.lib_reference = s.import_dynamic_string("LibReference")?;
    hll.not_use_library_name = s.import_boolean("NotUseLibraryName")?;
    hll.database_table_name = s.import_dynamic_string("DatabaseTableName")?;
    hll.component_kind = s.import_byte("ComponentKind")?;
    Ok(())
}

// ============================================================================
// HarnessLayoutCovering — graphical + covering-specific fields
// ============================================================================

pub fn export_harness_layout_covering(s: &mut dyn SchSerializer, hlc: &HarnessLayoutCoveringData) -> Result<()> {
    export_graphical_object(s, &hlc.graphical)?;
    s.export_size(hlc.border_width, "BorderWidth")?;
    s.export_color(hlc.color, "Color")?;
    s.export_color(hlc.area_color, "AreaColor")?;
    s.export_boolean(hlc.transparent, "Transparent")?;
    s.export_byte(hlc.thickness, "Thickness")?;
    s.export_long_int(hlc.start_point_distance, "StartPointDistance")?;
    s.export_long_int(hlc.end_point_distance, "EndPointDistance")?;
    s.export_long_int(hlc.length, "Length")?;
    s.export_byte(hlc.harness_layout_braid_brush, "HarnessLayoutBraidBrush")?;
    s.export_boolean(hlc.designator_locked, "DesignatorLocked")?;
    export_library_component(s, &hlc.vault_guid, &hlc.item_guid, &hlc.revision_guid, &hlc.design_item_id, &hlc.source_library_name, &hlc.library_path, &hlc.lib_reference, hlc.not_use_library_name, &hlc.database_table_name)?;
    s.export_coord(hlc.default_designator_position_x, "DefaultDesignatorPosition.X")?;
    s.export_coord(hlc.default_designator_position_y, "DefaultDesignatorPosition.Y")?;
    s.export_dynamic_string(&hlc.unique_id, "UniqueID")?;
    s.export_byte(hlc.component_kind, "ComponentKind")?;
    s.export_long(hlc.physical_start_distance, "PhysicalStartDistance")?;
    s.export_long(hlc.physical_end_distance, "PhysicalEndDistance")?;
    s.export_long(hlc.physical_length, "PhysicalLength")?;
    Ok(())
}

pub fn import_harness_layout_covering(s: &mut dyn SchSerializer, hlc: &mut HarnessLayoutCoveringData) -> Result<()> {
    import_graphical_object(s, &mut hlc.graphical)?;
    hlc.color = s.import_color("Color")?;
    hlc.area_color = s.import_color("AreaColor")?;
    hlc.border_width = s.import_size("BorderWidth")?;
    hlc.transparent = s.import_boolean("Transparent")?;
    hlc.thickness = s.import_byte("Thickness")?;
    hlc.start_point_distance = s.import_long_int("StartPointDistance")?;
    hlc.end_point_distance = s.import_long_int("EndPointDistance")?;
    hlc.length = s.import_long_int("Length")?;
    hlc.harness_layout_braid_brush = s.import_byte("HarnessLayoutBraidBrush")?;
    hlc.designator_locked = s.import_boolean("DesignatorLocked")?;
    hlc.unique_id = s.import_dynamic_string("UniqueID")?;
    import_library_component(s, &mut hlc.vault_guid, &mut hlc.item_guid, &mut hlc.revision_guid, &mut hlc.design_item_id, &mut hlc.source_library_name, &mut hlc.library_path, &mut hlc.lib_reference, &mut hlc.not_use_library_name, &mut hlc.database_table_name)?;
    hlc.default_designator_position_x = s.import_coord("DefaultDesignatorPosition.X")?;
    hlc.default_designator_position_y = s.import_coord("DefaultDesignatorPosition.Y")?;
    hlc.component_kind = s.import_byte("ComponentKind")?;
    hlc.physical_start_distance = s.import_long("PhysicalStartDistance")?;
    hlc.physical_end_distance = s.import_long("PhysicalEndDistance")?;
    hlc.physical_length = s.import_long("PhysicalLength")?;
    Ok(())
}

// ============================================================================
// Harness Library Connectivity helpers (Rectangle + library fields)
// Used by HarnessShield, HarnessTwist, HarnessCable
// ============================================================================

fn export_harness_library_connectivity(s: &mut dyn SchSerializer, rect: &crate::v2::fields::primitives::RectangleData, vault_guid: &str, item_guid: &str, revision_guid: &str, design_item_id: &str, source_library_name: &str, library_path: &str, lib_reference: &str, not_use_library_name: bool, database_table_name: &str) -> Result<()> {
    export_rectangle(s, rect)?;
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

fn import_harness_library_connectivity(s: &mut dyn SchSerializer, rect: &mut crate::v2::fields::primitives::RectangleData, vault_guid: &mut String, item_guid: &mut String, revision_guid: &mut String, design_item_id: &mut String, source_library_name: &mut String, library_path: &mut String, lib_reference: &mut String, not_use_library_name: &mut bool, database_table_name: &mut String) -> Result<()> {
    import_rectangle(s, rect)?;
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
// HarnessShield — Rectangle + library + shield-specific fields
// ============================================================================

pub fn export_harness_shield(s: &mut dyn SchSerializer, hs: &HarnessShieldData) -> Result<()> {
    export_harness_library_connectivity(s, &hs.rect, &hs.vault_guid, &hs.item_guid, &hs.revision_guid, &hs.design_item_id, &hs.source_library_name, &hs.library_path, &hs.lib_reference, hs.not_use_library_name, &hs.database_table_name)?;
    s.export_byte(hs.style, "Style")?;
    s.export_rotation_by90(hs.rotation, "Rotation")?;
    export_connected_objects(s, &hs.connected_wires, "ConnectedWiresUniqueIdsCount", "ConnectedWireUniqueId")?;
    export_connected_objects(s, &hs.connected_pin_wires, "ConnectedPinWiresUniqueIdsCount", "ConnectedPinWireUniqueId")?;
    s.export_boolean(hs.designator_locked, "DesignatorLocked")?;
    s.export_dynamic_string(&hs.comment, "Comment")?;
    s.export_byte(hs.component_kind, "ComponentKind")?;
    Ok(())
}

pub fn import_harness_shield(s: &mut dyn SchSerializer, hs: &mut HarnessShieldData) -> Result<()> {
    import_harness_library_connectivity(s, &mut hs.rect, &mut hs.vault_guid, &mut hs.item_guid, &mut hs.revision_guid, &mut hs.design_item_id, &mut hs.source_library_name, &mut hs.library_path, &mut hs.lib_reference, &mut hs.not_use_library_name, &mut hs.database_table_name)?;
    hs.style = s.import_byte("Style")?;
    hs.rotation = s.import_rotation_by90("Rotation")?;
    hs.connected_wires = import_connected_objects(s, "ConnectedWiresUniqueIdsCount", "ConnectedWireUniqueId")?;
    hs.connected_pin_wires = import_connected_objects(s, "ConnectedPinWiresUniqueIdsCount", "ConnectedPinWireUniqueId")?;
    hs.designator_locked = s.import_boolean("DesignatorLocked")?;
    hs.comment = s.import_dynamic_string("Comment")?;
    hs.component_kind = s.import_byte("ComponentKind")?;
    Ok(())
}

// ============================================================================
// HarnessTwist — Rectangle + library + twist-specific fields
// ============================================================================

pub fn export_harness_twist(s: &mut dyn SchSerializer, ht: &HarnessTwistData) -> Result<()> {
    export_harness_library_connectivity(s, &ht.rect, &ht.vault_guid, &ht.item_guid, &ht.revision_guid, &ht.design_item_id, &ht.source_library_name, &ht.library_path, &ht.lib_reference, ht.not_use_library_name, &ht.database_table_name)?;
    s.export_rotation_by90(ht.rotation, "Rotation")?;
    export_connected_objects(s, &ht.connected_wires, "ConnectedWiresUniqueIdsCount", "ConnectedWireUniqueId")?;
    s.export_boolean(ht.designator_locked, "DesignatorLocked")?;
    Ok(())
}

pub fn import_harness_twist(s: &mut dyn SchSerializer, ht: &mut HarnessTwistData) -> Result<()> {
    import_harness_library_connectivity(s, &mut ht.rect, &mut ht.vault_guid, &mut ht.item_guid, &mut ht.revision_guid, &mut ht.design_item_id, &mut ht.source_library_name, &mut ht.library_path, &mut ht.lib_reference, &mut ht.not_use_library_name, &mut ht.database_table_name)?;
    ht.rotation = s.import_rotation_by90("Rotation")?;
    ht.connected_wires = import_connected_objects(s, "ConnectedWiresUniqueIdsCount", "ConnectedWireUniqueId")?;
    ht.designator_locked = s.import_boolean("DesignatorLocked")?;
    Ok(())
}

// ============================================================================
// HarnessCable — Rectangle + library + cable-specific fields
// ============================================================================

pub fn export_harness_cable(s: &mut dyn SchSerializer, hc: &HarnessCableData) -> Result<()> {
    export_harness_library_connectivity(s, &hc.rect, &hc.vault_guid, &hc.item_guid, &hc.revision_guid, &hc.design_item_id, &hc.source_library_name, &hc.library_path, &hc.lib_reference, hc.not_use_library_name, &hc.database_table_name)?;
    s.export_rotation_by90(hc.rotation, "Rotation")?;
    export_connected_objects(s, &hc.connected_wires, "ConnectedWiresUniqueIdsCount", "ConnectedWireUniqueId")?;
    s.export_boolean(hc.designator_locked, "DesignatorLocked")?;
    s.export_byte(hc.component_kind, "ComponentKind")?;
    Ok(())
}

pub fn import_harness_cable(s: &mut dyn SchSerializer, hc: &mut HarnessCableData) -> Result<()> {
    import_harness_library_connectivity(s, &mut hc.rect, &mut hc.vault_guid, &mut hc.item_guid, &mut hc.revision_guid, &mut hc.design_item_id, &mut hc.source_library_name, &mut hc.library_path, &mut hc.lib_reference, &mut hc.not_use_library_name, &mut hc.database_table_name)?;
    hc.rotation = s.import_rotation_by90("Rotation")?;
    hc.connected_wires = import_connected_objects(s, "ConnectedWiresUniqueIdsCount", "ConnectedWireUniqueId")?;
    hc.designator_locked = s.import_boolean("DesignatorLocked")?;
    hc.component_kind = s.import_byte("ComponentKind")?;
    Ok(())
}

// ============================================================================
// HarnessAssociatedParts — just DataObject wrapper
// ============================================================================

pub fn export_harness_associated_parts(s: &mut dyn SchSerializer, hap: &HarnessAssociatedPartsData) -> Result<()> {
    export_data_object(s, &hap.base)
}

pub fn import_harness_associated_parts(s: &mut dyn SchSerializer, hap: &mut HarnessAssociatedPartsData) -> Result<()> {
    import_data_object(s, &mut hap.base)
}

// ============================================================================
// HarnessDocument — Sheet + harness length unit
// ============================================================================

pub fn export_harness_document(s: &mut dyn SchSerializer, hd: &HarnessDocumentData) -> Result<()> {
    export_sheet(s, &hd.sheet)?;
    s.export_byte(hd.harness_length_unit, "HarnessLengthUnit")?;
    Ok(())
}

pub fn import_harness_document(s: &mut dyn SchSerializer, hd: &mut HarnessDocumentData) -> Result<()> {
    import_sheet(s, &mut hd.sheet)?;
    hd.harness_length_unit = s.import_byte("HarnessLengthUnit")?;
    Ok(())
}

// ============================================================================
// HarnessWiringDiagram — delegates to HarnessDocument
// ============================================================================

pub fn export_harness_wiring_diagram(s: &mut dyn SchSerializer, hwd: &HarnessWiringDiagramData) -> Result<()> {
    export_harness_document(s, hwd)
}

pub fn import_harness_wiring_diagram(s: &mut dyn SchSerializer, hwd: &mut HarnessWiringDiagramData) -> Result<()> {
    import_harness_document(s, hwd)
}

// ============================================================================
// HarnessLayoutDrawing — delegates to HarnessDocument
// ============================================================================

pub fn export_harness_layout_drawing(s: &mut dyn SchSerializer, hld: &HarnessLayoutDrawingData) -> Result<()> {
    export_harness_document(s, hld)
}

pub fn import_harness_layout_drawing(s: &mut dyn SchSerializer, hld: &mut HarnessLayoutDrawingData) -> Result<()> {
    import_harness_document(s, hld)
}

// ============================================================================
// HarnessComponent — delegates to Component
// ============================================================================

pub fn export_harness_component(s: &mut dyn SchSerializer, hc: &HarnessComponentData) -> Result<()> {
    super::component::export_component(s, hc)
}

pub fn import_harness_component(s: &mut dyn SchSerializer, hc: &mut HarnessComponentData) -> Result<()> {
    super::component::import_component(s, hc)
}
