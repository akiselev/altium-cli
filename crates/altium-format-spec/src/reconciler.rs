//! Reconciler: diff SpecModel against loaded Altium documents to produce an ECO.
//!
//! Compares the desired state (SpecModel) against the current document state
//! and emits Add, Update, or Unchanged entries for each entity.

use std::path::PathBuf;
use std::time::SystemTime;

use altium_format::SchLib;

use crate::eco::{
    EngineeringChangeOrder, EntityChange, EntityKind, PropValue, compute_summary,
};
use crate::eval::{SpecError, SpecErrorCode};
use crate::model::{
    ComponentSpec, FootprintMapSpec, FootprintSpec, GraphicSpec, PadSpec, PinSpec,
    SchLibSpec, PcbLibSpec,
};

// ── Public API ────────────────────────────────────────────────────────────────

/// Reconcile a spec model against an existing SchLib document.
///
/// Currently unimplemented — the LowOps-based document query pipeline has been
/// removed. A high-level API on the document types will replace this.
pub fn reconcile_schlib(
    _spec: &SchLibSpec,
    _doc: &mut SchLib,
    _library_path: PathBuf,
    _spec_path: PathBuf,
) -> Result<EngineeringChangeOrder, SpecError> {
    Err(SpecError::no_span(
        SpecErrorCode::TypeMismatch,
        "reconciler removed; high-level API pending",
    ))
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

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
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
