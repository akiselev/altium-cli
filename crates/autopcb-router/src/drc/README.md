# DRC Engine

Design-rule checking for `autopcb-router`. Validates `RouteSolution` against
the rules in `PcbIr`, operates in two modes: fast routing-time feedback and
comprehensive post-route validation.

## Architecture

```
DrcEngine trait
    |
    +-- check_routing()  -- fast path, called per PathFinder iteration
    |       shorts::check_shorts()          (O(n²) same-layer pairs)
    |
    +-- check_full()     -- comprehensive, called after routing converges
            clearance, shorts, width, via, geometry, connectivity,
            length, diff_pair, board, manufacturing, topology
```

Two backend implementations share the same trait:

| Backend | Type | Selection |
|---------|------|-----------|
| `CpuDrcEngine` | `cpu_engine.rs` | Always available |
| `GpuDrcEngine` | `gpu/drc.rs` (feature-gated) | Delegates to CPU below `gpu_threshold` segments; GPU compute shaders not yet active |

`DrcConfig` controls PathFinder integration:
- `start_iteration: u32` — skip DRC in early noisy iterations (default: 3)
- `violation_penalty: f64` — history cost increment per violation (default: 10.0)
- `enabled: bool`

`check_routing()` returns a `DrcReport`; the PathFinder loop (not the engine)
iterates violations and increments `history[violation.grid_cell]`. Single
update site, no double-counting.

## Data Flow

```
PcbIr (design rules)          RouteSolution (segments, vias)
         |                              |
         v                              v
    DrcPolicy::build()          DRC input geometry
    - ClearanceMatrix           (TraceSegment, RoutedVia,
    - width_constraints           board outline from PcbIr)
    - via_bounds                          |
    - diff_pair, matched_length           |
         |                               |
         +---------------+---------------+
                         |
                         v
            DrcEngine::check_routing()  <-- per PathFinder iteration
                         |
                         +-- violations[] --> PathFinder history_costs[]
                         +-- violation_count --> convergence metric

            DrcEngine::check_full()     <-- after convergence
                         |
                         v
                    DrcReport
                    - violations: Vec<DrcViolation>
                    - count_by_rule() -> BTreeMap<RuleKind, usize>
                    - render_summary() / render_verbose() / to_json()
                    - to_violation_records() -> Vec<DrcViolationRecord>
                                               (stored in RouteSolution)
```

`DrcViolationRecord` is defined in `autopcb-routes` (serializable). `DrcReport`
stays in `autopcb-router`. The CLI and other consumers load violation summaries
from `.routes` files without depending on this crate.

## Rule Coverage

| Rule | Module | Violation Kinds | CPU | GPU |
|------|--------|-----------------|-----|-----|
| Clearance | `clearance.rs` | `ClearanceViolation`, `BoardOutlineClearance` | Yes | No (planned) |
| Short circuit | `shorts.rs` | `ShortCircuit` | Yes | No (planned) |
| Width | `width.rs` | `WidthBelowMinimum`, `WidthAboveMaximum` | Yes | No |
| Via | `via.rs` | `HoleSizeBelowMinimum`, `HoleSizeAboveMaximum`, `AnnularRingBelowMinimum`, `MaximumViaCountExceeded`, `HoleToHoleClearance` | Yes | No |
| Geometry | `geometry.rs` | `AcuteAngle` | Yes | No |
| Connectivity | `connectivity.rs` | `BrokenNet` | Yes | No |
| Length | `length.rs` | `MatchedLengthOutOfTolerance`, `NetLengthBelowMinimum`, `NetLengthAboveMaximum` | Yes | No |
| Diff pair | `diff_pair.rs` | `DiffPairSkew` | Yes | No |
| Board outline | `board.rs` | `BoardOutlineClearance` | Yes | No |
| Manufacturing | `manufacturing.rs` | _(placeholder — returns empty)_ | — | — |
| Topology | `topology.rs` | _(placeholder — returns empty)_ | — | — |

`clearance.rs` covers: segment-to-segment (same layer, different net),
segment-to-via, via-to-via, and segment-to-board-edge (via
`segment_to_polyline_distance`). `board.rs` checks the rectangular bounding box
of all segment endpoints; `clearance.rs` checks actual polyline distance to the
board outline polygon when `ir.board.outline` is non-empty.

`check_routing()` runs clearance and shorts checks. `check_full()` runs all
implemented modules sequentially and collects their violations into a single
`DrcReport`.

### Geometry notes

- **Clearance**: actual gap = centerline distance minus half-widths of both
  objects. A gap exactly equal to the required clearance is not a violation.
- **Short circuit**: copper bodies overlap when centerline distance < sum of
  half-widths. `actual_mm` is negative when bodies overlap.
- **Acute angle**: interior angle between the reversed incoming direction and the
  outgoing direction at a segment junction. Threshold is 45°. `actual_mm` stores
  the angle in degrees (not mm).
- **Matched length**: checks global spread (max − min) across all routed nets.
  Per-net-group scoping is not yet supported; matched-length check applies globally across all routed nets.
- **Diff pair skew**: computes `|len_pos − len_neg|` and checks against
  `policy.diff_pair.max_uncoupled_length_mm`. Each pair is visited once
  (canonical min-raw-index ordering via `BTreeSet`).

## DrcPolicy

`DrcPolicy::build(ir)` consumes `PcbIr::rules`, sorts by priority (lower
number = higher priority), and populates:

```
DrcPolicy {
    clearance_matrix: ClearanceMatrix,              // flat Vec<f64>, BTreeMap index
    clearance_scoped: Vec<(IrRuleScopePair, f64)>,  // scoped clearance rules, priority order
    width_constraints: Vec<(IrRuleScope, DrcWidthBounds)>,  // scoped, priority descending
    via_bounds_scoped: Vec<(IrRuleScope, DrcViaBounds)>,    // scoped, priority descending
    via_bounds: DrcViaBounds,                       // kept for direct test mutation
    board_outline_clearance_mm: f64,                // default 0.5 mm
    component_clearance_mm: f64,                    // default 0.25 mm
    matched_length: Option<MatchedLengthConstraint>,
    diff_pair: Option<DiffPairConstraint>,
    length_constraints: BTreeMap<Option<String>, LengthConstraint>,
    solder_mask_expansion_mm: f64,
    paste_mask_expansion_mm: f64,
}
```

Diagram shows scoped-resolution fields only; see `policy.rs` for the complete struct definition including manufacturing, creepage, and angle constraints.

### Scoped rule resolution

Width and via rules carry an `IrRuleScope` from the IR compiler. `DrcPolicy`
resolves rules by cascade priority: `NetClassAndLayer` > `NetClass` > `Layer`
> `All`. `check_widths()` and `check_vias()` pass net class and layer to
`width_bounds()` / `via_bounds_scoped_lookup()`, which scan the priority-sorted
vec and return the first matching entry.

Priority is implemented via explicit `match` arms in `scope_priority()` and
`scope_matches()` — no `Ord` derivation on `IrRuleScope`. This keeps the
cascade visible and auditable without relying on variant declaration order.

`ClearanceMatrix` is a flat `Vec<f64>` indexed by `class_a * size + class_b`
where class indices come from a `BTreeMap<String, usize>`. The matrix expands
transparently as IR net-class data becomes available.

### Test helpers (`test_helpers.rs`)

`test_helpers::empty_ir()` constructs a minimal `PcbIr` with a two-layer
copper stack for use in DRC unit tests. Gated behind `#[cfg(test)]`.

## Invariants

- **Determinism**: same `PcbIr + RouteSolution` → identical `DrcReport`. All
  iteration uses `BTreeMap`/`BTreeSet`; no `HashMap` or random-order collections
  in violation-producing code paths.
- **CPU/GPU equivalence**: `GpuDrcEngine` must produce the same violation set as
  `CpuDrcEngine` for the same inputs. The `gpu-tests` feature gate enables a
  comparison test.
- **No false-negatives on clearance**: routing-time DRC (shorts only) may miss
  clearance violations, but `check_full()` must catch all clearance violations
  present in the solution.
- **Rule priority**: lower priority number wins. `DrcPolicy::build()` sorts
  rules before inserting into `BTreeMap` with `entry().or_insert()`, so the
  highest-priority rule wins on conflict.
- **No history mutation in engine**: `check_routing()` and `check_full()` do not
  take or modify history. PathFinder owns the history update loop.

## Solverang Integration (M10-M11)

Post-route repair pipeline (not yet active; `repair.rs` scaffolding only):

```
DrcReport (violations)
    |
    v
repair::repair_violations()     [feature = "solverang"]
    For each clearance violation cluster:
    - Extract nearby trace vertices as solvable (x, y) params
    - Pin pad/via endpoints (fixed)
    - Build ClearanceConstraint: dist²(A, B) - gap² >= 0
      (squared-distance avoids 1/dist singularity at contact)
    - LM solve via solverang ConstraintSystem
    - Write back adjusted TraceSegment endpoints
    |
    v
DRC re-check to verify 0 violations
    |
    v
rubber_band::rubber_band_solverang()    [optimize/rubber_band.rs]
    Per-net: minimize trace length subject to all clearance constraints
```

`repair_violations()` returns `RepairResult { repaired_count, remaining_violations }`.
If the solver diverges or a violation is geometrically infeasible, the
violation is preserved in `remaining_violations` and the route is not modified.

Without `feature = "solverang"`, `repair_violations()` is a no-op that returns
all violations as remaining.
