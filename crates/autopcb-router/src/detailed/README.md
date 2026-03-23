# Pad Breakout System

`fanout.rs` — three-tier pipeline that pre-routes short escape traces from dense SMD
pads before the PathFinder negotiation loop starts. The output (`BreakoutPlan`) is
applied to obstacle maps so stub endpoints become the effective A* access points.

## Overview

```
plan_breakouts()
  |
  +-- Tier 1: plan_stubs()              [any layer count]
  |   Same-layer stubs with trace width necking.
  |   Handles: dense SMD pads on any board.
  |
  +-- Tier 2: plan_perimeter_escapes()  [any layer count]
  |   Perpendicular escapes outward from component edge.
  |   Handles: peripheral packages (QFP, TQFP, SOP).
  |   Input: pads not already handled by Tier 1.
  |
  +-- Tier 3: plan_via_escapes()        [>= 3 layers only]
      Via-based escape to an inner layer.
      Handles: BGA interior pads and remaining dense pads.
      Input: pads not handled by Tier 1 or Tier 2.
      Naturally produces nothing on <= 2-layer boards.
```

Each tier processes only the pads that prior tiers left unhandled. Tiers 1 and 2 run
on any board; Tier 3 is a no-op when there are no inner layers.

## Data Flow

```
PcbIr (components, pads, shapes)
  |
  +-- classify_component()
  |     Input:  IrComponent.pads (world positions)
  |     Output: ComponentKind { Peripheral, AreaArray, Other }
  |
  +-- plan_stubs()
  |     Input:  PcbIr, GridConfig, ObstacleMap[], RoutingPolicy, EscapeConfig
  |     Output: Vec<BreakoutRoute> (tier = Stub, via_cell = None)
  |
  +-- plan_perimeter_escapes()
  |     Input:  above + existing Tier 1 routes (to skip handled pads)
  |     Output: Vec<BreakoutRoute> (tier = PerimeterEscape, via_cell = None)
  |
  +-- plan_via_escapes()
  |     Input:  above + existing Tier 1+2 routes (to skip handled pads)
  |     Output: Vec<BreakoutRoute> (tier = ViaEscape, via_cell = Some(...))
  |
  +-- apply_breakouts()
        Marks trace_cells and via_cell as blocked in obstacle maps.
        Stub endpoints become access points for astar.rs.
```

## Tier Activation

### Tier 1 — Same-Layer Stubs

Activates for any SMD pad whose 8-neighbour free-cell count is below
`EscapeConfig.min_access_threshold`. Works on single-layer and 2-layer boards.

Algorithm:
1. Tries all 8 directions (4 cardinal + 4 diagonal, `DIRECTIONS_8`).
2. Walks up to `max_escape_mm / resolution_mm` cells.
3. Accepts the first direction where an unblocked cell is reached after
   `min_escape_mm` distance.
4. Applies neckdown width near the pad; transitions to preferred width beyond
   the neckdown zone (`width_sequence` field).

### Tier 2 — Perimeter Escapes

Activates for components classified as `Peripheral`. Skips pads already handled
by Tier 1 or pads with sufficient free neighbors.

Algorithm:
1. Assigns each pad to the nearest bounding box edge (`assign_edge`).
2. Sorts pads per edge by position along the edge axis.
3. Applies outer-first ordering (alternates from lo/hi ends inward) matching
   FreeRouting's fanout order — outer pads escape first, freeing channels for
   inner pads.
4. Escapes perpendicular to the assigned edge, outward from component center
   (`edge_escape_direction`).
5. Staggers adjacent pad stub lengths (1 or 3 extra cells, `stagger_offset`) to
   prevent adjacent stubs from colliding.

### Tier 3 — Via Escapes

Activates only when `layer_count >= 3`. Skips pads handled by Tiers 1 and 2.
Preserves the original fanout algorithm: walks in the primary escape direction
(component center → pad, quantized to 4 cardinal directions) and places a via
to an inner layer chosen by round-robin per component.

## Component Classification

`classify_component(comp: &IrComponent) -> ComponentKind`

Computes pad bounding box, average nearest-neighbor pitch, and counts pads that
sit strictly inside the bounding box (more than half a pitch from every edge).

| Result       | Condition                                                      | Typical package |
|--------------|----------------------------------------------------------------|-----------------|
| `AreaArray`  | `interior_count > 0` AND `pad_count > 8`                      | BGA             |
| `Peripheral` | `interior_count == 0` AND `perimeter_ratio > 0.8` AND `pad_count > 4` | QFP, TQFP, SOP  |
| `Other`      | Everything else                                                | SOT, discrete   |

`AreaArray` is detected by `interior_count > 0` rather than `perimeter_ratio < 0.3`
because the ratio threshold is geometrically impossible for small grids: a 4x4 BGA
has 12/16 = 75% perimeter pads, far above 0.3. Counting actual interior pads
directly expresses the semantic intent and works for all grid sizes.

## Neckdown Formulas

These formulas come from FreeRouting, validated across thousands of production boards.

Neckdown is only applied when `EscapeConfig.neckdown_enabled` is `true` (the default).
When disabled, `compute_neckdown_width` returns `preferred_width` directly and all cells
in `width_sequence` use the full trace width.

**Neckdown width** (narrowest trace near pad):

```
neckdown_width = max(pad.min_dim / 2.0, policy.trace_width(net).min)
if config.neckdown_min_width_mm > 0.0:
    neckdown_width = max(neckdown_width, config.neckdown_min_width_mm)
```

`pad.min_dim` is the smaller of the pad's X and Y dimensions. Dividing by 2 matches
Altium's percentage-based approach without per-net stackup analysis. The result is
floored first by the policy minimum (to satisfy DRC), then optionally by
`neckdown_min_width_mm` (a hard override floor set in `EscapeConfig`).

**Neckdown zone distance** (cells where narrow width applies):

```
neckdown_distance_cells = ceil(2.0 * (pad.max_dim / 2.0 + clearance_mm) / resolution_mm)
```

Beyond this distance, `width_sequence` assigns `preferred_width`. The zone boundary
creates a clean geometric transition that avoids acid traps from abrupt width changes.

## Invariants

- A pad receives at most one `BreakoutRoute` across all three tiers (first-tier-wins).
  Later tiers check `handled_cells` (a set of `pad_cell` values from prior tier output)
  and skip any pad already present.

- Breakout routes never overlap with pre-existing obstacles. Each cell is checked
  against `ObstacleMap.is_blocked()` before acceptance; a blocked cell aborts that
  direction.

- `neckdown_width_mm` is always `>= policy.trace_width(net).min`. The formula
  `max(pad.min_dim / 2.0, trace_min)` guarantees this.

- Stubs are fixed copper in `obstacle_maps` after `apply_breakouts()`. PathFinder
  (the negotiation loop in `astar.rs`) never rips them up — they are not in
  `solution_paths`. This matches FreeRouting's fanout-protection pattern.

- All three tiers complete before access point computation in workspace build. The
  stub endpoint (`stub_endpoint` field) is what `workspace.rs` uses as the A*
  start/goal cell, not the pad center.

## Neckdown in Trace Output

`route_subnet_to_traces` in `grid.rs` accepts a `NeckdownMap` (built via
`build_neckdown_map()` from the `BreakoutPlan`). When a path cell's width
differs from the current run, the run is split — producing separate
`TraceSegment`s with different `width_mm` values at neckdown boundaries.
