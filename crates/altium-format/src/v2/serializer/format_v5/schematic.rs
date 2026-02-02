//! Format functions for schematic connectivity record types.

use crate::error::Result;
use crate::v2::fields::schematic::*;
use crate::v2::serializer::SchSerializer;
use super::{export_graphical_object, import_graphical_object, export_vertices, import_vertices};

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
// Probe
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
// NoERC
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
// Symbol
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
// Note
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
// TextFrame
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
// BusEntry
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
// SignalHarness
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
