# autopcb-ir

Intermediate representation for PCB placement, routing, and DRC. All coordinates
are mm-based (f64). The IR is format-independent: downstream consumers (router,
DRC engine, placer, viewer) never import `altium_format`.

## Architecture

```
pcbdoc-spec file (source of truth)
     |
     v
SpecModel (PcbDocSpec)           PcbDoc file (import source)
     |                                |
     v                                v
spec_to_ir()  <---- only path     import_pcbdoc() adapter
     |                                |
     v                          produces PcbDocSpec
   PcbIr                              |
     |                          feeds into spec_to_ir()
     v                                |
  Router / DRC / Placer               v
                                   PcbIr (same output)
```

`spec_to_ir()` is the single IR compilation path. PcbDoc import is an adapter
that converts a `PcbDocBoard` into a `PcbDocSpec`, which then feeds into
`spec_to_ir()`. Future import formats (KiCad, Eagle, etc.) follow the same
pattern: `foreign_file -> import_adapter -> PcbDocSpec -> spec_to_ir() -> PcbIr`.

The `spec_bridge` module encapsulates the full pipeline behind a single call
(`load_ir_from_spec`). Callers never touch `PcbDoc` or `PcbDocBoard` directly.

## Data Flow

```
PcbDocSpec.boards[0]
  .nets[]          -> build net lookup (name -> NetId) + net classes + diff pairs
  .components[]    -> build component map with pads (designator -> ComponentId)
  .rules[]         -> type-check kind, parse properties, resolve scope -> IrDesignRule
  .tracks/vias/... -> evaluate Value properties, convert Coord -> mm -> FreeCopperGeometry
  .polygons[]      -> resolve net/layer, convert vertices -> IrPolygon
  .outline         -> tessellate polygon vertices -> IrBoardGeometry
  .keepouts[]      -> convert polygon vertices -> IrKeepoutZone
```

## Scope Resolution

Rule scope is resolved from scope strings to concrete `IrRuleScope` variants at
compile time. Downstream consumers receive precomputed lookups with no string
parsing at query time.

### Scope string grammar

Scope strings appear in `PcbDocRuleSpec.scope` and `PcbDocRuleSpec.scope2`:

| Expression                              | Resolves to                              |
| --------------------------------------- | ---------------------------------------- |
| `All` (or empty)                        | `IrRuleScope::All`                       |
| `InNetClass(<name>)`                    | `IrRuleScope::NetClass(name)`            |
| `OnLayer(<name>)`                       | `IrRuleScope::Layer(layer_id)`           |
| `InNetClass(<name>) And OnLayer(<name>)`| `IrRuleScope::NetClassAndLayer(...)`     |

This grammar is produced by both the spec DSL compiler and the PcbDoc import
adapter; `spec_to_ir()` is the single consumer.

### Scope cascade priority

When multiple rules match, the most specific scope wins:
`NetClassAndLayer` > `NetClass` > `Layer` > `All`

This matches Altium's `IPCB_RuleManager.FilteredPrimitivesSorted` behavior.
Priority is implemented via explicit match arms in DrcPolicy, not via `Ord` on
`IrRuleScope`, so the cascade remains visible and auditable.

### Two-object rules

Altium rules like `Clearance` and `ComponentClearance` have two scope sides.
`IrRuleScopePair { scope1, scope2 }` models this. Single-object rules set
`scope2: IrRuleScope::All`.

## Invariants

- `spec_to_ir()` never imports from `altium_format` (only `altium_format_types`
  and `altium_format_spec`)
- All scope resolution happens during compilation, not at query time
- `PcbIr` is the same struct regardless of whether source is spec or imported PcbDoc
- Handle IDs (`ComponentId`, `NetId`, etc.) are assigned sequentially during
  compilation via `IdMap::push()`
- Coordinate conversion (`Coord` -> mm via `.to_mms()`) happens exactly once,
  at the compilation boundary inside `spec_to_ir()`
- All lookup tables (`BTreeMap`) use deterministic iteration order

## Merge Strategy (spec + PcbDoc)

When loading from a spec file that references a PcbDoc:

1. Import PcbDoc into `PcbDocSpec` via `import_pcbdoc()`
2. Merge spec file mutations on top: **spec file wins on conflict**
   - `Option` fields: `Some(v)` from spec overwrites; `None` preserves import value
   - `Vec` fields: non-empty spec vec replaces import vec; empty spec vec preserves import
3. Compile merged spec via `spec_to_ir()`

The spec file acts as an override layer. Import adapter output is the default;
spec file provides targeted overrides.

## Error Handling

Compilation errors are `IrCompileError` variants. Hard errors on unknown rule
kinds, duplicate designators, and unresolved layer names — never silent skips.
