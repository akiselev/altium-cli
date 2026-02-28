//! Executor: applies spec models directly to Altium documents.
//!
//! Uses the high-level `altium_format::api` types for querying and mutating
//! documents, converting spec model types into API types.

use altium_format::api;
use altium_format::{AltiumProject, PcbLib, SchLib};

use altium_format_types::color::Color;
use altium_format_types::coord::{Coord, CoordPoint};
use altium_format_types::common::RotationBy90;
use altium_format_types::sch::{
    IeeeSymbol, LineStyle, PenWidth, ParameterReadOnlyState, ParameterType,
    PinElectricalType, StdLogicState, TextJustification, LineShape, HorizontalAlign,
};

use altium_format_types::pcb::{PadShape, PcbFlags, RegionKind, V6Layer};

use crate::eval::{SpecError, SpecErrorCode};
use crate::model::{
    ComponentSpec, FootprintMapSpec, FootprintSpec, GraphicSpec, GraphicType,
    PadSpec, ParameterSpec, PcbGraphicSpec, PcbGraphicType, PcbLibSpec,
    PinSpec, PrjPcbSpec, SchLibSpec,
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

fn bool_to_ini(v: bool) -> String {
    if v { "1".into() } else { "0".into() }
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
        name: spec.name.clone().unwrap_or_default(),
        electrical: spec.electrical.unwrap_or(PinElectricalType::Passive),
        location: spec.location,
        length: spec.length.unwrap_or(Coord::from_mils(25)),
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
        swap_id_pin: String::new(),
        swap_id_part: String::new(),
        swap_id_pair: String::new(),
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
            is_solid: props.is_solid.unwrap_or(false),
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
            is_solid: props.is_solid.unwrap_or(false),
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
            is_solid: props.is_solid.unwrap_or(false),
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
            is_solid: props.is_solid.unwrap_or(false),
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
            is_solid: props.is_solid.unwrap_or(false),
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
            is_solid: props.is_solid.unwrap_or(false),
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

fn pad_from_pcblib_spec(spec: &PadSpec) -> api::Pad {
    api::Pad {
        pad_name: spec.pad_name.clone(),
        unique_id: None,
        location: spec.at,
        shape: spec.shape.unwrap_or(PadShape::Round),
        x_size: spec.x_size.unwrap_or_else(|| Coord::from_mils(60)),
        y_size: spec.y_size.unwrap_or_else(|| Coord::from_mils(60)),
        rotation: spec.rotation.unwrap_or(0.0),
        hole_size: spec.hole_size.unwrap_or(Coord::ZERO),
        is_plated: spec.is_plated.unwrap_or(true),
        layer: spec.layer.unwrap_or(V6Layer::MultiLayer),
        pad_mode: spec.pad_mode.unwrap_or_default(),
        solder_mask_expansion: spec.solder_mask_expansion.unwrap_or(Coord::ZERO),
        paste_mask_expansion: spec.paste_mask_expansion.unwrap_or(Coord::ZERO),
        plane_connection: spec.plane_connection.unwrap_or_default(),
        relief_conductor_width: spec.relief_conductor_width.unwrap_or(Coord::ZERO),
        relief_entries: spec.relief_entries.unwrap_or(4),
        relief_air_gap: spec.relief_air_gap.unwrap_or(Coord::ZERO),
    }
}

fn pcb_graphic_from_spec(spec: &PcbGraphicSpec) -> Option<api::PcbGraphic> {
    let props = &spec.properties;
    let layer = props.layer.unwrap_or(V6Layer::TopOverlay);
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
        PcbGraphicType::Region => Some(api::PcbGraphic::Region(api::RegionGraphic {
            unique_id: Some(spec.unique_id.clone()),
            layer,
            flags,
            kind: RegionKind::default(),
            outline: props.points.clone().unwrap_or_default(),
            holes: Vec::new(),
        })),
        PcbGraphicType::Text => Some(api::PcbGraphic::Text(api::TextGraphic {
            unique_id: Some(spec.unique_id.clone()),
            layer,
            flags,
            location: props.at.unwrap_or_default(),
            text: props.text.clone().unwrap_or_default(),
            rotation: props.rotation.unwrap_or(0.0),
            height: props.width.unwrap_or_else(|| Coord::from_mils(60)),
            width: Coord::ZERO,
            color: altium_format_types::color::Color::default(),
            font_name: String::new(),
            is_mirrored: false,
        })),
        PcbGraphicType::Via => Some(api::PcbGraphic::Via(api::ViaGraphic {
            unique_id: Some(spec.unique_id.clone()),
            layer: V6Layer::MultiLayer,
            flags,
            location: props.center.unwrap_or_default(),
            diameter: props.diameter.unwrap_or_else(|| Coord::from_mils(50)),
            hole_size: props.hole_size.unwrap_or_else(|| Coord::from_mils(28)),
            from_layer: V6Layer::TopLayer,
            to_layer: V6Layer::BottomLayer,
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
    if let Some(l) = spec.layer { pad.layer = l; }
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
            x: Coord::from_mils(x_mils),
            y: Coord::from_mils(y_mils),
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
        }
    }

    fn make_component(lib_ref: &str, pins: Vec<PinSpec>) -> ComponentSpec {
        ComponentSpec {
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
        let mut doc = SchLib::new_blank_ad26();
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
            at: CoordPoint { x: Coord::from_mils(0), y: Coord::from_mils(0) },
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
        let mut lib = PcbLib::new_blank_ad26();

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
        let mut lib = PcbLib::new_blank_ad26();
        apply_spec_pcblib(&spec1, &mut lib).unwrap();

        let spec2 = PcbLibSpec {
            footprints: vec![FootprintSpec {
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
