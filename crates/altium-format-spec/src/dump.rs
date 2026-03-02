//! Reverse generation: produce `.schlib-spec` or `.pcblib-spec` source from
//! existing Altium library documents.
//!
//! Generated output uses absolute placement only (`at: (x, y)`, explicit
//! `orientation:`). No anchors, rows, grids, or template bindings are emitted.

use altium_format::{AltiumProject, PcbLib, SchDoc, SchLib};
use altium_format::api::{
    Component, Pin, Parameter, FootprintMap, Graphic,
    SheetObject, ComponentChild, SheetSymbolChild,
};
use altium_format_types::coord::Coord;
use altium_format_types::project::{
    ChannelRoomNamingStyle, CrossRefLocationStyle, CrossRefPorts, CrossRefSheetStyle,
    ErrorLevel, FlattenMode,
};
use indexmap::IndexMap;

// ── Public API ────────────────────────────────────────────────────────────────

/// Generate `.pcblib-spec` source from a PcbLib document.
pub fn dump_pcblib(lib: &PcbLib) -> String {
    let mut out = String::new();
    for fp in &lib.dump_footprints() {
        dump_footprint(&mut out, fp);
        out.push('\n');
    }
    out
}

/// Generate `.schlib-spec` source from a SchLib document.
pub fn dump_schlib(lib: &SchLib) -> Result<String, altium_format::AltiumFormatError> {
    let mut out = String::new();
    for comp in &lib.components()? {
        dump_component(&mut out, comp);
        out.push('\n');
    }
    Ok(out)
}

/// Generate `.prjpcb-spec` source from a PrjPcb project.
///
/// Returns `Err` if the project cannot be parsed into its typed representation.
pub fn dump_prjpcb(doc: &AltiumProject) -> Result<String, crate::eval::SpecError> {
    let project = doc.project()
        .map_err(|e| crate::eval::SpecError::no_span(
            crate::eval::SpecErrorCode::AltiumFormat,
            e.to_string(),
        ))?;

    let mut out = String::new();
    out.push_str(&format!("project {} {{\n", quote_entity_name(&project.name)));

    // [Design] scalar properties — only emit non-default values
    if project.hierarchy_mode != FlattenMode::Smart {
        out.push_str(&format!("    hierarchy_mode: {}\n", flatten_mode_to_spec(project.hierarchy_mode)?));
    }
    if project.channel_room_naming_style != ChannelRoomNamingStyle::FlatNumericWithNames {
        out.push_str(&format!("    channel_room_naming_style: {}\n", channel_room_naming_to_spec(project.channel_room_naming_style)?));
    }
    if !project.channel_designator_format.is_empty() {
        out.push_str(&format!("    channel_designator_format: {}\n", quote_string(&project.channel_designator_format)));
    }
    if !project.channel_room_level_separator.is_empty() {
        out.push_str(&format!("    channel_room_level_separator: {}\n", quote_string(&project.channel_room_level_separator)));
    }
    if project.allow_port_net_names { out.push_str("    allow_port_net_names: true\n"); }
    if project.allow_sheet_entry_net_names { out.push_str("    allow_sheet_entry_net_names: true\n"); }
    if project.netlist_single_pin_nets { out.push_str("    netlist_single_pin_nets: true\n"); }
    if project.append_sheet_number_to_local_nets { out.push_str("    append_sheet_number_to_local_nets: true\n"); }
    if project.name_nets_hierarchically { out.push_str("    name_nets_hierarchically: true\n"); }
    if project.power_port_names_take_priority { out.push_str("    power_port_names_take_priority: true\n"); }
    if project.pin_swap_by_netlabel { out.push_str("    pin_swap_by_netlabel: true\n"); }
    if project.pin_swap_by_pin { out.push_str("    pin_swap_by_pin: true\n"); }
    if project.cross_ref_sheet_style != CrossRefSheetStyle::None {
        out.push_str(&format!("    cross_ref_sheet_style: {}\n", cross_ref_sheet_to_spec(project.cross_ref_sheet_style)?));
    }
    if project.cross_ref_location_style != CrossRefLocationStyle::None {
        out.push_str(&format!("    cross_ref_location_style: {}\n", cross_ref_location_to_spec(project.cross_ref_location_style)?));
    }
    if project.cross_ref_ports != CrossRefPorts::Disabled {
        out.push_str(&format!("    cross_ref_ports: {}\n", cross_ref_ports_to_spec(project.cross_ref_ports)?));
    }
    if project.cross_ref_cross_sheets { out.push_str("    cross_ref_cross_sheets: true\n"); }
    if project.cross_ref_sheet_entries { out.push_str("    cross_ref_sheet_entries: true\n"); }
    if !project.output_path.is_empty() {
        out.push_str(&format!("    output_path: {}\n", quote_string(&project.output_path)));
    }

    out.push('\n');

    // Documents
    for doc_ref in &project.documents {
        out.push_str(&format!("    document {} {{\n", quote_string(&doc_ref.path)));
        if doc_ref.annotation_enabled { out.push_str("        annotation_enabled: true\n"); }
        if doc_ref.annotate_start_value != 0 {
            out.push_str(&format!("        annotate_start_value: {}\n", doc_ref.annotate_start_value));
        }
        if doc_ref.do_library_update { out.push_str("        do_library_update: true\n"); }
        if doc_ref.do_database_update { out.push_str("        do_database_update: true\n"); }
        out.push_str("    }\n\n");
    }

    // ERC matrix (only non-default cells)
    let mut erc_overrides = Vec::new();
    for (row_idx, row) in project.erc_matrix.cells.iter().enumerate() {
        for (col_idx, &level) in row.iter().enumerate() {
            if level != ErrorLevel::NoReport {
                erc_overrides.push((row_idx, col_idx, level));
            }
        }
    }
    if !erc_overrides.is_empty() {
        out.push_str("    erc_matrix {\n");
        for (row_idx, col_idx, level) in &erc_overrides {
            let row_code = connection_code_to_spec(*row_idx)?;
            let col_code = connection_code_to_spec(*col_idx)?;
            let level_str = error_level_to_spec(*level)?;
            out.push_str(&format!("        ({row_code}, {col_code}): {level_str}\n"));
        }
        out.push_str("    }\n\n");
    }

    // Output groups
    for group in &project.output_groups {
        out.push_str(&format!("    output_group {} {{\n", quote_string(&group.name)));
        if !group.description.is_empty() {
            out.push_str(&format!("        description: {}\n", quote_string(&group.description)));
        }
        for output in &group.outputs {
            out.push_str(&format!("        output {} {{\n", quote_string(&output.name)));
            if !output.output_type.is_empty() {
                out.push_str(&format!("            output_type: {}\n", quote_string(&output.output_type)));
            }
            if !output.document_path.is_empty() {
                out.push_str(&format!("            document_path: {}\n", quote_string(&output.document_path)));
            }
            out.push_str("        }\n");
        }
        out.push_str("    }\n\n");
    }

    // Variants
    for var in &project.variants {
        out.push_str(&format!("    variant {} {{\n", quote_string(&var.description)));
        for v in &var.variations {
            out.push_str(&format!("        variation {} {{\n", quote_entity_name(&v.designator)));
            out.push_str(&format!("            kind: {}\n", variation_kind_to_spec(v.kind)?));
            if !v.alternate_part.is_empty() {
                out.push_str(&format!("            alternate_part: {}\n", quote_string(&v.alternate_part)));
            }
            out.push_str("        }\n");
        }
        for pv in &var.param_variations {
            out.push_str(&format!("        param_variation {} {{\n", quote_entity_name(&pv.designator)));
            out.push_str(&format!("            parameter: {}\n", quote_string(&pv.parameter_name)));
            out.push_str(&format!("            value: {}\n", quote_string(&pv.variant_value)));
            out.push_str("        }\n");
        }
        out.push_str("    }\n\n");
    }

    out.push_str("}\n");
    Ok(out)
}

/// Generate `.schdoc-spec` source from a SchDoc document.
pub fn dump_schdoc(doc: &SchDoc) -> Result<String, altium_format::AltiumFormatError> {
    let sheet = doc.sheet()?;
    let mut out = String::new();

    out.push_str("sheet {\n");

    // Font table
    if !sheet.fonts.is_empty() {
        out.push_str("    fonts {\n");
        for f in &sheet.fonts {
            let mut props = vec![format!("name: {}", quote_string(&f.name))];
            props.push(format!("size: {}", f.size));
            if f.bold { props.push("bold: true".to_owned()); }
            if f.italic { props.push("italic: true".to_owned()); }
            if f.underline { props.push("underline: true".to_owned()); }
            if f.strikeout { props.push("strikeout: true".to_owned()); }
            if f.rotation != 0 { props.push(format!("rotation: {}", f.rotation)); }
            out.push_str(&format!("        font {} {{ {} }}\n", f.id, props.join(", ")));
        }
        out.push_str("    }\n\n");
    }

    // Sheet properties (non-default only)
    if sheet.use_custom_sheet {
        out.push_str(&format!("    custom_width: {}\n", sheet.custom_width));
        out.push_str(&format!("    custom_height: {}\n", sheet.custom_height));
    }
    if !sheet.snap_grid_on { out.push_str("    snap_grid_on: false\n"); }
    if !sheet.visible_grid_on { out.push_str("    visible_grid_on: false\n"); }
    if !sheet.hot_spot_grid_on { out.push_str("    hot_spot_grid_on: false\n"); }
    if sheet.show_hidden_pins { out.push_str("    show_hidden_pins: true\n"); }
    if !sheet.border_on { out.push_str("    border_on: false\n"); }
    if !sheet.title_block_on { out.push_str("    title_block_on: false\n"); }

    out.push_str("}\n");

    // Objects (top-level, outside the sheet metadata block)
    if !sheet.objects.is_empty() {
        out.push('\n');
        for obj in &sheet.objects {
            dump_sheet_object(&mut out, obj, 0);
        }
    }

    Ok(out)
}

fn dump_sheet_object(out: &mut String, obj: &SheetObject, indent: usize) {
    let pad = " ".repeat(indent);
    match obj {
        SheetObject::Component(comp) => dump_schdoc_component(out, comp, indent),
        SheetObject::Wire(w) => {
            let verts: Vec<String> = w.vertices.iter().map(|v| format!("{}", v)).collect();
            out.push_str(&format!("{}wire {{ vertices: [{}] }}\n", pad, verts.join(", ")));
        }
        SheetObject::Bus(b) => {
            let verts: Vec<String> = b.vertices.iter().map(|v| format!("{}", v)).collect();
            out.push_str(&format!("{}bus {{ vertices: [{}] }}\n", pad, verts.join(", ")));
        }
        SheetObject::NetLabel(n) => {
            out.push_str(&format!(
                "{}net_label {} {{ at: {}, orientation: {} }}\n",
                pad, quote_entity_name(&n.text), n.location, n.orientation
            ));
        }
        SheetObject::PowerObject(p) => {
            let mut props = vec![
                format!("at: {}", p.location),
                format!("orientation: {}", p.orientation),
            ];
            if p.show_net_name { props.push("show_net_name: true".to_owned()); }
            out.push_str(&format!(
                "{}power_object {} {{ {} }}\n",
                pad, quote_entity_name(&p.text), props.join(", ")
            ));
        }
        SheetObject::Port(p) => {
            out.push_str(&format!(
                "{}port {} {{ at: {} }}\n",
                pad, quote_entity_name(&p.name), p.location
            ));
        }
        SheetObject::Junction(j) => {
            out.push_str(&format!("{}junction {{ at: {} }}\n", pad, j.location));
        }
        SheetObject::NoConnect(n) => {
            out.push_str(&format!("{}no_connect {{ at: {} }}\n", pad, n.location));
        }
        SheetObject::BusEntry(b) => {
            out.push_str(&format!(
                "{}bus_entry {{ at: {}, corner: {} }}\n",
                pad, b.location, b.corner
            ));
        }
        SheetObject::SheetSymbol(ss) => dump_schdoc_sheet_symbol(out, ss, indent),
        SheetObject::ParameterSet(ps) => {
            out.push_str(&format!("{}parameter_set {} {{\n", pad, quote_entity_name(&ps.name)));
            for param in &ps.parameters {
                dump_parameter(out, param, indent + 4);
            }
            out.push_str(&format!("{}}}\n", pad));
        }
        SheetObject::Note(n) => {
            out.push_str(&format!(
                "{}note {{ at: {}, text: {} }}\n",
                pad, n.location, quote_string(&n.text)
            ));
        }
        SheetObject::Probe(p) => {
            out.push_str(&format!(
                "{}probe {} {{ at: {} }}\n",
                pad, quote_entity_name(&p.name), p.location
            ));
        }
        SheetObject::CompileMask(c) => {
            out.push_str(&format!(
                "{}compile_mask {{ at: {}, corner: {} }}\n",
                pad, c.location, c.corner
            ));
        }
        SheetObject::Blanket(b) => {
            let verts: Vec<String> = b.vertices.iter().map(|v| format!("{}", v)).collect();
            out.push_str(&format!(
                "{}blanket {{ at: {}, corner: {}, vertices: [{}] }}\n",
                pad, b.location, b.corner, verts.join(", ")
            ));
        }
        SheetObject::Graphic(g) => dump_graphic(out, g, indent),
        SheetObject::Parameter(p) => dump_parameter(out, p, indent),
        SheetObject::HarnessConnector(hc) => {
            out.push_str(&format!("{}harness_connector {{ at: {} }}\n", pad, hc.location));
        }
        SheetObject::SignalHarness(sh) => {
            let verts: Vec<String> = sh.vertices.iter().map(|v| format!("{}", v)).collect();
            out.push_str(&format!("{}signal_harness {{ vertices: [{}] }}\n", pad, verts.join(", ")));
        }
    }
}

fn dump_schdoc_component(out: &mut String, comp: &altium_format::api::SchDocComponent, indent: usize) {
    let pad = " ".repeat(indent);
    let name = if !comp.designator.is_empty() {
        quote_entity_name(&comp.designator)
    } else {
        quote_entity_name(&comp.lib_reference)
    };
    out.push_str(&format!("{}component {} {{\n", pad, name));
    out.push_str(&format!("{}    lib_reference: {}\n", pad, quote_string(&comp.lib_reference)));
    out.push_str(&format!("{}    at: {}\n", pad, comp.location));
    if comp.orientation != altium_format_types::RotationBy90::Rotate0 {
        out.push_str(&format!("{}    orientation: {}\n", pad, comp.orientation));
    }
    if comp.is_mirrored {
        out.push_str(&format!("{}    is_mirrored: true\n", pad));
    }
    if let Some(desc) = &comp.description {
        if !desc.is_empty() {
            out.push_str(&format!("{}    description: {}\n", pad, quote_string(desc)));
        }
    }

    for child in &comp.children {
        match child {
            ComponentChild::Pin(pin) => dump_pin(out, pin, indent + 4),
            ComponentChild::Parameter(param) => dump_parameter(out, param, indent + 4),
            ComponentChild::Graphic(g) => dump_graphic(out, g, indent + 4),
            ComponentChild::FootprintMap(fm) => dump_footprint_map(out, fm, indent + 4),
        }
    }

    out.push_str(&format!("{}}}\n\n", pad));
}

fn dump_schdoc_sheet_symbol(out: &mut String, ss: &altium_format::api::SheetSymbol, indent: usize) {
    let pad = " ".repeat(indent);
    out.push_str(&format!("{}sheet_symbol {} {{\n", pad, quote_string(&ss.sheet_name)));
    out.push_str(&format!("{}    file_name: {}\n", pad, quote_string(&ss.file_name)));
    out.push_str(&format!("{}    at: {}\n", pad, ss.location));
    out.push_str(&format!("{}    x_size: {}\n", pad, ss.x_size));
    out.push_str(&format!("{}    y_size: {}\n", pad, ss.y_size));

    for child in &ss.children {
        match child {
            SheetSymbolChild::Entry(e) => {
                out.push_str(&format!(
                    "{}    entry {} {{ side: {:?}, io_type: {:?} }}\n",
                    pad, quote_entity_name(&e.name), e.side, e.io_type
                ));
            }
            SheetSymbolChild::Parameter(p) => dump_parameter(out, p, indent + 4),
        }
    }

    out.push_str(&format!("{}}}\n\n", pad));
}

// ── Project enum formatters ──────────────────────────────────────────────────

use crate::eval::{SpecError, SpecErrorCode};

fn spec_err(msg: String) -> SpecError {
    SpecError::no_span(SpecErrorCode::AltiumFormat, msg)
}

fn flatten_mode_to_spec(v: FlattenMode) -> Result<&'static str, SpecError> {
    match v {
        FlattenMode::Smart => Ok("smart"),
        FlattenMode::Flat => Ok("flat"),
        FlattenMode::HierarchicalGlobalPorts => Ok("hierarchical_global_ports"),
        FlattenMode::Global => Ok("global"),
        FlattenMode::HierarchicalStrict => Ok("hierarchical_strict"),
        _ => Err(spec_err(format!("unknown FlattenMode variant: {:?}", v))),
    }
}

fn channel_room_naming_to_spec(v: ChannelRoomNamingStyle) -> Result<&'static str, SpecError> {
    match v {
        ChannelRoomNamingStyle::FlatNumericWithNames => Ok("flat_numeric_with_names"),
        ChannelRoomNamingStyle::FlatNumeric => Ok("flat_numeric"),
        ChannelRoomNamingStyle::FullyQualified => Ok("fully_qualified"),
        ChannelRoomNamingStyle::FullyQualifiedShort => Ok("fully_qualified_short"),
        ChannelRoomNamingStyle::MixedNamePath => Ok("mixed_name_path"),
        _ => Err(spec_err(format!("unknown ChannelRoomNamingStyle variant: {:?}", v))),
    }
}

fn cross_ref_sheet_to_spec(v: CrossRefSheetStyle) -> Result<&'static str, SpecError> {
    match v {
        CrossRefSheetStyle::None => Ok("none"),
        CrossRefSheetStyle::Name => Ok("name"),
        CrossRefSheetStyle::Number => Ok("number"),
        _ => Err(spec_err(format!("unknown CrossRefSheetStyle variant: {:?}", v))),
    }
}

fn cross_ref_location_to_spec(v: CrossRefLocationStyle) -> Result<&'static str, SpecError> {
    match v {
        CrossRefLocationStyle::None => Ok("none"),
        CrossRefLocationStyle::Zone => Ok("zone"),
        CrossRefLocationStyle::XY => Ok("xy"),
        _ => Err(spec_err(format!("unknown CrossRefLocationStyle variant: {:?}", v))),
    }
}

fn cross_ref_ports_to_spec(v: CrossRefPorts) -> Result<&'static str, SpecError> {
    match v {
        CrossRefPorts::Disabled => Ok("disabled"),
        CrossRefPorts::SheetEntry => Ok("sheet_entry"),
        CrossRefPorts::Ports => Ok("ports"),
        CrossRefPorts::SheetEntryAndPorts => Ok("sheet_entry_and_ports"),
        _ => Err(spec_err(format!("unknown CrossRefPorts variant: {:?}", v))),
    }
}

fn error_level_to_spec(v: ErrorLevel) -> Result<&'static str, SpecError> {
    match v {
        ErrorLevel::NoReport => Ok("no_report"),
        ErrorLevel::Warning => Ok("warning"),
        ErrorLevel::Error => Ok("error"),
        ErrorLevel::Fatal => Ok("fatal"),
        _ => Err(spec_err(format!("unknown ErrorLevel variant: {:?}", v))),
    }
}

fn connection_code_to_spec(idx: usize) -> Result<&'static str, SpecError> {
    match idx {
        0 => Ok("pin_input"),
        1 => Ok("pin_bidirectional"),
        2 => Ok("pin_output"),
        3 => Ok("pin_open_collector"),
        4 => Ok("pin_passive"),
        5 => Ok("pin_hi_z"),
        6 => Ok("pin_open_emitter"),
        7 => Ok("pin_power"),
        8 => Ok("sheet_entry_input"),
        9 => Ok("sheet_entry_bidirectional"),
        10 => Ok("sheet_entry_output"),
        11 => Ok("port_unspecified"),
        12 => Ok("pin_unspecified"),
        13 => Ok("sheet_entry_unspecified"),
        14 => Ok("port_input"),
        15 => Ok("port_output"),
        16 => Ok("unconnected"),
        _ => Err(spec_err(format!("unknown ERC connection code index: {}", idx))),
    }
}

fn variation_kind_to_spec(v: altium_format_types::project::VariationKind) -> Result<&'static str, SpecError> {
    match v {
        altium_format_types::project::VariationKind::None => Ok("none"),
        altium_format_types::project::VariationKind::NotFitted => Ok("not_fitted"),
        altium_format_types::project::VariationKind::Alternate => Ok("alternate"),
        _ => Err(spec_err(format!("unknown VariationKind variant: {:?}", v))),
    }
}

// ── Footprint (PcbLib — still uses DumpView) ─────────────────────────────────

fn dump_footprint(out: &mut String, fp: &altium_format::PcbLibFootprintDumpView) {
    out.push_str(&format!("footprint {} {{\n", quote_entity_name(&fp.display_name)));

    if !fp.description.is_empty() {
        out.push_str(&format!("    description: {}\n", quote_string(&fp.description)));
    }

    for pad in &fp.pads {
        dump_pcb_pad(out, pad, 4);
    }

    for graphic in &fp.graphics {
        dump_pcb_graphic(out, graphic, 4);
    }

    out.push_str("}\n");
}

fn dump_pcb_pad(out: &mut String, pad: &altium_format::PcbLibPadDumpView, indent: usize) {
    let p = " ".repeat(indent);
    let x = format_coord_mils(pad.location_x_mils);
    let y = format_coord_mils(pad.location_y_mils);
    let mut parts = vec![format!("at: ({}, {})", x, y)];

    if !pad.shape.is_empty() && pad.shape != "round" {
        parts.push(format!("shape: {}", pad.shape));
    }
    if pad.size_x_mils != 0.0 {
        parts.push(format!("x_size: {}", format_coord_mils(pad.size_x_mils)));
    }
    if pad.size_y_mils != 0.0 {
        parts.push(format!("y_size: {}", format_coord_mils(pad.size_y_mils)));
    }
    if pad.hole_size_mils != 0.0 {
        parts.push(format!("hole_size: {}", format_coord_mils(pad.hole_size_mils)));
    }
    if pad.rotation != 0.0 {
        parts.push(format!("rotation: {}", format_float(pad.rotation)));
    }
    if !pad.is_plated {
        parts.push("is_plated: false".to_owned());
    }
    if pad.layer != "MultiLayer" {
        parts.push(format!("layer: {}", pad.layer));
    }

    out.push_str(&format!(
        "{}pad {} {{ {} }}\n",
        p,
        quote_entity_name(&pad.pad_name),
        parts.join(", ")
    ));
}

fn dump_pcb_graphic(out: &mut String, g: &altium_format::PcbLibGraphicDumpView, indent: usize) {
    let p = " ".repeat(indent);
    match g.graphic_type.as_str() {
        "track" => {
            let fx = format_coord_mils(g.from_x_mils.unwrap_or(0.0));
            let fy = format_coord_mils(g.from_y_mils.unwrap_or(0.0));
            let tx = format_coord_mils(g.to_x_mils.unwrap_or(0.0));
            let ty = format_coord_mils(g.to_y_mils.unwrap_or(0.0));
            let mut props = vec![
                format!("layer: {}", g.layer),
                format!("from: ({}, {})", fx, fy),
                format!("to: ({}, {})", tx, ty),
            ];
            if let Some(w) = g.width_mils {
                if w != 0.0 {
                    props.push(format!("width: {}", format_coord_mils(w)));
                }
            }
            out.push_str(&format!("{}track {{ {} }}\n", p, props.join(", ")));
        }
        "arc" => {
            let cx = format_coord_mils(g.center_x_mils.unwrap_or(0.0));
            let cy = format_coord_mils(g.center_y_mils.unwrap_or(0.0));
            let r = format_coord_mils(g.radius_mils.unwrap_or(0.0));
            let mut props = vec![
                format!("layer: {}", g.layer),
                format!("center: ({}, {})", cx, cy),
                format!("radius: {}", r),
            ];
            if let Some(sa) = g.start_angle {
                props.push(format!("start_angle: {}", format_float(sa)));
            }
            if let Some(ea) = g.end_angle {
                props.push(format!("end_angle: {}", format_float(ea)));
            }
            if let Some(w) = g.width_mils {
                if w != 0.0 {
                    props.push(format!("width: {}", format_coord_mils(w)));
                }
            }
            out.push_str(&format!("{}arc {{ {} }}\n", p, props.join(", ")));
        }
        "fill" => {
            let x1 = format_coord_mils(g.corner1_x_mils.unwrap_or(0.0));
            let y1 = format_coord_mils(g.corner1_y_mils.unwrap_or(0.0));
            let x2 = format_coord_mils(g.corner2_x_mils.unwrap_or(0.0));
            let y2 = format_coord_mils(g.corner2_y_mils.unwrap_or(0.0));
            let mut props = vec![
                format!("layer: {}", g.layer),
                format!("corner1: ({}, {})", x1, y1),
                format!("corner2: ({}, {})", x2, y2),
            ];
            if let Some(rot) = g.rotation {
                if rot != 0.0 {
                    props.push(format!("rotation: {}", format_float(rot)));
                }
            }
            out.push_str(&format!("{}fill {{ {} }}\n", p, props.join(", ")));
        }
        "text" => {
            let lx = format_coord_mils(g.location_x_mils.unwrap_or(0.0));
            let ly = format_coord_mils(g.location_y_mils.unwrap_or(0.0));
            let text = g.text.as_deref().unwrap_or("");
            let mut props = vec![
                format!("layer: {}", g.layer),
                format!("at: ({}, {})", lx, ly),
                format!("text: {}", quote_string(text)),
            ];
            if let Some(rot) = g.rotation {
                if rot != 0.0 {
                    props.push(format!("rotation: {}", format_float(rot)));
                }
            }
            out.push_str(&format!("{}text {{ {} }}\n", p, props.join(", ")));
        }
        "via" => {
            let lx = format_coord_mils(g.location_x_mils.unwrap_or(0.0));
            let ly = format_coord_mils(g.location_y_mils.unwrap_or(0.0));
            let mut props = vec![format!("at: ({}, {})", lx, ly)];
            if let Some(d) = g.diameter_mils {
                props.push(format!("diameter: {}", format_coord_mils(d)));
            }
            if let Some(h) = g.hole_size_mils {
                props.push(format!("hole_size: {}", format_coord_mils(h)));
            }
            out.push_str(&format!("{}via {{ {} }}\n", p, props.join(", ")));
        }
        "region" => {
            if !g.outline.is_empty() {
                let verts: Vec<String> = g.outline.iter()
                    .map(|(x, y)| format!("({}, {})", format_coord_mils(*x), format_coord_mils(*y)))
                    .collect();
                out.push_str(&format!(
                    "{}region {{ layer: {}, outline: [{}] }}\n",
                    p, g.layer, verts.join(", ")
                ));
            }
        }
        _ => {
            out.push_str(&format!("{}// unknown graphic: {}\n", p, g.graphic_type));
        }
    }
}

// ── Component (SchLib — uses high-level API types) ───────────────────────────

fn dump_component(out: &mut String, comp: &Component) {
    out.push_str(&format!("component {} {{\n", quote_entity_name(&comp.lib_reference)));

    if let Some(desc) = &comp.description {
        if !desc.is_empty() {
            out.push_str(&format!("    description: {}\n", quote_string(desc)));
        }
    }

    // Group pins and graphics by owner_part_id > 0 into part blocks
    let part_ids: Vec<i32> = {
        let mut ids: Vec<i32> = comp.pins.iter()
            .filter(|p| p.owner_part_id > 0)
            .map(|p| p.owner_part_id)
            .chain(
                comp.graphics.iter()
                    .filter(|g| g.owner_part_id() > 0)
                    .map(|g| g.owner_part_id())
            )
            .collect();
        ids.sort_unstable();
        ids.dedup();
        ids
    };

    if part_ids.is_empty() {
        // No multi-part: emit all pins and graphics at top level
        for graphic in &comp.graphics {
            if graphic.owner_part_id() <= 0 {
                dump_graphic(out, graphic, 4);
            }
        }
        for pin in &comp.pins {
            if pin.owner_part_id <= 0 {
                dump_pin(out, pin, 4);
            }
        }
    } else {
        // Emit shared graphics/pins (owner_part_id <= 0) at top level
        for graphic in &comp.graphics {
            if graphic.owner_part_id() <= 0 {
                dump_graphic(out, graphic, 4);
            }
        }
        for pin in &comp.pins {
            if pin.owner_part_id <= 0 {
                dump_pin(out, pin, 4);
            }
        }
        // Emit per-part blocks
        for part_id in &part_ids {
            out.push_str(&format!("    part {} {{\n", part_id));
            for graphic in &comp.graphics {
                if graphic.owner_part_id() == *part_id {
                    dump_graphic(out, graphic, 8);
                }
            }
            for pin in &comp.pins {
                if pin.owner_part_id == *part_id {
                    dump_pin(out, pin, 8);
                }
            }
            out.push_str("    }\n");
        }
    }

    // Parameters (skip Designator/Comment — already handled via Component fields)
    for param in &comp.parameters {
        dump_parameter(out, param, 4);
    }

    // Aliases
    for alias in &comp.aliases {
        out.push_str(&format!("    alias {}\n", quote_entity_name(alias)));
    }

    // Footprint maps
    for fp in &comp.footprints {
        dump_footprint_map(out, fp, 4);
    }

    out.push_str("}\n");
}

// ── Pin ───────────────────────────────────────────────────────────────────────

fn dump_pin(out: &mut String, pin: &Pin, indent: usize) {
    let pad = " ".repeat(indent);
    let mut parts = vec![
        format!("at: {}", pin.location),
        format!("orientation: {}", pin.orientation),
        format!("electrical: {}", pin.electrical),
    ];
    // Default pin length in Altium is 25 mils.
    if pin.length != Coord::from_mils(25).expect("25 mils fits Coord") {
        parts.push(format!("length: {}", pin.length));
    }
    if !pin.name.is_empty() {
        parts.push(format!("name: {}", quote_string(&pin.name)));
    }
    if pin.is_hidden {
        parts.push("is_hidden: true".to_owned());
    }
    if !pin.hidden_net_name.is_empty() {
        parts.push(format!("hidden_net_name: {}", quote_string(&pin.hidden_net_name)));
    }
    out.push_str(&format!(
        "{}pin {} {{ {} }}\n",
        pad,
        quote_entity_name(&pin.designator),
        parts.join(", ")
    ));
}

// ── Graphic ───────────────────────────────────────────────────────────────────

fn dump_graphic(out: &mut String, g: &Graphic, indent: usize) {
    let pad = " ".repeat(indent);
    match g {
        Graphic::Line(l) => {
            out.push_str(&format!(
                "{}line {{ from: {}, to: {} }}\n",
                pad, l.location, l.corner
            ));
        }
        Graphic::Rectangle(r) => {
            let mut props = vec![
                format!("location: {}", r.location),
                format!("corner: {}", r.corner),
            ];
            if r.is_solid {
                props.push("is_solid: true".to_owned());
            }
            out.push_str(&format!(
                "{}rectangle {{ {} }}\n",
                pad,
                props.join(", ")
            ));
        }
        Graphic::RoundRectangle(r) => {
            let mut props = vec![
                format!("location: {}", r.location),
                format!("corner: {}", r.corner),
                format!("corner_x_radius: {}", r.corner_x_radius),
                format!("corner_y_radius: {}", r.corner_y_radius),
            ];
            if r.is_solid {
                props.push("is_solid: true".to_owned());
            }
            out.push_str(&format!(
                "{}round_rectangle {{ {} }}\n",
                pad,
                props.join(", ")
            ));
        }
        Graphic::Arc(a) => {
            let mut props = vec![
                format!("center: {}", a.location),
                format!("radius: {}", a.radius),
            ];
            props.push(format!("start_angle: {}", a.start_angle));
            if let Some(ea) = a.end_angle {
                props.push(format!("end_angle: {}", ea));
            }
            out.push_str(&format!("{}arc {{ {} }}\n", pad, props.join(", ")));
        }
        Graphic::EllipticalArc(a) => {
            let mut props = vec![
                format!("center: {}", a.location),
                format!("radius: {}", a.radius),
                format!("secondary_radius: {}", a.secondary_radius),
            ];
            props.push(format!("start_angle: {}", a.start_angle));
            if let Some(ea) = a.end_angle {
                props.push(format!("end_angle: {}", ea));
            }
            out.push_str(&format!("{}elliptical_arc {{ {} }}\n", pad, props.join(", ")));
        }
        Graphic::Ellipse(e) => {
            let mut props = vec![
                format!("center: {}", e.location),
                format!("radius: {}", e.radius),
                format!("secondary_radius: {}", e.secondary_radius),
            ];
            if e.is_solid {
                props.push("is_solid: true".to_owned());
            }
            out.push_str(&format!("{}ellipse {{ {} }}\n", pad, props.join(", ")));
        }
        Graphic::Pie(p_) => {
            let mut props = vec![
                format!("center: {}", p_.location),
                format!("radius: {}", p_.radius),
            ];
            props.push(format!("start_angle: {}", p_.start_angle));
            if let Some(ea) = p_.end_angle {
                props.push(format!("end_angle: {}", ea));
            }
            out.push_str(&format!("{}pie {{ {} }}\n", pad, props.join(", ")));
        }
        Graphic::Polyline(pl) => {
            let verts: Vec<String> = pl.vertices.iter()
                .map(|v| format!("{}", v))
                .collect();
            out.push_str(&format!(
                "{}polyline {{ vertices: [{}] }}\n",
                pad,
                verts.join(", ")
            ));
        }
        Graphic::Polygon(pg) => {
            let verts: Vec<String> = pg.vertices.iter()
                .map(|v| format!("{}", v))
                .collect();
            let mut props = vec![format!("vertices: [{}]", verts.join(", "))];
            if pg.is_solid {
                props.push("is_solid: true".to_owned());
            }
            out.push_str(&format!("{}polygon {{ {} }}\n", pad, props.join(", ")));
        }
        Graphic::Bezier(b) => {
            let verts: Vec<String> = b.vertices.iter()
                .map(|v| format!("{}", v))
                .collect();
            out.push_str(&format!(
                "{}bezier {{ vertices: [{}] }}\n",
                pad,
                verts.join(", ")
            ));
        }
        Graphic::Label(l) => {
            out.push_str(&format!(
                "{}label {{ at: {}, text: {} }}\n",
                pad, l.location, quote_string(&l.text)
            ));
        }
        Graphic::TextFrame(tf) => {
            out.push_str(&format!(
                "{}text_frame {{ location: {}, corner: {}, text: {} }}\n",
                pad, tf.location, tf.corner, quote_string(&tf.text)
            ));
        }
        Graphic::Image(img) => {
            out.push_str(&format!(
                "{}image {{ location: {}, corner: {}, file: {} }}\n",
                pad, img.location, img.corner, quote_string(&img.file_name)
            ));
        }
    }
}

// ── Parameter ─────────────────────────────────────────────────────────────────

fn dump_parameter(out: &mut String, param: &Parameter, indent: usize) {
    let pad = " ".repeat(indent);
    out.push_str(&format!("{}parameter {} {{\n", pad, quote_entity_name(&param.name)));
    out.push_str(&format!("{}    value: {}\n", pad, quote_string(&param.text)));
    if param.is_hidden {
        out.push_str(&format!("{}    is_hidden: true\n", pad));
    }
    out.push_str(&format!("{}}}\n", pad));
}

// ── Footprint map ─────────────────────────────────────────────────────────────

fn dump_footprint_map(out: &mut String, fp: &FootprintMap, indent: usize) {
    let pad = " ".repeat(indent);
    out.push_str(&format!(
        "{}footprint {} {{\n",
        pad,
        quote_entity_name(&fp.model_name)
    ));
    if !fp.description.is_empty() {
        out.push_str(&format!("{}    description: {}\n", pad, quote_string(&fp.description)));
    }
    // Group API entries by pin to reproduce `map PIN -> PAD1, PAD2` syntax.
    // The API uses Vec<PinPadMap> where each entry is 1:1 (pin, pad).
    let mut pin_to_pads: IndexMap<&str, Vec<&str>> = IndexMap::new();
    for m in &fp.pin_pad_maps {
        pin_to_pads.entry(&m.pin).or_default().push(&m.pad);
    }
    for (pin, pads) in &pin_to_pads {
        if pads.is_empty() {
            continue;
        }
        let pad_strs: Vec<String> = pads.iter()
            .map(|p| quote_entity_name(p))
            .collect();
        out.push_str(&format!(
            "{}    map {} -> {}\n",
            pad,
            quote_entity_name(pin),
            pad_strs.join(", ")
        ));
    }
    out.push_str(&format!("{}}}\n", pad));
}

// ── Formatting helpers ────────────────────────────────────────────────────────

/// Format a coordinate in mils as the most natural unit.
/// Prefers mm if the value is "clean" (exact to 3 decimal places in mm).
/// Falls back to mils otherwise.
/// Used by PcbLib dump (which still works with raw f64 mils from DumpView).
pub fn format_coord_mils(mils: f64) -> String {
    let mm = mils * 0.0254;
    if (mm * 1000.0).round() == mm * 1000.0 && mm.abs() >= 0.001 {
        format!("{}mm", format_float(mm))
    } else {
        format!("{}mil", format_float(mils))
    }
}

/// Format a float, removing trailing zeros after the decimal point.
/// Used by PcbLib dump for angles and coordinates.
pub fn format_float(v: f64) -> String {
    let s = format!("{:.4}", v);
    let s = s.trim_end_matches('0');
    let s = s.trim_end_matches('.');
    s.to_string()
}

/// Quote an entity name: bare if it's a valid ident or integer, quoted otherwise.
pub fn quote_entity_name(name: &str) -> String {
    if name.parse::<i64>().is_ok() {
        return name.to_string();
    }
    if is_valid_ident(name) {
        return name.to_string();
    }
    format!("\"{}\"", name.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Always quotes the string value with backslash escaping.
pub fn quote_string(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Returns true if the string is a valid bare identifier in the spec language.
fn is_valid_ident(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let mut chars = s.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use altium_format_types::coord::Coord;

    #[test]
    fn test_coord_display_mm_clean() {
        // 100 mils = 2.54 mm (clean value)
        let c = Coord::from_mils(100).expect("test coord");
        assert_eq!(format!("{}", c), "2.54mm");
    }

    #[test]
    fn test_coord_display_mils_fallback() {
        // 1 mil = 0.0254 mm (not clean to 3 decimal places)
        let c = Coord::from_mils(1).expect("test coord");
        assert_eq!(format!("{}", c), "1mil");
    }

    #[test]
    fn test_coord_display_zero() {
        let c = Coord::ZERO;
        assert_eq!(format!("{}", c), "0mil");
    }

    #[test]
    fn test_format_coord_mils_mm_clean() {
        // 100 mils = 2.54 mm (clean value)
        let s = format_coord_mils(100.0);
        assert_eq!(s, "2.54mm");
    }

    #[test]
    fn test_format_coord_mils_mils_fallback() {
        let s = format_coord_mils(1.0);
        assert_eq!(s, "1mil");
    }

    #[test]
    fn test_format_coord_mils_zero() {
        let s = format_coord_mils(0.0);
        assert_eq!(s, "0mil");
    }

    #[test]
    fn test_format_float_trailing_zeros() {
        assert_eq!(format_float(1.5), "1.5");
        assert_eq!(format_float(2.0), "2");
        assert_eq!(format_float(1.2500), "1.25");
    }

    #[test]
    fn test_quote_entity_name_valid_ident() {
        assert_eq!(quote_entity_name("foo"), "foo");
        assert_eq!(quote_entity_name("_bar"), "_bar");
        assert_eq!(quote_entity_name("A1"), "A1");
    }

    #[test]
    fn test_quote_entity_name_integer() {
        assert_eq!(quote_entity_name("1"), "1");
        assert_eq!(quote_entity_name("42"), "42");
        assert_eq!(quote_entity_name("-1"), "-1");
    }

    #[test]
    fn test_quote_entity_name_needs_quotes() {
        assert_eq!(quote_entity_name("foo bar"), "\"foo bar\"");
        assert_eq!(quote_entity_name("a-b"), "\"a-b\"");
        assert_eq!(quote_entity_name(""), "\"\"");
        assert_eq!(quote_entity_name("has\"quote"), "\"has\\\"quote\"");
    }

    #[test]
    fn test_quote_string_always_quoted() {
        assert_eq!(quote_string("hello"), "\"hello\"");
        assert_eq!(quote_string("say \"hi\""), "\"say \\\"hi\\\"\"");
        assert_eq!(quote_string("back\\slash"), "\"back\\\\slash\"");
    }
}
