# DFM (Design for Manufacturability) Integration Plan

## Overview

Add 10 manufacturability optimization features to the autopcb-router crate,
deeply integrated into every pipeline stage: workspace build, A* cost function,
PathFinder negotiation, post-route optimization, and DRC. The approach is "Deep
Integration" (Approach B) — the router prevents DFM issues during routing rather
than just detecting them post-route. Each feature is a milestone with
configurable weights via a new `DfmConfig` struct in `RoutingConfig`, and every
DFM pass returns a typed report struct for observability and testing.

Research backing is in `docs/research/dfm-*.md` (6 documents, ~5800 lines of
algorithms, pseudocode, and citations).

## Planning Context

### Decision Log

| Decision | Reasoning Chain |
|----------|----------------|
| Deep Integration (Approach B) over Bolt-On | Bolt-on can only detect DFM issues post-route -> router never learns to avoid them -> repeated rip-up/fix cycles -> Deep Integration lets A* and PathFinder prevent issues during routing, which is how commercial routers (Altium, Allegro) work |
| Extend GridNode with direction for bend cost | Router currently has no direction memory -> cannot compute bend angles during A* -> edge-based penalty is approximate and misses compound angle sequences -> extending GridNode with (prev_dx, prev_dy) increases state space ~8x but gives exact interior angle at every successor, which is required for reliable acid trap prevention |
| FabProfile first-class in RoutingConfig | FabProfile as overlay only checks post-route -> constraints not available during routing -> policy.trace_width() and policy.clearance() cannot respect fab limits -> first-class integration means build_policy() floors/caps design rules against fab capabilities, ensuring routing always respects manufacturing limits |
| Configurable DFM weights with defaults | Hard-coded weights prevent tuning for different board types -> 2-layer consumer board needs different acid trap sensitivity than 12-layer HDI -> configurable weights with sensible defaults let power users tune while most users never touch them |
| M2 changes A* state space even when bend cost disabled | GridNode direction in Hash/Eq means same cell reached via different directions = different A* states -> different exploration order -> potentially different (but still valid) paths even with bend_cost_enabled=false -> backward-compatibility invariant weakened to "no additional DFM penalties applied" rather than "bit-identical output" -> this is acceptable because the paths are still optimal under the original cost function, just explored in a different order |
| DFM history is injected into shared history array (not separate) | Separate dfm_history array would require changing A* cost function signature, adding new linearization, decay, and storage -> shared array means inject_dfm_history() calls history.increment() directly -> A* cost function reads one array -> simpler, fewer moving parts -> tradeoff: cannot independently decay DFM vs congestion history, but in practice both should decay together |
| acid_trap_min_angle_deg reuses existing acute_angle_min_deg | DrcPolicy already has acute_angle_min_deg from IrRuleParams::AcuteAngle (default 45deg) -> acid trap detection is the same geometric check applied in manufacturing context -> reuse same field to avoid duplication -> M3's check_acid_traps reads policy.acute_angle_min_deg, not a new field |
| board_thickness_mm sourced from FabProfile | PcbIr IrBoardGeometry has no thickness field -> Altium stores board thickness in Board region properties which map to spec -> for now, add board_thickness_mm to FabProfile (M1) with default 1.6mm (standard FR-4) -> aspect ratio check in M3 reads FabProfile.board_thickness_mm |
| Greedy MIS for redundant via placement | Typical via counts are small (<500 per layer) and conflict graph is sparse (only vias within 2x via_diameter interact) -> sparse nearly-planar graphs allow greedy to achieve near-optimal results -> exact MIS is NP-hard and unnecessary for this domain -> greedy sort-by-degree-ascending produces >90% of optimal for sparse conflict graphs per empirical studies (Halldorsson 1995) |
| Bend cost defaults: weight 0.3, tiers 0.5x/2.0x/10.0x | bend_cost_weight 0.3 is conservative: routes still use moderate bends freely, only strong penalty for near-acid-trap angles -> tier multipliers follow exponential severity: 90-135deg is cosmetic (0.5x * 0.3 = 0.15 penalty), 45-90deg is manufacturing concern (2.0x * 0.3 = 0.6), <45deg is acid trap (10.0x * 0.3 = 3.0, effectively blocking) -> calibrated against FreeRouting's bend_cost baseline (dfm-cost-functions.md §3) |
| Teardrop defaults: via 30%/70%, SMD 100%/200% | Via teardrops: 30% length / 70% width of pad diameter matches Altium defaults (dfm-acid-traps-geometry.md §4) -> SMD teardrops: 100% trace width length / 200% trace width width from KiCad cubic Bezier approach -> these are starting points, tunable per DfmConfig |
| Quality grid defaults: void 0.5, edge 0.85, proximity 3 cells | void 0.5 penalty doubles effective routing cost over plane gaps -> discourages but doesn't prohibit -> edge 0.85 gives gentle bias away from plane boundaries -> 3-cell proximity at 0.25mm grid = 0.75mm transition zone matching typical reference plane clearance (dfm-impedance-si.md §2) |
| DFM penalty defaults: acid trap 5.0, mask dam 3.0, sliver 2.0 | Acid trap is highest priority DFM violation (most common fab rejection) -> 5.0 weight ensures strong PathFinder history pressure -> mask dam 3.0 reflects moderate fab impact -> sliver 2.0 is lowest because slivers are often cosmetic unless very thin -> relative scaling matches ISPD 2018 contest violation weights (dfm-academic-papers.md §1) |
| Density defaults: tile 1.0mm, max 80%, min 20% | 1.0mm tile size balances resolution vs computation -> industry copper balance target is 30-70% (dfm-copper-thermal.md §1) -> 80% max and 20% min provide margin beyond the ideal range -> IPC-6012 limits bow/twist to 0.75% for SMT boards, achieved when copper distribution is within 20-80% per layer |
| Return via distance: 2.0mm default | 0.5-2mm is the recommended range for return path vias near signal layer transitions (dfm-impedance-si.md §3) -> 2.0mm is the conservative end, suitable for signals up to ~1 GHz -> higher-speed designs should tighten to 0.5mm via DfmConfig |
| dfm_history_weight default 1.0 | Congestion history values typically reach 5-20x per iteration while DFM violations occur in <10% of cells -> 1.0 means DFM pressure is at most 1/5 of typical congestion pressure -> conservative starting point that avoids PathFinder oscillation between DFM avoidance and congestion resolution -> configurable for boards where DFM compliance is higher priority than congestion |
| spreading_iterations default 3 | SFF (DAC '07) reports convergence within 3-5 iterations for typical PCB trace densities (dfm-cost-functions.md §4 post-route passes) -> 3 is the lower bound of convergence range -> conservative choice favoring speed over completeness -> each iteration is O(segments * log(segments)) per layer for neighbor queries -> configurable for users who need tighter spreading on dense boards |
| Solder mask dam check uses proxy geometry (expanded pad diameters) | PcbIr lacks explicit mask layer polygons -> M3 approximates mask openings as via_pad_diameter + 2 * solder_mask_expansion_mm from FabProfile -> measures edge-to-edge distance between expanded circles -> this is the same approximation Altium uses for DRC when mask layers are auto-generated -> accurate for circular/octagonal pads, conservative for rectangular |
| min_copper_feature_mm = 0.1mm (4mil) for sliver detection | IPC-6012 Class 2 minimum external conductor width is 0.10mm (4mil) for standard process -> matches JLCPCB/PCBWay minimum feature capability (dfm-fab-constraints.md §1 trace/space table, 1oz copper) -> 4mil is the industry-standard floor for copper features on standard-process boards -> Class 3 uses the same 0.10mm minimum per IPC-6012 Table 3-3 |
| Property-based testing for DFM algorithms | DFM algorithms have geometric invariants (teardrop always wider than trace, bend cost monotonically increases with angle sharpness, density always 0-100%) -> property-based tests cover wide input space -> catches edge cases hand-crafted tests miss |
| Constructed PcbIr fixtures for integration tests | File-based fixtures require test-fixtures feature flag and data/ repos -> slow CI -> constructed IR is deterministic, fast, and self-documenting -> builds small boards programmatically for targeted testing |
| Structured reports from DFM passes | In-place mutation with DRC-only observability makes it hard to test individual passes -> typed report structs (e.g., TeardropReport { inserted, skipped_no_room }) enable direct assertion in tests and structured CLI output |
| Teardrop insertion as post-route pass (not during A*) | Teardrops are geometric additions at junction points -> A* operates on grid cells, not physical geometry -> teardrop shape depends on final trace width and pad geometry known only after routing -> post-route pass after rubber-banding is canonical (Altium, KiCad) |
| DFM history injection into existing PathFinder mechanism | PathFinder already accumulates history costs per cell -> DFM violations map naturally to grid cells -> adding DFM costs to the same history array requires no new data structures -> router learns to avoid manufacturing-problematic regions over iterations |
| Bend cost in g(n) not h(n) | Adding bend penalty to heuristic h(n) would make it inadmissible -> A* loses optimality guarantee -> adding to g(n) (actual cost so far) preserves admissibility while still steering routes away from sharp bends |
| Tiered bend penalty (gentle 90-135deg, strong 45-90deg, near-prohibitive below 45deg) | Binary threshold (e.g., all-or-nothing at 45deg) creates cliff effects in routing -> tiered penalty provides smooth gradient -> router can still use moderate bends when necessary but strongly prefers gentle turns -> matches physical reality where manufacturing difficulty is proportional to angle sharpness |
| Via cost should reflect manufacturing cost (drill wear, aspect ratio) | Current flat via cost of 10.0 treats all vias equally -> manufacturing cost varies dramatically: small drill vias wear bits faster, high aspect ratio vias have lower yield -> manufacturing-aware via cost in FabProfile enables realistic cost modeling |
| Copper density as post-route analysis, not routing-time | Copper density changes with every trace placed -> recomputing per-tile density during A* is O(tiles) per node expansion -> too expensive -> compute once after routing, report violations, optionally inject into PathFinder history for next iteration |
| Reference plane quality bitmap computed at workspace build | Reference plane geometry (pours, fills) is static during routing -> compute once during build_workspace() -> per-cell quality factor multiplied into A* cost -> amortized O(1) per node expansion |
| Return path via stitching as last post-route pass | Must see final solution before knowing which signal vias need return path vias -> depends on reference plane bitmap (Milestone 4) for knowing which layer transitions cross plane boundaries -> runs after all other optimization passes to avoid conflicts |
| SFF trace spreading after rubber-banding | Rubber-band pulls vertices inward (shortening) -> spreading pushes traces apart (widening gaps) -> these are complementary operations -> spreading after rubber-banding fills the space created by path shortening |

### Rejected Alternatives

| Alternative | Why Rejected |
|-------------|-------------|
| Bolt-On architecture (Approach A) | Post-route-only DFM cannot influence routing decisions -> acid traps detected but not prevented -> redundant spatial queries since DFM modules re-query what workspace already computed -> FabProfile disconnected from policy requires manual synchronization |
| Layered Hybrid (Approach C) | Marginal risk reduction vs Deep Integration, but loses cohesion -> post-route passes would still need workspace data for clearance queries -> artificial boundary between "core" and "bolt-on" features complicates future integration (e.g., trace spreading needs reference plane awareness) |
| Edge-based bend penalty (no GridNode extension) | Cannot detect compound angles (two consecutive 90-degree bends creating a 45-degree acid trap) -> only sees individual edges, not path curvature -> misses the most dangerous acid trap configurations |
| Post-route-only bend detection (no A* modification) | Routes placed with acid traps must be ripped up and rerouted -> PathFinder already does rip-up but has no way to avoid the same trap on reroute without A* guidance -> post-route detection creates an infinite correction loop |
| FabProfile as optional DRC overlay | Constraints not available during routing -> router may place traces/vias that violate fab limits -> DRC reports violations after the fact -> user must manually adjust and re-route -> first-class integration prevents violations from occurring |
| ML-based cost function tuning | Google AlphaChip and DeepPCB are interesting but require training data we don't have -> Bayesian optimization of PathFinder parameters is more practical but adds ML dependency -> configurable weights with manual tuning is sufficient for current scale and simpler to debug |
| Dynamic copper pour during routing | FreeRouting tried this and found it architecturally incompatible -> pour shape changes with every trace -> O(n^2) recomputation -> route-first, pour-second is universal industry practice |

### Constraints & Assumptions

- Router determinism invariant must be preserved: same seed + PcbIr + RoutingConfig = identical RouteSolution (BTreeMap everywhere, seeded RNG)
- GridNode state space increase (~8x for direction tracking) must not cause OOM on boards up to 100x100mm at 0.1mm resolution (~8M states per layer)
- All new config fields must have `#[serde(default)]` for backward compatibility with existing routing specs
- DFM passes must be individually disable-able via DfmConfig booleans (zero behavioral change when all false)
- No new crate dependencies unless strictly necessary; prefer extending existing abstractions
- Test files that read from data/ must be gated with `#[cfg(feature = "test-fixtures")]`
- Property tests must be gated with `#[cfg(feature = "proptest")]`

### Known Risks

| Risk | Mitigation | Anchor |
|------|-----------|--------|
| GridNode direction state increases A* memory ~8x | Direction is 2 bytes (dx, dy as i8); GridNode goes from 8 bytes to 10 bytes -> ~25% memory increase, not 8x, because pathfinding crate stores nodes in a HashMap, not a dense array | `crates/autopcb-router/src/detailed/grid.rs:48-56` |
| Bend cost penalty degrades routability on dense boards | Tiered penalty with configurable weight (default 0.3) allows router to still use sharp bends when no alternative exists -> PathFinder negotiation naturally relaxes constraints under congestion | `crates/autopcb-router/src/pathfinder/mod.rs:1-23` (cost function doc) |
| FabProfile constraints conflict with design rules | FabProfile floors/caps design rules (takes the more conservative value) -> never relaxes constraints -> if FabProfile says 5mil min trace but design rules say 4mil, FabProfile wins -> log warning when override occurs | `crates/autopcb-router/src/rules.rs` (build_policy) |
| DFM history injection causes PathFinder oscillation | DFM penalties use same decay as congestion history (config.history_decay) -> prevents fossilization -> weight capped at dfm_history_weight (default 1.0) which is low relative to congestion penalties | `crates/autopcb-router/src/pathfinder/history.rs` |
| Teardrop insertion creates DRC violations (clearance to adjacent traces) | Teardrop geometry respects clearance envelope from workspace.spatial_index -> skip insertion when insufficient room -> report skipped count in TeardropReport | N/A (new code) |
| M3 solder mask dam check uses proxy geometry (no real mask polygons in IR) | PcbIr lacks explicit mask layer polygons -> M3 approximates mask openings as expanded pad circles using FabProfile.solder_mask_expansion_mm -> accurate for round/octagonal pads, conservative for rectangular -> same approximation Altium uses for auto-generated mask DRC -> full mask geometry support deferred to future PcbIr extension | `crates/autopcb-router/src/drc/manufacturing.rs:1-5` (existing placeholder comment) |

## Invisible Knowledge

### Architecture

```
PcbIr + RoutingConfig (with FabProfile + DfmConfig)
          |
          v
  build_workspace()
    ├── build_policy() ← FabProfile floors/caps design rules [M1]
    ├── build_obstacle_maps()
    ├── build_spatial_index()
    ├── compute_access_points()
    ├── compute_escape_plan()
    ├── compute_reference_plane_quality() [M4]
    └── compute_copper_density_baseline() [M9]
          |
          v
  pathfinder_route()
    ├── global_route() (MST decomposition, congestion, layer assignment, ordering)
    ├── per-iteration loop:
    │     ├── rip_up + reroute via GridRouter.route_subnet()
    │     │     └── A* successor expansion:
    │     │           cost += bend_penalty(prev_dir, new_dir) [M2]
    │     │           cost *= quality_grid[layer][cell]       [M4]
    │     │           cost += hist_weight * history[n]  [M5: DFM costs injected into same array]
    │     ├── check_routing() (fast DRC: shorts + clearance)
    │     ├── check_manufacturing() → manufacturing violations [M3]
    │     ├── inject_dfm_history(violations, history_array) [M5]
    │     └── update_history(), update_present_usage()
    └── return best_solution
          |
          v
  optimize_solution()
    ├── staircase::eliminate_staircases()       (existing)
    ├── corners::convert_corners()              (existing)
    ├── rubber_band::rubber_band()              (existing)
    ├── teardrops::insert_teardrops() → TeardropReport      [M6]
    ├── spreading::spread_and_fatten() → SpreadingReport    [M7]
    ├── redundant_vias::insert_redundant() → RedundantViaReport [M8]
    └── return_path::stitch_return_vias() → ReturnPathReport   [M10]
          |
          v
  check_full() (comprehensive DRC)
    ├── existing checks (clearance, shorts, width, via, geometry, ...)
    ├── manufacturing checks (acid traps, mask dam, slivers, ...) [M3]
    └── copper_density_check() → density violations [M9]
```

### Data Flow

```
FabProfile (JSON config)
    → build_policy(): floors/caps clearance, width, via constraints
    → RoutingPolicy: consumed by A*, PathFinder, DRC, post-route passes

DfmConfig (JSON config)
    → feature flags (bend_cost_enabled, teardrops_enabled, ...)
    → weight parameters (bend_cost_weight, dfm_history_weight, ...)
    → consumed by: GridRouter, PathFinder loop, optimize_solution()

Reference Plane Quality Grid (computed at workspace build)
    → per-layer per-cell f64 factor (0.5 = void, 1.0 = solid)
    → multiplied into A* edge cost in GridRouter.route_subnet()

DFM History (injected into PathFinder history array)
    → manufacturing DRC violations mapped to grid cells
    → accumulated with same mechanism as congestion history
    → decayed by config.history_decay each iteration

DFM Reports (returned from each post-route pass)
    → TeardropReport, SpreadingReport, RedundantViaReport, ReturnPathReport
    → aggregated into RouteSolution.metrics for CLI display
    → asserted on in integration tests
```

### Why This Structure

The DFM features span the entire routing pipeline (workspace → A* → PathFinder →
post-route → DRC) because manufacturing quality is an emergent property of all
pipeline stages working together. Separating DFM into its own module would
require duplicating spatial queries, policy lookups, and grid traversal logic
that already exists in the core pipeline.

Post-route passes (teardrops, spreading, redundant vias, return path) are
independent modules in `optimize/` because they operate on the finished
`RouteSolution` geometry, not the grid representation. They can be developed and
tested independently.

### Invariants

- DfmConfig with all features disabled MUST NOT apply any DFM penalties to
  routing cost (no bend penalty, no quality grid multiplier, no DFM history
  injection). Note: M2's GridNode direction tracking changes A* state space
  exploration order, so paths may differ from pre-DFM code even when disabled,
  but they remain optimal under the original cost function.
- FabProfile constraints MUST only tighten, never relax, design rules (monotonic
  constraint strengthening)
- Bend cost penalty MUST be added to g(n), never h(n), to preserve A* admissibility
- All DFM-related BTreeMap/BTreeSet usage for determinism (no HashMap in DFM code paths)
- DFM reports MUST be deterministic: same input → same report fields
- Teardrop insertion MUST NOT create DRC violations (skip when insufficient clearance)
- Redundant via insertion MUST NOT create DRC violations (conflict graph prevents this)
- Reference plane quality grid cells outside board bounds MUST be 0.0 (blocked)

### Tradeoffs

- GridNode direction tracking: ~25% memory increase for A* visited set → exact
  bend angle computation (worth it: acid traps are a top-3 DFM violation)
- DFM history injection: slight PathFinder convergence slowdown → manufacturing-
  aware routing (worth it: prevents repeated DFM violations across iterations)
- FabProfile in RoutingConfig: slightly larger config surface → constraints flow
  through entire pipeline (worth it: single source of truth for fab limits)
- Post-route spreading: additional optimization time → wider trace spacing for
  better etching (worth it: configurable, skipped when disabled)


## Milestones

### Milestone 1: FabProfile Data Model & DfmConfig

**Files**:
- `crates/autopcb-router/src/fab_profile.rs` (new)
- `crates/autopcb-router/src/config.rs`
- `crates/autopcb-router/src/rules.rs`
- `crates/autopcb-router/src/lib.rs`

**Flags**: `conformance`, `needs-rationale`

**Requirements**:
- Add `FabProfile` struct with fields: min_trace_width_mm, min_trace_spacing_mm,
  min_via_drill_mm, max_via_drill_mm, min_annular_ring_mm, solder_mask_expansion_mm,
  solder_mask_dam_min_mm, aspect_ratio_max, drill_to_copper_clearance_mm,
  board_thickness_mm (default 1.6mm, standard FR-4),
  copper_weight_oz, ipc_class (enum: Class1/Class2/Class3), and capability flags
  (supports_via_in_pad, supports_blind_vias, supports_filled_vias)
- Add `DfmConfig` struct with boolean enable flags for all 10 features plus
  weight parameters (bend_cost_weight, dfm_history_weight, quality_grid_weight,
  spreading_iterations, teardrop defaults)
- Add `fab_profile: Option<FabProfile>` and `dfm: DfmConfig` fields to RoutingConfig
  with `#[serde(default)]`
- In `build_policy()`, when FabProfile is present, floor/cap the resolved
  clearance, width, and via constraints against fab limits (take the more
  conservative value). Log a tracing::warn when FabProfile overrides a design rule.
- Add `IpcClass` enum with methods for default constraint floors per class
- All fields serde-deserializable; empty JSON `{}` must still produce valid defaults

**Acceptance Criteria**:
- `RoutingConfig::default()` produces valid config with `fab_profile: None` and
  `dfm: DfmConfig::default()` where all features are disabled
- `serde_json::from_str("{}")` deserializes to default RoutingConfig (backward compat)
- `build_policy()` with FabProfile floors constraints: if fab says 0.15mm min trace
  but design rule says 0.1mm, policy returns 0.15mm
- `build_policy()` without FabProfile produces identical policy to pre-DFM code

**Tests**:
- **Test files**: `crates/autopcb-router/src/fab_profile.rs` (inline), `crates/autopcb-router/src/config.rs` (inline)
- **Test type**: property-based + example-based
- **Backing**: user-specified
- **Scenarios**:
  - Normal: FabProfile floors clearance from 0.1mm to 0.15mm
  - Normal: IpcClass::Class3 defaults produce conservative constraints
  - Edge: FabProfile all-zero fields (should use IPC defaults)
  - Edge: design rule already stricter than FabProfile (no override)
  - Proptest: arbitrary FabProfile + arbitrary design rules → policy constraints >= max(fab, rule)

**Code Intent**:
- New file `fab_profile.rs`: `FabProfile` struct, `IpcClass` enum, `DfmConfig` struct,
  all with Serialize/Deserialize/Default derives
- `IpcClass` has method `default_constraints() -> FabConstraintFloors` returning
  per-class minimum values from IPC-6012 tables
- Modify `config.rs`: add `fab_profile: Option<FabProfile>` and `dfm: DfmConfig`
  to `RoutingConfig` struct and Default impl
- Modify `rules.rs` `build_policy()`: after resolving design rules, if
  `config.fab_profile` is Some, apply floor/cap logic via
  `FabProfile::apply_to_policy(&mut policy)`
- Modify `lib.rs`: add `pub mod fab_profile;` and re-export `FabProfile`, `DfmConfig`, `IpcClass`
- Update existing config deserialization tests to verify backward compatibility

---

### Milestone 2: Bend Cost in A*

**Files**:
- `crates/autopcb-router/src/detailed/grid.rs`
- `crates/autopcb-router/src/detailed/astar.rs`

**Flags**: `complex-algorithm`, `performance`, `needs-rationale`

**Requirements**:
- Extend `GridNode` with `prev_dx: i8, prev_dy: i8` fields (0,0 for start node)
  to track the direction of the edge that reached this node
- In `GridRouter::route_subnet()` A* successor expansion, compute bend angle
  between previous direction and proposed direction. Apply tiered multiplicative
  penalty to edge cost:
  - Interior angle >= 135deg (gentle bend): penalty = 1.0 (no penalty)
  - Interior angle 90-135deg (moderate bend): penalty = 1.0 + 0.5 * bend_cost_weight
  - Interior angle 45-90deg (sharp bend): penalty = 1.0 + 2.0 * bend_cost_weight
  - Interior angle < 45deg (acid trap risk): penalty = 1.0 + 10.0 * bend_cost_weight
- bend_cost_weight comes from `DfmConfig.bend_cost_weight` (default 0.3)
- When `DfmConfig.bend_cost_enabled` is false, skip bend cost entirely (1.0 multiplier)
- Penalty applies to g(n) (edge cost), never to h(n) (heuristic)
- Start node (prev_dx=0, prev_dy=0) always gets penalty 1.0 (no previous direction)

**Acceptance Criteria**:
- A* with bend_cost_enabled=false applies no bend penalties (all multipliers = 1.0).
  Note: paths may differ from pre-DFM code due to direction-extended state space,
  but no DFM-specific cost is added.
- A* with bend_cost_enabled=true avoids 45-degree interior angles when alternative
  paths exist (verified by routing a simple L-shaped net)
- GridNode Hash/Eq includes direction fields (same cell, different approach
  direction = different node)
- No regression in routability: all previously routable nets still route

**Tests**:
- **Test files**: `crates/autopcb-router/src/detailed/grid.rs` (inline)
- **Test type**: property-based + example-based
- **Backing**: user-specified
- **Scenarios**:
  - Normal: straight-line route has zero bend penalty
  - Normal: L-shaped route with 90deg bend gets moderate penalty
  - Edge: U-turn (180deg direction change) gets maximum penalty
  - Edge: start node with no previous direction gets no penalty
  - Proptest: for any two directions, bend_penalty(d1, d2) >= 1.0 AND
    bend_penalty(d1, d2) is monotonically non-decreasing with angle sharpness

**Code Intent**:
- Modify `GridNode` struct: add `prev_dx: i8, prev_dy: i8` fields, update Hash/Eq
  derives to include them
- Add `fn bend_penalty(prev_dx: i8, prev_dy: i8, new_dx: i8, new_dy: i8, weight: f64) -> f64`
  to `astar.rs` that computes interior angle via dot product and returns tiered penalty
- Modify `GridRouter::route_subnet()` successor expansion: pass DfmConfig to
  GridRouter (stored as field), compute bend penalty for each successor, multiply
  into edge cost before adding to g(n)
- GridRouter constructor takes `&DfmConfig` reference (or clone relevant fields)
- Decision: "Bend cost in g(n) not h(n)" — preserves A* admissibility
- Decision: "Tiered penalty" — smooth gradient, no cliff effects

---

### Milestone 3: Manufacturing DRC Implementation

**Files**:
- `crates/autopcb-router/src/drc/manufacturing.rs`
- `crates/autopcb-router/src/drc/mod.rs`
- `crates/autopcb-router/src/drc/policy.rs`

**Flags**: `error-handling`, `needs-rationale`

**Dependencies**: Milestone 1 (FabProfile provides constraint values)

**Requirements**:
- Replace `check_manufacturing()` placeholder with real checks:
  1. **Acid trap detection**: scan consecutive segment pairs per net, compute
     interior angle, flag if < policy.acute_angle_min_deg (reuses existing field,
     default 45deg — see Decision Log "acid_trap_min_angle_deg reuses existing")
  2. **Solder mask dam**: for each via pair on the same layer, compute mask opening
     as `via_pad_diameter + 2 * policy.solder_mask_expansion_mm` (proxy geometry —
     PcbIr lacks explicit mask polygons, see Decision Log), check if edge-to-edge
     distance between expanded circles < policy.solder_mask_dam_min_mm
  3. **Annular ring enforcement**: for each via, verify
     (pad_diameter - drill_diameter) / 2 >= policy.annular_ring_min_mm
  4. **Copper sliver detection**: scan segment junctions for thin copper features
     (width < policy.min_copper_feature_mm, default 0.1mm / 4mil)
  5. **Aspect ratio check**: for each via, verify board_thickness / drill_diameter
     <= policy.aspect_ratio_max
- Add `ManufacturingReport` struct: acid_trap_count, mask_dam_violations,
  annular_ring_violations, sliver_count, aspect_ratio_violations
- Add new DrcViolationKind variants: AcidTrap, SolderMaskDam, CopperSliver,
  AspectRatio (AnnularRing already exists in via.rs)
- DrcPolicy extended with manufacturing thresholds from FabProfile

**Acceptance Criteria**:
- Route with known acid trap (two segments at 30deg) → AcidTrap violation reported
- Route with two vias closer than mask dam minimum → SolderMaskDam violation
- Via with undersized annular ring → AnnularRing violation
- ManufacturingReport fields match violation counts
- Empty solution → empty report (no false positives)

**Tests**:
- **Test files**: `crates/autopcb-router/src/drc/manufacturing.rs` (inline)
- **Test type**: example-based + property-based
- **Backing**: user-specified
- **Scenarios**:
  - Normal: two segments at 30deg → acid trap detected
  - Normal: two segments at 90deg → no acid trap
  - Normal: two vias 0.05mm apart with 0.1mm mask expansion → mask dam violation
  - Edge: parallel segments (180deg) → no acid trap
  - Edge: single-segment net → no acid trap (nothing to compare)
  - Proptest: for any two segment angles, acid_trap_detected iff interior_angle < threshold

**Code Intent**:
- Rewrite `check_manufacturing()` body: call sub-functions for each check category
- Add `check_acid_traps(solution, policy) -> Vec<DrcViolation>` that iterates
  per-net segments, computes pairwise interior angles at shared endpoints,
  using policy.acute_angle_min_deg as threshold (reuses existing field)
- Add `check_solder_mask_dams(solution, policy) -> Vec<DrcViolation>` that uses
  O(n^2) via pair scan per layer with mask expansion
- Add `check_annular_rings(solution, policy) -> Vec<DrcViolation>` from via geometry
- Add `check_copper_slivers(solution, policy) -> Vec<DrcViolation>` using vertex
  angle analysis at segment junctions (law-of-cosines approach from KiCad research)
- Add `check_aspect_ratios(solution, ir, policy) -> Vec<DrcViolation>` using
  FabProfile.board_thickness_mm (default 1.6mm, see Decision Log
  "board_thickness_mm sourced from FabProfile")
- Add ManufacturingReport struct that counts violations by category
- Extend DrcViolationKind enum in `drc/mod.rs`
- Extend DrcPolicy in `drc/policy.rs` with manufacturing thresholds sourced from
  FabProfile (or conservative defaults)

---

### Milestone 4: Reference Plane Quality Bitmap

**Files**:
- `crates/autopcb-router/src/workspace.rs`
- `crates/autopcb-router/src/detailed/grid.rs`
- `crates/autopcb-router/src/quality.rs` (new)

**Flags**: `complex-algorithm`, `performance`

**Requirements**:
- New module `quality.rs` with `QualityGrid` struct: per-layer `Vec<f64>` using
  same linearization as HistoryArray (`x * (h * L) + y * L + layer`)
- During `build_workspace()`, compute quality grid from copper pour/fill geometry
  in `PcbIr.regions` and `PcbIr.polygons`:
  - Cell has solid reference plane on adjacent layer → quality = 1.0
  - Cell is near edge of reference plane (within 3 cells) → quality = 0.85
  - Cell has no reference plane on adjacent layer → quality = 0.5
  - Cell outside board bounds → quality = 0.0
- Add `quality_grid: Option<QualityGrid>` to `RoutingWorkspace`
- In `GridRouter::route_subnet()` A* successor expansion, multiply edge cost by
  `quality_grid[cell]` when DfmConfig.reference_plane_quality is true
- Quality grid weight scaled by `DfmConfig.quality_grid_weight` (default 1.0):
  `effective_quality = 1.0 - quality_grid_weight * (1.0 - raw_quality)`
- When reference_plane_quality is false, skip quality grid (effective multiplier = 1.0)

**Acceptance Criteria**:
- Workspace with no polygons/regions → quality grid is None (no overhead)
- Workspace with ground plane → cells over plane have quality 1.0
- A* with quality enabled prefers routing over solid reference planes
- A* with quality disabled applies no quality grid multiplier (effective
  multiplier = 1.0 for all cells). Note: paths may differ from pre-DFM code
  due to M2's direction-extended GridNode state space, but no quality grid
  cost is applied.

**Tests**:
- **Test files**: `crates/autopcb-router/src/quality.rs` (inline)
- **Test type**: property-based + example-based
- **Backing**: user-specified
- **Scenarios**:
  - Normal: full ground plane → all cells quality 1.0
  - Normal: plane with slot → cells over slot quality 0.5
  - Edge: empty board (no pours) → quality grid is None
  - Edge: cell at plane edge → quality 0.85
  - Proptest: quality values always in [0.0, 1.0]

**Code Intent**:
- New file `quality.rs`: `QualityGrid` struct with build(), get(x, y, layer) methods
- `QualityGrid::build(ir, grid_config)` iterates `ir.regions` and `ir.polygons`,
  rasterizes copper pour outlines onto the grid, computes per-cell quality
- Modify `workspace.rs` `build_workspace()`: after existing steps, if
  `config.dfm.reference_plane_quality` is true, compute and store QualityGrid
- Modify `grid.rs` GridRouter: accept optional `&QualityGrid`, multiply into
  edge cost during successor expansion
- Modify `lib.rs`: add `pub mod quality;`

---

### Milestone 5: DFM Terms in PathFinder History

**Files**:
- `crates/autopcb-router/src/pathfinder/mod.rs`
- `crates/autopcb-router/src/pathfinder/history.rs`

**Flags**: `complex-algorithm`, `needs-rationale`

**Dependencies**: Milestone 3 (Manufacturing DRC provides violation sources)

**Requirements**:
- After each PathFinder iteration's `check_manufacturing()` call, map violations
  to grid cells and inject as additive costs into the history array
- New function `inject_dfm_history(violations: &[DrcViolation], history: &mut HistoryArray, grid: &GridConfig, weight: f64)`
  that converts violation locations to grid coordinates and increments history
- DFM history weight = `config.dfm.dfm_history_weight` (default 1.0)
- DFM history decays with the same `config.history_decay` factor as congestion history
- When `DfmConfig.dfm_history_injection` is false, skip injection entirely
- Each violation type has configurable penalty multiplier in DfmConfig
  (acid_trap_penalty: 5.0, mask_dam_penalty: 3.0, sliver_penalty: 2.0)

**Acceptance Criteria**:
- PathFinder with dfm_history_injection=false injects no DFM costs into the
  history array (history values unchanged by DFM injection). Note: paths may
  differ from pre-DFM code due to M2's direction-extended GridNode state space,
  but no DFM history injection occurs.
- PathFinder with dfm_history_injection=true reduces acid trap count over iterations
  (verified on a board where initial routing creates acid traps)
- History array values only increase from DFM injection (never negative)
- DFM penalties accumulate with congestion penalties (additive, not replacing)

**Tests**:
- **Test files**: `crates/autopcb-router/src/pathfinder/mod.rs` (inline)
- **Test type**: example-based
- **Backing**: user-specified
- **Scenarios**:
  - Normal: inject 3 acid trap violations → history cells at those locations increase
  - Normal: two iterations with same violation → history accumulates
  - Edge: empty violation list → no history change
  - Edge: violation outside grid bounds → safely ignored (no panic)

**Code Intent**:
- Add `inject_dfm_history()` function to `pathfinder/mod.rs` (or `history.rs`)
- In the PathFinder iteration loop (after `check_routing()` and
  `check_manufacturing()`), call `inject_dfm_history()` with the manufacturing
  violations when `config.dfm.dfm_history_injection` is enabled
- Decision: "DFM history injection into existing PathFinder mechanism" — reuses
  existing history array, no new data structures
- DFM penalties are additive to existing history values, not a separate array,
  because the A* cost function already reads `history[n]` once

---

### Milestone 6: Teardrop Insertion

**Files**:
- `crates/autopcb-router/src/optimize/teardrops.rs` (new)
- `crates/autopcb-router/src/optimize/mod.rs`

**Flags**: `complex-algorithm`, `needs-rationale`

**Requirements**:
- New post-route optimization pass that inserts teardrop shapes at:
  1. Pad-to-trace junctions (SMD and through-hole)
  2. Via-to-trace junctions
- Teardrop geometry: straight-edge (two angled segments + arc) or curved (cubic
  Bezier) controlled by DfmConfig.teardrop_style (default: straight)
- Teardrop parameters from DfmConfig:
  - teardrop_via_length_pct: 0.3 (30% of pad diameter)
  - teardrop_via_width_pct: 0.7 (70% of pad diameter)
  - teardrop_smd_length_pct: 1.0 (100% of trace width)
  - teardrop_smd_width_pct: 2.0 (200% of trace width)
- Before inserting, check clearance envelope against workspace spatial index.
  Skip insertion if teardrop would violate clearance. Record skip in report.
- Return `TeardropReport { inserted_via: usize, inserted_pad: usize, skipped_no_room: usize }`
- Insert teardrops after rubber-banding (pass 4 in optimize pipeline)

**Acceptance Criteria**:
- Route with via → teardrop segments added at via-trace junction
- Route with SMD pad → teardrop segments added at pad-trace junction
- Teardrop width > trace width at junction point
- No DRC violations introduced by teardrop insertion
- TeardropReport counts match actual insertions
- teardrops_enabled=false → no teardrops, solution unchanged

**Tests**:
- **Test files**: `crates/autopcb-router/src/optimize/teardrops.rs` (inline)
- **Test type**: property-based + example-based
- **Backing**: user-specified
- **Scenarios**:
  - Normal: single trace entering via center → two teardrop segments inserted
  - Normal: trace entering SMD pad → teardrop at pad junction
  - Edge: trace already wider than pad → skip teardrop (no benefit)
  - Edge: two traces entering same via from opposite sides → two teardrops
  - Edge: insufficient clearance → skip, increment skipped_no_room
  - Proptest: teardrop max width >= trace width AND teardrop length > 0

**Code Intent**:
- New file `optimize/teardrops.rs`: `insert_teardrops()` function and `TeardropReport`
- `insert_teardrops(solution, workspace) -> Result<TeardropReport, RoutingError>`
  iterates all routed nets, finds segment endpoints that coincide with via or pad
  locations, computes teardrop geometry, inserts replacement segments
- Teardrop geometry computed by `compute_teardrop_shape(trace_width, pad_diameter,
  approach_angle, length_pct, width_pct) -> Vec<TraceSegment>`
- Clearance check: `workspace.spatial_index.query_envelope(teardrop_bbox)` to verify
  no obstacles within teardrop footprint
- Modify `optimize/mod.rs`: add `pub mod teardrops;` and call `insert_teardrops()`
  in `optimize_solution()` after rubber_band when config.dfm.teardrops_enabled

---

### Milestone 7: Post-Route Trace Spreading (SFF)

**Files**:
- `crates/autopcb-router/src/optimize/spreading.rs` (new)
- `crates/autopcb-router/src/optimize/mod.rs`

**Flags**: `complex-algorithm`, `performance`

**Requirements**:
- New post-route pass implementing Spread-Fatten-Fill (SFF) from DAC '07:
  1. **Spread**: push trace segments apart to maximize spacing between adjacent
     traces on the same layer. Uses iterative relaxation (N iterations, default 3)
     checking clearance via spatial index.
  2. **Fatten**: widen traces to fill available space up to policy maximum width.
     Width increase limited by clearance to nearest neighbor.
- Run after teardrop insertion (pass 5 in optimize pipeline)
- Return `SpreadingReport { segments_spread: usize, segments_fattened: usize,
  avg_spacing_increase_mm: f64 }`
- Spreading iterations configurable via DfmConfig.spreading_iterations (default 3)
- When spreading_enabled is false, skip entirely

**Acceptance Criteria**:
- Two parallel traces with excess spacing → traces spread apart further
- Trace with excess clearance to neighbors → trace widened
- No DRC violations introduced by spreading/fattening
- SpreadingReport fields are accurate
- spreading_enabled=false → solution unchanged

**Tests**:
- **Test files**: `crates/autopcb-router/src/optimize/spreading.rs` (inline)
- **Test type**: example-based
- **Backing**: user-specified
- **Scenarios**:
  - Normal: two parallel traces 1mm apart with 0.15mm clearance → spread to fill gap
  - Normal: trace with 0.2mm width, 0.5mm clearance → fattened to fill space
  - Edge: traces already at minimum clearance → no spreading
  - Edge: single trace on layer → fattened to max width

**Code Intent**:
- New file `optimize/spreading.rs`: `spread_and_fatten()` function and `SpreadingReport`
- Spread phase: for each layer, collect all segments, sort by position, iteratively
  push segments apart by small increments while respecting clearance via spatial index
- Fatten phase: for each segment, query nearest neighbor distance, compute maximum
  width = min(neighbor_distance - clearance, policy.max_width)
- Modify `optimize/mod.rs`: add `pub mod spreading;` and call in `optimize_solution()`

---

### Milestone 8: Redundant Via Insertion

**Files**:
- `crates/autopcb-router/src/optimize/redundant_vias.rs` (new)
- `crates/autopcb-router/src/optimize/mod.rs`

**Flags**: `complex-algorithm`, `needs-rationale`

**Requirements**:
- Post-route pass that adds redundant (double-cut) vias adjacent to existing vias
  for manufacturing reliability
- Algorithm: build conflict graph (two candidate via locations conflict if they
  violate clearance), solve Maximum Independent Set for non-conflicting placements
- Candidate positions: 4 cardinal offsets from each existing via (offset = via
  diameter + clearance)
- Check each candidate against obstacle map and spatial index
- Return `RedundantViaReport { candidates_found: usize, inserted: usize,
  skipped_conflict: usize, skipped_no_room: usize }`
- DfmConfig.redundant_vias_enabled (default false — opt-in for high-reliability boards)

**Acceptance Criteria**:
- Via with open space → redundant via inserted adjacent
- Via in tight area → skipped (no room)
- Two adjacent vias → at most one redundant via between them (conflict graph)
- No DRC violations from redundant vias
- redundant_vias_enabled=false → no changes

**Tests**:
- **Test files**: `crates/autopcb-router/src/optimize/redundant_vias.rs` (inline)
- **Test type**: example-based
- **Backing**: user-specified
- **Scenarios**:
  - Normal: isolated via → redundant via inserted at one of 4 cardinal positions
  - Normal: via near board edge → fewer candidates, one inserted
  - Edge: via in dense area → all candidates blocked, skipped
  - Edge: two vias adjacent → conflict graph prevents overlapping redundant vias

**Code Intent**:
- New file `optimize/redundant_vias.rs`: `insert_redundant_vias()` function,
  `RedundantViaReport`, and conflict graph builder
- Candidate generation: for each via in solution, compute 4 cardinal offset positions
- Filtering: check each candidate against obstacle_maps.is_blocked() and
  spatial_index.query_envelope() for clearance
- Conflict graph: adjacency list where edge = two candidates violate mutual clearance
- MIS solver: greedy approximation (sort by degree ascending, greedily select)
- Insert selected candidates as new RoutedVia entries in solution
- Modify `optimize/mod.rs`: add module and call in pipeline

---

### Milestone 9: Copper Density Tracking

**Files**:
- `crates/autopcb-router/src/density.rs` (new)
- `crates/autopcb-router/src/workspace.rs`
- `crates/autopcb-router/src/drc/manufacturing.rs`
- `crates/autopcb-router/src/lib.rs`

**Flags**: `performance`

**Dependencies**: Milestone 3 (M9 integrates density check into drc/manufacturing.rs
created by M3)

**Requirements**:
- New `CopperDensityGrid` struct: per-layer tile-based density tracking
  - Tile size configurable via DfmConfig.density_tile_size_mm (default 1.0mm)
  - Each tile stores copper coverage percentage (0-100%)
- Compute baseline density during `build_workspace()` from existing copper
  (pads, pre-routed tracks, vias, polygons)
- After routing, recompute density including routed traces
- Report tiles exceeding density threshold (DfmConfig.density_max_pct, default 80%)
  and tiles below minimum (DfmConfig.density_min_pct, default 20%) as DRC violations
- Return `DensityReport { tiles_over: usize, tiles_under: usize,
  min_density_pct: f64, max_density_pct: f64, avg_density_pct: f64 }`
- Optional: suggest dummy fill locations (tiles below minimum)

**Acceptance Criteria**:
- Board with uniform copper → density near 50% everywhere, no violations
- Board with dense area → tiles over threshold reported
- Board with empty area → tiles under threshold reported
- DensityReport statistics are accurate
- copper_density_tracking=false → no density computation

**Tests**:
- **Test files**: `crates/autopcb-router/src/density.rs` (inline)
- **Test type**: property-based + example-based
- **Backing**: user-specified
- **Scenarios**:
  - Normal: board with single wide trace → density increases in trace tiles
  - Edge: empty board → all tiles 0% density
  - Edge: board completely filled → all tiles 100%
  - Proptest: density values always in [0.0, 100.0]

**Code Intent**:
- New file `density.rs`: `CopperDensityGrid` struct, `DensityReport`
- `CopperDensityGrid::build(ir, grid_config, tile_size_mm)` rasterizes all copper
  features (pads, tracks, vias, polygons) into tiles, computing area coverage
- `CopperDensityGrid::update(solution)` adds routed trace/via coverage
- `CopperDensityGrid::check(min_pct, max_pct) -> (Vec<DrcViolation>, DensityReport)`
- Modify `workspace.rs`: optionally compute baseline density during build
- Modify `drc/manufacturing.rs`: call density check in `check_manufacturing()`
- Modify `lib.rs`: add `pub mod density;`

---

### Milestone 10: Return Path Via Stitching

**Files**:
- `crates/autopcb-router/src/optimize/return_path.rs` (new)
- `crates/autopcb-router/src/optimize/mod.rs`

**Flags**: `complex-algorithm`, `needs-rationale`

**Dependencies**: Milestone 4 (Reference plane quality bitmap for plane detection)

**Requirements**:
- Post-route pass that adds ground stitching vias near signal layer transitions
- For each signal via that transitions between layers with different reference planes,
  find the nearest unoccupied grid cell within DfmConfig.return_via_max_distance_mm
  (default 2.0mm) and insert a ground via connecting the two reference planes
- Ground via net assignment: use the net of the reference plane (typically GND or
  a power net identified from the copper pour)
- Check placement against obstacle map and spatial index
- Return `ReturnPathReport { signal_vias_analyzed: usize, return_vias_inserted: usize,
  skipped_no_room: usize, skipped_same_reference: usize, skipped_no_plane_data: usize }`
- Run as last pass in optimize pipeline (after redundant vias)
- When `workspace.quality_grid` is None (reference_plane_quality=false),
  skip all insertions, log `tracing::warn("return_path_vias enabled but
  reference_plane_quality disabled — skipping, no plane data available")`,
  and return report with `skipped_no_plane_data: signal_vias_analyzed`

**Acceptance Criteria**:
- Signal via from layer 0 (ref: GND plane on layer 1) to layer 2 (ref: GND plane
  on layer 1) → same reference, no return via needed (skipped_same_reference++)
- Signal via from layer 0 (ref: GND on layer 1) to layer 2 (ref: VCC on layer 3)
  → return via inserted near signal via
- No DRC violations from return path vias
- return_path_vias_enabled=false → no changes

**Tests**:
- **Test files**: `crates/autopcb-router/src/optimize/return_path.rs` (inline)
- **Test type**: example-based
- **Backing**: user-specified
- **Scenarios**:
  - Normal: signal via changing reference plane → return via inserted within 2mm
  - Normal: signal via with same reference on both layers → skipped
  - Edge: no room for return via → skipped_no_room
  - Edge: board with only 2 layers → no reference plane changes possible
  - Edge: quality_grid is None (reference_plane_quality=false with
    return_path_vias_enabled=true) → tracing::warn logged,
    skipped_no_plane_data == signal_vias_analyzed, return_vias_inserted == 0, no panics

**Code Intent**:
- New file `optimize/return_path.rs`: `stitch_return_vias()` function and `ReturnPathReport`
- `stitch_return_vias(solution, workspace) -> Result<ReturnPathReport, RoutingError>`
  iterates all vias in solution, determines reference plane for source and target
  layers using QualityGrid (from M4), identifies layer transitions crossing plane
  boundaries, finds nearest clear location for ground via
- Reference plane identification: use QualityGrid cell values — solid (1.0) cells
  on adjacent layers indicate reference plane presence; use polygon/region net
  assignment from IR to determine reference net
- Modify `optimize/mod.rs`: add module and call as last pass

---

### Milestone 11: Documentation

**Delegated to**: @agent-technical-writer (mode: post-implementation)

**Source**: `## Invisible Knowledge` section of this plan

**Files**:
- `crates/autopcb-router/CLAUDE.md` (index updates)
- `crates/autopcb-router/README.md` (invisible knowledge)
- `crates/autopcb-router/src/optimize/README.md` (post-route pipeline docs)

**Requirements**:
Delegate to Technical Writer. Update CLAUDE.md index with new modules.
Create README.md with architecture diagram, data flow, invariants from
Invisible Knowledge section.

**Acceptance Criteria**:
- CLAUDE.md is tabular index only (no prose sections)
- README.md exists with architecture diagram matching Invisible Knowledge
- README.md is self-contained (no external references)
- Post-route pipeline order documented in optimize/README.md


## Milestone Dependencies

```
M1 (FabProfile) ──────→ M3 (Mfg DRC) ──────→ M5 (DFM in PathFinder)
                              │          \
M2 (Bend cost)                │           └──→ M9 (Copper density)
                              │                  (needs drc/manufacturing.rs)
M4 (Ref plane) ──────→ M10 (Return path)
                              │
M6 (Teardrops)                │
                              │
M7 (Spreading)                │
                              │
M8 (Redundant vias)           │
                              │
                         M11 (Documentation) ← all milestones
```

**Parallel waves**:
- Wave 1: M1 (FabProfile), M2 (Bend cost), M4 (Ref plane)
- Wave 2: M3 (Mfg DRC), M6 (Teardrops), M7 (Spreading), M8 (Redundant vias)
- Wave 3: M5 (DFM in PathFinder), M9 (Copper density — needs M3's drc/manufacturing.rs), M10 (Return path)
- Wave 4: M11 (Documentation)
