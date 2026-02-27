# 09 - Executor (ECO to Low-Level Ops)

## Location

`crates/altium-format-ops/src/spec/executor.rs`

## Purpose

Convert `EngineeringChangeOrder` entries into `HighOp` sequences that flow
through the existing lowering pipeline (`HighOp -> ComposedOp -> LowOp ->
apply`).

## Public API

```rust
/// Convert an ECO into a sequence of high-level operations.
pub fn eco_to_high_ops(eco: &EngineeringChangeOrder) -> Vec<HighOp>

/// Full pipeline: compile spec, reconcile, execute.
pub fn apply_spec_to_schlib(
    spec_source: &str,
    spec_path: &Path,
    lib: &mut SchLib,
) -> Result<(EngineeringChangeOrder, ApplyReport), SpecError>

pub fn apply_spec_to_pcblib(
    spec_source: &str,
    spec_path: &Path,
    lib: &mut PcbLib,
) -> Result<(EngineeringChangeOrder, ApplyReport), SpecError>
```

## Mapping: EntityChange -> HighOp

### Add Component

```
EntityChange::Add { kind: Component, identity: "R_0603", props, children }
```

Maps to:
```rust
HighOp::AddComponent(AddComponentOp {
    lib_reference: "R_0603".to_string(),
    designator: props.get("designator"),
    // ... other component fields from props
    pins: children.iter()
        .filter(|c| c.kind() == Pin && c.is_add())
        .map(|c| pin_change_to_add_pin_op(c))
        .collect(),
    footprint: children.iter()
        .find(|c| c.kind() == Footprint && c.is_add())
        .map(|c| footprint_change_to_op(c)),
})
```

For each child Add entry:
- `Pin` -> `AddPinOp` (embedded in `AddComponentOp.pins`)
- `Parameter` -> `AddParameterOp`
- `Alias` -> `AddAliasOp`
- `Graphic` -> graphic-specific `HighOp` (AddRectangle, AddLine, etc.)
- `FootprintMap` -> `FootprintOp` (embedded in `AddComponentOp.footprint`)

### Update Component

For `EntityChange::Update`, we need Edit ops. Currently available:
- `EditComponent(EditComponentHighOp)` — change component-level properties
- `EditRecord(EditRecordHighOp)` — patch any record by selector

**Strategy for initial implementation**: Use `EditComponent` for component-level
property changes. For child updates (pin field changes, parameter text changes),
use `EditRecord` with a targeted selector.

For children that are `Add` within an `Update` parent, emit the corresponding
`Add` op (e.g., `AddPin` for a new pin in an existing component).

### Add Footprint (PcbLib)

```
EntityChange::Add { kind: Footprint, identity: "SOT23", props, children }
```

Maps to:
```rust
HighOp::AddFootprint(AddFootprintHighOp {
    name: "SOT23".to_string(),
    description: props.get("description"),
    // ...
})
```

Followed by `AddTrack`, `AddVia`, etc. for child graphics and pads.

Note: `AddPad` does not exist as a HighOp yet (see §13). Until it does, pad
creation requires extending the ops infrastructure.

### Unchanged

`EntityChange::Unchanged` entries produce no ops.

## Execution Pipeline

```rust
pub fn apply_spec_to_schlib(
    spec_source: &str,
    spec_path: &Path,
    lib: &mut SchLib,
) -> Result<(EngineeringChangeOrder, ApplyReport), SpecError> {
    // 1. Parse spec
    let ast = parse_spec(spec_source)?;

    // 2. Resolve imports
    let resolved = resolve_imports(spec_path, ast)?;

    // 3. Compile to SpecModel
    let model = compile_spec(&resolved, SpecDomain::SchLib)?;

    // 4. Reconcile against document
    let eco = reconcile_schlib(model.as_schlib(), lib)?;

    // 5. Convert ECO to HighOps
    let high_ops = eco_to_high_ops(&eco);

    // 6. Apply through existing pipeline
    let report = apply_schlib(lib, &high_ops)?;

    Ok((eco, report))
}
```

## OpId Strategy

The executor generates opids for tracking:

```
spec:component:R_0603                    // component add/update
spec:component:R_0603:pin:1              // pin add/update
spec:component:R_0603:parameter:Value    // parameter add
spec:component:R_0603:graphic:body       // graphic add
spec:footprint:SOT23                     // footprint add
spec:footprint:SOT23:pad:1              // pad add
```

These opids are used in the `ApplyReport` to correlate results back to spec
entities.

## Edit Op Fallback

For entities where a targeted edit op does not exist, the executor uses
delete + re-add. This preserves the identity key and is semantically correct
for library entities:

```rust
fn update_entity_fallback(
    kind: EntityKind,
    identity: &str,
    new_props: &[PropValue],
) -> Vec<HighOp> {
    vec![
        // Remove the old entity
        remove_op(kind, identity),
        // Re-add with new properties
        add_op(kind, identity, new_props),
    ]
}
```

This fallback is used only until proper Edit ops are implemented (§13).

## Test Strategy

- Add-only ECO: verify correct HighOp sequence
- Update ECO: verify Edit/remove+add ops
- Mixed ECO: adds + updates + unchanged
- Verify opid generation
- Roundtrip: apply ECO, re-reconcile, verify all Unchanged
- PcbLib path: footprint add with pads and graphics
