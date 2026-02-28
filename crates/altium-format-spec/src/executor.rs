//! Executor: applies spec models directly to Altium documents.
//!
//! Uses the high-level `altium_format::api` types for querying and mutating
//! documents, converting spec model types into API types.

use altium_format::api;
use altium_format::{PcbLib, SchLib};

use altium_format_types::color::Color;
use altium_format_types::coord::{Coord, CoordPoint};
use altium_format_types::common::RotationBy90;
use altium_format_types::sch::{
    IeeeSymbol, LineStyle, PenWidth, ParameterReadOnlyState, ParameterType,
    PinElectricalType, StdLogicState, TextJustification, LineShape, HorizontalAlign,
};

use crate::eval::{SpecError, SpecErrorCode};
use crate::model::{
    ComponentSpec, FootprintMapSpec, GraphicSpec, GraphicType,
    ParameterSpec, PcbLibSpec, PinSpec, SchLibSpec,
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
pub fn apply_spec_pcblib(
    _spec: &PcbLibSpec,
    _lib: &mut PcbLib,
) -> Result<(), SpecError> {
    Err(SpecError::no_span(
        SpecErrorCode::AltiumFormat,
        "PcbLib executor not yet implemented (no PcbLib high-level write API)",
    ))
}

// ── Component from spec (new components) ──────────────────────────────────────

/// Create a complete `api::Component` from a `ComponentSpec`, filling fields
/// not specified in the spec with sensible defaults matching `schlib_write.rs`.
fn component_from_spec(spec: &ComponentSpec) -> api::Component {
    let mut pins: Vec<api::Pin> = spec.pins.iter().map(pin_from_spec).collect();
    for part in &spec.parts {
        pins.extend(part.pins.iter().map(pin_from_spec));
    }

    let mut graphics: Vec<api::Graphic> = spec.graphics.iter()
        .filter_map(graphic_from_spec)
        .collect();
    for part in &spec.parts {
        graphics.extend(part.graphics.iter().filter_map(graphic_from_spec));
    }

    api::Component {
        lib_reference: spec.lib_reference.clone(),
        designator: spec.designator.clone(),
        description: spec.description.clone(),
        component_kind: spec.component_kind,
        part_count: spec.part_count.unwrap_or(1),
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
    merge_graphics(&mut result.graphics, &spec.graphics);
    for part in &spec.parts {
        merge_graphics(&mut result.graphics, &part.graphics);
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

fn merge_graphics(existing: &mut Vec<api::Graphic>, spec_graphics: &[GraphicSpec]) {
    for spec_graphic in spec_graphics {
        if let Some(pos) = existing.iter().position(|g| {
            g.unique_id().map_or(false, |uid| uid == spec_graphic.unique_id)
        }) {
            // Replace the existing graphic with the new one from spec
            if let Some(new_graphic) = graphic_from_spec(spec_graphic) {
                existing[pos] = new_graphic;
            }
        } else if let Some(new_graphic) = graphic_from_spec(spec_graphic) {
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

fn graphic_from_spec(spec: &GraphicSpec) -> Option<api::Graphic> {
    let props = &spec.properties;
    match spec.graphic_type {
        GraphicType::Line => Some(api::Graphic::Line(api::LineGraphic {
            unique_id: spec.unique_id.clone(),
            owner_part_id: 0,
            location: props.from.unwrap_or_default(),
            corner: props.to.unwrap_or_default(),
            line_width: PenWidth::default(),
            line_style: LineStyle::default(),
            color: props.color.unwrap_or_default(),
        })),
        GraphicType::Rectangle => Some(api::Graphic::Rectangle(api::RectangleGraphic {
            unique_id: spec.unique_id.clone(),
            owner_part_id: 0,
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
            owner_part_id: 0,
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
            owner_part_id: 0,
            location: props.center.unwrap_or_default(),
            radius: props.radius.unwrap_or_default(),
            start_angle: api::SchAngle(props.start_angle.unwrap_or(0.0)),
            end_angle: props.end_angle.map(api::SchAngle),
            line_width: PenWidth::default(),
            color: props.color.unwrap_or_default(),
        })),
        GraphicType::EllipticalArc => Some(api::Graphic::EllipticalArc(api::EllipticalArcGraphic {
            unique_id: spec.unique_id.clone(),
            owner_part_id: 0,
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
            owner_part_id: 0,
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
            owner_part_id: 0,
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
            owner_part_id: 0,
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
            owner_part_id: 0,
            vertices: props.points.clone().unwrap_or_default(),
            line_width: PenWidth::default(),
            color: props.color.unwrap_or_default(),
            area_color: props.area_color.unwrap_or_default(),
            is_solid: props.is_solid.unwrap_or(false),
            transparent: false,
        })),
        GraphicType::Bezier => Some(api::Graphic::Bezier(api::BezierGraphic {
            unique_id: spec.unique_id.clone(),
            owner_part_id: 0,
            vertices: props.points.clone().unwrap_or_default(),
            line_width: PenWidth::default(),
            color: props.color.unwrap_or_default(),
        })),
        GraphicType::Label => Some(api::Graphic::Label(api::LabelGraphic {
            unique_id: spec.unique_id.clone(),
            owner_part_id: 0,
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
            owner_part_id: 0,
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
            owner_part_id: 0,
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
}
