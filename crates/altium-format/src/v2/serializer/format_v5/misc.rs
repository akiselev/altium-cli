//! Format functions for miscellaneous record types.

use crate::error::Result;
use crate::v2::fields::misc::*;
use crate::v2::fields::harness::HarnessConnectorData;
use crate::v2::serializer::SchSerializer;
use crate::v2::types::*;
use super::{export_graphical_object, import_graphical_object, export_vertices, import_vertices};
use super::sheet::{export_rectangular_entry_container, import_rectangular_entry_container};
use super::schematic::{export_power, import_power, export_label, import_label, export_text_frame, import_text_frame};
use super::parameter::{export_parameter, import_parameter};
use super::sheet::{export_sheet, import_sheet, export_sheet_entry, import_sheet_entry, export_sheet_symbol, import_sheet_symbol};

// ============================================================================
// ErrorMarker
// ============================================================================

pub fn export_error_marker(s: &mut dyn SchSerializer, em: &ErrorMarkerData) -> Result<()> {
    export_graphical_object(s, &em.graphical)?;
    s.export_coord(em.location_x, "Location.X")?;
    s.export_coord(em.location_y, "Location.Y")?;
    s.export_color(em.color, "Color")?;
    Ok(())
}

pub fn import_error_marker(s: &mut dyn SchSerializer, em: &mut ErrorMarkerData) -> Result<()> {
    import_graphical_object(s, &mut em.graphical)?;
    em.location_x = s.import_coord("Location.X")?;
    em.location_y = s.import_coord("Location.Y")?;
    em.color = s.import_color("Color")?;
    Ok(())
}

// ============================================================================
// ClipBoard
// ============================================================================

pub fn export_clipboard(s: &mut dyn SchSerializer, cb: &ClipBoardData) -> Result<()> {
    export_graphical_object(s, &cb.graphical)?;
    s.export_coord(cb.location_x, "Location.X")?;
    s.export_coord(cb.location_y, "Location.Y")?;
    Ok(())
}

pub fn import_clipboard(s: &mut dyn SchSerializer, cb: &mut ClipBoardData) -> Result<()> {
    import_graphical_object(s, &mut cb.graphical)?;
    cb.location_x = s.import_coord("Location.X")?;
    cb.location_y = s.import_coord("Location.Y")?;
    Ok(())
}

// ============================================================================
// HarnessConnector
// ============================================================================

pub fn export_harness_connector(s: &mut dyn SchSerializer, hc: &HarnessConnectorData) -> Result<()> {
    export_rectangular_entry_container(s, &hc.container)?;
    s.export_coord(hc.primary_connection_position, "PrimaryConnectionPosition")?;
    s.export_left_right_side(hc.harness_connector_side, "HarnessConnectorSide")?;
    s.export_dynamic_string(&hc.unique_id, "UniqueID")?;
    Ok(())
}

pub fn import_harness_connector(s: &mut dyn SchSerializer, hc: &mut HarnessConnectorData) -> Result<()> {
    import_rectangular_entry_container(s, &mut hc.container)?;
    hc.primary_connection_position = s.import_coord("PrimaryConnectionPosition")?;
    hc.harness_connector_side = s.import_left_right_side("HarnessConnectorSide")?;
    hc.unique_id = s.import_dynamic_string("UniqueID")?;
    Ok(())
}

// ============================================================================
// RTFLink
// ============================================================================

pub fn export_rtf_link(s: &mut dyn SchSerializer, r: &RTFLinkData) -> Result<()> {
    export_rectangular_entry_container(s, &r.container)?;
    s.export_dynamic_string(&r.file_name_rtf, "FileNameRTF")?;
    s.export_boolean(r.collapsed, "Collapsed")?;
    Ok(())
}

pub fn import_rtf_link(s: &mut dyn SchSerializer, r: &mut RTFLinkData) -> Result<()> {
    import_rectangular_entry_container(s, &mut r.container)?;
    r.file_name_rtf = s.import_dynamic_string("FileNameRTF")?;
    r.collapsed = s.import_boolean("Collapsed")?;
    Ok(())
}

// ============================================================================
// RichTextDocument
// ============================================================================

pub fn export_rich_text_document(s: &mut dyn SchSerializer, rtd: &RichTextDocumentData) -> Result<()> {
    export_graphical_object(s, &rtd.graphical)?;
    s.export_coord(rtd.location_x, "Location.X")?;
    s.export_coord(rtd.location_y, "Location.Y")?;
    s.export_coord(rtd.corner_x, "Corner.X")?;
    s.export_coord(rtd.corner_y, "Corner.Y")?;
    s.export_size(rtd.line_width, "LineWidth")?;
    s.export_color(rtd.color, "Color")?;
    s.export_color(rtd.area_color, "AreaColor")?;
    s.export_boolean(rtd.is_solid, "IsSolid")?;
    s.export_boolean(rtd.show_border, "ShowBorder")?;
    s.export_binary(&rtd.rtf_stream, "RTFStream")?;
    Ok(())
}

pub fn import_rich_text_document(s: &mut dyn SchSerializer, rtd: &mut RichTextDocumentData) -> Result<()> {
    import_graphical_object(s, &mut rtd.graphical)?;
    rtd.location_x = s.import_coord("Location.X")?;
    rtd.location_y = s.import_coord("Location.Y")?;
    rtd.corner_x = s.import_coord("Corner.X")?;
    rtd.corner_y = s.import_coord("Corner.Y")?;
    rtd.line_width = s.import_size("LineWidth")?;
    rtd.color = s.import_color("Color")?;
    rtd.area_color = s.import_color("AreaColor")?;
    rtd.is_solid = s.import_boolean("IsSolid")?;
    rtd.show_border = s.import_boolean("ShowBorder")?;
    rtd.rtf_stream = s.import_binary("RTFStream")?;
    Ok(())
}

// ============================================================================
// CompileMask
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
// Delegate record types — these just call existing export/import functions
// ============================================================================

/// CrossSheetConnector — delegates to Power.
pub fn export_cross_sheet_connector(s: &mut dyn SchSerializer, csc: &CrossSheetConnectorData) -> Result<()> {
    export_power(s, csc)
}

pub fn import_cross_sheet_connector(s: &mut dyn SchSerializer, csc: &mut CrossSheetConnectorData) -> Result<()> {
    import_power(s, csc)
}

/// Hyperlink — delegates to Label.
pub fn export_hyperlink(s: &mut dyn SchSerializer, h: &HyperlinkData) -> Result<()> {
    export_label(s, h)
}

pub fn import_hyperlink(s: &mut dyn SchSerializer, h: &mut HyperlinkData) -> Result<()> {
    import_label(s, h)
}

/// ImageParameter — delegates to Parameter.
pub fn export_image_parameter(s: &mut dyn SchSerializer, ip: &ImageParameterData) -> Result<()> {
    export_parameter(s, ip)
}

pub fn import_image_parameter(s: &mut dyn SchSerializer, ip: &mut ImageParameterData) -> Result<()> {
    import_parameter(s, ip)
}

/// FunctionalTextFrame — delegates to TextFrame.
pub fn export_functional_text_frame(s: &mut dyn SchSerializer, ftf: &FunctionalTextFrameData) -> Result<()> {
    export_text_frame(s, ftf)
}

pub fn import_functional_text_frame(s: &mut dyn SchSerializer, ftf: &mut FunctionalTextFrameData) -> Result<()> {
    import_text_frame(s, ftf)
}

/// ElectronicsSystemDesignDocument — delegates to Sheet.
pub fn export_electronics_system_design_document(s: &mut dyn SchSerializer, esdd: &ElectronicsSystemDesignDocumentData) -> Result<()> {
    export_sheet(s, esdd)
}

pub fn import_electronics_system_design_document(s: &mut dyn SchSerializer, esdd: &mut ElectronicsSystemDesignDocumentData) -> Result<()> {
    import_sheet(s, esdd)
}

/// TaskHolder — empty (C# has empty export/import bodies).
pub fn export_task_holder(_s: &mut dyn SchSerializer, _th: &TaskHolderData) -> Result<()> {
    Ok(())
}

pub fn import_task_holder(_s: &mut dyn SchSerializer, _th: &mut TaskHolderData) -> Result<()> {
    Ok(())
}

/// HighLevelCodeEntry — delegates to SheetEntry.
pub fn export_high_level_code_entry(s: &mut dyn SchSerializer, hlce: &crate::v2::fields::harness::HighLevelCodeEntryData) -> Result<()> {
    export_sheet_entry(s, hlce)
}

pub fn import_high_level_code_entry(s: &mut dyn SchSerializer, hlce: &mut crate::v2::fields::harness::HighLevelCodeEntryData) -> Result<()> {
    import_sheet_entry(s, hlce)
}

/// HighLevelCodeSymbol — delegates to SheetSymbol.
pub fn export_high_level_code_symbol(s: &mut dyn SchSerializer, hlcs: &crate::v2::fields::harness::HighLevelCodeSymbolData) -> Result<()> {
    export_sheet_symbol(s, hlcs)
}

pub fn import_high_level_code_symbol(s: &mut dyn SchSerializer, hlcs: &mut crate::v2::fields::harness::HighLevelCodeSymbolData) -> Result<()> {
    import_sheet_symbol(s, hlcs)
}
