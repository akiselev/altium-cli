//! Reconciler: diff SpecModel against loaded Altium documents to produce an ECO.
//!
//! Compares the desired state (SpecModel) against the current document state
//! and emits Add, Update, or Unchanged entries for each entity.

use std::path::PathBuf;
use std::time::SystemTime;

use altium_format::sch_ops_core::{
    QueryComponentsOp, QueryPinsOp, QueryRecordsOp, RefExpr, SchLibLowOp, Value as OpsValue,
    apply_schlib_low_ops,
};
use altium_format::SchLib;
use altium_format_types::SchRecordType;
use indexmap::IndexMap;

use crate::spec::eco::{
    EngineeringChangeOrder, EntityChange, EntityKind, PropChange, PropValue, compute_summary,
};
use crate::spec::eval::SpecError;
use crate::spec::model::{
    ComponentSpec, FootprintMapSpec, FootprintSpec, GraphicSpec, PadSpec, ParameterSpec, PinSpec,
    SchLibSpec, PcbLibSpec,
};

// ── Public API ────────────────────────────────────────────────────────────────

/// Reconcile a spec model against an existing SchLib document.
///
/// Produces an ECO describing what changes are needed to bring the document
/// into alignment with the spec. Document-only entities are left Unchanged
/// (additive semantics — never delete).
pub fn reconcile_schlib(
    spec: &SchLibSpec,
    doc: &mut SchLib,
    library_path: PathBuf,
    spec_path: PathBuf,
) -> Result<EngineeringChangeOrder, SpecError> {
    let doc_view = query_doc_view(doc)?;
    let changes = reconcile_schlib_against_view(spec, &doc_view);
    let summary = compute_summary(&changes);
    Ok(EngineeringChangeOrder {
        library_path,
        spec_path,
        timestamp: SystemTime::now(),
        summary,
        changes,
    })
}

/// Reconcile against an empty document: every entity in the spec is an Add.
pub fn reconcile_schlib_empty(
    spec: &SchLibSpec,
    library_path: PathBuf,
    spec_path: PathBuf,
) -> EngineeringChangeOrder {
    let changes: Vec<EntityChange> = spec
        .components
        .iter()
        .map(component_to_add)
        .collect();
    let summary = compute_summary(&changes);
    EngineeringChangeOrder {
        library_path,
        spec_path,
        timestamp: SystemTime::now(),
        summary,
        changes,
    }
}

// ── PcbLib reconcilers ────────────────────────────────────────────────────────

/// Reconcile a spec model against an existing PcbLib document.
///
/// Produces an ECO describing what changes are needed to bring the document
/// into alignment with the spec. This is currently additive-only: footprints
/// not present in the spec are left unchanged.
pub fn reconcile_pcblib(
    spec: &PcbLibSpec,
    library_path: PathBuf,
    spec_path: PathBuf,
) -> EngineeringChangeOrder {
    // For now, treat every footprint as Add (no existing-doc query yet).
    reconcile_pcblib_empty(spec, library_path, spec_path)
}

/// Reconcile against an empty PcbLib document: every entity in the spec is an Add.
pub fn reconcile_pcblib_empty(
    spec: &PcbLibSpec,
    library_path: PathBuf,
    spec_path: PathBuf,
) -> EngineeringChangeOrder {
    let changes: Vec<EntityChange> = spec
        .footprints
        .iter()
        .map(footprint_spec_to_add)
        .collect();
    let summary = compute_summary(&changes);
    EngineeringChangeOrder {
        library_path,
        spec_path,
        timestamp: SystemTime::now(),
        summary,
        changes,
    }
}

fn footprint_spec_to_add(spec: &FootprintSpec) -> EntityChange {
    let mut props = Vec::new();
    if let Some(ref desc) = spec.description {
        props.push(PropValue { field: "description".to_string(), value: desc.clone() });
    }
    if let Some(ref pattern) = spec.pattern {
        props.push(PropValue { field: "pattern".to_string(), value: pattern.clone() });
    }

    let children: Vec<EntityChange> = spec.pads.iter().map(pad_spec_to_add).collect();

    EntityChange::Add {
        kind: EntityKind::Footprint,
        identity: spec.display_name.clone(),
        props,
        children,
    }
}

fn pad_spec_to_add(spec: &PadSpec) -> EntityChange {
    let mut props = Vec::new();
    props.push(PropValue {
        field: "at".to_string(),
        value: format!("{},{}", spec.at.x.to_mils(), spec.at.y.to_mils()),
    });
    if let Some(shape) = spec.shape {
        props.push(PropValue { field: "shape".to_string(), value: format!("{shape:?}") });
    }
    if let Some(x_size) = spec.x_size {
        props.push(PropValue { field: "x_size_mils".to_string(), value: x_size.to_mils().to_string() });
    }
    if let Some(y_size) = spec.y_size {
        props.push(PropValue { field: "y_size_mils".to_string(), value: y_size.to_mils().to_string() });
    }
    if let Some(hole_size) = spec.hole_size {
        props.push(PropValue { field: "hole_size_mils".to_string(), value: hole_size.to_mils().to_string() });
    }
    if let Some(is_plated) = spec.is_plated {
        props.push(PropValue { field: "is_plated".to_string(), value: is_plated.to_string() });
    }
    if let Some(layer) = spec.layer {
        props.push(PropValue { field: "layer".to_string(), value: format!("{layer:?}") });
    }
    if let Some(rotation) = spec.rotation {
        props.push(PropValue { field: "rotation".to_string(), value: rotation.to_string() });
    }

    EntityChange::Add {
        kind: EntityKind::Pad,
        identity: spec.pad_name.clone(),
        props,
        children: vec![],
    }
}

// ── Document view (queried via low ops) ──────────────────────────────────────

/// A lightweight view of the current SchLib state, built by querying with low ops.
struct DocView {
    /// Map from lowercase lib_reference → component view.
    components: IndexMap<String, DocComponent>,
}

struct DocComponent {
    lib_reference: String,
    description: String,
    part_count: i32,
    aliases: Vec<String>,
    /// Map from (owner_part_id, lowercase designator) → pin view.
    pins: IndexMap<(i32, String), DocPin>,
    /// Map from lowercase name → parameter view.
    parameters: IndexMap<String, DocParameter>,
}

struct DocPin {
    designator: String,
    name: String,
    electrical: String,
    owner_part_id: i32,
    is_hidden: bool,
    x_mils: i64,
    y_mils: i64,
    length_mils: i64,
    orientation: i64,
}

struct DocParameter {
    name: String,
    text: String,
    is_hidden: bool,
}

fn query_doc_view(doc: &mut SchLib) -> Result<DocView, SpecError> {
    // Step 1: Query all components
    let qc_op = SchLibLowOp::QueryComponents(QueryComponentsOp {
        opid: "qc".to_string(),
        pattern: None,
    });
    let results = apply_schlib_low_ops(doc, &[qc_op]).map_err(|e| {
        SpecError::no_span(
            crate::spec::eval::SpecErrorCode::TypeMismatch,
            format!("failed to query components: {e}"),
        )
    })?;

    let comp_list = results
        .into_iter()
        .find(|r| r.opid == "qc")
        .and_then(|r| {
            if let Some(OpsValue::List(list)) = r.fields.get("components") {
                Some(list.clone())
            } else {
                None
            }
        })
        .unwrap_or_default();

    let mut components: IndexMap<String, DocComponent> = IndexMap::new();

    for comp_val in &comp_list {
        let map = match comp_val {
            OpsValue::Map(m) => m,
            _ => continue,
        };
        let lib_ref = extract_string(map, "lib_reference").unwrap_or_default();
        let description = extract_string(map, "description").unwrap_or_default();
        let part_count = extract_i64(map, "part_count").unwrap_or(1) as i32;
        let aliases = extract_string_list(map, "aliases");

        // Query pins and parameters for this component by lib ref pattern.
        let qc2_op = SchLibLowOp::QueryComponents(QueryComponentsOp {
            opid: "qc2".to_string(),
            pattern: Some(lib_ref.clone()),
        });
        let qp_op2 = SchLibLowOp::QueryPins(QueryPinsOp {
            opid: "qp".to_string(),
            component_ref: RefExpr::op("qc2"),
        });
        let qr_op2 = SchLibLowOp::QueryRecords(QueryRecordsOp {
            opid: "qr".to_string(),
            component_ref: RefExpr::op("qc2"),
            record_type: Some(SchRecordType::Parameter as i32),
        });

        let sub_results = apply_schlib_low_ops(doc, &[qc2_op, qp_op2, qr_op2]).map_err(|e| {
            SpecError::no_span(
                crate::spec::eval::SpecErrorCode::TypeMismatch,
                format!("failed to query component '{}': {e}", lib_ref),
            )
        })?;

        let pin_list = sub_results
            .iter()
            .find(|r| r.opid == "qp")
            .and_then(|r| {
                if let Some(OpsValue::List(list)) = r.fields.get("pins") {
                    Some(list.clone())
                } else {
                    None
                }
            })
            .unwrap_or_default();

        let record_list = sub_results
            .iter()
            .find(|r| r.opid == "qr")
            .and_then(|r| {
                if let Some(OpsValue::List(list)) = r.fields.get("records") {
                    Some(list.clone())
                } else {
                    None
                }
            })
            .unwrap_or_default();

        let mut pins: IndexMap<(i32, String), DocPin> = IndexMap::new();
        for pin_val in &pin_list {
            if let OpsValue::Map(m) = pin_val {
                let designator = extract_string(m, "designator").unwrap_or_default();
                let name = extract_string(m, "name").unwrap_or_default();
                let electrical = extract_string(m, "electrical").unwrap_or_default();
                let owner_part_id = extract_i64(m, "owner_part_id").unwrap_or(0) as i32;
                let x_mils = extract_i64(m, "x").unwrap_or(0);
                let y_mils = extract_i64(m, "y").unwrap_or(0);
                let length_mils = extract_i64(m, "length").unwrap_or(0);
                let orientation = extract_i64(m, "orientation").unwrap_or(0);
                let is_hidden = extract_bool(m, "is_hidden").unwrap_or(false);
                let key = (owner_part_id, designator.to_lowercase());
                pins.insert(
                    key,
                    DocPin {
                        designator,
                        name,
                        electrical,
                        owner_part_id,
                        is_hidden,
                        x_mils,
                        y_mils,
                        length_mils,
                        orientation,
                    },
                );
            }
        }

        // Parse parameter records from summary strings "Parameter name=value"
        let mut parameters: IndexMap<String, DocParameter> = IndexMap::new();
        for rec_val in &record_list {
            if let OpsValue::Map(m) = rec_val {
                let summary = extract_string(m, "summary").unwrap_or_default();
                if let Some(param) = parse_parameter_summary(&summary) {
                    parameters.insert(param.name.to_lowercase(), param);
                }
            }
        }

        components.insert(
            lib_ref.to_lowercase(),
            DocComponent {
                lib_reference: lib_ref,
                description,
                part_count,
                aliases,
                pins,
                parameters,
            },
        );
    }

    Ok(DocView { components })
}

/// Parse "Parameter name=value" summary strings from QueryRecords.
fn parse_parameter_summary(summary: &str) -> Option<DocParameter> {
    let rest = summary.strip_prefix("Parameter ")?;
    let eq_pos = rest.find('=')?;
    let name = rest[..eq_pos].to_string();
    let text = rest[eq_pos + 1..].to_string();
    Some(DocParameter {
        name,
        text,
        is_hidden: false,
    })
}

// ── Helper extractors for OpsValue::Map ──────────────────────────────────────

fn extract_string(map: &IndexMap<String, OpsValue>, key: &str) -> Option<String> {
    match map.get(key) {
        Some(OpsValue::String(s)) => Some(s.clone()),
        _ => None,
    }
}

fn extract_i64(map: &IndexMap<String, OpsValue>, key: &str) -> Option<i64> {
    match map.get(key) {
        Some(OpsValue::I64(n)) => Some(*n),
        _ => None,
    }
}

fn extract_bool(map: &IndexMap<String, OpsValue>, key: &str) -> Option<bool> {
    match map.get(key) {
        Some(OpsValue::Bool(b)) => Some(*b),
        _ => None,
    }
}

fn extract_string_list(map: &IndexMap<String, OpsValue>, key: &str) -> Vec<String> {
    match map.get(key) {
        Some(OpsValue::List(list)) => list
            .iter()
            .filter_map(|v| {
                if let OpsValue::String(s) = v {
                    Some(s.clone())
                } else {
                    None
                }
            })
            .collect(),
        _ => vec![],
    }
}

// ── Reconciliation logic ──────────────────────────────────────────────────────

fn reconcile_schlib_against_view(
    spec: &SchLibSpec,
    doc: &DocView,
) -> Vec<EntityChange> {
    spec.components
        .iter()
        .map(|comp_spec| {
            let doc_comp = doc.components.get(&comp_spec.lib_reference.to_lowercase());
            reconcile_component(comp_spec, doc_comp)
        })
        .collect()
}

fn reconcile_component(
    spec: &ComponentSpec,
    doc: Option<&DocComponent>,
) -> EntityChange {
    match doc {
        None => component_to_add(spec),
        Some(doc_comp) => {
            let mut prop_changes = Vec::new();

            // Compare description
            if let Some(ref desc) = spec.description {
                if !strings_equal(desc, &doc_comp.description) {
                    prop_changes.push(PropChange {
                        field: "description".to_string(),
                        old_value: doc_comp.description.clone(),
                        new_value: desc.clone(),
                    });
                }
            }

            // Compare part_count
            if let Some(part_count) = spec.part_count {
                if part_count != doc_comp.part_count {
                    prop_changes.push(PropChange {
                        field: "part_count".to_string(),
                        old_value: doc_comp.part_count.to_string(),
                        new_value: part_count.to_string(),
                    });
                }
            }

            let children = reconcile_component_children(spec, doc_comp);

            let all_unchanged = prop_changes.is_empty()
                && children
                    .iter()
                    .all(|c| matches!(c, EntityChange::Unchanged { .. }));

            if all_unchanged {
                EntityChange::Unchanged {
                    kind: EntityKind::Component,
                    identity: spec.lib_reference.clone(),
                }
            } else {
                EntityChange::Update {
                    kind: EntityKind::Component,
                    identity: spec.lib_reference.clone(),
                    prop_changes,
                    children,
                }
            }
        }
    }
}

fn reconcile_component_children(
    spec: &ComponentSpec,
    doc: &DocComponent,
) -> Vec<EntityChange> {
    let mut children = Vec::new();

    // Reconcile component-level pins (owner_part_id = 0)
    for pin_spec in &spec.pins {
        let key = (pin_spec.owner_part_id, pin_spec.designator.to_lowercase());
        let doc_pin = doc.pins.get(&key);
        children.push(reconcile_pin(pin_spec, doc_pin));
    }

    // Reconcile part-scoped pins
    for part in &spec.parts {
        for pin_spec in &part.pins {
            let key = (part.part_number, pin_spec.designator.to_lowercase());
            let doc_pin = doc.pins.get(&key);
            children.push(reconcile_pin(pin_spec, doc_pin));
        }
    }

    // Reconcile parameters
    for param_spec in &spec.parameters {
        let doc_param = doc.parameters.get(&param_spec.name.to_lowercase());
        children.push(reconcile_parameter(param_spec, doc_param));
    }

    // Reconcile aliases
    let doc_aliases_lower: Vec<String> =
        doc.aliases.iter().map(|a| a.to_lowercase()).collect();
    for alias_name in &spec.aliases {
        let exists = doc_aliases_lower.contains(&alias_name.to_lowercase());
        if exists {
            children.push(EntityChange::Unchanged {
                kind: EntityKind::Alias,
                identity: alias_name.clone(),
            });
        } else {
            children.push(EntityChange::Add {
                kind: EntityKind::Alias,
                identity: alias_name.clone(),
                props: vec![],
                children: vec![],
            });
        }
    }

    // Reconcile graphics (unique_id-based, case-sensitive)
    for graphic_spec in &spec.graphics {
        // Graphics have no doc-side query result, treat as Add (stable via unique_id)
        children.push(graphic_to_add(graphic_spec));
    }

    // Reconcile footprint maps
    for fp_spec in &spec.footprints {
        children.push(footprint_to_add(fp_spec));
    }

    children
}

fn reconcile_pin(spec: &PinSpec, doc: Option<&DocPin>) -> EntityChange {
    match doc {
        None => pin_to_add(spec),
        Some(doc_pin) => {
            let mut prop_changes = Vec::new();

            if let Some(ref name) = spec.name {
                if !strings_equal(name, &doc_pin.name) {
                    prop_changes.push(PropChange {
                        field: "name".to_string(),
                        old_value: doc_pin.name.clone(),
                        new_value: name.clone(),
                    });
                }
            }

            if let Some(ref elec) = spec.electrical {
                let spec_elec = format!("{elec:?}");
                if !strings_equal_ci(&spec_elec, &doc_pin.electrical) {
                    prop_changes.push(PropChange {
                        field: "electrical".to_string(),
                        old_value: doc_pin.electrical.clone(),
                        new_value: spec_elec,
                    });
                }
            }

            if let Some(is_hidden) = spec.is_hidden {
                if is_hidden != doc_pin.is_hidden {
                    prop_changes.push(PropChange {
                        field: "is_hidden".to_string(),
                        old_value: doc_pin.is_hidden.to_string(),
                        new_value: is_hidden.to_string(),
                    });
                }
            }

            if prop_changes.is_empty() {
                EntityChange::Unchanged {
                    kind: EntityKind::Pin,
                    identity: spec.designator.clone(),
                }
            } else {
                EntityChange::Update {
                    kind: EntityKind::Pin,
                    identity: spec.designator.clone(),
                    prop_changes,
                    children: vec![],
                }
            }
        }
    }
}

fn reconcile_parameter(spec: &ParameterSpec, doc: Option<&DocParameter>) -> EntityChange {
    match doc {
        None => EntityChange::Add {
            kind: EntityKind::Parameter,
            identity: spec.name.clone(),
            props: vec![
                PropValue { field: "text".to_string(), value: spec.text.clone() },
            ],
            children: vec![],
        },
        Some(doc_param) => {
            let mut prop_changes = Vec::new();
            if !strings_equal(&spec.text, &doc_param.text) {
                prop_changes.push(PropChange {
                    field: "text".to_string(),
                    old_value: doc_param.text.clone(),
                    new_value: spec.text.clone(),
                });
            }
            if let Some(is_hidden) = spec.is_hidden {
                if is_hidden != doc_param.is_hidden {
                    prop_changes.push(PropChange {
                        field: "is_hidden".to_string(),
                        old_value: doc_param.is_hidden.to_string(),
                        new_value: is_hidden.to_string(),
                    });
                }
            }
            if prop_changes.is_empty() {
                EntityChange::Unchanged {
                    kind: EntityKind::Parameter,
                    identity: spec.name.clone(),
                }
            } else {
                EntityChange::Update {
                    kind: EntityKind::Parameter,
                    identity: spec.name.clone(),
                    prop_changes,
                    children: vec![],
                }
            }
        }
    }
}

// ── Build full Add entries ────────────────────────────────────────────────────

pub fn component_to_add(spec: &ComponentSpec) -> EntityChange {
    let mut props = Vec::new();
    if let Some(ref d) = spec.designator {
        props.push(PropValue { field: "designator".to_string(), value: d.clone() });
    }
    if let Some(ref d) = spec.description {
        props.push(PropValue { field: "description".to_string(), value: d.clone() });
    }
    if let Some(pc) = spec.part_count {
        props.push(PropValue { field: "part_count".to_string(), value: pc.to_string() });
    }

    let mut children: Vec<EntityChange> = Vec::new();

    // Component-level pins
    for pin in &spec.pins {
        children.push(pin_to_add(pin));
    }
    // Part-scoped pins
    for part in &spec.parts {
        for pin in &part.pins {
            children.push(pin_to_add(pin));
        }
    }
    // Parameters
    for param in &spec.parameters {
        children.push(EntityChange::Add {
            kind: EntityKind::Parameter,
            identity: param.name.clone(),
            props: vec![PropValue { field: "text".to_string(), value: param.text.clone() }],
            children: vec![],
        });
    }
    // Aliases
    for alias in &spec.aliases {
        children.push(EntityChange::Add {
            kind: EntityKind::Alias,
            identity: alias.clone(),
            props: vec![],
            children: vec![],
        });
    }
    // Graphics
    for graphic in &spec.graphics {
        children.push(graphic_to_add(graphic));
    }
    // Footprint maps
    for fp in &spec.footprints {
        children.push(footprint_to_add(fp));
    }

    EntityChange::Add {
        kind: EntityKind::Component,
        identity: spec.lib_reference.clone(),
        props,
        children,
    }
}

fn pin_to_add(spec: &PinSpec) -> EntityChange {
    let mut props = Vec::new();
    if let Some(ref name) = spec.name {
        props.push(PropValue { field: "name".to_string(), value: name.clone() });
    }
    if let Some(ref elec) = spec.electrical {
        props.push(PropValue { field: "electrical".to_string(), value: format!("{elec:?}") });
    }
    if let Some(len) = spec.length {
        props.push(PropValue {
            field: "length".to_string(),
            value: format!("{}mil", len.to_mils()),
        });
    }
    EntityChange::Add {
        kind: EntityKind::Pin,
        identity: spec.designator.clone(),
        props,
        children: vec![],
    }
}

fn graphic_to_add(spec: &GraphicSpec) -> EntityChange {
    EntityChange::Add {
        kind: EntityKind::Graphic,
        identity: spec.unique_id.clone(),
        props: vec![
            PropValue {
                field: "type".to_string(),
                value: format!("{:?}", spec.graphic_type),
            },
        ],
        children: vec![],
    }
}

fn footprint_to_add(spec: &FootprintMapSpec) -> EntityChange {
    EntityChange::Add {
        kind: EntityKind::Footprint,
        identity: spec.model_name.clone(),
        props: vec![
            PropValue {
                field: "maps".to_string(),
                value: format!("{} pin-pad maps", spec.maps.len()),
            },
        ],
        children: vec![],
    }
}

// ── Value normalization ───────────────────────────────────────────────────────

/// Case-sensitive string equality (for most fields).
fn strings_equal(a: &str, b: &str) -> bool {
    a == b
}

/// Case-insensitive comparison (for identity keys and enums).
fn strings_equal_ci(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::model::{
        ComponentSpec, FootprintMapSpec, ParameterSpec, PartSpec, PinPadMap, PinSpec, SchLibSpec,
    };
    use altium_format_types::{CoordPoint, Coord, RotationBy90};

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

    // ── Test: reconcile_schlib_empty → all Add ─────────────────────────────

    #[test]
    fn empty_doc_all_add() {
        let spec = make_spec(vec![
            make_component("R_0603", vec![make_pin("1", 0), make_pin("2", 0)]),
            make_component("C_0805", vec![make_pin("1", 0), make_pin("2", 0)]),
        ]);

        let eco = reconcile_schlib_empty(
            &spec,
            PathBuf::from("test.SchLib"),
            PathBuf::from("test.schlib-spec"),
        );

        assert_eq!(eco.changes.len(), 2);
        for change in &eco.changes {
            assert!(matches!(change, EntityChange::Add { kind: EntityKind::Component, .. }));
        }

        // Check children are Add too
        if let EntityChange::Add { children, .. } = &eco.changes[0] {
            let pin_adds = children
                .iter()
                .filter(|c| matches!(c, EntityChange::Add { kind: EntityKind::Pin, .. }))
                .count();
            assert_eq!(pin_adds, 2);
        } else {
            panic!("expected Add");
        }
    }

    #[test]
    fn empty_doc_summary_counts() {
        let spec = make_spec(vec![
            make_component("R_0603", vec![make_pin("1", 0), make_pin("2", 0)]),
        ]);

        let eco = reconcile_schlib_empty(
            &spec,
            PathBuf::from("test.SchLib"),
            PathBuf::from("test.schlib-spec"),
        );

        let comp_summary = eco.summary.by_kind.get(&EntityKind::Component).unwrap();
        assert_eq!(comp_summary.adds, 1);
        assert_eq!(comp_summary.updates, 0);
        assert_eq!(comp_summary.unchanged, 0);

        let pin_summary = eco.summary.by_kind.get(&EntityKind::Pin).unwrap();
        assert_eq!(pin_summary.adds, 2);
    }

    #[test]
    fn component_add_includes_parameters() {
        let spec = make_spec(vec![ComponentSpec {
            lib_reference: "R".to_string(),
            designator: Some("R?".to_string()),
            description: Some("Resistor".to_string()),
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

        let eco = reconcile_schlib_empty(
            &spec,
            PathBuf::from("test.SchLib"),
            PathBuf::from("test.schlib-spec"),
        );

        assert_eq!(eco.changes.len(), 1);
        if let EntityChange::Add { children, .. } = &eco.changes[0] {
            let param_adds = children
                .iter()
                .filter(|c| matches!(c, EntityChange::Add { kind: EntityKind::Parameter, .. }))
                .count();
            assert_eq!(param_adds, 1);

            let alias_adds = children
                .iter()
                .filter(|c| matches!(c, EntityChange::Add { kind: EntityKind::Alias, .. }))
                .count();
            assert_eq!(alias_adds, 1);

            let fp_adds = children
                .iter()
                .filter(|c| matches!(c, EntityChange::Add { kind: EntityKind::Footprint, .. }))
                .count();
            assert_eq!(fp_adds, 1);
        } else {
            panic!("expected Add");
        }
    }

    #[test]
    fn multi_part_component_pins() {
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

        let eco = reconcile_schlib_empty(
            &spec,
            PathBuf::from("test.SchLib"),
            PathBuf::from("test.schlib-spec"),
        );

        let pin_summary = eco.summary.by_kind.get(&EntityKind::Pin).unwrap();
        assert_eq!(pin_summary.adds, 8); // 2 shared + 3 part1 + 3 part2
    }

    #[test]
    fn reconcile_against_view_unchanged() {
        // Build a mock doc view with one component
        let mut doc_view = DocView {
            components: IndexMap::new(),
        };
        doc_view.components.insert(
            "r_0603".to_string(),
            DocComponent {
                lib_reference: "R_0603".to_string(),
                description: "A resistor".to_string(),
                part_count: 1,
                aliases: vec![],
                pins: {
                    let mut map = IndexMap::new();
                    map.insert(
                        (0, "1".to_string()),
                        DocPin {
                            designator: "1".to_string(),
                            name: "Pin".to_string(),
                            electrical: "Passive".to_string(),
                            owner_part_id: 0,
                            is_hidden: false,
                            x_mils: 0,
                            y_mils: 0,
                            length_mils: 25,
                            orientation: 0,
                        },
                    );
                    map.insert(
                        (0, "2".to_string()),
                        DocPin {
                            designator: "2".to_string(),
                            name: "Pin".to_string(),
                            electrical: "Passive".to_string(),
                            owner_part_id: 0,
                            is_hidden: false,
                            x_mils: 0,
                            y_mils: 0,
                            length_mils: 25,
                            orientation: 0,
                        },
                    );
                    map
                },
                parameters: IndexMap::new(),
            },
        );

        let spec = make_spec(vec![make_component(
            "R_0603",
            vec![make_pin("1", 0), make_pin("2", 0)],
        )]);

        let changes = reconcile_schlib_against_view(&spec, &doc_view);
        assert_eq!(changes.len(), 1);
        assert!(matches!(changes[0], EntityChange::Unchanged { kind: EntityKind::Component, .. }));
    }

    #[test]
    fn reconcile_description_change() {
        let mut doc_view = DocView {
            components: IndexMap::new(),
        };
        doc_view.components.insert(
            "r_0603".to_string(),
            DocComponent {
                lib_reference: "R_0603".to_string(),
                description: "Old description".to_string(),
                part_count: 1,
                aliases: vec![],
                pins: IndexMap::new(),
                parameters: IndexMap::new(),
            },
        );

        let spec = make_spec(vec![make_component("R_0603", vec![])]);
        let changes = reconcile_schlib_against_view(&spec, &doc_view);

        assert!(matches!(
            &changes[0],
            EntityChange::Update {
                kind: EntityKind::Component,
                prop_changes,
                ..
            } if prop_changes.iter().any(|p| p.field == "description")
        ));
    }

    // ── PcbLib tests ───────────────────────────────────────────────────────

    fn make_pad(name: &str, x_mils: i32, y_mils: i32) -> PadSpec {
        use altium_format_types::{Coord, CoordPoint, PadShape};
        PadSpec {
            pad_name: name.to_string(),
            at: CoordPoint {
                x: Coord::from_mils(x_mils),
                y: Coord::from_mils(y_mils),
            },
            shape: Some(PadShape::Round),
            x_size: Some(Coord::from_mils(60)),
            y_size: Some(Coord::from_mils(60)),
            rotation: None,
            hole_size: Some(Coord::from_mils(28)),
            is_plated: Some(true),
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

    fn make_footprint(name: &str, pads: Vec<PadSpec>) -> crate::spec::model::FootprintSpec {
        crate::spec::model::FootprintSpec {
            display_name: name.to_string(),
            description: Some("Test footprint".to_string()),
            height: None,
            pattern: None,
            pads,
            graphics: vec![],
        }
    }

    fn make_pcblib_spec(footprints: Vec<crate::spec::model::FootprintSpec>) -> PcbLibSpec {
        PcbLibSpec { footprints }
    }

    #[test]
    fn pcblib_empty_doc_all_add() {
        let spec = make_pcblib_spec(vec![
            make_footprint("SOT23", vec![make_pad("1", -50, 0), make_pad("2", 50, 0), make_pad("3", 0, 80)]),
            make_footprint("0603", vec![make_pad("1", -70, 0), make_pad("2", 70, 0)]),
        ]);

        let eco = reconcile_pcblib_empty(
            &spec,
            PathBuf::from("test.PcbLib"),
            PathBuf::from("test.pcblib-spec"),
        );

        assert_eq!(eco.changes.len(), 2);
        for change in &eco.changes {
            assert!(matches!(change, EntityChange::Add { kind: EntityKind::Footprint, .. }));
        }
    }

    #[test]
    fn pcblib_empty_doc_pad_children() {
        let spec = make_pcblib_spec(vec![make_footprint(
            "SOT23",
            vec![make_pad("1", -50, 0), make_pad("2", 50, 0), make_pad("3", 0, 80)],
        )]);

        let eco = reconcile_pcblib_empty(
            &spec,
            PathBuf::from("test.PcbLib"),
            PathBuf::from("test.pcblib-spec"),
        );

        assert_eq!(eco.changes.len(), 1);
        if let EntityChange::Add { children, .. } = &eco.changes[0] {
            let pad_adds = children
                .iter()
                .filter(|c| matches!(c, EntityChange::Add { kind: EntityKind::Pad, .. }))
                .count();
            assert_eq!(pad_adds, 3);
        } else {
            panic!("expected Add");
        }
    }

    #[test]
    fn pcblib_empty_summary_counts() {
        let spec = make_pcblib_spec(vec![make_footprint(
            "0805",
            vec![make_pad("1", -70, 0), make_pad("2", 70, 0)],
        )]);

        let eco = reconcile_pcblib_empty(
            &spec,
            PathBuf::from("test.PcbLib"),
            PathBuf::from("test.pcblib-spec"),
        );

        let fp_summary = eco.summary.by_kind.get(&EntityKind::Footprint).unwrap();
        assert_eq!(fp_summary.adds, 1);
        assert_eq!(fp_summary.updates, 0);

        let pad_summary = eco.summary.by_kind.get(&EntityKind::Pad).unwrap();
        assert_eq!(pad_summary.adds, 2);
    }

    #[test]
    fn pcblib_pad_props_encoded() {
        let spec = make_pcblib_spec(vec![make_footprint(
            "DO35",
            vec![make_pad("K", -100, 0)],
        )]);

        let eco = reconcile_pcblib_empty(
            &spec,
            PathBuf::from("test.PcbLib"),
            PathBuf::from("test.pcblib-spec"),
        );

        if let EntityChange::Add { children, .. } = &eco.changes[0] {
            if let EntityChange::Add { kind: EntityKind::Pad, identity, props, .. } = &children[0] {
                assert_eq!(identity, "K");
                // at should be encoded
                let at_prop = props.iter().find(|p| p.field == "at").expect("at prop");
                assert!(at_prop.value.contains("-100"), "at.x should encode -100");
                // shape encoded
                let shape_prop = props.iter().find(|p| p.field == "shape").expect("shape prop");
                assert_eq!(shape_prop.value, "Round");
            } else {
                panic!("expected Pad Add");
            }
        } else {
            panic!("expected Footprint Add");
        }
    }
}
