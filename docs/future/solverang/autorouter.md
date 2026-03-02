# PCB Autorouter Design

Design notes for a state-of-the-art PCB autorouter integrated with the Solverang
placement pipeline. The autorouter is a **separate algorithmic engine** from the placer
— placement is continuous optimization (Levenberg-Marquardt), routing is discrete
graph-search (A*, PathFinder negotiation) — but both engines communicate via feedback
loops to achieve placement-routing co-optimization.

## Table of Contents

1. [Why Routing is Different from Placement](#why-routing-is-different-from-placement)
2. [Algorithm Pipeline Overview](#algorithm-pipeline-overview)
3. [Phase 1: Global Routing](#phase-1-global-routing)
4. [Phase 2: Detailed Routing](#phase-2-detailed-routing)
5. [Phase 3: Rip-Up and Reroute (PathFinder)](#phase-3-rip-up-and-reroute-pathfinder)
6. [Phase 4: Trace Optimization and Smoothing](#phase-4-trace-optimization-and-smoothing)
7. [Placement-Router Co-Optimization](#placement-router-co-optimization)
8. [Board Representation and Data Structures](#board-representation-and-data-structures)
9. [Design Rule Integration](#design-rule-integration)
10. [Differential Pair and Bus Routing](#differential-pair-and-bus-routing)
11. [GPU Acceleration Strategy](#gpu-acceleration-strategy)
12. [Rust Crate Ecosystem](#rust-crate-ecosystem)
13. [IR Extensions for Routing](#ir-extensions-for-routing)
14. [Implementation Roadmap](#implementation-roadmap)

---

## Why Routing is Different from Placement

| Property | Placement (Solverang) | Routing (Autorouter) |
|----------|----------------------|---------------------|
| **Problem type** | Continuous optimization | Discrete graph search |
| **Variables** | Component (x, y, θ) — floats | Path through grid cells — integers |
| **Algorithm** | ConstraintSystem (AutoSolver→LM), SA | A*, PathFinder, Steiner trees |
| **Differentiable?** | Yes (smooth HPWL, clearance) | No (path exists or doesn't) |
| **Output** | Component positions | Copper traces, vias |
| **Constraint checking** | Residual norm ≈ 0 | DRC pass/fail per segment |
| **Failure mode** | Poor HPWL, overlaps | Unrouted nets, DRC violations |
| **GPU affinity** | High (tensor math, FFT density) | Low (irregular graph traversal) |

The placer optimizes **where** components go. The router determines **how** copper
connects their pins. These are coupled problems — bad placement creates unroutable
boards — but they require fundamentally different algorithmic engines.

---

## Algorithm Pipeline Overview

```
┌─────────────────────────────────────────────────────────────────┐
│ Input: Placed PcbDoc (from Solverang pipeline or manual)        │
│        + Design Rules + Net Classes + Layer Stack               │
└──────────────────────────┬──────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────────┐
│ Phase 0: Pre-Processing                                         │
│   • Extract routing IR from PcbDoc (pads, obstacles, rules)     │
│   • Build spatial index (R-tree) of all fixed copper            │
│   • Decompose multi-pin nets → Steiner tree topologies (FLUTE)  │
│   • Assign net routing order (criticality heuristic)            │
│   • Build 3D routing grid (x, y, layer)                         │
└──────────────────────────┬──────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────────┐
│ Phase 1: Global Routing                                         │
│   • Coarse grid overlay → routing regions/channels              │
│   • Assign each 2-pin subnet to routing regions                 │
│   • Respect layer capacity limits                               │
│   • Output: per-subnet region path (no exact coordinates)       │
│   ←── Forward feedback: congestion map → placer SA cost         │
└──────────────────────────┬──────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────────┐
│ Phase 2: Detailed Routing                                       │
│   • 3D A* pathfinding per 2-pin subnet                          │
│   • Fine grid with via costs, layer-bias, design rules          │
│   • Routes within regions assigned by global router             │
└──────────────────────────┬──────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────────┐
│ Phase 3: Rip-Up and Reroute (PathFinder)                        │
│   • Negotiation-based conflict resolution                       │
│   • Allow temporary resource sharing with escalating penalties   │
│   • Iterate until no oversubscribed routing resources            │
│   ←── Backward feedback: deadlock → micro-placement adjust      │
└──────────────────────────┬──────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────────┐
│ Phase 4: Trace Optimization                                     │
│   • Remove unnecessary bends (staircase elimination)            │
│   • Pull traces tight (rubber-banding via Solverang)            │
│   • Convert 90° to 45° angles where allowed                    │
│   • Length matching for differential pairs                       │
└──────────────────────────┬──────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────────┐
│ Phase 5: DRC Verification                                       │
│   • Full design rule check on routed board                      │
│   • Report any remaining violations                             │
│   • Output: routed PcbDoc (or ECO changelist)                   │
└─────────────────────────────────────────────────────────────────┘
```

---

## Phase 1: Global Routing

Global routing divides the board into coarse regions and decides which regions each
net passes through, without determining exact trace coordinates. This prevents
congestion bottlenecks before expensive detailed routing begins.

### Net Decomposition: Steiner Trees (FLUTE)

A net connecting N pins (N ≥ 3) must be decomposed into 2-pin subnets before routing.
The optimal decomposition is a **Rectilinear Steiner Minimum Tree (RSMT)** — a tree
connecting all pins with minimum total Manhattan wirelength, potentially introducing
Steiner (branch) points.

**FLUTE algorithm:**
- For nets with degree ≤ 9: exact RSMT via precomputed lookup tables (O(1))
- For nets with degree > 9: recursive decomposition into degree-9 subproblems
- Near-optimal for nets up to degree ~100 (within ~1% of optimal)
- Tables: `POWV9.dat` (Potentially Optimal Wirelength Vector) and `POST9.dat`
  (Potentially Optimal Steiner Tree)

**Alternative for simple cases:** Kruskal's MST (Minimum Spanning Tree) is simpler
but produces slightly longer trees. For 2-pin and 3-pin nets (the vast majority
on typical PCBs), MST and RSMT are identical.

**Rust implementation:** The [LibrEDA steiner-tree](https://codeberg.org/libreda/steiner-tree)
crate provides a Rust FLUTE implementation. Alternatively, `petgraph`'s
`min_spanning_tree()` gives MST as a quick starting point.

### Congestion Grid

Overlay a coarse grid on the board (cell size = 5-10× trace pitch). Each cell has:

```rust
struct GlobalRoutingCell {
    /// Maximum number of traces that can pass through this cell per layer
    capacity: Vec<u16>,  // indexed by layer
    /// Current demand: how many nets are assigned through this cell
    demand: Vec<u16>,    // indexed by layer
    /// Congestion ratio = demand / capacity (> 1.0 means oversubscribed)
    congestion: Vec<f64>,
}
```

**Capacity estimation:**
```
cell_capacity = (cell_width - obstacle_width) / (trace_width + trace_clearance)
```

Cells containing pads, keepouts, or board edge have reduced capacity.

### Global Routing Algorithm

Use **negotiation-based A*** on the coarse grid (a simplified PathFinder):

1. Build a graph where nodes = grid cells, edges = adjacent cells
2. Edge cost = `base_cost + congestion_penalty`
3. Route all 2-pin subnets through this graph using A*
4. After routing all nets: update demand counts per cell
5. Increase penalty for oversubscribed cells
6. Rip up and reroute nets through congested cells
7. Iterate until no cell exceeds capacity (or max iterations)

**Layer assignment** is part of global routing: decide which layer each trace segment
uses. This can be formulated as an ILP:

```
minimize: Σ via_transitions(net_i)
subject to: demand(cell_j, layer_k) ≤ capacity(cell_j, layer_k) ∀ j, k
```

Solvable with `good_lp` (HiGHS backend for performance, or `microlp` for zero-dep
testing).

### Net Ordering

Route order significantly affects quality. Heuristic priority:
1. **Critical nets first**: high-speed, differential pairs, length-matched
2. **Short nets first**: they have fewer routing options, so route them early
3. **High-fanout nets last**: power/ground nets have many alternatives
4. **Random tiebreaker** with seeded RNG for reproducibility

---

## Phase 2: Detailed Routing

Detailed routing finds exact copper trace coordinates within the regions assigned
by the global router.

### 3D Routing Grid

The board is discretized into a 3D grid `(x, y, layer)`:

```rust
struct RoutingGrid {
    /// Grid resolution (typically 0.1mm or trace_width/2)
    resolution_mm: f64,
    /// Board dimensions in grid cells
    width: u32,
    height: u32,
    /// Number of copper layers
    layers: u16,
    /// Per-cell obstacle map: true = blocked
    /// Indexed as [layer][y][x] for cache-friendly row scans
    obstacles: Vec<BitVec>,
    /// Per-cell cost modifiers (congestion history, preferred direction)
    costs: Vec<Vec<f32>>,
}
```

**Grid resolution tradeoff:**
- Coarser (0.5mm): less memory, faster search, may miss tight channels
- Finer (0.05mm): more accurate, quadratically more memory
- Recommendation: adaptive — coarse for global routing, fine for detailed routing
  in congested areas

### 3D A* Pathfinding

Each node in the search space is `(x, y, layer)`. Successors include:

```rust
fn successors(&self, node: GridNode) -> Vec<(GridNode, f64)> {
    let mut succs = Vec::new();
    let (x, y, layer) = (node.x, node.y, node.layer);

    // Same-layer movement (4-connected or 8-connected for 45°)
    for (dx, dy) in [(1,0), (-1,0), (0,1), (0,-1)] {
        let nx = x.wrapping_add_signed(dx);
        let ny = y.wrapping_add_signed(dy);
        if self.in_bounds(nx, ny) && !self.is_blocked(nx, ny, layer) {
            let cost = self.base_cost(nx, ny, layer)
                     + self.congestion_cost(nx, ny, layer)
                     + self.direction_penalty(layer, dx, dy);
            succs.push((GridNode::new(nx, ny, layer), cost));
        }
    }

    // Via transitions to adjacent layers
    for dl in [-1i16, 1] {
        let nl = layer.checked_add_signed(dl);
        if let Some(nl) = nl {
            if nl < self.layers && !self.is_blocked(x, y, nl) {
                let cost = self.via_cost(layer, nl);
                succs.push((GridNode::new(x, y, nl), cost));
            }
        }
    }

    succs
}
```

**Heuristic function** (must be admissible for optimal A*):

```rust
fn heuristic(node: GridNode, goal: GridNode) -> f64 {
    let manhattan = ((node.x as i64 - goal.x as i64).abs()
                   + (node.y as i64 - goal.y as i64).abs()) as f64
                   * self.resolution_mm;
    let layer_changes = (node.layer as i64 - goal.layer as i64).unsigned_abs() as f64;
    manhattan + layer_changes * MIN_VIA_COST
}
```

**Layer-direction bias:** Many PCB designs use preferred routing directions per layer
(e.g., horizontal on even layers, vertical on odd). Penalize movement against the
preferred direction:

```rust
fn direction_penalty(&self, layer: u16, dx: i32, dy: i32) -> f64 {
    let preferred = self.layer_stack.preferred_direction(layer);
    match preferred {
        Direction::Horizontal if dy != 0 => DIRECTION_PENALTY,
        Direction::Vertical if dx != 0 => DIRECTION_PENALTY,
        Direction::Any => 0.0,
        _ => 0.0,
    }
}
```

### Via Cost Model

Via cost must be tunable per net class:

```rust
struct ViaCostModel {
    /// Base manufacturing cost (fewer vias = cheaper board)
    base_cost: f64,              // default: 10.0 (= 10 grid cells of wire)
    /// Signal integrity penalty (impedance discontinuity)
    si_penalty: f64,             // default: 5.0 for signal nets, 0.0 for power
    /// Per-net-class overrides
    net_class_overrides: HashMap<String, f64>,
}
```

**Typical PCB via costs (in grid-cell equivalents):**
- Power/ground nets: 2-5 (vias are cheap, wide planes available)
- General signal nets: 10-20 (moderate penalty)
- High-speed signals: 50-100 (avoid vias for impedance)
- Differential pairs: 100-200 (each via is a signal integrity disaster)

### Shape-Based Routing (Alternative)

For surface layers and BGA fanout, **shape-based routing** may outperform grid-based:

Instead of a grid, represent all obstacles as polygonal shapes and find paths through
free-space regions. This is how Freerouting works:

1. Build a `ShapeSearchTree` (spatial index) of all pads, existing traces, keepouts
2. Compute "expansion rooms" — convex free-space polygons between obstacles
3. Route by expanding through these rooms using a modified maze algorithm
4. Support any-angle routing (not just Manhattan)

**Advantages:** no resolution limit, better space utilization, natural BGA fanout
**Disadvantages:** more complex algorithms, harder to parallelize

**Recommendation:** Start with grid-based (simpler, proven, GPU-friendly). Add
shape-based routing later for surface layers as an optimization.

**Rust crates:** `spade` for Delaunay triangulation/Voronoi diagrams, `geo` for
polygon operations, `rstar` for spatial indexing.

---

## Phase 3: Rip-Up and Reroute (PathFinder)

No router achieves 100% completion on the first pass. Nets inevitably block each
other. PathFinder (McMurchie & Ebeling 1995) resolves conflicts through negotiation.

### Algorithm

PathFinder allows all nets to temporarily share routing resources, then iteratively
increases the cost of shared resources until each net naturally finds a non-conflicting
path.

**Cost function per routing resource (grid cell or edge):**

```
C(n) = (b_n + h_n) × p_n
```

Where:
- `b_n` = **base cost** (intrinsic: wire length, via penalty, direction bias)
- `h_n` = **history congestion** — accumulated over all iterations; increases
  linearly each iteration if cell `n` is oversubscribed
- `p_n` = **present congestion** — based on current-iteration occupancy:
  `p_n = 1 + max(0, occupancy_n - capacity_n) × pres_fac`

**Iteration loop:**

```rust
fn pathfinder_route(
    grid: &mut RoutingGrid,
    nets: &[Net],
    max_iterations: u32,
) -> RoutingResult {
    let mut history: Vec<f64> = vec![0.0; grid.total_cells()];
    let mut pres_fac = 1.0;

    for iteration in 0..max_iterations {
        // Rip up all nets (or just the conflicting subset)
        grid.rip_up_all();

        // Route each net using A* with PathFinder costs
        for net in nets.iter().sorted_by_key(|n| n.priority()) {
            for subnet in &net.steiner_subnets {
                let path = astar_3d(
                    grid,
                    subnet.source,
                    subnet.target,
                    |cell| base_cost(cell) + history[cell.id()] * present_congestion(cell, pres_fac),
                );
                grid.commit_path(net.id, &path);
            }
        }

        // Check for convergence (no oversubscribed resources)
        let violations = grid.count_oversubscribed();
        if violations == 0 {
            return RoutingResult::Success;
        }

        // Update history: cells that are oversubscribed accumulate penalty
        for cell_id in 0..grid.total_cells() {
            if grid.occupancy(cell_id) > grid.capacity(cell_id) {
                history[cell_id] += HISTORY_INCREMENT;
            }
        }

        // Increase present congestion factor (exponential growth)
        pres_fac *= PRES_FAC_MULTIPLIER;  // typically 1.15
        pres_fac = pres_fac.min(PRES_FAC_CAP);  // cap at ~8.0
    }

    RoutingResult::Incomplete { unrouted: grid.oversubscribed_nets() }
}
```

**Why PathFinder works:**
- Gradual cost increase lets nets "negotiate" rather than permanently blocking
- A net that truly needs a contested resource will keep routing through it (absorbing
  the increasing cost), while nets with alternatives naturally find cheaper paths
- History prevents oscillation (same two nets repeatedly swapping the same resource)

**Typical convergence:** 5-30 iterations for medium complexity boards. Each iteration
routes all nets, so total A* calls = `iterations × num_subnets`.

**Parameters:**

| Parameter | Typical Value | Notes |
|-----------|--------------|-------|
| `HISTORY_INCREMENT` | 1.0 | Linear accumulation per oversubscribed iteration |
| `PRES_FAC_MULTIPLIER` | 1.15 | Exponential growth per iteration |
| `PRES_FAC_CAP` | 8.0 | Maximum present congestion factor |
| `max_iterations` | 50 | Give up after this many iterations |

**Optimization: partial rip-up.** Instead of ripping up all nets each iteration,
only rip up nets that pass through oversubscribed cells. This significantly reduces
runtime. OrthoRoute uses a "hot set" of 100 worst-offending nets.

### Key References

- McMurchie & Ebeling, "PathFinder: A Negotiation-Based Performance-Driven Router
  for FPGAs," ACM FPGA 1995
- Tessier, "Negotiated A* Routing for FPGAs," FPD 1998

---

## Phase 4: Trace Optimization and Smoothing

After PathFinder converges, traces are often jagged with unnecessary stair-stepping,
right angles, and suboptimal paths. Optimization improves manufacturing quality and
signal integrity.

### Staircase Elimination

Replace staircase patterns (alternating H-V segments) with diagonal 45° segments:

```
Before:     After:
  │            │
  └──┐         └─╲
     │            ╲
     └──          ──
```

Algorithm: scan each trace for consecutive bend pairs and test whether replacing
them with a diagonal violates clearance (query the R-tree spatial index).

### Rubber-Banding (Trace Tightening)

Treat each trace as a rubber band pinned at its endpoints and pulled tight around
obstacles. This is where **Solverang can be repurposed**:

1. Represent each trace vertex as a Solverang `Entity` with `[x, y]` params
   (allocated via `system.alloc_param(value, vertex_entity_id)`)
2. Fix the two endpoints via `system.fix_param()` (pad positions)
3. Add `Clearance` constraints against all nearby obstacles (from R-tree query)
4. Add a `MinimizeLength` soft objective (sum of segment lengths, `is_soft() = true`)
5. `system.solve()` → `SystemResult` — Solverang pulls the trace tight while
   maintaining clearance. The 5-phase pipeline automatically decomposes
   independent trace segments into separate clusters for parallel solving.

This naturally handles complex obstacle configurations that heuristic smoothing
algorithms struggle with.

### 45-Degree Conversion

For designs using 45° routing style, convert right-angle bends to chamfered 45°:

```
Before:     After:
  │            │
  └────        └╲
                 ╲───
```

This eliminates acid traps (internal acute angles that trap etchant during
manufacturing) and improves impedance continuity.

### Length Matching (Post-Route)

For length-matched net groups (DDR data lines, differential pairs), add serpentine
tuning segments. This is a separate pass after initial routing:

1. Measure routed length of each net in the matched group
2. Identify the longest net (target length)
3. For shorter nets, insert serpentine segments in uncongested areas
4. Serpentine parameters: amplitude, pitch, corner style (curved vs. mitered)

---

## Placement-Router Co-Optimization

The most advanced EDA tools don't treat placement and routing as sequential stages.
They use **feedback loops** in both directions.

### Forward Integration: Placer Uses Router Metrics

During the SA (Simulated Annealing) placement phase, invoke a "fast global router"
to evaluate each proposed component move:

```
SA_Cost = w₁·HPWL + w₂·net_crossings + w₃·max_cell_congestion
```

When the SA proposes swapping two components:
1. Quickly update the Steiner trees for affected nets
2. Update the global routing congestion grid
3. Compute `max_cell_congestion` — if a narrow channel between two BGAs is
   oversubscribed, the congestion penalty explodes and SA rejects the move

This prevents the placer from creating locally-optimal-but-unroutable configurations.

**Performance consideration:** Full global routing per SA move is too slow. Use a
simplified "congestion oracle" — project net bounding boxes onto the grid and
increment demand, without actually pathfinding. O(nets_affected × bbox_cells) per
move, typically < 1ms.

### Backward Integration: Router Adjusts Placement

When the detailed router (PathFinder) gets completely stuck:

1. **Identify the bottleneck:** which grid cells are persistently oversubscribed?
2. **Identify blocking components:** which components border those cells?
3. **Generate a placement constraint:** e.g., `Repel(U4, U5, distance=3mm)`
4. **Fork the board state** using persistent data structures
5. **Run micro-SA or a quick Solverang pass** to shift components slightly
6. **Resume routing** on the adjusted placement

This requires the `im` crate (or equivalent persistent data structures) for cheap
state forking. Clone-on-write means forking a full board state is O(1), and only
the mutated parts allocate new memory.

### The Co-Optimization Loop

```rust
fn co_optimize(board: &mut PcbIr, max_outer_loops: u32) -> CoOptResult {
    for outer in 0..max_outer_loops {
        // Phase 1-2: Place (or micro-adjust placement)
        let placement = if outer == 0 {
            solverang_place(board)
        } else {
            micro_adjust_placement(board, &routing_feedback)
        };

        // Phase 3: SA with congestion oracle
        let sa_placement = simulated_annealing(placement, |move_| {
            let congestion = fast_congestion_estimate(board, move_);
            hpwl_cost(move_) + CONGESTION_WEIGHT * congestion
        });

        // Phase 4: Route
        let routing = pathfinder_route(board, &sa_placement);

        match routing {
            RoutingResult::Success => return CoOptResult::Success,
            RoutingResult::Incomplete { bottlenecks } => {
                routing_feedback = bottlenecks;
                // Loop back to micro-adjust placement
            }
        }
    }
    CoOptResult::PartialSuccess
}
```

---

## Board Representation and Data Structures

### Spatial Index (R-tree)

All copper geometry (pads, traces, vias, fills, keepouts) is stored in an R-tree
for O(log n) spatial queries:

```rust
use rstar::RTree;

struct BoardSpatialIndex {
    /// All fixed obstacles (pads, keepouts, board edge)
    fixed: RTree<CopperObject>,
    /// Currently routed traces (mutated during routing)
    routed: RTree<TraceSegment>,
}

impl BoardSpatialIndex {
    /// Find all objects within `clearance` distance of a proposed trace segment
    fn clearance_query(&self, segment: &LineSegment, clearance: f64)
        -> Vec<&CopperObject>;

    /// Check if a grid cell is blocked (for obstacle map generation)
    fn is_blocked(&self, cell: GridCell, layer: u16, clearance: f64) -> bool;
}
```

### Persistent Data Structures

For co-optimization and undo/redo, use structural sharing:

```rust
use im::HashMap as ImHashMap;
use im::Vector as ImVector;

struct RoutingState {
    /// Per-net routed paths (persistent — forking is O(1))
    paths: ImHashMap<NetId, ImVector<TraceSegment>>,
    /// Grid occupancy (persistent)
    occupancy: ImHashMap<GridCellId, u16>,
}
```

Cloning `RoutingState` shares all underlying memory. Mutations only allocate for
changed tree nodes. This enables:
- **PathFinder iteration rollback:** cheaply snapshot before each iteration
- **Co-optimization branching:** fork board state, try micro-adjustment, drop if worse
- **Parallel exploration:** clone state for speculative routing of multiple orderings

**Crate status:** `im` is stable but unmaintained since 2022. Evaluate `rpds` as
alternative if needed. For the routing use case, `im`'s `HashMap` and `Vector`
are sufficient.

---

## Design Rule Integration

### Rules Relevant to Routing

From the 70 Altium design rules (see `design-rules-mapping.md`), these are
routing-critical:

| Rule | TRuleKind | Router Impact |
|------|-----------|---------------|
| **Clearance** | 0 | Minimum copper-to-copper distance → obstacle inflation |
| **Width** | 2 | Min/max/preferred trace width per net class |
| **Length** | 3 | Max trace length (timing) |
| **MatchedLengths** | 4 | Length matching within net groups |
| **DaisyChainStubLength** | 5 | Max stub length for daisy-chain nets |
| **PowerPlaneClearance** | 12 | Clearance in power planes |
| **MinimumAnnularRing** | 19 | Via annular ring → min via size |
| **RoutingViaStyle** | 20 | Allowed via sizes and drill diameters |
| **RoutingTopology** | 40 | Star, daisy-chain, min-spanning-tree |
| **RoutingPriority** | 41 | Net routing order |
| **MaxMinHoleSize** | 42 | Via drill size limits |
| **RoutingLayers** | 43 | Which layers a net may use |
| **RoutingCornerStyle** | 56 | 90°, 45°, or rounded corners |
| **DiffPairsRouting** | 44 | Gap, width, length matching |

### Rule Application in the Router

Rules are applied at different phases:

**Pre-routing (obstacle map generation):**
- Clearance → inflate all obstacles by clearance distance
- RoutingLayers → mask out forbidden layers per net

**During A* search:**
- Width → set minimum grid cell width requirement per net
- RoutingCornerStyle → penalize or forbid certain bend angles
- RoutingViaStyle → via cost model per allowed via type
- DiffPairsRouting → route paired nets simultaneously

**Post-routing verification (DRC):**
- All rules checked against final copper geometry
- Violations reported with exact locations and distances

---

## Differential Pair and Bus Routing

### Differential Pairs

Differential pairs (USB D+/D−, HDMI TMDS, Ethernet) must be routed as coupled
traces with:
- Matched impedance (controlled by trace width and gap)
- Matched length (within ±5 mils typically)
- Constant separation (specified gap)

**Algorithm:** Route both traces simultaneously. Modify A* to search over
`(x, y, layer, gap_offset)` where `gap_offset` ∈ {-gap/2, +gap/2}`:

1. Find the path for the "primary" trace using standard A*
2. Offset the path by ±gap/2 to create the secondary trace
3. Check that both traces have clearance to all obstacles
4. If the offset trace collides, backtrack and find an alternative primary path

**Alternative (post-route pairing):** Route both signals independently, then use
Solverang trace smoothing to pull them into alignment. This is simpler but may
produce suboptimal results.

### Bus Routing

Buses (DDR data lines, address buses) should be routed as parallel groups:

1. **Pre-routing:** Identify bus nets from net classes
2. **Ordering:** Assign each bus signal a position within the group (minimize crossings)
3. **Channel routing:** Route the group through a single channel, maintaining order
4. **Length matching:** Post-route serpentine insertion for matched-length groups

---

## GPU Acceleration Strategy

### What to GPU-Accelerate

GPUs are excellent at massive, regular, parallel computation but terrible at
branching and irregular memory access. Map work accordingly:

**Good for GPU (regular, parallel):**
- Obstacle map generation (render all obstacles to grid bitmap)
- Clearance distance field computation (parallel flood-fill)
- Congestion map updates (increment counters per cell)
- Multiple independent net BFS/Dijkstra (parallel wavefront expansion)
- FFT-based density computation (for analytical placement)

**Bad for GPU (irregular, branching):**
- A* priority queue operations (heap manipulations)
- PathFinder negotiation logic (conditional cost updates)
- Steiner tree construction (recursive decomposition)
- Design rule evaluation (complex conditional logic)

### Architecture

```
CPU (control plane):           GPU (data plane):
├── PathFinder loop            ├── Obstacle bitmaps (per-layer)
├── Net ordering               ├── Clearance distance fields
├── A* search (priority queue) ├── Congestion grid (atomic counters)
├── DRC evaluation             ├── Parallel BFS for N independent nets
└── Co-opt decisions           └── Density FFT (analytical placement)
```

### OrthoRoute's Approach (Proven)

OrthoRoute (2025) demonstrated GPU-accelerated PathFinder on real PCBs:
- **Parallel Dijkstra:** GPU computes single-source shortest path (SSSP) for each
  net, replacing CPU A* with GPU-parallel wavefront expansion
- **Results:** Full backplane routing (17,600 pads, 32 layers) in 41 hours on A100
  (vs. Freerouting's projected ~1 month)
- **Memory:** ~33.5 GB VRAM for the largest boards (significant!)
- **Limitation:** Nets routed sequentially within each PathFinder iteration; GPU
  parallelism is within each net's SSSP

### Rust GPU Stack

| Crate | Role | Backend |
|-------|------|---------|
| `wgpu` | Compute shaders (WGSL) | Vulkan, Metal, DX12, WebGPU |
| `burn` | Tensor operations + autograd | WGPU, CUDA, ROCm, CPU |
| `rust-gpu` | Write GPU kernels in Rust (experimental) | SPIR-V → Vulkan |

**Recommendation:** Start CPU-only. Add GPU acceleration when CPU becomes the
bottleneck (boards with >1000 nets or >8 layers). Use `wgpu` compute shaders for
the obstacle/congestion maps and parallel BFS.

---

## Rust Crate Ecosystem

### Required Dependencies

| Crate | Version | Purpose | Status |
|-------|---------|---------|--------|
| `pathfinding` | 4.14 | A*, Dijkstra, BFS inner-loop routing | Active, well-maintained |
| `petgraph` | 0.8 | Netlist graph, connectivity, MST | Active, 300M downloads |
| `rstar` | 0.12 | R-tree spatial index for copper geometry | Active, georust ecosystem |
| `geo` | 0.32 | Polygon ops, boolean ops, distance | Active, georust ecosystem |
| `spade` | 2.15 | Delaunay triangulation (shape-based routing) | Active |
| `good_lp` | 1.15 | ILP for layer assignment | Active, HiGHS backend |
| `bitvec` | 1.x | Obstacle bitmaps (memory-efficient grids) | Active |

### Optional Dependencies

| Crate | Version | Purpose | Status |
|-------|---------|---------|--------|
| `im` | 15.1 | Persistent data structures (state forking) | Stable but unmaintained (2022) |
| `wgpu` | 28.0 | GPU compute shaders | Active, production-ready |
| `burn` | 0.21 | Tensor ops + autograd (DREAMPlace-style) | Active, pre-1.0 |
| `rayon` | 1.x | CPU parallelism (parallel net routing) | Active |

### Not Recommended

| Crate/Approach | Why Not |
|----------------|---------|
| CUDA directly | Locks to NVIDIA; `wgpu` is cross-vendor |
| RL training (DeepPCB-style) | Needs massive training infra; LLM already encodes heuristics |
| Diffusion models | No constraint guarantees; PCB-scale doesn't need generative models |
| Custom graph library | `petgraph` + `pathfinding` cover all needs |

---

## IR Extensions for Routing

The `altium-format-ir` crate (see `ir.md`) needs routing-specific extensions:

### New Types

```rust
/// Routed trace segment
pub struct IrTraceSegment {
    pub net: NetId,
    pub layer: LayerId,
    pub start: IrPoint,    // mm coordinates
    pub end: IrPoint,
    pub width_mm: f64,
}

/// Via instance
pub struct IrVia {
    pub net: NetId,
    pub position: IrPoint,
    pub from_layer: LayerId,
    pub to_layer: LayerId,
    pub drill_mm: f64,
    pub annular_ring_mm: f64,
}

/// Routing result for a single net
pub struct IrRoutedNet {
    pub net: NetId,
    pub segments: Vec<IrTraceSegment>,
    pub vias: Vec<IrVia>,
    pub routed_length_mm: f64,
}

/// Complete routing solution
pub struct IrRoutingSolution {
    pub nets: IdMap<NetId, IrRoutedNet>,
    pub unrouted: Vec<NetId>,
    pub total_vias: u32,
    pub total_length_mm: f64,
    pub drc_violations: Vec<IrDrcViolation>,
}
```

### Routing-Specific Extraction

```rust
/// Extract routing-relevant data from PcbDoc
pub fn extract_routing_ir(doc: &PcbDoc) -> Result<RoutingIr> {
    // Board outline → obstacle boundary
    // Pads → pin positions and obstacle regions
    // Existing traces → pre-routed segments (honor locked traces)
    // Keepouts → forbidden regions
    // Design rules → clearance/width/via constraints
    // Net classes → routing parameters per net group
    // Layer stack → available routing layers and preferred directions
}
```

### Bridge to PcbDoc Write-Back

The routing solution must be convertible back to PcbDoc format:

```rust
/// Convert routing solution to PcbDoc track and via records
pub fn routing_to_pcbdoc(
    solution: &IrRoutingSolution,
    ir: &PcbIr,
) -> Result<Vec<PcbDocRecord>> {
    // IrTraceSegment → Track6 records
    // IrVia → Via6 records
    // Coordinate conversion: mm → Altium internal units
}
```

---

## Implementation Roadmap

### Milestone 1: Single-Net A* Router (MVP)

**Goal:** Route one 2-pin net through a grid with obstacles.

- [ ] `RoutingGrid` struct with obstacle bitmap
- [ ] 3D A* using `pathfinding` crate
- [ ] Via cost model (basic)
- [ ] Obstacle extraction from PcbIr
- [ ] CLI: `altium route <pcbdoc> --net <name>` (single net, debug output)

### Milestone 2: Multi-Net with PathFinder

**Goal:** Route all nets on a board using PathFinder negotiation.

- [ ] FLUTE net decomposition (or MST via `petgraph`)
- [ ] PathFinder iteration loop
- [ ] Net ordering heuristic
- [ ] History and present congestion tracking
- [ ] Partial rip-up (hot set optimization)
- [ ] CLI: `altium route <pcbdoc>` (all nets, report completion rate)

### Milestone 3: Design Rule Integration

**Goal:** DRC-clean routing.

- [ ] Clearance-aware obstacle inflation
- [ ] Width/via rules per net class
- [ ] Layer restriction rules
- [ ] Routing corner style (45° support)
- [ ] Post-route DRC verification
- [ ] CLI: `altium route <pcbdoc> --drc` (route + verify)

### Milestone 4: Global Routing

**Goal:** Congestion-aware routing for complex boards.

- [ ] Global routing grid (coarse overlay)
- [ ] Congestion capacity estimation
- [ ] ILP layer assignment (`good_lp`)
- [ ] Region-guided detailed routing
- [ ] Forward integration with SA placer (congestion oracle)

### Milestone 5: Trace Optimization

**Goal:** Clean, manufacturable traces.

- [ ] Staircase elimination
- [ ] 45° conversion
- [ ] Rubber-banding via Solverang
- [ ] Length matching / serpentine insertion

### Milestone 6: Differential Pairs and Buses

**Goal:** High-speed design support.

- [ ] Coupled differential pair routing
- [ ] Bus routing (parallel groups)
- [ ] Length matching within groups

### Milestone 7: Placement-Router Co-Optimization

**Goal:** Feedback loop between placer and router.

- [ ] Congestion oracle for SA cost function
- [ ] Backward feedback (router → placer nudging)
- [ ] Persistent data structures for state forking
- [ ] Multi-round co-optimization loop

### Milestone 8: GPU Acceleration (Future)

**Goal:** Handle large boards (>1000 nets, >8 layers).

- [ ] `wgpu` compute shaders for obstacle/congestion maps
- [ ] Parallel BFS/Dijkstra per net on GPU
- [ ] `burn` for analytical placement (DREAMPlace-style)

---

## Key References

### Algorithms
- McMurchie & Ebeling, "PathFinder: A Negotiation-Based Performance-Driven Router
  for FPGAs," ACM FPGA 1995
- Tessier, "Negotiated A* Routing for FPGAs," FPD 1998
- Chu & Wong, "FLUTE: Fast Lookup Table Based Rectilinear Steiner Minimal Tree
  Algorithm for VLSI Design," IEEE TCAD 2008
- Lin et al., "DREAMPlace: Deep Learning Toolkit-Enabled GPU Acceleration for
  Modern VLSI Placement," DAC 2019

### Tools
- OrthoRoute (2025): GPU-accelerated PathFinder for PCB
  (https://github.com/bbenchoff/OrthoRoute)
- Freerouting: Open-source shape-based PCB router
  (https://github.com/freerouting/freerouting)
- DeepPCB: AI-powered cloud router (https://deeppcb.ai/)
- Cypress (ISPD 2025): GPU-accelerated PCB placement

### Papers (2023-2026)
- FanoutNet (AAAI 2023): RL for BGA fanout automation
- 3D Line Exploration for Multi-Layer PCB (Nature Scientific Reports 2026)
- Multi-Agent Minimal-Layer Via Routing (2025)
