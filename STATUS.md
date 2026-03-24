# Codebase Status

Updated: 2026-03-23

## Crate Rename: autopcb-spec → autopcb-spec

The spec DSL crate has been renamed from `autopcb-spec` to `autopcb-spec`.
File extensions changed: `.sym`→`.sym`, `.sym`→`.sym`,
`.sch`→`.sch`, `.pcb`→`.pcb`, `.proj`→`.proj`.
Model types changed: `SchLibSpec`+`PcbLibSpec` → `SymSpec`.
Domain variants changed: `SpecDomain::SchLib`/`PcbLib`/`SchDoc`/`PcbDoc`/`PrjPcb`
→ `Sym`/`Sch`/`Pcb`/`Proj`.

## Three-Tier Pad Breakout System — Complete

Tiered pipeline in `crates/autopcb-router/src/detailed/fanout.rs` that pre-routes
short escape traces from dense SMD pads before the PathFinder negotiation loop.

| Tier | Function | Scope | Handles |
|------|----------|-------|---------|
| 1 | `plan_stubs()` | Any layer count | Dense SMD pads — 8-direction same-layer stubs with neckdown |
| 2 | `plan_perimeter_escapes()` | Any layer count | Peripheral packages (QFP/TQFP/SOP) — perpendicular outward escapes |
| 3 | `plan_via_escapes()` | >= 3 layers | BGA interior pads — via to inner layer (original algorithm) |

New types: `BreakoutRoute`, `BreakoutTier`, `ComponentKind`, `BreakoutPlan`.
`EscapeRoute`/`EscapePlan` are backward-compat aliases.
Neckdown formula: `max(pad_min_dim/2, trace_min_width)` (FreeRouting-validated).
Architecture documented in `crates/autopcb-router/src/detailed/README.md`.

## Routing Pipeline Integration

Full spec-to-routing pipeline: `routing solve` CLI command + routes loading in spec bridge + spec model `routing { }` block.

**New:**
- `routing { solution: "board.routes" }` spec block (convention: `<stem>.routes` if omitted)
- `altium routing solve --target board.PcbDoc board.pcb` → generates `.routes` file
- `load_ir_from_spec()` loads `.routes` and merges into `PcbIr.free_copper` (tracks/vias)
- `apply` injects routed tracks/vias from `.routes` into PcbDoc output
- Router `build_policy()` filters `Other`-kind rules instead of hard-erroring
- DRC violation records now fully populated in `.routes` files (was empty before)
- Pad rotation preserved through spec pipeline (was hardcoded to 0.0)

**Verified end-to-end:**
- `routing solve` on cobra board → `.routes` file with real traces
- `apply` injects routed primitives into PcbDoc from `.routes` file
- `routing inspect --verbose` shows detailed per-violation DRC records

**Known issues:**
- ShapeBasedRegions6 parse error on read-back of written PcbDocs (pre-existing write-path bug in region parameter serialization; file valid for Altium)
- Per-component merge not yet implemented (spec components replace ALL imported components including pads — see docs/plans/netlist-sync.md)

**Next:** Per-component merge (Phase 1 of netlist-sync plan) to preserve imported pad geometry through the merge, enabling routing on PcbDocs with ECO-applied netlists

## Spec-to-IR Pipeline: Full Rule Kind Coverage

All 70 Altium rule kinds now compile through `spec_to_ir()`. Previously only 40 were handled; the remaining 27 (SmdToPlane, FanoutControl, LayerPair, signal integrity rules, test point rules, etc.) caused hard errors that blocked placement solve on any PcbDoc containing them.

Also: unrecognized scope expressions (InPolygon, InNet, etc.) now fall back to `All` (global) instead of hard-erroring. This is conservative — treats rules as global rather than silently dropping them.

**Verified**: `placement solve` successfully runs on the ee-template hub board (32 components, 25 nets, 41 rules, solved in 2.2s).

## Spec Language: Function Calls and Shape Values

First-class function call syntax and geometric shape values in the spec language expression system.

**New language features:**
- `Expr::Call` with positional and named arguments: `rect(100mm, 50mm, center: (10mm, 0mm))`
- `Value::Shape` — geometric shapes as first-class values in expressions
- 14 builtin functions:
  - Geometry constructors: `rect()`, `rounded_rect()`, `circle()`, `polygon()`
  - Geometry operations: `inset()`, `outset()`, `translate()`
  - Shape accessors: `width()`, `height()`, `center()`
  - Math: `min()`, `max()`, `clamp()`, `abs()`
- Shape field access: `rect(100mm, 50mm).width` → `100mm`
- Board `outline:` property accepts `Value::Shape` or `Value::Array` of points
- 31 new tests (22 eval + 9 parser)

**Files changed:** ast.rs (CallArg, Expr::Call), parser.rs (parse_call_args), eval.rs (Shape enum, builtins), compiler.rs (outline extraction), formatter.rs (Call formatting), lib.rs (Shape re-export)

## Spec-to-IR Compiler Pipeline — Complete (All 7 Milestones)

Direct `PcbDocSpec → PcbIr` compilation pipeline replacing the old lossy `spec → PcbDoc mutation → extract` path.

**New modules in autopcb-ir:**
- `spec_compiler.rs` — `spec_to_ir()`: compiles PcbDocSpec directly to PcbIr (no altium_format imports)
- `compile_error.rs` — `IrCompileError` enum with fail-fast error handling (9 variants)
- `pcbdoc_import.rs` — `import_pcbdoc()`: PcbDoc→PcbDocSpec adapter + `merge_pcbdoc_spec()`
- `geometry.rs` — Shared geometry helpers (bounding boxes, arc tessellation)

**Key changes:**
- `IrDesignRule` has `scope: IrRuleScopePair` with `IrRuleScope` variants: All, NetClass, Layer, NetClassAndLayer
- `DrcPolicy` resolves width/via/clearance rules by scope cascade: NetClassAndLayer > NetClass > Layer > All
- `BoardSpec` has geometry fields (outline, keepouts, layers); `PcbDocRuleSpec` has scope2; components have pad geometry
- `load_ir_from_spec()` routes through `import_pcbdoc() → spec_to_ir()` pipeline
- CLI uses `load_ir_from_spec()` exclusively (no direct `PcbIr::extract()` calls)
- Net class membership assigned from `PcbDocClassSpec.members`
- Rule property values fail-fast on malformed input (no silent 0.0 defaults)

## Workspace Overview

Rust workspace for reading, writing, querying, rendering, and specifying Altium Designer files — plus automated PCB placement/routing and viewer.

```
altium-format-types    (domain types, enums, constants — zero deps)
altium-format-derive   (proc macros: FromParams, ToParams, OpsSchema, OpsEnum)
altium-format          (core: parsing, serialization, high-level API, rendering infra)
  ├→ altium-format-query       (AQL query language engine)
  ├→ altium-format-render-svg  (SVG rendering backend)
  └→ altium-format-render-png  (PNG rasterization via resvg)
autopcb-spec           (spec DSL: compiler, executor, reconciler, dump)
altium-cli             (CLI binary)
autopcb-ir             (PCB intermediate representation, mm-based, serde JSON)
autopcb-placement      (simulated annealing placer with pin/part swap)
autopcb-viewer         (egui/wgpu 2D+2.5D viewer, spec-centric)
autopcb-shell          (IDE shell with dock layout, command palette, IPC)
```

## Document Type Support

| Document   | Ext     | Parse | Serialize | High-Level API | Spec | Query | Render    | CLI validate | CLI save-as | CLI new |
|------------|---------|-------|-----------|----------------|------|-------|-----------|--------------|-------------|---------|
| **SchLib** | .SchLib | ✅    | ✅        | ✅ Full CRUD   | ✅   | ✅    | ✅ SVG/PNG | ✅           | ✅          | ✅      |
| **SchDoc** | .SchDoc | ✅    | ✅        | ✅ Read/Write  | ✅   | ✅    | ✅ SVG/PNG | ✅           | ✅          | ✅      |
| **PcbLib** | .PcbLib | ✅    | ✅        | ✅ Full CRUD   | ✅   | ✅    | ✅ SVG/PNG | ✅           | ✅          | ✅      |
| **PcbDoc** | .PcbDoc | ✅    | ✅        | ✅ Read/Write  | ✅   | ✅    | ❌        | ✅           | ✅          | ❌      |
| **PrjPcb** | .PrjPcb | ✅    | ✅        | ✅ Read-only   | ✅   | ❌    | ❌        | ✅           | ✅          | ✅      |
| **IntLib** | .IntLib | ✅    | ❌        | ❌ Read-only   | ❌   | ❌    | ❌        | ✅           | ✅ dump     | ❌      |

## CLI Command Matrix

| Command       | SchLib | SchDoc | PcbLib | PcbDoc | PrjPcb | IntLib |
|---------------|--------|--------|--------|--------|--------|--------|
| `new`         | ✅     | ✅     | ✅     | ❌     | ✅     | ❌     |
| `validate`    | ✅     | ✅     | ✅     | ✅     | ✅     | ✅     |
| `save-as`     | ✅     | ✅     | ✅     | ✅     | ✅     | ❌     |
| `get version` | ✅     | ❌     | ✅     | ❌     | ❌     | ❌     |
| `render`      | ✅     | ✅     | ✅     | ❌     | ❌     | ❌     |
| `query`       | ✅     | ✅     | ✅     | ✅     | ❌     | ❌     |
| `info`        | ✅     | ✅     | ✅     | ✅     | ❌     | ❌     |
| `plan/apply`  | ✅     | ✅     | ✅     | ✅     | ✅     | ❌     |
| `dump`        | ✅     | ✅     | ✅     | ✅     | ✅     | ✅     |
| `cfb *`       | ✅     | ✅     | ✅     | ✅     | n/a    | ✅     |

Additional commands: `spec sync` (forward/diff/dry-run), `placement autoplace/dump/plan/apply`, `inspect ir-json`, `cfb ls/dump/blocks/diff/cat`.

## Per-Document Notes

### SchLib — Most Complete
All schematic record types parsed. Full CRUD API. 9 per-component sidecar streams. Complete roundtrip with semantic CFB diff verification. Spec dump/compile/plan/apply/reconcile all working.

### SchDoc
All 40+ record types parsed. Flat OWNERINDEX → nested tree conversion. UniqueId-based field preservation on save. Spec supports sheet metadata, all object types, `net`/`power` blocks, pin connections (`pin X -> #NET`), and SchLib import references.

### PcbLib
8 primitive types (Pad, Via, Track, Arc, Text, Fill, Region, ComponentBody). 6 sidecar stream types. Complete roundtrip. PadStack/PcbContour shared types with PcbDoc. Spec supports pad templates and spread operators.

### PcbDoc
18+ section types parsed. DRC engine with 39 rule classes and 38 violation classes. V2 API: LayerStack, RuleParams, PadStack, BoardGeometry, BoardConnectivity. 94/96 V6 test files passing. PrimitiveParameters (BOM data) pipeline complete.

### PrjPcb
INI-style format with indexed sections. Complete roundtrip. Read-only high-level API (internal write exists but not surfaced).

### IntLib
Read-only. Decompresses embedded SchLib/PcbLib from CFB. Dump produces `.sym` files.

## Spec Language (autopcb-spec)

Full pipeline: lexer → parser → AST → compiler → SpecModel → executor/reconciler/dump.

Features: let bindings, arithmetic, units (`100mil`, `2.54mm`), color literals, spread operators, templates, pad expansion, multi-part components, pin anchoring, import resolution, annotations (Altium UniqueID format).

**Sync IR:** SchDoc↔PcbDoc spec synchronization via `SyncSnapshot` intermediate representation. Direction-aware filtering, pin variant hard-errors, remove-before-add ordering.

**Validators:** Duplicate designators, dangling net refs, duplicate annotation IDs, unresolved pin refs. Cross-file duplicate ID detection.

**Constraints:** 5 typed kinds (edge_placement, directional, near, region, fixed_position). PcbDoc rules with scope expressions.

## AutoPCB Placer — Complete

Full pipeline: `.pcb` → constraints → solve → rewrite spec → apply to `.PcbDoc`.

| Phase | Description |
|-------|-------------|
| 1+2   | Analytical solver (solverang LM) + legalization |
| 2.5   | Greedy part swap (optional) |
| 3     | SA refinement with adaptive cooling, rotation-aware overlap, spatial grid |
| 4.5   | Greedy pin swap (optional) |

Spec extensions: `autoplace`, `group`, `separate`, `unplaced` directives. AST-based spec rewriter with trivia/comment preservation.

## Placer Pipeline Gap Fixes — Complete

| Fix | Status |
|-----|--------|
| Board6 merge (preserves layer stack) | ✅ |
| Pin→Pad resolution via SchLib FootprintMap | ✅ |
| Pad-Net assignment from sibling .sch specs | ✅ |
| Connections6 star-topology ratsnest | ✅ |
| Footprint graphics instantiation (tracks/arcs/fills/texts/regions/bodies) | ✅ |
| SOURCEUNIQUEID (read/write roundtrip, not yet populated for new components) | Partial |
| Layer name abbreviation (TOP/BOTTOM) | ✅ |
| Classes6 auto-generation (All Components/All Nets) | ✅ |
| Swap group IDs (SchLib → PcbDoc → IR) | ✅ |
| PrimitiveParameters BOM data pipeline | ✅ |
| Import-based spec redesign | Deferred (needs RFC) |
| UniqueID rebuild for new pads | ✅ |

## AutoPCB Viewer

Spec-centric: accepts ONLY `.pcb` files. No `altium-format` dependency.

- 2D: layer-colored tracks, keepout zones, copper pours, fills, cutouts, net highlighting
- 2.5D: extruded board/copper/vias/components with orbit camera and Lambertian shading
- File watch (`--watch`): auto-reload on spec/PcbDoc/playback changes with debounce
- Screenshot mode: `--screenshot output.png`
- Bridge: `autopcb-ir::spec_bridge::load_ir_from_spec()`

## AutoPCB Shell (IDE) — Phase 1 Complete

egui-based IDE shell with: `egui_tiles` dock layout, command palette (`Ctrl+Shift+P`), multi-document workbench, activity bar, VSCode Dark theme, split editor groups, keyboard shortcuts editor, filesystem explorer, IPC control server (`autopcb-shell.sock`).

Phase 2+ planned: full command catalog, undo/redo, file watcher, job system, 3D render, spec editor with diagnostics.

## Known Issues

**Moderate:**
- PrjPcb has no public write API (internal write exists)
- PcbDoc rendering not supported (no SVG/PNG)

**Minor:**
- PcbDoc: 2/96 V6 files failing (EmbeddedFonts, WideStrings edge cases — see PCBDOC-next.md)
- PcbDoc V5 format not supported (2 test files deferred)
- SVG clip regions not applied
- `get version` only works for SchLib/PcbLib
- `apply --report-json` flag accepted but unused
- SOURCEUNIQUEID not populated from SchDoc for new components during apply

## Roundtrip Known Differences (Acceptable)

All document types: font name buffer zero-fill (vs Altium heap garbage), boolean normalization (non-zero → 0x01).

PcbLib-specific: text WideStrings upgrade, via format upgrade (ext_size 42→45), SharedUnion NUL terminator.

PcbDoc-specific: pad sub4 format upgrade (171→172 bytes), via section 4/5 always written, Rules6 tier2 serialization, param key ordering, duplicate param deduplication.
