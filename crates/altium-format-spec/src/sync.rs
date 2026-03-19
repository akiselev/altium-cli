//! SyncSnapshot intermediate representation and projection functions.
//!
//! `SyncSnapshot` is a common IR that both SchDoc and PcbDoc specs project into,
//! enabling spec-to-spec synchronization via diff and apply.
//!
//! The snapshot is ephemeral (never persisted) and computed fresh from a compiled
//! spec model. Projection fails hard on invalid input (dangling refs, duplicate
//! designators) per the fail-fast invariant.

use std::collections::HashMap;

use indexmap::IndexMap;

use crate::eval::{SpecError, SpecErrorCode};
use crate::model::{ComponentSpec, PcbDocSpec, SchDocSpec, SymbolRef};

// ── IR types ─────────────────────────────────────────────────────────────────

/// Flat snapshot of components and nets from a spec, used as common IR for sync.
#[derive(Debug, Clone, Default)]
pub struct SyncSnapshot {
    /// Components keyed by designator. IndexMap preserves spec declaration order.
    pub components: IndexMap<String, SyncComponent>,
    /// Nets keyed by name. IndexMap preserves spec declaration order.
    pub nets: IndexMap<String, SyncNet>,
}

/// A component entry in the sync snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncComponent {
    pub designator: String,
    pub comment: Option<String>,
    pub footprint: Option<String>,
    pub source_library: Option<String>,
    pub parameters: IndexMap<String, String>,
    pub pins: IndexMap<String, SyncPin>,
    pub annotation_id: Option<String>,
    /// Altium UNIQUE_ID of the source schematic component (no backslash prefix).
    pub source_unique_id: Option<String>,
}

/// A pin entry within a component in the sync snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncPin {
    pub designator: String,
    pub net: Option<String>,
}

/// A net entry in the sync snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncNet {
    pub name: String,
    pub color: Option<String>,
    /// (component_designator, pin_designator) tuples.
    pub pins: Vec<(String, String)>,
    pub annotation_id: Option<String>,
}

// ── Pin-to-pad resolution ─────────────────────────────────────────────────────

/// Builds a map from pin name (or designator) → pad designator for one component.
///
/// Resolution chain:
/// 1. `PinSpec.name` → `PinSpec.designator` (name-to-designator lookup)
/// 2. `FootprintMapSpec.maps` → `PinPadMap { pin, pad }` (designator-to-pad lookup)
/// 3. Implicit 1:1 when `maps` is empty: pin designator IS pad name
///
/// Both pin names and pin designators are entered as keys so that callers can
/// look up either form without knowing which one the schdoc uses.
fn build_pin_to_pad_map(lib_comp: &ComponentSpec) -> HashMap<String, String> {
    // Step 1: Build pin_designator → pad_name from FootprintMapSpec.maps.
    let pin_des_to_pad: HashMap<String, String> = lib_comp
        .footprints
        .first()
        .map(|fp| {
            fp.maps
                .iter()
                .map(|m| (m.pin.clone(), m.pad.clone()))
                .collect()
        })
        .unwrap_or_default();

    let mut result: HashMap<String, String> = HashMap::new();

    for pin in &lib_comp.pins {
        let pad = pin_des_to_pad
            .get(&pin.designator)
            .cloned()
            .unwrap_or_else(|| pin.designator.clone());

        // Map by pin designator.
        result.insert(pin.designator.clone(), pad.clone());

        // Also map by pin name when present.
        if let Some(name) = &pin.name {
            result.insert(name.clone(), pad);
        }
    }

    result
}

// ── Projection: SchDoc ────────────────────────────────────────────────────────

/// Projects a compiled `SchDocSpec` into a `SyncSnapshot`.
///
/// Fails hard on:
/// - Duplicate component designators across sheets
/// - Net or power pin references to non-existent component designators
pub fn project_schdoc_spec(
    spec: &SchDocSpec,
    imported_components: &std::collections::HashMap<String, crate::model::ComponentSpec>,
) -> Result<SyncSnapshot, SpecError> {
    let mut snapshot = SyncSnapshot::default();
    // Keyed by component designator; built in Pass 1 from SchLib data.
    let mut pin_to_pad_maps: HashMap<String, HashMap<String, String>> = HashMap::new();

    // Pass 1: collect all components across sheets.
    for sheet in &spec.sheets {
        for comp in &sheet.components {
            if snapshot.components.contains_key(&comp.designator) {
                return Err(SpecError::no_span(
                    SpecErrorCode::DuplicateEntity,
                    format!(
                        "duplicate component designator '{}' found across sheets",
                        comp.designator
                    ),
                ));
            }

            let parameters: IndexMap<String, String> = comp
                .parameters
                .iter()
                .map(|p| (p.name.clone(), p.text.clone()))
                .collect();

            let annotation_id = comp.annotation.as_ref().map(|a| a.id.clone());
            let source_unique_id = comp.annotation.as_ref().and_then(|a| a.source_id.clone());

            // Resolve footprint pattern from imported SchLib component.
            let lib_ref = match &comp.symbol {
                SymbolRef::Import { name, .. } => name.as_str(),
                SymbolRef::Literal(name) => name.as_str(),
            };

            let (footprint, source_library) = match imported_components.get(lib_ref) {
                Some(lib_comp) => {
                    let pin_pad_map = build_pin_to_pad_map(lib_comp);
                    pin_to_pad_maps.insert(comp.designator.clone(), pin_pad_map);
                    let fp = lib_comp.footprints.first().map(|f| f.model_name.clone());
                    (fp, Some(lib_ref.to_string()))
                }
                None => (None, None),
            };

            snapshot.components.insert(
                comp.designator.clone(),
                SyncComponent {
                    designator: comp.designator.clone(),
                    comment: comp.description.clone(),
                    footprint,
                    source_library,
                    parameters,
                    pins: IndexMap::new(),
                    annotation_id,
                    source_unique_id,
                },
            );
        }
    }

    // Pass 2: populate pin.net from nets and powers across sheets.
    for sheet in &spec.sheets {
        for net in &sheet.nets {
            let annotation_id = net.annotation.as_ref().map(|a| a.id.clone());
            let mut net_pins: Vec<(String, String)> = Vec::new();

            for pin_ref in &net.pins {
                let comp = snapshot
                    .components
                    .get_mut(&pin_ref.component)
                    .ok_or_else(|| {
                        SpecError::no_span(
                            SpecErrorCode::InvalidFieldAccess,
                            format!(
                                "net '{}' references non-existent component '{}'",
                                net.name, pin_ref.component
                            ),
                        )
                    })?;

                let pad_designator = pin_to_pad_maps
                    .get(&pin_ref.component)
                    .and_then(|m| m.get(&pin_ref.pin))
                    .cloned()
                    .unwrap_or_else(|| pin_ref.pin.clone());

                let pin_entry =
                    comp.pins
                        .entry(pad_designator.clone())
                        .or_insert_with(|| SyncPin {
                            designator: pad_designator.clone(),
                            net: None,
                        });
                pin_entry.net = Some(net.name.clone());

                net_pins.push((pin_ref.component.clone(), pad_designator));
            }

            snapshot.nets.insert(
                net.name.clone(),
                SyncNet {
                    name: net.name.clone(),
                    color: None,
                    pins: net_pins,
                    annotation_id,
                },
            );
        }

        for power in &sheet.powers {
            let annotation_id = power.annotation.as_ref().map(|a| a.id.clone());
            let mut net_pins: Vec<(String, String)> = Vec::new();

            for pin_ref in &power.pins {
                let comp = snapshot
                    .components
                    .get_mut(&pin_ref.component)
                    .ok_or_else(|| {
                        SpecError::no_span(
                            SpecErrorCode::InvalidFieldAccess,
                            format!(
                                "power net '{}' references non-existent component '{}'",
                                power.name, pin_ref.component
                            ),
                        )
                    })?;

                let pad_designator = pin_to_pad_maps
                    .get(&pin_ref.component)
                    .and_then(|m| m.get(&pin_ref.pin))
                    .cloned()
                    .unwrap_or_else(|| pin_ref.pin.clone());

                let pin_entry =
                    comp.pins
                        .entry(pad_designator.clone())
                        .or_insert_with(|| SyncPin {
                            designator: pad_designator.clone(),
                            net: None,
                        });
                pin_entry.net = Some(power.name.clone());

                net_pins.push((pin_ref.component.clone(), pad_designator));
            }

            // Insert power net; if already present (e.g. net defined earlier in sheet),
            // merge pins into the existing entry rather than overwriting.
            snapshot
                .nets
                .entry(power.name.clone())
                .and_modify(|existing| {
                    existing.pins.extend(net_pins.clone());
                })
                .or_insert_with(|| SyncNet {
                    name: power.name.clone(),
                    color: None,
                    pins: net_pins,
                    annotation_id,
                });
        }
    }

    // Pass 3: collect nets from component pin_connections (`pin X -> #NET` syntax).
    // These may overlap with nets/powers already collected in Pass 2; merge them.
    for sheet in &spec.sheets {
        for comp in &sheet.components {
            for pin_conn in &comp.pin_connections {
                let net_name = match &pin_conn.target {
                    crate::model::PinConnectionTarget::Signal(name) => name.clone(),
                    crate::model::PinConnectionTarget::Power(name) => name.clone(),
                    crate::model::PinConnectionTarget::NoConnect => continue,
                };

                // Resolve pin name to pad designator.
                let pad_designator = pin_to_pad_maps
                    .get(&comp.designator)
                    .and_then(|m| m.get(&pin_conn.pin_name))
                    .cloned()
                    .unwrap_or_else(|| pin_conn.pin_name.clone());

                // Update the component's pin entry.
                if let Some(sync_comp) = snapshot.components.get_mut(&comp.designator) {
                    let pin_entry =
                        sync_comp
                            .pins
                            .entry(pad_designator.clone())
                            .or_insert_with(|| SyncPin {
                                designator: pad_designator.clone(),
                                net: None,
                            });
                    pin_entry.net = Some(net_name.clone());
                }

                // Insert or merge the net entry.
                let pin_tuple = (comp.designator.clone(), pad_designator);
                snapshot
                    .nets
                    .entry(net_name.clone())
                    .and_modify(|existing| {
                        existing.pins.push(pin_tuple.clone());
                    })
                    .or_insert_with(|| SyncNet {
                        name: net_name,
                        color: None,
                        pins: vec![pin_tuple],
                        annotation_id: None,
                    });
            }
        }
    }

    Ok(snapshot)
}

// ── Projection: PcbDoc ────────────────────────────────────────────────────────

/// Projects a compiled `PcbDocSpec` into a `SyncSnapshot`.
///
/// Fails hard on:
/// - Duplicate component designators across boards
/// - Duplicate net names across boards
///
/// Pins are left empty — PcbDoc specs do not carry pin-level connectivity.
pub fn project_pcbdoc_spec(spec: &PcbDocSpec) -> Result<SyncSnapshot, SpecError> {
    let mut snapshot = SyncSnapshot::default();

    for board in &spec.boards {
        for comp in &board.components {
            if snapshot.components.contains_key(&comp.designator) {
                return Err(SpecError::no_span(
                    SpecErrorCode::DuplicateEntity,
                    format!(
                        "duplicate component designator '{}' found across boards",
                        comp.designator
                    ),
                ));
            }

            let annotation_id = comp.annotation.as_ref().map(|a| a.id.clone());
            let source_unique_id = comp.annotation.as_ref().and_then(|a| a.source_id.clone());

            snapshot.components.insert(
                comp.designator.clone(),
                SyncComponent {
                    designator: comp.designator.clone(),
                    comment: comp.comment.clone(),
                    footprint: comp.pattern.clone(),
                    source_library: comp.source_library.clone(),
                    parameters: comp.parameters.clone(),
                    pins: IndexMap::new(),
                    annotation_id,
                    source_unique_id,
                },
            );
        }

        for net in &board.nets {
            if snapshot.nets.contains_key(&net.name) {
                return Err(SpecError::no_span(
                    SpecErrorCode::DuplicateEntity,
                    format!("duplicate net name '{}' found across boards", net.name),
                ));
            }

            let annotation_id = net.annotation.as_ref().map(|a| a.id.clone());
            let color = net.color.map(|c| c.to_string());

            snapshot.nets.insert(
                net.name.clone(),
                SyncNet {
                    name: net.name.clone(),
                    color,
                    pins: Vec::new(),
                    annotation_id,
                },
            );
        }
    }

    Ok(snapshot)
}

// ── Diff types ───────────────────────────────────────────────────────────────

/// A single field-level change within an update operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldChange {
    pub field: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
}

/// A change produced by `diff_snapshots`, describing what the target must do to match the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncChange {
    AddComponent {
        designator: String,
        component: SyncComponent,
    },
    RemoveComponent {
        designator: String,
    },
    UpdateComponent {
        designator: String,
        field_changes: Vec<FieldChange>,
    },
    AddNet {
        name: String,
        net: SyncNet,
    },
    RemoveNet {
        name: String,
    },
    UpdateNet {
        name: String,
        field_changes: Vec<FieldChange>,
    },
    AddPin {
        component_designator: String,
        pin_designator: String,
        pin: SyncPin,
    },
    RemovePin {
        component_designator: String,
        pin_designator: String,
    },
    UpdatePin {
        component_designator: String,
        pin_designator: String,
        field_changes: Vec<FieldChange>,
    },
}

/// Sync direction for a policy property.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncDirection {
    Forward,
    Back,
    Bidirectional,
    None,
}

/// Per-property sync policy controlling which changes are applied in a given direction.
///
/// Do NOT derive or implement `Default` — an all-`None` policy silently skips all sync,
/// which would be a hard-to-diagnose bug. Always construct explicitly with named fields.
pub struct SyncPolicy {
    pub comment: SyncDirection,
    pub footprint: SyncDirection,
    pub source_library: SyncDirection,
    pub parameters: SyncDirection,
    pub net_name: SyncDirection,
    pub net_color: SyncDirection,
    pub pin_net_assignment: SyncDirection,
    /// Present for policy completeness but not consulted during filtering because
    /// no location FieldChanges are generated by diff (location sync is excluded from
    /// all phases).
    pub component_location: SyncDirection,
}

// ── Diff algorithm ────────────────────────────────────────────────────────────

/// Computes what `target` must change to match `source`.
///
/// Direction-agnostic: the caller determines sync direction via `filter_changes`.
/// Component matching is by designator; net matching is by name.
pub fn diff_snapshots(source: &SyncSnapshot, target: &SyncSnapshot) -> Vec<SyncChange> {
    let mut changes: Vec<SyncChange> = Vec::new();

    // ── Components ────────────────────────────────────────────────────────────

    for (designator, src_comp) in &source.components {
        match target.components.get(designator) {
            None => {
                changes.push(SyncChange::AddComponent {
                    designator: designator.clone(),
                    component: src_comp.clone(),
                });
            }
            Some(tgt_comp) => {
                let mut field_changes: Vec<FieldChange> = Vec::new();

                if src_comp.comment != tgt_comp.comment {
                    field_changes.push(FieldChange {
                        field: "comment".to_string(),
                        old_value: tgt_comp.comment.clone(),
                        new_value: src_comp.comment.clone(),
                    });
                }

                if src_comp.footprint != tgt_comp.footprint {
                    field_changes.push(FieldChange {
                        field: "footprint".to_string(),
                        old_value: tgt_comp.footprint.clone(),
                        new_value: src_comp.footprint.clone(),
                    });
                }

                if src_comp.source_library != tgt_comp.source_library {
                    field_changes.push(FieldChange {
                        field: "source_library".to_string(),
                        old_value: tgt_comp.source_library.clone(),
                        new_value: src_comp.source_library.clone(),
                    });
                }

                // Parameters: compare each key present in either side.
                let all_param_keys: indexmap::IndexSet<&String> = src_comp
                    .parameters
                    .keys()
                    .chain(tgt_comp.parameters.keys())
                    .collect();
                for key in all_param_keys {
                    let src_val = src_comp.parameters.get(key).cloned();
                    let tgt_val = tgt_comp.parameters.get(key).cloned();
                    if src_val != tgt_val {
                        field_changes.push(FieldChange {
                            field: format!("parameter:{key}"),
                            old_value: tgt_val,
                            new_value: src_val,
                        });
                    }
                }

                if !field_changes.is_empty() {
                    changes.push(SyncChange::UpdateComponent {
                        designator: designator.clone(),
                        field_changes,
                    });
                }

                // Diff pins within matched components.
                for (pin_des, src_pin) in &src_comp.pins {
                    match tgt_comp.pins.get(pin_des) {
                        None => {
                            changes.push(SyncChange::AddPin {
                                component_designator: designator.clone(),
                                pin_designator: pin_des.clone(),
                                pin: src_pin.clone(),
                            });
                        }
                        Some(tgt_pin) => {
                            if src_pin.net != tgt_pin.net {
                                changes.push(SyncChange::UpdatePin {
                                    component_designator: designator.clone(),
                                    pin_designator: pin_des.clone(),
                                    field_changes: vec![FieldChange {
                                        field: "net".to_string(),
                                        old_value: tgt_pin.net.clone(),
                                        new_value: src_pin.net.clone(),
                                    }],
                                });
                            }
                        }
                    }
                }

                for pin_des in tgt_comp.pins.keys() {
                    if !src_comp.pins.contains_key(pin_des) {
                        changes.push(SyncChange::RemovePin {
                            component_designator: designator.clone(),
                            pin_designator: pin_des.clone(),
                        });
                    }
                }
            }
        }
    }

    for designator in target.components.keys() {
        if !source.components.contains_key(designator) {
            changes.push(SyncChange::RemoveComponent {
                designator: designator.clone(),
            });
        }
    }

    // ── Nets ──────────────────────────────────────────────────────────────────

    for (name, src_net) in &source.nets {
        match target.nets.get(name) {
            None => {
                changes.push(SyncChange::AddNet {
                    name: name.clone(),
                    net: src_net.clone(),
                });
            }
            Some(tgt_net) => {
                let mut field_changes: Vec<FieldChange> = Vec::new();

                debug_assert_eq!(
                    src_net.name, tgt_net.name,
                    "nets matched by key should always have equal names"
                );

                if src_net.color != tgt_net.color {
                    field_changes.push(FieldChange {
                        field: "net_color".to_string(),
                        old_value: tgt_net.color.clone(),
                        new_value: src_net.color.clone(),
                    });
                }

                if !field_changes.is_empty() {
                    changes.push(SyncChange::UpdateNet {
                        name: name.clone(),
                        field_changes,
                    });
                }
            }
        }
    }

    for name in target.nets.keys() {
        if !source.nets.contains_key(name) {
            changes.push(SyncChange::RemoveNet { name: name.clone() });
        }
    }

    changes
}

// ── Policy filtering ──────────────────────────────────────────────────────────

/// Filters `changes` according to `policy`, keeping only changes whose property direction
/// is `Bidirectional` or equals `direction`. `SyncDirection::None` always excludes a field.
///
/// Pin changes (`AddPin`/`RemovePin`/`UpdatePin`) are handled based on `policy.pin_net_assignment`:
/// - `SyncDirection::None`: pin changes are silently dropped (expected when pin sync is excluded).
/// - Any other direction: returns `Err` because apply does not support pin-level sync.
pub fn filter_changes(
    changes: &[SyncChange],
    policy: &SyncPolicy,
    direction: SyncDirection,
) -> Result<Vec<SyncChange>, SpecError> {
    let direction_allows = |policy_dir: SyncDirection| -> bool {
        match policy_dir {
            SyncDirection::None => false,
            SyncDirection::Bidirectional => true,
            d => d == direction,
        }
    };

    let mut result: Vec<SyncChange> = Vec::new();

    for change in changes {
        match change {
            SyncChange::AddPin { .. } | SyncChange::RemovePin { .. } | SyncChange::UpdatePin { .. } => {
                match policy.pin_net_assignment {
                    SyncDirection::None => continue,
                    _ => return Err(SpecError::no_span(
                        SpecErrorCode::NotSupported,
                        "pin-level sync is not supported: PcbDoc specs do not carry pin-level connectivity".to_string(),
                    )),
                }
            }

            SyncChange::AddComponent { .. } | SyncChange::RemoveComponent { .. } => {
                // Add/remove operations pass through unconditionally and cannot be suppressed
                // via per-field SyncPolicy directions.  Policy only governs which *fields* are
                // updated on existing components (UpdateComponent); the presence or absence of
                // a component in the target is always authoritative.
                result.push(change.clone());
            }

            SyncChange::UpdateComponent { designator, field_changes } => {
                let filtered: Vec<FieldChange> = field_changes
                    .iter()
                    .filter(|fc| {
                        let allowed = if fc.field == "comment" {
                            direction_allows(policy.comment)
                        } else if fc.field == "footprint" {
                            direction_allows(policy.footprint)
                        } else if fc.field == "source_library" {
                            direction_allows(policy.source_library)
                        } else if fc.field.starts_with("parameter:") {
                            direction_allows(policy.parameters)
                        } else {
                            // Unknown field: exclude by default (fail-safe).
                            false
                        };
                        allowed
                    })
                    .cloned()
                    .collect();

                if !filtered.is_empty() {
                    result.push(SyncChange::UpdateComponent {
                        designator: designator.clone(),
                        field_changes: filtered,
                    });
                }
            }

            SyncChange::AddNet { .. } | SyncChange::RemoveNet { .. } => {
                // Add/remove operations pass through unconditionally and cannot be suppressed
                // via per-field SyncPolicy directions.  Policy only governs which *fields* are
                // updated on existing nets (UpdateNet); the presence or absence of a net in
                // the target is always authoritative.
                result.push(change.clone());
            }

            SyncChange::UpdateNet { name, field_changes } => {
                let filtered: Vec<FieldChange> = field_changes
                    .iter()
                    .filter(|fc| {
                        if fc.field == "net_name" {
                            direction_allows(policy.net_name)
                        } else if fc.field == "net_color" {
                            direction_allows(policy.net_color)
                        } else {
                            false
                        }
                    })
                    .cloned()
                    .collect();

                if !filtered.is_empty() {
                    result.push(SyncChange::UpdateNet {
                        name: name.clone(),
                        field_changes: filtered,
                    });
                }
            }
        }
    }

    Ok(result)
}

// ── Change application ────────────────────────────────────────────────────────

/// Applies a filtered set of `SyncChange`s to a `PcbDocSpec` in 3-phase dependency order:
/// 1. Removes (RemoveComponent, RemoveNet)
/// 2. Updates (UpdateComponent, UpdateNet)
/// 3. Adds (AddComponent, AddNet)
///
/// Guards:
/// - Empty boards → error
/// - Multiple boards → error (only single-board specs are accepted)
/// - Pin-level changes → hard error (never silently drop connectivity changes)
pub fn apply_sync_changes_to_pcbdoc(
    changes: &[SyncChange],
    spec: &mut crate::model::PcbDocSpec,
) -> Result<(), SpecError> {
    if spec.boards.is_empty() {
        return Err(SpecError::no_span(
            SpecErrorCode::NotSupported,
            "PcbDoc spec must contain at least one board block for sync apply".to_string(),
        ));
    }
    if spec.boards.len() != 1 {
        return Err(SpecError::no_span(
            SpecErrorCode::NotSupported,
            "Multi-board PcbDoc specs are not supported by spec sync (only single-board specs are accepted)".to_string(),
        ));
    }

    // Phase 1: Removes.
    for change in changes {
        match change {
            SyncChange::RemoveComponent { designator } => {
                spec.boards[0]
                    .components
                    .retain(|c| &c.designator != designator);
            }
            SyncChange::RemoveNet { name } => {
                spec.boards[0].nets.retain(|n| &n.name != name);
            }
            SyncChange::AddPin { .. }
            | SyncChange::RemovePin { .. }
            | SyncChange::UpdatePin { .. } => {
                return Err(SpecError::no_span(
                    SpecErrorCode::NotSupported,
                    format!("pin-level sync not supported (change: {:?})", change),
                ));
            }
            _ => {}
        }
    }

    // Phase 2: Updates.
    for change in changes {
        match change {
            SyncChange::UpdateComponent {
                designator,
                field_changes,
            } => {
                let comp = spec.boards[0]
                    .components
                    .iter_mut()
                    .find(|c| &c.designator == designator)
                    .ok_or_else(|| {
                        SpecError::no_span(
                            SpecErrorCode::InvalidFieldAccess,
                            format!(
                                "applying sync change UpdateComponent to PcbDoc spec: \
                                 component '{}' not found",
                                designator
                            ),
                        )
                    })?;

                for fc in field_changes {
                    match fc.field.as_str() {
                        "comment" => {
                            comp.comment = fc.new_value.clone();
                        }
                        "footprint" => {
                            comp.pattern = fc.new_value.clone();
                        }
                        "source_library" => {
                            comp.source_library = fc.new_value.clone();
                        }
                        field if field.starts_with("parameter:") => {
                            let param_name = &field["parameter:".len()..];
                            match &fc.new_value {
                                Some(val) => {
                                    comp.parameters.insert(param_name.to_string(), val.clone());
                                }
                                None => {
                                    comp.parameters.shift_remove(param_name);
                                }
                            }
                        }
                        // component_location, rotation, layer: NEVER touched by sync.
                        _ => {}
                    }
                }
            }
            SyncChange::UpdateNet {
                name,
                field_changes,
            } => {
                let net = spec.boards[0]
                    .nets
                    .iter_mut()
                    .find(|n| &n.name == name)
                    .ok_or_else(|| {
                        SpecError::no_span(
                            SpecErrorCode::InvalidFieldAccess,
                            format!(
                                "applying sync change UpdateNet to PcbDoc spec: \
                                 net '{}' not found",
                                name
                            ),
                        )
                    })?;

                for fc in field_changes {
                    match fc.field.as_str() {
                        "net_name" => {
                            if let Some(new_name) = &fc.new_value {
                                net.name = new_name.clone();
                            }
                        }
                        // net_color: excluded in Phase 1 policy (SyncDirection::None).
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    // Phase 3: Adds.
    for change in changes {
        match change {
            SyncChange::AddComponent {
                designator,
                component,
            } => {
                let annotation = Some(crate::annotation::CompiledAnnotation {
                    id: crate::annotation::generate_short_id(),
                    stable: false,
                    group: None,
                    source_id: Some(crate::annotation::generate_source_id(designator)),
                });
                spec.boards[0]
                    .components
                    .push(crate::model::PcbDocComponentSpec {
                        annotation,
                        designator: designator.clone(),
                        pattern: component.footprint.clone(),
                        comment: component.comment.clone(),
                        location: None,
                        rotation: None,
                        layer: None,
                        source_library: component.source_library.clone(),
                        parameters: component.parameters.clone(),
                    });
            }
            SyncChange::AddNet { name, .. } => {
                spec.boards[0].nets.push(crate::model::PcbDocNetSpec {
                    annotation: None,
                    name: name.clone(),
                    color: None,
                    visible: None,
                });
            }
            _ => {}
        }
    }

    Ok(())
}

// ── Spec text rewriter ────────────────────────────────────────────────────────

/// Rewrites a `.pcbdoc-spec` source text to reflect applied `SyncChange`s.
///
/// This is a source-level rewriter that preserves all non-component/net content
/// (geometry, tracks, arcs, vias, pads, polygons, rules, placement blocks, etc.)
/// verbatim. Only `component` and `net` top-level declarations are modified.
///
/// Strategy:
/// 1. Parse the AST to find component/net span boundaries.
/// 2. For removed items: delete their span.
/// 3. For updated items: replace relevant property values within their span.
/// 4. For added items: append new declarations at end of source.
///
/// Returns the rewritten spec text.
pub fn rewrite_pcbdoc_spec_with_changes(
    source: &str,
    changes: &[SyncChange],
) -> Result<String, SpecError> {
    use crate::ast::SpecItem;
    use crate::parser::parse_spec;

    let file = parse_spec(source).map_err(|e| {
        SpecError::no_span(
            SpecErrorCode::ParseError,
            format!("failed to re-parse spec for sync rewrite: {e}"),
        )
    })?;

    // Build maps from the changes for fast lookup.
    let mut removes_comp: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut removes_net: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut updates_comp: std::collections::HashMap<String, &[FieldChange]> =
        std::collections::HashMap::new();
    let mut updates_net: std::collections::HashMap<String, &[FieldChange]> =
        std::collections::HashMap::new();
    let mut adds_comp: Vec<(&str, &SyncComponent)> = Vec::new();
    let mut adds_net: Vec<&str> = Vec::new();

    for change in changes {
        match change {
            SyncChange::RemoveComponent { designator } => {
                removes_comp.insert(designator.clone());
            }
            SyncChange::RemoveNet { name } => {
                removes_net.insert(name.clone());
            }
            SyncChange::UpdateComponent {
                designator,
                field_changes,
            } => {
                updates_comp.insert(designator.clone(), field_changes.as_slice());
            }
            SyncChange::UpdateNet {
                name,
                field_changes,
            } => {
                updates_net.insert(name.clone(), field_changes.as_slice());
            }
            SyncChange::AddComponent {
                designator,
                component,
            } => {
                adds_comp.push((designator.as_str(), component));
            }
            SyncChange::AddNet { name, .. } => {
                adds_net.push(name.as_str());
            }
            _ => {}
        }
    }

    // Collect replacements: (span_start, span_end, replacement_text).
    let mut replacements: Vec<(u32, u32, String)> = Vec::new();

    for item in &file.items {
        match &item.node {
            SpecItem::Component(decl) => {
                let name = decl.name.node.as_str();
                if removes_comp.contains(&name) {
                    collect_remove_replacement(source, item, &mut replacements);
                } else if let Some(field_changes) = updates_comp.get(&name) {
                    collect_component_update_replacement(
                        source,
                        item,
                        decl,
                        field_changes,
                        &mut replacements,
                    )?;
                }
            }
            SpecItem::Net(decl) => {
                let name = decl.name.node.as_str();
                if removes_net.contains(&name) {
                    collect_remove_replacement(source, item, &mut replacements);
                } else if let Some(field_changes) = updates_net.get(&name) {
                    collect_net_update_replacement(
                        source,
                        item,
                        decl,
                        field_changes,
                        &mut replacements,
                    );
                }
            }
            _ => {}
        }
    }

    // Build append text for new components and nets.
    let mut append_text = String::new();
    for (designator, comp) in &adds_comp {
        append_text.push_str(&format_new_component(designator, comp));
    }
    for net_name in &adds_net {
        append_text.push_str(&format_new_net(net_name));
    }

    // Sort replacements in reverse byte order and apply.
    replacements.sort_by(|a, b| b.0.cmp(&a.0));

    let mut output = source.to_owned();
    for (start, end, replacement) in replacements {
        output.replace_range(start as usize..end as usize, &replacement);
    }

    // Append new blocks at end of file.
    if !append_text.is_empty() {
        if !output.ends_with("\n\n") {
            if !output.ends_with('\n') {
                output.push('\n');
            }
            output.push('\n');
        }
        output.push_str(&append_text);
    }

    Ok(output)
}

/// Appends a span-deletion replacement (removes the item and its trailing newline).
fn collect_remove_replacement<T>(
    source: &str,
    item: &crate::diagnostic::Spanned<T>,
    replacements: &mut Vec<(u32, u32, String)>,
) {
    let end = consume_trailing_newline(source, item.span.end);
    replacements.push((item.span.start, end, String::new()));
}

/// Builds and appends a span replacement that rewrites a component declaration with updated fields.
fn collect_component_update_replacement(
    source: &str,
    item: &crate::diagnostic::Spanned<crate::ast::SpecItem>,
    decl: &crate::ast::ComponentDecl,
    field_changes: &[FieldChange],
    replacements: &mut Vec<(u32, u32, String)>,
) -> Result<(), SpecError> {
    use crate::ast::ComponentItem;

    // Collect existing properties from ComponentItem::Property entries.
    let mut props: Vec<(String, String)> = Vec::new();
    for body_item in &decl.body {
        if let ComponentItem::Property(prop) = &body_item.node {
            let key = prop.key.node.clone();
            let val_text = source[prop.value.span.start as usize..prop.value.span.end as usize]
                .trim()
                .to_string();
            props.push((key, val_text));
        }
    }

    // Build override map from field changes.
    let mut overrides: std::collections::HashMap<String, Option<String>> =
        std::collections::HashMap::new();
    for fc in field_changes {
        match fc.field.as_str() {
            "comment" => {
                overrides.insert("comment".to_string(), fc.new_value.clone());
            }
            "footprint" => {
                overrides.insert("pattern".to_string(), fc.new_value.clone());
            }
            "source_library" => {
                overrides.insert("source_library".to_string(), fc.new_value.clone());
            }
            field if field.starts_with("parameter:") => {
                // Parameter sync is handled at the model level; the text rewriter
                // ignores parameter fields since they are not stored as spec properties.
            }
            _ => {}
        }
    }

    // Rebuild properties: preserve all existing, apply overrides.
    let mut new_props: Vec<String> = Vec::new();
    let mut seen_keys: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (key, orig_val) in &props {
        seen_keys.insert(key.clone());
        if let Some(new_val) = overrides.get(key) {
            if let Some(v) = new_val {
                new_props.push(format!("{}: {}", key, quote_spec_string(v)));
            }
            // None means remove the property — omit it.
        } else {
            new_props.push(format!("{}: {}", key, orig_val));
        }
    }
    // Append new properties not previously present.
    for (key, new_val) in &overrides {
        if !seen_keys.contains(key) {
            if let Some(v) = new_val {
                new_props.push(format!("{}: {}", key, quote_spec_string(v)));
            }
        }
    }

    let name = decl.name.node.as_str();
    let name_str = quote_spec_entity_name(&name);
    let orig = &source[item.span.start as usize..item.span.end as usize];
    let has_trailing_nl = orig.ends_with('\n');

    // Preserve annotation if present.
    // When the annotation sits on the same line as the `component` keyword we must
    // extend the replacement span back to the start of that line so that the original
    // annotation text is removed together with the old component body.
    let (replacement_start, annotation_prefix) = if let Some(ann) = &decl.annotation {
        let ann_start = ann.span.start as usize;
        let ann_end = ann.span.end as usize;
        // Walk back to the start of the annotation line.
        let line_start = source[..ann_start].rfind('\n').map(|p| p + 1).unwrap_or(0);
        // Emit the annotation token plus whatever whitespace follows it (up to the
        // component keyword), so the rebuilt declaration is properly formatted.
        let prefix = source[line_start..ann_end].to_string() + " ";
        (line_start as u32, prefix)
    } else {
        (item.span.start, String::new())
    };

    let body_str = if new_props.is_empty() {
        format!("component {} {{}}", name_str)
    } else {
        format!("component {} {{ {} }}", name_str, new_props.join(", "))
    };

    let mut result = annotation_prefix;
    result.push_str(&body_str);
    if has_trailing_nl {
        result.push('\n');
    }

    replacements.push((replacement_start, item.span.end, result));
    Ok(())
}

/// Builds and appends a span replacement that rewrites a net declaration with updated fields.
fn collect_net_update_replacement(
    source: &str,
    item: &crate::diagnostic::Spanned<crate::ast::SpecItem>,
    decl: &crate::ast::NetDecl,
    field_changes: &[FieldChange],
    replacements: &mut Vec<(u32, u32, String)>,
) {
    let mut new_name = decl.name.node.as_str().to_string();
    for fc in field_changes {
        if fc.field == "net_name" {
            if let Some(n) = &fc.new_value {
                new_name = n.clone();
            }
        }
    }

    // Preserve body verbatim.
    let body_start = decl.body.span.start as usize;
    let body_end = decl.body.span.end as usize;
    let body_text = &source[body_start..body_end.min(source.len())];

    let orig = &source[item.span.start as usize..item.span.end as usize];
    let has_trailing_nl = orig.ends_with('\n');

    let name_str = quote_spec_entity_name(&new_name);
    let mut result = format!("net {} {}", name_str, body_text);
    if has_trailing_nl {
        result.push('\n');
    }

    replacements.push((item.span.start, item.span.end, result));
}

/// Consumes a trailing newline after `pos` in the source, returning the new end position.
fn consume_trailing_newline(source: &str, pos: u32) -> u32 {
    let pos = pos as usize;
    let bytes = source.as_bytes();
    if pos < bytes.len() && bytes[pos] == b'\n' {
        (pos + 1) as u32
    } else {
        pos as u32
    }
}

/// Quote a string value for use as a spec property value.
pub fn quote_spec_string(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Quote an entity name (identifier or quoted string) for use in spec declarations.
pub fn quote_spec_entity_name(name: &str) -> String {
    // Use a quoted string if the name contains spaces or special characters.
    if name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.')
        && !name.is_empty()
    {
        name.to_string()
    } else {
        format!("\"{}\"", name.replace('\\', "\\\\").replace('"', "\\\""))
    }
}

/// Formats a new component declaration for appending.
fn format_new_component(designator: &str, comp: &SyncComponent) -> String {
    let id = crate::annotation::generate_short_id();
    let source_id = crate::annotation::generate_source_id(designator);
    let annotation_line = format!(
        "#[annotation(id = \"{}\", source_id = \"{}\")]\n",
        id, source_id
    );

    let mut props: Vec<String> = Vec::new();

    if let Some(pattern) = &comp.footprint {
        props.push(format!("pattern: {}", quote_spec_string(pattern)));
    }
    if let Some(comment) = &comp.comment {
        props.push(format!("comment: {}", quote_spec_string(comment)));
    }
    if let Some(lib) = &comp.source_library {
        props.push(format!("source_library: {}", quote_spec_string(lib)));
    }

    let name_str = quote_spec_entity_name(designator);
    let body = if props.is_empty() {
        format!("component {} {{}}\n", name_str)
    } else {
        format!("component {} {{ {} }}\n", name_str, props.join(", "))
    };
    format!("{}{}", annotation_line, body)
}

/// Formats a new net declaration for appending.
fn format_new_net(name: &str) -> String {
    let name_str = quote_spec_entity_name(name);
    format!("net {} {{}}\n", name_str)
}

// ── ECO report ────────────────────────────────────────────────────────────────

/// Renders a human-readable ECO-style report of sync changes.
///
/// # Precondition
///
/// `changes` MUST have been filtered through [`filter_changes`] before being
/// passed to this function.  The diff algorithm can produce `AddPin`,
/// `RemovePin`, and `UpdatePin` variants when diffing SchDoc specs that carry
/// net-pin assignments; `render_eco_report` treats those variants as
/// unreachable and will panic if any are present.  `filter_changes` rejects
/// pin-level changes with a hard error (pin sync is not supported for PcbDoc),
/// so calling `filter_changes` first guarantees a pin-free change list.
pub fn render_eco_report(changes: &[SyncChange]) -> String {
    if changes.is_empty() {
        return "No changes.\n".to_string();
    }

    let mut out = String::new();
    let mut component_adds = 0usize;
    let mut component_removes = 0usize;
    let mut component_updates = 0usize;
    let mut net_adds = 0usize;
    let mut net_removes = 0usize;
    let mut net_updates = 0usize;

    for change in changes {
        match change {
            SyncChange::AddComponent {
                designator,
                component,
            } => {
                component_adds += 1;
                out.push_str(&format!(
                    "  + component {} (pattern: {})\n",
                    designator,
                    component.footprint.as_deref().unwrap_or("(none)")
                ));
            }
            SyncChange::RemoveComponent { designator } => {
                component_removes += 1;
                out.push_str(&format!("  - component {}\n", designator));
            }
            SyncChange::UpdateComponent {
                designator,
                field_changes,
            } => {
                component_updates += 1;
                out.push_str(&format!("  ~ component {}\n", designator));
                for fc in field_changes {
                    out.push_str(&format!(
                        "      {} : {} -> {}\n",
                        fc.field,
                        fc.old_value.as_deref().unwrap_or("(none)"),
                        fc.new_value.as_deref().unwrap_or("(none)")
                    ));
                }
            }
            SyncChange::AddNet { name, .. } => {
                net_adds += 1;
                out.push_str(&format!("  + net {}\n", name));
            }
            SyncChange::RemoveNet { name } => {
                net_removes += 1;
                out.push_str(&format!("  - net {}\n", name));
            }
            SyncChange::UpdateNet {
                name,
                field_changes,
            } => {
                net_updates += 1;
                out.push_str(&format!("  ~ net {}\n", name));
                for fc in field_changes {
                    out.push_str(&format!(
                        "      {} : {} -> {}\n",
                        fc.field,
                        fc.old_value.as_deref().unwrap_or("(none)"),
                        fc.new_value.as_deref().unwrap_or("(none)")
                    ));
                }
            }
            SyncChange::AddPin { .. }
            | SyncChange::RemovePin { .. }
            | SyncChange::UpdatePin { .. } => {
                unreachable!("pin changes should have been filtered by filter_changes")
            }
        }
    }

    let total = component_adds
        + component_removes
        + component_updates
        + net_adds
        + net_removes
        + net_updates;
    let mut header = format!(
        "ECO ({} change{}):\n",
        total,
        if total == 1 { "" } else { "s" }
    );
    header.push_str(&out);
    header
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        BoardSpec, ComponentSpec, FootprintMapSpec, NetSpec, PcbDocComponentSpec, PcbDocNetSpec,
        PcbDocSpec, PinConnectionSpec, PinConnectionTarget, PinRef, PowerSpec, SchDocComponentSpec,
        SchDocSpec, SheetSpec, SymbolRef,
    };
    use altium_format_types::sch::PowerObjectStyle;
    use altium_format_types::{CoordPoint, RotationBy90};

    fn make_schdoc_component(designator: &str) -> SchDocComponentSpec {
        SchDocComponentSpec {
            annotation: None,
            designator: designator.to_string(),
            symbol: SymbolRef::Literal("TestSymbol".to_string()),
            location: CoordPoint {
                x: altium_format_types::Coord::from_mils_f64(0.0),
                y: altium_format_types::Coord::from_mils_f64(0.0),
            },
            orientation: Some(RotationBy90::Rotate0),
            is_mirrored: None,
            description: None,
            parameters: Vec::new(),
            pin_connections: Vec::new(),
        }
    }

    fn make_empty_sheet() -> SheetSpec {
        SheetSpec {
            annotation: None,
            fonts: Vec::new(),
            power_declarations: std::collections::HashMap::new(),
            custom_width: None,
            custom_height: None,
            snap_grid_on: None,
            visible_grid_on: None,
            hot_spot_grid_on: None,
            show_hidden_pins: None,
            border_on: None,
            title_block_on: None,
            components: Vec::new(),
            nets: Vec::new(),
            powers: Vec::new(),
            objects: Vec::new(),
            constraints: Vec::new(),
        }
    }

    fn make_empty_board(name: &str) -> BoardSpec {
        BoardSpec {
            annotation: None,
            name: name.to_string(),
            signal_layer_count: None,
            snap_grid_size: None,
            visible_grid_size: None,
            display_unit: None,
            nets: Vec::new(),
            components: Vec::new(),
            tracks: Vec::new(),
            arcs: Vec::new(),
            vias: Vec::new(),
            pads: Vec::new(),
            fills: Vec::new(),
            texts: Vec::new(),
            regions: Vec::new(),
            component_bodies: Vec::new(),
            dimensions: Vec::new(),
            polygons: Vec::new(),
            rules: Vec::new(),
            classes: Vec::new(),
            differential_pairs: Vec::new(),
        }
    }

    // ── SchDoc: empty spec → empty snapshot ──────────────────────────────────

    #[test]
    fn test_schdoc_empty_spec() {
        let spec = SchDocSpec { sheets: Vec::new() };
        let snapshot = project_schdoc_spec(&spec, &std::collections::HashMap::new()).unwrap();
        assert!(snapshot.components.is_empty());
        assert!(snapshot.nets.is_empty());
    }

    // ── SchDoc: 2 components, 1 net ──────────────────────────────────────────

    #[test]
    fn test_schdoc_two_components_one_net() {
        let mut sheet = make_empty_sheet();
        sheet.components.push(make_schdoc_component("R1"));
        sheet.components.push(make_schdoc_component("C1"));
        sheet.nets.push(NetSpec {
            annotation: None,
            name: "VCC".to_string(),
            pins: vec![
                PinRef {
                    component: "R1".to_string(),
                    pin: "1".to_string(),
                },
                PinRef {
                    component: "C1".to_string(),
                    pin: "1".to_string(),
                },
            ],
        });

        let spec = SchDocSpec {
            sheets: vec![sheet],
        };
        let snapshot = project_schdoc_spec(&spec, &std::collections::HashMap::new()).unwrap();

        assert_eq!(snapshot.components.len(), 2);
        assert_eq!(snapshot.nets.len(), 1);

        let r1 = &snapshot.components["R1"];
        assert_eq!(r1.designator, "R1");
        assert_eq!(r1.pins["1"].net.as_deref(), Some("VCC"));

        let c1 = &snapshot.components["C1"];
        assert_eq!(c1.pins["1"].net.as_deref(), Some("VCC"));

        let vcc = &snapshot.nets["VCC"];
        assert_eq!(vcc.pins.len(), 2);
        assert!(vcc.pins.contains(&("R1".to_string(), "1".to_string())));
        assert!(vcc.pins.contains(&("C1".to_string(), "1".to_string())));
    }

    // ── SchDoc: power pins populate component pin.net ────────────────────────

    #[test]
    fn test_schdoc_power_pins() {
        let mut sheet = make_empty_sheet();
        sheet.components.push(make_schdoc_component("U1"));
        sheet.powers.push(PowerSpec {
            annotation: None,
            name: "GND".to_string(),
            style: PowerObjectStyle::Circle,
            pins: vec![PinRef {
                component: "U1".to_string(),
                pin: "GND".to_string(),
            }],
            show_net_name: None,
            orientation: None,
        });

        let spec = SchDocSpec {
            sheets: vec![sheet],
        };
        let snapshot = project_schdoc_spec(&spec, &std::collections::HashMap::new()).unwrap();

        let u1 = &snapshot.components["U1"];
        assert_eq!(u1.pins["GND"].net.as_deref(), Some("GND"));

        let gnd_net = &snapshot.nets["GND"];
        assert_eq!(gnd_net.pins.len(), 1);
    }

    // ── SchDoc: duplicate designators across sheets → error ──────────────────

    #[test]
    fn test_schdoc_duplicate_designator_error() {
        let mut sheet1 = make_empty_sheet();
        sheet1.components.push(make_schdoc_component("R1"));

        let mut sheet2 = make_empty_sheet();
        sheet2.components.push(make_schdoc_component("R1"));

        let spec = SchDocSpec {
            sheets: vec![sheet1, sheet2],
        };
        let err = project_schdoc_spec(&spec, &std::collections::HashMap::new()).unwrap_err();
        assert_eq!(err.code, SpecErrorCode::DuplicateEntity);
        assert!(
            err.message.contains("R1"),
            "error should mention designator: {}",
            err.message
        );
    }

    // ── SchDoc: net references non-existent component → error ─────────────────

    #[test]
    fn test_schdoc_dangling_net_pin_error() {
        let mut sheet = make_empty_sheet();
        sheet.components.push(make_schdoc_component("R1"));
        sheet.nets.push(NetSpec {
            annotation: None,
            name: "VCC".to_string(),
            pins: vec![PinRef {
                component: "MISSING".to_string(),
                pin: "1".to_string(),
            }],
        });

        let spec = SchDocSpec {
            sheets: vec![sheet],
        };
        let err = project_schdoc_spec(&spec, &std::collections::HashMap::new()).unwrap_err();
        assert_eq!(err.code, SpecErrorCode::InvalidFieldAccess);
        assert!(
            err.message.contains("VCC"),
            "error should mention net name: {}",
            err.message
        );
        assert!(
            err.message.contains("MISSING"),
            "error should mention component: {}",
            err.message
        );
    }

    // ── PcbDoc: empty spec → empty snapshot ──────────────────────────────────

    #[test]
    fn test_pcbdoc_empty_spec() {
        let spec = PcbDocSpec {
            boards: Vec::new(),
            placement: None,
            placement_rules: Vec::new(),
        };
        let snapshot = project_pcbdoc_spec(&spec).unwrap();
        assert!(snapshot.components.is_empty());
        assert!(snapshot.nets.is_empty());
    }

    // ── PcbDoc: 3 components, 2 nets ─────────────────────────────────────────

    #[test]
    fn test_pcbdoc_three_components_two_nets() {
        let mut board = make_empty_board("Main");
        for des in ["R1", "C1", "U1"] {
            board.components.push(PcbDocComponentSpec {
                annotation: None,
                designator: des.to_string(),
                pattern: Some("0402".to_string()),
                comment: Some(des.to_string()),
                location: None,
                rotation: None,
                layer: None,
                source_library: None,
                parameters: IndexMap::new(),
            });
        }
        for net_name in ["VCC", "GND"] {
            board.nets.push(PcbDocNetSpec {
                annotation: None,
                name: net_name.to_string(),
                color: None,
                visible: None,
            });
        }

        let spec = PcbDocSpec {
            boards: vec![board],
            placement: None,
            placement_rules: Vec::new(),
        };
        let snapshot = project_pcbdoc_spec(&spec).unwrap();

        assert_eq!(snapshot.components.len(), 3);
        assert_eq!(snapshot.nets.len(), 2);

        for des in ["R1", "C1", "U1"] {
            let comp = &snapshot.components[des];
            assert_eq!(comp.designator, des);
            assert_eq!(comp.footprint.as_deref(), Some("0402"));
            assert!(comp.pins.is_empty(), "PcbDoc pins should be empty");
        }

        for net_name in ["VCC", "GND"] {
            let net = &snapshot.nets[net_name];
            assert_eq!(net.name, net_name);
            assert!(net.pins.is_empty(), "PcbDoc net pins should be empty");
        }
    }

    // ── PcbDoc: duplicate designators → error ────────────────────────────────

    #[test]
    fn test_pcbdoc_duplicate_designator_error() {
        let mut board = make_empty_board("Main");
        for _ in 0..2 {
            board.components.push(PcbDocComponentSpec {
                annotation: None,
                designator: "U1".to_string(),
                pattern: None,
                comment: None,
                location: None,
                rotation: None,
                layer: None,
                source_library: None,
                parameters: IndexMap::new(),
            });
        }

        let spec = PcbDocSpec {
            boards: vec![board],
            placement: None,
            placement_rules: Vec::new(),
        };
        let err = project_pcbdoc_spec(&spec).unwrap_err();
        assert_eq!(err.code, SpecErrorCode::DuplicateEntity);
        assert!(
            err.message.contains("U1"),
            "error should mention designator: {}",
            err.message
        );
    }

    // ── PcbDoc: duplicate net names → error ──────────────────────────────────

    #[test]
    fn test_pcbdoc_duplicate_net_error() {
        let mut board = make_empty_board("Main");
        for _ in 0..2 {
            board.nets.push(PcbDocNetSpec {
                annotation: None,
                name: "VCC".to_string(),
                color: None,
                visible: None,
            });
        }

        let spec = PcbDocSpec {
            boards: vec![board],
            placement: None,
            placement_rules: Vec::new(),
        };
        let err = project_pcbdoc_spec(&spec).unwrap_err();
        assert_eq!(err.code, SpecErrorCode::DuplicateEntity);
        assert!(
            err.message.contains("VCC"),
            "error should mention net name: {}",
            err.message
        );
    }

    // ── Diff helpers ──────────────────────────────────────────────────────────

    fn make_sync_component(designator: &str) -> SyncComponent {
        SyncComponent {
            designator: designator.to_string(),
            comment: None,
            footprint: None,
            source_library: None,
            parameters: IndexMap::new(),
            pins: IndexMap::new(),
            annotation_id: None,
            source_unique_id: None,
        }
    }

    fn make_sync_net(name: &str) -> SyncNet {
        SyncNet {
            name: name.to_string(),
            color: None,
            pins: Vec::new(),
            annotation_id: None,
        }
    }

    fn make_empty_snapshot() -> SyncSnapshot {
        SyncSnapshot::default()
    }

    fn all_forward_policy() -> SyncPolicy {
        SyncPolicy {
            comment: SyncDirection::Forward,
            footprint: SyncDirection::Forward,
            source_library: SyncDirection::Forward,
            parameters: SyncDirection::Forward,
            net_name: SyncDirection::Forward,
            net_color: SyncDirection::Forward,
            pin_net_assignment: SyncDirection::Forward,
            component_location: SyncDirection::Forward,
        }
    }

    // ── Diff: identical snapshots → empty changeset ───────────────────────────

    #[test]
    fn test_diff_identical_snapshots_empty() {
        let mut snapshot = make_empty_snapshot();
        snapshot
            .components
            .insert("R1".to_string(), make_sync_component("R1"));
        snapshot
            .nets
            .insert("VCC".to_string(), make_sync_net("VCC"));

        let changes = diff_snapshots(&snapshot, &snapshot);
        assert!(
            changes.is_empty(),
            "identical snapshots should produce no changes"
        );
    }

    // ── Diff: source has extra component → AddComponent ──────────────────────

    #[test]
    fn test_diff_source_extra_component_add() {
        let mut source = make_empty_snapshot();
        source
            .components
            .insert("R1".to_string(), make_sync_component("R1"));
        source
            .components
            .insert("C1".to_string(), make_sync_component("C1"));

        let mut target = make_empty_snapshot();
        target
            .components
            .insert("R1".to_string(), make_sync_component("R1"));

        let changes = diff_snapshots(&source, &target);
        assert_eq!(changes.len(), 1);
        assert!(
            matches!(&changes[0], SyncChange::AddComponent { designator, .. } if designator == "C1"),
            "expected AddComponent for C1, got: {:?}",
            changes
        );
    }

    // ── Diff: target has extra component → RemoveComponent ───────────────────

    #[test]
    fn test_diff_target_extra_component_remove() {
        let mut source = make_empty_snapshot();
        source
            .components
            .insert("R1".to_string(), make_sync_component("R1"));

        let mut target = make_empty_snapshot();
        target
            .components
            .insert("R1".to_string(), make_sync_component("R1"));
        target
            .components
            .insert("C1".to_string(), make_sync_component("C1"));

        let changes = diff_snapshots(&source, &target);
        assert_eq!(changes.len(), 1);
        assert!(
            matches!(&changes[0], SyncChange::RemoveComponent { designator } if designator == "C1"),
            "expected RemoveComponent for C1, got: {:?}",
            changes
        );
    }

    // ── Diff: same component, different footprint → UpdateComponent ──────────

    #[test]
    fn test_diff_component_footprint_change() {
        let mut src_comp = make_sync_component("R1");
        src_comp.footprint = Some("0402".to_string());

        let mut tgt_comp = make_sync_component("R1");
        tgt_comp.footprint = Some("0603".to_string());

        let mut source = make_empty_snapshot();
        source.components.insert("R1".to_string(), src_comp);

        let mut target = make_empty_snapshot();
        target.components.insert("R1".to_string(), tgt_comp);

        let changes = diff_snapshots(&source, &target);
        assert_eq!(changes.len(), 1);

        match &changes[0] {
            SyncChange::UpdateComponent {
                designator,
                field_changes,
            } => {
                assert_eq!(designator, "R1");
                assert_eq!(field_changes.len(), 1);
                assert_eq!(field_changes[0].field, "footprint");
                assert_eq!(field_changes[0].old_value.as_deref(), Some("0603"));
                assert_eq!(field_changes[0].new_value.as_deref(), Some("0402"));
            }
            other => panic!("expected UpdateComponent, got: {other:?}"),
        }
    }

    // ── Diff: source has extra net → AddNet ───────────────────────────────────

    #[test]
    fn test_diff_source_extra_net_add() {
        let mut source = make_empty_snapshot();
        source.nets.insert("VCC".to_string(), make_sync_net("VCC"));
        source.nets.insert("GND".to_string(), make_sync_net("GND"));

        let mut target = make_empty_snapshot();
        target.nets.insert("VCC".to_string(), make_sync_net("VCC"));

        let changes = diff_snapshots(&source, &target);
        assert_eq!(changes.len(), 1);
        assert!(
            matches!(&changes[0], SyncChange::AddNet { name, .. } if name == "GND"),
            "expected AddNet for GND, got: {:?}",
            changes
        );
    }

    // ── Diff: pin net difference → UpdatePin ─────────────────────────────────

    #[test]
    fn test_diff_pin_net_difference_update_pin() {
        let mut src_comp = make_sync_component("R1");
        src_comp.pins.insert(
            "1".to_string(),
            SyncPin {
                designator: "1".to_string(),
                net: Some("VCC".to_string()),
            },
        );

        let mut tgt_comp = make_sync_component("R1");
        tgt_comp.pins.insert(
            "1".to_string(),
            SyncPin {
                designator: "1".to_string(),
                net: Some("GND".to_string()),
            },
        );

        let mut source = make_empty_snapshot();
        source.components.insert("R1".to_string(), src_comp);

        let mut target = make_empty_snapshot();
        target.components.insert("R1".to_string(), tgt_comp);

        let changes = diff_snapshots(&source, &target);
        assert_eq!(changes.len(), 1);
        assert!(
            matches!(&changes[0], SyncChange::UpdatePin { component_designator, pin_designator, .. }
                if component_designator == "R1" && pin_designator == "1"),
            "expected UpdatePin for R1.1, got: {:?}",
            changes
        );
    }

    // ── Diff: a vs b and b vs a produce inverse-ish changes ──────────────────

    #[test]
    fn test_diff_inverse_symmetry() {
        let mut a = make_empty_snapshot();
        a.components
            .insert("R1".to_string(), make_sync_component("R1"));
        a.nets.insert("VCC".to_string(), make_sync_net("VCC"));

        let mut b = make_empty_snapshot();
        b.components
            .insert("C1".to_string(), make_sync_component("C1"));
        b.nets.insert("GND".to_string(), make_sync_net("GND"));

        let changes_ab = diff_snapshots(&a, &b);
        let changes_ba = diff_snapshots(&b, &a);

        // a→b: add R1, remove C1, add VCC, remove GND
        // b→a: add C1, remove R1, add GND, remove VCC
        assert_eq!(
            changes_ab.len(),
            changes_ba.len(),
            "diff(a,b) and diff(b,a) should have same change count"
        );

        let adds_ab = changes_ab
            .iter()
            .filter(|c| matches!(c, SyncChange::AddComponent { .. }))
            .count();
        let removes_ab = changes_ab
            .iter()
            .filter(|c| matches!(c, SyncChange::RemoveComponent { .. }))
            .count();
        let adds_ba = changes_ba
            .iter()
            .filter(|c| matches!(c, SyncChange::AddComponent { .. }))
            .count();
        let removes_ba = changes_ba
            .iter()
            .filter(|c| matches!(c, SyncChange::RemoveComponent { .. }))
            .count();

        assert_eq!(
            adds_ab, removes_ba,
            "adds in a→b should equal removes in b→a"
        );
        assert_eq!(
            removes_ab, adds_ba,
            "removes in a→b should equal adds in b→a"
        );
    }

    // ── Policy filtering: SyncDirection::None suppresses changes ─────────────

    #[test]
    fn test_filter_none_suppresses_footprint_change() {
        let mut src_comp = make_sync_component("R1");
        src_comp.footprint = Some("0402".to_string());

        let mut tgt_comp = make_sync_component("R1");
        tgt_comp.footprint = Some("0603".to_string());

        let mut source = make_empty_snapshot();
        source.components.insert("R1".to_string(), src_comp);

        let mut target = make_empty_snapshot();
        target.components.insert("R1".to_string(), tgt_comp);

        let changes = diff_snapshots(&source, &target);
        assert!(!changes.is_empty());

        let policy = SyncPolicy {
            comment: SyncDirection::Forward,
            footprint: SyncDirection::None,
            source_library: SyncDirection::None,
            parameters: SyncDirection::None,
            net_name: SyncDirection::None,
            net_color: SyncDirection::None,
            pin_net_assignment: SyncDirection::None,
            component_location: SyncDirection::None,
        };

        let filtered = filter_changes(&changes, &policy, SyncDirection::Forward).unwrap();
        // UpdateComponent with only footprint change filtered → no UpdateComponent remains
        assert!(
            filtered
                .iter()
                .all(|c| !matches!(c, SyncChange::UpdateComponent { .. })),
            "footprint UpdateComponent should be filtered out: {:?}",
            filtered
        );
    }

    // ── Policy filtering: pin changes dropped when pin_net_assignment is None ────

    #[test]
    fn test_filter_drops_add_pin_when_policy_none() {
        let changes = vec![SyncChange::AddPin {
            component_designator: "R1".to_string(),
            pin_designator: "1".to_string(),
            pin: SyncPin {
                designator: "1".to_string(),
                net: None,
            },
        }];

        let policy = SyncPolicy {
            comment: SyncDirection::Forward,
            footprint: SyncDirection::Forward,
            source_library: SyncDirection::Forward,
            parameters: SyncDirection::Forward,
            net_name: SyncDirection::Forward,
            net_color: SyncDirection::Forward,
            pin_net_assignment: SyncDirection::None,
            component_location: SyncDirection::Forward,
        };
        let result = filter_changes(&changes, &policy, SyncDirection::Forward).unwrap();
        assert!(
            result.is_empty(),
            "AddPin should be silently dropped when pin_net_assignment is None"
        );
    }

    #[test]
    fn test_filter_drops_remove_pin_when_policy_none() {
        let changes = vec![SyncChange::RemovePin {
            component_designator: "R1".to_string(),
            pin_designator: "1".to_string(),
        }];

        let policy = SyncPolicy {
            comment: SyncDirection::Forward,
            footprint: SyncDirection::Forward,
            source_library: SyncDirection::Forward,
            parameters: SyncDirection::Forward,
            net_name: SyncDirection::Forward,
            net_color: SyncDirection::Forward,
            pin_net_assignment: SyncDirection::None,
            component_location: SyncDirection::Forward,
        };
        let result = filter_changes(&changes, &policy, SyncDirection::Forward).unwrap();
        assert!(
            result.is_empty(),
            "RemovePin should be silently dropped when pin_net_assignment is None"
        );
    }

    #[test]
    fn test_filter_drops_update_pin_when_policy_none() {
        let changes = vec![SyncChange::UpdatePin {
            component_designator: "R1".to_string(),
            pin_designator: "1".to_string(),
            field_changes: vec![FieldChange {
                field: "net".to_string(),
                old_value: Some("GND".to_string()),
                new_value: Some("VCC".to_string()),
            }],
        }];

        let policy = SyncPolicy {
            comment: SyncDirection::Forward,
            footprint: SyncDirection::Forward,
            source_library: SyncDirection::Forward,
            parameters: SyncDirection::Forward,
            net_name: SyncDirection::Forward,
            net_color: SyncDirection::Forward,
            pin_net_assignment: SyncDirection::None,
            component_location: SyncDirection::Forward,
        };
        let result = filter_changes(&changes, &policy, SyncDirection::Forward).unwrap();
        assert!(
            result.is_empty(),
            "UpdatePin should be silently dropped when pin_net_assignment is None"
        );
    }

    // ── Policy filtering: error on pin variants when pin_net_assignment is non-None ──

    #[test]
    fn test_filter_errors_on_add_pin_when_policy_enabled() {
        let changes = vec![SyncChange::AddPin {
            component_designator: "R1".to_string(),
            pin_designator: "1".to_string(),
            pin: SyncPin {
                designator: "1".to_string(),
                net: None,
            },
        }];

        let policy = all_forward_policy();
        let result = filter_changes(&changes, &policy, SyncDirection::Forward);
        assert!(
            result.is_err(),
            "filter_changes should error on AddPin when pin_net_assignment is Forward"
        );
        let err = result.unwrap_err();
        assert!(
            err.message.contains("pin-level sync is not supported"),
            "error message should mention pin-level sync: {}",
            err.message
        );
    }

    #[test]
    fn test_filter_errors_on_remove_pin_when_policy_enabled() {
        let changes = vec![SyncChange::RemovePin {
            component_designator: "R1".to_string(),
            pin_designator: "1".to_string(),
        }];

        let policy = all_forward_policy();
        let result = filter_changes(&changes, &policy, SyncDirection::Forward);
        assert!(
            result.is_err(),
            "filter_changes should error on RemovePin when pin_net_assignment is Forward"
        );
    }

    #[test]
    fn test_filter_errors_on_update_pin_when_policy_enabled() {
        let changes = vec![SyncChange::UpdatePin {
            component_designator: "R1".to_string(),
            pin_designator: "1".to_string(),
            field_changes: vec![FieldChange {
                field: "net".to_string(),
                old_value: Some("GND".to_string()),
                new_value: Some("VCC".to_string()),
            }],
        }];

        let policy = all_forward_policy();
        let result = filter_changes(&changes, &policy, SyncDirection::Forward);
        assert!(
            result.is_err(),
            "filter_changes should error on UpdatePin when pin_net_assignment is Forward"
        );
    }

    // ── apply: empty boards → error ───────────────────────────────────────────

    #[test]
    fn test_apply_empty_boards_error() {
        let changes = vec![];
        let mut spec = PcbDocSpec {
            boards: Vec::new(),
            placement: None,
            placement_rules: Vec::new(),
        };
        let err = apply_sync_changes_to_pcbdoc(&changes, &mut spec).unwrap_err();
        assert!(
            err.message.contains("at least one board block"),
            "error must mention board block: {}",
            err.message
        );
    }

    // ── apply: multi-board → error ────────────────────────────────────────────

    #[test]
    fn test_apply_multi_board_error() {
        let changes = vec![];
        let mut spec = PcbDocSpec {
            boards: vec![make_empty_board("A"), make_empty_board("B")],
            placement: None,
            placement_rules: Vec::new(),
        };
        let err = apply_sync_changes_to_pcbdoc(&changes, &mut spec).unwrap_err();
        assert!(
            err.message.contains("not supported"),
            "error must mention not supported: {}",
            err.message
        );
    }

    // ── apply: AddComponent appends component ─────────────────────────────────

    #[test]
    fn test_apply_add_component() {
        let mut spec = PcbDocSpec {
            boards: vec![make_empty_board("Main")],
            placement: None,
            placement_rules: Vec::new(),
        };

        let new_comp = SyncComponent {
            designator: "R1".to_string(),
            comment: Some("100R".to_string()),
            footprint: Some("0402".to_string()),
            source_library: None,
            parameters: IndexMap::new(),
            pins: IndexMap::new(),
            annotation_id: None,
            source_unique_id: None,
        };

        let changes = vec![SyncChange::AddComponent {
            designator: "R1".to_string(),
            component: new_comp,
        }];

        apply_sync_changes_to_pcbdoc(&changes, &mut spec).unwrap();

        assert_eq!(spec.boards[0].components.len(), 1);
        let comp = &spec.boards[0].components[0];
        assert_eq!(comp.designator, "R1");
        assert_eq!(comp.comment.as_deref(), Some("100R"));
        assert_eq!(comp.pattern.as_deref(), Some("0402"));
        // Location/rotation/layer must be None (never set by sync).
        assert!(comp.location.is_none(), "location must not be set by sync");
        assert!(comp.rotation.is_none(), "rotation must not be set by sync");
        assert!(comp.layer.is_none(), "layer must not be set by sync");
    }

    // ── apply: RemoveComponent removes component ──────────────────────────────

    #[test]
    fn test_apply_remove_component() {
        let mut board = make_empty_board("Main");
        board.components.push(PcbDocComponentSpec {
            annotation: None,
            designator: "R1".to_string(),
            pattern: Some("0402".to_string()),
            comment: None,
            location: None,
            rotation: None,
            layer: None,
            source_library: None,
            parameters: indexmap::IndexMap::new(),
        });
        board.components.push(PcbDocComponentSpec {
            annotation: None,
            designator: "C1".to_string(),
            pattern: Some("0603".to_string()),
            comment: None,
            location: None,
            rotation: None,
            layer: None,
            source_library: None,
            parameters: indexmap::IndexMap::new(),
        });

        let mut spec = PcbDocSpec {
            boards: vec![board],
            placement: None,
            placement_rules: Vec::new(),
        };

        let changes = vec![SyncChange::RemoveComponent {
            designator: "R1".to_string(),
        }];
        apply_sync_changes_to_pcbdoc(&changes, &mut spec).unwrap();

        assert_eq!(spec.boards[0].components.len(), 1);
        assert_eq!(spec.boards[0].components[0].designator, "C1");
    }

    // ── apply: UpdateComponent updates fields, preserves location ─────────────

    #[test]
    fn test_apply_update_component_preserves_location() {
        use altium_format_types::CoordPoint;
        let fixed_location = CoordPoint {
            x: altium_format_types::Coord::from_mils_f64(100.0),
            y: altium_format_types::Coord::from_mils_f64(200.0),
        };

        let mut board = make_empty_board("Main");
        board.components.push(PcbDocComponentSpec {
            annotation: None,
            designator: "R1".to_string(),
            pattern: Some("0402".to_string()),
            comment: Some("original".to_string()),
            location: Some(fixed_location),
            rotation: Some(90.0),
            layer: None,
            source_library: None,
            parameters: indexmap::IndexMap::new(),
        });

        let mut spec = PcbDocSpec {
            boards: vec![board],
            placement: None,
            placement_rules: Vec::new(),
        };

        let changes = vec![SyncChange::UpdateComponent {
            designator: "R1".to_string(),
            field_changes: vec![
                FieldChange {
                    field: "comment".to_string(),
                    old_value: Some("original".to_string()),
                    new_value: Some("updated".to_string()),
                },
                FieldChange {
                    field: "footprint".to_string(),
                    old_value: Some("0402".to_string()),
                    new_value: Some("0603".to_string()),
                },
            ],
        }];

        apply_sync_changes_to_pcbdoc(&changes, &mut spec).unwrap();

        let comp = &spec.boards[0].components[0];
        assert_eq!(comp.comment.as_deref(), Some("updated"));
        assert_eq!(comp.pattern.as_deref(), Some("0603"));
        // Location and rotation MUST NOT be modified.
        assert!(comp.location.is_some(), "location must be preserved");
        assert_eq!(comp.rotation, Some(90.0), "rotation must be preserved");
    }

    // ── apply: AddNet appends net ─────────────────────────────────────────────

    #[test]
    fn test_apply_add_net() {
        let mut spec = PcbDocSpec {
            boards: vec![make_empty_board("Main")],
            placement: None,
            placement_rules: Vec::new(),
        };

        let changes = vec![SyncChange::AddNet {
            name: "VCC".to_string(),
            net: make_sync_net("VCC"),
        }];

        apply_sync_changes_to_pcbdoc(&changes, &mut spec).unwrap();

        assert_eq!(spec.boards[0].nets.len(), 1);
        assert_eq!(spec.boards[0].nets[0].name, "VCC");
        assert!(spec.boards[0].nets[0].color.is_none(), "color must be None");
    }

    // ── apply: RemoveNet removes net ──────────────────────────────────────────

    #[test]
    fn test_apply_remove_net() {
        let mut board = make_empty_board("Main");
        board.nets.push(PcbDocNetSpec {
            annotation: None,
            name: "VCC".to_string(),
            color: None,
            visible: None,
        });
        board.nets.push(PcbDocNetSpec {
            annotation: None,
            name: "GND".to_string(),
            color: None,
            visible: None,
        });

        let mut spec = PcbDocSpec {
            boards: vec![board],
            placement: None,
            placement_rules: Vec::new(),
        };

        let changes = vec![SyncChange::RemoveNet {
            name: "VCC".to_string(),
        }];
        apply_sync_changes_to_pcbdoc(&changes, &mut spec).unwrap();

        assert_eq!(spec.boards[0].nets.len(), 1);
        assert_eq!(spec.boards[0].nets[0].name, "GND");
    }

    // ── apply: pin changes → hard error ──────────────────────────────────────

    #[test]
    fn test_apply_pin_change_hard_error() {
        let mut spec = PcbDocSpec {
            boards: vec![make_empty_board("Main")],
            placement: None,
            placement_rules: Vec::new(),
        };

        let changes = vec![SyncChange::AddPin {
            component_designator: "R1".to_string(),
            pin_designator: "1".to_string(),
            pin: SyncPin {
                designator: "1".to_string(),
                net: None,
            },
        }];

        let err = apply_sync_changes_to_pcbdoc(&changes, &mut spec).unwrap_err();
        assert!(
            err.message.contains("pin-level sync not supported"),
            "error must mention pin-level sync: {}",
            err.message
        );
    }

    // ── rewrite: UpdateComponent on annotated component ──────────────────────

    #[test]
    fn test_rewrite_update_component_annotated() {
        let source = "#[annotation(id = \"ABCD1234\")] component R1 { comment: \"old\" }\n";
        let changes = vec![SyncChange::UpdateComponent {
            designator: "R1".to_string(),
            field_changes: vec![FieldChange {
                field: "comment".to_string(),
                old_value: Some("old".to_string()),
                new_value: Some("new").map(str::to_string),
            }],
        }];
        let result = rewrite_pcbdoc_spec_with_changes(source, &changes).unwrap();

        // Exactly one annotation line.
        let annotation_count = result
            .lines()
            .filter(|l| l.contains("#[annotation"))
            .count();
        assert_eq!(
            annotation_count, 1,
            "output must have exactly one annotation line, got:\n{result}"
        );

        // Updated field value is present.
        assert!(
            result.contains("\"new\""),
            "output must contain updated comment value, got:\n{result}"
        );
        // Old value is gone.
        assert!(
            !result.contains("\"old\""),
            "output must not contain old comment value, got:\n{result}"
        );

        // Re-parses cleanly.
        crate::parser::parse_spec(&result)
            .unwrap_or_else(|e| panic!("rewritten spec does not parse: {e}\n---\n{result}"));
    }

    // ── rewrite: RemoveComponent on annotated component removes annotation ────

    #[test]
    fn test_rewrite_remove_component_annotated() {
        let source = concat!(
            "#[annotation(id = \"ABCD1234\")] component R1 { comment: \"keep\" }\n",
            "component C1 {}\n",
        );
        let changes = vec![SyncChange::RemoveComponent {
            designator: "R1".to_string(),
        }];
        let result = rewrite_pcbdoc_spec_with_changes(source, &changes).unwrap();

        // R1 and its annotation must be gone.
        assert!(
            !result.contains("R1"),
            "removed component R1 must not appear in output, got:\n{result}"
        );
        assert!(
            !result.contains("#[annotation(id = \"ABCD1234\")]"),
            "annotation for removed component must not appear in output, got:\n{result}"
        );

        // C1 must remain.
        assert!(
            result.contains("C1"),
            "non-removed component C1 must remain in output, got:\n{result}"
        );

        // Re-parses cleanly.
        crate::parser::parse_spec(&result)
            .unwrap_or_else(|e| panic!("rewritten spec does not parse: {e}\n---\n{result}"));
    }

    // ── rewrite: AddComponent appends a new component declaration ─────────────

    #[test]
    fn test_rewrite_add_component() {
        let source = "component R1 {}\n";
        let new_comp = SyncComponent {
            designator: "C1".to_string(),
            comment: Some("10uF".to_string()),
            footprint: Some("0805".to_string()),
            source_library: None,
            parameters: IndexMap::new(),
            pins: IndexMap::new(),
            annotation_id: None,
            source_unique_id: None,
        };
        let changes = vec![SyncChange::AddComponent {
            designator: "C1".to_string(),
            component: new_comp,
        }];
        let result = rewrite_pcbdoc_spec_with_changes(source, &changes).unwrap();

        // New component appears.
        assert!(
            result.contains("C1"),
            "added component C1 must appear in output, got:\n{result}"
        );
        // Existing component preserved.
        assert!(
            result.contains("R1"),
            "existing component R1 must remain in output, got:\n{result}"
        );

        // Re-parses cleanly.
        crate::parser::parse_spec(&result)
            .unwrap_or_else(|e| panic!("rewritten spec does not parse: {e}\n---\n{result}"));
    }

    // ── rewrite: UpdateNet changes net color field ────────────────────────────

    #[test]
    fn test_rewrite_update_net_name() {
        let source = "net VCC {}\n";
        let changes = vec![SyncChange::UpdateNet {
            name: "VCC".to_string(),
            field_changes: vec![FieldChange {
                field: "net_name".to_string(),
                old_value: Some("VCC".to_string()),
                new_value: Some("POWER".to_string()),
            }],
        }];
        let result = rewrite_pcbdoc_spec_with_changes(source, &changes).unwrap();

        // New net name appears.
        assert!(
            result.contains("POWER"),
            "updated net name POWER must appear in output, got:\n{result}"
        );

        // Re-parses cleanly.
        crate::parser::parse_spec(&result)
            .unwrap_or_else(|e| panic!("rewritten spec does not parse: {e}\n---\n{result}"));
    }

    // ── apply: idempotency — apply, then diff again → empty changeset ─────────

    #[test]
    fn test_apply_idempotency() {
        // Build a SchDoc snapshot with R1 and VCC.
        let mut schdoc_snapshot = SyncSnapshot::default();
        schdoc_snapshot.components.insert(
            "R1".to_string(),
            SyncComponent {
                designator: "R1".to_string(),
                comment: Some("100R".to_string()),
                footprint: None, // SchDoc has no footprint
                source_library: None,
                parameters: IndexMap::new(),
                pins: IndexMap::new(),
                annotation_id: None,
                source_unique_id: None,
            },
        );
        schdoc_snapshot
            .nets
            .insert("VCC".to_string(), make_sync_net("VCC"));

        // Build a PcbDoc spec with C1 (extra) and no VCC net.
        let mut board = make_empty_board("Main");
        board.components.push(PcbDocComponentSpec {
            annotation: None,
            designator: "C1".to_string(),
            pattern: None,
            comment: None,
            location: None,
            rotation: None,
            layer: None,
            source_library: None,
            parameters: indexmap::IndexMap::new(),
        });
        let mut spec = PcbDocSpec {
            boards: vec![board],
            placement: None,
            placement_rules: Vec::new(),
        };

        // First sync: forward.
        let policy = SyncPolicy {
            comment: SyncDirection::Forward,
            footprint: SyncDirection::None,
            source_library: SyncDirection::None,
            parameters: SyncDirection::Forward,
            net_name: SyncDirection::Forward,
            net_color: SyncDirection::None,
            pin_net_assignment: SyncDirection::None,
            component_location: SyncDirection::None,
        };

        let pcbdoc_snapshot = project_pcbdoc_spec(&spec).unwrap();
        let changes = diff_snapshots(&schdoc_snapshot, &pcbdoc_snapshot);
        let filtered = filter_changes(&changes, &policy, SyncDirection::Forward).unwrap();
        apply_sync_changes_to_pcbdoc(&filtered, &mut spec).unwrap();

        // Second sync: diff again should produce no changes.
        let pcbdoc_snapshot2 = project_pcbdoc_spec(&spec).unwrap();
        let changes2 = diff_snapshots(&schdoc_snapshot, &pcbdoc_snapshot2);
        let filtered2 = filter_changes(&changes2, &policy, SyncDirection::Forward).unwrap();

        assert!(
            filtered2.is_empty(),
            "second sync should produce no changes (idempotency): {:?}",
            filtered2
        );
    }

    // ── Footprint resolution: Import symbol → footprint populated ─────────────

    #[test]
    fn test_schdoc_footprint_resolution() {
        let mut sheet = make_empty_sheet();
        let mut comp = make_schdoc_component("U1");
        comp.symbol = SymbolRef::Import {
            alias: "mcu".to_string(),
            name: "ESP32_C6".to_string(),
        };
        sheet.components.push(comp);

        let spec = SchDocSpec {
            sheets: vec![sheet],
        };

        let mut imported = std::collections::HashMap::new();
        imported.insert(
            "ESP32_C6".to_string(),
            ComponentSpec {
                annotation: None,
                lib_reference: "ESP32_C6".to_string(),
                designator: None,
                description: None,
                component_kind: None,
                part_count: None,
                show_hidden_pins: None,
                pins: Vec::new(),
                parameters: Vec::new(),
                aliases: Vec::new(),
                footprints: vec![FootprintMapSpec {
                    model_name: "QFN-48".to_string(),
                    maps: Vec::new(),
                    source: None,
                }],
                graphics: Vec::new(),
                parts: Vec::new(),
            },
        );

        let snapshot = project_schdoc_spec(&spec, &imported).unwrap();
        let u1 = &snapshot.components["U1"];
        assert_eq!(
            u1.footprint.as_deref(),
            Some("QFN-48"),
            "footprint should be resolved from imported SchLib"
        );
        assert_eq!(
            u1.source_library.as_deref(),
            Some("ESP32_C6"),
            "source_library should be set"
        );
    }

    // ── Footprint resolution: Literal symbol → footprint populated ─────────────

    #[test]
    fn test_schdoc_literal_symbol_footprint_resolution() {
        let mut sheet = make_empty_sheet();
        let mut comp = make_schdoc_component("R1");
        comp.symbol = SymbolRef::Literal("RESISTOR".to_string());
        sheet.components.push(comp);

        let spec = SchDocSpec {
            sheets: vec![sheet],
        };

        let mut imported = std::collections::HashMap::new();
        imported.insert(
            "RESISTOR".to_string(),
            ComponentSpec {
                annotation: None,
                lib_reference: "RESISTOR".to_string(),
                designator: None,
                description: None,
                component_kind: None,
                part_count: None,
                show_hidden_pins: None,
                pins: Vec::new(),
                parameters: Vec::new(),
                aliases: Vec::new(),
                footprints: vec![FootprintMapSpec {
                    model_name: "0603".to_string(),
                    maps: Vec::new(),
                    source: None,
                }],
                graphics: Vec::new(),
                parts: Vec::new(),
            },
        );

        let snapshot = project_schdoc_spec(&spec, &imported).unwrap();
        let r1 = &snapshot.components["R1"];
        assert_eq!(r1.footprint.as_deref(), Some("0603"));
    }

    // ── Footprint resolution: empty imported_components → footprint None ──────

    #[test]
    fn test_schdoc_no_imported_components_footprint_none() {
        let mut sheet = make_empty_sheet();
        sheet.components.push(make_schdoc_component("R1"));

        let spec = SchDocSpec {
            sheets: vec![sheet],
        };
        let imported = std::collections::HashMap::new();

        let snapshot = project_schdoc_spec(&spec, &imported).unwrap();
        let r1 = &snapshot.components["R1"];
        assert_eq!(
            r1.footprint, None,
            "footprint should be None when no imported components"
        );
    }

    // ── Diff: detects footprint difference ───────────────────────────────────

    #[test]
    fn test_diff_includes_footprint_changes() {
        let mut source = SyncSnapshot::default();
        source.components.insert(
            "U1".to_string(),
            SyncComponent {
                designator: "U1".to_string(),
                comment: None,
                footprint: Some("QFN-48".to_string()),
                source_library: None,
                parameters: IndexMap::new(),
                pins: IndexMap::new(),
                annotation_id: None,
                source_unique_id: None,
            },
        );

        let mut target = SyncSnapshot::default();
        target.components.insert(
            "U1".to_string(),
            SyncComponent {
                designator: "U1".to_string(),
                comment: None,
                footprint: None,
                source_library: None,
                parameters: IndexMap::new(),
                pins: IndexMap::new(),
                annotation_id: None,
                source_unique_id: None,
            },
        );

        let changes = diff_snapshots(&source, &target);
        assert_eq!(changes.len(), 1);
        match &changes[0] {
            SyncChange::UpdateComponent {
                designator,
                field_changes,
            } => {
                assert_eq!(designator, "U1");
                assert!(
                    field_changes
                        .iter()
                        .any(|fc| fc.field == "footprint"
                            && fc.new_value.as_deref() == Some("QFN-48"))
                );
            }
            _ => panic!("expected UpdateComponent"),
        }
    }

    // ── Policy: Forward policy includes footprint changes ────────────────────

    #[test]
    fn test_forward_sync_policy_includes_footprint() {
        let changes = vec![SyncChange::UpdateComponent {
            designator: "U1".to_string(),
            field_changes: vec![FieldChange {
                field: "footprint".to_string(),
                old_value: None,
                new_value: Some("QFN-48".to_string()),
            }],
        }];

        let policy = SyncPolicy {
            comment: SyncDirection::Forward,
            footprint: SyncDirection::Forward,
            source_library: SyncDirection::Forward,
            parameters: SyncDirection::None,
            net_name: SyncDirection::Forward,
            net_color: SyncDirection::None,
            pin_net_assignment: SyncDirection::None,
            component_location: SyncDirection::None,
        };

        let filtered = filter_changes(&changes, &policy, SyncDirection::Forward).unwrap();
        assert_eq!(filtered.len(), 1);
        match &filtered[0] {
            SyncChange::UpdateComponent { field_changes, .. } => {
                assert!(field_changes.iter().any(|fc| fc.field == "footprint"));
            }
            _ => panic!("expected UpdateComponent"),
        }
    }

    // ── Rewrite: AddComponent preserves footprint ─────────────────────────────

    #[test]
    fn test_add_component_includes_footprint_in_rewrite() {
        let source = "board \"test\" { signal_layer_count: 2 }\n";
        let changes = vec![SyncChange::AddComponent {
            designator: "U1".to_string(),
            component: SyncComponent {
                designator: "U1".to_string(),
                comment: Some("MCU".to_string()),
                footprint: Some("QFN-48".to_string()),
                source_library: Some("mcu".to_string()),
                parameters: IndexMap::new(),
                pins: IndexMap::new(),
                annotation_id: None,
                source_unique_id: None,
            },
        }];

        let result = rewrite_pcbdoc_spec_with_changes(source, &changes).unwrap();
        assert!(
            result.contains("pattern: \"QFN-48\""),
            "rewritten spec should contain pattern: {:?}",
            result
        );
        assert!(
            result.contains("comment: \"MCU\""),
            "rewritten spec should contain comment: {:?}",
            result
        );
        assert!(
            result.contains("source_library: \"mcu\""),
            "rewritten spec should contain source_library: {:?}",
            result
        );
    }

    // ── Pin connections: signal and power nets extracted from pin_connections ──

    #[test]
    fn test_schdoc_pin_connections_create_nets() {
        let mut sheet = make_empty_sheet();
        sheet
            .power_declarations
            .insert("GND".to_string(), PowerObjectStyle::Circle);

        let mut comp = make_schdoc_component("U1");
        comp.pin_connections = vec![
            PinConnectionSpec {
                pin_name: "GPIO4".to_string(),
                target: PinConnectionTarget::Signal("SDA".to_string()),
            },
            PinConnectionSpec {
                pin_name: "GPIO5".to_string(),
                target: PinConnectionTarget::Signal("SCL".to_string()),
            },
            PinConnectionSpec {
                pin_name: "VDD".to_string(),
                target: PinConnectionTarget::Power("VCC".to_string()),
            },
            PinConnectionSpec {
                pin_name: "GND".to_string(),
                target: PinConnectionTarget::Power("GND".to_string()),
            },
            PinConnectionSpec {
                pin_name: "NC1".to_string(),
                target: PinConnectionTarget::NoConnect,
            },
        ];
        sheet.components.push(comp);

        let spec = SchDocSpec {
            sheets: vec![sheet],
        };
        let imported = std::collections::HashMap::new();
        let snapshot = project_schdoc_spec(&spec, &imported).unwrap();

        // Should have 4 nets: SDA, SCL, VCC, GND (NoConnect is excluded)
        assert_eq!(
            snapshot.nets.len(),
            4,
            "nets: {:?}",
            snapshot.nets.keys().collect::<Vec<_>>()
        );
        assert!(snapshot.nets.contains_key("SDA"));
        assert!(snapshot.nets.contains_key("SCL"));
        assert!(snapshot.nets.contains_key("VCC"));
        assert!(snapshot.nets.contains_key("GND"));

        // Pin assignments should be set on the component
        let u1 = &snapshot.components["U1"];
        assert_eq!(u1.pins["GPIO4"].net.as_deref(), Some("SDA"));
        assert_eq!(u1.pins["GPIO5"].net.as_deref(), Some("SCL"));
        assert_eq!(u1.pins["VDD"].net.as_deref(), Some("VCC"));
        assert_eq!(u1.pins["GND"].net.as_deref(), Some("GND"));

        // NC1 should not have a pin entry
        assert!(
            !u1.pins.contains_key("NC1"),
            "NoConnect pins should not appear in pin map"
        );
    }

    #[test]
    fn test_schdoc_pin_connections_merge_with_explicit_nets() {
        let mut sheet = make_empty_sheet();

        // Explicit net declaration
        let mut comp1 = make_schdoc_component("R1");
        comp1.pin_connections = Vec::new();
        sheet.components.push(comp1);

        let mut comp2 = make_schdoc_component("U1");
        comp2.pin_connections = vec![PinConnectionSpec {
            pin_name: "SDA".to_string(),
            target: PinConnectionTarget::Signal("I2C_SDA".to_string()),
        }];
        sheet.components.push(comp2);

        // Also an explicit net connecting R1 to the same net
        sheet.nets.push(NetSpec {
            annotation: None,
            name: "I2C_SDA".to_string(),
            pins: vec![PinRef {
                component: "R1".to_string(),
                pin: "1".to_string(),
            }],
        });

        let spec = SchDocSpec {
            sheets: vec![sheet],
        };
        let imported = std::collections::HashMap::new();
        let snapshot = project_schdoc_spec(&spec, &imported).unwrap();

        // Should have one merged I2C_SDA net with 2 pin connections
        let net = &snapshot.nets["I2C_SDA"];
        assert_eq!(
            net.pins.len(),
            2,
            "net should have pins from both explicit and pin_connection: {:?}",
            net.pins
        );
        assert!(net.pins.contains(&("R1".to_string(), "1".to_string())));
        assert!(net.pins.contains(&("U1".to_string(), "SDA".to_string())));
    }

    #[test]
    fn test_schdoc_pin_connections_multiple_components_same_net() {
        let mut sheet = make_empty_sheet();

        let mut comp1 = make_schdoc_component("U1");
        comp1.pin_connections = vec![PinConnectionSpec {
            pin_name: "SDA".to_string(),
            target: PinConnectionTarget::Signal("I2C_SDA".to_string()),
        }];
        sheet.components.push(comp1);

        let mut comp2 = make_schdoc_component("U2");
        comp2.pin_connections = vec![PinConnectionSpec {
            pin_name: "SDA".to_string(),
            target: PinConnectionTarget::Signal("I2C_SDA".to_string()),
        }];
        sheet.components.push(comp2);

        let spec = SchDocSpec {
            sheets: vec![sheet],
        };
        let imported = std::collections::HashMap::new();
        let snapshot = project_schdoc_spec(&spec, &imported).unwrap();

        let net = &snapshot.nets["I2C_SDA"];
        assert_eq!(net.pins.len(), 2);
    }
}

#[cfg(feature = "proptest")]
mod proptests {
    use super::*;
    use crate::model::{
        BoardSpec, PcbDocComponentSpec, PcbDocNetSpec, PcbDocSpec, SchDocComponentSpec, SchDocSpec,
        SheetSpec, SymbolRef,
    };
    use altium_format_types::{CoordPoint, RotationBy90};
    use proptest::prelude::*;

    fn arb_sync_component(designator: String) -> SyncComponent {
        SyncComponent {
            designator,
            comment: None,
            footprint: None,
            source_library: None,
            parameters: IndexMap::new(),
            pins: IndexMap::new(),
            annotation_id: None,
            source_unique_id: None,
        }
    }

    fn arb_sync_net(name: String) -> SyncNet {
        SyncNet {
            name,
            color: None,
            pins: Vec::new(),
            annotation_id: None,
        }
    }

    proptest! {
        #[test]
        fn prop_diff_reflexive(
            designators in proptest::collection::hash_set("[A-Z][0-9]{1,3}", 0..8),
            net_names in proptest::collection::hash_set("[A-Z_][A-Z_0-9]{1,7}", 0..5),
        ) {
            let mut snapshot = SyncSnapshot::default();
            for des in &designators {
                snapshot.components.insert(des.clone(), arb_sync_component(des.clone()));
            }
            for name in &net_names {
                snapshot.nets.insert(name.clone(), arb_sync_net(name.clone()));
            }

            let changes = diff_snapshots(&snapshot, &snapshot);
            prop_assert!(changes.is_empty(), "diff(a, a) must be empty; got: {:?}", changes);
        }

        #[test]
        fn prop_diff_nonempty_when_different(
            des_a in "[A-Z][0-9]",
            des_b in "[A-Z][0-9]",
        ) {
            // Only test when designators differ (otherwise snapshots are identical)
            prop_assume!(des_a != des_b);

            let mut a = SyncSnapshot::default();
            a.components.insert(des_a.clone(), arb_sync_component(des_a.clone()));

            let mut b = SyncSnapshot::default();
            b.components.insert(des_b.clone(), arb_sync_component(des_b.clone()));

            let changes = diff_snapshots(&a, &b);
            prop_assert!(!changes.is_empty(), "diff(a, b) must be non-empty when a != b");
        }
    }

    fn arb_designator() -> impl Strategy<Value = String> {
        "[A-Z][0-9]{1,3}".prop_map(|s| s)
    }

    fn arb_net_name() -> impl Strategy<Value = String> {
        "[A-Z_][A-Z_0-9]{1,7}".prop_map(|s| s)
    }

    proptest! {
        #[test]
        fn prop_schdoc_projection_preserves_component_count(
            designators in proptest::collection::hash_set(arb_designator(), 0..10)
        ) {
            let designators: Vec<String> = designators.into_iter().collect();
            let mut sheet = SheetSpec {
                annotation: None,
                fonts: Vec::new(),
                power_declarations: std::collections::HashMap::new(),
                custom_width: None,
                custom_height: None,
                snap_grid_on: None,
                visible_grid_on: None,
                hot_spot_grid_on: None,
                show_hidden_pins: None,
                border_on: None,
                title_block_on: None,
                components: Vec::new(),
                nets: Vec::new(),
                powers: Vec::new(),
                objects: Vec::new(),
                constraints: Vec::new(),
            };

            let count = designators.len();
            for des in &designators {
                sheet.components.push(SchDocComponentSpec {
                    annotation: None,
                    designator: des.clone(),
                    symbol: SymbolRef::Literal("S".to_string()),
                    location: CoordPoint {
                        x: altium_format_types::Coord::from_mils_f64(0.0),
                        y: altium_format_types::Coord::from_mils_f64(0.0),
                    },
                    orientation: Some(RotationBy90::Rotate0),
                    is_mirrored: None,
                    description: None,
                    parameters: Vec::new(),
                    pin_connections: Vec::new(),
                });
            }

            let spec = SchDocSpec { sheets: vec![sheet] };
            let snapshot = project_schdoc_spec(&spec, &std::collections::HashMap::new()).unwrap();
            prop_assert_eq!(snapshot.components.len(), count);
        }

        #[test]
        fn prop_pcbdoc_projection_preserves_component_and_net_count(
            designators in proptest::collection::hash_set(arb_designator(), 0..10),
            net_names in proptest::collection::hash_set(arb_net_name(), 0..5),
        ) {
            let designators: Vec<String> = designators.into_iter().collect();
            let net_names: Vec<String> = net_names.into_iter().collect();

            let comp_count = designators.len();
            let net_count = net_names.len();

            let mut board = BoardSpec {
                annotation: None,
                name: "Board".to_string(),
                signal_layer_count: None,
                snap_grid_size: None,
                visible_grid_size: None,
                display_unit: None,
                nets: Vec::new(),
                components: Vec::new(),
                tracks: Vec::new(),
                arcs: Vec::new(),
                vias: Vec::new(),
                pads: Vec::new(),
                fills: Vec::new(),
                texts: Vec::new(),
                regions: Vec::new(),
                component_bodies: Vec::new(),
                dimensions: Vec::new(),
                polygons: Vec::new(),
                rules: Vec::new(),
                classes: Vec::new(),
                differential_pairs: Vec::new(),
            };

            for des in &designators {
                board.components.push(PcbDocComponentSpec {
                    annotation: None,
                    designator: des.clone(),
                    pattern: None,
                    comment: None,
                    location: None,
                    rotation: None,
                    layer: None,
                    source_library: None,
                parameters: indexmap::IndexMap::new(),
                });
            }

            for name in &net_names {
                board.nets.push(PcbDocNetSpec {
                    annotation: None,
                    name: name.clone(),
                    color: None,
                    visible: None,
                });
            }

            let spec = PcbDocSpec {
                boards: vec![board],
                placement: None,
                placement_rules: Vec::new(),
            };
            let snapshot = project_pcbdoc_spec(&spec).unwrap();
            prop_assert_eq!(snapshot.components.len(), comp_count);
            prop_assert_eq!(snapshot.nets.len(), net_count);
        }
    }
}
