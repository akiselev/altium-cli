//! Format functions for sheet/document record types.

use crate::error::Result;
use crate::v2::fields::*;
use crate::v2::fields::sheet::{LibraryData, FontEntry};
use crate::v2::serializer::SchSerializer;
use super::{export_graphical_object, import_graphical_object};

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

pub fn export_sheet(s: &mut dyn SchSerializer, sheet: &SheetData) -> Result<()> {
    // Font table
    s.export_long_int(sheet.font_id_count, "FontIdCount")?;
    for (i, font) in sheet.fonts.iter().enumerate() {
        let n = i + 1;
        s.export_string(&font.font_name, &format!("FontName{}", n))?;
        s.export_short_int(font.size, &format!("Size{}", n))?;
        s.export_short_int(font.rotation, &format!("Rotation{}", n))?;
        s.export_boolean(font.italic, &format!("Italic{}", n))?;
        s.export_boolean(font.bold, &format!("Bold{}", n))?;
        s.export_boolean(font.underline, &format!("Underline{}", n))?;
        s.export_boolean(font.strike_out, &format!("StrikeOut{}", n))?;
    }
    // Document fields
    s.export_boolean(sheet.use_mbcs, "UseMBCS")?;
    s.export_boolean(sheet.is_boc, "IsBOC")?;
    s.export_boolean(sheet.hot_spot_grid_on, "HotSpotGridOn")?;
    s.export_coord(sheet.hot_spot_grid_size, "HotSpotGridSize")?;
    s.export_byte(sheet.sheet_style, "SheetStyle")?;
    s.export_font_id(sheet.system_font, "SystemFont")?;
    s.export_byte(sheet.document_border_style, "DocumentBorderStyle")?;
    s.export_byte(sheet.workspace_orientation, "WorkspaceOrientation")?;
    s.export_boolean(sheet.border_on, "BorderOn")?;
    s.export_boolean(sheet.title_block_on, "TitleBlockOn")?;
    s.export_long_int(sheet.sheet_number_space_size, "SheetNumberSpaceSize")?;
    s.export_color(sheet.color, "Color")?;
    s.export_color(sheet.area_color, "AreaColor")?;
    s.export_boolean(sheet.snap_grid_on, "SnapGridOn")?;
    s.export_coord(sheet.snap_grid_size, "SnapGridSize")?;
    s.export_boolean(sheet.visible_grid_on, "VisibleGridOn")?;
    s.export_coord(sheet.visible_grid_size, "VisibleGridSize")?;
    s.export_coord(sheet.custom_x, "CustomX")?;
    s.export_coord(sheet.custom_y, "CustomY")?;
    s.export_boolean(sheet.use_custom_sheet, "UseCustomSheet")?;
    s.export_boolean(sheet.show_hidden_pins, "ShowHiddenPins")?;
    s.export_boolean(!sheet.reference_zones_on, "ReferenceZonesOn")?;
    s.export_long_int(sheet.custom_x_zones, "CustomXZones")?;
    s.export_long_int(sheet.custom_y_zones, "CustomYZones")?;
    s.export_coord(sheet.custom_margin_width, "CustomMarginWidth")?;
    s.export_boolean(sheet.show_template_graphics, "ShowTemplateGraphics")?;
    s.export_string(&sheet.template_file_name, "TemplateFileName")?;
    s.export_byte(sheet.display_unit, "Display_Unit")?;
    s.export_byte(sheet.reference_zone_style, "ReferenceZoneStyle")?;
    s.export_boolean(sheet.always_show_cd, "AlwaysShowCD")?;
    s.export_dynamic_string(&sheet.release_vault_guid, "ReleaseVaultGUID")?;
    s.export_dynamic_string(&sheet.release_item_guid, "ReleaseItemGUID")?;
    s.export_dynamic_string(&sheet.item_revision_guid, "ItemRevisionGUID")?;
    s.export_dynamic_string(&sheet.props_vault_guid, "PropsVaultGUID")?;
    s.export_dynamic_string(&sheet.props_revision_guid, "PropsRevisionGUID")?;
    s.export_dynamic_string(&sheet.file_version_info, "FileVersionInfo")?;
    Ok(())
}

pub fn import_sheet(s: &mut dyn SchSerializer, sheet: &mut SheetData) -> Result<()> {
    // Font table
    sheet.font_id_count = s.import_long_int("FontIdCount")?;
    sheet.fonts.clear();
    for i in 0..sheet.font_id_count {
        let n = (i + 1) as usize;
        let font = FontEntry {
            font_name: s.import_string(&format!("FontName{}", n))?,
            size: s.import_short_int(&format!("Size{}", n))?,
            rotation: s.import_short_int(&format!("Rotation{}", n))?,
            italic: s.import_boolean(&format!("Italic{}", n))?,
            bold: s.import_boolean(&format!("Bold{}", n))?,
            underline: s.import_boolean(&format!("Underline{}", n))?,
            strike_out: s.import_boolean(&format!("StrikeOut{}", n))?,
        };
        sheet.fonts.push(font);
    }
    // Document fields
    sheet.use_mbcs = s.import_boolean("UseMBCS")?;
    sheet.is_boc = s.import_boolean("IsBOC")?;
    sheet.hot_spot_grid_on = s.import_boolean("HotSpotGridOn")?;
    sheet.hot_spot_grid_size = s.import_coord("HotSpotGridSize")?;
    sheet.sheet_style = s.import_byte("SheetStyle")?;
    sheet.system_font = s.import_font_id("SystemFont")?;
    sheet.document_border_style = s.import_byte("DocumentBorderStyle")?;
    sheet.workspace_orientation = s.import_byte("WorkspaceOrientation")?;
    sheet.border_on = s.import_boolean("BorderOn")?;
    sheet.title_block_on = s.import_boolean("TitleBlockOn")?;
    sheet.sheet_number_space_size = s.import_long_int("SheetNumberSpaceSize")?;
    sheet.color = s.import_color("Color")?;
    sheet.area_color = s.import_color("AreaColor")?;
    sheet.snap_grid_on = s.import_boolean("SnapGridOn")?;
    sheet.snap_grid_size = s.import_coord("SnapGridSize")?;
    sheet.visible_grid_on = s.import_boolean("VisibleGridOn")?;
    sheet.visible_grid_size = s.import_coord("VisibleGridSize")?;
    sheet.custom_x = s.import_coord("CustomX")?;
    sheet.custom_y = s.import_coord("CustomY")?;
    sheet.use_custom_sheet = s.import_boolean("UseCustomSheet")?;
    sheet.show_hidden_pins = s.import_boolean("ShowHiddenPins")?;
    let ref_zones_inv = s.import_boolean("ReferenceZonesOn")?;
    sheet.reference_zones_on = !ref_zones_inv;
    sheet.custom_x_zones = s.import_long_int("CustomXZones")?;
    sheet.custom_y_zones = s.import_long_int("CustomYZones")?;
    sheet.custom_margin_width = s.import_coord("CustomMarginWidth")?;
    sheet.show_template_graphics = s.import_boolean("ShowTemplateGraphics")?;
    sheet.template_file_name = s.import_string("TemplateFileName")?;
    sheet.display_unit = s.import_byte("Display_Unit")?;
    sheet.reference_zone_style = s.import_byte("ReferenceZoneStyle")?;
    sheet.always_show_cd = s.import_boolean("AlwaysShowCD")?;
    sheet.release_vault_guid = s.import_dynamic_string("ReleaseVaultGUID")?;
    sheet.release_item_guid = s.import_dynamic_string("ReleaseItemGUID")?;
    sheet.item_revision_guid = s.import_dynamic_string("ItemRevisionGUID")?;
    sheet.props_vault_guid = s.import_dynamic_string("PropsVaultGUID")?;
    sheet.props_revision_guid = s.import_dynamic_string("PropsRevisionGUID")?;
    sheet.file_version_info = s.import_dynamic_string("FileVersionInfo")?;
    Ok(())
}

// ============================================================================
// Library (ExportLibrary / ImportLibrary)
// ============================================================================

pub fn export_library(s: &mut dyn SchSerializer, lib: &LibraryData) -> Result<()> {
    // Font table (same as Sheet)
    s.export_long_int(lib.font_id_count, "FontIdCount")?;
    for (i, font) in lib.fonts.iter().enumerate() {
        let n = i + 1;
        s.export_string(&font.font_name, &format!("FontName{}", n))?;
        s.export_short_int(font.size, &format!("Size{}", n))?;
        s.export_short_int(font.rotation, &format!("Rotation{}", n))?;
        s.export_boolean(font.italic, &format!("Italic{}", n))?;
        s.export_boolean(font.bold, &format!("Bold{}", n))?;
        s.export_boolean(font.underline, &format!("Underline{}", n))?;
        s.export_boolean(font.strike_out, &format!("StrikeOut{}", n))?;
    }
    // Library-specific fields
    s.export_boolean(lib.use_mbcs, "UseMBCS")?;
    s.export_boolean(lib.is_boc, "IsBOC")?;
    s.export_dynamic_string(&lib.description, "Description")?;
    s.export_byte(lib.document_border_style, "DocumentBorderStyle")?;
    s.export_byte(lib.sheet_style, "SheetStyle")?;
    s.export_byte(lib.workspace_orientation, "WorkspaceOrientation")?;
    s.export_boolean(lib.border_on, "BorderOn")?;
    s.export_boolean(lib.title_block_on, "TitleBlockOn")?;
    s.export_long_int(lib.sheet_number_space_size, "SheetNumberSpaceSize")?;
    s.export_color(lib.color, "Color")?;
    s.export_color(lib.area_color, "AreaColor")?;
    s.export_boolean(lib.snap_grid_on, "SnapGridOn")?;
    s.export_coord(lib.snap_grid_size, "SnapGridSize")?;
    s.export_boolean(lib.visible_grid_on, "VisibleGridOn")?;
    s.export_coord(lib.visible_grid_size, "VisibleGridSize")?;
    s.export_coord(lib.custom_x, "CustomX")?;
    s.export_coord(lib.custom_y, "CustomY")?;
    s.export_boolean(lib.use_custom_sheet, "UseCustomSheet")?;
    s.export_boolean(lib.show_hidden_pins, "ShowHiddenPins")?;
    s.export_boolean(!lib.reference_zones_on, "ReferenceZonesOn")?;
    s.export_byte(lib.display_unit, "Display_Unit")?;
    s.export_boolean(lib.always_show_cd, "AlwaysShowCD")?;
    s.export_dynamic_string(&lib.release_vault_guid, "ReleaseVaultGUID")?;
    s.export_dynamic_string(&lib.folder_guid, "FolderGUID")?;
    s.export_dynamic_string(&lib.life_cycle_definition_guid, "LifeCycleDefinitionGUID")?;
    s.export_dynamic_string(&lib.revision_naming_scheme_guid, "RevisionNamingSchemeGUID")?;
    Ok(())
}

pub fn import_library(s: &mut dyn SchSerializer, lib: &mut LibraryData) -> Result<()> {
    // Font table
    lib.font_id_count = s.import_long_int("FontIdCount")?;
    lib.fonts.clear();
    for i in 0..lib.font_id_count {
        let n = (i + 1) as usize;
        let font = FontEntry {
            font_name: s.import_string(&format!("FontName{}", n))?,
            size: s.import_short_int(&format!("Size{}", n))?,
            rotation: s.import_short_int(&format!("Rotation{}", n))?,
            italic: s.import_boolean(&format!("Italic{}", n))?,
            bold: s.import_boolean(&format!("Bold{}", n))?,
            underline: s.import_boolean(&format!("Underline{}", n))?,
            strike_out: s.import_boolean(&format!("StrikeOut{}", n))?,
        };
        lib.fonts.push(font);
    }
    // Library-specific fields
    lib.use_mbcs = s.import_boolean("UseMBCS")?;
    lib.is_boc = s.import_boolean("IsBOC")?;
    lib.description = s.import_dynamic_string("Description")?;
    lib.document_border_style = s.import_byte("DocumentBorderStyle")?;
    lib.sheet_style = s.import_byte("SheetStyle")?;
    lib.workspace_orientation = s.import_byte("WorkspaceOrientation")?;
    lib.border_on = s.import_boolean("BorderOn")?;
    lib.title_block_on = s.import_boolean("TitleBlockOn")?;
    lib.sheet_number_space_size = s.import_long_int("SheetNumberSpaceSize")?;
    lib.color = s.import_color("Color")?;
    lib.area_color = s.import_color("AreaColor")?;
    lib.snap_grid_on = s.import_boolean("SnapGridOn")?;
    lib.snap_grid_size = s.import_coord("SnapGridSize")?;
    lib.visible_grid_on = s.import_boolean("VisibleGridOn")?;
    lib.visible_grid_size = s.import_coord("VisibleGridSize")?;
    lib.custom_x = s.import_coord("CustomX")?;
    lib.custom_y = s.import_coord("CustomY")?;
    lib.use_custom_sheet = s.import_boolean("UseCustomSheet")?;
    lib.show_hidden_pins = s.import_boolean("ShowHiddenPins")?;
    let ref_zones_inv = s.import_boolean("ReferenceZonesOn")?;
    lib.reference_zones_on = !ref_zones_inv;
    lib.display_unit = s.import_byte("Display_Unit")?;
    lib.always_show_cd = s.import_boolean("AlwaysShowCD")?;
    lib.release_vault_guid = s.import_dynamic_string("ReleaseVaultGUID")?;
    lib.folder_guid = s.import_dynamic_string("FolderGUID")?;
    lib.life_cycle_definition_guid = s.import_dynamic_string("LifeCycleDefinitionGUID")?;
    lib.revision_naming_scheme_guid = s.import_dynamic_string("RevisionNamingSchemeGUID")?;
    Ok(())
}
