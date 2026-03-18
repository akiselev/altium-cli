//! Reverse generation: produce `.schlib-spec` or `.pcblib-spec` source from
//! existing Altium library documents.
//!
//! Generated output uses absolute placement only (`at: (x, y)`, explicit
//! `orientation:`). No anchors, rows, grids, or template bindings are emitted.

use altium_format::{AltiumProject, IntLib, PcbDoc, PcbLib, SchDoc, SchLib};
use altium_format::api::{
    Component, Pin, Parameter, FootprintMap, Graphic,
    SheetObject, ComponentChild, SheetSymbolChild,
};
use altium_format::api::{
    PcbDocBoard, LayerStack, StackLayer, RuleParams, BoardGeometry, ContourSegment,
    Footprint, Pad as PcbLibPad, PcbGraphic, PcbContour, PadStack,
};
use altium_format_types::coord::Coord;
use altium_format_types::common::Unit;
use altium_format_types::pcb::{
    ClassMemberKind, DimensionKind, PadShape, PadStackMode, PlaneConnectionStyle, RegionKind,
    RuleKind,
};
use altium_format_types::{DielectricType, LayerStackStyle};
use altium_format_types::project::{
    ChannelRoomNamingStyle, CrossRefLocationStyle, CrossRefPorts, CrossRefSheetStyle,
    ErrorLevel, FlattenMode,
};
use indexmap::IndexMap;
use std::collections::HashSet;

// ── Public API ────────────────────────────────────────────────────────────────

/// Generate `.pcblib-spec` source from a PcbLib document.
pub fn dump_pcblib(lib: &PcbLib) -> String {
    let mut out = String::new();
    for name in lib.footprint_names() {
        match lib.footprint(name) {
            Ok(fp) => {
                dump_footprint(&mut out, &fp);
                out.push('\n');
            }
            Err(e) => {
                out.push_str(&format!("// ERROR loading footprint {name}: {e}\n\n"));
            }
        }
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

/// Result of dumping an IntLib — produces separate SchLib and PcbLib spec text.
pub struct IntLibDump {
    /// `.schlib-spec` source for all embedded schematic symbols, or `None` if
    /// the IntLib contains no SchLib data.
    pub schlib_spec: Option<String>,
    /// `.pcblib-spec` source for all embedded footprints, or `None` if the
    /// IntLib contains no PcbLib data.
    pub pcblib_spec: Option<String>,
}

/// Dump an IntLib's embedded libraries as `.schlib-spec` and `.pcblib-spec`.
pub fn dump_intlib(lib: &IntLib) -> Result<IntLibDump, altium_format::AltiumFormatError> {
    let schlib_spec = if lib.schlibs().is_empty() {
        None
    } else {
        let mut out = String::new();
        for schlib in lib.schlibs() {
            for comp in &schlib.components()? {
                dump_component(&mut out, comp);
                out.push('\n');
            }
        }
        Some(out)
    };

    let pcblib_spec = if lib.pcblibs().is_empty() {
        None
    } else {
        let mut out = String::new();
        for pcblib in lib.pcblibs() {
            for name in pcblib.footprint_names() {
                let fp = pcblib.footprint(name)?;
                dump_footprint(&mut out, &fp);
                out.push('\n');
            }
        }
        Some(out)
    };

    Ok(IntLibDump {
        schlib_spec,
        pcblib_spec,
    })
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

/// Generate a human-readable dump from a PcbDoc document.
///
/// This is informational only (no roundtrip compiler/reconciler).
pub fn dump_pcbdoc(doc: &PcbDoc) -> Result<String, crate::eval::SpecError> {
    let board = doc.board()
        .map_err(|e| crate::eval::SpecError::no_span(
            crate::eval::SpecErrorCode::AltiumFormat,
            e.to_string(),
        ))?;
    let mut out = String::new();
    dump_pcbdoc_board(&mut out, &board);
    Ok(out)
}

fn dump_pcbdoc_board(out: &mut String, board: &PcbDocBoard) {
    // Board settings
    out.push_str(&format!("board {} {{\n", quote_entity_name(&board.settings.document_name)));
    out.push_str(&format!("    signal_layer_count: {}\n", board.settings.signal_layer_count));
    out.push_str(&format!("    snap_grid_size: {}\n", board.settings.snap_grid_size));
    out.push_str(&format!("    visible_grid_size: {}\n", board.settings.visible_grid_size));
    out.push_str(&format!("    display_unit: \"{}\"\n", unit_to_spec_string(board.settings.display_unit)));
    out.push_str("}\n\n");

    // Layer stack
    dump_layer_stack(out, &board.settings.layer_stack);

    // Board geometry
    dump_board_geometry(out, &board.settings.geometry);

    // Nets
    for net in &board.nets {
        out.push_str(&format!(
            "net {} {{ color: #{:02X}{:02X}{:02X}, visible: {} }}\n",
            quote_entity_name(&net.name),
            net.color.r(), net.color.g(), net.color.b(),
            net.visible
        ));
    }
    if !board.nets.is_empty() { out.push('\n'); }

    // Components
    for comp in &board.components {
        out.push_str(&format!(
            "component {} {{ pattern: {}, at: {}, layer: {}, rotation: {} }}\n",
            quote_entity_name(&comp.designator),
            quote_string(&comp.pattern),
            comp.location,
            comp.layer,
            comp.rotation
        ));
    }
    if !board.components.is_empty() { out.push('\n'); }

    // Primitives
    let has_primitives = !board.tracks.is_empty() || !board.arcs.is_empty()
        || !board.vias.is_empty() || !board.pads.is_empty()
        || !board.fills.is_empty() || !board.texts.is_empty()
        || !board.regions.is_empty() || !board.component_bodies.is_empty();

    for track in &board.tracks {
        let mut props = vec![
            format!("layer: {}", track.layer),
        ];
        if let Some(net) = &track.net { props.push(format!("net: {}", quote_entity_name(net))); }
        props.push(format!("from: {}", track.start));
        props.push(format!("to: {}", track.end));
        props.push(format!("width: {}", track.width));
        out.push_str(&format!("track {{ {} }}\n", props.join(", ")));
    }

    for arc in &board.arcs {
        let mut props = vec![
            format!("layer: {}", arc.layer),
        ];
        if let Some(net) = &arc.net { props.push(format!("net: {}", quote_entity_name(net))); }
        props.push(format!("center: {}", arc.center));
        props.push(format!("radius: {}", arc.radius));
        props.push(format!("start_angle: {}", arc.start_angle));
        props.push(format!("end_angle: {}", arc.end_angle));
        props.push(format!("width: {}", arc.width));
        out.push_str(&format!("arc {{ {} }}\n", props.join(", ")));
    }

    for via in &board.vias {
        let mut props = Vec::new();
        if let Some(net) = &via.net { props.push(format!("net: {}", quote_entity_name(net))); }
        props.push(format!("at: {}", via.location));
        props.push(format!("diameter: {}", via.diameter));
        props.push(format!("hole_size: {}", via.hole_size));
        props.push(format!("from_layer: {}", via.from_layer));
        props.push(format!("to_layer: {}", via.to_layer));
        out.push_str(&format!("via {{ {} }}\n", props.join(", ")));
    }

    for pad in &board.pads {
        let mut props = Vec::new();
        if let Some(net) = &pad.net { props.push(format!("net: {}", quote_entity_name(net))); }
        if let Some(comp) = &pad.component { props.push(format!("component: {}", quote_entity_name(comp))); }
        props.push(format!("at: {}", pad.location));
        props.push(format!("layer: {}", pad.layer));
        props.push(format!("shape: \"{}\"", pad_shape_to_spec_string(pad.shape)));
        props.push(format!("x_size: {}", pad.x_size));
        props.push(format!("y_size: {}", pad.y_size));
        props.push(format!("pad_mode: \"{:?}\"", pad.pad_mode));
        // Emit stack info for non-simple pads
        if pad.pad_mode != PadStackMode::Simple {
            props.push(format!(
                "stack_top: \"{}\" {}x{}, stack_mid: \"{}\" {}x{}, stack_bot: \"{}\" {}x{}",
                pad_shape_to_spec_string(pad.stack.top.shape), pad.stack.top.x_size, pad.stack.top.y_size,
                pad_shape_to_spec_string(pad.stack.mid.shape), pad.stack.mid.x_size, pad.stack.mid.y_size,
                pad_shape_to_spec_string(pad.stack.bot.shape), pad.stack.bot.x_size, pad.stack.bot.y_size,
            ));
        }
        if pad.stack.hole_shape != PadShape::Round {
            props.push(format!("hole_shape: \"{}\", slot_size: {}", pad_shape_to_spec_string(pad.stack.hole_shape), pad.stack.slot_size));
        }
        out.push_str(&format!("pad {} {{ {} }}\n", quote_entity_name(&pad.pad_name), props.join(", ")));
    }

    for fill in &board.fills {
        let mut props = vec![format!("layer: {}", fill.layer)];
        if let Some(net) = &fill.net { props.push(format!("net: {}", quote_entity_name(net))); }
        props.push(format!("corner1: {}", fill.corner1));
        props.push(format!("corner2: {}", fill.corner2));
        props.push(format!("rotation: {}", fill.rotation));
        out.push_str(&format!("fill {{ {} }}\n", props.join(", ")));
    }

    for text in &board.texts {
        let mut props = vec![format!("layer: {}", text.layer)];
        props.push(format!("at: {}", text.location));
        props.push(format!("text: {}", quote_string(&text.text)));
        props.push(format!("height: {}", text.height));
        out.push_str(&format!("text {{ {} }}\n", props.join(", ")));
    }

    for region in &board.regions {
        let mut props = vec![format!("layer: {}", region.layer)];
        if let Some(net) = &region.net { props.push(format!("net: {}", quote_entity_name(net))); }
        props.push(format!("kind: \"{}\"", region_kind_to_spec_string(region.kind)));
        out.push_str(&format!("region {{ {} }}\n", props.join(", ")));
    }

    for body in &board.component_bodies {
        let mut props = vec![format!("layer: {}", body.layer)];
        if let Some(comp) = &body.component { props.push(format!("component: {}", quote_entity_name(comp))); }
        if !body.model_name.is_empty() { props.push(format!("model: {}", quote_string(&body.model_name))); }
        out.push_str(&format!("component_body {{ {} }}\n", props.join(", ")));
    }

    if has_primitives { out.push('\n'); }

    // Polygons
    for poly in &board.polygons {
        let mut props = Vec::new();
        if let Some(net) = &poly.net { props.push(format!("net: {}", quote_entity_name(net))); }
        props.push(format!("layer: {}", poly.layer));
        props.push(format!("connect_style: \"{}\"", plane_connection_to_spec_string(poly.connect_style)));
        props.push(format!("pour_order: {}", poly.pour_order));
        out.push_str(&format!("polygon {} {{ {} }}\n", quote_entity_name(&poly.name), props.join(", ")));
    }
    if !board.polygons.is_empty() { out.push('\n'); }

    // Design rules
    for rule in &board.rules {
        let mut props = vec![
            format!("kind: \"{}\"", rule_kind_to_spec_string(rule.kind)),
            format!("enabled: {}", rule.enabled),
            format!("priority: {}", rule.priority),
        ];
        dump_rule_params(&rule.params, &mut props);
        out.push_str(&format!(
            "rule {} {{ {} }}\n",
            quote_entity_name(&rule.name),
            props.join(", "),
        ));
    }
    if !board.rules.is_empty() { out.push('\n'); }

    // Net/component classes
    for class in &board.classes {
        out.push_str(&format!(
            "class {} {{ kind: \"{}\" }}\n",
            quote_entity_name(&class.name),
            class_member_kind_to_spec_string(class.kind)
        ));
    }
    if !board.classes.is_empty() { out.push('\n'); }

    // Dimensions
    for dim in &board.dimensions {
        out.push_str(&format!(
            "dimension {{ kind: \"{}\", layer: {} }}\n",
            dimension_kind_to_spec_string(dim.kind),
            dim.layer
        ));
    }
    if !board.dimensions.is_empty() { out.push('\n'); }

    // Differential pairs
    for dp in &board.differential_pairs {
        out.push_str(&format!(
            "differential_pair {} {{ positive_net: {}, negative_net: {} }}\n",
            quote_entity_name(&dp.name),
            quote_entity_name(&dp.positive_net),
            quote_entity_name(&dp.negative_net)
        ));
    }
}

// ── Layer stack dump ─────────────────────────────────────────────────────────

fn dump_layer_stack(out: &mut String, stack: &LayerStack) {
    if stack.layers.is_empty() {
        return;
    }

    out.push_str(&format!(
        "layer_stack {{ style: \"{}\", is_flex: {}, copper_layers: {} }}\n",
        layer_stack_style_to_spec_string(stack.style),
        stack.is_flex,
        stack.copper_layer_count,
    ));
    for layer in &stack.layers {
        dump_stack_layer(out, layer);
    }
    out.push('\n');
}

fn dump_stack_layer(out: &mut String, layer: &StackLayer) {
    out.push_str(&format!("    layer {} {{", quote_entity_name(&layer.name)));
    out.push_str(&format!(" copper_thickness: {},", layer.copper_thickness));
    out.push_str(&format!(
        " dielectric: \"{}\" {} {} er={:.3}",
        dielectric_type_to_spec_string(layer.dielectric_type),
        layer.dielectric_height,
        quote_string(&layer.dielectric_material),
        layer.dielectric_constant,
    ));
    if layer.is_plane {
        out.push_str(", plane: true");
    }
    out.push_str(" }\n");
}

fn layer_stack_style_to_spec_string(s: LayerStackStyle) -> &'static str {
    match s {
        LayerStackStyle::Pairs => "layer_pairs",
        LayerStackStyle::InsidePairs => "inside_pairs",
        LayerStackStyle::Buildup => "buildup",
        LayerStackStyle::Custom => "custom",
        _ => "layer_pairs",
    }
}

fn dielectric_type_to_spec_string(d: DielectricType) -> &'static str {
    match d {
        DielectricType::NoDielectric => "none",
        DielectricType::Core => "core",
        DielectricType::PrePreg => "prepreg",
        DielectricType::SurfaceMaterial => "surface_material",
        DielectricType::Film => "film",
        _ => "none",
    }
}

// ── Board geometry dump ──────────────────────────────────────────────────────

fn dump_board_geometry(out: &mut String, geom: &BoardGeometry) {
    if geom.outline.is_none() && geom.cutouts.is_empty() && geom.keepouts.is_empty() {
        return;
    }

    out.push_str("geometry {\n");
    if let Some(outline) = &geom.outline {
        out.push_str("    outline {\n");
        for seg in &outline.segments {
            match seg {
                ContourSegment::Line { endpoint } => {
                    out.push_str(&format!("        line {endpoint}\n"));
                }
                ContourSegment::Arc {
                    endpoint,
                    center,
                    radius,
                    start_angle,
                    end_angle,
                } => {
                    out.push_str(&format!(
                        "        arc {endpoint} center: {center}, radius: {radius}, angles: {start_angle}..{end_angle}\n"
                    ));
                }
            }
        }
        out.push_str("    }\n");
    }
    for (i, cutout) in geom.cutouts.iter().enumerate() {
        out.push_str(&format!("    cutout #{} {{ {} segments }}\n", i, cutout.segments.len()));
    }
    for (i, keepout) in geom.keepouts.iter().enumerate() {
        out.push_str(&format!(
            "    keepout #{} {{ layer: {}, {} segments }}\n",
            i, keepout.layer, keepout.outline.segments.len()
        ));
    }
    out.push_str("}\n\n");
}

// ── Rule params dump ─────────────────────────────────────────────────────────

fn dump_rule_params(params: &RuleParams, props: &mut Vec<String>) {
    match params {
        RuleParams::Clearance { gap, .. } => {
            props.push(format!("gap: {gap}"));
        }
        RuleParams::Width { min, max, preferred } => {
            props.push(format!("min: {min}, max: {max}, preferred: {preferred}"));
        }
        RuleParams::Length { min, max } => {
            props.push(format!("min: {min}, max: {max}"));
        }
        RuleParams::MatchedLengths { tolerance } => {
            props.push(format!("tolerance: {tolerance}"));
        }
        RuleParams::ParallelSegment { gap, limit, .. } => {
            props.push(format!("gap: {gap}, limit: {limit}"));
        }
        RuleParams::DaisyChainStubLength { max_limit } => {
            props.push(format!("max_limit: {max_limit}"));
        }
        RuleParams::ShortCircuit { allowed } => {
            props.push(format!("allowed: {allowed}"));
        }
        RuleParams::BrokenNets { check_bad_connections } => {
            props.push(format!("check_bad_connections: {check_bad_connections}"));
        }
        RuleParams::ViasUnderSmd { allowed } => {
            props.push(format!("allowed: {allowed}"));
        }
        RuleParams::MaximumViaCount { max_via_count } => {
            props.push(format!("max_via_count: {max_via_count}"));
        }
        RuleParams::MinimumAnnularRing { min } => {
            props.push(format!("min: {min}"));
        }
        RuleParams::HoleToHoleClearance { gap } => {
            props.push(format!("gap: {gap}"));
        }
        RuleParams::BoardOutlineClearance { gap } => {
            props.push(format!("gap: {gap}"));
        }
        RuleParams::MaxMinHoleSize { min, max } => {
            props.push(format!("min: {min}, max: {max}"));
        }
        RuleParams::SolderMaskExpansion { expansion, is_tenting_top, is_tenting_bottom } => {
            props.push(format!("expansion: {expansion}"));
            if *is_tenting_top { props.push("tenting_top: true".into()); }
            if *is_tenting_bottom { props.push("tenting_bottom: true".into()); }
        }
        RuleParams::PasteMaskExpansion { expansion, percent } => {
            props.push(format!("expansion: {expansion}, percent: {percent:.1}"));
        }
        RuleParams::PowerPlaneClearance { clearance } => {
            props.push(format!("clearance: {clearance}"));
        }
        RuleParams::PowerPlaneConnectStyle { connect_style, relief_conductor_width, relief_entries, relief_air_gap } => {
            props.push(format!("connect_style: \"{}\", relief_width: {}, relief_entries: {}, relief_air_gap: {}",
                plane_connection_to_spec_string(*connect_style), relief_conductor_width, relief_entries, relief_air_gap));
        }
        RuleParams::PolygonConnectStyle { connect_style, relief_conductor_width, relief_entries, air_gap_width, .. } => {
            props.push(format!("connect_style: \"{}\", relief_width: {}, relief_entries: {}, air_gap: {}",
                plane_connection_to_spec_string(*connect_style), relief_conductor_width, relief_entries, air_gap_width));
        }
        RuleParams::RoutingTopology { topology } => {
            props.push(format!("topology: \"{topology:?}\""));
        }
        RuleParams::RoutingPriority { priority } => {
            props.push(format!("routing_priority: {priority}"));
        }
        RuleParams::RoutingCornerStyle { corner_style, .. } => {
            props.push(format!("corner_style: \"{corner_style:?}\""));
        }
        RuleParams::RoutingViaStyle { min_width, max_width, preferred_width, min_hole_width, max_hole_width, preferred_hole_width, .. } => {
            props.push(format!("width: {min_width}..{max_width} (pref {preferred_width}), hole: {min_hole_width}..{max_hole_width} (pref {preferred_hole_width})"));
        }
        RuleParams::ComponentClearance { gap, .. } => {
            props.push(format!("gap: {gap}"));
        }
        RuleParams::DiffPairsRouting { min_gap, max_gap, preferred_gap, max_uncoupled_length } => {
            props.push(format!("gap: {min_gap}..{max_gap} (pref {preferred_gap}), max_uncoupled: {max_uncoupled_length}"));
        }
        RuleParams::MaxMinHeight { min_height, max_height, .. } => {
            props.push(format!("min_height: {min_height}, max_height: {max_height}"));
        }
        RuleParams::MinimumSolderMaskSliver { min_width } => {
            props.push(format!("min_width: {min_width}"));
        }
        RuleParams::SilkToSolderMaskClearance { gap } | RuleParams::SilkToSilkClearance { gap } => {
            props.push(format!("gap: {gap}"));
        }
        RuleParams::NetAntennae { tolerance } => {
            props.push(format!("tolerance: {tolerance}"));
        }
        RuleParams::SmdToCorner { distance } | RuleParams::SmdToPlane { distance } => {
            props.push(format!("distance: {distance}"));
        }
        RuleParams::SmdNeckDown { percent } => {
            props.push(format!("percent: {percent:.1}"));
        }
        RuleParams::SmdEntry { side, corner, any_angle } => {
            props.push(format!("side: {side}, corner: {corner}, any_angle: {any_angle}"));
        }
        RuleParams::UnpouredPolygon { allow_unpoured } => {
            props.push(format!("allow_unpoured: {allow_unpoured}"));
        }
        RuleParams::BackDrilling { depth } => {
            props.push(format!("depth: {depth}"));
        }
        RuleParams::CreepageDistance { gap } => {
            props.push(format!("gap: {gap}"));
        }
        RuleParams::AcuteAngle { minimum } => {
            props.push(format!("minimum: {minimum:.1}"));
        }
        RuleParams::LayerPair { enforce } => {
            props.push(format!("enforce: {enforce}"));
        }
        RuleParams::RoutingLayers { .. }
        | RuleParams::ConfinementConstraint { .. }
        | RuleParams::FanoutControl { .. }
        | RuleParams::Other { .. } => {
            // No additional params emitted for these
        }
    }
}

// ── Enum-to-spec-string helpers ───────────────────────────────────────────────
//
// These produce the canonical lowercase spec-language string representation for
// each Altium enum, matching what the compiler's parse_* functions accept.

fn unit_to_spec_string(u: Unit) -> &'static str {
    match u {
        Unit::Metric => "metric",
        Unit::Imperial => "imperial",
        _ => "imperial",
    }
}

fn pad_shape_to_spec_string(s: PadShape) -> &'static str {
    match s {
        PadShape::NoShape => "no_shape",
        PadShape::Round => "round",
        PadShape::Rectangular => "rectangular",
        PadShape::Octagonal => "octagonal",
        PadShape::Arc => "arc",
        PadShape::Terminator => "terminator",
        PadShape::RoundRect => "round_rect",
        PadShape::RotatedRect => "rotated_rect",
        PadShape::RoundedRectangular => "rounded_rectangular",
        PadShape::Custom => "custom",
        _ => "round",
    }
}

fn region_kind_to_spec_string(k: RegionKind) -> &'static str {
    match k {
        RegionKind::Copper => "copper",
        RegionKind::Cutout => "cutout",
        RegionKind::Named => "named",
        RegionKind::BoardCutout => "board_cutout",
        RegionKind::Cavity => "cavity",
        _ => "copper",
    }
}

fn plane_connection_to_spec_string(c: PlaneConnectionStyle) -> &'static str {
    match c {
        PlaneConnectionStyle::NoConnect => "no_connect",
        PlaneConnectionStyle::Relief => "relief",
        PlaneConnectionStyle::Direct => "direct",
        _ => "no_connect",
    }
}

fn rule_kind_to_spec_string(k: RuleKind) -> String {
    // Convert PascalCase Display to snake_case
    let display = format!("{}", k);
    pascal_to_snake(&display)
}

fn class_member_kind_to_spec_string(k: ClassMemberKind) -> String {
    let display = format!("{:?}", k);
    pascal_to_snake(&display)
}

fn dimension_kind_to_spec_string(k: DimensionKind) -> String {
    let display = format!("{:?}", k);
    pascal_to_snake(&display)
}

/// Convert PascalCase to snake_case.
fn pascal_to_snake(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    for (i, ch) in s.chars().enumerate() {
        if ch.is_ascii_uppercase() && i > 0 {
            out.push('_');
        }
        out.push(ch.to_ascii_lowercase());
    }
    out
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
            ComponentChild::Pin(pin) => dump_pin(out, pin, indent + 4, &HashSet::new()),
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

fn dump_footprint(out: &mut String, fp: &Footprint) {
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

/// Format a Coord as mils for spec output.
fn coord_to_spec(c: Coord) -> String {
    format_coord_mils(c.to_mils())
}

fn dump_pcb_pad(out: &mut String, pad: &PcbLibPad, indent: usize) {
    let p = " ".repeat(indent);
    let x = coord_to_spec(pad.location.x);
    let y = coord_to_spec(pad.location.y);
    let mut parts = vec![format!("at: ({}, {})", x, y)];

    if pad.shape != PadShape::Round {
        parts.push(format!("shape: {}", format!("{:?}", pad.shape).to_lowercase()));
    }
    if pad.x_size != Coord::ZERO {
        parts.push(format!("x_size: {}", coord_to_spec(pad.x_size)));
    }
    if pad.y_size != Coord::ZERO {
        parts.push(format!("y_size: {}", coord_to_spec(pad.y_size)));
    }
    if pad.hole_size != Coord::ZERO {
        parts.push(format!("hole_size: {}", coord_to_spec(pad.hole_size)));
    }
    if pad.rotation != 0.0 {
        parts.push(format!("rotation: {}", format_float(pad.rotation)));
    }
    if !pad.is_plated {
        parts.push("is_plated: false".to_owned());
    }
    if let Some(name) = pad.layer.display_name() {
        if name != "Multi Layer" {
            parts.push(format!("layer: {}", quote_entity_name(name)));
        }
    }

    // Emit pad stack info for non-Simple modes
    if pad.pad_mode != PadStackMode::Simple {
        parts.push(format!("pad_mode: {}", format!("{:?}", pad.pad_mode).to_lowercase()));
        dump_pad_stack_inline(&pad.stack, &mut parts);
    }

    out.push_str(&format!(
        "{}pad {} {{ {} }}\n",
        p,
        quote_entity_name(&pad.pad_name),
        parts.join(", ")
    ));
}

/// Append pad stack details as inline properties (for non-Simple pads).
fn dump_pad_stack_inline(stack: &PadStack, parts: &mut Vec<String>) {
    // Only emit mid/bot if they differ from top
    let top = &stack.top;
    let mid = &stack.mid;
    let bot = &stack.bot;

    if mid.shape != top.shape || mid.x_size != top.x_size || mid.y_size != top.y_size {
        parts.push(format!(
            "mid_shape: {}, mid_x_size: {}, mid_y_size: {}",
            format!("{:?}", mid.shape).to_lowercase(),
            coord_to_spec(mid.x_size),
            coord_to_spec(mid.y_size),
        ));
    }
    if bot.shape != top.shape || bot.x_size != top.x_size || bot.y_size != top.y_size {
        parts.push(format!(
            "bot_shape: {}, bot_x_size: {}, bot_y_size: {}",
            format!("{:?}", bot.shape).to_lowercase(),
            coord_to_spec(bot.x_size),
            coord_to_spec(bot.y_size),
        ));
    }
    if stack.hole_shape != PadShape::Round {
        parts.push(format!("hole_shape: {}", format!("{:?}", stack.hole_shape).to_lowercase()));
    }
    if stack.slot_size != Coord::ZERO {
        parts.push(format!("slot_size: {}", coord_to_spec(stack.slot_size)));
    }
}

fn dump_pcb_graphic(out: &mut String, g: &PcbGraphic, indent: usize) {
    use altium_format::api::*;
    let p = " ".repeat(indent);
    match g {
        PcbGraphic::Track(t) => {
            let fx = coord_to_spec(t.start.x);
            let fy = coord_to_spec(t.start.y);
            let tx = coord_to_spec(t.end.x);
            let ty = coord_to_spec(t.end.y);
            let layer = t.layer.display_name().unwrap_or("Unknown");
            let mut props = vec![
                format!("layer: {}", quote_entity_name(layer)),
                format!("from: ({}, {})", fx, fy),
                format!("to: ({}, {})", tx, ty),
            ];
            if t.width != Coord::ZERO {
                props.push(format!("width: {}", coord_to_spec(t.width)));
            }
            out.push_str(&format!("{}track {{ {} }}\n", p, props.join(", ")));
        }
        PcbGraphic::Arc(a) => {
            let cx = coord_to_spec(a.center.x);
            let cy = coord_to_spec(a.center.y);
            let r = coord_to_spec(a.radius);
            let layer = a.layer.display_name().unwrap_or("Unknown");
            let mut props = vec![
                format!("layer: {}", quote_entity_name(layer)),
                format!("center: ({}, {})", cx, cy),
                format!("radius: {}", r),
            ];
            props.push(format!("start_angle: {}", format_float(a.start_angle)));
            props.push(format!("end_angle: {}", format_float(a.end_angle)));
            if a.width != Coord::ZERO {
                props.push(format!("width: {}", coord_to_spec(a.width)));
            }
            out.push_str(&format!("{}arc {{ {} }}\n", p, props.join(", ")));
        }
        PcbGraphic::Fill(f) => {
            let x1 = coord_to_spec(f.corner1.x);
            let y1 = coord_to_spec(f.corner1.y);
            let x2 = coord_to_spec(f.corner2.x);
            let y2 = coord_to_spec(f.corner2.y);
            let layer = f.layer.display_name().unwrap_or("Unknown");
            let mut props = vec![
                format!("layer: {}", quote_entity_name(layer)),
                format!("corner1: ({}, {})", x1, y1),
                format!("corner2: ({}, {})", x2, y2),
            ];
            if f.rotation != 0.0 {
                props.push(format!("rotation: {}", format_float(f.rotation)));
            }
            out.push_str(&format!("{}fill {{ {} }}\n", p, props.join(", ")));
        }
        PcbGraphic::Text(t) => {
            let lx = coord_to_spec(t.location.x);
            let ly = coord_to_spec(t.location.y);
            let layer = t.layer.display_name().unwrap_or("Unknown");
            let mut props = vec![
                format!("layer: {}", quote_entity_name(layer)),
                format!("at: ({}, {})", lx, ly),
                format!("text: {}", quote_string(&t.text)),
            ];
            if t.rotation != 0.0 {
                props.push(format!("rotation: {}", format_float(t.rotation)));
            }
            out.push_str(&format!("{}text {{ {} }}\n", p, props.join(", ")));
        }
        PcbGraphic::Via(v) => {
            let lx = coord_to_spec(v.location.x);
            let ly = coord_to_spec(v.location.y);
            let mut props = vec![format!("at: ({}, {})", lx, ly)];
            if v.diameter != Coord::ZERO {
                props.push(format!("diameter: {}", coord_to_spec(v.diameter)));
            }
            if v.hole_size != Coord::ZERO {
                props.push(format!("hole_size: {}", coord_to_spec(v.hole_size)));
            }
            out.push_str(&format!("{}via {{ {} }}\n", p, props.join(", ")));
        }
        PcbGraphic::Region(r) => {
            if !r.outline.segments.is_empty() {
                let layer = r.layer.display_name().unwrap_or("Unknown");
                let verts = dump_contour_segments(&r.outline);
                out.push_str(&format!(
                    "{}region {{ layer: {}, outline: [{}] }}\n",
                    p, quote_entity_name(layer), verts
                ));
            }
        }
        PcbGraphic::ComponentBody(b) => {
            let layer = b.layer.display_name().unwrap_or("Unknown");
            let mut props = vec![
                format!("layer: {}", quote_entity_name(layer)),
            ];
            if b.overall_height != Coord::ZERO {
                props.push(format!("height: {}", coord_to_spec(b.overall_height)));
            }
            if !b.model_name.is_empty() {
                props.push(format!("model: {}", quote_string(&b.model_name)));
            }
            if !b.outline.segments.is_empty() {
                props.push(format!("outline: [{}]", dump_contour_segments(&b.outline)));
            }
            out.push_str(&format!("{}component_body {{ {} }}\n", p, props.join(", ")));
        }
    }
}

/// Format contour segments for spec output, preserving arc geometry.
fn dump_contour_segments(contour: &PcbContour) -> String {
    let segs: Vec<String> = contour.segments.iter().map(|seg| {
        match seg {
            ContourSegment::Line { endpoint } => {
                format!("({}, {})", coord_to_spec(endpoint.x), coord_to_spec(endpoint.y))
            }
            ContourSegment::Arc { endpoint, center, radius, start_angle, end_angle } => {
                format!(
                    "arc({}, {}, center: ({}, {}), radius: {}, {}-{})",
                    coord_to_spec(endpoint.x), coord_to_spec(endpoint.y),
                    coord_to_spec(center.x), coord_to_spec(center.y),
                    coord_to_spec(*radius),
                    format_float(*start_angle), format_float(*end_angle),
                )
            }
        }
    }).collect();
    segs.join(", ")
}

// ── Component (SchLib — uses high-level API types) ───────────────────────────

fn dump_component(out: &mut String, comp: &Component) {
    out.push_str(&format!("component {} {{\n", quote_entity_name(&comp.lib_reference)));

    if let Some(desc) = &comp.description {
        if !desc.is_empty() {
            out.push_str(&format!("    description: {}\n", quote_string(desc)));
        }
    }

    // Pre-scan: collect all unique swap group strings from all pins.
    // Groups with 2+ members get declared as `swap_group` blocks.
    let declared_swap_groups: HashSet<String> = {
        let mut all_ids: Vec<&str> = Vec::new();
        for pin in &comp.pins {
            if !pin.swap_id_pin.is_empty() {
                all_ids.push(&pin.swap_id_pin);
            }
            if !pin.swap_id_part.is_empty() {
                all_ids.push(&pin.swap_id_part);
            }
            if !pin.swap_id_pair.is_empty() {
                all_ids.push(&pin.swap_id_pair);
            }
        }
        // Deduplicate
        let unique: HashSet<String> = all_ids.into_iter().map(|s| s.to_string()).collect();
        // Emit swap_group declarations for groups that appear on 2+ pins
        let mut declared = HashSet::new();
        for sg in &unique {
            // Only declare swap_group bindings for valid identifiers.
            // Groups with special characters can't be referenced as $name,
            // so they use inline string literals instead.
            if !is_valid_ident(sg) {
                continue;
            }
            let count = comp.pins.iter().filter(|p| {
                p.swap_id_pin == *sg || p.swap_id_part == *sg || p.swap_id_pair == *sg
            }).count();
            if count >= 2 {
                out.push_str(&format!("    swap_group {} {{}}\n", quote_entity_name(sg)));
                declared.insert(sg.clone());
            }
        }
        declared
    };

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
                dump_pin(out, pin, 4, &declared_swap_groups);
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
                dump_pin(out, pin, 4, &declared_swap_groups);
            }
        }
        // Emit per-part blocks
        for part_id in &part_ids {
            // Check if all pins in this part share the same swap_id_part.
            // If so, emit it as a part-level property instead of per-pin.
            let part_pins: Vec<&Pin> = comp.pins.iter()
                .filter(|p| p.owner_part_id == *part_id)
                .collect();
            let uniform_part_swap: Option<&str> = {
                let first = part_pins.first().and_then(|p| {
                    if p.swap_id_part.is_empty() { None } else { Some(p.swap_id_part.as_str()) }
                });
                if let Some(val) = first {
                    if part_pins.iter().all(|p| p.swap_id_part == val) {
                        Some(val)
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            out.push_str(&format!("    part {} {{\n", part_id));
            if let Some(sg) = uniform_part_swap {
                out.push_str(&format!("        swap_group: {}\n", swap_group_ref(sg, &declared_swap_groups)));
            }
            for graphic in &comp.graphics {
                if graphic.owner_part_id() == *part_id {
                    dump_graphic(out, graphic, 8);
                }
            }
            for pin in &comp.pins {
                if pin.owner_part_id == *part_id {
                    // If part_swap_group was emitted at part level, suppress it on pins
                    dump_pin_with_part_swap_override(
                        out, pin, 8, &declared_swap_groups,
                        uniform_part_swap.is_some(),
                    );
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

fn dump_pin(out: &mut String, pin: &Pin, indent: usize, declared_groups: &HashSet<String>) {
    dump_pin_with_part_swap_override(out, pin, indent, declared_groups, false);
}

fn dump_pin_with_part_swap_override(
    out: &mut String,
    pin: &Pin,
    indent: usize,
    declared_groups: &HashSet<String>,
    suppress_part_swap: bool,
) {
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
    if !pin.swap_id_pin.is_empty() {
        parts.push(format!("swap_group: {}", swap_group_ref(&pin.swap_id_pin, declared_groups)));
    }
    if !suppress_part_swap && !pin.swap_id_part.is_empty() {
        parts.push(format!("part_swap_group: {}", swap_group_ref(&pin.swap_id_part, declared_groups)));
    }
    if !pin.swap_id_pair.is_empty() {
        parts.push(format!("pair_swap_group: {}", swap_group_ref(&pin.swap_id_pair, declared_groups)));
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
    // description is not part of FootprintMapSpec; emit as a comment for reference
    if !fp.description.is_empty() {
        out.push_str(&format!("{}    // {}\n", pad, fp.description));
    }
    for m in &fp.pin_pad_maps {
        out.push_str(&format!(
            "{}    map {{ pin: {}, pad: {} }}\n",
            pad,
            quote_string(&m.pin),
            quote_string(&m.pad),
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

/// Format a swap group reference for dump output.
///
/// If the group was declared with `swap_group <name> { ... }` and the name is
/// a valid identifier, emit `$name` (a binding reference). Otherwise fall back
/// to a plain string literal.
fn swap_group_ref(name: &str, declared_groups: &HashSet<String>) -> String {
    if declared_groups.contains(name) && is_valid_ident(name) {
        format!("${name}")
    } else {
        quote_string(name)
    }
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
