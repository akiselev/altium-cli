# Implementation Plan: Placement, Routing & DRC

Unified roadmap for the entire Solverang integration, from foundational infrastructure
through placement, autorouting, and co-optimization. Each phase builds on the previous
one — the dependency chain is strict.

Status key:
- `[x]` implemented in the current codebase
- `[ ]` planned / not implemented yet

## Dependency Graph

```
Phase 0: IR Crate + Viewer          ← FOUNDATION (everything depends on this)
    │
    ├── Phase 1: Placement MVP      ← first algorithm work
    │       │
    │       ├── Phase 2: SA Detailed Placement
    │       │       │
    │       │       └── Phase 3: Full Placement Pipeline
    │       │
    │       └── Phase 4: Single-Net Router MVP
    │               │
    │               ├── Phase 5: Multi-Net PathFinder
    │               │       │
    │               │       └── Phase 6: Placement-Router Co-Optimization
    │               │
    │               └── Phase 7: Trace Optimization
    │
    ├── Phase 8: DRC Engine
    │
    ├── Phase 9: Differential Pairs & Buses
    │
    └── Phase 10: GPU Acceleration (future)
```

---

## Phase 0: Foundation — IR Crate + egui Viewer + CLI Inspect

**Goal:** Build the shared data layer that all downstream consumers depend on, plus
a visual feedback loop for development.

**Why this is first:** Every algorithm (solver, router, DRC) consumes the IR. Without
it, nothing else can start. The viewer provides instant visual regression testing —
if extraction is wrong, you *see* it. Building them together means every new IR type
gets rendered immediately.

### 0a: `autopcb-ir` Crate Scaffold

- [x] Create `crates/autopcb-ir/` with `Cargo.toml`
- [x] Add to workspace members, depends on `altium-format`
- [x] Typed handles: `ComponentId`, `NetId`, `PadId`, `RuleId`, `LayerId`
- [x] `IdMap<K, V>` container (typed handle → value lookup)
- [x] Coordinate conversion: `Coord` → `f64` mm at extraction boundary

### 0b: PCB IR Core Types

- [x] `PcbIr` top-level struct (components, nets, outline, rules, layer stack)
- [x] `BoardGeometry` — tessellated outline/cutouts (mm), bounds AABB, keepouts
- [x] `IrComponent` — designator, footprint name, position (mm), rotation, side,
      local/world bounding box, pads
- [x] `IrComponentPad` — pad ID, name, local/world position, net, shape,
      through-hole flag, hole size
- [x] `IrNet` — name, connected pins (`Vec<IrNetPin { component, pad, position }>`),
      component count
- [x] `IrDesignRule` — name, kind, priority, enabled, typed parameters (partial coverage)
- [x] `LayerStack` — ordered copper layers (name/top/bottom flags, count)
- [x] `PcbIr::extract(doc: &PcbDocBoard) -> Result<PcbIr>`
- [ ] Unit tests against fixture PcbDoc files (108 available in `data/pcbdoc/`)

### 0c: egui Viewer MVP

- [x] New binary crate `crates/autopcb-viewer/` (standalone viewer)
- [x] Depends on `eframe = "0.33"`, `autopcb-ir`
- [x] `egui::Scene` container with scroll-to-zoom, drag-to-pan
- [x] Render board outline (polygon)
- [x] Render components (bounding box rectangles, designator labels)
- [x] Render pads (small rectangles/circles at world positions)
- [x] Hover tooltip: component designator, footprint, position, pad info
- [x] Side panel: component/net list and display toggles

### 0d: CLI `inspect` Commands

- [x] `altium inspect <file> components` — table of designator, footprint, position (mm)
- [x] `altium inspect <file> nets` — netlist with pin counts
- [x] `altium inspect <file> board-outline` — outline points + cutout summary
- [x] `altium inspect <file> rules` — design rules summary
- [x] `altium inspect <file> ir-json` — full IR as JSON (for LLM consumption)

### 0e: Viewer Enhancements (build alongside Phase 1)

- [x] Ratsnest rendering (straight lines between connected pads, per net)
- [x] Net highlighting (click net in side panel → highlight all pads/connections)
- [x] Color-coded layers (top=red, bottom=blue, inner=green/yellow)
- [x] Board keepout regions
- [x] Component selection → show all connected nets
- [x] Keyboard shortcuts: `F` = fit to board, `N` = toggle nets, `L` = toggle layers

---

## Phase 1: Placement MVP — Solverang-Only Placer

**Goal:** Place components on a board using Solverang's `ConstraintSystem` (which
automatically selects LM via `AutoSolver` for over-determined placement problems).

**Depends on:** Phase 0 (IR crate)

**Design docs:** `architecture.md`, `constraint-types.md`, `rotation-and-ratsnest.md`

### 1a: Solver Bridge

- [ ] `ir_to_solver_input()` — convert IR components → Solverang `ConstraintSystem`
- [ ] `PcbComponent` entity (impl `Entity` trait): `[x, y]` params via `system.alloc_param()`, fixed rotation/bounding box
- [ ] `PcbBoardOutline` entity: fixed AABB (all params fixed via `system.fix_param()`)
- [ ] Pattern after `Sketch2DBuilder`: ergonomic `PcbPlacementBuilder` wrapping `ConstraintSystem`

### 1b: Hard Constraints

- [ ] `BoardContainment` — 4 inequalities per component (left/right/top/bottom inside)
- [ ] `ComponentClearance` — smooth overlap guidance + exact non-overlap legalization, O(N²) pairs (spatial pruning later)
- [ ] `BoardEdgeClearance` — board containment with gap
- [ ] `FixedPosition` — 2 equalities (pin component at exact coordinates)

### 1c: Soft Objectives

- [ ] `SmoothHPWL` — log-sum-exp approximation per net
- [ ] Adaptive γ: start γ=2, increase to γ=10 during solve
- [ ] Pin position computation with fixed rotation offsets

### 1d: User Constraints (from spec)

- [ ] `EdgePlacement` — pin component to board edge with inset
- [ ] `DirectionalOrdering` — left_of, right_of, above, below
- [ ] `NearConstraint` — max distance between two components
- [ ] `RegionContainment` — component center within rectangular region

### 1e: Integration + Legalization

- [ ] Solve → `PlacementSolution` (per-component x, y)
- [ ] Snap to grid if required
- [ ] Resolve any remaining overlaps (greedy shifting)
- [ ] CLI: `altium placement solve <spec> --target <pcbdoc>`
- [ ] **Viewer: live solver iteration display** (show components moving each iteration)

---

## Phase 2: SA Detailed Placement

**Goal:** Escape local minima via discrete moves that gradient-based LM cannot make.

**Depends on:** Phase 1

**Design doc:** `placement-algorithms.md` §14-15

- [ ] Move generation: displace, swap, rotate, slide
- [ ] Cost function: HPWL + overlap + constraint penalties + net crossings
- [ ] Incremental cost evaluation (O(k) per move, not O(N²))
- [ ] Adaptive cooling schedule (T_initial auto-set, α=0.95)
- [ ] Net crossing metric (2-pin segment intersection count)
- [ ] **Viewer: SA animation** (show accepted/rejected moves, temperature bar)

---

## Phase 3: Full Placement Pipeline

**Goal:** Complete placement system with pre-processing and post-refinement.

**Depends on:** Phase 2

**Design docs:** `placement-algorithms.md` §13, `llm-constraint-generation.md`

### 3a: Pre-Processing (Phase 0 in pipeline)

- [ ] Auto-detect component clusters from netlist connectivity
- [ ] Generate initial grouping constraints
- [ ] Graph partitioning (spectral clustering via `petgraph`)

### 3b: Final Refinement (Phase 4 in pipeline)

- [ ] Fix rotations from SA, re-solve (x, y) only with tight γ
- [ ] Fine-tune within clearance envelopes

### 3c: DRC Verification (Phase 5 in pipeline)

- [ ] Evaluate all placement rules as constraint residuals
- [ ] Report violations with exact distances and locations

### 3d: LLM Integration

- [ ] `ir_to_board_summary()` for LLM consumption
- [ ] CLI: `altium placement generate <pcbdoc> --model <model>` (LLM → spec)
- [ ] CLI: `altium placement interactive <pcbdoc>` (multi-round LLM + solve loop)

### 3e: Spec Language Extensions

- [ ] Parser: `placement { ... }` blocks in `.pcbdoc-spec`
- [ ] `place`, `left_of`, `right_of`, `above`, `below`, `separate`, `group`
- [ ] `optimize { ratsnest, thermal }`
- [ ] `clearance { all, connectors, edge }` shorthand
- [ ] CLI: `altium placement plan <spec>` (preview without mutating)
- [ ] CLI: `altium placement apply <spec>` (solve + write back)

---

## Phase 4: Single-Net Router MVP

**Goal:** Route one 2-pin net through a 3D grid with obstacles.

**Depends on:** Phase 0 (IR crate, viewer)

**Design doc:** `autorouter.md` §Phase 2

- [ ] `RoutingGrid` struct with per-layer obstacle bitmap
- [ ] Obstacle extraction from `PcbIr` (pads, keepouts → blocked cells)
- [ ] 3D A* using `pathfinding` crate (x, y, layer)
- [ ] Via cost model (base + SI penalty, tunable per net class)
- [ ] Layer-direction bias (horizontal/vertical preferred direction)
- [ ] Admissible heuristic (Manhattan + min via transitions)
- [ ] CLI: `altium route <pcbdoc> --net <name>` (single net, debug output)
- [ ] **Viewer: route visualization** (colored traces per layer, via markers)

---

## Phase 5: Multi-Net PathFinder Router

**Goal:** Route all nets on a board using PathFinder negotiation.

**Depends on:** Phase 4

**Design doc:** `autorouter.md` §Phase 1, §Phase 3

### 5a: Net Decomposition

- [ ] Multi-pin nets → 2-pin subnets via MST (`petgraph::min_spanning_tree`)
- [ ] FLUTE Steiner tree decomposition (via `libreda/steiner-tree` or custom)

### 5b: PathFinder Loop

- [ ] History congestion tracking (per grid cell, linear accumulation)
- [ ] Present congestion factor (exponential growth: `pres_fac *= 1.15`)
- [ ] Cost function: `(base + history) × present_congestion`
- [ ] Full rip-up and reroute per iteration
- [ ] Partial rip-up optimization (hot set of worst-offending nets)
- [ ] Convergence detection (no oversubscribed cells)

### 5c: Net Ordering

- [ ] Priority heuristic: critical nets first, short nets early, high-fanout last
- [ ] Seeded RNG for reproducible tiebreaking

### 5d: Integration

- [ ] CLI: `altium route <pcbdoc>` (all nets, report completion rate + stats)
- [ ] **Viewer: PathFinder animation** (iteration counter, congestion heatmap overlay,
      completed/unrouted net counts)

---

## Phase 6: Placement-Router Co-Optimization

**Goal:** Feedback loop between placer and router for hard-to-route boards.

**Depends on:** Phase 3 (full placement) + Phase 5 (multi-net router)

**Design doc:** `autorouter.md` §Placement-Router Co-Optimization

### 6a: Forward Integration (Placer → Router)

- [ ] Global routing congestion grid (coarse overlay)
- [ ] Congestion capacity estimation per cell
- [ ] "Congestion oracle" for SA cost function (fast, no full routing)
- [ ] ILP layer assignment (`good_lp` with HiGHS backend)

### 6b: Backward Integration (Router → Placer)

- [ ] Persistent data structures for cheap state forking (`im` crate)
- [ ] Bottleneck identification (persistently oversubscribed cells → blocking components)
- [ ] Generate temporary placement constraints from routing deadlocks
- [ ] Micro-SA or quick Solverang pass to shift blocking components

### 6c: Co-Optimization Loop

- [ ] Outer loop: place → route → if deadlock, adjust placement → re-route
- [ ] Max outer iterations with diminishing returns detection
- [ ] CLI: `altium place-and-route <pcbdoc>` (full co-optimization)
- [ ] **Viewer: co-opt visualization** (show placement adjustments between routing passes)

---

## Phase 7: Trace Optimization

**Goal:** Clean, manufacturable traces.

**Depends on:** Phase 5 (multi-net router)

**Design doc:** `autorouter.md` §Phase 4

- [ ] Staircase elimination (consecutive bend pairs → diagonals)
- [ ] 45° conversion (right-angle → chamfered, acid trap removal)
- [ ] Rubber-banding via Solverang (trace vertices as entities, minimize length
      subject to clearance constraints from R-tree queries)
- [ ] Length matching / serpentine insertion for matched net groups
- [ ] **Viewer: before/after toggle** for optimization passes

---

## Phase 8: DRC Engine

**Goal:** Full design rule checking against routed boards.

**Depends on:** Phase 0 (IR crate)

**Design docs:** `design-rules-mapping.md`, `ir.md` §Phase 3

### 8a: IR Extensions

- [ ] Free copper geometry extraction (tracks, vias, fills, regions not owned by components)
- [ ] `IrPadDetail` — per-layer shapes, mask expansions, thermal reliefs
- [ ] Lazy-loaded `SpatialIndex` (R-tree over all copper for pairwise checks)

### 8b: Rule Evaluation

- [ ] Scope expression parser and evaluator (`InNet(...)`, `OnLayer(...)`, `All`, boolean ops)
- [ ] Phase 1 rules: Clearance (0), ComponentClearance (24), BoardOutlineClearance (63)
- [ ] Phase 2 rules: Width (2), HoleToHoleClearance (52), MinimumAnnularRing (19)
- [ ] Phase 3 rules: SolderMaskExpansion (13), PasteMaskExpansion (14), MaxMinHoleSize (42)
- [ ] Phase 4 rules: DiffPairsRouting (51), MatchedLengths (4), Length (3)

### 8c: Reporting

- [ ] CLI: `altium drc <pcbdoc>` (human-readable violation report)
- [ ] CLI: `altium drc <pcbdoc> --json` (machine-readable for LLM)
- [ ] **Viewer: DRC overlay** (highlight violations with distance annotations)

---

## Phase 9: Differential Pairs & Buses

**Goal:** High-speed design support.

**Depends on:** Phase 5 (multi-net router) + Phase 8 (DRC for validation)

**Design doc:** `autorouter.md` §Differential Pair and Bus Routing

- [ ] Coupled differential pair routing (simultaneous A* with gap offset)
- [ ] Bus routing (parallel groups, channel routing, maintain order)
- [ ] Length matching within groups
- [ ] **Viewer: differential pair overlay** (show coupling, length difference)

---

## Phase 10: GPU Acceleration (Future)

**Goal:** Handle large boards (>1000 nets, >8 layers).

**Depends on:** Phases 5-6 working on CPU first

**Design doc:** `autorouter.md` §GPU Acceleration Strategy

- [ ] `wgpu` compute shaders for obstacle/congestion bitmaps
- [ ] Parallel BFS/Dijkstra per net on GPU (OrthoRoute approach)
- [ ] `burn` for analytical placement (DREAMPlace-style tensor + autograd)
- [ ] GPU-rendered congestion heatmaps in viewer

---

## Phase Summary

| Phase | Name | Key Deliverable | Depends On |
|-------|------|----------------|------------|
| **0** | Foundation | IR crate + egui viewer + CLI inspect | PcbDoc parsing (done) |
| **1** | Placement MVP | Solverang ConstraintSystem placer (AutoSolver) with hard/soft constraints | 0 |
| **2** | SA Placement | Simulated annealing for discrete moves | 1 |
| **3** | Full Placement | Pre-processing, refinement, LLM, spec language | 2 |
| **4** | Single-Net Router | 3D A* through grid with obstacles | 0 |
| **5** | Multi-Net Router | PathFinder negotiation, FLUTE decomposition | 4 |
| **6** | Co-Optimization | Placement-router feedback loop | 3 + 5 |
| **7** | Trace Optimization | Smoothing, 45°, rubber-banding, length match | 5 |
| **8** | DRC Engine | Full design rule checking | 0 |
| **9** | Diff Pairs & Buses | High-speed routing support | 5 + 8 |
| **10** | GPU Acceleration | wgpu compute, burn tensors | 5-6 on CPU |

### Parallelism Opportunities

After Phase 0 is complete, **Phases 1 and 4 can proceed in parallel** — the placer
and router both consume the IR but don't depend on each other until Phase 6. Similarly,
**Phase 8 (DRC) can start any time after Phase 0** since it only needs the IR + spatial
index.

```
Phase 0 ─┬─ Phase 1 → Phase 2 → Phase 3 ─┐
          │                                 ├── Phase 6 → ...
          ├─ Phase 4 → Phase 5 ────────────┘
          │       └── Phase 7
          └─ Phase 8 → Phase 9
```

---

## Viewer Feature Roadmap (cross-cutting)

The viewer grows alongside the algorithm phases:

| Phase | Viewer Addition |
|-------|----------------|
| 0c | Board outline, components, pads, hover tooltips, side panel |
| 0e | Ratsnest, net highlighting, layer colors, keepouts |
| 1e | Live solver iteration display (components moving) |
| 2 | SA animation (accepted/rejected moves, temperature) |
| 4 | Routed traces per layer, via markers |
| 5d | PathFinder iteration counter, congestion heatmap |
| 6c | Co-opt adjustments between routing passes |
| 7 | Before/after optimization toggle |
| 8c | DRC violation overlay with distance annotations |
| 9 | Differential pair coupling and length display |

---

## Technology Stack

### Required Crates (Phase 0)

| Crate | Version | Purpose |
|-------|---------|---------|
| `eframe` | 0.33 | GUI viewer (egui + wgpu + winit) |

### Required Crates (Phase 1+)

| Crate | Version | Purpose |
|-------|---------|---------|
| `solverang` | — | Nonlinear solver (LM/NR/Auto), ConstraintSystem, `#[auto_jacobian]` macro |
| `petgraph` | 0.8 | Netlist graph, MST, connectivity |
| `pathfinding` | 4.14 | A*, Dijkstra inner-loop routing |
| `rstar` | 0.12 | R-tree spatial index |
| `geo` | 0.32 | Polygon operations, distance |
| `good_lp` | 1.15 | ILP layer assignment (HiGHS) |
| `bitvec` | 1.x | Memory-efficient obstacle bitmaps |
| `rayon` | 1.x | CPU parallelism |

### Optional Crates (Later Phases)

| Crate | Version | Purpose |
|-------|---------|---------|
| `spade` | 2.15 | Delaunay triangulation (shape-based routing) |
| `im` | 15.1 | Persistent data structures (state forking) |
| `wgpu` | 28.0 | GPU compute shaders |
| `burn` | 0.21 | Tensor ops + autograd (DREAMPlace-style) |

**Solverang Feature Flags** (for the `solverang` dependency):
| Feature | Purpose | Needed For |
|---------|---------|------------|
| `sparse` | Sparse matrix ops via `faer` | Boards with >100 components |
| `parallel` | Parallel solving via `rayon` | Multi-start, cluster decomposition |
| `macros` | `#[auto_jacobian]` proc macro | Simpler constraint implementations |
| `jit` | Cranelift JIT for hot constraints | Optional performance (Phase 10) |

**Note:** The `geometry` feature has been removed from solverang. PCB-specific
geometry (bounding boxes, clearance zones, AABB tests) is computed in constraint
residual functions, not in solverang's core. Solverang's domain-specific plugins
(sketch2d, sketch3d, assembly) provide examples of this pattern.
