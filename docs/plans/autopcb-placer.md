# AutoPCB Placer: Downstream Plumbing Plan

## Overview

The autopcb analytical placement solver (Phases 1, 2, 4) is implemented. The
spec language parses placement blocks. This plan finishes the **downstream
plumbing** that connects spec parsing to the solver, enables spec rewriting,
adds SA refinement, wires in the viewer, and implements pin/part swap
optimization.

Approach: **Parallel Waves**. Wave 1 handles independent tasks (parser
extensions, dump, reconciler, viewer file watch). Wave 2 wires the sequential
bridge→executor→rewriter chain. Wave 3 adds SA and swap optimization. Each wave
produces a deployable increment.


## Planning Context

### Decision Log

| Decision | Reasoning Chain |
|----------|----------------|
| Parallel waves over sequential | Tasks 1/5/6/9 have zero dependencies → can start in parallel → reduces wall-clock time by ~40% vs sequential → Wave 2 depends only on Wave 1 parser, Wave 3 only on Wave 2 |
| Spec-to-spec transformer (not PcbDoc mutation) | User requirement: autoplacer edits .pcb files → preserves human-readable output → existing reconciler applies spec to PcbDoc binary → clean separation of concerns |
| `autoplace: true` → `at: (x,y)` rewriting | Autoplacer reads partial spec with autoplace directives → solves → rewrites same file with explicit positions → user can review/tweak → re-run → iterate → idempotent |
| Swap overlay file (not inline schematic edit) | Pin/part swaps change net-to-pin mapping → modifying user's schematic spec directly risks data loss → overlay file imported by `.pcb` spec → user can delete import to undo all swaps → safe and reviewable |
| Viewer watches iterations.json (not live IPC) | Live solver streaming requires IPC infrastructure → file watching with `notify` crate is 10 lines → autoplacer writes snapshots incrementally → viewer polls file → sufficient for interactive feedback → can upgrade to IPC later |
| SA as opt-in (`algorithm: analytical` default) | MVP autoplacer works with analytical solver alone → SA adds ~800-1500 lines → making it optional means MVP ships faster → users opt in via `algorithm: full_pipeline` |
| Grid-based spatial index over R-tree | PCB scale N<500 → grid is simpler (HashMap<(i32,i32), Vec>) → O(k) neighbor lookup sufficient → R-tree complexity only justified at VLSI scale N>10K |
| BFS clustering over spectral for MVP | BFS is O(N+E), deterministic, ~100 lines → spectral requires eigenvalue computation → BFS sufficient for N<100 → spectral available as upgrade |
| Regenerate spec file over AST round-trip rewriting | AST round-trip requires preserving every whitespace/comment token → significant parser infrastructure → regenerating from PlacementSpec model is simpler → comments preserved via `// autoplace: solved` annotations → user constraints preserved verbatim |
| `petgraph` for Phase 0 graph | Standard Rust graph library → BFS/DFS built-in → used by many EDA tools → no custom graph needed |
| Reconciler position tolerance 0.01mm / 0.1° | Altium internal coords = 10,000 units per mil → 1 mil = 0.0254mm → coordinate round-trip through Coord→f64→Coord introduces ≤0.003mm error → 0.01mm threshold (3× round-trip error) catches real moves while ignoring encoding artifacts → 0.1° = Altium's minimum rotation granularity in UI, sub-0.1° differences are serialization rounding |
| Parser spans on all AST nodes for rewriter | M6 spec rewriter needs byte offsets to locate `place` blocks in source text → without spans, rewriter cannot perform targeted replacements → must add span fields in M1 when defining AST nodes → amortized cost is near-zero (lexer already tracks positions) |
| Swap group data sourced from PcbIr (not separate SchLib parameter) | PcbDoc stores back-annotated pin swap groups from SchLib on component records → PcbIr extraction already captures pad metadata → swap_id_pin/part/pair should be extracted into IrComponentPad during IR extraction → avoids needing SchLib as separate input → if PcbIr extraction doesn't yet carry swap IDs, M8 must extend `IrComponentPad` to include them |

### Rejected Alternatives

| Alternative | Why Rejected |
|-------------|-------------|
| Direct PcbDoc binary mutation | Loses human-readable output → can't review placement decisions → violates spec-as-source-of-truth architecture |
| Live solver IPC (Unix socket/pipe) | Over-engineering for MVP → file watching achieves same result with 10× less code → upgrade path exists |
| Full AST round-trip rewriter | Requires whitespace-preserving token stream in parser → large effort → regenerating spec from model is sufficient |
| Spectral clustering for MVP Phase 0 | Power iteration adds complexity → BFS gives 80% quality for 20% effort → spectral available as upgrade |
| SA as mandatory in pipeline | Doubles solve time → not all boards need it → opt-in via config is cleaner |

### Constraints & Assumptions

- solverang crate is external at `~/git/solverang/` (path dependency)
- Spec language lexer/parser/compiler/executor/reconciler/dump all exist and work
- PlacementSpec model exists in model.rs with all place properties
- `solve_placement()` in autopcb-placement accepts `Vec<UserConstraint>` and returns `PlacementResult`
- SchLib pin swap fields (`swap_id_pin`, `swap_id_part`, `swap_id_pair`) are fully parsed
- Test fixtures available in `data/pcbdoc/` (108 files)
- Viewer uses egui + wgpu, has snapshot playback with `--playback iterations.json`

### Known Risks

| Risk | Mitigation | Anchor |
|------|-----------|--------|
| Spec regeneration loses user formatting | Preserve user-written constraints verbatim; only modify/add `place` blocks with autoplace results; add `// autoplace: solved` comments for traceability | docs/future/solverang/autoplacer-spec-integration.md §6 |
| SA convergence on real boards | Start from Phase 2 legal placement (warm start) → SA only refines → worst case = returns Phase 2 result unchanged | docs/future/solverang/sa-implementation-spec.md §8 |
| O(N²) clearance constraints for large boards | Spatial grid pruning reduces to O(N·k) where k≈10-20 neighbors → only needed for N>200 | docs/future/solverang/placement-algorithms.md §10 |
| Pin swap creates invalid netlist | Verify swap integrity: net count unchanged, pin count per net unchanged, only swap within same swap group | docs/future/solverang/pin-part-swap-spec.md §6 |


## Invisible Knowledge

### Architecture

```
User writes .pcb (partial, with autoplace: true)
         │
         ▼
┌─────────────────────────────────────────────────┐
│  autopcb-spec PARSER                       │
│  parse .pcb → PlacementSpec model       │
│  (lexer → tokens → AST → compiler → model)      │
└──────────────────────┬──────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────┐
│  CONSTRAINT BRIDGE (new)                         │
│  PlacementSpec → Vec<UserConstraint>            │
│  locked components → FixedPositionConstraint    │
│  autoplace components → solver variables        │
└──────────────────────┬──────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────┐
│  autopcb-placement SOLVER (existing)             │
│  Phase 0: clustering (new, optional)            │
│  Phase 1: analytical (solverang, existing)       │
│  Phase 2: legalization (existing)                │
│  Phase 3: SA refinement (new, optional)          │
│  Phase 4: final refinement (existing)            │
│  Phase 4.5: swap optimization (new, optional)   │
└──────────────────────┬──────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────┐
│  SPEC REWRITER (new)                             │
│  PlacementResult → updated .pcb         │
│  autoplace:true → at:(x,y) + rotation:N        │
│  + board-swaps.sch overlay              │
└──────────────────────┬──────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────┐
│  autopcb-spec RECONCILER (existing)        │
│  `altium placement apply` reads final spec      │
│  writes component positions to .PcbDoc binary    │
└─────────────────────────────────────────────────┘
```

### Data Flow

```
.pcb ──parse──→ PlacementSpec
    +                       │
.PcbDoc ──extract──→ PcbIr  │
                       │    │
                       ▼    ▼
              solve_placement(ir, constraints, config)
                       │
                       ▼
              PlacementResult { components, snapshots, hpwl }
                       │
              ┌────────┼────────┐
              ▼        ▼        ▼
         .pcb  .json    .sch
         (positions)  (viewer)  (swap overlay)
```

### Why This Structure

The spec-as-intermediate-representation pattern means:
- The solver never touches PcbDoc binaries (separation of concerns)
- Users can inspect, modify, and version-control placement decisions
- The reconciler (already built) handles the binary write
- Iterative refinement is natural: edit spec, re-solve, review

### Invariants

- A component with `at:` and no `autoplace: true` is LOCKED — solver must not move it
- A component with `autoplace: true` is a solver variable
- Unmentioned components follow the `unplaced:` strategy (default: autoplace)
- Pin/part swaps only within same swap group — never cross groups
- After swap, net connectivity graph is topologically equivalent to original
- Spec rewriting never modifies user-written constraints, groups, or rules

### Tradeoffs

- **Regenerate vs round-trip rewrite**: Chose regeneration for simplicity. Cost: user comments in `place` blocks may not survive. Benefit: 10× less parser infrastructure needed.
- **File watching vs live IPC**: Chose file watching for simplicity. Cost: ~250ms polling latency. Benefit: no IPC infrastructure, works cross-platform.
- **BFS vs spectral clustering**: Chose BFS for MVP. Cost: may produce unbalanced clusters on complex boards. Benefit: zero dependencies, deterministic, 100 lines.


## Milestones

### Milestone 1: Spec Parser Extensions + Model Updates

**Files**:
- `crates/autopcb-spec/src/model.rs`
- `crates/autopcb-spec/src/ast.rs`
- `crates/autopcb-spec/src/parser.rs`
- `crates/autopcb-spec/src/compiler.rs`
- `crates/autopcb-spec/src/lexer.rs`

**Flags**: `conformance`

**Requirements**:
- Add `autoplace: bool` field to `PlacementPlaceSpec`
- Add `no_pin_swap: Vec<String>` and `no_part_swap: bool` fields to `PlacementPlaceSpec`
- Add `AutoplaceConfig` struct (algorithm, sa_cooling, sa_moves_per_temp, sa_max_steps, enable_net_crossings, default_clearance, board_edge_clearance, grid_snap, auto_cluster)
- Add `autoplace_config: Option<AutoplaceConfig>` to `PlacementSpec`
- Add `unplaced: UnplacedStrategy` enum (Autoplace, Ignore, Error) to `PlacementSpec`
- Add `allow_pin_swap: bool`, `allow_part_swap: bool`, `allow_gate_swap: bool` to `PlacementSpec`
- Parse `group NAME { components: [...] }` declaration → add `PlacementGroupSpec` to model
- Parse `separate $group_a, $group_b { gap: Nmm }` → add to `PlacementConstraintSpec`
- Parse `autoplace { ... }` block inside `placement { ... }`
- Parse `unplaced: autoplace | ignore | error` property
- All new AST node types must carry `Span { start: usize, end: usize }` byte offsets populated from lexer token positions (required by M6 spec rewriter for targeted text replacement)

**Acceptance Criteria**:
- `placement { place U1 { autoplace: true, region: center } }` parses without error
- `placement { autoplace { algorithm: full_pipeline, grid_snap: 0.5mm } }` parses
- `placement { unplaced: autoplace }` parses to `UnplacedStrategy::Autoplace`
- `placement { group analog { components: [U5, R10, C20] } }` parses
- `placement { separate $analog, $digital { gap: 8mm } }` parses
- Existing spec tests still pass

**Tests**:
- **Test files**: `crates/autopcb-spec/src/parser.rs` (inline `#[cfg(test)]`)
- **Test type**: unit
- **Backing**: doc-derived (from spec-grammar.md)
- **Scenarios**:
  - Normal: parse complete placement block with all new properties
  - Edge: empty autoplace block `autoplace {}`, missing optional fields
  - Error: `autoplace: invalid_value` produces clear error message

**Code Intent**:
- `model.rs`: Add `autoplace: bool` to `PlacementPlaceSpec`, add `AutoplaceConfig` struct, add `UnplacedStrategy` enum, add `PlacementGroupSpec` struct, add `groups: Vec<PlacementGroupSpec>` + `unplaced: UnplacedStrategy` + `autoplace_config: Option<AutoplaceConfig>` + swap allow flags to `PlacementSpec`
- `ast.rs`: Add `GroupDecl` and `SeparateDecl` variants to `PlacementItem`, add `AutoplaceBlock` variant. All new AST node types must carry `Span { start: usize, end: usize }` byte offsets from source (M6 rewriter depends on these spans for targeted text replacement)
- `lexer.rs`: Add `group` and `separate` as new keywords (`swap_group` exists as a single token but `group` alone is NOT a keyword — verified in lexer.rs)
- `parser.rs`: Extend `parse_placement_item()` to handle `group`, `separate`, `autoplace` blocks; extend `parse_placement_place_property()` to handle `autoplace`, `no_pin_swap`, `no_part_swap`
- `compiler.rs`: Extend `compile_placement_decl()` to compile group declarations, separate constraints, autoplace config, unplaced strategy

---

### Milestone 2: Placement Dump (pcbdoc → spec)

**Files**:
- `crates/autopcb-spec/src/dump.rs`

**Requirements**:
- `dump_pcbdoc()` emits a `placement { ... }` block containing a `place` declaration for each component
- Each `place` block includes `at: (x, y)` and `rotation: N`
- Components are sorted by designator for stable output
- Board clearance rules are emitted as `clearance { all: N }` if available

**Acceptance Criteria**:
- Load a PcbDoc, dump to spec, verify output contains `placement { place U1 { at: (...), rotation: 0 } }`
- Dumped spec can be re-parsed by the parser without errors
- All components in PcbDoc appear in dumped spec

**Tests**:
- **Test files**: `crates/autopcb-spec/src/dump.rs` (inline `#[cfg(test)]`)
- **Test type**: unit + integration (with `test-fixtures` flag)
- **Backing**: default-derived
- **Scenarios**:
  - Normal: dump a PcbDoc with 5 components, verify all appear
  - Edge: component with 270° rotation serialized correctly
  - Roundtrip: dump → parse → compare model fields

**Code Intent**:
- `dump.rs`: In `dump_pcbdoc_board()` (or new helper), iterate `board.components`, emit `place DESIGNATOR { at: (x_mm, y_mm), rotation: DEG }` for each. Sort by designator. If board has component clearance rules, emit `clearance { all: Nmm }`.

---

### Milestone 3: Reconciler Placement Comparison

**Files**:
- `crates/autopcb-spec/src/reconciler.rs`

**Requirements**:
- `reconcile_pcbdoc()` compares component positions from spec vs PcbDoc
- Reports MOVE ECO entries for components whose position or rotation differs
- Tolerance: 0.01mm for position, 0.1° for rotation
- Applies position/rotation changes to PcbDoc when reconciling

**Acceptance Criteria**:
- Spec with `place U1 { at: (10mm, 20mm) }` vs PcbDoc with U1 at (5mm, 5mm) → ECO reports MOVE
- Spec with `place U1 { at: (10mm, 20mm) }` vs PcbDoc with U1 at (10mm, 20mm) → no change reported
- After apply, PcbDoc component position matches spec

**Tests**:
- **Test files**: `crates/autopcb-spec/src/reconciler.rs` (inline `#[cfg(test)]`)
- **Test type**: unit
- **Backing**: default-derived
- **Scenarios**:
  - Normal: component moved, rotation changed
  - Edge: within tolerance (0.005mm difference) → no change
  - Error: designator in spec doesn't exist in PcbDoc → warning

**Code Intent**:
- `reconciler.rs`: In `reconcile_pcbdoc()`, after existing net/component reconciliation, iterate `spec.placement.places`. For each place with `at:`, find component by designator in PcbDoc, compare position and rotation with tolerance. If different, add ECO entry and update component position/rotation in the PcbDoc model.

---

### Milestone 4: Viewer File Watch + Spec Overlay

**Files**:
- `crates/autopcb-viewer/Cargo.toml`
- `crates/autopcb-viewer/src/main.rs`
- `crates/autopcb-viewer/src/app.rs`

**Requirements**:
- Add `notify` crate dependency for file system watching
- Watch the input PcbDoc file for changes; reload IR on modification
- If `--playback` JSON file specified, watch it too; reload snapshots on change
- New `--watch` flag enables continuous watching (default: off)
- Display "Reloaded" indicator in sidebar when file changes detected

**Acceptance Criteria**:
- `autopcb-viewer board.PcbDoc --watch` opens viewer
- Externally modifying `board.PcbDoc` triggers reload within 500ms
- Viewer renders updated component positions after reload
- `autopcb-viewer board.PcbDoc --playback iter.json --watch` reloads both files

**Tests**:
- Skip automated tests (GUI component, manual verification)
- **Skip reason**: File watcher + egui interaction requires manual testing

**Code Intent**:
- `Cargo.toml`: Add `notify = "7"` dependency
- `main.rs`: Accept `--watch` CLI flag. If enabled, create `notify::RecommendedWatcher` watching input paths. Pass watcher handle to ViewerApp.
- `app.rs`: In `ViewerApp::update()`, check if watcher fired a change event (via `mpsc::Receiver`). If yes: re-open PcbDoc, re-extract IR, rebuild GPU scene resources, optionally reload playback JSON. Show "Reloaded at HH:MM:SS" text in sidebar.

---

### Milestone 5: PlacementSpec → UserConstraint Bridge

**Files**:
- `crates/autopcb-spec/src/executor.rs`

**Flags**: `needs-rationale`, `error-handling`

**Requirements**:
- New function `placement_spec_to_constraints(spec: &PlacementSpec, ir: &PcbIr) -> Result<(Vec<UserConstraint>, Vec<String>), SpecError>` returning constraints and list of autoplace designators
- Locked components (has `at:`, no `autoplace: true`) → `UserConstraint::FixedPosition`
- `edge:` → `UserConstraint::EdgePlacement`
- `near:` → `UserConstraint::Near`
- `region:` → `UserConstraint::RegionContainment`
- `left_of` / `right_of` / `above` / `below` → `UserConstraint::Directional`
- `unplaced: autoplace` → components in PcbDoc not in spec added as autoplace variables
- `unplaced: error` → error if any PcbDoc component not in spec
- `unplaced: ignore` → unmentioned components get `FixedPosition` at current location
- Designator in spec but NOT in PcbIr: if `unplaced: error`, return SpecError listing unknown designators; otherwise log warning and skip (do not add as solver variable or fixed position)
- Returns the set of designators that should be auto-placed (for the solver)

**Acceptance Criteria**:
- PlaceSpec with `at: (10, 20)` and no `autoplace` → produces `FixedPosition { x: 10, y: 20 }`
- PlaceSpec with `autoplace: true, edge: top, inset: 2mm` → produces `EdgePlacement { edge: Top, inset: 2.0 }`
- PlaceSpec with `autoplace: true, near: $U1, max_distance: 5mm` → produces `Near { max_distance: 5.0 }`
- Unplaced strategy `Error` with unmentioned component → returns SpecError
- PlaceSpec for designator not in PcbIr + unplaced: ignore → warning emitted, no constraint added

**Tests**:
- **Test files**: `crates/autopcb-spec/src/executor.rs` (inline `#[cfg(test)]`)
- **Test type**: unit
- **Backing**: doc-derived (from autoplacer-spec-integration.md)
- **Scenarios**:
  - Normal: mixed locked + autoplace components produce correct constraints
  - Edge: all components locked → empty autoplace set, all FixedPosition constraints
  - Edge: designator in spec not in PcbIr → warning, no constraint added
  - Error: unplaced: error with missing component

**Code Intent**:
- `executor.rs`: New function `placement_spec_to_constraints()`. Iterate `spec.places`: if `fixed` or has `at:` without `autoplace`, emit `FixedPosition`. If `autoplace: true`, emit constraints based on properties (edge→EdgePlacement, near→Near, region→RegionContainment). Iterate `spec.constraints`: compile directional constraints to `Directional`. Handle `unplaced` strategy by cross-referencing spec designators against PcbIr component list. Return `(constraints, autoplace_designators)`.

---

### Milestone 6: Executor Integration + Spec Rewriter

**Files**:
- `crates/autopcb-spec/src/executor.rs`
- `crates/autopcb-spec/src/spec_rewriter.rs` (new)

**Flags**: `needs-rationale`, `complex-algorithm`

**Requirements**:
- New function `autoplace_spec(spec_path: &Path, pcbdoc_path: &Path) -> Result<AutoplaceReport>`
- Reads spec file, parses PlacementSpec
- Opens PcbDoc, extracts PcbIr
- Calls `placement_spec_to_constraints()` (Milestone 5)
- Calls `solve_placement()` from autopcb-placement
- Optionally calls SA refinement if `algorithm: full_pipeline`
- Writes updated spec file: replaces `autoplace: true` with `at: (x, y)` + `rotation: N`
- Preserves all non-autoplace content (constraints, groups, rules, clearance, optimize)
- Multi-designator `place C1, C2, C3 { autoplace: true }` expanded to individual blocks
- Unmentioned autoplace components appended at end of placement block
- Returns `AutoplaceReport` with HPWL, component count, changes made

**Acceptance Criteria**:
- Input spec with `place U1 { autoplace: true, region: center }` → output spec has `place U1 { at: (x, y), rotation: N, region: center }`
- Locked components appear unchanged in output
- Output spec can be re-parsed without errors
- Output spec applied via `altium placement apply` produces valid PcbDoc

**Tests**:
- **Test files**: `crates/autopcb-spec/src/executor.rs`, `crates/autopcb-spec/src/spec_rewriter.rs`
- **Test type**: integration (with `test-fixtures` flag)
- **Backing**: doc-derived
- **Scenarios**:
  - Normal: spec with 3 autoplace + 2 locked → output has all 5 with positions
  - Edge: all components locked → output identical to input
  - Roundtrip: autoplace → re-parse output → verify PlacementSpec fields

**Code Intent**:
- `executor.rs`: New `autoplace_spec()` orchestrator function. Opens PcbDoc, extracts IR, calls bridge, calls solver, calls rewriter. Add `AutoplaceReport` struct.
- `spec_rewriter.rs` (new): Function `rewrite_spec_with_placement(original_spec_text: &str, spec: &PlacementSpec, result: &PlacementResult) -> String`. Strategy: re-parse original text to identify `place` block locations. For each autoplace component in result, generate new `place DESIGNATOR { at: (x, y), rotation: N }` text. Replace corresponding section in original. Append unmentioned components. Preserve everything else verbatim using byte offsets from parser spans.

---

### Milestone 7: Simulated Annealing (Phase 3)

**Files**:
- `crates/autopcb-placement/src/simulated_annealing.rs` (new)
- `crates/autopcb-placement/src/lib.rs`
- `crates/autopcb-placement/Cargo.toml`

**Flags**: `complex-algorithm`, `performance`, `needs-rationale`

**Requirements**:
- New `SAConfig` struct with temperature schedule, move probabilities, cost weights
- New `refine_with_sa(initial: &PlacementResult, ir: &PcbIr, config: &SAConfig) -> Result<PlacementResult>`
- Move types: Displace, Swap, Rotate (90/180/270), Slide
- Cost function: exact HPWL + overlap penalty + board containment rejection
- Incremental cost evaluation via `NetComponentIndex` (component → affected nets)
- `SpatialGrid` for O(k) neighbor overlap checking
- Auto-init temperature from sample moves (80% initial acceptance)
- Adaptive cooling (alpha adjusts based on acceptance rate)
- Stopping: T < t_frozen OR acceptance < 1% for 5 steps
- Records `PlacementIterationSnapshot` periodically for viewer playback
- Wired into `solve_placement()` when `PlacementConfig` includes SA config

**Acceptance Criteria**:
- SA on a 50-component board completes in <1s
- SA HPWL ≤ Phase 2 HPWL (never makes things worse — best-tracking)
- SA with 0 moves_per_temp returns Phase 2 result unchanged
- SA snapshots can be loaded by autopcb-viewer for playback

**Tests**:
- **Test files**: `crates/autopcb-placement/src/simulated_annealing.rs` (inline `#[cfg(test)]`)
- **Test type**: unit + integration
- **Backing**: doc-derived (from sa-implementation-spec.md)
- **Scenarios**:
  - Unit: HPWL computation for known 4-pin net = expected value
  - Unit: AABB overlap detection with known geometry
  - Unit: Metropolis acceptance at T=∞ always accepts
  - Unit: Metropolis acceptance at T=0 rejects uphill moves
  - Integration (test-fixtures): SA on real PcbDoc, HPWL ≤ Phase 2 HPWL

**Code Intent**:
- `Cargo.toml`: Add `rand = "0.8"` dependency
- `lib.rs`: Add `pub mod simulated_annealing;`, add `sa_config: Option<SAConfig>` to `PlacementConfig`, call `refine_with_sa()` after Phase 2 if SA enabled
- `simulated_annealing.rs` (new ~800-1200 lines): `SAConfig` with `Default` impl, `Placement` state struct (components with cached pad offsets, spatial grid, net-component index), `Move` enum, `generate_move()`, `apply_move()`/`revert_move()`, `delta_cost()` (incremental HPWL + overlap), `refine_with_sa()` main loop with Metropolis acceptance, `SpatialGrid` (HashMap<(i32,i32), Vec<usize>>), `NetComponentIndex` (bidirectional comp↔net mapping)

---

### Milestone 8: Pin/Part Swap Optimization

**Files**:
- `crates/autopcb-placement/src/swap.rs` (new)
- `crates/autopcb-placement/src/lib.rs`
- `crates/autopcb-placement/src/simulated_annealing.rs`
- `crates/autopcb-ir/src/component.rs` (conditional: only if `IrComponentPad` lacks swap ID fields)
- `crates/autopcb-ir/src/extract.rs` (conditional: only if swap ID extraction not yet implemented)

**Flags**: `complex-algorithm`, `needs-rationale`

**Requirements**:
- New `SwapModel` struct built from PcbIr pad data (swap_id_pin, swap_id_part, swap_id_pair fields on `IrComponentPad` in `crates/autopcb-ir/src/component.rs`). If PcbIr does not yet carry swap IDs, extend `IrComponentPad` to include them and update extraction in `crates/autopcb-ir/src/extract.rs` to populate from PcbDoc back-annotated component records.
- `pin_swap_groups`: (component, group_id) → swappable pin indices
- `part_swap_groups`: group_id → list of components with identical pinouts
- Greedy pin swap sweep: after Phase 4, try all pin swaps, accept HPWL improvements
- Greedy part swap pass: after Phase 2, try all pairwise part swaps
- `PinSwap` and `PartSwap` as SA move types (integrated into Phase 3 if enabled)
- `SwapChangelog` output: records all applied swaps with HPWL improvement
- Swap overlay file generation: writes `board-swaps.sch` with net reassignments
- Validation: verify net connectivity preserved after all swaps

**Acceptance Criteria**:
- Swap model correctly identifies pin swap groups from SchLib data
- Pin swap on symmetric component (resistor) correctly exchanges nets
- Part swap between identical resistors exchanges all net assignments
- SwapChangelog lists all swaps with positive HPWL improvement
- Net count and pin-count-per-net unchanged after swaps

**Tests**:
- **Test files**: `crates/autopcb-placement/src/swap.rs` (inline `#[cfg(test)]`)
- **Test type**: unit
- **Backing**: doc-derived (from pin-part-swap-spec.md)
- **Scenarios**:
  - Normal: two pins in same group, swap reduces HPWL
  - Normal: two identical resistors, part swap reduces HPWL
  - Edge: single-pin swap group → no swaps possible
  - Validation: swap then verify netlist integrity

**Code Intent**:
- `swap.rs` (new ~400-600 lines): `SwapModel` struct built from PcbIr pad swap IDs (Decision: "Swap group data sourced from PcbIr"), `build_swap_model(ir: &PcbIr)`, `greedy_pin_swap_sweep()`, `greedy_part_swap_pass()`, `SwapChangelog` struct, `verify_swap_integrity()`, `write_swap_overlay()` (generates .sch text)
- `lib.rs`: Add `pub mod swap;`, call swap passes at Phase 2.5 and Phase 4.5 if swap flags enabled
- `simulated_annealing.rs`: Add `PinSwap` and `PartSwap` variants to `Move` enum, add to `generate_move()` probability schedule

---

### Milestone 9: CLI Commands + Integration

**Files**:
- `crates/altium-cli/src/main.rs`
- `crates/altium-cli/src/commands/placement.rs` (new or extend existing)

**Requirements**:
- `altium placement autoplace <spec.pcb>` — run autoplacer, rewrite spec
- `altium placement autoplace <spec> --target <board.PcbDoc>` — explicit PcbDoc target
- `altium placement autoplace <spec> --dry-run` — show plan without writing
- `altium placement autoplace <spec> --output <out.pcb>` — write to different file
- `altium placement dump <board.PcbDoc>` — dump current positions as spec
- `altium placement plan <spec>` — show placement ECO (uses existing reconciler)
- `altium placement apply <spec>` — apply spec to PcbDoc (uses existing reconciler)
- Output format: component count, HPWL, moves made, swap summary

**Acceptance Criteria**:
- `altium placement autoplace test.pcb` produces updated spec file
- `altium placement dump data/pcbdoc/test.PcbDoc` produces valid spec output
- `--dry-run` produces plan output without modifying files
- Exit code 0 on success, non-zero on error

**Tests**:
- **Test files**: `crates/altium-cli/src/main.rs` (CLI integration tests with `test-fixtures`)
- **Test type**: integration
- **Backing**: default-derived
- **Scenarios**:
  - Normal: autoplace a spec, verify output file exists and parses
  - Normal: dump a PcbDoc, verify output parseable
  - Edge: dry-run produces output but no file modification

**Code Intent**:
- `main.rs`: Add `placement autoplace`, `placement dump` subcommands to clap argument parser
- `commands/placement.rs`: `cmd_placement_autoplace()` calls `autoplace_spec()` from executor. `cmd_placement_dump()` calls `dump_pcbdoc()`. Format output report.

---

### Milestone 10: Documentation

**Delegated to**: @agent-technical-writer (mode: post-implementation)

**Source**: `## Invisible Knowledge` section of this plan

**Files**:
- `crates/autopcb-placement/README.md`
- `crates/autopcb-spec/README.md` (update)

**Requirements**:
- Document autoplacer architecture (spec → bridge → solver → rewriter flow)
- Document constraint bridge mapping (PlacementSpec → UserConstraint)
- Document SA configuration and tuning
- Document swap optimization flow
- Update STATUS.md with autoplacer completion state

**Acceptance Criteria**:
- README.md in autopcb-placement explains the placement pipeline
- README.md in autopcb-spec documents the placement spec syntax
- STATUS.md reflects current implementation state


## Milestone Dependencies

```
Wave 1 (parallel):
  M1 (Parser) ─────┐
  M2 (Dump)         │──→ Wave 2:
  M3 (Reconciler)   │      M5 (Bridge, needs M1) ──→ M6 (Executor+Rewriter)
  M4 (Viewer)       │
                    │
                    └──→ Wave 3 (after M6):
                           M7 (SA) ──→ M8 (Swaps, needs M7)
                                  └──→ M9 (CLI, needs M6)

  M10 (Docs) — after all others
```

**Wave 1**: M1, M2, M3, M4 — all independent, run in parallel
**Wave 2**: M5 (needs M1) → M6 (needs M5)
**Wave 3**: M7, M8, M9 — after M6; M7 and M9 can run in parallel; M8 needs M7
**Final**: M10 — after all implementation milestones
