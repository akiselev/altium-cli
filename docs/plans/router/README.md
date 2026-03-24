# AutoPCB Router: Full Implementation Plan

## Overview

This plan defines the implementation of a PCB autorouter as a set of workspace
crates (`autopcb-router`, `autopcb-routes`) that integrate with the existing
spec-centric pipeline. The router is a **pure algorithm library** — it receives
`PcbIr` (produced by the spec compiler) and routing config (from the
`routing { ... }` block in a `.pcb` spec), and produces a `RouteSolution` that
is serialized to a `.routes` file. The spec imports this file via
`import "board.routes"`, and the spec compiler applies the routes when
generating output.

**PcbDoc is never touched directly by the router.** Everything flows through
the `.pcb` spec.

## Planning Context

### Decision Log

| Decision | Reasoning Chain |
|----------|----------------|
| Spec-centric architecture | User requires `.pcb` spec as sole entry point → router must not reference PcbDoc → router receives PcbIr + config, returns solution → spec compiler owns all I/O |
| Separate `autopcb-routes` crate | Route solution format needed by both router (writes) and spec compiler (reads/imports) → shared dependency avoids circular deps → thin crate with serde types only |
| Import-first (no spec rewriting) | Spec stays config-only → route file imported via `import "board.routes"` → avoids coupling router to spec rewriter → spec remains human-authored truth |
| Binary + JSON route format | Binary for production (compact, fast load) → JSON for debugging (human-readable, diffable) → both via serde with bincode + serde_json |
| Full solution + iteration snapshots in route file | Viewer needs PathFinder iteration history for playback → metrics needed for CLI reporting → single file bundles everything → avoids scatter of artifacts |
| `routing { ... }` block for misc config | Net priorities, diff pairs, layers declared in their own spec blocks → routing block has autorouter-specific config (grid resolution, max iterations, via cost tuning) → avoids duplication |
| Canonical IR in `autopcb-ir`, derived state in `autopcb-router` | Board geometry, components, pads, nets, rules are shared concerns → duplicating them creates drift → router builds transient optimization structures (grids, obstacle maps, congestion arrays) from PcbIr |
| R-tree (rstar) for spatial index | O(log n) spatial queries for clearance checks → cache-friendly for A* inner loop → rstar is well-maintained in georust ecosystem |
| PathFinder over greedy sequential | Greedy route-one-net-at-a-time produces order-dependent results → PathFinder negotiation allows all nets to compete → history congestion prevents oscillation → proven by McMurchie & Ebeling 1995 |
| MST + FLUTE for net decomposition | 2-pin and 3-pin nets (vast majority) have identical MST and RSMT → MST via petgraph for simplicity → FLUTE trait interface provides near-optimal Steiner tree decomposition for high-fanout multi-pin nets |
| Grid-based routing first, shape-based later | Grid is simpler, proven, cache-friendly → shape-based better for fanout/surface but more complex → grid covers 90% of cases → shape backend behind same trait for tight channels |
| Example-based + proptest for testing | Example-based for core algorithm correctness (known-answer tests) → proptest behind feature gate for invariant coverage (heuristic admissibility, determinism) → synthetic PcbIr only, no fixture files |
| BTreeMap for RouteSolution.nets | Determinism invariant requires identical binary output for identical inputs → HashMap has non-deterministic iteration order (randomized hash seed) → BTreeMap guarantees stable key-sorted iteration → bincode output is byte-identical across runs |
| LayerId newtype in autopcb-routes | CLAUDE.md mandates domain types over raw primitives → autopcb-routes is independent of autopcb-ir → define own `LayerId(u16)` newtype in routes crate → type safety without IR dependency |
| Unsupported rules return hard error | CLAUDE.md fail-fast mandate: "never silently skipped" → unknown rule kind during policy build returns `RoutingError::UnsupportedRule` → caller (spec pipeline) decides whether to proceed or abort → no silent data loss |
| PathFinder pres_fac growth = 1.15 | McMurchie & Ebeling 1995 §3.2 uses 1.15 as baseline → too low (1.05) causes slow convergence, too high (1.5) causes oscillation → 1.15 is empirically validated middle ground |
| good_lp for ILP layer assignment | Pure-Rust with HiGHS backend (no native dep for LP, optional native for MIP) → supports integer constraints needed for layer assignment → `lpsolve` requires native C library build → `highs-sys` is an option but `good_lp` abstracts backend choice |
| RNG seed in RoutingConfig | Determinism requires user-controllable seed → `RoutingConfig.seed: u64` (default 0) is sole source of non-determinism in net ordering → same seed + same PcbIr = identical solution → seed 0 gives reproducible default behavior |
| RNG algorithm: ChaCha8Rng | `rand`'s `SmallRng`/`StdRng` are NOT stable across versions or platforms → `ChaCha8Rng` from `rand_chacha` is platform-independent and version-stable → must not change without semver-major bump on `autopcb-router` → guarantees seed-based determinism invariant |
| NetId newtype in autopcb-routes | autopcb-routes must have no runtime dependency on autopcb-ir (thin format crate) → route file consumers (spec compiler) must not transitively depend on IR extraction logic → own NetId(u32) with same bit width avoids dependency while preserving semantic identity → conversion at route_board() boundary is `routes::NetId(ir_net_id.raw())` |
| `pathfinding` crate for A* | Callback-based A* eliminates custom priority-queue and visited-set bookkeeping → crate is version-stable in Rust ecosystem → `GridNode` directly satisfies `Hash + Eq` for visited set → custom implementation not warranted |
| `geo` crate for polygon operations | Shape-based router (M6 `ShapeRouter`) and keepout containment checks use `geo::algorithm::Intersects` and `Contains` → polygon/line intersection for obstacle queries → `rstar` handles spatial indexing, `geo` handles geometric predicates |
| Coordinate conversion at spec apply boundary | RouteSolution stores mm (f64) matching PcbIr convention → spec compiler's import resolver converts mm → Altium internal units using `Coord::from_mm()` from `altium-format-types` → router never touches Altium coordinate space |
| `LayerId(u16)` width in autopcb-routes | Altium supports at most 32 copper layers (layer ID ≤ 31 in practice) → u16 upper bound (65535) has 2000× headroom → u16 matches compact serialization → conversion from IR's LayerId(u32) is `id.raw() as u16` with `debug_assert!(id.raw() <= u16::MAX as u32)` guard at `route_board()` boundary |
| Per-layer collections use Vec not BTreeMap | Obstacle maps and history arrays indexed by `layer.raw() as usize` into Vec → no `Ord` needed on LayerId → cache-friendly contiguous storage → layer count fixed at workspace build time |
| ViaCostModel.overrides uses BTreeMap | ViaCostModel is part of RoutingConfig (serde Deserialize) → HashMap would produce non-deterministic serialization → BTreeMap provides O(log n) lookup, acceptable for override queries during A* |
| good_lp with minilp backend | `good_lp = { default-features = false, features = ["minilp"] }` → pure-Rust LP solver, no native C dependency → builds in CI without native headers → sufficient for LP layer assignment (integer constraints via rounding) |

### Rejected Alternatives

| Alternative | Why Rejected |
|-------------|-------------|
| Router reads PcbDoc directly | Violates spec-centric architecture. PcbDoc is an implementation detail of the spec pipeline. |
| Router rewrites spec with solved routes | Routes are complex geometry (thousands of segments) — doesn't fit text-based spec language. Import-first is cleaner. |
| Route solution types inside `autopcb-router` | Spec compiler needs to deserialize route files for import. Would create `autopcb-spec` → `autopcb-router` dependency. Thin `autopcb-routes` crate breaks the cycle. |
| Single canonical route + IR crate | Route solution types have different lifecycle (serialized artifacts) than IR types (in-memory extraction). Separate crate keeps concerns clean. |
| Pure GPU routing | CPU-first is required — GPU is an acceleration layer, not a requirement. Most PCB boards (< 1000 nets) route in seconds on CPU. |
| RL/diffusion routing | Massive training infrastructure cost. LLM + spec already encodes design knowledge declaratively. |
| Convention-based route file discovery | Implicit file association is fragile. Explicit `import` is standard, versionable, and already supported by the spec language. |

### Constraints & Assumptions

- Router is a pure function: `route(ir: &PcbIr, config: &RoutingConfig) -> Result<RouteSolution>`
- All routing config comes from the spec language (parsed by spec compiler, passed to router as typed config)
- PcbIr is produced by the spec pipeline — router never calls `PcbIr::extract()` itself
- Route solution serialization uses serde with bincode (binary) and serde_json (JSON)
- Spec import resolver must be extended to handle `.routes` files (deserialize and convert to spec primitives)
- Existing `autopcb-ir` types (PointMm, BoundingBoxMm, handles) are reused — no new coordinate types
- Testing uses synthetic PcbIr construction (no PcbDoc fixture files)
- `default-conventions domain="testing"` applied: prefer integration + property-based over unit tests
- RNG seed is provided via `RoutingConfig.seed: u64` (default 0) — sole source of non-determinism in net ordering
- RouteSolution mm coordinates are converted to Altium internal units by the spec compiler's import resolver using `Coord::from_mm()` from `altium-format-types` — the router never touches Altium coordinate space

### Known Risks

| Risk | Mitigation | Anchor |
|------|-----------|--------|
| PcbIr may lack routing-critical fields | Milestone 2 extends PcbIr with routing metadata before router work begins | crates/autopcb-ir/src/extract.rs |
| Spec import resolver lacks `.routes` support | Import resolver extension point is well-defined — Milestone 9 adds `.routes` handling | crates/autopcb-spec/src/import.rs |
| PathFinder may not converge on complex boards | Max iteration cap + explicit unrouted-net reporting + congestion bottleneck data for diagnostics | N/A (new code) |
| Route file format may need evolution | Version field in route file header + backward-compatible serde defaults | N/A (new code) |

## Invisible Knowledge

### Architecture

```
.pcb spec (source of truth)
  │
  ├── spec compiler ──→ PcbIr (board model in mm)
  │                       │
  │                       ▼
  │                 autopcb-router (pure algorithm)
  │                   RoutingConfig ──→ RouteSolution
  │                                        │
  │                                        ▼
  │                               autopcb-routes (format crate)
  │                                 write ──→ board.routes (binary/JSON)
  │                                        │
  ├── import "board.routes" ◄──────────────┘
  │
  ▼
spec compiler apply ──→ PcbDoc output (tracks, vias from route solution)
```

### Data Flow

```
.pcb spec ──parse──→ PcbDocSpec
                         │
                         ├── PlacementSpec (existing)
                         ├── RoutingSpec (new: autorouter config)
                         └── target PcbDoc path
                                │
                          load + extract
                                │
                                ▼
                             PcbIr (with placement applied)
                                │
                    ┌───────────┴───────────┐
                    ▼                       ▼
            RoutingConfig              Board geometry,
          (from RoutingSpec)           nets, pads, rules,
                    │                  obstacles, layers
                    └───────────┬───────────┘
                                ▼
                    RoutingWorkspace (derived)
                    ├── ObstacleMap (R-tree + bitmaps)
                    ├── RoutingGrid (3D: x, y, layer)
                    ├── GlobalRoutingGraph (coarse grid)
                    └── PathFinderState (history, congestion)
                                │
                         route_board()
                                │
                                ▼
                         RouteSolution
                         ├── routed nets (segments + vias)
                         ├── unrouted nets
                         ├── metrics (HPWL, via count, etc.)
                         └── iteration snapshots (for viewer)
                                │
                           serialize
                                │
                    ┌───────────┴───────────┐
                    ▼                       ▼
            board.routes (bincode)   board.routes.json
```

### Why This Structure

- **`autopcb-routes` is a thin format crate** because both `autopcb-router`
  (producer) and `autopcb-spec` (consumer/importer) need the types.
  Putting them in either crate creates a dependency cycle.
- **Router is stateless** — `RoutingWorkspace` is built fresh from PcbIr + config
  for each invocation. No persistent state between runs. This enables
  deterministic replay and trivial parallelism.
- **Iteration snapshots are inside the solution file** (not separate files)
  because they're part of the routing artifact. The viewer loads a single
  `.routes` file and can play back the routing process.

### Invariants

- Router never modifies PcbIr — it is a read-only input
- RouteSolution must be self-contained — viewer can load it without PcbIr
- Route file format is versioned — older readers skip unknown fields
- All coordinates in RouteSolution are in mm (f64), matching PcbIr convention
- Obstacle maps are deterministic — same PcbIr + config always produces same obstacles
- RouteSolution uses BTreeMap (not HashMap) for deterministic serialization — same input produces byte-identical binary output
- All collections in serialized types use ordered containers (BTreeMap, Vec) to guarantee deterministic output
- Net ordering uses `ChaCha8Rng` (from `rand_chacha`) seeded from `RoutingConfig.seed` — algorithm is fixed; changing it is a breaking change
- History array cells are linearized as `x * (grid_height * layer_count) + y * layer_count + layer.raw() as usize` — grid dimensions are fixed at workspace build time; all read/write sites must use this formula

### Tradeoffs

- **Grid resolution vs. memory**: Finer grids (0.05mm) use quadratically more
  memory but find tighter channels. Default 0.1mm balances accuracy and memory.
- **Full rip-up vs. partial**: Full rip-up is simpler but slower. Hot-set partial
  rip-up (100 worst nets) is implemented as an optimization but full rip-up is
  the default for correctness.
- **Separate route file vs. inline in spec**: Route geometry is too complex for
  spec language. Binary file is compact and fast; JSON variant enables debugging.
  Cost: one extra file to manage.

## Plan Flags

| Flag | Consumer | Meaning |
|------|----------|---------|
| `conformance` | QR | Milestone touches cross-crate API contracts; verify backward compatibility before merge |
| `needs-rationale` | TW/QR | Milestone implements a non-obvious algorithmic tradeoff; implementation must include inline comments citing literature and explaining parameter choices |
| `performance` | QR | Milestone contains inner-loop code; profiling baseline and regression benchmark required |
| `complex-algorithm` | QR/TW | Milestone requires specialist review; add block comments explaining algorithm strategy |

## Milestones

### Milestone 0: `autopcb-routes` Crate — Route Solution Format

**Files**:
- `crates/autopcb-routes/Cargo.toml`
- `crates/autopcb-routes/src/lib.rs`
- `Cargo.toml` (workspace members)

**Requirements**:
- Define `RouteSolution` as the top-level route file type
- Define `RoutedNet` with segments (`Vec<TraceSegment>`) and vias (`Vec<RoutedVia>`)
- Define `TraceSegment` with net_id, layer, start/end (PointMm), width
- Define `RoutedVia` with net_id, position (PointMm), from_layer, to_layer, drill, annular_ring
- Define `RoutingMetrics` with total_length_mm, total_vias, unrouted_count, drc_violation_count
- Define `RoutingIterationSnapshot` with iteration index, routed/unrouted counts, congestion data, per-net paths
- Support binary serialization (bincode) and JSON serialization (serde_json)
- Route file has a version header for forward compatibility
- Coordinate types reuse `autopcb-ir::PointMm` (or define equivalent to avoid IR dependency — see Code Intent)

**Acceptance Criteria**:
- `cargo check -p autopcb-routes` passes
- Round-trip test: serialize RouteSolution to binary, deserialize, assert equality
- Round-trip test: serialize RouteSolution to JSON, deserialize, assert equality
- Version field is present and checked on deserialization

**Tests**:
- **Test files**: `crates/autopcb-routes/src/lib.rs` (inline `#[cfg(test)]`)
- **Test type**: unit (serde round-trip)
- **Backing**: default-derived
- **Scenarios**:
  - Normal: round-trip empty solution, single-net solution, multi-net solution
  - Edge: zero-length segments, zero-area vias, empty iteration snapshots
  - Error: version mismatch on deserialize returns error

**Code Intent**:
- New crate `autopcb-routes` with `serde`, `bincode`, `serde_json` dependencies
- `RouteSolution` struct: version, nets (BTreeMap<NetId, RoutedNet>), unrouted (Vec<NetId>), metrics, iterations (Vec<RoutingIterationSnapshot>). BTreeMap for deterministic serialization order.
- `RoutedNet`: net_id, segments, vias, routed_length_mm
- `TraceSegment`: net_id, layer: LayerId, start (Point), end (Point), width_mm
- `RoutedVia`: net_id, position: Point, from_layer: LayerId, to_layer: LayerId, drill_mm, annular_ring_mm
- `LayerId(u16)` newtype — derives `Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize`. Domain type per CLAUDE.md, defined in this crate (independent of autopcb-ir)
- `NetId(u32)` newtype — derives `Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize`. Domain type, defined in this crate
- `RoutingMetrics`: total_length_mm, total_vias, unrouted_count, completion_pct, drc_violations
- `RoutingIterationSnapshot`: iteration, conflicts, routed_count, unrouted_count, paths snapshot
- Use own `Point` and `NetId` types (simple newtypes) to avoid depending on `autopcb-ir` — the crate must be lightweight
- `save_binary(path)`, `load_binary(path)`, `save_json(path)`, `load_json(path)` functions

---

### Milestone 1: `autopcb-router` Crate Scaffold

**Files**:
- `crates/autopcb-router/Cargo.toml`
- `crates/autopcb-router/src/lib.rs`
- `crates/autopcb-router/src/config.rs`
- `Cargo.toml` (workspace members)

**Requirements**:
- Add `autopcb-router` to workspace with dependencies: `autopcb-ir`, `autopcb-routes`, `petgraph`, `pathfinding`, `rstar`, `geo`, `bitvec`, `rayon`, `good_lp = { default-features = false, features = ["minilp"] }`, `rand`, `rand_chacha`, `serde`, `thiserror`, `tracing`
- Define public top-level API stubs:
  - `build_workspace(ir: &PcbIr, config: &RoutingConfig) -> Result<RoutingWorkspace>`
  - `route_single_net(workspace: &mut RoutingWorkspace, net_id: NetId) -> Result<RoutedNet>`
  - `route_board(workspace: &mut RoutingWorkspace) -> Result<RouteSolution>`
  - `optimize_solution(workspace: &RoutingWorkspace, solution: &mut RouteSolution) -> Result<()>`
- Define `RoutingConfig` with fields: grid_resolution_mm, max_iterations, via_cost_base, pres_fac_multiplier, pres_fac_cap, history_increment, corner_style, allowed_layers, net_configs
- Define `RoutingError` error enum
- Declare module layout matching the plan

**Acceptance Criteria**:
- `cargo check -p autopcb-router` passes
- No duplicate board IR types in `autopcb-router` (only derived runtime types)
- Public API compiles with stub `todo!()` implementations

**Tests**:
- **Test files**: `crates/autopcb-router/src/config.rs` (inline `#[cfg(test)]`)
- **Test type**: unit
- **Backing**: default-derived
- **Scenarios**:
  - Normal: RoutingConfig deserializes from JSON with all fields
  - Edge: RoutingConfig with defaults (minimal JSON)

**Code Intent**:
- New crate with module declarations: `config`, `workspace`, `spatial`, `obstacles`, `global/`, `detailed/`, `pathfinder/`, `optimize/`, `high_speed/`, `drc`, `solution`, `pipeline`
- `RoutingConfig` struct (serde Deserialize) with nested `ViaCostConfig`, `CornerStyle` enum, per-net-class overrides, `seed: u64` (default 0, sole source of non-determinism)
- `RoutingError` with variants: WorkspaceBuildError, RoutingFailed, NoPath, InvalidConfig, UnsupportedRule { kind: String }
- Top-level functions as stubs returning `todo!()`
- Crate-level doc comment stating derived-state-only policy

---

### Milestone 2: `autopcb-ir` Routing Extensions

**Files**:
- `crates/autopcb-ir/src/component.rs`
- `crates/autopcb-ir/src/copper.rs`
- `crates/autopcb-ir/src/net.rs`
- `crates/autopcb-ir/src/rule.rs`
- `crates/autopcb-ir/src/layer_stack.rs`
- `crates/autopcb-ir/src/extract.rs`

**Flags**: `conformance`

**Requirements**:
- Add `locked` and `pre_routed` flags to `IrTrack` and `IrVia`
- Add `from_layer` and `to_layer` fields to `IrVia`
- Add per-layer pad existence to `IrComponentPad` (field: `layer_set: Vec<LayerId>`)
- Add `net_class: Option<String>` to `IrNet`
- Add `diff_pair_partner: Option<NetId>` to `IrNet`
- Add `preferred_direction: Option<PreferredDirection>` to `IrCopperLayer` (enum: Horizontal, Vertical, Any)
- Add `layer: LayerId` field to `IrTrack`, populated via layer-name-to-LayerId lookup from `IrLayerStack::copper_layers` during `extract_tracks()`
- Add typed `IrRuleParams` variants for ALL routing rule kinds: RoutingTopology, RoutingPriority, RoutingLayers, RoutingViaStyle, RoutingCornerStyle, DiffPairsRouting, MatchedLengths (unconditionally — no wildcard fallback permitted; every routing `RuleKind` variant must map to a typed `IrRuleParams` entry)
- Populate new fields during `PcbIr::extract()`

**Acceptance Criteria**:
- `cargo test -p autopcb-ir` passes
- New fields are populated (not always `None`/empty) when source data exists
- Existing tests continue to pass (backward compatible additions)

**Tests**:
- **Test files**: `crates/autopcb-ir/src/extract.rs` (inline tests)
- **Test type**: unit
- **Backing**: default-derived
- **Scenarios**:
  - Synthetic board with locked tracks → `locked: true` on extracted IrTrack
  - Synthetic board with via spanning layers → `from_layer`/`to_layer` populated
  - Net with class membership → `net_class` populated

**Code Intent**:
- Add fields to existing structs (Option types for backward compat)
- `PreferredDirection` enum in `layer_stack.rs`
- Add `layer: LayerId` to `IrTrack`, replacing `layer_name: String`. Build `HashMap<String, LayerId>` lookup table from `IrLayerStack::copper_layers`. For tracks on non-copper layers (display_name not in lookup table), return `IrError::ExtractionError(format!("track on unknown layer {:?}", layer))` — do not silently skip. Update all call sites in `extract_free_copper()`.
- Extend `extract_tracks()`, `extract_vias()`, `extract_nets()`, AND `extract_rules()` to populate new fields
- In `extract_rules()`: add match arms for `RuleKind::RoutingTopology`, `RoutingPriority`, `RoutingLayers`, `RoutingViaStyle`, `RoutingCornerStyle`, `DiffPairsRouting`, `MatchedLengths` — dispatch to the new typed `IrRuleParams` variants BEFORE the wildcard `_ => Other { kind }` arm
- Add typed `IrRuleParams` variants unconditionally: `RoutingTopology { ... }`, `RoutingPriority { priority: i32 }`, `RoutingLayers { allowed: Vec<LayerId> }`, `RoutingViaStyle { width_min, width_max, hole_min, hole_max }`, `RoutingCornerStyle { style: CornerStyle }`, `DiffPairsRouting { gap, max_gap, max_skew }`, `MatchedLengths { tolerance }`
- For `IrVia::from_layer`/`to_layer`: inspect `PcbDocVia` in `altium-format` for start/end layer fields. Cross-reference `docs/dxp/pcb-records.md` for via binary record format. If `PcbDocVia` does not expose these fields, extend `altium-format`'s PcbDoc API first as a prerequisite

---

### Milestone 3: Routing Rules Bridge

**Files**:
- `crates/autopcb-router/src/rules.rs`
- `crates/autopcb-router/src/config.rs`

**Flags**: `needs-rationale`

**Requirements**:
- Convert `PcbIr` design rules + routing config into router-native policy
- Support rule types: Clearance (0), Width (2), RoutingLayers (9), RoutingCornerStyle (10), RoutingViaStyle (11), DiffPairsRouting (51), MatchedLengths (4)
- Policy queryable as:
  - `policy.clearance(net_a, net_b) -> f64`
  - `policy.trace_width(net_id, layer) -> WidthConstraint { min, max, preferred }`
  - `policy.allowed_layers(net_id) -> Vec<LayerId>`
  - `policy.via_candidates(net_id, from_layer, to_layer) -> Vec<ViaTemplate>`
  - `policy.corner_style(net_id) -> CornerStyle`
  - `policy.diff_pair_config(net_id) -> Option<DiffPairConfig>`
- Handle rule precedence (lower priority number = higher priority, first match wins)
- Unsupported rule kinds return `RoutingError::UnsupportedRule { kind }` (fail-fast per CLAUDE.md)

**Acceptance Criteria**:
- Policy resolves correct clearance for net-class-specific rules
- Policy returns correct via candidates per net class
- Ambiguous/conflicting rules produce deterministic resolution (highest priority wins)
- Unsupported rules return `RoutingError::UnsupportedRule` with the rule kind

**Tests**:
- **Test files**: `crates/autopcb-router/src/rules.rs` (inline tests)
- **Test type**: unit
- **Backing**: default-derived
- **Scenarios**:
  - Single clearance rule → all pairs get that clearance
  - Net-class-specific width rule overrides default
  - Two conflicting rules → higher priority wins
  - Unknown rule kind → `RoutingError::UnsupportedRule` returned

**Code Intent**:
- `RoutingPolicy` struct built from `PcbIr::rules` + `RoutingConfig`
- `WidthConstraint`, `ViaTemplate`, `DiffPairConfig`, `CornerStyle` types
- `build_policy(ir: &PcbIr, config: &RoutingConfig) -> RoutingPolicy`
- Rule precedence resolver: sort by priority, iterate until first match
- Per-net-class cache for hot-path queries during A*

---

### Milestone 4: Routing Workspace + Spatial/Obstacles

**Files**:
- `crates/autopcb-router/src/workspace.rs`
- `crates/autopcb-router/src/spatial.rs`
- `crates/autopcb-router/src/obstacles.rs`

**Flags**: `performance`, `complex-algorithm`

**Requirements**:
- Build `RoutingWorkspace` from `PcbIr` + `RoutingConfig`:
  - R-tree over fixed obstacles (pads, keepouts, board edge, pre-routed traces)
  - Per-layer obstacle bitmaps (bitvec) at configured grid resolution
  - Clearance-inflated occupancy queries
  - Pin access points per pad
  - Pre-routed segment reservation
  - Legal via stack transitions
- `SpatialIndex` wrapper over `rstar::RTree<ObstacleEntry>`
- `ObstacleMap` with per-layer `BitVec` grids
- Distinguish: fixed obstacles, pre-routed (must preserve), solution occupancy (mutable)
- Support both coarse and fine grid resolutions

**Acceptance Criteria**:
- `workspace.is_blocked(layer, x, y, net)` returns correct result for pad locations
- `workspace.clearance_query(segment)` identifies nearby obstacles within clearance
- `workspace.pin_accesses(pad_id)` returns valid access points
- Obstacle maps are deterministic (same input → same output)

**Tests**:
- **Test files**: `crates/autopcb-router/src/workspace.rs`, `obstacles.rs` (inline tests)
- **Test type**: unit + property-based (behind `proptest` feature)
- **Backing**: user-specified (both)
- **Scenarios**:
  - Obstacle inflation: pad at (5, 5) with 0.5mm clearance blocks (4.5, 4.5) to (5.5, 5.5)
  - Keepout region blocks all grid cells within it
  - Pre-routed trace reserves cells along its path
  - Board edge clips obstacle map
  - Property: for any random obstacle set, `is_blocked` at obstacle center is always true

**Code Intent**:
- `RoutingWorkspace` struct: ir ref, policy, spatial_index, obstacle_maps: `Vec<ObstacleMap>` indexed by `layer.raw() as usize` (no Ord needed, cache-friendly), grid config, pin_accesses
- `build_workspace()` implementation: iterate PcbIr pads/keepouts/tracks → populate R-tree + bitmaps
- `ObstacleEntry` enum: Pad, Keepout, BoardEdge, PreRoutedTrack, PreRoutedVia
- `GridConfig`: resolution_mm, width_cells, height_cells, origin (PointMm)
- `is_blocked(layer, gx, gy, net_id) -> bool` — net_id for same-net pass-through
- `clearance_query(layer, segment, clearance) -> Vec<&ObstacleEntry>`
- `pin_accesses(pad_id) -> Vec<AccessPoint>` — grid cells adjacent to pad that are routable

---

### Milestone 5: Global Routing

**Files**:
- `crates/autopcb-router/src/global/mod.rs`
- `crates/autopcb-router/src/global/steiner.rs`
- `crates/autopcb-router/src/global/congestion.rs`
- `crates/autopcb-router/src/global/layer_assignment.rs`
- `crates/autopcb-router/src/global/ordering.rs`

**Flags**: `complex-algorithm`, `needs-rationale`

**Requirements**:
- Net decomposition: multi-pin nets → 2-pin subnets
  - MST backend via `petgraph::algo::min_spanning_tree`
  - `NetDecomposer` trait interface allows FLUTE backend substitution for near-optimal Steiner trees
- Coarse congestion grid (cell size = 5-10× trace pitch):
  - Per-cell capacity estimation: `(cell_width - obstacle_width) / (trace_width + clearance)`
  - Per-cell demand tracking
  - Congestion ratio = demand / capacity
- Global routing via negotiation-based A* on coarse grid
- Layer assignment:
  - Heuristic path first (preferred layer direction)
  - `good_lp` ILP backend for constrained cases
- Net ordering heuristic: critical nets first, short nets early, high-fanout last, seeded RNG tiebreaker
- Output: per-subnet region guidance for detailed router

**Acceptance Criteria**:
- MST decomposition produces n-1 edges for n-pin net
- Global router produces stable region plans for synthetic boards
- Congestion grid identifies oversubscribed cells
- Net ordering is deterministic (same seed → same order)

**Tests**:
- **Test files**: `crates/autopcb-router/src/global/steiner.rs`, `congestion.rs`, `ordering.rs` (inline `#[cfg(test)]`)
- **Test type**: unit + property-based (proptest behind feature gate)
- **Backing**: user-specified (both)
- **Scenarios**:
  - 2-pin net: MST produces 1 edge
  - 4-pin square: MST produces 3 edges, total length ≤ complete graph
  - Congestion: 10 nets through 2-cell channel → demand > capacity detected
  - Ordering: critical net always before non-critical with same length
  - Property: MST edge count = n-1 for any n-pin net

**Code Intent**:
- `NetDecomposer` trait: `decompose(pins: &[PointMm]) -> Vec<Subnet>`
- `MstDecomposer` impl using petgraph
- `GlobalRoutingGrid`: cells Vec, rows, cols, cell_size, per-cell capacity/demand
- `global_route(workspace, decomposed_nets) -> GlobalRoutePlan`
- `assign_layers(plan, policy) -> LayerAssignment` — heuristic + optional ILP
- `order_nets(nets, policy) -> Vec<NetId>` — priority sort with seeded RNG
- `Subnet { source: PointMm, target: PointMm, net_id: NetId, region_path: Vec<CellId> }`

---

### Milestone 6: Detailed Routing

**Files**:
- `crates/autopcb-router/src/detailed/mod.rs`
- `crates/autopcb-router/src/detailed/grid.rs`
- `crates/autopcb-router/src/detailed/astar.rs`
- `crates/autopcb-router/src/detailed/via_cost.rs`
- `crates/autopcb-router/src/detailed/shape.rs`
- `crates/autopcb-router/src/detailed/fanout.rs`

**Flags**: `complex-algorithm`, `performance`

**Requirements**:
- 3D A* pathfinding on `(x, y, layer)` node space:
  - 4-way and 8-way movement (configurable)
  - Via transitions with net-class-sensitive costs
  - Layer-direction bias penalty
  - Admissible heuristic: Manhattan + min via transitions
  - Region-guided routing from global stage
- Via cost model: base_cost + si_penalty, per-net-class overrides
- Shape-based routing backend (behind same trait as grid):
  - Surface escape routing
  - BGA/fine-pitch fanout
  - Tight channels where grid is too coarse
- Fanout routing hooks from routing policy

**Acceptance Criteria**:
- `route_single_net` produces valid segments/vias for:
  - Same-layer route (no via)
  - Multi-layer route with via
  - Route around keepout
  - Preserve pre-routed segments
- Shape backend routes simple fanout/escape cases
- Heuristic is admissible (never overestimates)

**Tests**:
- **Test files**: `crates/autopcb-router/src/detailed/astar.rs`, `via_cost.rs` (inline `#[cfg(test)]`)
- **Test type**: unit + property-based (proptest behind feature gate)
- **Backing**: user-specified (both)
- **Scenarios**:
  - Straight path on empty grid: optimal (no unnecessary bends)
  - Path around single obstacle: routes around it
  - Multi-layer with via: via placed, cost includes via penalty
  - Blocked path: returns NoPath error
  - Property: heuristic(node, goal) ≤ actual_cost(node, goal) for random grids

**Code Intent**:
- `DetailedRouter` trait: `route_subnet(workspace, subnet, costs) -> Result<Vec<PathSegment>>`
- `GridRouter` impl using `pathfinding::astar`
- `GridNode { x: u32, y: u32, layer: autopcb_routes::LayerId }` — search space node (domain type per CLAUDE.md, use `.raw()` for indexing)
- `successors(node) -> Vec<(GridNode, f64)>` — 4/8-way + via transitions
- `heuristic(node, goal) -> f64` — Manhattan + layer changes × min_via_cost
- `ViaCostModel { base: f64, si_penalty: f64, overrides: BTreeMap<String, f64> }` (BTreeMap for deterministic serialization of RoutingConfig)
- `ShapeRouter` stub impl for fanout/escape
- `direction_penalty(layer, dx, dy) -> f64` — preferred direction bias

---

### Milestone 7: PathFinder Negotiation

**Files**:
- `crates/autopcb-router/src/pathfinder/mod.rs`
- `crates/autopcb-router/src/pathfinder/history.rs`
- `crates/autopcb-router/src/pathfinder/ripup.rs`
- `crates/autopcb-router/src/pathfinder/hot_set.rs`
- `crates/autopcb-router/src/solution.rs`

**Flags**: `complex-algorithm`, `performance`, `needs-rationale`

**Requirements**:
- Full-board routing loop:
  - Cost function: `C(n) = (b_n + h_n) × p_n` (base + history × present congestion)
  - History congestion: linear accumulation per oversubscribed iteration
  - Present congestion factor: exponential growth (`pres_fac *= 1.15`), capped
  - Full rip-up: rip up all nets each iteration, reroute
  - Hot-set partial rip-up: only rip up nets through oversubscribed cells (optimization)
  - Convergence detection: no oversubscribed routing resources
  - Unrouted/failed-net reporting with bottleneck data
- Iteration snapshots: capture state after each iteration for viewer playback
- Deterministic replay: same config + PcbIr → same solution
- `route_board()` implementation that orchestrates global → detailed → PathFinder

**Acceptance Criteria**:
- `route_board` completes on small synthetic boards with fewer conflicts each iteration
- PathFinder converges or exits with explicit bottleneck data
- Iteration snapshots are populated in RouteSolution
- Same seed produces identical routing

**Tests**:
- **Test files**: `crates/autopcb-router/src/pathfinder/mod.rs`, `history.rs`, `ripup.rs` (inline `#[cfg(test)]`)
- **Test type**: unit + integration (synthetic boards) + property-based (proptest behind feature gate)
- **Backing**: user-specified (both)
- **Scenarios**:
  - 2 non-conflicting nets: converges in 1 iteration
  - 2 crossing nets: converges within max_iterations
  - Impossible route (fully blocked): returns unrouted with bottleneck info
  - History cost increases monotonically for persistently oversubscribed cells
  - Property: conflict count is non-increasing after sufficient iterations

**Code Intent**:
- `PathFinderState { history: Vec<f64>, pres_fac: f64, iteration: u32 }` — history Vec is linearized from 3D grid: `index = x * (grid_height * layer_count) + y * layer_count + layer as usize`. Grid dimensions are fixed at workspace build time. Vec length = `grid_width * grid_height * layer_count`.
- `pathfinder_route(workspace, config) -> Result<RouteSolution>`
- Inner loop: rip_up → order_nets → route_each_net → update_history → check_convergence
- `HotSet`: track worst-offending nets, partial rip-up optimization
- `RouteSolutionBuilder`: accumulates per-net paths, builds `RouteSolution` with snapshots
- Snapshot capture: after each iteration, clone current paths + metrics

---

### Milestone 8: Trace Optimization + High-Speed

**Files**:
- `crates/autopcb-router/src/optimize/mod.rs`
- `crates/autopcb-router/src/optimize/staircase.rs`
- `crates/autopcb-router/src/optimize/corners.rs`
- `crates/autopcb-router/src/optimize/rubber_band.rs`
- `crates/autopcb-router/src/optimize/serpentine.rs`
- `crates/autopcb-router/src/high_speed/mod.rs`
- `crates/autopcb-router/src/high_speed/diff_pair.rs`
- `crates/autopcb-router/src/high_speed/bus.rs`

**Flags**: `complex-algorithm`

**Requirements**:
- Post-route cleanup:
  - Staircase elimination (consecutive H-V bends → diagonal 45°)
  - 45° angle conversion (right-angle → chamfered)
  - Corner-style-aware cleanup (respects RoutingCornerStyle policy)
  - Rubber-banding: pull traces tight using clearance queries from spatial index
- Differential-pair routing:
  - Coupled or semi-coupled routing mode
  - Width/gap enforcement from DiffPairConfig
  - Uncoupled length/skew checking
- Bus routing:
  - Member ordering (minimize crossings within group)
  - Channel routing (parallel group through constrained area)
  - Spacing preservation
- Matched-length correction:
  - Serpentine/accordion segment insertion
  - Per-group target length selection

**Acceptance Criteria**:
- Optimized solution reduces bend count without introducing clearance violations
- Diff-pair routes satisfy gap/skew policy on synthetic tests
- Bus routes maintain member order through constrained channel
- Serpentine insertion achieves target length within tolerance

**Tests**:
- **Test files**: `crates/autopcb-router/src/optimize/staircase.rs`, `corners.rs`, `rubber_band.rs`, `crates/autopcb-router/src/high_speed/diff_pair.rs` (inline `#[cfg(test)]`)
- **Test type**: unit + property-based (proptest for clearance invariant checks)
- **Backing**: user-specified (both)
- **Scenarios**:
  - Staircase: 3-segment stair → 1 diagonal segment
  - Rubber-band: trace with slack → shorter total length, no clearance violations
  - Diff-pair: 2 nets routed with specified gap, skew < max_skew
  - Serpentine: short net padded to target length ± matched-length rule tolerance
  - Property: optimized trace length ≤ original trace length (rubber-band never increases)

**Code Intent**:
- `optimize_solution(workspace, solution)` dispatches optimization passes in order
- `eliminate_staircases(net_paths)` — scan for consecutive bend pairs, replace with diagonal
- `convert_corners(net_paths, style)` — apply 45° chamfer per corner style
- `rubber_band(net_paths, spatial_index)` — iteratively pull vertices toward shorter path
- `DiffPairRouter`: route primary → offset secondary → verify gap → backtrack if collision
- `BusRouter`: order members → route as parallel group → maintain spacing
- `insert_serpentine(path, target_length, amplitude, pitch)` — accordion insertion in uncongested segments

---

### Milestone 9: Spec Integration + CLI

**Files**:
- `crates/autopcb-spec/src/import.rs` (extend for .routes)
- `crates/autopcb-spec/src/compiler.rs` (routing block compilation)
- `crates/autopcb-spec/src/model.rs` (RoutingSpec model types)
- `crates/altium-cli/src/main.rs` (routing CLI commands)

**Flags**: `conformance`

**Requirements**:
- Spec language: `routing { ... }` block parsed and compiled to `RoutingSpec`
- Import resolver: handle `import "board.routes"` — deserialize route file, convert to spec primitives (tracks, vias)
- CLI command: `altium routing solve <spec>` — load spec → build PcbIr → route → write .routes
- CLI command: `altium routing inspect <routes-file>` — print routing stats
- CLI output: completion stats, unrouted nets, via count, total length, DRC summary

**Acceptance Criteria**:
- `routing { grid_resolution: 0.1mm, max_iterations: 50 }` parses successfully
- `import "board.routes"` loads and deserializes a binary route file
- `altium routing solve test.pcb` produces a `.routes` file
- `altium routing inspect board.routes` prints human-readable stats

**Tests**:
- **Test files**: `crates/autopcb-spec/src/compiler.rs` (inline `#[cfg(test)]`), `crates/altium-cli/src/main.rs` (inline `#[cfg(test)]`)
- **Test type**: unit (parser) + integration (CLI with synthetic spec)
- **Backing**: default-derived
- **Scenarios**:
  - Parse routing block with all config fields
  - Import resolver loads binary .routes file
  - Import resolver loads JSON .routes file
  - CLI solve produces .routes file (end-to-end with synthetic PcbIr)

**Code Intent**:
- `RoutingSpec` model type: grid_resolution, max_iterations, via_cost config, corner_style, seed, etc.
- `compile_routing_decl()` in compiler.rs — parallel to `compile_placement_decl()`
- Extend `resolve_imports()` to detect `.routes` extension → call `autopcb_routes::load_binary()` or `load_json()`
- Convert loaded `RouteSolution` → spec primitive records (Track, Via) for apply, using `Coord::from_mm()` from `altium-format-types` for coordinate conversion (mm → Altium internal units)
- `cmd_routing_solve()` in CLI: compile spec → build PcbIr from spec pipeline → call `route_board()` → save .routes
- `cmd_routing_inspect()` in CLI: load .routes → print metrics table

---

### Milestone 10: Viewer + Shell Integration

**Files**:
- `crates/autopcb-viewer/src/app.rs`
- `crates/autopcb-viewer/src/renderer.rs`
- `crates/autopcb-shell/src/jobs.rs`
- `crates/autopcb-shell/src/commands.rs`

**Requirements**:
- Viewer: load `.routes` file and display:
  - Routed trace overlays (colored per layer)
  - Via markers
  - Global congestion heatmap overlay
  - PathFinder iteration playback (reuse playback infrastructure)
  - Before/after optimization toggle
- Shell: `JobKind::Route` with progress streaming
- Shell: routing job produces `JobArtifact::RoutingSolution`

**Acceptance Criteria**:
- Viewer loads `.routes` file and renders traces on correct layers
- PathFinder iteration playback works (step forward/back through iterations)
- Shell can queue and execute a routing job

**Tests**:
- **Test files**: `crates/autopcb-viewer/src/renderer.rs`, `crates/autopcb-shell/src/commands.rs` (inline `#[cfg(test)]`)
- **Test type**: unit
- **Backing**: default-derived
- **Scenarios**:
  - Renderer: route segment maps to correct layer color
  - Renderer: via marker placed at correct coordinates
  - Shell: JobKind::Route registered in command dispatch
  - Shell: route job payload serializes/deserializes correctly

**Code Intent**:
- Add `RenderOptions` fields: `show_routes`, `show_congestion_heatmap`, `show_iteration_playback`
- Load `RouteSolution` from `.routes` file into viewer state
- Render `TraceSegment` as colored lines per layer
- Render `RoutedVia` as circles at via positions
- Reuse `playback_index`/`playback_playing` for iteration snapshots
- Add `JobKind::Route` to shell job system
- Add `JobPayload::RouteSpec { spec_path, config }`
- `JobArtifact::RoutingSolution { path: PathBuf }` carries result path

---

### Milestone 11: Placement-Router Co-Optimization

**Files**:
- `crates/autopcb-router/src/coopt.rs`
- `crates/autopcb-placement/src/lib.rs` (congestion oracle integration)

**Requirements**:
- Forward hook: congestion oracle from global router into placement SA cost
  - Fast estimate: project net bounding boxes onto grid, increment demand (no full routing)
  - Interface: `congestion_oracle(ir: &PcbIr, config: &RoutingConfig) -> CongestionGrid`
- Backward hook: bottleneck extraction from failed PathFinder runs
  - Identify persistently oversubscribed cells → blocking components
  - Generate placement nudge requests
- Stable interface regardless of whether the full placement-router co-optimization outer loop is enabled

**Acceptance Criteria**:
- Placement code can request congestion estimate without invoking full routing
- Router emits bottleneck data tied back to blocking components
- Congestion oracle is deterministic

**Tests**:
- **Test files**: `crates/autopcb-router/src/coopt.rs` (inline `#[cfg(test)]`)
- **Test type**: unit + property-based (proptest for determinism)
- **Backing**: user-specified (both)
- **Scenarios**:
  - Congestion oracle deterministic: same PcbIr + config → identical CongestionGrid
  - Bottleneck extraction: oversubscribed cell maps to correct blocking ComponentId
  - Property: congestion oracle output is deterministic across 100 runs with same input

**Code Intent**:
- `CongestionOracle` in `coopt.rs`: builds coarse global grid, estimates demand from net bounding boxes
- `congestion_oracle(ir, config) -> CongestionGrid` — O(nets × bbox_cells), < 1ms
- `extract_bottlenecks(solution) -> Vec<Bottleneck>` — persistently oversubscribed cells → component IDs
- `Bottleneck { cell: GridCell, components: Vec<ComponentId>, severity: f64 }`
- Integration point in placement SA: add congestion penalty to cost function (optional, behind config flag)

---

### Milestone 12: Documentation

**Delegated to**: @agent-technical-writer (mode: post-implementation)

**Source**: `## Invisible Knowledge` section of this plan

**Files**:
- `crates/autopcb-routes/README.md`
- `crates/autopcb-router/README.md`
- `docs/plans/router/README.md` (this file, updated with status)

**Requirements**:
- README.md files capture architecture, data flow, invariants, tradeoffs
- Self-contained — no external references required to understand

**Acceptance Criteria**:
- README.md exists in each new crate directory
- Architecture diagrams match Invisible Knowledge section
- Spec-centric flow is documented clearly

## Milestone Dependencies

```
M0 (routes format) ──→ M1 (router scaffold) ──→ M3 (rules bridge) ──→ M4 (workspace)
                                                                          │
M2 (IR extensions) ────────────────────────────────────────────────────────┘
                                                                          │
                                                          ┌───────────────┤
                                                          ▼               ▼
                                                    M5 (global)     M6 (detailed)
                                                          │               │
                                                          └───────┬───────┘
                                                                  ▼
                                                          M7 (PathFinder)
                                                                  │
                                                          ┌───────┴───────┐
                                                          ▼               ▼
                                                    M8 (optimize)   M9 (spec+CLI)
                                                          │               │
                                                          └───────┬───────┘
                                                                  ▼
                                                          M10 (viewer+shell)
                                                                  │
                                                                  ▼
                                                          M11 (co-opt)
                                                                  │
                                                                  ▼
                                                          M12 (docs)
```

**Parallel opportunities:**
- M0 and M2 can proceed in parallel (no shared files)
- M5 and M6 can proceed in parallel after M4 (different modules)
- M8 and M9 can proceed in parallel after M7 (different crates)
