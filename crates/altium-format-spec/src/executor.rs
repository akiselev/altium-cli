//! Executor: applies spec models directly to Altium documents.
//!
//! Uses the high-level `altium_format::api` types for querying and mutating
//! documents, converting spec model types into API types.

use altium_format::api;
use altium_format::{AltiumProject, PcbDoc, PcbLib, SchDoc, SchLib};

use altium_format_types::color::Color;
use altium_format_types::coord::{Coord, CoordPoint};
use altium_format_types::common::{ComponentKind, RotationBy90};
use altium_format_types::sch::{
    IeeeSymbol, LeftRightSide, LineStyle, PenWidth, ParameterReadOnlyState, ParameterType,
    PinElectricalType, PortArrowStyle, PortIoType, PowerObjectStyle, SheetSymbolType,
    StdLogicState, TextJustification, LineShape, HorizontalAlign,
};

use altium_format_types::common::Unit;
use altium_format_types::pcb::{LayerRef, PadShape, PcbFlags, RegionKind, V6Layer};

use crate::eval::{SpecError, SpecErrorCode};
use crate::eval::Value;
use crate::model::{
    BoardSpec, ComponentSpec, FootprintMapSpec, FootprintSpec, GraphicSpec, GraphicType, LayerSpec,
    PadSpec, ParameterSpec, PcbDocComponentSpec, PcbDocNetSpec, PcbDocPolygonSpec,
    PcbDocPrimitiveSpec, PcbDocSpec, PcbGraphicSpec, PcbGraphicType, PcbLibSpec, PinSpec,
    PrjPcbSpec, SchLibSpec, SchDocComponentSpec, SchDocObjectSpec, SchDocSpec, SheetSpec, SymbolRef,
};

// ── Public API ────────────────────────────────────────────────────────────────

/// Apply a SchLib spec directly to a document.
///
/// For each component in the spec:
/// - If the component already exists (matched by `lib_reference`), merge the
///   spec fields over the existing component (additive-only: `Option::Some`
///   overrides, `None` preserves existing).
/// - If the component doesn't exist, create it from the spec with defaults.
pub fn apply_spec_schlib(
    spec: &SchLibSpec,
    doc: &mut SchLib,
) -> Result<(), SpecError> {
    for comp_spec in &spec.components {
        match doc.component(&comp_spec.lib_reference) {
            Ok(existing) => {
                let merged = merge_spec_into_component(&existing, comp_spec);
                doc.update_component(&merged)
                    .map_err(|e| SpecError::no_span(SpecErrorCode::AltiumFormat, e.to_string()))?;
            }
            Err(_) => {
                let comp = component_from_spec(comp_spec);
                doc.add_component(comp)
                    .map_err(|e| SpecError::no_span(SpecErrorCode::AltiumFormat, e.to_string()))?;
            }
        }
    }
    Ok(())
}

/// Apply a PcbLib spec directly to a document.
///
/// For each footprint in the spec:
/// - If the footprint already exists (matched by `display_name`), merge the
///   spec fields over the existing footprint (additive-only: `Option::Some`
///   overrides, `None` preserves existing).
/// - If the footprint doesn't exist, create it from the spec with defaults.
pub fn apply_spec_pcblib(
    spec: &PcbLibSpec,
    lib: &mut PcbLib,
) -> Result<(), SpecError> {
    for fp_spec in &spec.footprints {
        match lib.footprint(&fp_spec.display_name) {
            Ok(existing) => {
                let merged = merge_spec_into_footprint(&existing, fp_spec);
                lib.update_footprint(&merged)
                    .map_err(|e| SpecError::no_span(SpecErrorCode::AltiumFormat, e.to_string()))?;
            }
            Err(_) => {
                let fp = footprint_from_pcblib_spec(fp_spec);
                lib.add_footprint(fp)
                    .map_err(|e| SpecError::no_span(SpecErrorCode::AltiumFormat, e.to_string()))?;
            }
        }
    }
    Ok(())
}

/// Apply a PrjPcb spec to a project document.
///
/// Merges spec fields into the project's `[Design]` section:
/// `Some` overrides the existing value, `None` preserves it.
/// ERC matrix and ERC level overrides are applied sparsely.
pub fn apply_spec_prjpcb(
    spec: &PrjPcbSpec,
    doc: &mut AltiumProject,
) -> Result<(), SpecError> {
    use altium_format_types::project::ConnectionCode;

    for proj_spec in &spec.projects {
        // Merge scalar [Design] fields.
        let design = doc.design_mut();

        if let Some(v) = proj_spec.hierarchy_mode {
            design.insert("HierarchyMode".into(), (v as i32).to_string());
        }
        if let Some(v) = proj_spec.channel_room_naming_style {
            design.insert("ChannelRoomNamingStyle".into(), (v as i32).to_string());
        }
        if let Some(ref v) = proj_spec.channel_designator_format {
            design.insert("ChannelDesignatorFormatString".into(), v.clone());
        }
        if let Some(ref v) = proj_spec.channel_room_level_separator {
            design.insert("ChannelRoomLevelSeperator".into(), v.clone());
        }
        if let Some(v) = proj_spec.allow_port_net_names {
            design.insert("AllowPortNetNames".into(), bool_to_ini(v));
        }
        if let Some(v) = proj_spec.allow_sheet_entry_net_names {
            design.insert("AllowSheetEntryNetNames".into(), bool_to_ini(v));
        }
        if let Some(v) = proj_spec.netlist_single_pin_nets {
            design.insert("NetlistSinglePinNets".into(), bool_to_ini(v));
        }
        if let Some(v) = proj_spec.append_sheet_number_to_local_nets {
            design.insert("AppendSheetNumberToLocalNets".into(), bool_to_ini(v));
        }
        if let Some(v) = proj_spec.name_nets_hierarchically {
            design.insert("NameNetsHierarchically".into(), bool_to_ini(v));
        }
        if let Some(v) = proj_spec.power_port_names_take_priority {
            design.insert("PowerPortNamesTakePriority".into(), bool_to_ini(v));
        }
        if let Some(v) = proj_spec.pin_swap_by_netlabel {
            design.insert("PinSwapBy_Netlabel".into(), bool_to_ini(v));
        }
        if let Some(v) = proj_spec.pin_swap_by_pin {
            design.insert("PinSwapBy_Pin".into(), bool_to_ini(v));
        }
        if let Some(v) = proj_spec.cross_ref_sheet_style {
            design.insert("CrossRefSheetStyle".into(), (v as i32).to_string());
        }
        if let Some(v) = proj_spec.cross_ref_location_style {
            design.insert("CrossRefLocationStyle".into(), (v as i32).to_string());
        }
        if let Some(v) = proj_spec.cross_ref_ports {
            design.insert("CrossRefPorts".into(), (v as i32).to_string());
        }
        if let Some(v) = proj_spec.cross_ref_cross_sheets {
            design.insert("CrossRefCrossSheets".into(), bool_to_ini(v));
        }
        if let Some(v) = proj_spec.cross_ref_sheet_entries {
            design.insert("CrossRefSheetEntries".into(), bool_to_ini(v));
        }
        if let Some(ref v) = proj_spec.output_path {
            design.insert("OutputPath".into(), v.clone());
        }

        // Apply ERC matrix overrides (sparse: only override specified cells).
        for erc in &proj_spec.erc_matrix_overrides {
            let row_idx = erc.row as i32;
            let col_idx = erc.col as i32;
            let key = format!("L{}", row_idx + 1);

            let erc_matrix = doc.erc_matrix_mut();
            let row_str = erc_matrix.entry(key.clone()).or_insert_with(|| {
                // Build default row with all NoReport
                (0..ConnectionCode::Unconnected as i32 + 1)
                    .map(|_| 'N')
                    .collect::<String>()
            });
            // Replace the character at col_idx
            let mut chars: Vec<char> = row_str.chars().collect();
            if (col_idx as usize) < chars.len() {
                chars[col_idx as usize] = erc.level.to_matrix_char();
            }
            *row_str = chars.into_iter().collect();
        }

        // Apply ERC level overrides.
        for erc_level in &proj_spec.erc_level_overrides {
            let erc_levels = doc.erc_levels_mut();
            erc_levels.insert(erc_level.name.clone(), (erc_level.level as i32).to_string());
        }
    }
    Ok(())
}

// ── PcbDoc ────────────────────────────────────────────────────────────────────

/// Apply a PcbDoc spec directly to a document.
///
/// For named collections (nets, components, polygons, rules, classes, diff pairs):
/// match by name/designator; if found, merge spec fields over existing; if not found, create new.
///
/// For primitives (tracks, arcs, vias, etc.): additive — spec primitives are
/// appended to the board. Matching is deferred to the reconciler.
pub fn apply_spec_pcbdoc(
    spec: &PcbDocSpec,
    doc: &mut PcbDoc,
) -> Result<(), SpecError> {
    for board_spec in &spec.boards {
        let mut board = doc.board()
            .map_err(|e| SpecError::no_span(SpecErrorCode::AltiumFormat, e.to_string()))?;

        // Board settings
        apply_pcbdoc_board_settings(&mut board.settings, board_spec);

        // Named collections
        apply_pcbdoc_nets(&mut board, &board_spec.nets);
        apply_pcbdoc_components(&mut board, &board_spec.components);
        apply_pcbdoc_polygons(&mut board, &board_spec.polygons);

        // Primitives (additive)
        apply_pcbdoc_primitives(&mut board, board_spec)?;

        doc.update_board(&board)
            .map_err(|e| SpecError::no_span(SpecErrorCode::AltiumFormat, e.to_string()))?;
    }
    Ok(())
}

fn apply_pcbdoc_board_settings(
    settings: &mut api::BoardSettings,
    spec: &BoardSpec,
) {
    if let Some(count) = spec.signal_layer_count {
        settings.signal_layer_count = count;
    }
    if let Some(grid) = spec.snap_grid_size {
        settings.snap_grid_size = grid;
    }
    if let Some(grid) = spec.visible_grid_size {
        settings.visible_grid_size = grid;
    }
    if let Some(ref unit_str) = spec.display_unit {
        match unit_str.as_str() {
            "metric" => settings.display_unit = Unit::Metric,
            "imperial" => settings.display_unit = Unit::Imperial,
            _ => {} // unknown unit string, leave unchanged
        }
    }
}

fn apply_pcbdoc_nets(board: &mut api::PcbDocBoard, specs: &[PcbDocNetSpec]) {
    for spec in specs {
        if let Some(existing) = board.nets.iter_mut().find(|n| n.name == spec.name) {
            if let Some(color) = spec.color {
                existing.color = color;
            }
            if let Some(visible) = spec.visible {
                existing.visible = visible;
            }
        } else {
            board.nets.push(api::Net {
                id: spec.name.clone(),
                name: spec.name.clone(),
                color: spec.color.unwrap_or(Color::WHITE),
                visible: spec.visible.unwrap_or(true),
            });
        }
    }
}

fn apply_pcbdoc_components(
    board: &mut api::PcbDocBoard,
    specs: &[PcbDocComponentSpec],
) {
    for spec in specs {
        if let Some(existing) = board.components.iter_mut().find(|c| c.designator == spec.designator) {
            if let Some(ref pattern) = spec.pattern {
                existing.pattern = pattern.clone();
            }
            if let Some(ref comment) = spec.comment {
                existing.comment = comment.clone();
            }
            if let Some(loc) = spec.location {
                existing.location = loc;
            }
            if let Some(rot) = spec.rotation {
                existing.rotation = rot;
            }
            if let Some(ref layer) = spec.layer {
                existing.layer = resolve_layer_spec(layer);
            }
            if let Some(ref src) = spec.source_library {
                existing.source_library = src.clone();
            }
        } else {
            board.components.push(api::PcbDocComponent {
                id: spec.designator.clone(),
                designator: spec.designator.clone(),
                pattern: spec.pattern.clone().unwrap_or_default(),
                comment: spec.comment.clone().unwrap_or_default(),
                location: spec.location.unwrap_or_default(),
                rotation: spec.rotation.unwrap_or(0.0),
                layer: spec.layer.as_ref().map(resolve_layer_spec)
                    .unwrap_or(LayerRef::from_v6(V6Layer::TopLayer)),
                source_library: spec.source_library.clone().unwrap_or_default(),
                source_lib_reference: String::new(),
            });
        }
    }
}

fn apply_pcbdoc_polygons(
    board: &mut api::PcbDocBoard,
    specs: &[PcbDocPolygonSpec],
) {
    for spec in specs {
        if let Some(existing) = board.polygons.iter_mut().find(|p| p.name == spec.name) {
            if let Some(ref net) = spec.net {
                existing.net = Some(net.clone());
            }
            if let Some(ref layer) = spec.layer {
                existing.layer = resolve_layer_spec(layer);
            }
            if let Some(ref cs) = spec.connect_style {
                if let Some(style) = parse_plane_connection(cs) {
                    existing.connect_style = style;
                }
            }
            if let Some(order) = spec.pour_order {
                existing.pour_order = order;
            }
        }
        // Don't create new polygons from spec alone — they need vertices
        // which aren't captured in the simplified spec model.
    }
}

fn apply_pcbdoc_primitives(
    board: &mut api::PcbDocBoard,
    spec: &BoardSpec,
) -> Result<(), SpecError> {
    for prim in &spec.tracks {
        if let Some(track) = primitive_to_track(prim)? {
            board.tracks.push(track);
        }
    }
    for prim in &spec.arcs {
        if let Some(arc) = primitive_to_arc(prim)? {
            board.arcs.push(arc);
        }
    }
    for prim in &spec.vias {
        if let Some(via) = primitive_to_via(prim)? {
            board.vias.push(via);
        }
    }
    for prim in &spec.fills {
        if let Some(fill) = primitive_to_fill(prim)? {
            board.fills.push(fill);
        }
    }
    for prim in &spec.texts {
        if let Some(text) = primitive_to_text(prim)? {
            board.texts.push(text);
        }
    }
    Ok(())
}

// ── Primitive → API type converters ──────────────────────────────────────────

fn prop_coord(props: &indexmap::IndexMap<String, Value>, key: &str) -> Option<Coord> {
    props.get(key).and_then(|v| v.to_dim(None).ok()).map(Coord::new)
}

fn prop_coord_point(props: &indexmap::IndexMap<String, Value>, key: &str) -> Option<CoordPoint> {
    match props.get(key)? {
        Value::CoordPoint(x, y) => Some(CoordPoint::new(Coord::new(*x), Coord::new(*y))),
        _ => None,
    }
}

fn prop_string(props: &indexmap::IndexMap<String, Value>, key: &str) -> Option<String> {
    match props.get(key)? {
        Value::String(s) => Some(s.clone()),
        _ => None,
    }
}

fn prop_float(props: &indexmap::IndexMap<String, Value>, key: &str) -> Option<f64> {
    match props.get(key)? {
        Value::Float(f) => Some(*f),
        Value::Integer(n) => Some(*n as f64),
        _ => None,
    }
}

fn prop_bool(props: &indexmap::IndexMap<String, Value>, key: &str) -> Option<bool> {
    match props.get(key)? {
        Value::Bool(b) => Some(*b),
        _ => None,
    }
}

fn prop_layer(props: &indexmap::IndexMap<String, Value>, key: &str) -> LayerRef {
    prop_string(props, key)
        .and_then(|s| {
            if let Some(lr) = LayerRef::from_string_name(&s) {
                Some(lr)
            } else {
                Some(LayerRef::from_v6(V6Layer::TopLayer))
            }
        })
        .unwrap_or(LayerRef::from_v6(V6Layer::TopLayer))
}

fn primitive_to_track(prim: &PcbDocPrimitiveSpec) -> Result<Option<api::Track>, SpecError> {
    let p = &prim.properties;
    let start = match prop_coord_point(p, "from") {
        Some(pt) => pt,
        None => return Ok(None),
    };
    let end = match prop_coord_point(p, "to") {
        Some(pt) => pt,
        None => return Ok(None),
    };
    Ok(Some(api::Track {
        id: prim.id.clone(),
        layer: prop_layer(p, "layer"),
        net: prop_string(p, "net"),
        component: prop_string(p, "component"),
        start,
        end,
        width: prop_coord(p, "width").unwrap_or(Coord::from_mils_f64(10.0)),
    }))
}

fn primitive_to_arc(prim: &PcbDocPrimitiveSpec) -> Result<Option<api::Arc>, SpecError> {
    let p = &prim.properties;
    let center = match prop_coord_point(p, "center") {
        Some(pt) => pt,
        None => return Ok(None),
    };
    Ok(Some(api::Arc {
        id: prim.id.clone(),
        layer: prop_layer(p, "layer"),
        net: prop_string(p, "net"),
        component: prop_string(p, "component"),
        center,
        radius: prop_coord(p, "radius").unwrap_or(Coord::from_mils_f64(50.0)),
        start_angle: prop_float(p, "start_angle").unwrap_or(0.0),
        end_angle: prop_float(p, "end_angle").unwrap_or(360.0),
        width: prop_coord(p, "width").unwrap_or(Coord::from_mils_f64(10.0)),
    }))
}

fn primitive_to_via(prim: &PcbDocPrimitiveSpec) -> Result<Option<api::Via>, SpecError> {
    let p = &prim.properties;
    let location = match prop_coord_point(p, "at") {
        Some(pt) => pt,
        None => return Ok(None),
    };
    Ok(Some(api::Via {
        id: prim.id.clone(),
        net: prop_string(p, "net"),
        component: prop_string(p, "component"),
        location,
        diameter: prop_coord(p, "diameter").unwrap_or(Coord::from_mils_f64(50.0)),
        hole_size: prop_coord(p, "hole_size").unwrap_or(Coord::from_mils_f64(28.0)),
        from_layer: prop_layer(p, "from_layer"),
        to_layer: prop_string(p, "to_layer")
            .and_then(|s| LayerRef::from_string_name(&s))
            .unwrap_or(LayerRef::from_v6(V6Layer::BottomLayer)),
        solder_mask_expansion: prop_coord(p, "solder_mask_expansion"),
    }))
}

fn primitive_to_fill(prim: &PcbDocPrimitiveSpec) -> Result<Option<api::Fill>, SpecError> {
    let p = &prim.properties;
    let corner1 = match prop_coord_point(p, "corner1") {
        Some(pt) => pt,
        None => return Ok(None),
    };
    let corner2 = match prop_coord_point(p, "corner2") {
        Some(pt) => pt,
        None => return Ok(None),
    };
    Ok(Some(api::Fill {
        id: prim.id.clone(),
        layer: prop_layer(p, "layer"),
        net: prop_string(p, "net"),
        component: prop_string(p, "component"),
        corner1,
        corner2,
        rotation: prop_float(p, "rotation").unwrap_or(0.0),
    }))
}

fn primitive_to_text(prim: &PcbDocPrimitiveSpec) -> Result<Option<api::PcbDocText>, SpecError> {
    let p = &prim.properties;
    let location = match prop_coord_point(p, "at") {
        Some(pt) => pt,
        None => return Ok(None),
    };
    Ok(Some(api::PcbDocText {
        id: prim.id.clone(),
        layer: prop_layer(p, "layer"),
        component: prop_string(p, "component"),
        location,
        text: prop_string(p, "text").unwrap_or_default(),
        height: prop_coord(p, "height").unwrap_or(Coord::from_mils_f64(60.0)),
        width: prop_coord(p, "width").unwrap_or(Coord::from_mils_f64(6.0)),
        rotation: prop_float(p, "rotation").unwrap_or(0.0),
        font_name: prop_string(p, "font_name").unwrap_or_else(|| "Arial".to_string()),
        is_mirrored: prop_bool(p, "is_mirrored").unwrap_or(false),
        is_comment: prop_bool(p, "is_comment").unwrap_or(false),
        is_designator: prop_bool(p, "is_designator").unwrap_or(false),
    }))
}

fn parse_plane_connection(s: &str) -> Option<altium_format_types::pcb::PlaneConnectionStyle> {
    use altium_format_types::pcb::PlaneConnectionStyle;
    match s {
        "no_connect" => Some(PlaneConnectionStyle::NoConnect),
        "relief" => Some(PlaneConnectionStyle::Relief),
        "direct" | "direct_connect" => Some(PlaneConnectionStyle::Direct),
        _ => None,
    }
}

fn bool_to_ini(v: bool) -> String {
    if v { "1".into() } else { "0".into() }
}

// ── SchDoc ────────────────────────────────────────────────────────────────────

/// Apply a SchDocSpec directly to a document.
///
/// For each sheet in the spec (currently always one):
/// 1. Apply sheet metadata (fonts, grid settings, custom size)
/// 2. Add components (matched by designator, add-or-merge)
/// 3. Add low-level objects (wires, buses, labels, etc.)
/// 4. Nets/powers will be implemented later (require pin location resolution)
pub fn apply_spec_schdoc(
    spec: &SchDocSpec,
    doc: &mut SchDoc,
) -> Result<(), SpecError> {
    for sheet_spec in &spec.sheets {
        let mut sheet = doc.sheet()
            .map_err(|e| SpecError::no_span(SpecErrorCode::AltiumFormat, e.to_string()))?;

        // 1. Sheet metadata
        apply_sheet_metadata(&mut sheet, sheet_spec);

        // 2. Components
        for comp_spec in &sheet_spec.components {
            apply_schdoc_component(&mut sheet, comp_spec)?;
        }

        // 3. Low-level objects
        for obj_spec in &sheet_spec.objects {
            let obj = schdoc_object_from_spec(obj_spec);
            sheet.add_object(obj);
        }

        // 4. Nets and powers (wire stub generation)
        // TODO: requires resolving pin locations from placed components.
        // For now, nets/powers are a no-op — they'll be implemented when
        // we add pin location resolution.

        doc.update_sheet(&sheet)
            .map_err(|e| SpecError::no_span(SpecErrorCode::AltiumFormat, e.to_string()))?;
    }
    Ok(())
}

fn apply_sheet_metadata(sheet: &mut api::SchDocSheet, spec: &SheetSpec) {
    if !spec.fonts.is_empty() {
        sheet.fonts = spec.fonts.iter().map(|f| api::Font {
            id: f.id,
            name: f.name.clone(),
            size: f.size,
            bold: f.bold.unwrap_or(false),
            italic: f.italic.unwrap_or(false),
            underline: f.underline.unwrap_or(false),
            strikeout: f.strikeout.unwrap_or(false),
            rotation: f.rotation.unwrap_or(0),
        }).collect();
    }
    if let Some(w) = spec.custom_width {
        sheet.use_custom_sheet = true;
        sheet.custom_width = w;
    }
    if let Some(h) = spec.custom_height {
        sheet.use_custom_sheet = true;
        sheet.custom_height = h;
    }
    if let Some(v) = spec.snap_grid_on { sheet.snap_grid_on = v; }
    if let Some(v) = spec.visible_grid_on { sheet.visible_grid_on = v; }
    if let Some(v) = spec.hot_spot_grid_on { sheet.hot_spot_grid_on = v; }
    if let Some(v) = spec.show_hidden_pins { sheet.show_hidden_pins = v; }
    if let Some(v) = spec.border_on { sheet.border_on = v; }
    if let Some(v) = spec.title_block_on { sheet.title_block_on = v; }
}

fn apply_schdoc_component(
    sheet: &mut api::SchDocSheet,
    spec: &SchDocComponentSpec,
) -> Result<(), SpecError> {
    if let Some(existing) = sheet.component_mut(&spec.designator) {
        existing.location = spec.location;
        if let Some(orient) = spec.orientation {
            existing.orientation = orient;
        }
        if let Some(mirror) = spec.is_mirrored {
            existing.is_mirrored = mirror;
        }
        if let Some(ref desc) = spec.description {
            existing.description = Some(desc.clone());
        }
        match &spec.symbol {
            SymbolRef::Import { name, .. } => {
                existing.lib_reference = name.clone();
            }
            SymbolRef::Literal(name) => {
                existing.lib_reference = name.clone();
            }
        }
    } else {
        let lib_ref = match &spec.symbol {
            SymbolRef::Import { name, .. } => name.clone(),
            SymbolRef::Literal(name) => name.clone(),
        };
        let comp = api::SchDocComponent {
            designator: spec.designator.clone(),
            unique_id: String::new(),
            lib_reference: lib_ref,
            source_library_name: String::new(),
            design_item_id: String::new(),
            library_path: String::new(),
            location: spec.location,
            orientation: spec.orientation.unwrap_or(RotationBy90::Rotate0),
            is_mirrored: spec.is_mirrored.unwrap_or(false),
            description: spec.description.clone(),
            component_kind: ComponentKind::Standard,
            part_count: 1,
            current_part_id: 1,
            display_mode_count: 1,
            show_hidden_pins: false,
            children: Vec::new(),
        };
        sheet.add_object(api::SheetObject::Component(comp));
    }
    Ok(())
}

fn schdoc_object_from_spec(spec: &SchDocObjectSpec) -> api::SheetObject {
    // Default colors: dark red (128,0,0) = 0x000080 in BGR, dark blue (0,0,128) = 0x800000 in BGR,
    // dark green (0,100,0) = 0x006400 in BGR, white (255,255,255) = 0xFFFFFF.
    const DARK_RED: Color = Color::new(0x000080);
    const DARK_BLUE: Color = Color::new(0x800000);
    const DARK_GREEN: Color = Color::new(0x006400);
    const WHITE: Color = Color::WHITE;
    const YELLOW_NOTE: Color = Color::new(0x00C8FF); // 255,200,0 BGR

    match spec {
        SchDocObjectSpec::Wire(w) => api::SheetObject::Wire(api::Wire {
            unique_id: String::new(),
            vertices: w.vertices.clone(),
            color: w.color.unwrap_or(DARK_RED),
            line_width: w.line_width.unwrap_or(PenWidth::Small),
            line_style: w.line_style.unwrap_or(LineStyle::Solid),
        }),
        SchDocObjectSpec::Bus(b) => api::SheetObject::Bus(api::Bus {
            unique_id: String::new(),
            vertices: b.vertices.clone(),
            color: b.color.unwrap_or(DARK_BLUE),
            line_width: b.line_width.unwrap_or(PenWidth::Small),
        }),
        SchDocObjectSpec::NetLabel(n) => api::SheetObject::NetLabel(api::NetLabel {
            unique_id: String::new(),
            text: n.text.clone(),
            location: n.location,
            orientation: n.orientation.unwrap_or(RotationBy90::Rotate0),
            justification: n.justification.unwrap_or(TextJustification::BottomLeft),
            font_id: n.font_id.unwrap_or(1),
            color: n.color.unwrap_or(DARK_RED),
            is_mirrored: n.is_mirrored.unwrap_or(false),
        }),
        SchDocObjectSpec::PowerObject(p) => api::SheetObject::PowerObject(api::PowerObject {
            unique_id: String::new(),
            text: p.text.clone(),
            location: p.location,
            orientation: p.orientation.unwrap_or(RotationBy90::Rotate0),
            style: p.style.unwrap_or(PowerObjectStyle::Bar),
            show_net_name: p.show_net_name.unwrap_or(true),
            font_id: p.font_id.unwrap_or(1),
            color: p.color.unwrap_or(DARK_RED),
            is_cross_sheet_connector: p.is_cross_sheet_connector.unwrap_or(false),
        }),
        SchDocObjectSpec::Port(p) => api::SheetObject::Port(api::Port {
            unique_id: String::new(),
            name: p.name.clone(),
            location: p.location,
            io_type: p.io_type.unwrap_or(PortIoType::Unspecified),
            style: p.style.unwrap_or(PortArrowStyle::None),
            width: p.width.unwrap_or_else(|| Coord::from_mils(100).expect("100 mils fits Coord")),
            height: p.height.unwrap_or_else(|| Coord::from_mils(20).expect("20 mils fits Coord")),
            color: p.color.unwrap_or(DARK_RED),
            area_color: p.area_color.unwrap_or(WHITE),
            text_color: p.text_color.unwrap_or(DARK_RED),
            font_id: p.font_id.unwrap_or(1),
            alignment: p.alignment.unwrap_or(HorizontalAlign::Left),
            harness_type: String::new(),
            border_width: PenWidth::Small,
            auto_size: false,
            port_name_is_hidden: false,
        }),
        SchDocObjectSpec::Junction(j) => api::SheetObject::Junction(api::Junction {
            unique_id: String::new(),
            location: j.location,
            color: j.color.unwrap_or(DARK_GREEN),
        }),
        SchDocObjectSpec::NoConnect(n) => api::SheetObject::NoConnect(api::NoConnect {
            unique_id: String::new(),
            location: n.location,
            color: n.color.unwrap_or(DARK_RED),
            orientation: n.orientation.unwrap_or(RotationBy90::Rotate0),
            symbol: String::new(),
            is_active: true,
            suppress_all: false,
        }),
        SchDocObjectSpec::BusEntry(b) => api::SheetObject::BusEntry(api::BusEntry {
            unique_id: String::new(),
            location: b.location,
            corner: b.corner,
            color: b.color.unwrap_or(DARK_RED),
            line_width: b.line_width.unwrap_or(PenWidth::Small),
        }),
        SchDocObjectSpec::SheetSymbol(s) => api::SheetObject::SheetSymbol(api::SheetSymbol {
            unique_id: String::new(),
            location: s.location,
            x_size: s.x_size.unwrap_or_else(|| Coord::from_mils(100).expect("100 mils fits Coord")),
            y_size: s.y_size.unwrap_or_else(|| Coord::from_mils(100).expect("100 mils fits Coord")),
            color: s.color.unwrap_or(DARK_RED),
            area_color: s.area_color.unwrap_or(WHITE),
            line_width: PenWidth::Small,
            is_solid: false,
            symbol_type: SheetSymbolType::Normal,
            sheet_name: s.sheet_name.clone(),
            file_name: s.file_name.clone().unwrap_or_default(),
            children: s.entries.iter().map(|e| {
                api::SheetSymbolChild::Entry(api::SheetEntry {
                    unique_id: String::new(),
                    name: e.name.clone(),
                    io_type: e.io_type.unwrap_or(PortIoType::Unspecified),
                    side: e.side.unwrap_or(LeftRightSide::Left),
                    distance_from_top: e.distance_from_top.unwrap_or(Coord::ZERO),
                    style: PortArrowStyle::None,
                    color: DARK_RED,
                    area_color: WHITE,
                    text_color: DARK_RED,
                    text_font_id: 1,
                })
            }).collect(),
        }),
        SchDocObjectSpec::ParameterSet(p) => api::SheetObject::ParameterSet(api::ParameterSet {
            unique_id: String::new(),
            location: p.location.unwrap_or(CoordPoint::default()),
            color: DARK_RED,
            orientation: RotationBy90::Rotate0,
            name: p.name.clone(),
            style: 0,
            parameters: p.parameters.iter().map(param_from_spec).collect(),
        }),
        SchDocObjectSpec::Note(n) => api::SheetObject::Note(api::Note {
            unique_id: String::new(),
            location: n.location,
            corner: CoordPoint::default(),
            text: n.text.clone(),
            author: String::new(),
            font_id: n.font_id.unwrap_or(1),
            color: n.color.unwrap_or(DARK_RED),
            area_color: n.area_color.unwrap_or(YELLOW_NOTE),
            text_color: Color::BLACK,
            is_solid: true,
            show_border: true,
            alignment: HorizontalAlign::Left,
            word_wrap: true,
            clip_to_rect: false,
            text_margin: Coord::ZERO,
            collapsed: false,
        }),
        SchDocObjectSpec::Probe(p) => api::SheetObject::Probe(api::Probe {
            unique_id: String::new(),
            location: p.location,
            color: p.color.unwrap_or(DARK_RED),
            orientation: RotationBy90::Rotate0,
            name: p.name.clone(),
        }),
        SchDocObjectSpec::CompileMask(c) => api::SheetObject::CompileMask(api::CompileMask {
            unique_id: String::new(),
            location: c.location,
            corner: c.corner,
            color: c.color.unwrap_or(DARK_RED),
            area_color: WHITE,
            line_width: PenWidth::Small,
            collapsed: false,
        }),
        SchDocObjectSpec::Blanket(b) => api::SheetObject::Blanket(api::Blanket {
            unique_id: String::new(),
            location: b.location,
            corner: b.corner,
            color: b.color.unwrap_or(DARK_RED),
            area_color: WHITE,
            line_style: LineStyle::Solid,
            line_width: PenWidth::Small,
            vertices: b.vertices.clone().unwrap_or_default(),
            collapsed: false,
        }),
        SchDocObjectSpec::Graphic(g) => {
            // Sheet-level graphics default to owner_part_id 0 (not owned by a component part)
            let obj = graphic_from_spec(g, 0).unwrap_or_else(|| {
                api::Graphic::Line(api::LineGraphic {
                    unique_id: g.unique_id.clone(),
                    owner_part_id: 0,
                    location: CoordPoint::default(),
                    corner: CoordPoint::default(),
                    line_width: PenWidth::default(),
                    line_style: LineStyle::default(),
                    color: Color::default(),
                })
            });
            api::SheetObject::Graphic(obj)
        }
        SchDocObjectSpec::Parameter(p) => api::SheetObject::Parameter(param_from_spec(p)),
        SchDocObjectSpec::HarnessConnector(h) => {
            api::SheetObject::HarnessConnector(api::HarnessConnector {
                unique_id: String::new(),
                location: h.location,
                x_size: h.x_size.unwrap_or_else(|| Coord::from_mils(100).expect("100 mils fits Coord")),
                y_size: h.y_size.unwrap_or_else(|| Coord::from_mils(100).expect("100 mils fits Coord")),
                color: h.color.unwrap_or(DARK_RED),
                area_color: h.area_color.unwrap_or(WHITE),
                line_width: PenWidth::Small,
                children: Vec::new(),
            })
        }
        SchDocObjectSpec::SignalHarness(s) => {
            api::SheetObject::SignalHarness(api::SignalHarness {
                unique_id: String::new(),
                vertices: s.vertices.clone(),
                color: s.color.unwrap_or(DARK_RED),
                line_width: s.line_width.unwrap_or(PenWidth::Small),
            })
        }
    }
}

// ── Component from spec (new components) ──────────────────────────────────────

/// Create a complete `api::Component` from a `ComponentSpec`, filling fields
/// not specified in the spec with sensible defaults matching `schlib_write.rs`.
fn component_from_spec(spec: &ComponentSpec) -> api::Component {
    let mut pins: Vec<api::Pin> = spec.pins.iter().map(pin_from_spec).collect();
    for part in &spec.parts {
        pins.extend(part.pins.iter().map(pin_from_spec));
    }

    // For single-part components, all records belong to part 1.
    // For multi-part, component-level graphics are shared (part 0).
    let part_count = spec.part_count.unwrap_or(1);
    let default_owner_part_id = if part_count <= 1 { 1 } else { 0 };

    let mut graphics: Vec<api::Graphic> = spec.graphics.iter()
        .filter_map(|g| graphic_from_spec(g, default_owner_part_id))
        .collect();
    for part in &spec.parts {
        graphics.extend(part.graphics.iter().filter_map(|g| graphic_from_spec(g, part.part_number)));
    }

    api::Component {
        lib_reference: spec.lib_reference.clone(),
        designator: spec.designator.clone(),
        description: spec.description.clone(),
        component_kind: spec.component_kind,
        part_count,
        show_hidden_pins: spec.show_hidden_pins.unwrap_or(false),
        pins,
        parameters: spec.parameters.iter().map(param_from_spec).collect(),
        footprints: spec.footprints.iter().map(footprint_from_spec).collect(),
        graphics,
        aliases: spec.aliases.clone(),
    }
}

// ── Merge spec into existing component ────────────────────────────────────────

/// Merge `ComponentSpec` fields over an existing `api::Component`.
///
/// - Top-level `Option` fields: override if `Some`, preserve if `None`
/// - Children (pins, params, etc.): match by natural key, update matched, add unmatched
/// - Existing children not in spec: preserved (additive-only)
fn merge_spec_into_component(existing: &api::Component, spec: &ComponentSpec) -> api::Component {
    let mut result = existing.clone();

    // Top-level fields: override if spec provides them
    if let Some(ref d) = spec.designator {
        result.designator = Some(d.clone());
    }
    if let Some(ref d) = spec.description {
        result.description = Some(d.clone());
    }
    if let Some(ck) = spec.component_kind {
        result.component_kind = Some(ck);
    }
    if let Some(pc) = spec.part_count {
        result.part_count = pc;
    }
    if let Some(shp) = spec.show_hidden_pins {
        result.show_hidden_pins = shp;
    }

    // Merge pins by designator
    merge_pins(&mut result.pins, &spec.pins);
    for part in &spec.parts {
        merge_pins(&mut result.pins, &part.pins);
    }

    // Merge parameters by name
    merge_params(&mut result.parameters, &spec.parameters);

    // Merge footprints by model_name
    merge_footprints(&mut result.footprints, &spec.footprints);

    // Merge graphics by unique_id
    let default_owner_part_id = if result.part_count <= 1 { 1 } else { 0 };
    merge_graphics(&mut result.graphics, &spec.graphics, default_owner_part_id);
    for part in &spec.parts {
        merge_graphics(&mut result.graphics, &part.graphics, part.part_number);
    }

    // Merge aliases (union)
    for alias in &spec.aliases {
        if !result.aliases.contains(alias) {
            result.aliases.push(alias.clone());
        }
    }

    result
}

// ── Child merge helpers ───────────────────────────────────────────────────────

fn merge_pins(existing: &mut Vec<api::Pin>, spec_pins: &[PinSpec]) {
    for spec_pin in spec_pins {
        if let Some(pin) = existing.iter_mut().find(|p| p.designator == spec_pin.designator) {
            apply_pin_spec(pin, spec_pin);
        } else {
            existing.push(pin_from_spec(spec_pin));
        }
    }
}

/// Update only fields that have `Some` values in the spec.
fn apply_pin_spec(pin: &mut api::Pin, spec: &PinSpec) {
    if let Some(ref name) = spec.name {
        pin.name = name.clone();
    }
    if let Some(elec) = spec.electrical {
        pin.electrical = elec;
    }
    if let Some(len) = spec.length {
        pin.length = len;
    }
    pin.location = spec.location;
    pin.orientation = spec.orientation;
    if let Some(hidden) = spec.is_hidden {
        pin.is_hidden = hidden;
    }
    if let Some(ref hnn) = spec.hidden_net_name {
        pin.hidden_net_name = hnn.clone();
    }
    pin.owner_part_id = spec.owner_part_id;
}

fn merge_params(existing: &mut Vec<api::Parameter>, spec_params: &[ParameterSpec]) {
    for spec_param in spec_params {
        if let Some(param) = existing.iter_mut().find(|p| p.name == spec_param.name) {
            param.text = spec_param.text.clone();
            if let Some(hidden) = spec_param.is_hidden {
                param.is_hidden = hidden;
            }
        } else {
            existing.push(param_from_spec(spec_param));
        }
    }
}

fn merge_footprints(existing: &mut Vec<api::FootprintMap>, spec_fps: &[FootprintMapSpec]) {
    for spec_fp in spec_fps {
        if let Some(fp) = existing.iter_mut().find(|f| f.model_name == spec_fp.model_name) {
            // Update pin-pad maps
            fp.pin_pad_maps = spec_fp.maps.iter().map(|m| api::PinPadMap {
                pin: m.pin.clone(),
                pad: m.pad.clone(),
            }).collect();
        } else {
            existing.push(footprint_from_spec(spec_fp));
        }
    }
}

fn merge_graphics(existing: &mut Vec<api::Graphic>, spec_graphics: &[GraphicSpec], owner_part_id: i32) {
    for spec_graphic in spec_graphics {
        if let Some(pos) = existing.iter().position(|g| {
            g.unique_id().map_or(false, |uid| uid == spec_graphic.unique_id)
        }) {
            // Replace the existing graphic with the new one from spec
            if let Some(new_graphic) = graphic_from_spec(spec_graphic, owner_part_id) {
                existing[pos] = new_graphic;
            }
        } else if let Some(new_graphic) = graphic_from_spec(spec_graphic, owner_part_id) {
            existing.push(new_graphic);
        }
    }
}

// ── Pin conversion ────────────────────────────────────────────────────────────

fn pin_from_spec(spec: &PinSpec) -> api::Pin {
    api::Pin {
        designator: spec.designator.clone(),
        name: spec.name.clone().unwrap_or_else(|| spec.designator.clone()),
        electrical: spec.electrical.unwrap_or(PinElectricalType::Passive),
        location: spec.location,
        length: spec.length.unwrap_or(Coord::from_mils(25).expect("25 mils fits Coord")),
        orientation: spec.orientation,
        is_hidden: spec.is_hidden.unwrap_or(false),
        hidden_net_name: spec.hidden_net_name.clone().unwrap_or_default(),
        owner_part_id: spec.owner_part_id,
        show_name: true,
        show_designator: true,
        symbol_inner_edge: IeeeSymbol::default(),
        symbol_outer_edge: IeeeSymbol::default(),
        symbol_inside: IeeeSymbol::default(),
        symbol_outside: IeeeSymbol::default(),
        swap_id_pin: spec.swap_group.clone().unwrap_or_default(),
        swap_id_part: spec.part_swap_group.clone().unwrap_or_default(),
        swap_id_pair: spec.pair_swap_group.clone().unwrap_or_default(),
        default_value: String::new(),
        pin_package_length: String::new(),
        propagation_delay: String::new(),
        pin_symbol_line_width: None,
        name_text_data: None,
        designator_text_data: None,
        description: String::new(),
        formal_type: StdLogicState::default(),
        spice_pin_name: String::new(),
        unique_id: String::new(), // write path generates if empty
        color: Color::default(),
        is_not_accessible: false,
        graphically_locked: false,
        owner_part_display_mode: 0,
    }
}

// ── Parameter conversion ──────────────────────────────────────────────────────

fn param_from_spec(spec: &ParameterSpec) -> api::Parameter {
    api::Parameter {
        name: spec.name.clone(),
        text: spec.text.clone(),
        is_hidden: spec.is_hidden.unwrap_or(false),
        read_only: ParameterReadOnlyState::default(),
        location: CoordPoint::default(),
        orientation: RotationBy90::Rotate0,
        color: Color::default(),
        font_id: 1,
        justification: TextJustification::default(),
        is_mirrored: false,
        show_name: false,
        unique_id: String::new(),
        not_auto_position: false,
        param_type: ParameterType::default(),
        description: String::new(),
    }
}

// ── Footprint conversion ──────────────────────────────────────────────────────

fn footprint_from_spec(spec: &FootprintMapSpec) -> api::FootprintMap {
    api::FootprintMap {
        model_name: spec.model_name.clone(),
        description: String::new(),
        is_current: false,
        pin_pad_maps: spec.maps.iter().map(|m| api::PinPadMap {
            pin: m.pin.clone(),
            pad: m.pad.clone(),
        }).collect(),
    }
}

// ── Graphic conversion ────────────────────────────────────────────────────────

fn graphic_from_spec(spec: &GraphicSpec, owner_part_id: i32) -> Option<api::Graphic> {
    let props = &spec.properties;
    match spec.graphic_type {
        GraphicType::Line => Some(api::Graphic::Line(api::LineGraphic {
            unique_id: spec.unique_id.clone(),
            owner_part_id,
            location: props.from.unwrap_or_default(),
            corner: props.to.unwrap_or_default(),
            line_width: PenWidth::default(),
            line_style: LineStyle::default(),
            color: props.color.unwrap_or_default(),
        })),
        GraphicType::Rectangle => Some(api::Graphic::Rectangle(api::RectangleGraphic {
            unique_id: spec.unique_id.clone(),
            owner_part_id,
            location: props.from.unwrap_or_default(),
            corner: props.to.unwrap_or_default(),
            line_width: PenWidth::default(),
            line_style: LineStyle::default(),
            color: props.color.unwrap_or_default(),
            area_color: props.area_color.unwrap_or_default(),
            is_solid: props.is_solid.unwrap_or(true),
            transparent: false,
        })),
        GraphicType::RoundRectangle => Some(api::Graphic::RoundRectangle(api::RoundRectangleGraphic {
            unique_id: spec.unique_id.clone(),
            owner_part_id,
            location: props.from.unwrap_or_default(),
            corner: props.to.unwrap_or_default(),
            corner_x_radius: props.corner_x_radius.unwrap_or_default(),
            corner_y_radius: props.corner_y_radius.unwrap_or_default(),
            line_width: PenWidth::default(),
            color: props.color.unwrap_or_default(),
            area_color: props.area_color.unwrap_or_default(),
            is_solid: props.is_solid.unwrap_or(true),
        })),
        GraphicType::Arc => Some(api::Graphic::Arc(api::ArcGraphic {
            unique_id: spec.unique_id.clone(),
            owner_part_id,
            location: props.center.unwrap_or_default(),
            radius: props.radius.unwrap_or_default(),
            start_angle: api::SchAngle(props.start_angle.unwrap_or(0.0)),
            end_angle: props.end_angle.map(api::SchAngle),
            line_width: PenWidth::default(),
            color: props.color.unwrap_or_default(),
        })),
        GraphicType::EllipticalArc => Some(api::Graphic::EllipticalArc(api::EllipticalArcGraphic {
            unique_id: spec.unique_id.clone(),
            owner_part_id,
            location: props.center.unwrap_or_default(),
            radius: props.radius.unwrap_or_default(),
            secondary_radius: props.secondary_radius.unwrap_or_default(),
            start_angle: api::SchAngle(props.start_angle.unwrap_or(0.0)),
            end_angle: props.end_angle.map(api::SchAngle),
            line_width: PenWidth::default(),
            color: props.color.unwrap_or_default(),
        })),
        GraphicType::Ellipse => Some(api::Graphic::Ellipse(api::EllipseGraphic {
            unique_id: spec.unique_id.clone(),
            owner_part_id,
            location: props.center.unwrap_or_default(),
            radius: props.radius.unwrap_or_default(),
            secondary_radius: props.secondary_radius.unwrap_or_default(),
            line_width: PenWidth::default(),
            color: props.color.unwrap_or_default(),
            area_color: props.area_color.unwrap_or_default(),
            is_solid: props.is_solid.unwrap_or(true),
            transparent: false,
        })),
        GraphicType::Pie => Some(api::Graphic::Pie(api::PieGraphic {
            owner_part_id,
            location: props.center.unwrap_or_default(),
            radius: props.radius.unwrap_or_default(),
            start_angle: api::SchAngle(props.start_angle.unwrap_or(0.0)),
            end_angle: props.end_angle.map(api::SchAngle),
            line_width: PenWidth::default(),
            color: props.color.unwrap_or_default(),
            area_color: props.area_color.unwrap_or_default(),
            is_solid: props.is_solid.unwrap_or(true),
        })),
        GraphicType::Polyline => Some(api::Graphic::Polyline(api::PolylineGraphic {
            unique_id: spec.unique_id.clone(),
            owner_part_id,
            vertices: props.points.clone().unwrap_or_default(),
            line_width: PenWidth::default(),
            line_style: LineStyle::default(),
            start_line_shape: LineShape::default(),
            end_line_shape: LineShape::default(),
            line_shape_size: PenWidth::default(),
            color: props.color.unwrap_or_default(),
        })),
        GraphicType::Polygon => Some(api::Graphic::Polygon(api::PolygonGraphic {
            unique_id: spec.unique_id.clone(),
            owner_part_id,
            vertices: props.points.clone().unwrap_or_default(),
            line_width: PenWidth::default(),
            color: props.color.unwrap_or_default(),
            area_color: props.area_color.unwrap_or_default(),
            is_solid: props.is_solid.unwrap_or(true),
            transparent: false,
        })),
        GraphicType::Bezier => Some(api::Graphic::Bezier(api::BezierGraphic {
            unique_id: spec.unique_id.clone(),
            owner_part_id,
            vertices: props.points.clone().unwrap_or_default(),
            line_width: PenWidth::default(),
            color: props.color.unwrap_or_default(),
        })),
        GraphicType::Label => Some(api::Graphic::Label(api::LabelGraphic {
            unique_id: spec.unique_id.clone(),
            owner_part_id,
            location: props.at.unwrap_or_default(),
            orientation: RotationBy90::Rotate0,
            justification: TextJustification::default(),
            color: props.color.unwrap_or_default(),
            font_id: props.font_id.unwrap_or(1),
            text: props.text.clone().unwrap_or_default(),
            is_mirrored: false,
            url: String::new(),
        })),
        GraphicType::TextFrame => Some(api::Graphic::TextFrame(api::TextFrameGraphic {
            unique_id: spec.unique_id.clone(),
            owner_part_id,
            location: props.from.unwrap_or_default(),
            corner: props.to.unwrap_or_default(),
            line_width: PenWidth::default(),
            color: props.color.unwrap_or_default(),
            area_color: props.area_color.unwrap_or_default(),
            text_color: Color::default(),
            font_id: props.font_id.unwrap_or(1),
            is_solid: props.is_solid.unwrap_or(true),
            show_border: props.show_border.unwrap_or(true),
            alignment: HorizontalAlign::default(),
            word_wrap: true,
            clip_to_rect: false,
            text: props.text.clone().unwrap_or_default(),
            text_margin: Coord::default(),
            transparent: false,
        })),
        GraphicType::Image => Some(api::Graphic::Image(api::ImageGraphic {
            unique_id: spec.unique_id.clone(),
            owner_part_id,
            location: props.from.unwrap_or_default(),
            corner: props.to.unwrap_or_default(),
            orientation: RotationBy90::Rotate0,
            line_width: PenWidth::default(),
            color: props.color.unwrap_or_default(),
            is_solid: false,
            keep_aspect: true,
            embed_image: true,
            file_name: props.file_name.clone().unwrap_or_default(),
        })),
    }
}

// ── PCB: Footprint from spec (new footprints) ─────────────────────────────────

/// Create a complete `api::Footprint` from a `FootprintSpec`, filling fields
/// not specified in the spec with sensible defaults.
fn footprint_from_pcblib_spec(spec: &FootprintSpec) -> api::Footprint {
    api::Footprint {
        display_name: spec.display_name.clone(),
        description: spec.description.clone().unwrap_or_default(),
        pattern: spec.pattern.clone().unwrap_or_else(|| spec.display_name.clone()),
        height: spec.height.unwrap_or(Coord::ZERO),
        pads: spec.pads.iter().map(pad_from_pcblib_spec).collect(),
        graphics: spec.graphics.iter().filter_map(pcb_graphic_from_spec).collect(),
    }
}

/// Resolve a `LayerSpec` to a `LayerRef`. Without a board stack, only `Resolved` and
/// V6-name `NamedLayer` variants can be resolved; others fall back to a default.
fn resolve_layer_spec(spec: &LayerSpec) -> LayerRef {
    match spec {
        LayerSpec::Resolved(lr) => lr.clone(),
        LayerSpec::NamedLayer(name) => {
            LayerRef::from_string_name(name)
                .unwrap_or_else(|| LayerRef::from_v6(V6Layer::NoLayer).with_name(name.clone()))
        }
        LayerSpec::CopperPosition(_) => {
            // Cannot resolve without a board stack; fall back to MultiLayer
            LayerRef::from_v6(V6Layer::MultiLayer)
        }
    }
}

fn resolve_layer_spec_opt(spec: &Option<LayerSpec>, default: V6Layer) -> LayerRef {
    match spec {
        Some(s) => resolve_layer_spec(s),
        None => LayerRef::from_v6(default),
    }
}

fn pad_from_pcblib_spec(spec: &PadSpec) -> api::Pad {
    let shape = spec.shape.unwrap_or(PadShape::Rectangular);
    let x_size = spec.x_size.unwrap_or_else(|| Coord::from_mils(60).expect("60 mils fits Coord"));
    let y_size = spec.y_size.unwrap_or_else(|| Coord::from_mils(60).expect("60 mils fits Coord"));
    api::Pad {
        pad_name: spec.pad_name.clone(),
        unique_id: None,
        location: spec.at,
        shape,
        x_size,
        y_size,
        rotation: spec.rotation.unwrap_or(0.0),
        hole_size: spec.hole_size.unwrap_or(Coord::ZERO),
        is_plated: spec.is_plated.unwrap_or(true),
        layer: resolve_layer_spec_opt(&spec.layer, V6Layer::MultiLayer),
        pad_mode: spec.pad_mode.unwrap_or_default(),
        solder_mask_expansion: spec.solder_mask_expansion.unwrap_or(Coord::ZERO),
        paste_mask_expansion: spec.paste_mask_expansion.unwrap_or(Coord::ZERO),
        plane_connection: spec.plane_connection.unwrap_or_default(),
        relief_conductor_width: spec.relief_conductor_width.unwrap_or(Coord::ZERO),
        relief_entries: spec.relief_entries.unwrap_or(4),
        relief_air_gap: spec.relief_air_gap.unwrap_or(Coord::ZERO),
        stack: api::PadStack::simple(shape, x_size, y_size),
    }
}

fn pcb_graphic_from_spec(spec: &PcbGraphicSpec) -> Option<api::PcbGraphic> {
    let props = &spec.properties;
    let layer = resolve_layer_spec_opt(&props.layer, V6Layer::TopOverlay);
    let flags = PcbFlags::default();
    let width = props.width.unwrap_or(Coord::ZERO);

    match spec.graphic_type {
        PcbGraphicType::Track => Some(api::PcbGraphic::Track(api::TrackGraphic {
            unique_id: Some(spec.unique_id.clone()),
            layer,
            flags,
            start: props.from.unwrap_or_default(),
            end: props.to.unwrap_or_default(),
            width,
        })),
        PcbGraphicType::Arc => Some(api::PcbGraphic::Arc(api::PcbArcGraphic {
            unique_id: Some(spec.unique_id.clone()),
            layer,
            flags,
            center: props.center.unwrap_or_default(),
            radius: props.radius.unwrap_or(Coord::ZERO),
            start_angle: props.start_angle.unwrap_or(0.0),
            end_angle: props.end_angle.unwrap_or(360.0),
            width,
        })),
        PcbGraphicType::Fill => Some(api::PcbGraphic::Fill(api::FillGraphic {
            unique_id: Some(spec.unique_id.clone()),
            layer,
            flags,
            corner1: props.from.unwrap_or_default(),
            corner2: props.to.unwrap_or_default(),
            rotation: props.rotation.unwrap_or(0.0),
        })),
        PcbGraphicType::Region => {
            let points = props.points.clone().unwrap_or_default();
            let segments = points
                .iter()
                .map(|pt| api::ContourSegment::Line { endpoint: *pt })
                .collect();
            Some(api::PcbGraphic::Region(api::RegionGraphic {
                unique_id: Some(spec.unique_id.clone()),
                layer,
                flags,
                kind: RegionKind::default(),
                outline: api::PcbContour { segments },
                holes: Vec::new(),
            }))
        }
        PcbGraphicType::Text => Some(api::PcbGraphic::Text(api::TextGraphic {
            unique_id: Some(spec.unique_id.clone()),
            layer,
            flags,
            location: props.at.unwrap_or_default(),
            text: props.text.clone().unwrap_or_default(),
            rotation: props.rotation.unwrap_or(0.0),
            height: props.width.unwrap_or_else(|| Coord::from_mils(60).expect("60 mils fits Coord")),
            width: Coord::ZERO,
            color: altium_format_types::color::Color::default(),
            font_name: String::new(),
            is_mirrored: false,
        })),
        PcbGraphicType::Via => Some(api::PcbGraphic::Via(api::ViaGraphic {
            unique_id: Some(spec.unique_id.clone()),
            layer: LayerRef::from_v6(V6Layer::MultiLayer),
            flags,
            location: props.center.unwrap_or_default(),
            diameter: props.diameter.unwrap_or_else(|| Coord::from_mils(50).expect("50 mils fits Coord")),
            hole_size: props.hole_size.unwrap_or_else(|| Coord::from_mils(28).expect("28 mils fits Coord")),
            from_layer: LayerRef::from_v6(V6Layer::TopLayer),
            to_layer: LayerRef::from_v6(V6Layer::BottomLayer),
            is_testpoint_top: false,
            is_testpoint_bottom: false,
            is_assy_testpoint_top: false,
            is_assy_testpoint_bottom: false,
            solder_mask_override: false,
            use_separate_solder_mask_expansion: false,
            solder_mask_expansion_from_hole_edge: false,
            paste_mask_override: false,
        })),
        PcbGraphicType::ComponentBody | PcbGraphicType::Polyline => None,
    }
}

// ── PCB: Merge spec into existing footprint ────────────────────────────────────

/// Merge `FootprintSpec` fields over an existing `api::Footprint`.
///
/// - Top-level `Option` fields: override if `Some`, preserve if `None`
/// - Children (pads, graphics): match by natural key, update matched, add unmatched
/// - Existing children not in spec: preserved (additive-only)
fn merge_spec_into_footprint(existing: &api::Footprint, spec: &FootprintSpec) -> api::Footprint {
    let mut result = existing.clone();

    if let Some(ref d) = spec.description {
        result.description = d.clone();
    }
    if let Some(ref p) = spec.pattern {
        result.pattern = p.clone();
    }
    if let Some(h) = spec.height {
        result.height = h;
    }

    // Merge pads by pad_name
    for pad_spec in &spec.pads {
        if let Some(pad) = result.pads.iter_mut().find(|p| p.pad_name == pad_spec.pad_name) {
            apply_pad_spec(pad, pad_spec);
        } else {
            result.pads.push(pad_from_pcblib_spec(pad_spec));
        }
    }

    // Merge graphics by unique_id
    for graphic_spec in &spec.graphics {
        if let Some(pos) = result.graphics.iter().position(|g| {
            g.unique_id().map_or(false, |uid| uid == graphic_spec.unique_id)
        }) {
            if let Some(new_graphic) = pcb_graphic_from_spec(graphic_spec) {
                result.graphics[pos] = new_graphic;
            }
        } else if let Some(new_graphic) = pcb_graphic_from_spec(graphic_spec) {
            result.graphics.push(new_graphic);
        }
    }

    result
}

fn apply_pad_spec(pad: &mut api::Pad, spec: &PadSpec) {
    pad.location = spec.at;
    if let Some(shape) = spec.shape { pad.shape = shape; }
    if let Some(x) = spec.x_size { pad.x_size = x; }
    if let Some(y) = spec.y_size { pad.y_size = y; }
    if let Some(r) = spec.rotation { pad.rotation = r; }
    if let Some(h) = spec.hole_size { pad.hole_size = h; }
    if let Some(p) = spec.is_plated { pad.is_plated = p; }
    if let Some(l) = &spec.layer { pad.layer = resolve_layer_spec(l); }
    if let Some(m) = spec.pad_mode { pad.pad_mode = m; }
    if let Some(s) = spec.solder_mask_expansion { pad.solder_mask_expansion = s; }
    if let Some(p) = spec.paste_mask_expansion { pad.paste_mask_expansion = p; }
    if let Some(c) = spec.plane_connection { pad.plane_connection = c; }
    if let Some(w) = spec.relief_conductor_width { pad.relief_conductor_width = w; }
    if let Some(e) = spec.relief_entries { pad.relief_entries = e; }
    if let Some(g) = spec.relief_air_gap { pad.relief_air_gap = g; }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        ComponentSpec, FootprintMapSpec, ParameterSpec, PartSpec, PinPadMap, PinSpec,
        SchLibSpec,
    };
    use altium_format_types::{Coord, CoordPoint, RotationBy90};

    fn make_coord(x_mils: i32, y_mils: i32) -> CoordPoint {
        CoordPoint {
            x: Coord::from_mils(x_mils).expect("test coord"),
            y: Coord::from_mils(y_mils).expect("test coord"),
        }
    }

    fn make_pin(designator: &str, owner_part_id: i32) -> PinSpec {
        PinSpec {
            designator: designator.to_string(),
            name: Some("Pin".to_string()),
            electrical: None,
            length: None,
            location: make_coord(0, 0),
            orientation: RotationBy90::Rotate0,
            is_hidden: None,
            hidden_net_name: None,
            owner_part_id,
            swap_group: None,
            part_swap_group: None,
            pair_swap_group: None,
        }
    }

    fn make_component(lib_ref: &str, pins: Vec<PinSpec>) -> ComponentSpec {
        ComponentSpec {
            annotation: None,
            lib_reference: lib_ref.to_string(),
            designator: Some("R?".to_string()),
            description: Some("A resistor".to_string()),
            component_kind: None,
            part_count: None,
            show_hidden_pins: None,
            pins,
            parameters: vec![],
            aliases: vec![],
            footprints: vec![],
            graphics: vec![],
            parts: vec![],
        }
    }

    fn make_spec(components: Vec<ComponentSpec>) -> SchLibSpec {
        SchLibSpec { components }
    }

    /// Helper: create a blank library and remove the default "Component_1"
    /// that `new_blank_ad26()` creates.
    fn blank_doc() -> SchLib {
        let mut doc = SchLib::new_blank_ad26().expect("blank schlib");
        let _ = doc.remove_component("Component_1");
        doc
    }

    #[test]
    fn apply_to_blank_adds_components() {
        let spec = make_spec(vec![
            make_component("R_0603", vec![make_pin("1", 0), make_pin("2", 0)]),
        ]);
        let mut doc = blank_doc();

        apply_spec_schlib(&spec, &mut doc).unwrap();

        let names = doc.component_names();
        assert_eq!(names, vec!["R_0603"]);

        let comp = doc.component("R_0603").unwrap();
        assert_eq!(comp.pins.len(), 2);
        assert_eq!(comp.designator.as_deref(), Some("R?"));
        assert_eq!(comp.description.as_deref(), Some("A resistor"));
    }

    #[test]
    fn apply_multiple_components() {
        let spec = make_spec(vec![
            make_component("R_0603", vec![make_pin("1", 0), make_pin("2", 0)]),
            make_component("C_0805", vec![make_pin("1", 0), make_pin("2", 0)]),
        ]);
        let mut doc = blank_doc();

        apply_spec_schlib(&spec, &mut doc).unwrap();

        let names = doc.component_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"R_0603".to_string()));
        assert!(names.contains(&"C_0805".to_string()));
    }

    #[test]
    fn apply_updates_existing_component() {
        // First, add a component
        let spec1 = make_spec(vec![
            make_component("R_0603", vec![make_pin("1", 0)]),
        ]);
        let mut doc = blank_doc();
        apply_spec_schlib(&spec1, &mut doc).unwrap();

        // Now update it with a new spec that changes description and adds a pin
        let spec2 = make_spec(vec![ComponentSpec {
            annotation: None,
            lib_reference: "R_0603".to_string(),
            designator: None, // None → preserve existing
            description: Some("Updated resistor".to_string()),
            component_kind: None,
            part_count: None,
            show_hidden_pins: None,
            pins: vec![
                make_pin("1", 0), // existing pin, will be updated
                make_pin("2", 0), // new pin, will be added
            ],
            parameters: vec![],
            aliases: vec![],
            footprints: vec![],
            graphics: vec![],
            parts: vec![],
        }]);

        apply_spec_schlib(&spec2, &mut doc).unwrap();

        let comp = doc.component("R_0603").unwrap();
        assert_eq!(comp.pins.len(), 2);
        assert_eq!(comp.description.as_deref(), Some("Updated resistor"));
        // Designator should be preserved from the first apply
        assert_eq!(comp.designator.as_deref(), Some("R?"));
    }

    #[test]
    fn apply_with_parameters() {
        let spec = make_spec(vec![ComponentSpec {
            annotation: None,
            lib_reference: "R".to_string(),
            designator: Some("R?".to_string()),
            description: None,
            component_kind: None,
            part_count: None,
            show_hidden_pins: None,
            pins: vec![],
            parameters: vec![
                ParameterSpec {
                    name: "MFG".to_string(),
                    text: "ACME".to_string(),
                    is_hidden: None,
                },
            ],
            aliases: vec!["RES".to_string()],
            footprints: vec![FootprintMapSpec {
                model_name: "0603".to_string(),
                maps: vec![
                    PinPadMap { pin: "1".to_string(), pad: "1".to_string() },
                    PinPadMap { pin: "2".to_string(), pad: "2".to_string() },
                ],
                source: None,
            }],
            graphics: vec![],
            parts: vec![],
        }]);

        let mut doc = blank_doc();
        apply_spec_schlib(&spec, &mut doc).unwrap();

        let comp = doc.component("R").unwrap();
        assert_eq!(comp.parameters.len(), 1);
        assert_eq!(comp.parameters[0].name, "MFG");
        assert_eq!(comp.parameters[0].text, "ACME");
        assert_eq!(comp.footprints.len(), 1);
        assert_eq!(comp.footprints[0].model_name, "0603");
        assert_eq!(comp.footprints[0].pin_pad_maps.len(), 2);
        assert_eq!(comp.aliases, vec!["RES"]);
    }

    #[test]
    fn apply_multi_part_component() {
        let spec = make_spec(vec![ComponentSpec {
            annotation: None,
            lib_reference: "LM358".to_string(),
            designator: Some("U?".to_string()),
            description: Some("Dual Op-Amp".to_string()),
            component_kind: None,
            part_count: Some(2),
            show_hidden_pins: None,
            pins: vec![
                make_pin("4", 0), // shared GND
                make_pin("8", 0), // shared VCC
            ],
            parameters: vec![],
            aliases: vec![],
            footprints: vec![],
            graphics: vec![],
            parts: vec![
                PartSpec {
                    part_number: 1,
                    pins: vec![make_pin("1", 1), make_pin("2", 1), make_pin("3", 1)],
                    graphics: vec![],
                },
                PartSpec {
                    part_number: 2,
                    pins: vec![make_pin("5", 2), make_pin("6", 2), make_pin("7", 2)],
                    graphics: vec![],
                },
            ],
        }]);

        let mut doc = blank_doc();
        apply_spec_schlib(&spec, &mut doc).unwrap();

        let comp = doc.component("LM358").unwrap();
        assert_eq!(comp.pins.len(), 8); // 2 shared + 3 part1 + 3 part2
        assert_eq!(comp.part_count, 2);
    }

    #[test]
    fn merge_preserves_existing_children() {
        // Create a component with specific pins
        let spec1 = make_spec(vec![ComponentSpec {
            annotation: None,
            lib_reference: "R".to_string(),
            designator: Some("R?".to_string()),
            description: None,
            component_kind: None,
            part_count: None,
            show_hidden_pins: None,
            pins: vec![make_pin("1", 0), make_pin("2", 0)],
            parameters: vec![
                ParameterSpec { name: "MFG".to_string(), text: "ACME".to_string(), is_hidden: None },
            ],
            aliases: vec!["RES".to_string()],
            footprints: vec![],
            graphics: vec![],
            parts: vec![],
        }]);
        let mut doc = blank_doc();
        apply_spec_schlib(&spec1, &mut doc).unwrap();

        // Apply spec that only mentions pin "1" and a new parameter
        let spec2 = make_spec(vec![ComponentSpec {
            annotation: None,
            lib_reference: "R".to_string(),
            designator: None,
            description: None,
            component_kind: None,
            part_count: None,
            show_hidden_pins: None,
            pins: vec![PinSpec {
                designator: "1".to_string(),
                name: Some("Updated".to_string()),
                electrical: None,
                length: None,
                location: make_coord(0, 0),
                orientation: RotationBy90::Rotate0,
                is_hidden: None,
                hidden_net_name: None,
                owner_part_id: 0,
                swap_group: None,
                part_swap_group: None,
                pair_swap_group: None,
            }],
            parameters: vec![
                ParameterSpec { name: "VALUE".to_string(), text: "10K".to_string(), is_hidden: None },
            ],
            aliases: vec!["RESISTOR".to_string()],
            footprints: vec![],
            graphics: vec![],
            parts: vec![],
        }]);
        apply_spec_schlib(&spec2, &mut doc).unwrap();

        let comp = doc.component("R").unwrap();
        // Pin "2" should still exist (additive-only)
        assert_eq!(comp.pins.len(), 2);
        // Pin "1" should have updated name
        let pin1 = comp.pins.iter().find(|p| p.designator == "1").unwrap();
        assert_eq!(pin1.name, "Updated");
        // Both old and new parameters should exist
        assert_eq!(comp.parameters.len(), 2);
        // Both old and new aliases should exist
        assert!(comp.aliases.contains(&"RES".to_string()));
        assert!(comp.aliases.contains(&"RESISTOR".to_string()));
    }
    // ── PcbLib executor tests ──────────────────────────────────────────────────

    fn make_pad_spec(name: &str) -> PadSpec {
        PadSpec {
            pad_name: name.to_string(),
            at: CoordPoint { x: Coord::from_mils(0).expect("0 mils fits Coord"), y: Coord::from_mils(0).expect("0 mils fits Coord") },
            shape: None,
            x_size: None,
            y_size: None,
            rotation: None,
            hole_size: None,
            is_plated: None,
            layer: None,
            pad_mode: None,
            solder_mask_expansion: None,
            paste_mask_expansion: None,
            plane_connection: None,
            relief_conductor_width: None,
            relief_entries: None,
            relief_air_gap: None,
        }
    }

    fn make_footprint_spec(name: &str, pads: Vec<PadSpec>) -> FootprintSpec {
        FootprintSpec {
            annotation: None,
            display_name: name.to_string(),
            description: Some(format!("{name} footprint")),
            height: None,
            pattern: None,
            pads,
            graphics: vec![],
        }
    }

    #[test]
    fn executor_pcblib_add_to_blank() {
        let spec = PcbLibSpec {
            footprints: vec![
                make_footprint_spec("R0603", vec![make_pad_spec("1"), make_pad_spec("2")]),
            ],
        };
        let mut lib = PcbLib::new_blank_ad26().expect("blank pcblib");

        apply_spec_pcblib(&spec, &mut lib).unwrap();

        let fp = lib.footprint("R0603").unwrap();
        assert_eq!(fp.display_name, "R0603");
        assert_eq!(fp.pads.len(), 2);
        assert_eq!(fp.description, "R0603 footprint");
        assert_eq!(fp.pattern, "R0603");
    }

    #[test]
    fn executor_pcblib_merge() {
        let spec1 = PcbLibSpec {
            footprints: vec![make_footprint_spec("C0805", vec![make_pad_spec("1")])],
        };
        let mut lib = PcbLib::new_blank_ad26().expect("blank pcblib");
        apply_spec_pcblib(&spec1, &mut lib).unwrap();

        let spec2 = PcbLibSpec {
            footprints: vec![FootprintSpec {
                annotation: None,
                display_name: "C0805".to_string(),
                description: Some("Updated cap".to_string()),
                height: None,
                pattern: None,
                pads: vec![make_pad_spec("1"), make_pad_spec("2")],
                graphics: vec![],
            }],
        };
        apply_spec_pcblib(&spec2, &mut lib).unwrap();

        let fp = lib.footprint("C0805").unwrap();
        assert_eq!(fp.pads.len(), 2);
        assert_eq!(fp.description, "Updated cap");
    }
}
