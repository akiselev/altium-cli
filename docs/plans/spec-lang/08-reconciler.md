# 08 - Reconciler

## Location

`crates/altium-format-ops/src/spec/reconciler.rs`

## Purpose

Compare the SpecModel (desired state) against a loaded document (current state)
and produce an `EngineeringChangeOrder` — a structured diff of Add, Update, and
Unchanged entries.

## Public API

```rust
/// Reconcile a spec model against a SchLib document.
pub fn reconcile_schlib(
    spec: &SchLibSpec,
    doc: &SchLib,
) -> Result<EngineeringChangeOrder, SpecError>

/// Reconcile a spec model against a PcbLib document.
pub fn reconcile_pcblib(
    spec: &PcbLibSpec,
    doc: &PcbLib,
) -> Result<EngineeringChangeOrder, SpecError>

/// Reconcile against an empty document (everything is Add).
pub fn reconcile_schlib_empty(
    spec: &SchLibSpec,
) -> EngineeringChangeOrder

pub fn reconcile_pcblib_empty(
    spec: &PcbLibSpec,
) -> EngineeringChangeOrder
```

## Algorithm

### SchLib Reconciliation

For each `ComponentSpec` in the spec:

1. **Find matching component** in the document by `lib_reference`
   (case-insensitive match).

2. **If not found**: Emit `EntityChange::Add` for the component and all its
   children (pins, parameters, aliases, graphics, footprint maps).

3. **If found**: Compare spec fields against document fields. For each field:
   - If spec specifies a value and document has a different value:
     `PropChange { field, old_value, new_value }`
   - If spec specifies a value and document matches: no change
   - If spec does NOT specify a value (None): skip (additive semantics)

4. **Reconcile children**: For each pin, parameter, alias, graphic, footprint
   in the spec:
   - Find matching child in the document by identity key
   - If not found: `EntityChange::Add`
   - If found: compare fields, emit `EntityChange::Update` if different

5. **Document-only entities**: Entities in the document but NOT in the spec
   are `EntityChange::Unchanged` (additive semantics — never delete).

### Identity Key Matching

| Entity | Identity key | Case-sensitivity |
|--------|-------------|------------------|
| Component | `lib_reference` | Case-insensitive |
| Pin (single-part) | `designator` | Case-insensitive |
| Pin (multi-part) | `(owner_part_id, designator)` | Case-insensitive |
| Parameter | `name` | Case-insensitive |
| Alias | `alias_name` | Case-insensitive |
| Graphic | `unique_id` | Case-sensitive |
| Footprint | `display_name` | Case-insensitive |
| Pad | `pad_name` | Case-insensitive |

### Value Normalization (spec-lang.md §10.1)

Before comparison, normalize both spec and document values:

**(a) Dimensions**: Convert to i32 internal units (10,000 per mil). Tolerance
±1 internal unit.

```rust
fn coords_equal(a: Coord, b: Coord) -> bool {
    (a.raw() - b.raw()).abs() <= 1
}

fn coord_points_equal(a: &CoordPoint, b: &CoordPoint) -> bool {
    coords_equal(a.x, b.x) && coords_equal(a.y, b.y)
}
```

**(b) Colors**: Compare as normalized Win32 COLORREF `0x00BBGGRR`.

**(c) Strings**: Case-sensitive for most fields. Case-insensitive for identity
keys (lib_reference, designator, pad_name, display_name).

**(d) Enums**: Compare by canonical value (case-insensitive, underscore-insensitive).

**(e) Booleans**: Compare by value.

## Data Structures

```rust
pub struct EngineeringChangeOrder {
    pub library_path: PathBuf,
    pub spec_path: PathBuf,
    pub timestamp: SystemTime,
    pub summary: EcoSummary,
    pub changes: Vec<EntityChange>,
}

pub struct EcoSummary {
    pub by_kind: IndexMap<EntityKind, KindSummary>,
}

pub struct KindSummary {
    pub adds: usize,
    pub updates: usize,
    pub unchanged: usize,
}

pub enum EntityKind {
    Component, Pin, Parameter, Alias, Graphic,
    Footprint, Pad, Track, Via, Arc, Text, Fill, Region,
}

pub enum EntityChange {
    Add {
        kind: EntityKind,
        identity: String,
        props: Vec<PropValue>,
        children: Vec<EntityChange>,
    },
    Update {
        kind: EntityKind,
        identity: String,
        prop_changes: Vec<PropChange>,
        children: Vec<EntityChange>,
    },
    Unchanged {
        kind: EntityKind,
        identity: String,
    },
}

pub struct PropChange {
    pub field: String,
    pub old_value: String,    // display representation
    pub new_value: String,    // display representation
}

pub struct PropValue {
    pub field: String,
    pub value: String,        // display representation
}
```

## Component-Level Reconciliation Detail

```rust
fn reconcile_component(
    spec: &ComponentSpec,
    doc_component: Option<&SchComponent>,
) -> EntityChange {
    match doc_component {
        None => {
            // Full add: component + all children
            EntityChange::Add {
                kind: EntityKind::Component,
                identity: spec.lib_reference.clone(),
                props: component_props(spec),
                children: [
                    spec.pins.iter().map(pin_add),
                    spec.parameters.iter().map(param_add),
                    spec.aliases.iter().map(alias_add),
                    spec.graphics.iter().map(graphic_add),
                    spec.footprints.iter().map(footprint_add),
                ].concat(),
            }
        }
        Some(doc) => {
            // Compare fields
            let mut prop_changes = Vec::new();
            if let Some(ref desc) = spec.description {
                if desc != &doc.description {
                    prop_changes.push(PropChange {
                        field: "description".to_string(),
                        old_value: doc.description.clone(),
                        new_value: desc.clone(),
                    });
                }
            }
            // ... repeat for each spec field

            // Reconcile children
            let children = reconcile_children(spec, doc);

            if prop_changes.is_empty() && children.iter().all(|c| matches!(c, EntityChange::Unchanged { .. })) {
                EntityChange::Unchanged { kind: EntityKind::Component, identity: spec.lib_reference.clone() }
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
```

## Footprint Validation

When a component references a footprint from an import:

1. Check that the import exists and is a PcbLib spec
2. Check that the named footprint exists in the imported spec
3. Check that all mapped pads exist in the footprint definition
4. Check that all mapped pins exist in the component
5. Check that no pad is mapped more than once (E_DUPLICATE_MAP)
6. Emit informational note for unmapped pads (thermal pads, mounting holes)

This validation happens during reconciliation, not during compilation, because
it requires cross-file entity resolution.

## Summary Computation

After all changes are computed, aggregate counts:

```rust
fn compute_summary(changes: &[EntityChange]) -> EcoSummary {
    let mut summary = EcoSummary::default();
    for change in changes {
        count_change(change, &mut summary);
    }
    summary
}

fn count_change(change: &EntityChange, summary: &mut EcoSummary) {
    match change {
        EntityChange::Add { kind, children, .. } => {
            summary.by_kind.entry(*kind).or_default().adds += 1;
            for child in children { count_change(child, summary); }
        }
        EntityChange::Update { kind, children, .. } => {
            summary.by_kind.entry(*kind).or_default().updates += 1;
            for child in children { count_change(child, summary); }
        }
        EntityChange::Unchanged { kind, .. } => {
            summary.by_kind.entry(*kind).or_default().unchanged += 1;
        }
    }
}
```

## Test Strategy

- Empty document: all Add
- Identical document: all Unchanged
- Single field change: one Update
- Missing pin: Add nested under component Update
- Dimension tolerance: values within ±1 internal unit = equal
- Case-insensitive identity matching
- Multi-part component reconciliation
- Footprint map validation (missing pad, duplicate map)
- Mixed: some components added, some updated, some unchanged
