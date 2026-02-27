//! Executor: converts [`EngineeringChangeOrder`] entries into [`HighOp`] sequences.
//!
//! ## Mapping summary
//!
//! | ECO entry                       | HighOp(s) emitted                                        |
//! |---------------------------------|----------------------------------------------------------|
//! | Add Component                   | AddComponent (pins embedded), + AddParameter, AddAlias, graphic ops, footprint ops |
//! | Add Pin (standalone)            | AddPin                                                   |
//! | Add Parameter                   | AddParameter                                             |
//! | Add Alias                       | AddAlias                                                 |
//! | Add Graphic                     | graphic-specific Add* op                                 |
//! | Update Component                | EditComponent (prop changes) + child ops                 |
//! | Update Pin/Parameter/Alias      | EditRecord (targeted patch) or remove+re-add             |
//! | Unchanged (any kind)            | *(no ops)*                                               |
//! | Add Footprint (PcbLib)          | AddFootprint                                             |
//!
//! ## OpId scheme
//!
//! ```text
//! spec:component:R_0603                    component add/update
//! spec:component:R_0603:pin:1              pin
//! spec:component:R_0603:parameter:Value    parameter
//! spec:component:R_0603:alias:R            alias
//! spec:component:R_0603:graphic:body       graphic (unique_id used as name)
//! spec:footprint:SOT23                     footprint
//! spec:footprint:SOT23:pad:1               pad (not yet implemented, placeholder)
//! ```

use altium_format::sch_ops_core::RefExpr;

use crate::ops::model::{
    AddAliasOp, AddArcHighOp, AddBezierHighOp, AddComponentOp, AddEllipseHighOp,
    AddEllipticalArcHighOp, AddFootprintHighOp, AddImageHighOp, AddLabelHighOp, AddLineHighOp,
    AddPadHighOp, AddParameterOp, AddPieHighOp, AddPinOp, AddPolygonHighOp, AddPolylineHighOp,
    AddRectangleHighOp, AddRoundRectangleHighOp, AddTextFrameHighOp, EditComponentHighOp,
    FootprintMapEntry, FootprintOp, HighOp,
};
use crate::spec::eco::{
    EngineeringChangeOrder, EntityChange, EntityKind, PropChange, PropValue,
};

// ── Public API ────────────────────────────────────────────────────────────────

/// Convert an [`EngineeringChangeOrder`] into a sequence of [`HighOp`]s.
///
/// `Unchanged` entries produce no ops. `Add` and `Update` entries map to
/// the corresponding HighOp variants. The ops are emitted in ECO order.
pub fn eco_to_high_ops(eco: &EngineeringChangeOrder) -> Vec<HighOp> {
    let mut ops = Vec::new();
    for change in &eco.changes {
        emit_change(change, None, &mut ops);
    }
    ops
}

// ── Change dispatch ───────────────────────────────────────────────────────────

/// Emit HighOps for a single EntityChange.
/// `parent_ref` is the component/footprint reference for child entities.
fn emit_change(
    change: &EntityChange,
    parent_ref: Option<&RefExpr>,
    ops: &mut Vec<HighOp>,
) {
    match change {
        EntityChange::Unchanged { .. } => {
            // No ops for unchanged entities.
        }
        EntityChange::Add { kind, identity, props, children } => {
            emit_add(*kind, identity, props, children, parent_ref, ops);
        }
        EntityChange::Update { kind, identity, prop_changes, children } => {
            emit_update(*kind, identity, prop_changes, children, parent_ref, ops);
        }
    }
}

// ── Add dispatch ──────────────────────────────────────────────────────────────

fn emit_add(
    kind: EntityKind,
    identity: &str,
    props: &[PropValue],
    children: &[EntityChange],
    parent_ref: Option<&RefExpr>,
    ops: &mut Vec<HighOp>,
) {
    match kind {
        EntityKind::Component => {
            emit_add_component(identity, props, children, ops);
        }
        EntityKind::Pin => {
            let opid = pin_opid(parent_ref, identity);
            ops.push(HighOp::AddPin(build_add_pin_op(
                Some(opid),
                parent_ref.cloned(),
                identity,
                props,
            )));
        }
        EntityKind::Parameter => {
            let opid = param_opid(parent_ref, identity);
            ops.push(HighOp::AddParameter(AddParameterOp {
                opid: Some(opid),
                component_ref: parent_ref.cloned(),
                name: identity.to_string(),
                text: get_prop(props, "text").unwrap_or_default(),
                is_hidden: get_bool_prop(props, "is_hidden"),
            }));
        }
        EntityKind::Alias => {
            if let Some(comp_ref) = parent_ref {
                let opid = alias_opid(parent_ref, identity);
                ops.push(HighOp::AddAlias(AddAliasOp {
                    opid: Some(opid),
                    component_ref: comp_ref.clone(),
                    alias_name: identity.to_string(),
                }));
            }
        }
        EntityKind::Graphic => {
            let opid = graphic_opid(parent_ref, identity);
            if let Some(op) = build_graphic_add_op(opid, parent_ref.cloned(), identity, props, None) {
                ops.push(op);
            }
        }
        EntityKind::Footprint => {
            emit_add_footprint(identity, props, children, ops);
        }
        // PcbLib-specific kinds emitted as children of Footprint; standalone not supported.
        EntityKind::Pad
        | EntityKind::Track
        | EntityKind::Via
        | EntityKind::Arc
        | EntityKind::Text
        | EntityKind::Fill
        | EntityKind::Region => {
            // Standalone pad/track/via/arc entities are handled when emitting their
            // parent footprint. Standalone occurrences here produce no ops.
        }
    }
}

// ── Add Component ─────────────────────────────────────────────────────────────

fn emit_add_component(
    identity: &str,
    props: &[PropValue],
    children: &[EntityChange],
    ops: &mut Vec<HighOp>,
) {
    let comp_opid = component_opid(identity);
    let comp_ref = RefExpr::op(comp_opid.clone());

    // Collect embedded pins from Add children.
    let mut embedded_pins: Vec<AddPinOp> = Vec::new();
    let mut footprint_op: Option<FootprintOp> = None;

    for child in children {
        if let EntityChange::Add { kind: EntityKind::Pin, identity: pin_id, props: pin_props, .. } = child {
            embedded_pins.push(build_add_pin_op(None, None, pin_id, pin_props));
        }
        if let EntityChange::Add { kind: EntityKind::Footprint, identity: fp_id, children: fp_children, .. } = child {
            // First footprint map becomes the primary FootprintOp.
            if footprint_op.is_none() {
                footprint_op = Some(build_footprint_op(fp_id, fp_children));
            }
        }
    }

    ops.push(HighOp::AddComponent(AddComponentOp {
        opid: Some(comp_opid.clone()),
        id: None,
        component_ref: None,
        lib_reference: identity.to_string(),
        designator: get_prop(props, "designator"),
        value: get_prop(props, "value"),
        pins: embedded_pins,
        footprint: footprint_op,
    }));

    // Emit separate ops for non-pin children (parameters, aliases, graphics).
    for child in children {
        match child {
            EntityChange::Add { kind: EntityKind::Pin, .. } => {
                // Already embedded above.
            }
            EntityChange::Add { kind: EntityKind::Footprint, .. } => {
                // Already embedded as FootprintOp above.
            }
            EntityChange::Add { kind, identity: child_id, props: child_props, children: child_children } => {
                emit_add(*kind, child_id, child_props, child_children, Some(&comp_ref), ops);
            }
            EntityChange::Update { kind, identity: child_id, prop_changes, children: child_children } => {
                emit_update(*kind, child_id, prop_changes, child_children, Some(&comp_ref), ops);
            }
            EntityChange::Unchanged { .. } => {}
        }
    }
}

// ── Add Footprint (PcbLib) ────────────────────────────────────────────────────

fn emit_add_footprint(
    identity: &str,
    props: &[PropValue],
    children: &[EntityChange],
    ops: &mut Vec<HighOp>,
) {
    let opid = footprint_opid(identity);
    let fp_ref = RefExpr::op(opid.clone());
    ops.push(HighOp::AddFootprint(AddFootprintHighOp {
        opid: Some(opid),
        id: None,
        name: identity.to_string(),
        pattern: get_prop(props, "pattern"),
        description: get_prop(props, "description"),
    }));

    // Emit AddPad ops for each child pad.
    for child in children {
        if let EntityChange::Add { kind: EntityKind::Pad, identity: pad_id, props: pad_props, .. } = child {
            let pad_opid = pad_opid(identity, pad_id);
            let at_x = get_i32_prop(pad_props, "at_x").unwrap_or(0);
            let at_y = get_i32_prop(pad_props, "at_y").unwrap_or(0);
            // "at" may be stored as "x,y"
            let at = get_coord_prop(pad_props, "at").unwrap_or((at_x, at_y));
            ops.push(HighOp::AddPad(AddPadHighOp {
                opid: Some(pad_opid),
                footprint_ref: Some(fp_ref.clone()),
                pad_name: pad_id.clone(),
                at,
                shape: get_prop(pad_props, "shape"),
                x_size_mils: get_i32_prop(pad_props, "x_size_mils"),
                y_size_mils: get_i32_prop(pad_props, "y_size_mils"),
                hole_size_mils: get_i32_prop(pad_props, "hole_size_mils"),
                is_plated: get_bool_prop(pad_props, "is_plated"),
                layer: get_prop(pad_props, "layer"),
                rotation: get_f64_prop(pad_props, "rotation"),
            }));
        }
    }
}

// ── Update dispatch ───────────────────────────────────────────────────────────

fn emit_update(
    kind: EntityKind,
    identity: &str,
    prop_changes: &[PropChange],
    children: &[EntityChange],
    parent_ref: Option<&RefExpr>,
    ops: &mut Vec<HighOp>,
) {
    match kind {
        EntityKind::Component => {
            emit_update_component(identity, prop_changes, children, ops);
        }
        EntityKind::Pin | EntityKind::Parameter | EntityKind::Alias | EntityKind::Graphic => {
            // For sub-entity updates, emit EditRecord for supported fields.
            // Since the reconciler may emit these as children of an Update component,
            // the parent_ref carries the component reference.
            if !prop_changes.is_empty() {
                if let Some(comp_ref) = parent_ref {
                    let opid = sub_entity_opid(kind, comp_ref, identity);
                    let selector = build_record_selector_for_kind(kind, identity);
                    let patch = build_record_patch_from_changes(prop_changes);
                    ops.push(HighOp::EditRecord(crate::ops::model::EditRecordHighOp {
                        opid: Some(opid),
                        component_ref: Some(comp_ref.clone()),
                        selector,
                        patch,
                    }));
                }
            }
            // Child updates within sub-entities (rare, but forward through).
            for child in children {
                emit_change(child, parent_ref, ops);
            }
        }
        // PcbLib entities — skip for now.
        EntityKind::Footprint
        | EntityKind::Pad
        | EntityKind::Track
        | EntityKind::Via
        | EntityKind::Arc
        | EntityKind::Text
        | EntityKind::Fill
        | EntityKind::Region => {}
    }
}

// ── Update Component ──────────────────────────────────────────────────────────

fn emit_update_component(
    identity: &str,
    prop_changes: &[PropChange],
    children: &[EntityChange],
    ops: &mut Vec<HighOp>,
) {
    let comp_opid = component_opid(identity);
    let comp_ref = RefExpr::op(comp_opid.clone());

    // EditComponent for component-level property changes.
    if !prop_changes.is_empty() {
        let description = prop_change_new(prop_changes, "description");
        let part_count = prop_change_new(prop_changes, "part_count")
            .and_then(|s| s.parse::<i32>().ok());
        let component_kind = prop_change_new(prop_changes, "component_kind")
            .and_then(|s| s.parse::<i32>().ok());
        let show_hidden_pins = prop_change_new(prop_changes, "show_hidden_pins")
            .and_then(|s| s.parse::<bool>().ok());

        ops.push(HighOp::EditComponent(EditComponentHighOp {
            opid: Some(comp_opid),
            component_ref: comp_ref.clone(),
            description,
            part_count,
            display_mode_count: None,
            component_kind,
            show_hidden_pins,
        }));
    }

    // Process children of the Updated component.
    for child in children {
        emit_change(child, Some(&comp_ref), ops);
    }
}

// ── Graphic op builders ───────────────────────────────────────────────────────

/// Build a graphic HighOp from ECO props.
/// `graphic_name` is the unique_id of the graphic (e.g. "body", "line_0").
/// `graphic_type` encodes the type: the ECO's `type` property, or extracted from the identity.
fn build_graphic_add_op(
    opid: String,
    component_ref: Option<RefExpr>,
    identity: &str,
    props: &[PropValue],
    owner_part_id: Option<i32>,
) -> Option<HighOp> {
    let graphic_type = get_prop(props, "type").unwrap_or_else(|| identity.to_string());
    let color = get_i32_prop(props, "color");
    let area_color = get_i32_prop(props, "area_color");
    let line_width = get_i32_prop(props, "line_width");
    let is_solid = get_bool_prop(props, "is_solid");

    match graphic_type.as_str() {
        "line" => {
            let from = get_coord_prop(props, "from").unwrap_or((0, 0));
            let to = get_coord_prop(props, "to").unwrap_or((0, 0));
            Some(HighOp::AddLine(AddLineHighOp {
                opid: Some(opid),
                component_ref,
                from,
                to,
                color,
                line_width,
                line_style: None,
                owner_part_id,
                owner_part_display_mode: None,
            }))
        }
        "rectangle" => {
            let from = get_coord_prop(props, "from").unwrap_or((0, 0));
            let to = get_coord_prop(props, "to").unwrap_or((0, 0));
            Some(HighOp::AddRectangle(AddRectangleHighOp {
                opid: Some(opid),
                component_ref,
                from,
                to,
                color,
                area_color,
                is_solid,
                transparent: None,
                line_width,
                owner_part_id,
                owner_part_display_mode: None,
            }))
        }
        "arc" => {
            let cx = get_i32_prop(props, "cx").unwrap_or(0);
            let cy = get_i32_prop(props, "cy").unwrap_or(0);
            let radius = get_i32_prop(props, "radius").unwrap_or(0);
            Some(HighOp::AddArc(AddArcHighOp {
                opid: Some(opid),
                component_ref,
                cx_mils: cx,
                cy_mils: cy,
                radius_mils: radius,
                start_angle: get_f64_prop(props, "start_angle"),
                end_angle: get_f64_prop(props, "end_angle"),
                color,
                line_width,
                owner_part_id,
                owner_part_display_mode: None,
            }))
        }
        "elliptical_arc" => {
            let cx = get_i32_prop(props, "cx").unwrap_or(0);
            let cy = get_i32_prop(props, "cy").unwrap_or(0);
            let radius = get_i32_prop(props, "radius").unwrap_or(0);
            let secondary = get_i32_prop(props, "secondary_radius").unwrap_or(radius);
            Some(HighOp::AddEllipticalArc(AddEllipticalArcHighOp {
                opid: Some(opid),
                component_ref,
                cx_mils: cx,
                cy_mils: cy,
                radius_mils: radius,
                secondary_radius_mils: secondary,
                start_angle: get_f64_prop(props, "start_angle"),
                end_angle: get_f64_prop(props, "end_angle"),
                color,
                line_width,
                owner_part_id,
                owner_part_display_mode: None,
            }))
        }
        "ellipse" => {
            let cx = get_i32_prop(props, "cx").unwrap_or(0);
            let cy = get_i32_prop(props, "cy").unwrap_or(0);
            let radius = get_i32_prop(props, "radius").unwrap_or(0);
            let secondary = get_i32_prop(props, "secondary_radius").unwrap_or(radius);
            Some(HighOp::AddEllipse(AddEllipseHighOp {
                opid: Some(opid),
                component_ref,
                cx_mils: cx,
                cy_mils: cy,
                radius_mils: radius,
                secondary_radius_mils: secondary,
                color,
                area_color,
                is_solid,
                line_width,
                owner_part_id,
                owner_part_display_mode: None,
            }))
        }
        "polyline" => {
            let points = get_points_prop(props, "points");
            Some(HighOp::AddPolyline(AddPolylineHighOp {
                opid: Some(opid),
                component_ref,
                points_mils: points,
                color,
                line_width,
                line_style: None,
                owner_part_id,
                owner_part_display_mode: None,
            }))
        }
        "polygon" => {
            let points = get_points_prop(props, "points");
            Some(HighOp::AddPolygon(AddPolygonHighOp {
                opid: Some(opid),
                component_ref,
                points_mils: points,
                color,
                area_color,
                is_solid,
                line_width,
                owner_part_id,
                owner_part_display_mode: None,
            }))
        }
        "bezier" => {
            let points = get_points_prop(props, "points");
            Some(HighOp::AddBezier(AddBezierHighOp {
                opid: Some(opid),
                component_ref,
                points_mils: points,
                color,
                line_width,
                owner_part_id,
                owner_part_display_mode: None,
            }))
        }
        "pie" => {
            let cx = get_i32_prop(props, "cx").unwrap_or(0);
            let cy = get_i32_prop(props, "cy").unwrap_or(0);
            let radius = get_i32_prop(props, "radius").unwrap_or(0);
            Some(HighOp::AddPie(AddPieHighOp {
                opid: Some(opid),
                component_ref,
                cx_mils: cx,
                cy_mils: cy,
                radius_mils: radius,
                start_angle: get_f64_prop(props, "start_angle"),
                end_angle: get_f64_prop(props, "end_angle"),
                color,
                area_color,
                is_solid,
                line_width,
                owner_part_id,
                owner_part_display_mode: None,
            }))
        }
        "round_rectangle" => {
            let from = get_coord_prop(props, "from").unwrap_or((0, 0));
            let to = get_coord_prop(props, "to").unwrap_or((0, 0));
            let cx_r = get_i32_prop(props, "corner_x_radius").unwrap_or(0);
            let cy_r = get_i32_prop(props, "corner_y_radius").unwrap_or(cx_r);
            Some(HighOp::AddRoundRectangle(AddRoundRectangleHighOp {
                opid: Some(opid),
                component_ref,
                from,
                to,
                corner_x_radius_mils: cx_r,
                corner_y_radius_mils: cy_r,
                color,
                area_color,
                is_solid,
                line_width,
                owner_part_id,
                owner_part_display_mode: None,
            }))
        }
        "label" => {
            let at = get_coord_prop(props, "at").unwrap_or((0, 0));
            let text = get_prop(props, "text").unwrap_or_default();
            Some(HighOp::AddLabel(AddLabelHighOp {
                opid: Some(opid),
                component_ref,
                x_mils: at.0,
                y_mils: at.1,
                text,
                color,
                font_id: get_i32_prop(props, "font_id"),
                orientation: None,
                justification: None,
                is_mirrored: None,
                owner_part_id,
                owner_part_display_mode: None,
            }))
        }
        "text_frame" => {
            let from = get_coord_prop(props, "from").unwrap_or((0, 0));
            let to = get_coord_prop(props, "to").unwrap_or((0, 0));
            let text = get_prop(props, "text").unwrap_or_default();
            Some(HighOp::AddTextFrame(AddTextFrameHighOp {
                opid: Some(opid),
                component_ref,
                from,
                to,
                text,
                color,
                area_color,
                font_id: get_i32_prop(props, "font_id"),
                alignment: None,
                word_wrap: None,
                show_border: get_bool_prop(props, "show_border"),
                is_solid,
                clip_to_rect: None,
                owner_part_id,
                owner_part_display_mode: None,
            }))
        }
        "image" => {
            let from = get_coord_prop(props, "from").unwrap_or((0, 0));
            let to = get_coord_prop(props, "to").unwrap_or((0, 0));
            let file_name = get_prop(props, "file_name").unwrap_or_default();
            Some(HighOp::AddImage(AddImageHighOp {
                opid: Some(opid),
                component_ref,
                from,
                to,
                file_name,
                image_data: None,
                keep_aspect: None,
                owner_part_id,
                owner_part_display_mode: None,
            }))
        }
        _ => None,
    }
}

// ── Build helpers ─────────────────────────────────────────────────────────────

fn build_add_pin_op(
    opid: Option<String>,
    component_ref: Option<RefExpr>,
    designator: &str,
    props: &[PropValue],
) -> AddPinOp {
    AddPinOp {
        opid,
        id: None,
        component_ref,
        designator: designator.to_string(),
        name: get_prop(props, "name"),
        electrical: get_prop(props, "electrical"),
        length_mils: get_i32_prop(props, "length_mils").or_else(|| get_i32_prop(props, "length")),
        at: get_coord_prop(props, "at"),
        rotation: get_i32_prop(props, "rotation").or_else(|| get_i32_prop(props, "orientation")),
    }
}

fn build_footprint_op(model_name: &str, children: &[EntityChange]) -> FootprintOp {
    let mut map = Vec::new();
    for child in children {
        // FootprintMap children are encoded as Add entities with pin/pad props.
        if let EntityChange::Add { props, .. } = child {
            let pin = get_prop(props, "pin").unwrap_or_default();
            let pad = get_prop(props, "pad").unwrap_or_default();
            if !pin.is_empty() || !pad.is_empty() {
                map.push(FootprintMapEntry { pin, pad });
            }
        }
    }
    FootprintOp {
        model_name: model_name.to_string(),
        map,
    }
}

fn build_record_selector_for_kind(
    kind: EntityKind,
    identity: &str,
) -> altium_format::sch_ops_core::RecordSelector {
    use altium_format::sch_ops_core::RecordSelector;
    match kind {
        // Pins are selected by designator.
        EntityKind::Pin => RecordSelector::ByDesignator(identity.to_string()),
        // Parameters and aliases are selected by name.
        EntityKind::Parameter | EntityKind::Alias => RecordSelector::ByName(identity.to_string()),
        // Fallback: select by name.
        _ => RecordSelector::ByName(identity.to_string()),
    }
}

fn build_record_patch_from_changes(
    prop_changes: &[PropChange],
) -> altium_format::sch_ops_core::RecordPatch {
    use altium_format::sch_ops_core::RecordPatch;
    let mut patch = RecordPatch::default();
    for pc in prop_changes {
        match pc.field.as_str() {
            "text" => patch.text = Some(pc.new_value.clone()),
            "name" => patch.name = Some(pc.new_value.clone()),
            "designator" => patch.designator = Some(pc.new_value.clone()),
            "is_hidden" => patch.is_hidden = pc.new_value.parse::<bool>().ok(),
            "color" => patch.color = pc.new_value.parse::<i32>().ok(),
            "line_width" => patch.line_width = pc.new_value.parse::<i32>().ok(),
            _ => {} // Unknown fields silently skipped.
        }
    }
    patch
}

// ── OpId helpers ──────────────────────────────────────────────────────────────

fn component_opid(identity: &str) -> String {
    format!("spec:component:{identity}")
}

fn pin_opid(parent_ref: Option<&RefExpr>, pin_id: &str) -> String {
    let comp = comp_identity_from_ref(parent_ref);
    format!("spec:component:{comp}:pin:{pin_id}")
}

fn param_opid(parent_ref: Option<&RefExpr>, param_name: &str) -> String {
    let comp = comp_identity_from_ref(parent_ref);
    format!("spec:component:{comp}:parameter:{param_name}")
}

fn alias_opid(parent_ref: Option<&RefExpr>, alias_name: &str) -> String {
    let comp = comp_identity_from_ref(parent_ref);
    format!("spec:component:{comp}:alias:{alias_name}")
}

fn graphic_opid(parent_ref: Option<&RefExpr>, graphic_id: &str) -> String {
    let comp = comp_identity_from_ref(parent_ref);
    format!("spec:component:{comp}:graphic:{graphic_id}")
}

fn footprint_opid(identity: &str) -> String {
    format!("spec:footprint:{identity}")
}

fn pad_opid(footprint_id: &str, pad_id: &str) -> String {
    format!("spec:footprint:{footprint_id}:pad:{pad_id}")
}

fn sub_entity_opid(kind: EntityKind, comp_ref: &RefExpr, identity: &str) -> String {
    let comp = comp_identity_from_ref(Some(comp_ref));
    let kind_str = match kind {
        EntityKind::Pin => "pin",
        EntityKind::Parameter => "parameter",
        EntityKind::Alias => "alias",
        EntityKind::Graphic => "graphic",
        _ => "entity",
    };
    format!("spec:component:{comp}:{kind_str}:{identity}")
}

/// Extract the identity string from a component RefExpr (OpId root only).
fn comp_identity_from_ref(r: Option<&RefExpr>) -> String {
    use altium_format::sch_ops_core::RefRoot;
    r.and_then(|ref_| {
        if let RefRoot::OpId(id) = &ref_.root {
            // Strip "spec:component:" prefix if present.
            let s = id.as_str();
            if let Some(stripped) = s.strip_prefix("spec:component:") {
                Some(stripped.to_string())
            } else {
                Some(s.to_string())
            }
        } else {
            None
        }
    })
    .unwrap_or_else(|| "unknown".to_string())
}

// ── PropValue accessors ───────────────────────────────────────────────────────

fn get_prop(props: &[PropValue], key: &str) -> Option<String> {
    props.iter().find(|p| p.field == key).map(|p| p.value.clone())
}

fn get_bool_prop(props: &[PropValue], key: &str) -> Option<bool> {
    get_prop(props, key).and_then(|s| s.parse::<bool>().ok())
}

fn get_i32_prop(props: &[PropValue], key: &str) -> Option<i32> {
    get_prop(props, key).and_then(|s| s.parse::<i32>().ok())
}

fn get_f64_prop(props: &[PropValue], key: &str) -> Option<f64> {
    get_prop(props, key).and_then(|s| s.parse::<f64>().ok())
}

/// Parse a coord stored as "x,y" string (mils as i32).
fn get_coord_prop(props: &[PropValue], key: &str) -> Option<(i32, i32)> {
    get_prop(props, key).and_then(|s| {
        let s = s.trim_matches(|c| c == '(' || c == ')');
        let mut parts = s.splitn(2, ',');
        let x = parts.next()?.trim().parse::<i32>().ok()?;
        let y = parts.next()?.trim().parse::<i32>().ok()?;
        Some((x, y))
    })
}

/// Parse a points list stored as "(x1,y1);(x2,y2);..." string.
fn get_points_prop(props: &[PropValue], key: &str) -> Vec<(i32, i32)> {
    get_prop(props, key)
        .map(|s| {
            s.split(';')
                .filter_map(|pt| get_coord_from_str(pt.trim()))
                .collect()
        })
        .unwrap_or_default()
}

fn get_coord_from_str(s: &str) -> Option<(i32, i32)> {
    let s = s.trim_matches(|c| c == '(' || c == ')');
    let mut parts = s.splitn(2, ',');
    let x = parts.next()?.trim().parse::<i32>().ok()?;
    let y = parts.next()?.trim().parse::<i32>().ok()?;
    Some((x, y))
}

/// Extract the new_value for a changed field, if present.
fn prop_change_new(changes: &[PropChange], field: &str) -> Option<String> {
    changes
        .iter()
        .find(|c| c.field == field)
        .map(|c| c.new_value.clone())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::SystemTime;

    use crate::spec::eco::{compute_summary, PropChange, PropValue};

    fn make_eco(changes: Vec<EntityChange>) -> EngineeringChangeOrder {
        let summary = compute_summary(&changes);
        EngineeringChangeOrder {
            library_path: PathBuf::from("test.SchLib"),
            spec_path: PathBuf::from("test.schlib-spec"),
            timestamp: SystemTime::UNIX_EPOCH,
            summary,
            changes,
        }
    }

    // ── Add-only ECO ───────────────────────────────────────────────────────

    #[test]
    fn add_component_produces_add_component_op() {
        let eco = make_eco(vec![EntityChange::Add {
            kind: EntityKind::Component,
            identity: "R_0603".to_string(),
            props: vec![
                PropValue { field: "designator".to_string(), value: "R?".to_string() },
            ],
            children: vec![],
        }]);

        let ops = eco_to_high_ops(&eco);
        assert_eq!(ops.len(), 1);
        match &ops[0] {
            HighOp::AddComponent(op) => {
                assert_eq!(op.lib_reference, "R_0603");
                assert_eq!(op.designator.as_deref(), Some("R?"));
                assert_eq!(op.opid.as_deref(), Some("spec:component:R_0603"));
            }
            _ => panic!("expected AddComponent"),
        }
    }

    #[test]
    fn add_component_with_embedded_pins() {
        let eco = make_eco(vec![EntityChange::Add {
            kind: EntityKind::Component,
            identity: "R_0603".to_string(),
            props: vec![],
            children: vec![
                EntityChange::Add {
                    kind: EntityKind::Pin,
                    identity: "1".to_string(),
                    props: vec![
                        PropValue { field: "name".to_string(), value: "A".to_string() },
                    ],
                    children: vec![],
                },
                EntityChange::Add {
                    kind: EntityKind::Pin,
                    identity: "2".to_string(),
                    props: vec![],
                    children: vec![],
                },
            ],
        }]);

        let ops = eco_to_high_ops(&eco);
        // 1 AddComponent (pins embedded)
        assert_eq!(ops.len(), 1);
        match &ops[0] {
            HighOp::AddComponent(op) => {
                assert_eq!(op.pins.len(), 2);
                assert_eq!(op.pins[0].designator, "1");
                assert_eq!(op.pins[0].name.as_deref(), Some("A"));
                assert_eq!(op.pins[1].designator, "2");
            }
            _ => panic!("expected AddComponent"),
        }
    }

    #[test]
    fn add_component_with_parameter_and_alias() {
        let eco = make_eco(vec![EntityChange::Add {
            kind: EntityKind::Component,
            identity: "R_0603".to_string(),
            props: vec![],
            children: vec![
                EntityChange::Add {
                    kind: EntityKind::Parameter,
                    identity: "Value".to_string(),
                    props: vec![
                        PropValue { field: "text".to_string(), value: "10k".to_string() },
                    ],
                    children: vec![],
                },
                EntityChange::Add {
                    kind: EntityKind::Alias,
                    identity: "R".to_string(),
                    props: vec![],
                    children: vec![],
                },
            ],
        }]);

        let ops = eco_to_high_ops(&eco);
        // 1 AddComponent + 1 AddParameter + 1 AddAlias
        assert_eq!(ops.len(), 3);
        assert!(matches!(&ops[0], HighOp::AddComponent(_)));
        assert!(matches!(&ops[1], HighOp::AddParameter(_)));
        assert!(matches!(&ops[2], HighOp::AddAlias(_)));

        match &ops[1] {
            HighOp::AddParameter(op) => {
                assert_eq!(op.name, "Value");
                assert_eq!(op.text, "10k");
                assert_eq!(op.opid.as_deref(), Some("spec:component:R_0603:parameter:Value"));
            }
            _ => panic!("expected AddParameter"),
        }
        match &ops[2] {
            HighOp::AddAlias(op) => {
                assert_eq!(op.alias_name, "R");
                assert_eq!(op.opid.as_deref(), Some("spec:component:R_0603:alias:R"));
            }
            _ => panic!("expected AddAlias"),
        }
    }

    // ── Unchanged produces no ops ──────────────────────────────────────────

    #[test]
    fn unchanged_produces_no_ops() {
        let eco = make_eco(vec![
            EntityChange::Unchanged {
                kind: EntityKind::Component,
                identity: "R_0603".to_string(),
            },
            EntityChange::Unchanged {
                kind: EntityKind::Component,
                identity: "C_0402".to_string(),
            },
        ]);
        let ops = eco_to_high_ops(&eco);
        assert!(ops.is_empty());
    }

    // ── Update component ───────────────────────────────────────────────────

    #[test]
    fn update_component_produces_edit_component() {
        let eco = make_eco(vec![EntityChange::Update {
            kind: EntityKind::Component,
            identity: "R_0603".to_string(),
            prop_changes: vec![PropChange {
                field: "description".to_string(),
                old_value: "Old desc".to_string(),
                new_value: "New desc".to_string(),
            }],
            children: vec![],
        }]);

        let ops = eco_to_high_ops(&eco);
        assert_eq!(ops.len(), 1);
        match &ops[0] {
            HighOp::EditComponent(op) => {
                assert_eq!(op.description.as_deref(), Some("New desc"));
                assert_eq!(op.opid.as_deref(), Some("spec:component:R_0603"));
            }
            _ => panic!("expected EditComponent, got {:?}", ops[0]),
        }
    }

    // ── Update with Add children ───────────────────────────────────────────

    #[test]
    fn update_with_new_pin_child() {
        let eco = make_eco(vec![EntityChange::Update {
            kind: EntityKind::Component,
            identity: "R_0603".to_string(),
            prop_changes: vec![],
            children: vec![
                EntityChange::Add {
                    kind: EntityKind::Pin,
                    identity: "3".to_string(),
                    props: vec![],
                    children: vec![],
                },
            ],
        }]);

        let ops = eco_to_high_ops(&eco);
        // No EditComponent (no prop_changes), but AddPin for new child
        assert_eq!(ops.len(), 1);
        match &ops[0] {
            HighOp::AddPin(op) => {
                assert_eq!(op.designator, "3");
                assert_eq!(op.opid.as_deref(), Some("spec:component:R_0603:pin:3"));
            }
            _ => panic!("expected AddPin, got {:?}", ops[0]),
        }
    }

    // ── Graphic ops ────────────────────────────────────────────────────────

    #[test]
    fn add_rectangle_graphic() {
        let eco = make_eco(vec![EntityChange::Add {
            kind: EntityKind::Component,
            identity: "R_0603".to_string(),
            props: vec![],
            children: vec![
                EntityChange::Add {
                    kind: EntityKind::Graphic,
                    identity: "body".to_string(),
                    props: vec![
                        PropValue { field: "type".to_string(), value: "rectangle".to_string() },
                        PropValue { field: "from".to_string(), value: "0,0".to_string() },
                        PropValue { field: "to".to_string(), value: "100,50".to_string() },
                    ],
                    children: vec![],
                },
            ],
        }]);

        let ops = eco_to_high_ops(&eco);
        // 1 AddComponent + 1 AddRectangle
        assert_eq!(ops.len(), 2);
        match &ops[1] {
            HighOp::AddRectangle(op) => {
                assert_eq!(op.from, (0, 0));
                assert_eq!(op.to, (100, 50));
                assert_eq!(op.opid.as_deref(), Some("spec:component:R_0603:graphic:body"));
            }
            _ => panic!("expected AddRectangle, got {:?}", ops[1]),
        }
    }

    #[test]
    fn add_line_graphic() {
        let eco = make_eco(vec![EntityChange::Add {
            kind: EntityKind::Component,
            identity: "IC".to_string(),
            props: vec![],
            children: vec![
                EntityChange::Add {
                    kind: EntityKind::Graphic,
                    identity: "line_0".to_string(),
                    props: vec![
                        PropValue { field: "type".to_string(), value: "line".to_string() },
                        PropValue { field: "from".to_string(), value: "0,0".to_string() },
                        PropValue { field: "to".to_string(), value: "50,50".to_string() },
                    ],
                    children: vec![],
                },
            ],
        }]);

        let ops = eco_to_high_ops(&eco);
        assert_eq!(ops.len(), 2);
        assert!(matches!(&ops[1], HighOp::AddLine(_)));
    }

    // ── OpId generation ────────────────────────────────────────────────────

    #[test]
    fn opid_format() {
        assert_eq!(component_opid("R_0603"), "spec:component:R_0603");
        assert_eq!(footprint_opid("SOT23"), "spec:footprint:SOT23");

        let comp_ref = RefExpr::op("spec:component:LM358");
        assert_eq!(pin_opid(Some(&comp_ref), "1"), "spec:component:LM358:pin:1");
        assert_eq!(param_opid(Some(&comp_ref), "Value"), "spec:component:LM358:parameter:Value");
        assert_eq!(alias_opid(Some(&comp_ref), "LM358N"), "spec:component:LM358:alias:LM358N");
        assert_eq!(graphic_opid(Some(&comp_ref), "body"), "spec:component:LM358:graphic:body");
    }

    // ── Mixed ECO ──────────────────────────────────────────────────────────

    #[test]
    fn mixed_eco_ordering() {
        let eco = make_eco(vec![
            EntityChange::Add {
                kind: EntityKind::Component,
                identity: "R".to_string(),
                props: vec![],
                children: vec![],
            },
            EntityChange::Unchanged {
                kind: EntityKind::Component,
                identity: "C".to_string(),
            },
            EntityChange::Update {
                kind: EntityKind::Component,
                identity: "L".to_string(),
                prop_changes: vec![PropChange {
                    field: "description".to_string(),
                    old_value: "old".to_string(),
                    new_value: "new".to_string(),
                }],
                children: vec![],
            },
        ]);

        let ops = eco_to_high_ops(&eco);
        // AddComponent(R) + EditComponent(L)
        assert_eq!(ops.len(), 2);
        assert!(matches!(&ops[0], HighOp::AddComponent(_)));
        assert!(matches!(&ops[1], HighOp::EditComponent(_)));
    }

    // ── Footprint add ──────────────────────────────────────────────────────

    #[test]
    fn add_footprint_produces_add_footprint_op() {
        let eco = make_eco(vec![EntityChange::Add {
            kind: EntityKind::Footprint,
            identity: "SOT23".to_string(),
            props: vec![
                PropValue { field: "description".to_string(), value: "Small transistor".to_string() },
            ],
            children: vec![],
        }]);

        let ops = eco_to_high_ops(&eco);
        assert_eq!(ops.len(), 1);
        match &ops[0] {
            HighOp::AddFootprint(op) => {
                assert_eq!(op.name, "SOT23");
                assert_eq!(op.description.as_deref(), Some("Small transistor"));
                assert_eq!(op.opid.as_deref(), Some("spec:footprint:SOT23"));
            }
            _ => panic!("expected AddFootprint"),
        }
    }

    #[test]
    fn add_footprint_with_pads_emits_add_pad_ops() {
        let eco = make_eco(vec![EntityChange::Add {
            kind: EntityKind::Footprint,
            identity: "SOT23".to_string(),
            props: vec![],
            children: vec![
                EntityChange::Add {
                    kind: EntityKind::Pad,
                    identity: "1".to_string(),
                    props: vec![
                        PropValue { field: "at".to_string(), value: "-50,0".to_string() },
                        PropValue { field: "shape".to_string(), value: "Round".to_string() },
                        PropValue { field: "x_size_mils".to_string(), value: "60".to_string() },
                        PropValue { field: "y_size_mils".to_string(), value: "60".to_string() },
                    ],
                    children: vec![],
                },
                EntityChange::Add {
                    kind: EntityKind::Pad,
                    identity: "2".to_string(),
                    props: vec![
                        PropValue { field: "at".to_string(), value: "50,0".to_string() },
                    ],
                    children: vec![],
                },
                EntityChange::Add {
                    kind: EntityKind::Pad,
                    identity: "3".to_string(),
                    props: vec![
                        PropValue { field: "at".to_string(), value: "0,80".to_string() },
                    ],
                    children: vec![],
                },
            ],
        }]);

        let ops = eco_to_high_ops(&eco);
        // 1 AddFootprint + 3 AddPad
        assert_eq!(ops.len(), 4);
        assert!(matches!(&ops[0], HighOp::AddFootprint(_)));
        assert!(matches!(&ops[1], HighOp::AddPad(_)));
        assert!(matches!(&ops[2], HighOp::AddPad(_)));
        assert!(matches!(&ops[3], HighOp::AddPad(_)));

        match &ops[1] {
            HighOp::AddPad(op) => {
                assert_eq!(op.pad_name, "1");
                assert_eq!(op.at, (-50, 0));
                assert_eq!(op.shape.as_deref(), Some("Round"));
                assert_eq!(op.x_size_mils, Some(60));
                assert_eq!(op.y_size_mils, Some(60));
                assert_eq!(op.opid.as_deref(), Some("spec:footprint:SOT23:pad:1"));
            }
            _ => panic!("expected AddPad"),
        }
    }

    #[test]
    fn add_pad_opid_format() {
        assert_eq!(pad_opid("SOT23", "1"), "spec:footprint:SOT23:pad:1");
        assert_eq!(pad_opid("0603", "A1"), "spec:footprint:0603:pad:A1");
    }
}
