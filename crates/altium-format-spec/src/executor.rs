//! Executor: applies spec models directly to Altium documents via LowOps.
//!
//! This module provides the direct SpecModel → LowOps pipeline, which emits
//! LowOps directly from the typed SpecModel + document state. Used by the
//! `apply` command.

use altium_format::sch_ops_core::{
    AddAliasOp, AddArcOp, AddBezierOp, AddEllipseOp, AddEllipticalArcOp, AddImageOp, AddLabelOp,
    AddLineOp, AddPieOp, AddPolygonOp, AddPolylineOp, AddRectangleOp, AddRoundRectangleOp,
    AddTextFrameOp, ComponentRefOp, ComponentRootOp, ComponentTextOp, EditComponentOp,
    EditParameterOp, EditPinOp, ImplementationOp, MapDefinerOp, OpResult, ParameterOp, PinOp,
    RefExpr, SchLibLowOp, apply_schlib_low_ops,
};
use altium_format::pcb_ops_core::{AddFootprintOp, AddPadOp, PcbLibLowOp, apply_pcblib_low_ops};
use altium_format::{PcbLib, SchLib};
use altium_format_types::{Coord, CoordPoint};

use crate::eval::SpecError;
use crate::model::{
    ComponentSpec, GraphicSpec, GraphicType, ParameterSpec, PcbLibSpec, PinSpec, SchLibSpec,
};
use crate::reconciler::{DocComponent, DocParameter, DocPin, DocView, query_doc_view};

/// Apply a SchLib spec directly to a document, emitting LowOps without going
/// through ECO/HighOps.
pub fn apply_spec_schlib(
    spec: &SchLibSpec,
    doc: &mut SchLib,
) -> Result<Vec<OpResult>, SpecError> {
    let doc_view = query_doc_view(doc)?;
    let ops = build_schlib_low_ops(spec, &doc_view);
    let results = apply_schlib_low_ops(doc, &ops).map_err(|e| {
        SpecError::no_span(
            crate::eval::SpecErrorCode::TypeMismatch,
            format!("failed to apply ops: {e}"),
        )
    })?;
    Ok(results)
}

/// Apply a PcbLib spec directly to a document, emitting LowOps without going
/// through ECO/HighOps.
pub fn apply_spec_pcblib(
    spec: &PcbLibSpec,
    lib: &mut PcbLib,
) -> Result<Vec<OpResult>, SpecError> {
    let ops = build_pcblib_low_ops(spec);
    let results = apply_pcblib_low_ops(lib, &ops).map_err(|e| {
        SpecError::no_span(
            crate::eval::SpecErrorCode::TypeMismatch,
            format!("failed to apply ops: {e}"),
        )
    })?;
    // Convert PcbLib OpResults to the common OpResult type (they're the same struct)
    Ok(results)
}

fn build_schlib_low_ops(spec: &SchLibSpec, doc_view: &DocView) -> Vec<SchLibLowOp> {
    let mut ops = Vec::new();
    for comp_spec in &spec.components {
        let doc_comp = doc_view.components.get(&comp_spec.lib_reference.to_lowercase());
        match doc_comp {
            None => emit_add_component_ops(comp_spec, &mut ops),
            Some(doc_comp) => emit_update_component_ops(comp_spec, doc_comp, &mut ops),
        }
    }
    ops
}

fn emit_add_component_ops(spec: &ComponentSpec, ops: &mut Vec<SchLibLowOp>) {
    let comp_opid = format!("spec:component:{}", spec.lib_reference);

    // CreateComponentRoot
    ops.push(SchLibLowOp::CreateComponentRoot(ComponentRootOp {
        opid: format!("{comp_opid}/create_component_root"),
        id: None,
        lib_reference: spec.lib_reference.clone(),
        designator: spec.designator.clone(),
        value: None,
    }));

    // CreateComponentDesignator
    if let Some(ref designator) = spec.designator {
        ops.push(SchLibLowOp::CreateComponentDesignator(ComponentTextOp {
            opid: format!("{comp_opid}/create_component_designator"),
            component_ref: None,
            text: designator.clone(),
        }));
    }

    // CreateComponentComment (empty)
    ops.push(SchLibLowOp::CreateComponentComment(ComponentTextOp {
        opid: format!("{comp_opid}/create_component_comment"),
        component_ref: None,
        text: String::new(),
    }));

    // EditComponent for description, part_count, etc. (if needed)
    let needs_edit = spec.description.is_some()
        || spec.part_count.is_some()
        || spec.component_kind.is_some()
        || spec.show_hidden_pins.is_some();
    let root_ref = RefExpr::op(format!("{comp_opid}/create_component_root"));
    if needs_edit {
        ops.push(SchLibLowOp::EditComponent(EditComponentOp {
            opid: format!("{comp_opid}/edit_component"),
            component_ref: root_ref.clone(),
            description: spec.description.clone(),
            part_count: spec.part_count,
            display_mode_count: None,
            component_kind: spec.component_kind.map(|k| k as i32),
            show_hidden_pins: spec.show_hidden_pins,
        }));
    }

    // Pins (component-level, owner_part_id = 0)
    for pin in &spec.pins {
        emit_add_pin_op(&comp_opid, pin, ops);
    }

    // Part-scoped pins
    for part in &spec.parts {
        for pin in &part.pins {
            emit_add_pin_op(&comp_opid, pin, ops);
        }
    }

    // Parameters
    for param in &spec.parameters {
        ops.push(SchLibLowOp::AddParameter(ParameterOp {
            opid: format!("{comp_opid}/parameter:{}", param.name),
            component_ref: None,
            name: param.name.clone(),
            text: param.text.clone(),
            is_hidden: param.is_hidden,
        }));
    }

    // Aliases
    for alias in &spec.aliases {
        ops.push(SchLibLowOp::AddAlias(AddAliasOp {
            opid: format!("{comp_opid}/alias:{alias}"),
            component_ref: root_ref.clone(),
            alias_name: alias.clone(),
        }));
    }

    // Graphics
    for graphic in &spec.graphics {
        emit_graphic_low_op(&comp_opid, graphic, None, ops);
    }

    // Part-scoped graphics
    for part in &spec.parts {
        for graphic in &part.graphics {
            emit_graphic_low_op(&comp_opid, graphic, Some(part.part_number), ops);
        }
    }

    // Footprint maps → implementation chain
    for fp in &spec.footprints {
        emit_footprint_map_ops(&comp_opid, fp, ops);
    }
}

fn emit_update_component_ops(
    spec: &ComponentSpec,
    doc: &DocComponent,
    ops: &mut Vec<SchLibLowOp>,
) {
    let comp_opid = format!("spec:component:{}", spec.lib_reference);
    let comp_ref = RefExpr::op(comp_opid.clone());

    // Component-level property changes
    let desc_changed = spec.description.as_ref().map_or(false, |d| d != &doc.description);
    let part_count_changed = spec.part_count.map_or(false, |p| p != doc.part_count);
    let kind_changed = spec.component_kind.is_some(); // always apply if set
    let show_hidden_changed = spec.show_hidden_pins.is_some(); // always apply if set

    if desc_changed || part_count_changed || kind_changed || show_hidden_changed {
        ops.push(SchLibLowOp::EditComponent(EditComponentOp {
            opid: format!("{comp_opid}/edit_component"),
            component_ref: comp_ref.clone(),
            description: if desc_changed { spec.description.clone() } else { None },
            part_count: if part_count_changed { spec.part_count } else { None },
            display_mode_count: None,
            component_kind: spec.component_kind.map(|k| k as i32),
            show_hidden_pins: spec.show_hidden_pins,
        }));
    }

    // Pins (component-level)
    for pin_spec in &spec.pins {
        let key = (pin_spec.owner_part_id, pin_spec.designator.to_lowercase());
        let doc_pin = doc.pins.get(&key);
        emit_pin_ops(&comp_opid, &comp_ref, pin_spec, doc_pin, ops);
    }

    // Part-scoped pins
    for part in &spec.parts {
        for pin_spec in &part.pins {
            let key = (part.part_number, pin_spec.designator.to_lowercase());
            let doc_pin = doc.pins.get(&key);
            emit_pin_ops(&comp_opid, &comp_ref, pin_spec, doc_pin, ops);
        }
    }

    // Parameters
    for param_spec in &spec.parameters {
        let doc_param = doc.parameters.get(&param_spec.name.to_lowercase());
        emit_parameter_ops(&comp_opid, &comp_ref, param_spec, doc_param, ops);
    }

    // Aliases
    let doc_aliases_lower: Vec<String> = doc.aliases.iter().map(|a| a.to_lowercase()).collect();
    for alias in &spec.aliases {
        if !doc_aliases_lower.contains(&alias.to_lowercase()) {
            ops.push(SchLibLowOp::AddAlias(AddAliasOp {
                opid: format!("{comp_opid}/alias:{alias}"),
                component_ref: comp_ref.clone(),
                alias_name: alias.clone(),
            }));
        }
    }

    // Graphics — always re-add (no doc-side query yet)
    for graphic in &spec.graphics {
        emit_graphic_low_op(&comp_opid, graphic, None, ops);
    }

    // Part-scoped graphics
    for part in &spec.parts {
        for graphic in &part.graphics {
            emit_graphic_low_op(&comp_opid, graphic, Some(part.part_number), ops);
        }
    }

    // Footprint maps — always re-add (no doc-side query yet)
    for fp in &spec.footprints {
        emit_footprint_map_ops(&comp_opid, fp, ops);
    }
}

fn emit_add_pin_op(comp_opid: &str, pin: &PinSpec, ops: &mut Vec<SchLibLowOp>) {
    ops.push(SchLibLowOp::AddPin(PinOp {
        opid: format!("{comp_opid}/pin:{}", pin.designator),
        component_ref: None,
        designator: pin.designator.clone(),
        name: pin.name.clone(),
        electrical: pin.electrical.map(|e| format!("{e:?}")),
        length: pin.length,
        at: Some(pin.location),
        rotation: Some(pin.orientation),
    }));
}

fn emit_pin_ops(
    comp_opid: &str,
    comp_ref: &RefExpr,
    spec: &PinSpec,
    doc: Option<&DocPin>,
    ops: &mut Vec<SchLibLowOp>,
) {
    match doc {
        None => emit_add_pin_op(comp_opid, spec, ops),
        Some(doc_pin) => {
            // Check if anything changed
            let name_changed = spec.name.as_ref().map_or(false, |n| n != &doc_pin.name);
            let elec_changed = spec.electrical.map_or(false, |e| {
                let spec_elec = format!("{e:?}");
                !spec_elec.eq_ignore_ascii_case(&doc_pin.electrical)
            });
            let hidden_changed = spec.is_hidden.map_or(false, |h| h != doc_pin.is_hidden);

            if name_changed || elec_changed || hidden_changed {
                ops.push(SchLibLowOp::EditPin(EditPinOp {
                    opid: format!("{comp_opid}/pin:{}", spec.designator),
                    component_ref: comp_ref.clone(),
                    designator: spec.designator.clone(),
                    owner_part_id: Some(spec.owner_part_id),
                    name: if name_changed { spec.name.clone() } else { None },
                    electrical: if elec_changed {
                        spec.electrical.map(|e| format!("{e:?}"))
                    } else {
                        None
                    },
                    is_hidden: if hidden_changed { spec.is_hidden } else { None },
                }));
            }
        }
    }
}

fn emit_parameter_ops(
    comp_opid: &str,
    comp_ref: &RefExpr,
    spec: &ParameterSpec,
    doc: Option<&DocParameter>,
    ops: &mut Vec<SchLibLowOp>,
) {
    match doc {
        None => {
            ops.push(SchLibLowOp::AddParameter(ParameterOp {
                opid: format!("{comp_opid}/parameter:{}", spec.name),
                component_ref: Some(comp_ref.clone()),
                name: spec.name.clone(),
                text: spec.text.clone(),
                is_hidden: spec.is_hidden,
            }));
        }
        Some(doc_param) => {
            let text_changed = spec.text != doc_param.text;
            let hidden_changed = spec.is_hidden.map_or(false, |h| h != doc_param.is_hidden);

            if text_changed || hidden_changed {
                ops.push(SchLibLowOp::EditParameter(EditParameterOp {
                    opid: format!("{comp_opid}/parameter:{}", spec.name),
                    component_ref: comp_ref.clone(),
                    name: spec.name.clone(),
                    text: if text_changed { Some(spec.text.clone()) } else { None },
                    is_hidden: if hidden_changed { spec.is_hidden } else { None },
                }));
            }
        }
    }
}

fn emit_graphic_low_op(
    comp_opid: &str,
    graphic: &GraphicSpec,
    owner_part_id: Option<i32>,
    ops: &mut Vec<SchLibLowOp>,
) {
    let opid = format!("{comp_opid}/graphic:{}", graphic.unique_id);
    let p = &graphic.properties;
    let color = p.color.map(|c| c.raw());
    let area_color = p.area_color.map(|c| c.raw());
    let line_width = p.line_width.map(|w| w.to_mils() as i32);

    match graphic.graphic_type {
        GraphicType::Line => {
            let from = p.from.unwrap_or(CoordPoint::new(Coord::from_mils(0), Coord::from_mils(0)));
            let to = p.to.unwrap_or(CoordPoint::new(Coord::from_mils(0), Coord::from_mils(0)));
            ops.push(SchLibLowOp::AddLine(AddLineOp {
                opid,
                component_ref: None,
                from,
                to,
                color,
                line_width,
                line_style: None,
                owner_part_id,
                owner_part_display_mode: None,
            }));
        }
        GraphicType::Rectangle => {
            let from = p.from.unwrap_or(CoordPoint::new(Coord::from_mils(0), Coord::from_mils(0)));
            let to = p.to.unwrap_or(CoordPoint::new(Coord::from_mils(0), Coord::from_mils(0)));
            ops.push(SchLibLowOp::AddRectangle(AddRectangleOp {
                opid,
                component_ref: None,
                from,
                to,
                color,
                area_color,
                is_solid: p.is_solid,
                transparent: None,
                line_width,
                owner_part_id,
                owner_part_display_mode: None,
            }));
        }
        GraphicType::Arc => {
            let c = p.center.unwrap_or(CoordPoint::new(Coord::from_mils(0), Coord::from_mils(0)));
            let r = p.radius.map(|r| r.to_mils() as i32).unwrap_or(0);
            ops.push(SchLibLowOp::AddArc(AddArcOp {
                opid,
                component_ref: None,
                cx_mils: c.x.to_mils() as i32,
                cy_mils: c.y.to_mils() as i32,
                radius_mils: r,
                start_angle: p.start_angle,
                end_angle: p.end_angle,
                color,
                line_width,
                owner_part_id,
                owner_part_display_mode: None,
            }));
        }
        GraphicType::EllipticalArc => {
            let c = p.center.unwrap_or(CoordPoint::new(Coord::from_mils(0), Coord::from_mils(0)));
            let r = p.radius.map(|r| r.to_mils() as i32).unwrap_or(0);
            let sr = p.secondary_radius.map(|r| r.to_mils() as i32).unwrap_or(r);
            ops.push(SchLibLowOp::AddEllipticalArc(AddEllipticalArcOp {
                opid,
                component_ref: None,
                cx_mils: c.x.to_mils() as i32,
                cy_mils: c.y.to_mils() as i32,
                radius_mils: r,
                secondary_radius_mils: sr,
                start_angle: p.start_angle,
                end_angle: p.end_angle,
                color,
                line_width,
                owner_part_id,
                owner_part_display_mode: None,
            }));
        }
        GraphicType::Ellipse => {
            let c = p.center.unwrap_or(CoordPoint::new(Coord::from_mils(0), Coord::from_mils(0)));
            let r = p.radius.map(|r| r.to_mils() as i32).unwrap_or(0);
            let sr = p.secondary_radius.map(|r| r.to_mils() as i32).unwrap_or(r);
            ops.push(SchLibLowOp::AddEllipse(AddEllipseOp {
                opid,
                component_ref: None,
                cx_mils: c.x.to_mils() as i32,
                cy_mils: c.y.to_mils() as i32,
                radius_mils: r,
                secondary_radius_mils: sr,
                color,
                area_color,
                is_solid: p.is_solid,
                line_width,
                owner_part_id,
                owner_part_display_mode: None,
            }));
        }
        GraphicType::Polyline => {
            let points = p.points.as_ref().map(|pts| {
                pts.iter().map(|pt| (pt.x.to_mils() as i32, pt.y.to_mils() as i32)).collect()
            }).unwrap_or_default();
            ops.push(SchLibLowOp::AddPolyline(AddPolylineOp {
                opid,
                component_ref: None,
                points_mils: points,
                color,
                line_width,
                line_style: None,
                owner_part_id,
                owner_part_display_mode: None,
            }));
        }
        GraphicType::Polygon => {
            let points = p.points.as_ref().map(|pts| {
                pts.iter().map(|pt| (pt.x.to_mils() as i32, pt.y.to_mils() as i32)).collect()
            }).unwrap_or_default();
            ops.push(SchLibLowOp::AddPolygon(AddPolygonOp {
                opid,
                component_ref: None,
                points_mils: points,
                color,
                area_color,
                is_solid: p.is_solid,
                line_width,
                owner_part_id,
                owner_part_display_mode: None,
            }));
        }
        GraphicType::Bezier => {
            let points = p.points.as_ref().map(|pts| {
                pts.iter().map(|pt| (pt.x.to_mils() as i32, pt.y.to_mils() as i32)).collect()
            }).unwrap_or_default();
            ops.push(SchLibLowOp::AddBezier(AddBezierOp {
                opid,
                component_ref: None,
                points_mils: points,
                color,
                line_width,
                owner_part_id,
                owner_part_display_mode: None,
            }));
        }
        GraphicType::Pie => {
            let c = p.center.unwrap_or(CoordPoint::new(Coord::from_mils(0), Coord::from_mils(0)));
            let r = p.radius.map(|r| r.to_mils() as i32).unwrap_or(0);
            ops.push(SchLibLowOp::AddPie(AddPieOp {
                opid,
                component_ref: None,
                cx_mils: c.x.to_mils() as i32,
                cy_mils: c.y.to_mils() as i32,
                radius_mils: r,
                start_angle: p.start_angle,
                end_angle: p.end_angle,
                color,
                area_color,
                is_solid: p.is_solid,
                line_width,
                owner_part_id,
                owner_part_display_mode: None,
            }));
        }
        GraphicType::RoundRectangle => {
            let from = p.from.unwrap_or(CoordPoint::new(Coord::from_mils(0), Coord::from_mils(0)));
            let to = p.to.unwrap_or(CoordPoint::new(Coord::from_mils(0), Coord::from_mils(0)));
            let cx_r = p.corner_x_radius.map(|r| r.to_mils() as i32).unwrap_or(0);
            let cy_r = p.corner_y_radius.map(|r| r.to_mils() as i32).unwrap_or(cx_r);
            ops.push(SchLibLowOp::AddRoundRectangle(AddRoundRectangleOp {
                opid,
                component_ref: None,
                from,
                to,
                corner_x_radius_mils: cx_r,
                corner_y_radius_mils: cy_r,
                color,
                area_color,
                is_solid: p.is_solid,
                line_width,
                owner_part_id,
                owner_part_display_mode: None,
            }));
        }
        GraphicType::Label => {
            let at = p.at.unwrap_or(CoordPoint::new(Coord::from_mils(0), Coord::from_mils(0)));
            let text = p.text.clone().unwrap_or_default();
            ops.push(SchLibLowOp::AddLabel(AddLabelOp {
                opid,
                component_ref: None,
                x_mils: at.x.to_mils() as i32,
                y_mils: at.y.to_mils() as i32,
                text,
                color,
                font_id: p.font_id,
                orientation: None,
                justification: None,
                is_mirrored: None,
                owner_part_id,
                owner_part_display_mode: None,
            }));
        }
        GraphicType::TextFrame => {
            let from = p.from.unwrap_or(CoordPoint::new(Coord::from_mils(0), Coord::from_mils(0)));
            let to = p.to.unwrap_or(CoordPoint::new(Coord::from_mils(0), Coord::from_mils(0)));
            let text = p.text.clone().unwrap_or_default();
            ops.push(SchLibLowOp::AddTextFrame(AddTextFrameOp {
                opid,
                component_ref: None,
                from,
                to,
                text,
                color,
                area_color,
                font_id: p.font_id,
                alignment: None,
                word_wrap: None,
                show_border: None,
                is_solid: p.is_solid,
                clip_to_rect: None,
                owner_part_id,
                owner_part_display_mode: None,
            }));
        }
        GraphicType::Image => {
            let from = p.from.unwrap_or(CoordPoint::new(Coord::from_mils(0), Coord::from_mils(0)));
            let to = p.to.unwrap_or(CoordPoint::new(Coord::from_mils(0), Coord::from_mils(0)));
            let file_name = p.file_name.clone().unwrap_or_default();
            ops.push(SchLibLowOp::AddImage(AddImageOp {
                opid,
                component_ref: None,
                from,
                to,
                file_name,
                image_data: p.image_data.clone(),
                keep_aspect: None,
                owner_part_id,
                owner_part_display_mode: None,
            }));
        }
    }
}

fn emit_footprint_map_ops(
    comp_opid: &str,
    fp: &crate::model::FootprintMapSpec,
    ops: &mut Vec<SchLibLowOp>,
) {
    let fp_opid = format!("{comp_opid}/footprint:{}", fp.model_name);
    ops.push(SchLibLowOp::CreateImplementationList(ComponentRefOp {
        opid: format!("{fp_opid}/create_implementation_list"),
        component_ref: None,
    }));
    ops.push(SchLibLowOp::CreateImplementation(ImplementationOp {
        opid: format!("{fp_opid}/create_implementation"),
        component_ref: None,
        model_name: fp.model_name.clone(),
        model_type: None,
        is_current: None,
    }));
    ops.push(SchLibLowOp::CreateImplementationMap(ComponentRefOp {
        opid: format!("{fp_opid}/create_implementation_map"),
        component_ref: None,
    }));
    for (i, map) in fp.maps.iter().enumerate() {
        ops.push(SchLibLowOp::CreateMapDefiner(MapDefinerOp {
            opid: format!("{fp_opid}/create_map_definer[{i}]"),
            component_ref: None,
            pin_designator: map.pin.clone(),
            pad_designator: map.pad.clone(),
        }));
    }
    ops.push(SchLibLowOp::CreateParameterList(ComponentRefOp {
        opid: format!("{fp_opid}/create_parameter_list"),
        component_ref: None,
    }));
}

// ── PcbLib direct pipeline ────────────────────────────────────────────────────

fn build_pcblib_low_ops(spec: &PcbLibSpec) -> Vec<PcbLibLowOp> {
    let mut ops = Vec::new();
    for fp_spec in &spec.footprints {
        let fp_opid = format!("spec:footprint:{}", fp_spec.display_name);
        ops.push(PcbLibLowOp::AddFootprint(AddFootprintOp {
            opid: fp_opid.clone(),
            id: None,
            name: fp_spec.display_name.clone(),
            pattern: fp_spec.pattern.clone(),
            description: fp_spec.description.clone(),
        }));
        for pad in &fp_spec.pads {
            let pad_opid = format!("{fp_opid}/pad:{}", pad.pad_name);
            ops.push(PcbLibLowOp::AddPad(AddPadOp {
                opid: pad_opid,
                footprint_ref: Some(RefExpr::op(fp_opid.clone())),
                pad_name: pad.pad_name.clone(),
                at: pad.at,
                shape: pad.shape,
                x_size: pad.x_size,
                y_size: pad.y_size,
                hole_size: pad.hole_size,
                is_plated: pad.is_plated,
                layer: pad.layer.map(|l| format!("{l:?}")),
                rotation: pad.rotation,
            }));
        }
    }
    ops
}
