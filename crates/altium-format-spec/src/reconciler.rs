//! Reconciler: diff SpecModel against loaded Altium documents to produce an ECO.
//!
//! Compares the desired state (SpecModel) against the current document state
//! and emits Add, Update, or Unchanged entries for each entity.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;

use altium_format::api;
use altium_format::SchLib;

use crate::eco::{
    EngineeringChangeOrder, EntityChange, EntityKind, PropChange, PropValue, compute_summary,
};
use crate::eval::{SpecError, SpecErrorCode};
use crate::model::{
    ComponentSpec, FootprintMapSpec, FootprintSpec, GraphicSpec, PadSpec, PinSpec,
    SchLibSpec, PcbLibSpec,
};

// ── Public API ────────────────────────────────────────────────────────────────

/// Reconcile a spec model against an existing SchLib document.
///
/// Compares each component in the spec against the document's existing
/// components and produces an ECO describing what would change.
/// This is a read-only operation: the document is not modified.
pub fn reconcile_schlib(
    spec: &SchLibSpec,
    doc: &SchLib,
    library_path: PathBuf,
    spec_path: PathBuf,
) -> Result<EngineeringChangeOrder, SpecError> {
    let existing_components = doc.components()
        .map_err(|e| SpecError::no_span(SpecErrorCode::AltiumFormat, e.to_string()))?;

    // Build lookup by lib_reference
    let existing_map: HashMap<&str, &api::Component> = existing_components.iter()
        .map(|c| (c.lib_reference.as_str(), c))
        .collect();

    let mut changes = Vec::new();
    for comp_spec in &spec.components {
        match existing_map.get(comp_spec.lib_reference.as_str()) {
            Some(existing) => {
                changes.push(diff_component(comp_spec, existing));
            }
            None => {
                changes.push(component_to_add(comp_spec));
            }
        }
    }

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

// ── Component-level diff ──────────────────────────────────────────────────────

/// Diff a spec component against an existing API component.
fn diff_component(spec: &ComponentSpec, existing: &api::Component) -> EntityChange {
    let mut prop_changes = Vec::new();
    let mut children = Vec::new();

    // Top-level field diffs
    diff_opt_field("designator", &spec.designator, &existing.designator, &mut prop_changes);
    diff_opt_field("description", &spec.description, &existing.description, &mut prop_changes);
    if let Some(pc) = spec.part_count {
        if pc != existing.part_count {
            prop_changes.push(PropChange {
                field: "part_count".to_string(),
                old_value: existing.part_count.to_string(),
                new_value: pc.to_string(),
            });
        }
    }
    if let Some(shp) = spec.show_hidden_pins {
        if shp != existing.show_hidden_pins {
            prop_changes.push(PropChange {
                field: "show_hidden_pins".to_string(),
                old_value: existing.show_hidden_pins.to_string(),
                new_value: shp.to_string(),
            });
        }
    }

    // Child diffs
    diff_pins(&spec.all_pins(), &existing.pins, &mut children);
    diff_params(&spec.parameters, &existing.parameters, &mut children);
    diff_footprints(&spec.footprints, &existing.footprints, &mut children);
    diff_graphics(&spec.all_graphics(), &existing.graphics, &mut children);
    diff_aliases(&spec.aliases, &existing.aliases, &mut children);

    if prop_changes.is_empty() && children.iter().all(|c| matches!(c, EntityChange::Unchanged { .. })) {
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

// ── Child diff helpers ────────────────────────────────────────────────────────

fn diff_pins(spec_pins: &[&PinSpec], existing: &[api::Pin], out: &mut Vec<EntityChange>) {
    for spec_pin in spec_pins {
        match existing.iter().find(|p| p.designator == spec_pin.designator) {
            Some(existing_pin) => {
                let mut prop_changes = Vec::new();
                diff_opt_field_vs_str("name", &spec_pin.name, &existing_pin.name, &mut prop_changes);
                if let Some(elec) = spec_pin.electrical {
                    if elec != existing_pin.electrical {
                        prop_changes.push(PropChange {
                            field: "electrical".to_string(),
                            old_value: format!("{:?}", existing_pin.electrical),
                            new_value: format!("{elec:?}"),
                        });
                    }
                }
                if let Some(len) = spec_pin.length {
                    if len != existing_pin.length {
                        prop_changes.push(PropChange {
                            field: "length".to_string(),
                            old_value: format!("{}mil", existing_pin.length.to_mils()),
                            new_value: format!("{}mil", len.to_mils()),
                        });
                    }
                }
                if let Some(hidden) = spec_pin.is_hidden {
                    if hidden != existing_pin.is_hidden {
                        prop_changes.push(PropChange {
                            field: "is_hidden".to_string(),
                            old_value: existing_pin.is_hidden.to_string(),
                            new_value: hidden.to_string(),
                        });
                    }
                }
                if spec_pin.location != existing_pin.location {
                    prop_changes.push(PropChange {
                        field: "location".to_string(),
                        old_value: format!("{},{}", existing_pin.location.x.to_mils(), existing_pin.location.y.to_mils()),
                        new_value: format!("{},{}", spec_pin.location.x.to_mils(), spec_pin.location.y.to_mils()),
                    });
                }
                if spec_pin.orientation != existing_pin.orientation {
                    prop_changes.push(PropChange {
                        field: "orientation".to_string(),
                        old_value: format!("{:?}", existing_pin.orientation),
                        new_value: format!("{:?}", spec_pin.orientation),
                    });
                }

                if prop_changes.is_empty() {
                    out.push(EntityChange::Unchanged {
                        kind: EntityKind::Pin,
                        identity: spec_pin.designator.clone(),
                    });
                } else {
                    out.push(EntityChange::Update {
                        kind: EntityKind::Pin,
                        identity: spec_pin.designator.clone(),
                        prop_changes,
                        children: vec![],
                    });
                }
            }
            None => {
                out.push(pin_to_add(spec_pin));
            }
        }
    }
}

fn diff_params(spec_params: &[crate::model::ParameterSpec], existing: &[api::Parameter], out: &mut Vec<EntityChange>) {
    for spec_param in spec_params {
        match existing.iter().find(|p| p.name == spec_param.name) {
            Some(existing_param) => {
                let mut prop_changes = Vec::new();
                if spec_param.text != existing_param.text {
                    prop_changes.push(PropChange {
                        field: "text".to_string(),
                        old_value: existing_param.text.clone(),
                        new_value: spec_param.text.clone(),
                    });
                }
                if let Some(hidden) = spec_param.is_hidden {
                    if hidden != existing_param.is_hidden {
                        prop_changes.push(PropChange {
                            field: "is_hidden".to_string(),
                            old_value: existing_param.is_hidden.to_string(),
                            new_value: hidden.to_string(),
                        });
                    }
                }

                if prop_changes.is_empty() {
                    out.push(EntityChange::Unchanged {
                        kind: EntityKind::Parameter,
                        identity: spec_param.name.clone(),
                    });
                } else {
                    out.push(EntityChange::Update {
                        kind: EntityKind::Parameter,
                        identity: spec_param.name.clone(),
                        prop_changes,
                        children: vec![],
                    });
                }
            }
            None => {
                out.push(EntityChange::Add {
                    kind: EntityKind::Parameter,
                    identity: spec_param.name.clone(),
                    props: vec![PropValue { field: "text".to_string(), value: spec_param.text.clone() }],
                    children: vec![],
                });
            }
        }
    }
}

fn diff_footprints(spec_fps: &[FootprintMapSpec], existing: &[api::FootprintMap], out: &mut Vec<EntityChange>) {
    for spec_fp in spec_fps {
        match existing.iter().find(|f| f.model_name == spec_fp.model_name) {
            Some(existing_fp) => {
                // Compare pin-pad maps
                let spec_maps: Vec<(&str, &str)> = spec_fp.maps.iter()
                    .map(|m| (m.pin.as_str(), m.pad.as_str()))
                    .collect();
                let existing_maps: Vec<(&str, &str)> = existing_fp.pin_pad_maps.iter()
                    .map(|m| (m.pin.as_str(), m.pad.as_str()))
                    .collect();

                if spec_maps == existing_maps {
                    out.push(EntityChange::Unchanged {
                        kind: EntityKind::Footprint,
                        identity: spec_fp.model_name.clone(),
                    });
                } else {
                    out.push(EntityChange::Update {
                        kind: EntityKind::Footprint,
                        identity: spec_fp.model_name.clone(),
                        prop_changes: vec![PropChange {
                            field: "pin_pad_maps".to_string(),
                            old_value: format!("{} maps", existing_maps.len()),
                            new_value: format!("{} maps", spec_maps.len()),
                        }],
                        children: vec![],
                    });
                }
            }
            None => {
                out.push(footprint_to_add(spec_fp));
            }
        }
    }
}

fn diff_graphics(spec_graphics: &[&GraphicSpec], existing: &[api::Graphic], out: &mut Vec<EntityChange>) {
    for spec_graphic in spec_graphics {
        let found = existing.iter().any(|g| {
            g.unique_id().map_or(false, |uid| uid == spec_graphic.unique_id)
        });
        if found {
            // Graphic exists — for now, report as unchanged
            // (full field-by-field diff for 13 graphic types would be very verbose)
            out.push(EntityChange::Unchanged {
                kind: EntityKind::Graphic,
                identity: spec_graphic.unique_id.clone(),
            });
        } else {
            out.push(graphic_to_add(spec_graphic));
        }
    }
}

fn diff_aliases(spec_aliases: &[String], existing: &[String], out: &mut Vec<EntityChange>) {
    for alias in spec_aliases {
        if existing.contains(alias) {
            out.push(EntityChange::Unchanged {
                kind: EntityKind::Alias,
                identity: alias.clone(),
            });
        } else {
            out.push(EntityChange::Add {
                kind: EntityKind::Alias,
                identity: alias.clone(),
                props: vec![],
                children: vec![],
            });
        }
    }
}

// ── Diff helpers ──────────────────────────────────────────────────────────────

/// Diff an optional spec field against an optional existing field.
/// Only produces a PropChange if the spec provides a value AND it differs.
fn diff_opt_field(name: &str, spec_val: &Option<String>, existing: &Option<String>, out: &mut Vec<PropChange>) {
    if let Some(sv) = spec_val {
        let ev = existing.as_deref().unwrap_or("");
        if sv != ev {
            out.push(PropChange {
                field: name.to_string(),
                old_value: ev.to_string(),
                new_value: sv.clone(),
            });
        }
    }
}

/// Diff an optional spec string field against a concrete existing string.
fn diff_opt_field_vs_str(name: &str, spec_val: &Option<String>, existing: &str, out: &mut Vec<PropChange>) {
    if let Some(sv) = spec_val {
        if sv != existing {
            out.push(PropChange {
                field: name.to_string(),
                old_value: existing.to_string(),
                new_value: sv.clone(),
            });
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

// ── ComponentSpec helpers ─────────────────────────────────────────────────────

impl ComponentSpec {
    /// Collect all pins (component-level + part-scoped) as references.
    fn all_pins(&self) -> Vec<&PinSpec> {
        let mut pins: Vec<&PinSpec> = self.pins.iter().collect();
        for part in &self.parts {
            pins.extend(part.pins.iter());
        }
        pins
    }

    /// Collect all graphics (component-level + part-scoped) as references.
    fn all_graphics(&self) -> Vec<&GraphicSpec> {
        let mut graphics: Vec<&GraphicSpec> = self.graphics.iter().collect();
        for part in &self.parts {
            graphics.extend(part.graphics.iter());
        }
        graphics
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        ComponentSpec, FootprintMapSpec, ParameterSpec, PartSpec, PinPadMap, PinSpec, SchLibSpec,
    };
    use crate::executor::apply_spec_schlib;
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

    fn blank_doc() -> SchLib {
        let mut doc = SchLib::new_blank_ad26();
        let _ = doc.remove_component("Component_1");
        doc
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

    // ── Tests: reconcile_schlib (with existing doc) ─────────────────────────

    #[test]
    fn reconcile_existing_unchanged() {
        // Apply a spec, then reconcile with the same spec → all Unchanged
        let spec = make_spec(vec![
            make_component("R_0603", vec![make_pin("1", 0), make_pin("2", 0)]),
        ]);
        let mut doc = blank_doc();
        apply_spec_schlib(&spec, &mut doc).unwrap();

        let eco = reconcile_schlib(
            &spec,
            &doc,
            PathBuf::from("test.SchLib"),
            PathBuf::from("test.schlib-spec"),
        ).unwrap();

        assert_eq!(eco.changes.len(), 1);
        assert!(matches!(&eco.changes[0], EntityChange::Unchanged { kind: EntityKind::Component, identity } if identity == "R_0603"));
    }

    #[test]
    fn reconcile_detects_new_component() {
        let spec1 = make_spec(vec![
            make_component("R_0603", vec![make_pin("1", 0)]),
        ]);
        let mut doc = blank_doc();
        apply_spec_schlib(&spec1, &mut doc).unwrap();

        // Reconcile with a spec that has an additional component
        let spec2 = make_spec(vec![
            make_component("R_0603", vec![make_pin("1", 0)]),
            make_component("C_0805", vec![make_pin("1", 0), make_pin("2", 0)]),
        ]);

        let eco = reconcile_schlib(
            &spec2,
            &doc,
            PathBuf::from("test.SchLib"),
            PathBuf::from("test.schlib-spec"),
        ).unwrap();

        assert_eq!(eco.changes.len(), 2);
        assert!(matches!(&eco.changes[0], EntityChange::Unchanged { .. }));
        assert!(matches!(&eco.changes[1], EntityChange::Add { kind: EntityKind::Component, identity, .. } if identity == "C_0805"));
    }

    #[test]
    fn reconcile_detects_description_change() {
        let spec1 = make_spec(vec![
            make_component("R_0603", vec![make_pin("1", 0)]),
        ]);
        let mut doc = blank_doc();
        apply_spec_schlib(&spec1, &mut doc).unwrap();

        let spec2 = make_spec(vec![ComponentSpec {
            lib_reference: "R_0603".to_string(),
            designator: Some("R?".to_string()),
            description: Some("Updated description".to_string()),
            component_kind: None,
            part_count: None,
            show_hidden_pins: None,
            pins: vec![make_pin("1", 0)],
            parameters: vec![],
            aliases: vec![],
            footprints: vec![],
            graphics: vec![],
            parts: vec![],
        }]);

        let eco = reconcile_schlib(
            &spec2,
            &doc,
            PathBuf::from("test.SchLib"),
            PathBuf::from("test.schlib-spec"),
        ).unwrap();

        assert_eq!(eco.changes.len(), 1);
        if let EntityChange::Update { prop_changes, .. } = &eco.changes[0] {
            let desc_change = prop_changes.iter().find(|pc| pc.field == "description").unwrap();
            assert_eq!(desc_change.old_value, "A resistor");
            assert_eq!(desc_change.new_value, "Updated description");
        } else {
            panic!("expected Update, got {:?}", eco.changes[0]);
        }
    }

    #[test]
    fn reconcile_detects_new_pin() {
        let spec1 = make_spec(vec![
            make_component("R_0603", vec![make_pin("1", 0)]),
        ]);
        let mut doc = blank_doc();
        apply_spec_schlib(&spec1, &mut doc).unwrap();

        // Spec with an additional pin
        let spec2 = make_spec(vec![
            make_component("R_0603", vec![make_pin("1", 0), make_pin("2", 0)]),
        ]);

        let eco = reconcile_schlib(
            &spec2,
            &doc,
            PathBuf::from("test.SchLib"),
            PathBuf::from("test.schlib-spec"),
        ).unwrap();

        assert_eq!(eco.changes.len(), 1);
        if let EntityChange::Update { children, .. } = &eco.changes[0] {
            let pin_unchanged = children.iter()
                .filter(|c| matches!(c, EntityChange::Unchanged { kind: EntityKind::Pin, .. }))
                .count();
            let pin_adds = children.iter()
                .filter(|c| matches!(c, EntityChange::Add { kind: EntityKind::Pin, .. }))
                .count();
            assert_eq!(pin_unchanged, 1); // pin "1"
            assert_eq!(pin_adds, 1); // pin "2"
        } else {
            panic!("expected Update");
        }
    }

    #[test]
    fn reconcile_detects_new_parameter() {
        let spec1 = make_spec(vec![ComponentSpec {
            lib_reference: "R".to_string(),
            designator: Some("R?".to_string()),
            description: None,
            component_kind: None,
            part_count: None,
            show_hidden_pins: None,
            pins: vec![],
            parameters: vec![
                ParameterSpec { name: "MFG".to_string(), text: "ACME".to_string(), is_hidden: None },
            ],
            aliases: vec![],
            footprints: vec![],
            graphics: vec![],
            parts: vec![],
        }]);
        let mut doc = blank_doc();
        apply_spec_schlib(&spec1, &mut doc).unwrap();

        // Spec with an additional parameter
        let spec2 = make_spec(vec![ComponentSpec {
            lib_reference: "R".to_string(),
            designator: Some("R?".to_string()),
            description: None,
            component_kind: None,
            part_count: None,
            show_hidden_pins: None,
            pins: vec![],
            parameters: vec![
                ParameterSpec { name: "MFG".to_string(), text: "ACME".to_string(), is_hidden: None },
                ParameterSpec { name: "VALUE".to_string(), text: "10K".to_string(), is_hidden: None },
            ],
            aliases: vec![],
            footprints: vec![],
            graphics: vec![],
            parts: vec![],
        }]);

        let eco = reconcile_schlib(
            &spec2,
            &doc,
            PathBuf::from("test.SchLib"),
            PathBuf::from("test.schlib-spec"),
        ).unwrap();

        assert_eq!(eco.changes.len(), 1);
        if let EntityChange::Update { children, .. } = &eco.changes[0] {
            let param_unchanged = children.iter()
                .filter(|c| matches!(c, EntityChange::Unchanged { kind: EntityKind::Parameter, .. }))
                .count();
            let param_adds = children.iter()
                .filter(|c| matches!(c, EntityChange::Add { kind: EntityKind::Parameter, .. }))
                .count();
            assert_eq!(param_unchanged, 1); // MFG
            assert_eq!(param_adds, 1); // VALUE
        } else {
            panic!("expected Update");
        }
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

    fn make_footprint(name: &str, pads: Vec<PadSpec>) -> crate::model::FootprintSpec {
        crate::model::FootprintSpec {
            display_name: name.to_string(),
            description: Some("Test footprint".to_string()),
            height: None,
            pattern: None,
            pads,
            graphics: vec![],
        }
    }

    fn make_pcblib_spec(footprints: Vec<crate::model::FootprintSpec>) -> PcbLibSpec {
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
