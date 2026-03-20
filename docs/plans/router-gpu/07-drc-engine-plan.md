# DRC Engine Implementation Plan

## Overview

Implement a comprehensive DRC (Design Rule Check) engine for the autopcb-router,
supporting both CPU and GPU backends, with full coverage of Altium's 70 design rule
types. The DRC engine serves two roles: (1) routing-time feedback — clearance and
short violations increment PathFinder history costs to drive convergence toward
DRC-clean solutions, and (2) post-route validation — comprehensive checking of all
applicable rules with detailed violation reporting.

Approach: Modular CPU engine first (per-rule submodules), GPU acceleration via
X-Check parallel sweepline for clearance+shorts, DrcPolicy builder mirroring
RoutingPolicy. Solverang constraint-based repair is deferred but violations carry
enough geometry for future integration.

## Planning Context

### Decision Log

| Decision | Reasoning Chain |
|----------|----------------|
| CPU engine as primary, GPU as acceleration | GPU sweepline is complex → CPU must work first as reference → GPU DRC only wins at >5K segments → most PCB boards are below this threshold → CPU is always available (CI, headless) |
| DRC module inside autopcb-router (not separate crate) | DRC needs RoutingWorkspace, RouteSolution, PathFinder history costs → separate crate creates circular dependency → DRC is tightly coupled to routing pipeline |
| DrcPolicy separate from RoutingPolicy | RoutingPolicy handles routing-time decisions (width, via, layers) → DrcPolicy handles all 30+ checkable rules including manufacturing → different lifecycles (routing policy is hot-path, DRC policy is validation) |
| Per-rule submodules (clearance.rs, width.rs, etc.) | 30+ rule types with distinct geometry checks → single file would exceed 2000 lines → per-rule files enable independent development and testing |
| Routing-time DRC: clearance + shorts only | Other rules (width, length, topology) are satisfied by construction during routing → clearance and shorts are the violations PathFinder needs to resolve → keeps routing-time DRC budget < 20% of iteration time |
| Fixed-point u32 for GPU costs | WGSL lacks f32 atomicMin → u32 atomicMin with scale 1024 gives 0.001 resolution → sufficient for clearance distances in mm |
| R-tree spatial index for CPU candidate detection | O(log n) range queries for candidate pairs → filter then exact-check → rstar already a dependency |
| ClearanceMatrix indexed by net-class pairs | Per-net clearance lookup is O(1) with matrix → most boards have < 20 net classes → matrix is small (20x20 = 400 entries) |
| DRC violations carry full geometry via DrcObject (not just location) | Future solverang repair needs violation geometry to compute constraint residuals → `DrcObject` enum (Segment, Via, Pad, Keepout, BoardEdge, Component, Polygon) enables representation of all violation participants → storing object refs, violation distance, and required clearance enables repair without re-analysis |
| DrcViolationRecord in autopcb-routes for serialization | DrcReport is in autopcb-router → RouteSolution is in autopcb-routes (thin format crate) → autopcb-routes must not depend on autopcb-router → define a `DrcViolationRecord` serializable type in autopcb-routes → DrcReport converts to Vec<DrcViolationRecord> at save boundary → CLI can load violations from .routes file without router dependency |
| check_routing() returns violations only, external loop updates history | DrcEngine::check_routing() returns DrcReport → PathFinder loop iterates violations and increments history costs → avoids double-update (engine + external) → single responsibility: engine detects, PathFinder penalizes → check_routing() does NOT take history parameter |
| Skip DRC in early PathFinder iterations (1-2) | Early iterations have many routing conflicts → DRC violations are noise → start DRC at iteration 3 to avoid wasted work |
| IrRuleParams typed variants for ALL DRC-checkable rules | CLAUDE.md fail-fast mandate → unknown rule kind must hard-error → every DRC-checkable RuleKind must have a typed IrRuleParams variant → includes zero-param rules (ShortCircuit, BrokenNets, ViasUnderSmd, NetAntennae) as marker variants |
| BTreeMap for ALL DrcPolicy lookups | Determinism invariant → HashMap has non-deterministic iteration → BTreeMap guarantees identical resolution order → applies to width_constraints, length_constraints, diff_pair_constraints, not just clearance matrix |
| DrcError type separate from RoutingError | DRC is not routing → DrcPolicy build errors and check failures are domain-distinct → `DrcError` enum with UnsupportedRule, PolicyBuildError, CheckFailed variants → `impl From<DrcError> for RoutingError` for PathFinder integration site |
| Delete drc.rs stub before creating drc/ module | Rust resolves `mod drc` to either `drc.rs` OR `drc/mod.rs`, not both → existing `drc.rs` stub must be deleted → M1 includes this migration step |
| Solverang for DRC repair + rubber-banding | DRC detection finds violations → solverang can *fix* them by adjusting trace vertex positions → constraints are `dist(A,B) - gap ≥ 0` (inequality via slack) → squared-distance formulation avoids Jacobian singularity at zero distance → solverang already used in placement (same crate ecosystem) → repair is more powerful than rip-up+reroute for small violations (nudge trace 0.01mm vs re-route entire net) |
| Solverang repair as separate milestone after GPU DRC | Repair depends on detection working correctly → GPU DRC is an optimization, repair is a capability → decoupled milestones allow shipping detection-only first → repair adds solverang dependency to autopcb-router |
| Rubber-banding via solverang (not GPU) | Rubber-banding pulls trace vertices toward shorter paths subject to clearance constraints → this is continuous optimization, perfect for LM solver → GPU would require custom optimizer in WGSL → solverang already has the machinery (ConstraintSystem, Jacobian, inequality constraints) |

### Rejected Alternatives

| Alternative | Why Rejected |
|-------------|-------------|
| Separate `autopcb-drc` crate | Circular dependency: DRC needs routing workspace + PathFinder history → would require extracting workspace to shared crate → over-engineering |
| GPU-only DRC | GPU dispatch overhead > CPU time for small boards → no GPU in CI → CPU reference always needed |
| Full DRC every PathFinder iteration | Too slow — full DRC includes length matching, topology, manufacturing → 30+ checks per iteration is expensive → clearance+shorts is sufficient for convergence |
| Solverang repair in initial implementation | Continuous solver for discrete grid positions adds complexity → design violations for future repair but implement detection first |
| Single monolithic drc.rs | 30+ rule types × geometry checks = >3000 lines → unmaintainable → per-rule modules with shared DrcEngine trait |
| GPU rubber-banding instead of solverang | Would require implementing LM optimizer in WGSL → no standard GPU optimization library → solverang already has ConstraintSystem, auto-Jacobian, inequality support → CPU solverang on trace vertices is fast enough (< 100ms for typical boards) |
| Solverang for routing (replacing PathFinder) | Routing is discrete graph search (path exists or doesn't) → LM solver needs continuous differentiable residuals → path finding is non-differentiable → solverang handles post-route optimization, not route finding |

### Constraints & Assumptions

- All DRC checks must be deterministic (same PcbIr + RouteSolution → identical violations)
- DRC must work without GPU (CPU engine is mandatory, GPU is optional)
- IrRuleParams extensions follow CLAUDE.md fail-fast: no `Other` catch-all for DRC-checkable rules
- Testing uses synthetic PcbIr (no PcbDoc fixtures per CLAUDE.md)
- Property-based tests behind `proptest` feature gate
- GPU tests behind `gpu-tests` feature gate with graceful skip on no-adapter
- Coordinates in mm (f64) matching PcbIr convention
- `default-conventions domain="testing"` applied: prefer integration + property-based over unit

### Known Risks

| Risk | Mitigation | Anchor |
|------|-----------|--------|
| IrRuleParams may not have all fields for complex rules | Add typed variants incrementally, hard-error on missing data | crates/autopcb-ir/src/rule.rs |
| GPU sweepline may be too complex for 45° traces | Decompose 45° segments to bounding boxes for sweep, exact distance in check kernel | docs/plans/router-gpu/03-xcheck-gpu-drc.md |
| DRC history cost model may cause PathFinder oscillation | Use linear penalty (proven in McMurchie-Ebeling 1995), tune magnitude empirically | N/A (new code) |
| Per-net-class clearance matrix may be incomplete | Validate all net-class pairs present during DrcPolicy construction, error on missing | N/A (new code) |

## Invisible Knowledge

### Architecture

```
DrcEngine trait (CPU or GPU backend)
    │
    ├── check_routing() ─── fast path, per PathFinder iteration
    │   ├── clearance check (segment-to-segment, segment-to-pad)
    │   └── short circuit detection (overlapping different-net segments)
    │
    └── check_full() ─── comprehensive, after routing converges
        ├── clearance (all object types)
        ├── width (min/max/preferred per layer)
        ├── via (hole size, annular ring, count limits)
        ├── shorts (overlapping geometry)
        ├── connectivity (broken nets, antennae)
        ├── length (min/max, matched groups)
        ├── geometry (acute angles, SMD-to-corner)
        ├── board (outline clearance, component clearance)
        ├── diff_pair (gap, width, uncoupled length, skew)
        ├── topology (daisy chain stub, routing topology)
        └── manufacturing (solder mask, silk clearance)
```

### Data Flow

```
PcbIr (design rules)  +  RouteSolution (routed segments/vias)
         │                           │
         ▼                           ▼
    DrcPolicy                  DRC input geometry
  (rule resolution,          (segments, vias, pads,
   clearance matrix,          keepouts, board edge)
   per-net constraints)            │
         │                         │
         └────────┬────────────────┘
                  ▼
          DrcEngine::check_routing()  ← per PathFinder iteration
                  │
                  ├── violations[] → history_costs[] (increment at violation cells)
                  └── violation_count → convergence metric

          DrcEngine::check_full()     ← after convergence
                  │
                  └── DrcReport → CLI output, RouteSolution.metrics
```

### Invariants

- DRC violations are deterministic: same input → identical violation set
- Routing-time DRC never false-negatives on clearance (may have false positives from grid quantization)
- CPU and GPU engines produce identical violation sets (when GPU is available)
- DrcPolicy uses BTreeMap for deterministic rule resolution order
- All DRC-checkable rules have typed IrRuleParams variants (no silent skipping via `Other`)

### Tradeoffs

- **Routing-time DRC scope**: Only clearance+shorts during routing (fast) vs full DRC per iteration (slow but more accurate). Chose fast — other rules are satisfied by construction.
- **CPU vs GPU**: CPU is simpler and always available. GPU wins at >5K segments but adds complexity. Both share same DrcEngine trait.
- **Detection then repair**: DRC detects violations (M1-M7), solverang repairs them (M10-M11). Repair is a continuous optimization on trace vertex positions subject to clearance constraints. This is more precise than rip-up+reroute for small violations (solverang nudges a vertex 0.01mm vs PathFinder re-routing an entire net).
- **Solverang rubber-banding vs GPU rubber-banding**: Solverang (CPU, LM solver) naturally handles inequality constraints and produces mathematically optimal vertex positions. GPU would require implementing an optimizer in WGSL with no standard library support. CPU solverang is fast enough (< 100ms for typical boards) — the bottleneck is the solve, not the arithmetic.

### Solverang Integration Architecture

```
Post-route pipeline:
  RouteSolution (from PathFinder)
         │
         ▼
  DRC detect (CPU or GPU)
  → DrcReport with violations
         │
         ▼
  Solverang repair (M10)
  → For each violation cluster:
     • Extract trace vertices as solvable params (x, y)
     • Pin pad endpoints (fixed)
     • Build ClearanceConstraints: dist²(A,B) - gap² ≥ 0
     • LM solve → adjusted vertex positions
     • Write back to RouteSolution
         │
         ▼
  Solverang rubber-band (M11)
  → Per-net optimization:
     • All trace vertices solvable
     • Objective: minimize total length
     • Constraints: clearance to all nearby objects
     • LM solve → tighter traces
         │
         ▼
  DRC re-check (verify 0 violations)
         │
         ▼
  Final RouteSolution
```

The squared-distance formulation (`dist²` instead of `dist`) is critical: it avoids
the `1/dist` singularity in the Jacobian when two objects touch, giving the solver
smooth gradients everywhere. This is documented in `docs/future/solverang/constraint-types.md`.

## Plan Flags

| Flag | Consumer | Meaning |
|------|----------|---------|
| `conformance` | QR | Cross-crate API contracts with autopcb-ir and autopcb-routes |
| `needs-rationale` | TW | Non-obvious DRC algorithm choices need inline WHY comments |
| `performance` | QR | Routing-time DRC is inner-loop code; profiling required |
| `complex-algorithm` | QR/TW | GPU sweepline and spatial indexing need strategy comments |

## Milestones

### Milestone 1: DRC Core Types + DrcPolicy

**Files**:
- `crates/autopcb-router/src/drc/mod.rs` (replaces existing `drc.rs` stub)
- `crates/autopcb-router/src/drc/policy.rs`
- `crates/autopcb-router/src/drc/report.rs`
- `crates/autopcb-routes/src/lib.rs` (extend with `DrcViolationRecord`)

**Flags**: `conformance`

**Requirements**:
- Delete existing `crates/autopcb-router/src/drc.rs` stub (Rust cannot have both `drc.rs` and `drc/mod.rs`)
- Define `DrcEngine` trait with `check_routing(&self, solution, workspace) -> Result<DrcReport, DrcError>` and `check_full(&self, solution, workspace, ir) -> Result<DrcReport, DrcError>` (note: check_routing does NOT take history parameter — history updates happen in PathFinder loop)
- Define `DrcError` enum: `UnsupportedRule { kind: String }`, `PolicyBuildError(String)`, `CheckFailed(String)`. Implement `From<DrcError> for RoutingError`
- Define `DrcObject` enum: `Segment(TraceSegment)`, `Via(RoutedVia)`, `Pad { component: String, pad: String, position: PointMm }`, `Keepout { id: usize }`, `BoardEdge`, `Component { designator: String }`, `Polygon { id: usize }`
- Define `DrcViolation` struct: `kind: DrcViolationKind`, `rule_kind: RuleKind`, `rule_name: String`, `object_a: DrcObject`, `object_b: Option<DrcObject>`, `location: PointMm`, `layer: Option<LayerId>`, `actual_mm: f64`, `required_mm: f64`
- Define `DrcViolationKind` enum with all 29 violation types from plan doc
- Define `DrcReport` with per-rule violation counts, total count, violation list, `count_by_rule() -> BTreeMap<RuleKind, usize>`
- Define `DrcViolationRecord` in `autopcb-routes` (serializable): `kind_name: String`, `location: Point`, `layer: Option<u16>`, `actual_mm: f64`, `required_mm: f64`, `rule_name: String`. Add `drc_violation_records: Vec<DrcViolationRecord>` to `RouteSolution`
- Define `DrcPolicy` struct built from `PcbIr::design_rules` using BTreeMap for ALL lookups
- Build `ClearanceMatrix` (per-net-class-pair clearance values) from design rules
- `DrcPolicy::clearance(net_a_class, net_b_class) -> f64` lookup
- `DrcPolicy::width_bounds(net_class, layer) -> (min, max, preferred)` via `BTreeMap<Option<String>, WidthConstraint>`
- `DrcPolicy::via_bounds(net_class) -> ViaBounds`

**Acceptance Criteria**:
- `DrcPolicy` builds from synthetic PcbIr with multiple net classes and design rules
- `ClearanceMatrix` resolves correct clearance for all net-class pairs
- Rule precedence: lower priority number wins (matching RoutingPolicy behavior)
- Unknown rule kinds return `DrcError::UnsupportedRule`
- `DrcViolationRecord` round-trips through serde (bincode + JSON)

**Tests**:
- **Test files**: `crates/autopcb-router/src/drc/policy.rs` (inline `#[cfg(test)]`)
- **Test type**: unit
- **Backing**: default-derived
- **Scenarios**:
  - Single clearance rule → all pairs get that clearance
  - Net-class-specific clearance overrides default
  - Two conflicting rules → higher priority wins
  - Width bounds resolved per layer

**Code Intent**:
- Delete `crates/autopcb-router/src/drc.rs` (stub file, to be replaced by module directory)
- New `drc/mod.rs`: module declarations, `DrcEngine` trait (check_routing returns DrcReport, does NOT take history param), `DrcError` enum, `DrcViolation` struct, `DrcViolationKind` enum, `DrcObject` enum
- New `drc/policy.rs`: `DrcPolicy` struct with `build_policy(ir: &PcbIr) -> Result<DrcPolicy, DrcError>`, `ClearanceMatrix` as flat `Vec<f64>` with `class_map: BTreeMap<String, usize>` for O(1) lookup, `width_constraints: BTreeMap<Option<String>, WidthConstraint>`, `length_constraints: BTreeMap<NetId, LengthConstraint>`, `diff_pair_constraints: BTreeMap<NetId, DiffPairConstraint>`
- New `drc/report.rs`: `DrcReport` struct with `violations: Vec<DrcViolation>`, `count_by_rule() -> BTreeMap<RuleKind, usize>`, `render()` for CLI, `render_summary()` for compact output, `to_violation_records() -> Vec<DrcViolationRecord>` for RouteSolution serialization
- Extend `autopcb-routes/src/lib.rs`: add `DrcViolationRecord` struct (serde Serialize + Deserialize), add `drc_violation_records: Vec<DrcViolationRecord>` to `RouteSolution`
- `DrcViolation` carries: `kind: DrcViolationKind`, `rule_kind: RuleKind`, `rule_name: String`, `object_a: DrcObject`, `object_b: Option<DrcObject>`, `location: PointMm`, `layer: Option<LayerId>`, `actual_mm: f64`, `required_mm: f64`

---

### Milestone 2: CPU Clearance + Short Circuit Detection

**Files**:
- `crates/autopcb-router/src/drc/clearance.rs`
- `crates/autopcb-router/src/drc/shorts.rs`

**Flags**: `performance`, `complex-algorithm`

**Requirements**:
- Clearance checking: segment-to-segment, segment-to-pad, segment-to-via, via-to-via, segment-to-keepout, segment-to-board-edge
- Minimum distance calculation between two line segments (2D geometry)
- Per-net-class clearance from DrcPolicy
- Same-net filtering (same-net segments don't violate clearance)
- Short circuit detection: overlapping segments from different nets on same layer
- R-tree spatial index for candidate pair detection (reuse `SpatialIndex` from workspace)

**Acceptance Criteria**:
- Two parallel traces 0.1mm apart with 0.2mm clearance rule → violation detected
- Two traces from same net touching → no violation
- Overlapping traces from different nets → short circuit detected
- Trace touching board edge within clearance → violation detected

**Tests**:
- **Test files**: `crates/autopcb-router/src/drc/clearance.rs`, `shorts.rs` (inline `#[cfg(test)]`)
- **Test type**: unit + property-based (proptest behind feature gate)
- **Backing**: user-specified (both)
- **Scenarios**:
  - Two segments, parallel, known distance → correct violation distance
  - Two segments, perpendicular crossing → short detected
  - Same-net segments overlapping → no violation
  - Segment near pad → clearance violation with correct distance
  - Property: violation.actual_value < violation.required_value for all violations

**Code Intent**:
- New `drc/clearance.rs`: `check_clearance(solution: &RouteSolution, policy: &DrcPolicy, spatial: &SpatialIndex) -> Vec<DrcViolation>`
- Geometry helpers: `segment_to_segment_distance(s1, s2) -> f64`, `segment_to_point_distance(s, p) -> f64`
- Candidate detection: R-tree query with clearance envelope, then exact distance check
- New `drc/shorts.rs`: `check_shorts(solution: &RouteSolution) -> Vec<DrcViolation>`
- Short detection: per-layer, check if any two different-net segments overlap (intersection test)

---

### Milestone 3: CPU Width, Via, and Geometry Checks

**Files**:
- `crates/autopcb-router/src/drc/width.rs`
- `crates/autopcb-router/src/drc/via.rs`
- `crates/autopcb-router/src/drc/geometry.rs`

**Requirements**:
- Width checking: verify each segment width is within min/max bounds per net class per layer
- Via checking: hole size bounds, annular ring minimum, maximum via count per net
- Hole-to-hole clearance between vias
- Geometry checking: acute angle detection (angle between consecutive segments < threshold), SMD-to-corner distance

**Acceptance Criteria**:
- Segment with width 0.1mm and min 0.15mm → width violation
- Via with annular ring 0.05mm and min 0.1mm → annular ring violation
- Net with 6 vias and max 5 → via count violation
- 30° angle between segments with 45° minimum → acute angle violation

**Tests**:
- **Test files**: `crates/autopcb-router/src/drc/width.rs`, `via.rs`, `geometry.rs` (inline)
- **Test type**: unit
- **Backing**: default-derived
- **Scenarios**:
  - Width within bounds → pass
  - Width below min → violation with correct delta
  - Via annular ring check with various geometries
  - Angle between segments: 90° pass, 30° fail

**Code Intent**:
- New `drc/width.rs`: `check_widths(solution, policy) -> Vec<DrcViolation>` — iterate all segments, check width against `policy.width_bounds(net_class, layer)`
- New `drc/via.rs`: `check_vias(solution, policy) -> Vec<DrcViolation>` — hole size, annular ring, via count, hole-to-hole clearance
- New `drc/geometry.rs`: `check_geometry(solution, policy) -> Vec<DrcViolation>` — consecutive segment angle calculation, SMD pad entry angle

---

### Milestone 4: CPU Connectivity, Length, and Diff-Pair Checks

**Files**:
- `crates/autopcb-router/src/drc/connectivity.rs`
- `crates/autopcb-router/src/drc/length.rs`
- `crates/autopcb-router/src/drc/diff_pair.rs`
- `crates/autopcb-router/src/drc/topology.rs`

**Flags**: `needs-rationale`

**Requirements**:
- Connectivity: detect broken nets (unrouted pins), net antennae (dead-end traces)
- Length: min/max net length, matched-length group tolerance
- Diff pair: gap enforcement (min/max), width matching, uncoupled length limit, skew tolerance
- Topology: daisy chain stub length limit

**Acceptance Criteria**:
- Net with 4 pins, only 3 connected → broken net violation
- Net length 150mm with max 100mm → length violation
- Matched group: nets at 100mm and 120mm with 5mm tolerance → violation (delta 20mm > 5mm)
- Diff pair gap 0.05mm with min 0.1mm → gap violation

**Tests**:
- **Test files**: inline per file
- **Test type**: unit + integration (multi-rule synthetic board)
- **Backing**: user-specified
- **Scenarios**:
  - Fully connected net → no connectivity violation
  - Matched length within tolerance → pass
  - Diff pair with consistent gap → pass
  - Topology: daisy chain with long stub → violation

**Code Intent**:
- New `drc/connectivity.rs`: `check_connectivity(solution, ir) -> Vec<DrcViolation>` — compare solution's routed pins vs IR's expected pins per net
- New `drc/length.rs`: `check_lengths(solution, policy) -> Vec<DrcViolation>` — sum segment lengths per net, check against min/max, check matched groups
- New `drc/diff_pair.rs`: `check_diff_pairs(solution, policy, ir) -> Vec<DrcViolation>` — identify paired nets via `IrNet.diff_pair_partner`, check gap/width/uncoupled/skew
- New `drc/topology.rs`: `check_topology(solution, policy) -> Vec<DrcViolation>` — daisy chain stub length

---

### Milestone 5: CPU Board + Manufacturing Checks

**Files**:
- `crates/autopcb-router/src/drc/board.rs`
- `crates/autopcb-router/src/drc/manufacturing.rs`

**Requirements**:
- Board outline clearance: all copper objects must be ≥ clearance from board edge
- Component clearance: component courtyards must not overlap beyond threshold
- Solder mask expansion/sliver checking
- Silk-to-solder-mask clearance, silk-to-silk clearance, silk-to-board-region clearance

**Acceptance Criteria**:
- Trace 0.1mm from board edge with 0.3mm clearance → violation
- Components overlapping courtyards → violation
- Solder mask sliver < minimum → violation

**Tests**:
- **Test files**: inline per file
- **Test type**: unit
- **Backing**: default-derived
- **Scenarios**:
  - Trace within board with margin → pass
  - Trace near board edge → violation with correct distance

**Code Intent**:
- New `drc/board.rs`: `check_board(solution, ir, policy) -> Vec<DrcViolation>` — board outline geometry from `IrBoardGeometry`, segment-to-polygon distance
- New `drc/manufacturing.rs`: `check_manufacturing(solution, ir, policy) -> Vec<DrcViolation>` — mask expansion, silk clearance (may return empty if manufacturing data not in IR)

---

### Milestone 6: IrRuleParams Extensions

**Files**:
- `crates/autopcb-ir/src/rule.rs`
- `crates/autopcb-ir/src/extract.rs`

**Flags**: `conformance`

**Requirements**:
- Audit existing typed `IrRuleParams` variants (DO NOT re-add these — they already exist):
  `Clearance`, `Width`, `ComponentClearance`, `BoardOutlineClearance`, `HoleToHoleClearance`,
  `MinimumAnnularRing`, `SolderMaskExpansion`, `PasteMaskExpansion`, `RoutingTopology`,
  `RoutingPriority`, `RoutingLayers`, `RoutingViaStyle`, `RoutingCornerStyle`, `DiffPairsRouting`,
  `MatchedLengths`
- Add NEW typed `IrRuleParams` variants for all remaining DRC-checkable rules:
  `ShortCircuit` (marker, no params), `BrokenNets` (marker), `NetAntennae` (marker),
  `ViasUnderSmd` (marker), `AcuteAngle { min_angle_deg: f64 }`,
  `SmdToCorner { clearance_mm: f64 }`, `MaximumViaCount { max: u32 }`,
  `MaxMinHoleSize { min_mm: f64, max_mm: f64 }`, `Length { min_mm: f64, max_mm: f64 }`,
  `DaisyChainStubLength { max_mm: f64 }`, `SmdNeckDown`, `SmdEntry`,
  `ParallelSegment { max_run_mm: f64, check_gap_mm: f64 }`,
  `MinimumSolderMaskSliver { min_mm: f64 }`,
  `SilkToSolderMaskClearance { clearance_mm: f64 }`,
  `SilkToSilkClearance { clearance_mm: f64 }`,
  `SilkToBoardRegionClearance { clearance_mm: f64 }`
- Populate new variants during `extract_rules()` — parse from PcbDoc rule records
- Dispatch new `RuleKind` values to typed variants BEFORE the `_ => Other` wildcard arm

**Acceptance Criteria**:
- `cargo test -p autopcb-ir` passes
- New rule kinds no longer fall through to `Other { kind }`
- Existing tests continue to pass (backward compatible)

**Tests**:
- **Test files**: `crates/autopcb-ir/src/rule.rs` (inline)
- **Test type**: unit
- **Backing**: default-derived
- **Scenarios**:
  - Round-trip: construct IrDesignRule with new variant, verify fields

**Code Intent**:
- Extend `IrRuleParams` enum with ~15 new variants
- In `extract_rules()`: add match arms for each new `RuleKind` variant before the wildcard
- Parse rule parameters from the param-based rule record format (key-value pairs)

---

### Milestone 7: DRC Engine Integration with PathFinder

**Files**:
- `crates/autopcb-router/src/drc/mod.rs` (extend)
- `crates/autopcb-router/src/drc/cpu_engine.rs`
- `crates/autopcb-router/src/pathfinder/mod.rs` (extend)
- `crates/autopcb-router/src/solution.rs` (extend)

**Flags**: `performance`, `needs-rationale`

**Requirements**:
- Implement `CpuDrcEngine` struct implementing `DrcEngine` trait
- `check_routing()`: runs clearance + shorts checks, returns `DrcReport` (does NOT update history — separation of concerns)
- `check_full()`: runs all checks from M2-M5, returns comprehensive DrcReport
- Integrate DRC into PathFinder iteration loop: after all nets routed, call `check_routing()`, then PathFinder loop iterates `report.violations` and increments `history[violation.grid_cell] += violation_penalty` (single update site, no double-counting)
- DRC violations increment history costs at violation grid cells (in PathFinder loop, NOT in engine)
- `drc_violation_count == 0` is true convergence condition
- Update `RouteSolution::metrics::drc_violations` from full DRC
- Skip DRC in iterations 1-2 (configurable via `DrcConfig.start_iteration`)

**Acceptance Criteria**:
- PathFinder with DRC enabled converges to solution with 0 clearance violations
- DRC violations appear in RouteSolution.metrics
- Skipping DRC in early iterations does not affect final result quality
- Full DRC produces DrcReport with all violation categories

**Tests**:
- **Test files**: `crates/autopcb-router/src/drc/cpu_engine.rs` (inline)
- **Test type**: integration (synthetic board with known violations)
- **Backing**: user-specified
- **Scenarios**:
  - 2 nets, no violations → DRC passes, PathFinder converges
  - 2 crossing nets → DRC detects clearance violation, PathFinder resolves it
  - Full DRC on synthetic board → correct violation count per rule type

**Code Intent**:
- New `drc/cpu_engine.rs`: `CpuDrcEngine` with spatial index, dispatches to per-rule checkers from M2-M5
- Extend `drc/mod.rs`: `DrcConfig { start_iteration: u32, enabled_checks: Vec<DrcCheckKind> }`
- Extend `pathfinder/mod.rs`: after net routing, call `drc.check_routing()`, for each violation: `history[violation.grid_cell] += violation_penalty`
- Extend `solution.rs`: `RouteSolutionBuilder::set_drc_report(report: DrcReport)`

---

### Milestone 8: GPU DRC Pipeline

**Files**:
- `crates/autopcb-router/src/gpu/drc.rs`
- `crates/autopcb-router/src/gpu/shaders/segment_extract.wgsl`
- `crates/autopcb-router/src/gpu/shaders/segment_sort.wgsl`
- `crates/autopcb-router/src/gpu/shaders/sweepline_check.wgsl`
- `crates/autopcb-router/src/gpu/shaders/short_check.wgsl`
- `crates/autopcb-router/src/gpu/shaders/width_check.wgsl`
- `crates/autopcb-router/src/gpu/shaders/via_check.wgsl`
- `crates/autopcb-router/src/gpu/shaders/violation_compact.wgsl`
- `crates/autopcb-router/src/gpu/shaders/drc_history_update.wgsl`

**Flags**: `complex-algorithm`, `performance`

**Requirements**:
- Implement `GpuDrcEngine` implementing `DrcEngine` trait
- Parallel sweepline for clearance checking (X-Check algorithm)
- GPU short circuit detection via occupancy checking
- Violation compaction (stream compaction to remove NULL entries)
- History cost update from violations (GPU-side atomicAdd)
- Auto-selection: GPU if >5000 segments, CPU otherwise

**Acceptance Criteria**:
- GPU DRC produces identical violation set as CPU DRC
- GPU DRC is faster than CPU for boards with >5000 segments
- GPU DRC updates history costs without CPU round-trip
- Graceful fallback to CPU when no GPU available

**Tests**:
- **Test files**: `crates/autopcb-router/src/gpu/drc.rs` (inline, behind `gpu-tests` feature)
- **Test type**: integration (CPU vs GPU comparison)
- **Backing**: user-specified
- **Scenarios**:
  - Same segment set → CPU and GPU produce identical violations
  - Large synthetic board (10K segments) → GPU faster than CPU
  - No GPU available → falls back to CPU without error

**Code Intent**:
- New `gpu/drc.rs`: `GpuDrcEngine` using `GpuRoutingEngine` (shared device, buffers)
- Upload routed segments to GPU segment buffer
- 8 WGSL shaders: segment_extract (per-layer filtering), segment_sort (y-coordinate radix sort), sweepline_check (parallel clearance), short_check (occupancy overlap), width_check (per-segment width bounds), via_check (hole size + annular ring), violation_compact (stream compaction), drc_history_update (history cost increment)
- Dispatch pipeline: segment extraction → sort → sweepline → short check → width check → via check → violation compact → history update
- Per-layer processing (filter segments by layer before sweep)
- Clearance matrix uploaded as uniform buffer
- Violation output: compacted into readback buffer, mapped to CPU for DrcReport
- Auto-select via `DrcConfig.gpu_threshold: usize` (default 5000)

---

### Milestone 9: CLI + Reporting

**Files**:
- `crates/altium-cli/src/main.rs` (extend)
- `crates/autopcb-router/src/drc/report.rs` (extend)

**Requirements**:
- CLI: `altium routing inspect <routes-file>` includes DRC summary
- DRC report renders as categorized table (per-rule violation counts)
- Violation details available with `--verbose` flag
- JSON output available with `--json` flag

**Acceptance Criteria**:
- `altium routing inspect board.routes` shows DRC violation summary
- `--verbose` shows individual violations with locations
- `--json` outputs machine-readable violation data

**Tests**:
- **Test files**: `crates/altium-cli/src/main.rs` (inline)
- **Test type**: unit (output formatting)
- **Backing**: default-derived
- **Scenarios**:
  - DrcReport with 0 violations → "DRC: PASS" output
  - DrcReport with violations → categorized table

**Code Intent**:
- Extend `drc/report.rs`: `DrcReport::render_table() -> String`, `DrcReport::render_verbose() -> String`, `DrcReport::to_json() -> serde_json::Value`
- Extend CLI: add `--drc` flag to `routing inspect` command, run full DRC on loaded RouteSolution

---

### Milestone 10: Solverang DRC Repair

**Files**:
- `crates/autopcb-router/src/drc/repair.rs`
- `crates/autopcb-router/Cargo.toml` (add solverang dependency)

**Flags**: `complex-algorithm`, `needs-rationale`

**Requirements**:
- Add `solverang` dependency to autopcb-router: `solverang = { path = "../../../solverang/crates/solverang" }`
- For each DRC violation cluster: extract nearby trace vertices as solvable parameters
- Build solverang `ConstraintSystem` with clearance constraints: `dist²(A, B) - gap² ≥ 0` (squared-distance formulation, no sqrt singularity)
- Each trace vertex becomes two solvable parameters (x, y)
- Fixed objects (pads, vias, board edge) are non-solvable entities — only trace vertices move
- Run LM solver with `SystemConfig` tuned for small local adjustments (tight convergence, few iterations)
- After repair: re-run DRC to verify violations resolved
- If solverang can't fix a violation (solver diverges or constraint infeasible): leave violation in report, don't corrupt the route

**Acceptance Criteria**:
- Trace 0.09mm from pad with 0.1mm clearance → solverang nudges trace to 0.1mm → DRC passes
- Trace that can't be moved (boxed in by obstacles) → violation remains, trace not corrupted
- Repair does not increase total trace length by more than 5%
- Repair preserves connectivity (no broken nets after adjustment)

**Tests**:
- **Test files**: `crates/autopcb-router/src/drc/repair.rs` (inline)
- **Test type**: unit + integration
- **Backing**: user-specified
- **Scenarios**:
  - Single clearance violation, space to move → repaired
  - Multiple clustered violations → all repaired in one solve pass
  - Infeasible violation (no room) → violation preserved, route unchanged
  - Repair + re-DRC round-trip: violation count decreases

**Code Intent**:
- New `drc/repair.rs`: `repair_violations(solution: &mut RouteSolution, violations: &[DrcViolation], ir: &PcbIr, policy: &DrcPolicy) -> RepairResult`
- `RepairResult { repaired_count: usize, remaining_violations: Vec<DrcViolation> }`
- For each violation: extract the two involved objects → find movable trace vertices within a radius → create `solverang::Entity` for each vertex → create `ClearanceConstraint` (inequality, squared-distance) → solve
- Constraint types from `docs/future/solverang/constraint-types.md`:
  - `CopperClearance`: `dist²(seg_a, seg_b) - gap² ≥ 0`
  - `BoardEdgeClearance`: `dist²(vertex, outline) - gap² ≥ 0`
  - `ComponentClearance`: `bbox_dist²(trace, courtyard) - gap² ≥ 0`
- After solve: update `TraceSegment.start`/`.end` coordinates in RouteSolution
- Verify connectivity preserved (segment endpoints still connect to adjacent segments/pads)

---

### Milestone 11: Solverang Rubber-Banding

**Files**:
- `crates/autopcb-router/src/optimize/rubber_band.rs` (extend existing 217-line file)

**Flags**: `complex-algorithm`, `performance`

**Requirements**:
- Replace or augment existing rubber-banding with solverang-based optimization
- Objective: minimize total trace length subject to ALL clearance constraints
- Each trace vertex (x, y) is a solvable parameter
- Constraints:
  - Clearance to all nearby obstacles (pads, other traces, keepouts, board edge)
  - Clearance to other nets' traces
  - Connectivity preservation (endpoints pinned to pad locations)
- Use spatial index (R-tree) to find nearby obstacles for each vertex → only build constraints for nearby objects (not all-pairs)
- Run LM solver per-net (not all nets simultaneously — too many parameters)
- Iterate: optimize net, update spatial index with new positions, optimize next net

**Acceptance Criteria**:
- Rubber-banded traces are shorter than input traces
- No clearance violations introduced by rubber-banding
- Connectivity preserved (all pads still connected)
- Runtime < 1 second for typical PCB boards (< 2000 nets)

**Tests**:
- **Test files**: `crates/autopcb-router/src/optimize/rubber_band.rs` (inline)
- **Test type**: unit + property-based (proptest: rubber-banded length ≤ original length)
- **Backing**: user-specified
- **Scenarios**:
  - Trace with slack (unnecessary detour) → shortened
  - Trace already tight (no slack) → unchanged
  - Trace near obstacle → shortened but maintains clearance
  - Property: post-rubber-band DRC produces 0 new violations

**Code Intent**:
- Extend `optimize/rubber_band.rs`: `rubber_band_solverang(solution: &mut RouteSolution, workspace: &RoutingWorkspace, policy: &DrcPolicy)`
- Per-net optimization loop:
  1. Extract net's trace vertices as `solverang::Entity` with (x, y) params
  2. Pin endpoint vertices to pad positions (fixed params)
  3. Query R-tree for nearby obstacles within clearance radius of each vertex
  4. Build `ClearanceConstraint` for each (vertex, obstacle) pair
  5. Objective residuals: minimize segment lengths (sum of `sqrt(dx² + dy²)` per segment)
  6. Run `ConstraintSystem::solve()` with LMConfig
  7. Write back optimized vertex positions to TraceSegments
  8. Update spatial index with new segment positions
- Falls back to existing geometric rubber-banding if solverang is not available or solve diverges

---

### Milestone 12: Documentation

**Delegated to**: @agent-technical-writer (mode: post-implementation)

**Source**: `## Invisible Knowledge` section of this plan

**Files**:
- `crates/autopcb-router/src/drc/README.md`
- `crates/autopcb-router/src/gpu/README.md` (update with DRC section)

**Requirements**:
- README.md captures DRC architecture, data flow, rule coverage table
- GPU DRC algorithm explanation (X-Check parallel sweepline)
- Rule coverage matrix (which rules are implemented, CPU/GPU status)

**Acceptance Criteria**:
- README.md exists in drc/ directory
- Architecture diagram matches Invisible Knowledge section
- Rule coverage table is complete and accurate

## Milestone Dependencies

```
M1 (core types) ──→ M2 (clearance+shorts) ──→ M7 (PathFinder integration)
                ──→ M3 (width+via+geometry)──→ M7
                ──→ M4 (connectivity+length)→ M7
                ──→ M5 (board+manufacturing)→ M7
                                                │
M6 (IR extensions) ────────────────────────────→ M7
                                                │
                                    ┌───────────┼───────────┐
                                    ▼           ▼           ▼
                              M8 (GPU DRC) M9 (CLI) M10 (solverang repair)
                                    │           │           │
                                    │           │           ▼
                                    │           │    M11 (rubber-banding)
                                    │           │           │
                                    └─────┬─────┴───────────┘
                                          ▼
                                    M12 (docs)
```

**Parallel opportunities:**
- M2, M3, M4, M5 can proceed in parallel after M1 (different files, different rule types)
- M6 can proceed in parallel with M2-M5 (different crate)
- M8, M9, M10 can proceed in parallel after M7
- M11 depends on M10 (needs solverang constraint types from repair module)
