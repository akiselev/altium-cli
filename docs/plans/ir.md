# Direct Spec-to-IR Compiler Pipeline

## Overview

Replace the current lossy `.pcb spec -> SpecModel -> executor -> PcbDoc -> extract -> PcbIr`
pipeline with a direct `.pcb spec -> SpecModel -> spec_to_ir() -> PcbIr` compiler that
preserves all spec information (rule scope, placement constraints, annotations) without
touching PcbDoc types. PcbDoc import is a separate adapter that generates spec models,
making PcbDoc just another import format alongside future KiCad support.

Architecture C (Layered): `spec_to_ir()` is the only IR compilation path. PcbDoc import is a
separate adapter module that produces `PcbDocSpec`, which then feeds into `spec_to_ir()`.
`altium-format-types` domain enums (RuleKind, CornerStyle, etc.) are acceptable for now;
they'll be replaced with IR-native types in a later PR.

## Planning Context

### Decision Log

| Decision | Reasoning Chain |
|----------|----------------|
| Layered architecture (Approach C) | User wants future KiCad/etc import -> PcbDoc must be just one import adapter -> single IR compilation path avoids divergence -> spec_to_ir() is the canonical compiler, import adapters produce SpecModel |
| Add geometry to spec language | Full spec independence is required -> board outline and pad shapes must be spec-native -> cannot depend on PcbDoc for geometry -> spec language needs `outline`, `keepout`, `pad` geometry blocks |
| Resolve scope at compile time | Scope evaluation needs net/layer context -> compiler has all nets and layers -> resolving at compile produces concrete LayerId/net-class sets -> router/DRC gets precomputed lookups, no string parsing at query time |
| altium-format-types OK for now | Domain enums (RuleKind, CornerStyle, NetTopology) are semantic concepts, not format internals -> replacing them is mechanical -> defer to a later PR to avoid scope creep |
| No altium-format imports in new compiler | altium-format contains PcbDoc/SchLib/etc document types -> these are format-specific parsing artifacts -> spec_to_ir() must depend only on autopcb-spec (SpecModel) and altium-format-types (domain enums) |
| Keep extract.rs as legacy import path | Existing PcbDoc files need to be imported -> import adapter reads PcbDoc and produces PcbDocSpec -> spec_to_ir() is the canonical IR compilation path; extract.rs is available for direct PcbDoc extraction |
| IrDesignRule has resolved scope (IrRuleScopePair) | Raw scope strings are Altium-specific syntax -> resolved scope is format-independent -> IrRuleScope with concrete layer/net-class sets works for any import format |
| BTreeMap for all scope resolution tables | Determinism invariant from CLAUDE.md -> HashMap non-deterministic iteration -> BTreeMap everywhere in IR and policy |
| Spec file fixtures for testing | Spec file fixtures exercise the actual compilation pipeline -> parse+compile+extract roundtrip tests -> requires test-fixtures feature gate -> more realistic than synthetic struct construction |
| PcbIr struct field additions | Placement constraints, rule scope, annotations are spec-only concepts -> must be in PcbIr for downstream algorithms -> additive fields, non-breaking |
| Scope string grammar: `InNetClass(<name>)`, `OnLayer(<name>)`, `All` | Scope expressions must be parseable by spec_to_ir() and producible by import adapters -> Altium uses this syntax natively -> adopting the same grammar avoids translation -> spec DSL compiler and PcbDoc import adapter both emit this format -> spec_to_ir() is the single consumer |
| Scope cascade priority: exact > class > layer > All | Altium resolves rules by specificity (most-constrained wins) -> class+layer is more specific than class-only or layer-only -> this matches Altium's `IPCB_RuleManager.FilteredPrimitivesSorted` behavior -> prevents ambiguity when multiple scoped rules match |
| `load_ir_from_spec()` implementation: open PcbDoc → `import_pcbdoc()` → `spec_to_ir()` | `load_ir_from_spec()` opens PcbDoc file → calls import_pcbdoc() → merges spec mutations → calls spec_to_ir(). Spec file acts as mutation overlay; file-open contract is preserved. |
| `altium-format` is in Cargo.toml (required by extract.rs and pcbdoc_import.rs); spec_compiler.rs contains no `use altium_format` imports | autopcb-ir still needs `altium-format` for `extract.rs` (legacy) and `pcbdoc_import.rs` (adapter) -> removing the Cargo dependency would break these modules -> the constraint is per-file: `spec_compiler.rs` must not `use altium_format` |
| IrRuleScope uses explicit match cascade, not BTreeMap Ord | Cascade lookup is priority-based (exact > class > layer > All) -> BTreeMap Ord would impose variant-declaration order -> explicit match arms make priority visible and auditable -> no Ord derivation needed on IrRuleScope |
| Roundtrip tolerance 1e-9 mm for M5 comparison | Coord->mm is integer/10000.0, exact in f64 for all Altium coordinate values (|Coord| < 2^31) -> 1e-9 is effectively exact equality -> an off-by-one Coord error (0.0001mm = 1e-4 mm) would be caught since 1e-4 >> 1e-9 |
| `IrCompileError` as the error type for spec_to_ir() and import_pcbdoc() | autopcb-ir cannot import `AltiumFormatError` (bars altium-format) -> `SpecError` is semantically wrong for IR-specific errors (e.g., duplicate designator, missing net) -> new `IrCompileError` enum in autopcb-ir provides IR-specific variants -> both `spec_to_ir()` and `import_pcbdoc()` use it since they are part of the same compilation pipeline |
| Spec file wins on merge conflict | The spec file is the source of truth (Invisible Knowledge, Architecture) -> when PcbDocSpec from import_pcbdoc() and mutations from the .pcb file conflict on the same field, the spec file value overwrites the imported value -> import adapter provides defaults, spec file provides overrides. For Option fields: Some(v) overwrites, None preserves import value (non-destructive merge) |
| `PcbDocRuleSpec` has `scope2: Option<String>` field | Altium two-object rules (Clearance, ComponentClearance) have both scope and scope2 -> dropping scope2 silently scopes clearance rules wrong (e.g., Power-to-Signal becomes Power-to-all) -> PcbDocRuleSpec needs `scope2: Option<String>` -> spec_to_ir() resolves both scope strings into a two-object IrRuleScope variant or falls back to All with a warning |

### Rejected Alternatives

| Alternative | Why Rejected |
|-------------|-------------|
| PcbDoc fallback for geometry | Full spec independence is required: PcbDoc is not available in pure-spec workflows (KiCad import, synthetic test fixtures). Requiring PcbDoc couples all import formats to Altium's model. |
| Synthetic PcbIr test construction | Spec file fixtures exercise the actual compilation pipeline (parse + compile). Synthetic struct construction bypasses the compiler and cannot detect spec parser or scope resolution bugs. |
| Pass scope as raw strings | Router would need a scope expression parser -> duplicates compiler work -> resolved scope is simpler for downstream consumers and format-independent |
| Incremental approach (Approach A) | Two compilation paths would diverge over time -> maintenance burden -> single path with import adapters is cleaner |

### Constraints & Assumptions

- `altium-format-types` dependency is acceptable (domain enums, coordinate types)
- `altium-format` dependency is NOT acceptable in the new spec compiler module
- `autopcb-spec` dependency IS needed (provides SpecModel/PcbDocSpec)
- All coordinates in IR remain mm-based (f64), converted from Coord at compile time
- Spec language extensions (outline, pad geometry) use existing parser infrastructure
- Tests use `.pcb` fixture files, gated behind `test-fixtures` feature
- CLAUDE.md fail-fast: unknown rule kinds must hard-error, not silently skip

### Known Risks

| Risk | Mitigation | Anchor |
|------|-----------|--------|
| Spec language geometry extensions may be large scope | Start with minimal outline (polygon vertices) and rectangular pad shapes. Complex arcs/curves deferred. | N/A (new code) |
| PcbDoc import adapter is complex | Reuse existing extract.rs logic internally. Adapter wraps extraction output in PcbDocSpec form. | crates/autopcb-ir/src/extract.rs |
| Scope expression syntax may vary across import formats | Resolve to format-independent IrRuleScope at import boundary. Each adapter handles its own syntax. | N/A (new code) |
| IrDesignRule struct changes break downstream | Additive fields with defaults (Option/Vec). Existing code unaffected. | crates/autopcb-ir/src/rule.rs |

## Invisible Knowledge

### Architecture

```
.pcb spec file (source of truth)
     |
     v
SpecModel (PcbDocSpec)           PcbDoc file (import source)
     |                                |
     v                                v
spec_to_ir()  <---- only path     pcbdoc_import adapter
     |                                |
     v                          produces PcbDocSpec
   PcbIr                              |
     |                          feeds into spec_to_ir()
     v                                |
  Router / DRC / Placer               v
                                   PcbIr (same output)
```

Future import adapters (KiCad, Eagle, etc.) follow the same pattern:
`foreign_file -> import_adapter -> PcbDocSpec -> spec_to_ir() -> PcbIr`

### Data Flow

```
PcbDocSpec.boards[0]
  .nets[]          -> build net lookup (name -> NetId) + net classes
  .components[]    -> build component map with pads (designator -> ComponentId)
  .rules[]         -> type-check kind, parse properties, resolve scope -> IrDesignRule
  .tracks/vias/... -> evaluate Value properties, convert Coord -> mm -> FreeCopperGeometry
  .polygons[]      -> resolve net/layer, convert vertices -> IrPolygon
  .outline         -> NEW: tessellate polygon vertices -> IrBoardGeometry
  .keepouts[]      -> NEW: convert polygon vertices -> IrKeepoutZone
```

### Why This Structure

The layered architecture ensures:
1. **Single compilation path** — no divergence between spec and PcbDoc extraction
2. **Format independence** — IR types never reference format-specific concepts
3. **Scope preservation** — rule scope resolved to concrete IDs at compile time
4. **Extensibility** — new import formats just need an adapter to PcbDocSpec

### Scope String Grammar (Cross-Milestone Contract)

The canonical scope string format used in `PcbDocRuleSpec.scope` is:
- `All` — applies to all objects (default when scope is None or empty)
- `InNetClass(<name>)` — applies to objects in the named net class
- `OnLayer(<name>)` — applies to objects on the named layer
- `InNetClass(<name>) And OnLayer(<name>)` — both conditions must match

This grammar is:
- **Produced by**: spec DSL compiler (M3) and PcbDoc import adapter (M5)
- **Consumed by**: `spec_to_ir()` scope resolver (M4)
- **Resolved to**: `IrRuleScope` enum variants at compile time

The PcbDoc import adapter preserves Altium's native scope syntax (which uses the same `InNetClass(...)` / `OnLayer(...)` format). The spec DSL compiler generates the same format from its own syntax.

### Invariants

- `spec_to_ir()` never imports from `altium_format` (only `altium_format_types` and `autopcb_spec`)
- All scope resolution happens during compilation, not at query time
- PcbIr is the same struct regardless of whether source is spec or imported PcbDoc
- Handle IDs (ComponentId, NetId, etc.) are assigned sequentially during compilation
- Coordinate conversion (Coord -> mm) happens exactly once, at compilation boundary

### Tradeoffs

- **Spec language complexity increases** — adding geometry blocks makes spec files larger, but eliminates PcbDoc dependency
- **Import adapter is extra code** — wrapping PcbDoc extraction in a spec adapter is redundant, but ensures single compilation path
- **Resolved scope is less flexible** — can't evaluate dynamic scope at runtime, but simpler and faster for router/DRC

## Plan Flags

| Flag | Consumer | Meaning |
|------|----------|---------|
| `conformance` | QR | Cross-crate API contracts between spec, IR, and router |
| `needs-rationale` | TW | Scope resolution and geometry compilation need WHY comments |
| `complex-algorithm` | QR/TW | Scope expression evaluation is non-trivial |

## Milestones

### Milestone 1: IrDesignRule Scope Extension

**Files**:
- `crates/autopcb-ir/src/rule.rs`
- `crates/autopcb-ir/src/extract.rs`

**Flags**: `conformance`, `needs-rationale`

**Requirements**:
- Add `IrRuleScope` enum: `All`, `NetClass(String)`, `Layer(LayerId)`, `NetClassAndLayer(String, LayerId)` (no `Net(NetId)` — no scope grammar produces it; add when needed)
- Add `IrRuleScopePair` struct: `{ scope1: IrRuleScope, scope2: IrRuleScope }` for two-object rules (Clearance, ComponentClearance). `scope2` defaults to `All` for single-object rules
- Add `scope: IrRuleScopePair` field to `IrDesignRule` (default both `All`)
- All `IrDesignRule` construction sites compile with the `scope` field (default: IrRuleScopePair with both scopes All): `extract.rs` and router test helpers in `policy.rs::add_rule()`, `width.rs`, `via.rs`, `geometry.rs`, `clearance.rs`, `connectivity.rs`, `length.rs`, `diff_pair.rs`, `board.rs`, `cpu_engine.rs`, `repair.rs`, `manufacturing.rs`, `topology.rs`

**Acceptance Criteria**:
- `IrDesignRule` has a `scope` field
- Default scope is `All` (matches current behavior where all rules are global)
- `cargo check --workspace` passes (covers router test helpers that construct IrDesignRule)

**Tests**:
- **Test files**: `crates/autopcb-ir/src/rule.rs` (inline)
- **Test type**: unit
- **Backing**: default-derived
- **Scenarios**:
  - Construct IrDesignRule with All scope
  - Construct IrDesignRule with NetClass scope

**Code Intent**:
- Add `IrRuleScope` enum to `rule.rs`
- Add `pub scope: IrRuleScopePair` field to `IrDesignRule`
- `extract_rules()` in `extract.rs` sets `scope: IrRuleScopePair::default()` for all rules (All scope: global rules, no net-class or layer restriction)
- Router test helpers in policy.rs, width.rs, via.rs, geometry.rs, clearance.rs, connectivity.rs, length.rs, diff_pair.rs, board.rs, cpu_engine.rs, repair.rs, manufacturing.rs, topology.rs construct `IrDesignRule` with `scope: IrRuleScopePair::default()`

---

### Milestone 2: DrcPolicy Scoped Rule Resolution

**Files**:
- `crates/autopcb-router/src/drc/policy.rs`
- `crates/autopcb-router/src/drc/width.rs`
- `crates/autopcb-router/src/drc/via.rs`

**Flags**: `conformance`, `needs-rationale`

**Requirements**:
- `DrcPolicy::build()` collects scoped rules into `Vec<(IrRuleScope, DrcWidthBounds)>` ordered by rule priority, similar for via/clearance
- `width_bounds(net_class, layer)` resolves by checking: exact (class+layer) -> class-only -> layer-only -> All
- `via_bounds(net_class)` resolves by checking: class-specific -> All
- `check_widths()` passes actual net class and layer to `width_bounds()`
- `check_vias()` passes actual net class to `via_bounds()`

**Acceptance Criteria**:
- Width rule scoped to `NetClass("Power")` with min 0.3mm only applies to Power-class nets
- Width rule scoped to `Layer(TopLayer)` only applies to top-layer segments
- Fallback to `All`-scoped rule when no specific match
- Tests pass with multi-scope rule sets

**Tests**:
- **Test files**: `crates/autopcb-router/src/drc/policy.rs` (inline)
- **Test type**: unit
- **Backing**: user-specified
- **Scenarios**:
  - Net-class-specific width overrides default
  - Layer-specific width overrides default
  - Most specific scope wins (class+layer > class > all)
  - No matching scope falls back to All

**Code Intent**:
- `width_constraints` is `Vec<(IrRuleScope, DrcWidthBounds)>` ordered by rule priority
- In `build()`, push each Width rule with its `IrDesignRule.scope` into the vec
- Implement cascading lookup in `width_bounds()`: scan vec, return first match by specificity (NetClassAndLayer > NetClass > Layer > All). No `Ord` on `IrRuleScope` — explicit match arms per Decision Log
- `check_widths()` and `check_vias()` accept `ir: &PcbIr` to access `IrNet.net_class`. Callers in `cpu_engine.rs` pass this parameter.
- Update `check_widths()` to look up net class from solution net -> IR net -> `net_class` field
- Similar for `via_bounds()` and `check_vias()`

---

### Milestone 3: Spec Language Geometry Extensions

**Files**:
- `crates/autopcb-spec/src/model.rs`
- `crates/autopcb-spec/src/compiler.rs`
- `crates/autopcb-spec/src/parser.rs` (if separate from compiler)

**Flags**: `conformance`

**Requirements**:
- `BoardSpec` has: `outline: Option<Vec<CoordPoint>>`, `keepouts: Vec<KeepoutSpec>`, `layers: Vec<BoardLayerSpec>`
- `PcbDocRuleSpec` has: `scope2: Option<String>` for two-object rules (Clearance, ComponentClearance)
- `PcbDocComponentSpec` or new `PadSpec` has pad geometry: position, shape, size, layer, hole, net
- Compiler populates these from spec file syntax
- Spec syntax for outline: `outline { point(x, y); point(x, y); ... }` or coordinate list
- Spec syntax for pads: `pad { designator: "1"; shape: round; size: 1.5mm; hole: 0.8mm; layer: all; }`

**Acceptance Criteria**:
- A spec file with `outline { ... }` compiles to `BoardSpec` with populated outline field
- A spec file with pad definitions on a component compiles to pad geometry data
- Spec files without geometry blocks compile correctly (fields are Optional)

**Tests**:
- **Test files**: `crates/autopcb-spec/` (inline or fixtures)
- **Test type**: unit + integration (spec file parsing)
- **Backing**: user-specified
- **Scenarios**:
  - Spec with rectangular outline -> 4 CoordPoints
  - Spec with pad geometry -> PadSpec populated
  - Spec without geometry -> None fields

**Code Intent**:
- Add outline/keepout/layer fields to `BoardSpec` in model.rs:
  - `outline: Option<Vec<CoordPoint>>` — closed polygon of board edge vertices
  - `keepouts: Vec<KeepoutSpec>` where `KeepoutSpec { vertices: Vec<CoordPoint>, restrict_copper: bool, restrict_components: bool, layer: Option<LayerSpec> }`
  - `layers: Vec<BoardLayerSpec>` where `BoardLayerSpec { name: String, is_copper: bool, copper_index: Option<u32> }`
- Add pad geometry to component spec via `PadGeometrySpec { designator: String, position: CoordPoint, shape: PadShape, size_x: Coord, size_y: Coord, hole_size: Option<Coord>, layer: LayerSpec, net: Option<String> }`
- `PcbDocComponentSpec` gains `pads: Vec<PadGeometrySpec>`
- Extend compiler to parse geometry blocks from spec AST
- Geometry is optional — specs without it are valid

---

### Milestone 4: spec_to_ir() Core Compiler

**Files**:
- `crates/autopcb-ir/src/spec_compiler.rs` (NEW)
- `crates/autopcb-ir/src/compile_error.rs` (NEW — `IrCompileError` enum)
- `crates/autopcb-ir/src/lib.rs` (re-export)
- `crates/autopcb-ir/Cargo.toml` (dependency adjustment)

**Flags**: `conformance`, `needs-rationale`, `complex-algorithm`

**Requirements**:
- `pub fn spec_to_ir(spec: &PcbDocSpec) -> Result<PcbIr>` compiles PcbDocSpec directly to PcbIr
- NO imports from `altium_format` — only `autopcb_spec` and `altium_format_types`
- Handles: layer stack, board geometry, nets, net classes, diff pairs, components with pads, rules with scope resolution, free copper, polygons, texts, regions
- Coordinate conversion: Coord -> mm via `.to_mms()` at compilation boundary
- Layer resolution: `LayerSpec` -> `LayerId` using built layer stack
- Rule compilation: `PcbDocRuleSpec` -> `IrDesignRule` with `IrRuleScope` (resolved from scope string)
- Handle generation: sequential IDs via `IdMap::push()` pattern

**Acceptance Criteria**:
- A complete spec file compiles to PcbIr with all fields populated
- `cargo check -p autopcb-ir` passes without `altium-format` import in spec_compiler.rs
- Rule scope strings resolved to `IrRuleScope` variants
- Net class membership assigned from spec classes
- Component pads populated from spec pad geometry

**Tests**:
- **Test files**: `crates/autopcb-ir/tests/` (fixture-based, behind test-fixtures gate)
- **Test type**: integration
- **Backing**: user-specified
- **Scenarios**:
  - Minimal spec (1 component, 1 net, 1 rule) -> valid PcbIr
  - Spec with scoped width rule -> IrDesignRule.scope is NetClass("Power")
  - Spec with board outline -> IrBoardGeometry.outline populated
  - Spec with diff pairs -> IrNet.diff_pair_partner linked

**Code Intent**:
- New file `compile_error.rs`: `IrCompileError` enum with variants `NoBoardsDefined`, `DuplicateDesignator(String)`, `UnknownNet(String)`, `UnknownLayer(String)`, `UnknownRuleKind(String)`, `MissingBoardOutline`, `InvalidScope(String)`. Implements `std::error::Error` via `thiserror`. `spec_to_ir()` checks `spec.boards.is_empty()` first and returns `Err(IrCompileError::NoBoardsDefined)` — never panics on missing boards.
- New file `spec_compiler.rs` with `pub fn spec_to_ir(spec: &PcbDocSpec) -> Result<PcbIr, IrCompileError>`
- Internal helpers: `compile_layer_stack()`, `compile_nets()`, `compile_components()`, `compile_rules()`, `compile_copper()`, `compile_board_geometry()`
- Scope resolution: parse scope string -> match on `InNetClass(...)`, `OnLayer(...)`, `All` -> produce `IrRuleScope`
- Layer resolution: build `BTreeMap<String, LayerId>` from layer stack, resolve `LayerSpec` to `LayerId`
- Net resolution: build `BTreeMap<String, NetId>` from net list
- Re-export `spec_to_ir` from `lib.rs`
- Cargo.toml: `altium-format` is a dependency (used by extract.rs and pcbdoc_import.rs). Constraint is per-file: `spec_compiler.rs` must NOT contain `use altium_format` imports.

---

### Milestone 5: PcbDoc Import Adapter

**Files**:
- `crates/autopcb-ir/src/pcbdoc_import.rs` (NEW)
- `crates/autopcb-ir/src/spec_bridge.rs`

**Flags**: `conformance`

**Requirements**:
- `pub fn import_pcbdoc(board: &PcbDocBoard) -> Result<PcbDocSpec, IrCompileError>` converts PcbDoc data into a PcbDocSpec (uses same error type as `spec_to_ir()` per Decision Log)
- `load_ir_from_spec()` opens PcbDoc → calls `import_pcbdoc()` → merges spec mutations → calls `spec_to_ir()`. Spec file values overwrite imported values for overlapping fields.
- `extract.rs` is available for direct PcbDoc extraction. `spec_bridge` routes through `import_pcbdoc()` -> `spec_to_ir()`.
- Import adapter maps: PcbDoc components -> PcbDocComponentSpec, PcbDoc rules -> PcbDocRuleSpec (with scope strings in canonical `InNetClass(...)` / `OnLayer(...)` format), PcbDoc nets -> PcbDocNetSpec
- Import adapter stores coordinates as `Coord` values (deferred conversion); `spec_to_ir()` does the single Coord→mm conversion

**Acceptance Criteria**:
- `import_pcbdoc(board)` produces a PcbDocSpec that, when compiled via `spec_to_ir()`, produces a PcbIr matching the old `PcbIr::extract(board)` on these fields (within 1e-9 mm tolerance for f64): component positions, pad positions, net assignments, rule parameters, board outline vertices
- Scope strings from PcbDoc `DesignRule.scope`/`.scope2` are preserved in the PcbDocRuleSpec and resolved by `spec_to_ir()`

**Tests**:
- **Test files**: `crates/autopcb-ir/tests/` (fixture-based, behind test-fixtures gate)
- **Test type**: integration (roundtrip comparison)
- **Backing**: user-specified
- **Scenarios**:
  - Import PcbDoc -> compile -> compare component positions against extract.rs direct extraction path (within 1e-9 mm)
  - Import PcbDoc -> compile -> compare net assignments against legacy extract
  - Rule scope strings preserved through import and resolved to IrRuleScope

**Code Intent**:
- New `pcbdoc_import.rs`: `import_pcbdoc()` that reads PcbDocBoard and constructs PcbDocSpec
- Maps each PcbDoc concept to its spec equivalent, storing Coord values (not mm)
- Preserves scope strings from `DesignRule.scope` / `DesignRule.scope2` in canonical grammar
- Update `spec_bridge.rs::load_ir_from_spec()`: open PcbDoc → `import_pcbdoc(&board)` → merge spec → `spec_to_ir(&merged_spec)`. Merge strategy: spec file values overwrite import adapter values for any overlapping field (Decision: "Spec file wins on merge conflict")

---

### Milestone 6: Wire Up CLI and Viewer

**Files**:
- `crates/altium-cli/src/main.rs`
- `crates/autopcb-viewer/src/main.rs` (if applicable)

**Flags**: `conformance`

**Requirements**:
- CLI routing/placement commands call `load_ir_from_spec()`, which routes through `import_pcbdoc()` -> `spec_to_ir()` internally
- CLI code contains no direct `PcbIr::extract()` calls
- Existing CLI tests pass

**Acceptance Criteria**:
- `altium routing solve` exercises the new spec_to_ir() pipeline via `load_ir_from_spec()`
- All existing CLI tests pass
- `cargo test -p altium-cli` passes

**Tests**:
- **Test files**: `crates/altium-cli/src/main.rs` (inline)
- **Test type**: integration
- **Backing**: default-derived
- **Scenarios**:
  - All CLI tests pass
  - Minimal spec file with board outline and 1 component compiles and produces IR with non-empty components (verifies the spec_to_ir() pipeline end-to-end)

**Code Intent**:
- CLI calls `load_ir_from_spec()`, which internally routes through `import_pcbdoc()` -> `spec_to_ir()`
- Verify no direct `PcbIr::extract()` calls exist in CLI code
- Add one integration test with a minimal `.pcb` fixture that exercises the spec_to_ir() compilation pipeline end-to-end

---

### Milestone 7: Documentation

**Delegated to**: @agent-technical-writer (mode: post-implementation)

**Source**: `## Invisible Knowledge` section of this plan

**Files**:
- `crates/autopcb-ir/README.md`
- `crates/autopcb-ir/CLAUDE.md` (index update)

**Requirements**:
- README captures spec-to-IR architecture, data flow, scope resolution
- CLAUDE.md index updated with new modules

**Acceptance Criteria**:
- README.md exists in autopcb-ir directory
- Architecture diagram matches Invisible Knowledge section

## Milestone Dependencies

```
M1 (IrRuleScope) --> M2 (DrcPolicy scoped resolution)
                 --> M4 (spec_to_ir compiler)
M3 (spec geometry) --> M4
M4 --> M5 (PcbDoc import adapter)
M4 --> M6 (CLI/viewer wiring)
M5 + M6 --> M7 (docs)
```

**Parallel opportunities:**
- M1 and M3 can proceed in parallel (different crates)
- M2 can start after M1
- M5 and M6 can proceed in parallel after M4
