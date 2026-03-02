//! Reconciler: diff SpecModel against loaded Altium documents to produce an ECO.
//!
//! Compares the desired state (SpecModel) against the current document state
//! and emits Add, Update, or Unchanged entries for each entity.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;

use altium_format::api;
use altium_format::{PcbDoc, PcbLib, SchDoc, SchLib};

use crate::eco::{
    EngineeringChangeOrder, EntityChange, EntityKind, PropChange, PropValue, compute_summary,
};
use crate::eval::{SpecError, SpecErrorCode};
use crate::model::{
    BoardSpec, ComponentSpec, FootprintMapSpec, FootprintSpec, GraphicSpec, LayerSpec, PadSpec,
    PcbDocClassSpec, PcbDocComponentSpec, PcbDocDifferentialPairSpec, PcbDocNetSpec,
    PcbDocPolygonSpec, PcbDocPrimitiveSpec, PcbDocRuleSpec, PcbDocSpec, PinSpec, PrjPcbSpec,
    ProjectSpec, SchDocSpec, SchDocObjectSpec, SchLibSpec, PcbLibSpec,
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
/// Tries to open the PcbLib at `library_path`. If successful, compares each
/// footprint in the spec against the document's existing footprints and produces
/// an ECO describing what would change. Falls back to treating every footprint
/// as an Add if the document cannot be opened.
///
/// This is a read-only operation: the document is not modified.
pub fn reconcile_pcblib(
    spec: &PcbLibSpec,
    library_path: PathBuf,
    spec_path: PathBuf,
) -> EngineeringChangeOrder {
    let lib = match PcbLib::open(&library_path) {
        Ok(lib) => lib,
        Err(_) => return reconcile_pcblib_empty(spec, library_path, spec_path),
    };

    let existing_footprints = lib.footprints();

    // Build lookup by display_name
    let existing_map: HashMap<&str, &api::Footprint> = existing_footprints
        .iter()
        .map(|f| (f.display_name.as_str(), f))
        .collect();

    let mut changes = Vec::new();
    for fp_spec in &spec.footprints {
        match existing_map.get(fp_spec.display_name.as_str()) {
            Some(existing) => {
                changes.push(diff_footprint(fp_spec, existing));
            }
            None => {
                changes.push(footprint_spec_to_add(fp_spec));
            }
        }
    }

    let summary = compute_summary(&changes);
    EngineeringChangeOrder {
        library_path,
        spec_path,
        timestamp: SystemTime::now(),
        summary,
        changes,
    }
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

// ── PrjPcb reconcilers ──────────────────────────────────────────────────────

/// Reconcile a spec model against an existing PrjPcb project.
pub fn reconcile_prjpcb(
    spec: &PrjPcbSpec,
    doc: &altium_format::AltiumProject,
    library_path: PathBuf,
    spec_path: PathBuf,
) -> Result<EngineeringChangeOrder, SpecError> {
    let existing = doc.project()
        .map_err(|e| SpecError::no_span(SpecErrorCode::AltiumFormat, e.to_string()))?;

    let mut changes = Vec::new();
    for proj_spec in &spec.projects {
        changes.push(diff_project(proj_spec, &existing));
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

/// Reconcile against an empty project: every entity in the spec is an Add.
pub fn reconcile_prjpcb_empty(
    spec: &PrjPcbSpec,
    library_path: PathBuf,
    spec_path: PathBuf,
) -> EngineeringChangeOrder {
    let changes: Vec<EntityChange> = spec
        .projects
        .iter()
        .map(project_to_add)
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

fn diff_project(spec: &ProjectSpec, existing: &api::Project) -> EntityChange {
    let mut prop_changes = Vec::new();

    // Diff scalar string fields
    diff_opt_field_vs_str("output_path", &spec.output_path, &existing.output_path, &mut prop_changes);
    diff_opt_field_vs_str("channel_designator_format", &spec.channel_designator_format, &existing.channel_designator_format, &mut prop_changes);
    diff_opt_field_vs_str("channel_room_level_separator", &spec.channel_room_level_separator, &existing.channel_room_level_separator, &mut prop_changes);

    // Diff scalar bool fields
    diff_opt_bool("allow_port_net_names", spec.allow_port_net_names, existing.allow_port_net_names, &mut prop_changes);
    diff_opt_bool("allow_sheet_entry_net_names", spec.allow_sheet_entry_net_names, existing.allow_sheet_entry_net_names, &mut prop_changes);
    diff_opt_bool("netlist_single_pin_nets", spec.netlist_single_pin_nets, existing.netlist_single_pin_nets, &mut prop_changes);
    diff_opt_bool("append_sheet_number_to_local_nets", spec.append_sheet_number_to_local_nets, existing.append_sheet_number_to_local_nets, &mut prop_changes);
    diff_opt_bool("name_nets_hierarchically", spec.name_nets_hierarchically, existing.name_nets_hierarchically, &mut prop_changes);
    diff_opt_bool("power_port_names_take_priority", spec.power_port_names_take_priority, existing.power_port_names_take_priority, &mut prop_changes);
    diff_opt_bool("pin_swap_by_netlabel", spec.pin_swap_by_netlabel, existing.pin_swap_by_netlabel, &mut prop_changes);
    diff_opt_bool("pin_swap_by_pin", spec.pin_swap_by_pin, existing.pin_swap_by_pin, &mut prop_changes);
    diff_opt_bool("cross_ref_cross_sheets", spec.cross_ref_cross_sheets, existing.cross_ref_cross_sheets, &mut prop_changes);
    diff_opt_bool("cross_ref_sheet_entries", spec.cross_ref_sheet_entries, existing.cross_ref_sheet_entries, &mut prop_changes);

    // Diff scalar enum fields
    diff_opt_enum("hierarchy_mode", spec.hierarchy_mode, existing.hierarchy_mode, &mut prop_changes);
    diff_opt_enum("channel_room_naming_style", spec.channel_room_naming_style, existing.channel_room_naming_style, &mut prop_changes);
    diff_opt_enum("cross_ref_sheet_style", spec.cross_ref_sheet_style, existing.cross_ref_sheet_style, &mut prop_changes);
    diff_opt_enum("cross_ref_location_style", spec.cross_ref_location_style, existing.cross_ref_location_style, &mut prop_changes);
    diff_opt_enum("cross_ref_ports", spec.cross_ref_ports, existing.cross_ref_ports, &mut prop_changes);

    // Diff children
    let mut children = Vec::new();

    // Documents
    let existing_docs: HashMap<&str, &api::DocumentRef> = existing.documents.iter()
        .map(|d| (d.path.as_str(), d))
        .collect();
    for doc_spec in &spec.documents {
        match existing_docs.get(doc_spec.path.as_str()) {
            Some(_existing_doc) => {
                // Document exists — mark as unchanged for now
                children.push(EntityChange::Unchanged {
                    kind: EntityKind::Document,
                    identity: doc_spec.path.clone(),
                });
            }
            None => {
                children.push(document_to_add(doc_spec));
            }
        }
    }

    // Output groups
    let existing_groups: HashMap<&str, &api::OutputGroup> = existing.output_groups.iter()
        .map(|g| (g.name.as_str(), g))
        .collect();
    for group_spec in &spec.output_groups {
        match existing_groups.get(group_spec.name.as_str()) {
            Some(_existing_group) => {
                children.push(EntityChange::Unchanged {
                    kind: EntityKind::OutputGroup,
                    identity: group_spec.name.clone(),
                });
            }
            None => {
                children.push(output_group_to_add(group_spec));
            }
        }
    }

    // Variants
    let existing_variants: HashMap<&str, &api::ProjectVariant> = existing.variants.iter()
        .map(|v| (v.description.as_str(), v))
        .collect();
    for var_spec in &spec.variants {
        match existing_variants.get(var_spec.name.as_str()) {
            Some(_existing_var) => {
                children.push(EntityChange::Unchanged {
                    kind: EntityKind::Variant,
                    identity: var_spec.name.clone(),
                });
            }
            None => {
                children.push(variant_to_add(var_spec));
            }
        }
    }

    // ERC matrix overrides
    for erc in &spec.erc_matrix_overrides {
        let identity = format!("({}, {})", erc.row, erc.col);
        let existing_level = existing.erc_matrix.cells[erc.row as usize][erc.col as usize];
        if existing_level != erc.level {
            children.push(EntityChange::Update {
                kind: EntityKind::ErcMatrixCell,
                identity,
                prop_changes: vec![PropChange {
                    field: "level".to_string(),
                    old_value: existing_level.to_string(),
                    new_value: erc.level.to_string(),
                }],
                children: vec![],
            });
        } else {
            children.push(EntityChange::Unchanged {
                kind: EntityKind::ErcMatrixCell,
                identity,
            });
        }
    }

    // ERC level overrides
    for erc_level in &spec.erc_level_overrides {
        let existing_el = existing.erc_levels.iter().find(|e| e.key == erc_level.name);
        match existing_el {
            Some(existing) => {
                if existing.level != erc_level.level {
                    children.push(EntityChange::Update {
                        kind: EntityKind::ErcLevel,
                        identity: erc_level.name.clone(),
                        prop_changes: vec![PropChange {
                            field: "level".to_string(),
                            old_value: existing.level.to_string(),
                            new_value: erc_level.level.to_string(),
                        }],
                        children: vec![],
                    });
                } else {
                    children.push(EntityChange::Unchanged {
                        kind: EntityKind::ErcLevel,
                        identity: erc_level.name.clone(),
                    });
                }
            }
            None => {
                children.push(EntityChange::Add {
                    kind: EntityKind::ErcLevel,
                    identity: erc_level.name.clone(),
                    props: vec![PropValue {
                        field: "level".to_string(),
                        value: erc_level.level.to_string(),
                    }],
                    children: vec![],
                });
            }
        }
    }

    if prop_changes.is_empty() && children.iter().all(|c| matches!(c, EntityChange::Unchanged { .. })) {
        EntityChange::Unchanged {
            kind: EntityKind::Project,
            identity: spec.name.clone(),
        }
    } else {
        EntityChange::Update {
            kind: EntityKind::Project,
            identity: spec.name.clone(),
            prop_changes,
            children,
        }
    }
}

fn project_to_add(spec: &ProjectSpec) -> EntityChange {
    let mut props = Vec::new();
    if let Some(ref v) = spec.hierarchy_mode { props.push(PropValue { field: "hierarchy_mode".into(), value: v.to_string() }); }
    if let Some(ref v) = spec.output_path { props.push(PropValue { field: "output_path".into(), value: v.clone() }); }
    if let Some(v) = spec.allow_port_net_names { props.push(PropValue { field: "allow_port_net_names".into(), value: v.to_string() }); }
    if let Some(v) = spec.allow_sheet_entry_net_names { props.push(PropValue { field: "allow_sheet_entry_net_names".into(), value: v.to_string() }); }

    let mut children = Vec::new();
    for doc in &spec.documents { children.push(document_to_add(doc)); }
    for group in &spec.output_groups { children.push(output_group_to_add(group)); }
    for var in &spec.variants { children.push(variant_to_add(var)); }

    EntityChange::Add {
        kind: EntityKind::Project,
        identity: spec.name.clone(),
        props,
        children,
    }
}

fn document_to_add(spec: &crate::model::DocumentSpec) -> EntityChange {
    let mut props = Vec::new();
    if let Some(v) = spec.annotation_enabled { props.push(PropValue { field: "annotation_enabled".into(), value: v.to_string() }); }
    if let Some(v) = spec.annotate_start_value { props.push(PropValue { field: "annotate_start_value".into(), value: v.to_string() }); }
    EntityChange::Add {
        kind: EntityKind::Document,
        identity: spec.path.clone(),
        props,
        children: vec![],
    }
}

fn output_group_to_add(spec: &crate::model::OutputGroupSpec) -> EntityChange {
    let mut props = Vec::new();
    if let Some(ref v) = spec.description { props.push(PropValue { field: "description".into(), value: v.clone() }); }
    let children: Vec<EntityChange> = spec.outputs.iter().map(|out| {
        let mut out_props = Vec::new();
        if let Some(ref v) = out.output_type { out_props.push(PropValue { field: "output_type".into(), value: v.clone() }); }
        if let Some(ref v) = out.document_path { out_props.push(PropValue { field: "document_path".into(), value: v.clone() }); }
        EntityChange::Add {
            kind: EntityKind::OutputJob,
            identity: out.name.clone(),
            props: out_props,
            children: vec![],
        }
    }).collect();
    EntityChange::Add {
        kind: EntityKind::OutputGroup,
        identity: spec.name.clone(),
        props,
        children,
    }
}

fn variant_to_add(spec: &crate::model::VariantSpec) -> EntityChange {
    let mut props = Vec::new();
    if let Some(ref v) = spec.description { props.push(PropValue { field: "description".into(), value: v.clone() }); }
    let mut children = Vec::new();
    for var in &spec.variations {
        let mut var_props = Vec::new();
        if let Some(ref k) = var.kind { var_props.push(PropValue { field: "kind".into(), value: k.to_string() }); }
        if let Some(ref v) = var.alternate_part { var_props.push(PropValue { field: "alternate_part".into(), value: v.clone() }); }
        children.push(EntityChange::Add {
            kind: EntityKind::Variation,
            identity: var.designator.clone(),
            props: var_props,
            children: vec![],
        });
    }
    EntityChange::Add {
        kind: EntityKind::Variant,
        identity: spec.name.clone(),
        props,
        children,
    }
}

fn diff_opt_bool(name: &str, spec_val: Option<bool>, existing: bool, out: &mut Vec<PropChange>) {
    if let Some(sv) = spec_val {
        if sv != existing {
            out.push(PropChange {
                field: name.to_string(),
                old_value: existing.to_string(),
                new_value: sv.to_string(),
            });
        }
    }
}

fn diff_opt_enum<T: PartialEq + std::fmt::Display>(name: &str, spec_val: Option<T>, existing: T, out: &mut Vec<PropChange>) {
    if let Some(sv) = spec_val {
        if sv != existing {
            out.push(PropChange {
                field: name.to_string(),
                old_value: existing.to_string(),
                new_value: sv.to_string(),
            });
        }
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
    if let Some(ref layer) = spec.layer {
        props.push(PropValue { field: "layer".to_string(), value: format_layer_spec(layer) });
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

// ── Footprint-level diff ─────────────────────────────────────────────────────

/// Diff a spec footprint against an existing API footprint.
fn diff_footprint(spec: &FootprintSpec, existing: &api::Footprint) -> EntityChange {
    let mut prop_changes = Vec::new();
    let mut children = Vec::new();

    // Top-level field diffs
    diff_opt_field_vs_str("description", &spec.description, &existing.description, &mut prop_changes);
    diff_opt_field_vs_str("pattern", &spec.pattern, &existing.pattern, &mut prop_changes);
    if let Some(h) = spec.height {
        if h != existing.height {
            prop_changes.push(PropChange {
                field: "height".to_string(),
                old_value: format!("{}mil", existing.height.to_mils()),
                new_value: format!("{}mil", h.to_mils()),
            });
        }
    }

    // Diff pads by pad_name
    diff_pcb_pads(&spec.pads, &existing.pads, &mut children);

    // Diff graphics by unique_id
    diff_pcb_graphics(&spec.graphics, &existing.graphics, &mut children);

    if prop_changes.is_empty() && children.iter().all(|c| matches!(c, EntityChange::Unchanged { .. })) {
        EntityChange::Unchanged {
            kind: EntityKind::Footprint,
            identity: spec.display_name.clone(),
        }
    } else {
        EntityChange::Update {
            kind: EntityKind::Footprint,
            identity: spec.display_name.clone(),
            prop_changes,
            children,
        }
    }
}

fn diff_pcb_pads(spec_pads: &[PadSpec], existing: &[api::Pad], out: &mut Vec<EntityChange>) {
    for spec_pad in spec_pads {
        match existing.iter().find(|p| p.pad_name == spec_pad.pad_name) {
            Some(existing_pad) => {
                let mut prop_changes = Vec::new();

                // Compare location
                if spec_pad.at != existing_pad.location {
                    prop_changes.push(PropChange {
                        field: "location".to_string(),
                        old_value: format!("{},{}", existing_pad.location.x.to_mils(), existing_pad.location.y.to_mils()),
                        new_value: format!("{},{}", spec_pad.at.x.to_mils(), spec_pad.at.y.to_mils()),
                    });
                }
                if let Some(shape) = spec_pad.shape {
                    if shape != existing_pad.shape {
                        prop_changes.push(PropChange {
                            field: "shape".to_string(),
                            old_value: format!("{:?}", existing_pad.shape),
                            new_value: format!("{shape:?}"),
                        });
                    }
                }
                if let Some(x_size) = spec_pad.x_size {
                    if x_size != existing_pad.x_size {
                        prop_changes.push(PropChange {
                            field: "x_size".to_string(),
                            old_value: format!("{}mil", existing_pad.x_size.to_mils()),
                            new_value: format!("{}mil", x_size.to_mils()),
                        });
                    }
                }
                if let Some(y_size) = spec_pad.y_size {
                    if y_size != existing_pad.y_size {
                        prop_changes.push(PropChange {
                            field: "y_size".to_string(),
                            old_value: format!("{}mil", existing_pad.y_size.to_mils()),
                            new_value: format!("{}mil", y_size.to_mils()),
                        });
                    }
                }
                if let Some(hole_size) = spec_pad.hole_size {
                    if hole_size != existing_pad.hole_size {
                        prop_changes.push(PropChange {
                            field: "hole_size".to_string(),
                            old_value: format!("{}mil", existing_pad.hole_size.to_mils()),
                            new_value: format!("{}mil", hole_size.to_mils()),
                        });
                    }
                }
                if let Some(is_plated) = spec_pad.is_plated {
                    if is_plated != existing_pad.is_plated {
                        prop_changes.push(PropChange {
                            field: "is_plated".to_string(),
                            old_value: existing_pad.is_plated.to_string(),
                            new_value: is_plated.to_string(),
                        });
                    }
                }
                if let Some(ref layer) = spec_pad.layer {
                    let resolved = format_layer_spec(layer);
                    let existing_str = format!("{}", existing_pad.layer);
                    if resolved != existing_str {
                        prop_changes.push(PropChange {
                            field: "layer".to_string(),
                            old_value: existing_str,
                            new_value: resolved,
                        });
                    }
                }
                if let Some(rotation) = spec_pad.rotation {
                    if (rotation - existing_pad.rotation).abs() > f64::EPSILON {
                        prop_changes.push(PropChange {
                            field: "rotation".to_string(),
                            old_value: existing_pad.rotation.to_string(),
                            new_value: rotation.to_string(),
                        });
                    }
                }

                if prop_changes.is_empty() {
                    out.push(EntityChange::Unchanged {
                        kind: EntityKind::Pad,
                        identity: spec_pad.pad_name.clone(),
                    });
                } else {
                    out.push(EntityChange::Update {
                        kind: EntityKind::Pad,
                        identity: spec_pad.pad_name.clone(),
                        prop_changes,
                        children: vec![],
                    });
                }
            }
            None => {
                out.push(pad_spec_to_add(spec_pad));
            }
        }
    }
}

fn diff_pcb_graphics(
    spec_graphics: &[crate::model::PcbGraphicSpec],
    existing: &[api::PcbGraphic],
    out: &mut Vec<EntityChange>,
) {
    for spec_graphic in spec_graphics {
        let found = existing.iter().any(|g| {
            g.unique_id().map_or(false, |uid| uid == spec_graphic.unique_id)
        });
        if found {
            // Graphic exists — report as unchanged
            // (full field-by-field diff for PCB graphic types would be very verbose)
            out.push(EntityChange::Unchanged {
                kind: EntityKind::Graphic,
                identity: spec_graphic.unique_id.clone(),
            });
        } else {
            out.push(EntityChange::Add {
                kind: EntityKind::Graphic,
                identity: spec_graphic.unique_id.clone(),
                props: vec![PropValue {
                    field: "type".to_string(),
                    value: format!("{:?}", spec_graphic.graphic_type),
                }],
                children: vec![],
            });
        }
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

/// Format a `LayerSpec` for display in ECO reports.
fn format_layer_spec(spec: &LayerSpec) -> String {
    match spec {
        LayerSpec::Resolved(lr) => format!("{lr}"),
        LayerSpec::CopperPosition(n) => format!("copper({n})"),
        LayerSpec::NamedLayer(name) => name.clone(),
    }
}

// ── SchDoc reconcilers ─────────────────────────────────────────────────────

/// Reconcile a SchDoc spec against an existing SchDoc document.
///
/// Compares components by designator, low-level objects by type + position,
/// and produces an ECO describing what would change.
pub fn reconcile_schdoc(
    spec: &SchDocSpec,
    doc: &SchDoc,
    library_path: PathBuf,
    spec_path: PathBuf,
) -> Result<EngineeringChangeOrder, SpecError> {
    let sheet = doc.sheet()
        .map_err(|e| SpecError::no_span(SpecErrorCode::AltiumFormat, e.to_string()))?;

    let mut changes = Vec::new();

    for sheet_spec in &spec.sheets {
        // Diff components by designator
        let existing_map: HashMap<&str, &api::SchDocComponent> = sheet.components()
            .into_iter()
            .map(|c| (c.designator.as_str(), c))
            .collect();

        for comp_spec in &sheet_spec.components {
            if let Some(existing) = existing_map.get(comp_spec.designator.as_str()) {
                let mut prop_changes = Vec::new();
                let new_loc = format!("{}", comp_spec.location);
                let old_loc = format!("{}", existing.location);
                if new_loc != old_loc {
                    prop_changes.push(PropChange {
                        field: "location".to_string(),
                        old_value: old_loc,
                        new_value: new_loc,
                    });
                }
                if prop_changes.is_empty() {
                    changes.push(EntityChange::Unchanged {
                        kind: EntityKind::Component,
                        identity: comp_spec.designator.clone(),
                    });
                } else {
                    changes.push(EntityChange::Update {
                        kind: EntityKind::Component,
                        identity: comp_spec.designator.clone(),
                        prop_changes,
                        children: vec![],
                    });
                }
            } else {
                changes.push(schdoc_component_to_add(comp_spec));
            }
        }

        // Low-level objects: all are treated as adds (no identity matching)
        for obj_spec in &sheet_spec.objects {
            changes.push(schdoc_object_to_add(obj_spec));
        }

        // Nets and powers
        for net_spec in &sheet_spec.nets {
            changes.push(EntityChange::Add {
                kind: EntityKind::Net,
                identity: net_spec.name.clone(),
                props: vec![PropValue {
                    field: "pins".to_string(),
                    value: net_spec.pins.iter()
                        .map(|p| format!("{}.{}", p.component, p.pin))
                        .collect::<Vec<_>>()
                        .join(", "),
                }],
                children: vec![],
            });
        }
        for power_spec in &sheet_spec.powers {
            changes.push(EntityChange::Add {
                kind: EntityKind::Power,
                identity: power_spec.name.clone(),
                props: vec![PropValue {
                    field: "pins".to_string(),
                    value: power_spec.pins.iter()
                        .map(|p| format!("{}.{}", p.component, p.pin))
                        .collect::<Vec<_>>()
                        .join(", "),
                }],
                children: vec![],
            });
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

/// Reconcile against an empty SchDoc: every entity in the spec is an Add.
pub fn reconcile_schdoc_empty(
    spec: &SchDocSpec,
    library_path: PathBuf,
    spec_path: PathBuf,
) -> EngineeringChangeOrder {
    let mut changes = Vec::new();

    for sheet_spec in &spec.sheets {
        for comp_spec in &sheet_spec.components {
            changes.push(schdoc_component_to_add(comp_spec));
        }
        for obj_spec in &sheet_spec.objects {
            changes.push(schdoc_object_to_add(obj_spec));
        }
        for net_spec in &sheet_spec.nets {
            changes.push(EntityChange::Add {
                kind: EntityKind::Net,
                identity: net_spec.name.clone(),
                props: vec![PropValue {
                    field: "pins".to_string(),
                    value: net_spec.pins.iter()
                        .map(|p| format!("{}.{}", p.component, p.pin))
                        .collect::<Vec<_>>()
                        .join(", "),
                }],
                children: vec![],
            });
        }
        for power_spec in &sheet_spec.powers {
            changes.push(EntityChange::Add {
                kind: EntityKind::Power,
                identity: power_spec.name.clone(),
                props: vec![PropValue {
                    field: "pins".to_string(),
                    value: power_spec.pins.iter()
                        .map(|p| format!("{}.{}", p.component, p.pin))
                        .collect::<Vec<_>>()
                        .join(", "),
                }],
                children: vec![],
            });
        }
    }

    let summary = compute_summary(&changes);
    EngineeringChangeOrder {
        library_path,
        spec_path,
        timestamp: SystemTime::now(),
        summary,
        changes,
    }
}

fn schdoc_component_to_add(spec: &crate::model::SchDocComponentSpec) -> EntityChange {
    let mut props = vec![
        PropValue {
            field: "designator".to_string(),
            value: spec.designator.clone(),
        },
        PropValue {
            field: "location".to_string(),
            value: format!("{}", spec.location),
        },
    ];
    match &spec.symbol {
        crate::model::SymbolRef::Import { alias, name } => {
            props.push(PropValue {
                field: "symbol".to_string(),
                value: format!("${}.{}", alias, name),
            });
        }
        crate::model::SymbolRef::Literal(name) => {
            props.push(PropValue {
                field: "lib_reference".to_string(),
                value: name.clone(),
            });
        }
    }
    EntityChange::Add {
        kind: EntityKind::Component,
        identity: spec.designator.clone(),
        props,
        children: vec![],
    }
}

fn schdoc_object_to_add(spec: &SchDocObjectSpec) -> EntityChange {
    let (kind, identity, props) = match spec {
        SchDocObjectSpec::Wire(w) => (
            EntityKind::Wire,
            format!("wire@{}", w.vertices.first().map(|v| format!("{v}")).unwrap_or_default()),
            vec![PropValue {
                field: "vertices".to_string(),
                value: w.vertices.iter().map(|v| format!("{v}")).collect::<Vec<_>>().join(", "),
            }],
        ),
        SchDocObjectSpec::Bus(b) => (
            EntityKind::Bus,
            format!("bus@{}", b.vertices.first().map(|v| format!("{v}")).unwrap_or_default()),
            vec![PropValue {
                field: "vertices".to_string(),
                value: b.vertices.iter().map(|v| format!("{v}")).collect::<Vec<_>>().join(", "),
            }],
        ),
        SchDocObjectSpec::NetLabel(n) => (
            EntityKind::NetLabel,
            n.text.clone(),
            vec![PropValue { field: "location".to_string(), value: format!("{}", n.location) }],
        ),
        SchDocObjectSpec::PowerObject(p) => (
            EntityKind::PowerObject,
            p.text.clone(),
            vec![PropValue { field: "location".to_string(), value: format!("{}", p.location) }],
        ),
        SchDocObjectSpec::Port(p) => (
            EntityKind::Port,
            p.name.clone(),
            vec![PropValue { field: "location".to_string(), value: format!("{}", p.location) }],
        ),
        SchDocObjectSpec::Junction(j) => (
            EntityKind::Junction,
            format!("junction@{}", j.location),
            vec![],
        ),
        SchDocObjectSpec::NoConnect(n) => (
            EntityKind::NoConnect,
            format!("no_connect@{}", n.location),
            vec![],
        ),
        SchDocObjectSpec::BusEntry(b) => (
            EntityKind::BusEntry,
            format!("bus_entry@{}", b.location),
            vec![PropValue { field: "corner".to_string(), value: format!("{}", b.corner) }],
        ),
        SchDocObjectSpec::SheetSymbol(s) => (
            EntityKind::SheetSymbol,
            s.sheet_name.clone(),
            vec![PropValue { field: "location".to_string(), value: format!("{}", s.location) }],
        ),
        SchDocObjectSpec::ParameterSet(p) => (
            EntityKind::ParameterSet,
            p.name.clone(),
            vec![],
        ),
        SchDocObjectSpec::Note(n) => (
            EntityKind::Note,
            format!("note@{}", n.location),
            vec![PropValue { field: "text".to_string(), value: n.text.clone() }],
        ),
        SchDocObjectSpec::Probe(p) => (
            EntityKind::Probe,
            p.name.clone(),
            vec![PropValue { field: "location".to_string(), value: format!("{}", p.location) }],
        ),
        SchDocObjectSpec::CompileMask(c) => (
            EntityKind::CompileMask,
            format!("compile_mask@{}", c.location),
            vec![PropValue { field: "corner".to_string(), value: format!("{}", c.corner) }],
        ),
        SchDocObjectSpec::Blanket(b) => (
            EntityKind::Blanket,
            format!("blanket@{}", b.location),
            vec![PropValue { field: "corner".to_string(), value: format!("{}", b.corner) }],
        ),
        SchDocObjectSpec::Graphic(g) => (
            EntityKind::Graphic,
            g.unique_id.clone(),
            vec![],
        ),
        SchDocObjectSpec::Parameter(p) => (
            EntityKind::Parameter,
            p.name.clone(),
            vec![PropValue { field: "text".to_string(), value: p.text.clone() }],
        ),
        SchDocObjectSpec::HarnessConnector(h) => (
            EntityKind::HarnessConnector,
            format!("harness@{}", h.location),
            vec![],
        ),
        SchDocObjectSpec::SignalHarness(s) => (
            EntityKind::SignalHarness,
            format!("signal_harness@{}", s.vertices.first().map(|v| format!("{v}")).unwrap_or_default()),
            vec![],
        ),
    };
    EntityChange::Add { kind, identity, props, children: vec![] }
}

// ── PcbDoc reconciler ─────────────────────────────────────────────────────────

/// Reconcile a PcbDoc spec against an existing PcbDoc document.
///
/// Compares named collections by name/designator and primitives by ID/position.
/// This is a read-only operation: the document is not modified.
pub fn reconcile_pcbdoc(
    spec: &PcbDocSpec,
    doc: &PcbDoc,
    library_path: PathBuf,
    spec_path: PathBuf,
) -> Result<EngineeringChangeOrder, SpecError> {
    let board = doc.board()
        .map_err(|e| SpecError::no_span(SpecErrorCode::AltiumFormat, e.to_string()))?;

    let mut changes = Vec::new();

    for board_spec in &spec.boards {
        // Board settings diff
        diff_board_settings(&board.settings, board_spec, &mut changes);

        // Named collections: match by name/designator
        diff_pcbdoc_nets(&board, &board_spec.nets, &mut changes);
        diff_pcbdoc_components(&board, &board_spec.components, &mut changes);
        diff_pcbdoc_polygons(&board, &board_spec.polygons, &mut changes);
        diff_pcbdoc_rules(&board, &board_spec.rules, &mut changes);
        diff_pcbdoc_classes(&board, &board_spec.classes, &mut changes);
        diff_pcbdoc_diff_pairs(&board, &board_spec.differential_pairs, &mut changes);

        // Primitives: match by ID, fallback to position_index
        diff_pcbdoc_tracks(&board, &board_spec.tracks, &mut changes);
        diff_pcbdoc_arcs(&board, &board_spec.arcs, &mut changes);
        diff_pcbdoc_vias(&board, &board_spec.vias, &mut changes);
        diff_pcbdoc_fills(&board, &board_spec.fills, &mut changes);
        diff_pcbdoc_texts(&board, &board_spec.texts, &mut changes);
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

/// Reconcile against an empty PcbDoc: every entity in the spec is an Add.
pub fn reconcile_pcbdoc_empty(
    spec: &PcbDocSpec,
    library_path: PathBuf,
    spec_path: PathBuf,
) -> EngineeringChangeOrder {
    let mut changes = Vec::new();

    for board_spec in &spec.boards {
        // Board settings as Add
        if !board_spec.name.is_empty() {
            changes.push(EntityChange::Add {
                kind: EntityKind::Board,
                identity: board_spec.name.clone(),
                props: board_settings_props(board_spec),
                children: vec![],
            });
        }

        // Named collections
        for net in &board_spec.nets {
            changes.push(pcbdoc_net_to_add(net));
        }
        for comp in &board_spec.components {
            changes.push(pcbdoc_component_to_add(comp));
        }
        for poly in &board_spec.polygons {
            changes.push(pcbdoc_polygon_to_add(poly));
        }
        for rule in &board_spec.rules {
            changes.push(pcbdoc_rule_to_add(rule));
        }
        for cls in &board_spec.classes {
            changes.push(pcbdoc_class_to_add(cls));
        }
        for dp in &board_spec.differential_pairs {
            changes.push(pcbdoc_diff_pair_to_add(dp));
        }

        // Primitives
        for prim in board_spec.tracks.iter()
            .chain(&board_spec.arcs)
            .chain(&board_spec.vias)
            .chain(&board_spec.pads)
            .chain(&board_spec.fills)
            .chain(&board_spec.texts)
            .chain(&board_spec.regions)
            .chain(&board_spec.component_bodies)
            .chain(&board_spec.dimensions)
        {
            changes.push(pcbdoc_primitive_to_add(prim));
        }
    }

    let summary = compute_summary(&changes);
    EngineeringChangeOrder {
        library_path,
        spec_path,
        timestamp: SystemTime::now(),
        summary,
        changes,
    }
}

// ── PcbDoc diff helpers ──────────────────────────────────────────────────────

fn diff_board_settings(
    existing: &api::BoardSettings,
    spec: &BoardSpec,
    changes: &mut Vec<EntityChange>,
) {
    let mut prop_changes = Vec::new();
    if let Some(count) = spec.signal_layer_count {
        if count != existing.signal_layer_count {
            prop_changes.push(PropChange {
                field: "signal_layer_count".to_string(),
                old_value: existing.signal_layer_count.to_string(),
                new_value: count.to_string(),
            });
        }
    }
    if let Some(grid) = spec.snap_grid_size {
        if grid != existing.snap_grid_size {
            prop_changes.push(PropChange {
                field: "snap_grid_size".to_string(),
                old_value: format!("{}", existing.snap_grid_size),
                new_value: format!("{}", grid),
            });
        }
    }
    if let Some(grid) = spec.visible_grid_size {
        if grid != existing.visible_grid_size {
            prop_changes.push(PropChange {
                field: "visible_grid_size".to_string(),
                old_value: format!("{}", existing.visible_grid_size),
                new_value: format!("{}", grid),
            });
        }
    }
    if let Some(ref unit_str) = spec.display_unit {
        let existing_unit = format!("{:?}", existing.display_unit).to_lowercase();
        if *unit_str != existing_unit {
            prop_changes.push(PropChange {
                field: "display_unit".to_string(),
                old_value: existing_unit,
                new_value: unit_str.clone(),
            });
        }
    }

    if !prop_changes.is_empty() {
        changes.push(EntityChange::Update {
            kind: EntityKind::Board,
            identity: spec.name.clone(),
            prop_changes,
            children: vec![],
        });
    } else if !spec.name.is_empty() {
        changes.push(EntityChange::Unchanged {
            kind: EntityKind::Board,
            identity: spec.name.clone(),
        });
    }
}

fn diff_pcbdoc_nets(
    board: &api::PcbDocBoard,
    specs: &[PcbDocNetSpec],
    changes: &mut Vec<EntityChange>,
) {
    let existing_map: HashMap<&str, &api::Net> = board.nets.iter()
        .map(|n| (n.name.as_str(), n))
        .collect();

    for spec in specs {
        if let Some(existing) = existing_map.get(spec.name.as_str()) {
            let mut prop_changes = Vec::new();
            if let Some(color) = spec.color {
                if color != existing.color {
                    prop_changes.push(PropChange {
                        field: "color".to_string(),
                        old_value: format!("{:?}", existing.color),
                        new_value: format!("{:?}", color),
                    });
                }
            }
            if let Some(visible) = spec.visible {
                if visible != existing.visible {
                    prop_changes.push(PropChange {
                        field: "visible".to_string(),
                        old_value: existing.visible.to_string(),
                        new_value: visible.to_string(),
                    });
                }
            }
            if prop_changes.is_empty() {
                changes.push(EntityChange::Unchanged {
                    kind: EntityKind::PcbDocNet,
                    identity: spec.name.clone(),
                });
            } else {
                changes.push(EntityChange::Update {
                    kind: EntityKind::PcbDocNet,
                    identity: spec.name.clone(),
                    prop_changes,
                    children: vec![],
                });
            }
        } else {
            changes.push(pcbdoc_net_to_add(spec));
        }
    }
}

fn diff_pcbdoc_components(
    board: &api::PcbDocBoard,
    specs: &[PcbDocComponentSpec],
    changes: &mut Vec<EntityChange>,
) {
    let existing_map: HashMap<&str, &api::PcbDocComponent> = board.components.iter()
        .map(|c| (c.designator.as_str(), c))
        .collect();

    for spec in specs {
        if let Some(existing) = existing_map.get(spec.designator.as_str()) {
            let mut prop_changes = Vec::new();
            if let Some(ref pattern) = spec.pattern {
                if *pattern != existing.pattern {
                    prop_changes.push(PropChange {
                        field: "pattern".to_string(),
                        old_value: existing.pattern.clone(),
                        new_value: pattern.clone(),
                    });
                }
            }
            if let Some(loc) = spec.location {
                if loc != existing.location {
                    prop_changes.push(PropChange {
                        field: "location".to_string(),
                        old_value: format!("{}", existing.location),
                        new_value: format!("{}", loc),
                    });
                }
            }
            if let Some(rot) = spec.rotation {
                if (rot - existing.rotation).abs() > 0.001 {
                    prop_changes.push(PropChange {
                        field: "rotation".to_string(),
                        old_value: format!("{}", existing.rotation),
                        new_value: format!("{}", rot),
                    });
                }
            }
            if prop_changes.is_empty() {
                changes.push(EntityChange::Unchanged {
                    kind: EntityKind::PcbDocComponent,
                    identity: spec.designator.clone(),
                });
            } else {
                changes.push(EntityChange::Update {
                    kind: EntityKind::PcbDocComponent,
                    identity: spec.designator.clone(),
                    prop_changes,
                    children: vec![],
                });
            }
        } else {
            changes.push(pcbdoc_component_to_add(spec));
        }
    }
}

fn diff_pcbdoc_polygons(
    board: &api::PcbDocBoard,
    specs: &[PcbDocPolygonSpec],
    changes: &mut Vec<EntityChange>,
) {
    let existing_map: HashMap<&str, &api::Polygon> = board.polygons.iter()
        .map(|p| (p.name.as_str(), p))
        .collect();

    for spec in specs {
        if let Some(_existing) = existing_map.get(spec.name.as_str()) {
            // For now, just mark as unchanged (polygon property diff is complex)
            changes.push(EntityChange::Unchanged {
                kind: EntityKind::Polygon,
                identity: spec.name.clone(),
            });
        } else {
            changes.push(pcbdoc_polygon_to_add(spec));
        }
    }
}

fn diff_pcbdoc_rules(
    board: &api::PcbDocBoard,
    specs: &[PcbDocRuleSpec],
    changes: &mut Vec<EntityChange>,
) {
    let existing_map: HashMap<&str, &api::DesignRule> = board.rules.iter()
        .map(|r| (r.name.as_str(), r))
        .collect();

    for spec in specs {
        if let Some(existing) = existing_map.get(spec.name.as_str()) {
            let mut prop_changes = Vec::new();
            if let Some(enabled) = spec.enabled {
                if enabled != existing.enabled {
                    prop_changes.push(PropChange {
                        field: "enabled".to_string(),
                        old_value: existing.enabled.to_string(),
                        new_value: enabled.to_string(),
                    });
                }
            }
            if let Some(priority) = spec.priority {
                if priority != existing.priority {
                    prop_changes.push(PropChange {
                        field: "priority".to_string(),
                        old_value: existing.priority.to_string(),
                        new_value: priority.to_string(),
                    });
                }
            }
            if prop_changes.is_empty() {
                changes.push(EntityChange::Unchanged {
                    kind: EntityKind::Rule,
                    identity: spec.name.clone(),
                });
            } else {
                changes.push(EntityChange::Update {
                    kind: EntityKind::Rule,
                    identity: spec.name.clone(),
                    prop_changes,
                    children: vec![],
                });
            }
        } else {
            changes.push(pcbdoc_rule_to_add(spec));
        }
    }
}

fn diff_pcbdoc_classes(
    board: &api::PcbDocBoard,
    specs: &[PcbDocClassSpec],
    changes: &mut Vec<EntityChange>,
) {
    let existing_map: HashMap<&str, &api::NetClass> = board.classes.iter()
        .map(|c| (c.name.as_str(), c))
        .collect();

    for spec in specs {
        if existing_map.contains_key(spec.name.as_str()) {
            changes.push(EntityChange::Unchanged {
                kind: EntityKind::Class,
                identity: spec.name.clone(),
            });
        } else {
            changes.push(pcbdoc_class_to_add(spec));
        }
    }
}

fn diff_pcbdoc_diff_pairs(
    board: &api::PcbDocBoard,
    specs: &[PcbDocDifferentialPairSpec],
    changes: &mut Vec<EntityChange>,
) {
    let existing_map: HashMap<&str, &api::DifferentialPair> = board.differential_pairs.iter()
        .map(|dp| (dp.name.as_str(), dp))
        .collect();

    for spec in specs {
        if let Some(existing) = existing_map.get(spec.name.as_str()) {
            let mut prop_changes = Vec::new();
            if let Some(ref pos) = spec.positive_net {
                if *pos != existing.positive_net {
                    prop_changes.push(PropChange {
                        field: "positive_net".to_string(),
                        old_value: existing.positive_net.clone(),
                        new_value: pos.clone(),
                    });
                }
            }
            if let Some(ref neg) = spec.negative_net {
                if *neg != existing.negative_net {
                    prop_changes.push(PropChange {
                        field: "negative_net".to_string(),
                        old_value: existing.negative_net.clone(),
                        new_value: neg.clone(),
                    });
                }
            }
            if prop_changes.is_empty() {
                changes.push(EntityChange::Unchanged {
                    kind: EntityKind::DifferentialPair,
                    identity: spec.name.clone(),
                });
            } else {
                changes.push(EntityChange::Update {
                    kind: EntityKind::DifferentialPair,
                    identity: spec.name.clone(),
                    prop_changes,
                    children: vec![],
                });
            }
        } else {
            changes.push(pcbdoc_diff_pair_to_add(spec));
        }
    }
}

// ── Primitive diff (match by ID then position_index) ─────────────────────────

fn diff_pcbdoc_tracks(
    board: &api::PcbDocBoard,
    specs: &[PcbDocPrimitiveSpec],
    changes: &mut Vec<EntityChange>,
) {
    let existing_by_id: HashMap<&str, &api::Track> = board.tracks.iter()
        .map(|t| (t.id.as_str(), t))
        .collect();

    for spec in specs {
        if let Some(existing) = existing_by_id.get(spec.id.as_str()) {
            let mut prop_changes = Vec::new();
            check_coord_prop(&spec.properties, "from", &format!("{}", existing.start), &mut prop_changes, "start");
            check_coord_prop(&spec.properties, "to", &format!("{}", existing.end), &mut prop_changes, "end");
            if prop_changes.is_empty() {
                changes.push(EntityChange::Unchanged { kind: EntityKind::Track, identity: spec.id.clone() });
            } else {
                changes.push(EntityChange::Update { kind: EntityKind::Track, identity: spec.id.clone(), prop_changes, children: vec![] });
            }
        } else {
            changes.push(pcbdoc_primitive_to_add(spec));
        }
    }
}

fn diff_pcbdoc_arcs(
    board: &api::PcbDocBoard,
    specs: &[PcbDocPrimitiveSpec],
    changes: &mut Vec<EntityChange>,
) {
    let existing_by_id: HashMap<&str, &api::Arc> = board.arcs.iter()
        .map(|a| (a.id.as_str(), a))
        .collect();

    for spec in specs {
        if existing_by_id.contains_key(spec.id.as_str()) {
            changes.push(EntityChange::Unchanged { kind: EntityKind::Arc, identity: spec.id.clone() });
        } else {
            changes.push(pcbdoc_primitive_to_add(spec));
        }
    }
}

fn diff_pcbdoc_vias(
    board: &api::PcbDocBoard,
    specs: &[PcbDocPrimitiveSpec],
    changes: &mut Vec<EntityChange>,
) {
    let existing_by_id: HashMap<&str, &api::Via> = board.vias.iter()
        .map(|v| (v.id.as_str(), v))
        .collect();

    for spec in specs {
        if existing_by_id.contains_key(spec.id.as_str()) {
            changes.push(EntityChange::Unchanged { kind: EntityKind::Via, identity: spec.id.clone() });
        } else {
            changes.push(pcbdoc_primitive_to_add(spec));
        }
    }
}

fn diff_pcbdoc_fills(
    board: &api::PcbDocBoard,
    specs: &[PcbDocPrimitiveSpec],
    changes: &mut Vec<EntityChange>,
) {
    let existing_by_id: HashMap<&str, &api::Fill> = board.fills.iter()
        .map(|f| (f.id.as_str(), f))
        .collect();

    for spec in specs {
        if existing_by_id.contains_key(spec.id.as_str()) {
            changes.push(EntityChange::Unchanged { kind: EntityKind::Fill, identity: spec.id.clone() });
        } else {
            changes.push(pcbdoc_primitive_to_add(spec));
        }
    }
}

fn diff_pcbdoc_texts(
    board: &api::PcbDocBoard,
    specs: &[PcbDocPrimitiveSpec],
    changes: &mut Vec<EntityChange>,
) {
    let existing_by_id: HashMap<&str, &api::PcbDocText> = board.texts.iter()
        .map(|t| (t.id.as_str(), t))
        .collect();

    for spec in specs {
        if existing_by_id.contains_key(spec.id.as_str()) {
            changes.push(EntityChange::Unchanged { kind: EntityKind::Text, identity: spec.id.clone() });
        } else {
            changes.push(pcbdoc_primitive_to_add(spec));
        }
    }
}

fn check_coord_prop(
    props: &indexmap::IndexMap<String, crate::eval::Value>,
    key: &str,
    existing_str: &str,
    prop_changes: &mut Vec<PropChange>,
    field_name: &str,
) {
    use crate::eval::Value;
    if let Some(val) = props.get(key) {
        let new_str = match val {
            Value::CoordPoint(x, y) => {
                use altium_format_types::Coord;
                format!("{}", altium_format_types::CoordPoint::new(Coord::new(*x), Coord::new(*y)))
            }
            other => format!("{:?}", other),
        };
        if new_str != existing_str {
            prop_changes.push(PropChange {
                field: field_name.to_string(),
                old_value: existing_str.to_string(),
                new_value: new_str,
            });
        }
    }
}

// ── PcbDoc Add helpers ───────────────────────────────────────────────────────

fn board_settings_props(spec: &BoardSpec) -> Vec<PropValue> {
    let mut props = Vec::new();
    if let Some(count) = spec.signal_layer_count {
        props.push(PropValue { field: "signal_layer_count".to_string(), value: count.to_string() });
    }
    if let Some(ref unit) = spec.display_unit {
        props.push(PropValue { field: "display_unit".to_string(), value: unit.clone() });
    }
    props
}

fn pcbdoc_net_to_add(spec: &PcbDocNetSpec) -> EntityChange {
    let mut props = vec![
        PropValue { field: "name".to_string(), value: spec.name.clone() },
    ];
    if let Some(visible) = spec.visible {
        props.push(PropValue { field: "visible".to_string(), value: visible.to_string() });
    }
    EntityChange::Add {
        kind: EntityKind::PcbDocNet,
        identity: spec.name.clone(),
        props,
        children: vec![],
    }
}

fn pcbdoc_component_to_add(spec: &PcbDocComponentSpec) -> EntityChange {
    let mut props = vec![
        PropValue { field: "designator".to_string(), value: spec.designator.clone() },
    ];
    if let Some(ref pattern) = spec.pattern {
        props.push(PropValue { field: "pattern".to_string(), value: pattern.clone() });
    }
    if let Some(loc) = spec.location {
        props.push(PropValue { field: "location".to_string(), value: format!("{}", loc) });
    }
    EntityChange::Add {
        kind: EntityKind::PcbDocComponent,
        identity: spec.designator.clone(),
        props,
        children: vec![],
    }
}

fn pcbdoc_polygon_to_add(spec: &PcbDocPolygonSpec) -> EntityChange {
    let mut props = vec![
        PropValue { field: "name".to_string(), value: spec.name.clone() },
    ];
    if let Some(ref net) = spec.net {
        props.push(PropValue { field: "net".to_string(), value: net.clone() });
    }
    EntityChange::Add {
        kind: EntityKind::Polygon,
        identity: spec.name.clone(),
        props,
        children: vec![],
    }
}

fn pcbdoc_rule_to_add(spec: &PcbDocRuleSpec) -> EntityChange {
    let mut props = vec![
        PropValue { field: "name".to_string(), value: spec.name.clone() },
    ];
    if let Some(ref kind) = spec.kind {
        props.push(PropValue { field: "kind".to_string(), value: kind.clone() });
    }
    EntityChange::Add {
        kind: EntityKind::Rule,
        identity: spec.name.clone(),
        props,
        children: vec![],
    }
}

fn pcbdoc_class_to_add(spec: &PcbDocClassSpec) -> EntityChange {
    let mut props = vec![
        PropValue { field: "name".to_string(), value: spec.name.clone() },
    ];
    if let Some(ref kind) = spec.kind {
        props.push(PropValue { field: "kind".to_string(), value: kind.clone() });
    }
    EntityChange::Add {
        kind: EntityKind::Class,
        identity: spec.name.clone(),
        props,
        children: vec![],
    }
}

fn pcbdoc_diff_pair_to_add(spec: &PcbDocDifferentialPairSpec) -> EntityChange {
    let mut props = vec![
        PropValue { field: "name".to_string(), value: spec.name.clone() },
    ];
    if let Some(ref pos) = spec.positive_net {
        props.push(PropValue { field: "positive_net".to_string(), value: pos.clone() });
    }
    if let Some(ref neg) = spec.negative_net {
        props.push(PropValue { field: "negative_net".to_string(), value: neg.clone() });
    }
    EntityChange::Add {
        kind: EntityKind::DifferentialPair,
        identity: spec.name.clone(),
        props,
        children: vec![],
    }
}

fn pcbdoc_primitive_to_add(spec: &PcbDocPrimitiveSpec) -> EntityChange {
    let kind = match spec.primitive_type.as_str() {
        "track" => EntityKind::Track,
        "arc" => EntityKind::Arc,
        "via" => EntityKind::Via,
        "pad" => EntityKind::Pad,
        "fill" => EntityKind::Fill,
        "text" => EntityKind::Text,
        "region" => EntityKind::Region,
        "component_body" => EntityKind::ComponentBody,
        "dimension" => EntityKind::Dimension,
        _ => EntityKind::Track, // fallback
    };
    let props: Vec<PropValue> = spec.properties.iter()
        .map(|(k, v)| PropValue { field: k.clone(), value: format!("{:?}", v) })
        .collect();
    EntityChange::Add {
        kind,
        identity: spec.id.clone(),
        props,
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
    use crate::executor::apply_spec_schlib;
    use altium_format_types::{CoordPoint, Coord, RotationBy90};

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
        let mut doc = SchLib::new_blank_ad26().expect("blank schlib");
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
                x: Coord::from_mils(x_mils).expect("test coord"),
                y: Coord::from_mils(y_mils).expect("test coord"),
            },
            shape: Some(PadShape::Round),
            x_size: Some(Coord::from_mils(60).expect("60 mils fits Coord")),
            y_size: Some(Coord::from_mils(60).expect("60 mils fits Coord")),
            rotation: None,
            hole_size: Some(Coord::from_mils(28).expect("28 mils fits Coord")),
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

    // ── PcbLib reconciler with existing doc ────────────────────────────────

    use crate::executor::apply_spec_pcblib;
    use altium_format::PcbLib;

    fn blank_pcblib() -> PcbLib {
        PcbLib::new_blank_ad26().expect("blank pcblib")
    }

    #[test]
    fn pcblib_reconcile_unchanged() {
        let spec = make_pcblib_spec(vec![
            make_footprint("SOT23", vec![make_pad("1", -50, 0), make_pad("2", 50, 0)]),
        ]);
        let mut lib = blank_pcblib();
        apply_spec_pcblib(&spec, &mut lib).unwrap();

        let tmp = tempfile::NamedTempFile::new().unwrap();
        lib.save(tmp.path()).unwrap();

        let eco = reconcile_pcblib(
            &spec,
            tmp.path().to_path_buf(),
            PathBuf::from("test.pcblib-spec"),
        );

        assert_eq!(eco.changes.len(), 1);
        assert!(matches!(&eco.changes[0], EntityChange::Unchanged { kind: EntityKind::Footprint, identity } if identity == "SOT23"));
    }

    #[test]
    fn pcblib_reconcile_detects_new_footprint() {
        let spec1 = make_pcblib_spec(vec![
            make_footprint("SOT23", vec![make_pad("1", 0, 0)]),
        ]);
        let mut lib = blank_pcblib();
        apply_spec_pcblib(&spec1, &mut lib).unwrap();

        let tmp = tempfile::NamedTempFile::new().unwrap();
        lib.save(tmp.path()).unwrap();

        let spec2 = make_pcblib_spec(vec![
            make_footprint("SOT23", vec![make_pad("1", 0, 0)]),
            make_footprint("0603", vec![make_pad("1", -30, 0), make_pad("2", 30, 0)]),
        ]);

        let eco = reconcile_pcblib(
            &spec2,
            tmp.path().to_path_buf(),
            PathBuf::from("test.pcblib-spec"),
        );

        assert_eq!(eco.changes.len(), 2);
        assert!(matches!(&eco.changes[0], EntityChange::Unchanged { .. }));
        assert!(matches!(&eco.changes[1], EntityChange::Add { kind: EntityKind::Footprint, identity, .. } if identity == "0603"));
    }

    #[test]
    fn pcblib_reconcile_detects_description_change() {
        let spec1 = make_pcblib_spec(vec![
            make_footprint("0805", vec![make_pad("1", 0, 0)]),
        ]);
        let mut lib = blank_pcblib();
        apply_spec_pcblib(&spec1, &mut lib).unwrap();

        let tmp = tempfile::NamedTempFile::new().unwrap();
        lib.save(tmp.path()).unwrap();

        let spec2 = make_pcblib_spec(vec![FootprintSpec {
            display_name: "0805".to_string(),
            description: Some("Updated 0805".to_string()),
            height: None,
            pattern: None,
            pads: vec![make_pad("1", 0, 0)],
            graphics: vec![],
        }]);

        let eco = reconcile_pcblib(
            &spec2,
            tmp.path().to_path_buf(),
            PathBuf::from("test.pcblib-spec"),
        );

        assert_eq!(eco.changes.len(), 1);
        if let EntityChange::Update { prop_changes, .. } = &eco.changes[0] {
            let desc_change = prop_changes.iter().find(|pc| pc.field == "description").unwrap();
            assert_eq!(desc_change.old_value, "Test footprint");
            assert_eq!(desc_change.new_value, "Updated 0805");
        } else {
            panic!("expected Update, got {:?}", eco.changes[0]);
        }
    }

    #[test]
    fn pcblib_reconcile_detects_new_pad() {
        let spec1 = make_pcblib_spec(vec![
            make_footprint("QFP", vec![make_pad("1", 0, 0)]),
        ]);
        let mut lib = blank_pcblib();
        apply_spec_pcblib(&spec1, &mut lib).unwrap();

        let tmp = tempfile::NamedTempFile::new().unwrap();
        lib.save(tmp.path()).unwrap();

        let spec2 = make_pcblib_spec(vec![
            make_footprint("QFP", vec![make_pad("1", 0, 0), make_pad("2", 100, 0)]),
        ]);

        let eco = reconcile_pcblib(
            &spec2,
            tmp.path().to_path_buf(),
            PathBuf::from("test.pcblib-spec"),
        );

        assert_eq!(eco.changes.len(), 1);
        if let EntityChange::Update { children, .. } = &eco.changes[0] {
            let pad_unchanged = children.iter()
                .filter(|c| matches!(c, EntityChange::Unchanged { kind: EntityKind::Pad, .. }))
                .count();
            let pad_adds = children.iter()
                .filter(|c| matches!(c, EntityChange::Add { kind: EntityKind::Pad, .. }))
                .count();
            assert_eq!(pad_unchanged, 1); // pad "1"
            assert_eq!(pad_adds, 1); // pad "2"
        } else {
            panic!("expected Update");
        }
    }
}
