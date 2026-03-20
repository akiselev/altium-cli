# GAMER Sweep Routing: GPU-Accelerated Maze Routing via H/V Sweep Decomposition

## Overview

This plan describes implementing the GAMER algorithm (GPU-Accelerated Maze Routing,
IEEE TCAD 2023) as an alternative GPU SSSP backend alongside Bellman-Ford (Corolla
approach) in our `autopcb-router` crate. GAMER decomposes shortest-path search into
alternating horizontal and vertical sweep operations, each running in O(log n)
parallel time on an n-cell row/column, versus O(n) for a single Bellman-Ford
iteration over the same dimension.

**Paper**: "GAMER: GPU-Accelerated Maze Routing" -- IEEE TCAD 2023, Vol. 42, Issue 2,
pp. 583-593. https://ieeexplore.ieee.org/document/9799536

**Reported results**: 16x average speedup on ICCAD 2019 global routing benchmarks.
Applied to CUGR open-source global router: 19.85x on coarse routing, 2.59x on fine
routing, 2.7x overall. No quality loss.

**Relationship to existing plan**: This implements a second GPU SSSP strategy behind
the same `DetailedRouter` trait (defined in
`crates/autopcb-router/src/detailed/grid.rs`). The Bellman-Ford approach (Corolla)
and the GAMER sweep approach share the same obstacle maps, cost encoding, grid
linearization, and `PathFinder` integration points.

---

## Pipeline Integration

GAMER (this plan) implements an alternative GPU SSSP kernel — interchangeable with Corolla (01) at step 3 of each PathFinder iteration.

```
PathFinder Iteration:
  1. Rip-up (CPU)
  2. InstantGR (05) → batch nets into independent groups
  3. For each batch:
     Corolla (01) OR GAMER (02) [this plan] → GPU SSSP per net in batch
  4. X-Check (03) → GPU DRC, violations → history
  5. History update (GPU kernel)
  6. Convergence check (CPU)

After routing:
  Cypress (04) → congestion feedback → placement SA
```

**This plan's role**: GAMER sweep is the second GPU SSSP backend (step 3). It receives the same net batches from InstantGR (05) as Corolla does. Auto-selection (`backend_select.rs`) picks GAMER for large subnets (bbox > 10,000 cells) and Corolla for smaller ones. Output (`Vec<PathSegment>`) is identical — consumed by X-Check (03).

### Shared `GpuRoutingEngine`

Uses shared `GpuRoutingEngine` from `gpu/engine.rs` (see Plan 01 for full definition). GAMER-specific fields/pipelines used:

| Field | Purpose |
|-------|---------|
| `device`, `queue` | wgpu primitives |
| `obstacle_bitmap` | Read-only obstacles per layer |
| `history_costs` | PathFinder history, read during sweep cost computation |
| `distance` | Reset per net, H/V sweeps write via `atomicMin` |
| `predecessor` | Written by sweep passes |
| `routing_params` | Grid dims, costs, source/target |
| `sweep_h_pipeline` | Horizontal sweep compute pipeline |
| `sweep_v_pipeline` | Vertical sweep compute pipeline |
| `via_transition_pipeline` | Layer-to-layer via propagation |
| `reset_pipeline` | Reset distances to INFINITY before each net |

### Module Structure

All files live under the shared GPU module:

```
crates/autopcb-router/src/gpu/
├── mod.rs              // GpuRoutingEngine (shared device, queue, buffers, pipelines)
├── engine.rs           // GpuRoutingEngine struct, initialization, buffer management
├── buffers.rs          // Buffer types, layout, upload/download helpers
├── bellman_ford.rs     // Corolla BF dispatch (01)
├── sweep.rs            // GAMER H/V sweep dispatch (02) [this file]
├── drc.rs              // X-Check GPU DRC (03)
├── congestion.rs       // Cypress congestion estimation (04)
├── batching.rs         // InstantGR net batching logic (05)
├── backend_select.rs   // Auto-select Corolla vs GAMER
├── cpu_reference.rs    // CPU reference BF for testing
└── shaders/
    ├── bellman_ford.wgsl
    ├── reset_dist.wgsl
    ├── convergence.wgsl
    ├── horizontal_sweep.wgsl
    ├── vertical_sweep.wgsl
    ├── via_transition.wgsl
    ├── history_update.wgsl
    ├── drc_sweepline.wgsl
    ├── drc_short_check.wgsl
    ├── drc_violation_collect.wgsl
    ├── congestion_rudy.wgsl
    ├── congestion_overflow.wgsl
    ├── congestion_component_score.wgsl
    ├── batch_reset.wgsl
    └── batch_conflict_check.wgsl
```

---

## 1. GAMER Algorithm Breakdown

### 1.1 Core Insight: H/V Sweep Decomposition

Standard maze routing (Lee/BFS, Dijkstra, Bellman-Ford) propagates shortest-path
wavefronts in all directions simultaneously. On an N x N grid, Bellman-Ford needs
O(N) iterations to propagate a distance across the full grid diagonal, and each
iteration processes O(N^2) cells -- giving O(N^3) total work.

GAMER's key insight: on a Manhattan (rectilinear) grid, shortest paths are composed
of horizontal and vertical segments. Instead of propagating distances in all
directions at once, decompose the problem:

1. **Horizontal sweep (H-sweep)**: For each row independently, propagate distances
   left-to-right and right-to-left along the row. Each cell's distance is updated
   to `min(current, left_neighbor + 1, right_neighbor + 1)`. This is a parallel
   prefix min operation -- O(log W) parallel time for a row of width W.

2. **Vertical sweep (V-sweep)**: For each column independently, propagate distances
   top-to-bottom and bottom-to-top. Same parallel prefix min, O(log H) time for
   height H.

3. **Alternate H and V sweeps** until convergence. On a clear grid, O(1) H+V
   sweep pair suffices for a single source-target pair. With obstacles, O(D)
   pairs may be needed where D is the "obstacle detour depth" -- typically small
   (2-5 for PCB routing grids).

### 1.2 Step-by-Step Algorithm

```
Input:  2D grid, source cells (distance = 0), obstacle cells (distance = INF, blocked)
Output: shortest distance from any source to every reachable cell

1. Initialize distance[source] = 0, distance[*] = INFINITY
2. Repeat until no distance changes:
   a. H-sweep: for each row r in parallel:
      - Forward pass (left to right):
        for col = 1..W:
          dist[r][col] = min(dist[r][col], dist[r][col-1] + cost)
        if obstacle at (r, col): dist[r][col] = INFINITY
      - Backward pass (right to left):
        for col = W-2..0:
          dist[r][col] = min(dist[r][col], dist[r][col+1] + cost)
        if obstacle at (r, col): dist[r][col] = INFINITY
   b. V-sweep: for each column c in parallel:
      - Forward pass (top to bottom):
        for row = 1..H:
          dist[row][c] = min(dist[row][c], dist[row-1][c] + cost)
        if obstacle at (row, c): dist[row][c] = INFINITY
      - Backward pass (bottom to top):
        for row = H-2..0:
          dist[row][c] = min(dist[row][c], dist[row+1][c] + cost)
        if obstacle at (row, c): dist[row][c] = INFINITY
3. Read distance[target] for shortest path cost
4. Trace back predecessor chain for path reconstruction
```

### 1.3 Parallel Prefix Min Reduction (O(log n) per Sweep)

Within each row/column, the forward and backward passes are scan operations
(prefix min). A naive sequential scan is O(n). GAMER parallelizes each scan using
the Blelloch parallel prefix sum algorithm adapted for min:

**Up-sweep (reduce)**: Build a balanced binary tree over the row. At each level,
combine pairs: `val[parent] = min(val[left_child], val[right_child] + offset)`.
This takes O(log n) parallel steps.

**Down-sweep (distribute)**: Propagate the combined minimums back down the tree.
Each node distributes its value to children, accounting for offsets. Also O(log n)
steps.

Total: O(log n) parallel time per row/column, with O(n) work per row/column.
For the full grid: O(log(max(W,H))) parallel time, O(W*H) work per sweep.

### 1.4 Multi-Source Multi-Destination Formulation

GAMER naturally handles multi-source multi-destination shortest paths:

- **Multi-source**: Set `distance = 0` at ALL source cells (e.g., all pins of a
  partially-routed net). The sweep propagates from all sources simultaneously.
  No algorithmic change needed -- this is inherent to the wavefront approach.

- **Multi-destination**: After convergence, read distances at all target cells.
  The minimum gives the closest target. For Steiner tree construction, connect
  the closest target first, then add its tree cells to the source set and
  re-run for the next target (same as PathFinder's incremental tree growing).

This maps to our `Subnet` decomposition in
`crates/autopcb-router/src/global/steiner.rs`. Each `Subnet` has a source
`PointMm` and target `PointMm`. For multi-pin nets, the MST decomposer
(`MstDecomposer`) produces 2-pin subnets routed incrementally.

### 1.5 Obstacle Handling

Obstacles differ from standard Bellman-Ford in the sweep approach:

- **Bellman-Ford**: Obstacles are handled by skipping blocked cells during
  relaxation (check `is_blocked()` before writing to neighbor). Distance
  at blocked cells stays INFINITY.

- **GAMER sweeps**: After each scan step within a row/column, obstacle cells
  reset to INFINITY. This "breaks" the scan chain at obstacle boundaries,
  forcing the wavefront to route around obstacles in subsequent H/V sweep
  pairs. The key correctness property: if an obstacle blocks a row segment,
  the horizontal scan cannot propagate across it. The vertical sweep in the
  next iteration carries distances around the obstacle via adjacent rows,
  and the following horizontal sweep continues propagation on the far side.

  **Critical implementation detail**: Obstacle enforcement must happen AFTER
  each min-update within the scan, not just at scan boundaries. In the
  parallel prefix approach, obstacle cells are treated as "absorbers" that
  prevent propagation through them. This is equivalent to inserting infinite-
  cost barriers in the prefix tree.

### 1.6 3D Extension (Multiple Layers + Via Transitions)

PCB routing is 3D: (x, y, layer). GAMER extends to 3D by adding a **via
transition sweep** between H and V sweeps:

```
Repeat until convergence:
  1. H-sweep on each layer (independently, in parallel)
  2. Via-sweep: for each (x, y) cell, propagate distances between layers:
     for each layer pair (L_i, L_j) that allows via transition:
       dist[x][y][L_j] = min(dist[x][y][L_j], dist[x][y][L_i] + via_cost)
  3. V-sweep on each layer (independently, in parallel)
  4. Via-sweep again (distances may have improved after V-sweep)
```

The via-sweep is embarrassingly parallel: each (x, y) column through the
layer stack is independent. With K layers, it is O(K^2) work per cell
(check all layer pairs), or O(K) if only adjacent-layer vias are allowed.

For typical PCB boards (2-32 copper layers per `IrLayerStack`), K is small
enough that the via sweep is negligible compared to H/V sweeps.

---

## 2. WGSL Shaders Needed

All shaders live in `crates/autopcb-router/src/gpu/shaders/` (same directory
as the Bellman-Ford shaders described in `05-wgpu-implementation.md`).

### 2.1 `horizontal_sweep.wgsl`

Parallel row-wise distance propagation. One workgroup per row.

```wgsl
// Shared type definitions (prepended at load time from types.wgsl)

@group(0) @binding(0) var<storage, read_write> distance: array<atomic<u32>>;
@group(0) @binding(1) var<storage, read>       obstacle: array<u32>;
@group(0) @binding(2) var<storage, read_write> predecessor: array<u32>;
@group(1) @binding(0) var<uniform>             params: GridParams;

// Workgroup shared memory for parallel prefix min
var<workgroup> shared_dist: array<u32, 2048>;  // max row width
var<workgroup> shared_pred: array<u32, 2048>;

override WORKGROUP_SIZE: u32 = 256;

@compute @workgroup_size(WORKGROUP_SIZE, 1, 1)
fn horizontal_sweep(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
    @builtin(workgroup_id) wid: vec3<u32>,
) {
    let row = wid.y;
    let layer = wid.z;

    if row >= params.height || layer >= params.layer_count { return; }

    // Phase 1: Load row segment into shared memory
    // Each thread loads ceil(width / WORKGROUP_SIZE) elements
    let cells_per_thread = (params.width + WORKGROUP_SIZE - 1u) / WORKGROUP_SIZE;
    for (var i = 0u; i < cells_per_thread; i++) {
        let col = lid.x * cells_per_thread + i;
        if col < params.width {
            let idx = cell_index(col, row, layer);
            shared_dist[col] = atomicLoad(&distance[idx]);
            shared_pred[col] = predecessor[idx];
        }
    }
    workgroupBarrier();

    // Phase 2: Forward scan (left to right) -- sequential per thread chunk,
    // then parallel prefix min across chunks
    // ... (parallel prefix min implementation)

    // Phase 3: Backward scan (right to left) -- same pattern

    // Phase 4: Obstacle enforcement -- reset blocked cells
    for (var i = 0u; i < cells_per_thread; i++) {
        let col = lid.x * cells_per_thread + i;
        if col < params.width {
            if is_blocked(col, row, layer) {
                shared_dist[col] = 0xFFFFFFFFu;  // INFINITY
                shared_pred[col] = 0xFFFFFFFFu;  // NONE
            }
        }
    }
    workgroupBarrier();

    // Phase 5: Write back to global memory
    for (var i = 0u; i < cells_per_thread; i++) {
        let col = lid.x * cells_per_thread + i;
        if col < params.width {
            let idx = cell_index(col, row, layer);
            atomicStore(&distance[idx], shared_dist[col]);
            predecessor[idx] = shared_pred[col];
        }
    }
}
```

**Workgroup sizing**: One workgroup per (row, layer) pair. Dispatch as:
```rust
encoder.dispatch_workgroups(1, grid_height, layer_count);
```

For rows wider than `max_compute_workgroup_storage_size` (16 KB = 4096 u32
values = 2048 cells if storing both dist and pred), split the row into
tiles and run multi-pass prefix min with global memory synchronization
between tiles. For typical PCB grids (< 2000 cells wide at 0.1mm
resolution on a 200mm board), single-workgroup processing fits.

### 2.2 `vertical_sweep.wgsl`

Parallel column-wise distance propagation. One workgroup per column.

```wgsl
@compute @workgroup_size(WORKGROUP_SIZE, 1, 1)
fn vertical_sweep(
    @builtin(global_invocation_id) gid: vec3<u32>,
    @builtin(local_invocation_id) lid: vec3<u32>,
    @builtin(workgroup_id) wid: vec3<u32>,
) {
    let col = wid.y;
    let layer = wid.z;

    if col >= params.width || layer >= params.layer_count { return; }

    // Load column into shared memory (stride = width in global memory)
    // Forward scan (top to bottom)
    // Backward scan (bottom to top)
    // Obstacle enforcement
    // Write back
}
```

**Dispatch**: `encoder.dispatch_workgroups(1, grid_width, layer_count);`

**Memory access pattern**: Column access is strided in global memory
(`distance[cell_index(col, row, layer)]` with row varying). When loading
into shared memory, each thread reads `cells_per_thread` elements at
stride `grid_width`. This is NOT coalesced -- adjacent threads read
addresses separated by `grid_width * sizeof(u32)`. However, this is
mitigated by:
1. The data is in shared memory during the scan (coalesced shared access).
2. The load/store phases are bandwidth-bound, not compute-bound -- acceptable.
3. An alternative: transpose the grid between H and V sweeps (described in
   section 6.3 as an optimization).

### 2.3 `via_transition.wgsl`

Layer transition pass. One thread per (x, y) cell.

```wgsl
@compute @workgroup_size(8, 8, 1)
fn via_transition(
    @builtin(global_invocation_id) gid: vec3<u32>,
) {
    let x = gid.x;
    let y = gid.y;

    if x >= params.width || y >= params.height { return; }

    // For each layer pair that allows via transition:
    for (var from_layer = 0u; from_layer < params.layer_count; from_layer++) {
        let from_idx = cell_index(x, y, from_layer);
        let from_dist = atomicLoad(&distance[from_idx]);
        if from_dist == 0xFFFFFFFFu { continue; }

        for (var to_layer = 0u; to_layer < params.layer_count; to_layer++) {
            if from_layer == to_layer { continue; }
            let to_idx = cell_index(x, y, to_layer);

            // Check via transition is legal at this (x, y)
            if is_blocked(x, y, to_layer) { continue; }

            let new_dist = from_dist + params.via_cost_fp;
            atomicMin(&distance[to_idx], new_dist);
            // Predecessor update: encode via transition
        }
    }
}
```

**Dispatch**: `encoder.dispatch_workgroups(ceil(W/8), ceil(H/8), 1);`

Each thread processes all layers at its (x, y) cell. For 2-4 copper layers,
this is a small constant-time inner loop.

### 2.4 `reset_distances.wgsl`

Same as the Bellman-Ford reset shader. Sets all distances to INFINITY and
predecessors to NONE. Reused without changes.

### 2.5 `set_sources.wgsl`

Set distance = 0 at source cells. For multi-source (Steiner tree growing),
writes a small number of cells. Dispatched with one thread per source cell.

### 2.6 Shared `types.wgsl`

Shared type and constant definitions, prepended to all shaders:

```wgsl
struct GridParams {
    width: u32,
    height: u32,
    layer_count: u32,
    iteration: u32,
    via_cost_fp: u32,       // fixed-point via cost
    base_cost_fp: u32,      // fixed-point per-step cost
    pres_fac_fp: u32,       // fixed-point present congestion factor
    history_fac_fp: u32,    // fixed-point history multiplier
    source_cell: u32,
    target_cell: u32,
    _pad: vec2<u32>,        // pad to 48 bytes (multiple of 16)
}

const INFINITY: u32 = 0xFFFFFFFFu;
const NONE: u32 = 0xFFFFFFFFu;

fn cell_index(x: u32, y: u32, layer: u32) -> u32 {
    // GPU-friendly layout: layer-major, row-major (adjacent x = adjacent memory)
    return layer * (params.width * params.height) + y * params.width + x;
}

fn is_blocked(x: u32, y: u32, layer: u32) -> bool {
    let idx2d = y * params.width + x;
    return (obstacle[idx2d] & (1u << layer)) != 0u;
}
```

This `GridParams` struct matches the one defined in `05-wgpu-implementation.md`
section 1.4, ensuring consistency between the Bellman-Ford and GAMER pipelines.

---

## 3. Comparison with Bellman-Ford (Corolla Approach)

### 3.1 Algorithmic Complexity

| Metric | Bellman-Ford | GAMER Sweep |
|--------|-------------|-------------|
| Parallel time per iteration | O(1) per thread (relax neighbors) | O(log n) per sweep (prefix min) |
| Iterations to converge | O(diameter) = O(W+H) on grid | O(D) sweep pairs, D = obstacle detour depth |
| Total parallel time | O(W+H) | O(D * log(max(W,H))) |
| Work per iteration | O(W * H * L) | O(W * H * L) |
| Best case (no obstacles) | O(W+H) iterations | O(1) sweep pair |
| Worst case (maze-like) | O(W*H) iterations | O(W+H) sweep pairs |

**Key difference**: Bellman-Ford propagates distance by 1 cell per iteration in
the worst case. GAMER propagates across an entire row/column in O(log n) time per
sweep. For a 1000x1000 grid with no obstacles, Bellman-Ford needs ~2000 iterations;
GAMER needs 1 H+V sweep pair (2 dispatches instead of ~2000).

### 3.2 When is GAMER Faster?

- **Regular grids with few obstacles**: GAMER converges in 1-3 sweep pairs.
  Bellman-Ford needs O(diameter) iterations. Speedup: 100-1000x.
- **Large search spaces**: The O(log n) prefix min dominates as grid dimensions
  grow. A 2000x2000 grid has diameter ~4000; GAMER sweep pair is O(11) steps.
- **Multi-source routing**: GAMER handles all sources in one sweep (no extra
  iterations per source). Bellman-Ford handles this equally well with `atomicMin`.
- **PCB routing**: Typical PCB grids have sparse obstacles (pads, keepouts) with
  large open regions. GAMER excels here. Reported 16x speedup on VLSI benchmarks
  which have denser obstacles than PCBs.

### 3.3 When is Bellman-Ford Better?

- **Irregular graphs**: GAMER requires a regular grid. FPGA routing resource
  graphs (Corolla's target) are irregular. Bellman-Ford works on any graph.
  Our PCB routing IS a regular grid, so this is not a concern.
- **Small subgraphs**: Corolla's key optimization is restricting Bellman-Ford to
  a small bounding-box subgraph per net. GAMER sweeps over the full grid (or a
  large subgrid). For very short nets where the bounding box is small, Bellman-Ford
  on the subgraph may be faster due to lower constant factors.
- **Complex cost functions**: GAMER's parallel prefix min works with additive
  costs only. If costs are non-uniform and neighbor-dependent (e.g., congestion-
  weighted), the prefix reduction becomes a segmented prefix operation with
  per-element costs. This is still O(log n) but with higher constants.
  Bellman-Ford handles arbitrary cost functions trivially.
- **Very few iterations needed**: If Bellman-Ford converges in 5-10 iterations
  (e.g., short nets on uncongested grids), GAMER's overhead of parallel prefix
  setup may not pay off. The crossover point is roughly when the grid dimension
  exceeds ~100 cells per side.

### 3.4 Auto-Selection Strategy

Both backends implement the same `DetailedRouter` trait:

```rust
// crates/autopcb-router/src/detailed/grid.rs (existing)
pub trait DetailedRouter {
    fn route_subnet(
        &self,
        workspace: &RoutingWorkspace,
        subnet: &Subnet,
        net_id: NetId,
        history_costs: Option<&[f64]>,
    ) -> Result<Vec<PathSegment>, RoutingError>;
}
```

Auto-selection heuristic in `GpuRouter::route_subnet()`:

```rust
fn select_backend(&self, workspace: &RoutingWorkspace, subnet: &Subnet) -> GpuBackend {
    let bbox = subnet.bounding_box();
    let bbox_cells = (bbox.width() / workspace.grid.resolution_mm) as u32
        * (bbox.height() / workspace.grid.resolution_mm) as u32;

    if bbox_cells < 10_000 {
        // Small subnet: Bellman-Ford on subgraph (lower constant overhead)
        GpuBackend::BellmanFord
    } else {
        // Large subnet: GAMER sweep (asymptotically faster)
        GpuBackend::GamerSweep
    }
}
```

The threshold (10,000 cells = ~100x100 subgrid) is empirically tunable. Start
with this value and benchmark against specific PCB test cases.

### 3.5 Memory Usage Comparison

| Buffer | Bellman-Ford | GAMER |
|--------|-------------|-------|
| Distance | W*H*L * 4 bytes | Same |
| Predecessor | W*H*L * 4 bytes | Same |
| Obstacle bitmap | W*H * 4 bytes (packed) | Same |
| History | W*H*L * 4 bytes | Same |
| Workgroup shared | None required | 2 * max(W,H) * 4 bytes per workgroup |
| Change flag | 4 bytes | 4 bytes |
| **Total extra** | **0** | **~16 KB shared memory** (fits in workgroup limit) |

GAMER uses slightly more workgroup shared memory for the prefix scan buffers,
but no additional global buffers. Both approaches share the same SoA buffer
layout defined in `05-wgpu-implementation.md` section 1.1.

---

## 4. Integration Points

### 4.1 Replacing/Complementing A* in Milestone 6

The existing `GridRouter` in `crates/autopcb-router/src/detailed/grid.rs` uses
`pathfinding::directed::astar::astar` for CPU-side pathfinding. GAMER replaces
this with GPU-side sweep routing:

```
DetailedRouter trait
    |
    +-- GridRouter (CPU, pathfinding::astar)       [Existing, M6]
    |
    +-- GpuBellmanFordRouter (GPU, wgpu BF)        [Corolla plan, 01]
    |
    +-- GpuGamerRouter (GPU, wgpu H/V sweep)       [This plan, 02]
    |
    +-- ShapeRouter (CPU, geometry-based)           [Existing, M6]
```

All GPU backends are behind the same `DetailedRouter` trait. The `PathFinder`
negotiation loop in `crates/autopcb-router/src/pathfinder/mod.rs` is agnostic
to which backend is used -- it calls `router.route_subnet()` and receives
`Vec<PathSegment>` regardless.

**Config selection**: `RoutingConfig` in `crates/autopcb-router/src/config.rs`
gains a `gpu_backend: GpuBackendConfig` field:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum GpuBackendConfig {
    #[default]
    Cpu,
    BellmanFord,
    GamerSweep,
    Auto,  // auto-select per subnet based on bounding box size
}
```

### 4.2 Grid Construction from PcbIr

GAMER uses the same grid infrastructure as the Bellman-Ford approach:

- `GridConfig` from `crates/autopcb-router/src/workspace.rs` provides
  `resolution_mm`, `width_cells`, `height_cells`, and `origin`.
- `ObstacleMap` from `crates/autopcb-router/src/obstacles.rs` provides
  per-layer `BitVec` bitmaps. For GPU upload, these are packed into the
  `obstacle_mask: array<u32>` buffer where bit `i` represents layer `i`.
  Packing code:

  ```rust
  fn pack_obstacle_bitmaps(maps: &[ObstacleMap], width: u32, height: u32) -> Vec<u32> {
      let total_2d = (width as usize) * (height as usize);
      let mut packed = vec![0u32; total_2d];
      for (layer_idx, map) in maps.iter().enumerate() {
          for gy in 0..height {
              for gx in 0..width {
                  if map.is_blocked(gx, gy) {
                      let idx = gy as usize * width as usize + gx as usize;
                      packed[idx] |= 1 << layer_idx;
                  }
              }
          }
      }
      packed
  }
  ```

  This function is shared between Bellman-Ford and GAMER.

### 4.3 Cost Function Encoding

Both backends use the same fixed-point u32 cost encoding:

- **Scale factor**: 1000 (0.001 resolution, max cost ~4,294,967 in raw units)
- **Base move cost**: `(1.0 * 1000.0) as u32 = 1000` for cardinal,
  `(1.414 * 1000.0) as u32 = 1414` for diagonal
- **Via cost**: `(config.via_cost_base * 1000.0) as u32`
- **History cost at cell n**: `(history.get(x, y, layer) * 1000.0) as u32`
- **Present congestion**: `(pres_fac * 1000.0) as u32`

The PathFinder cost formula `C(n) = (b_n + h_n) * p_n` is computed in the
shader as:

```wgsl
let base = params.base_cost_fp;
let history = history_buf[cell_index(x, y, layer)];
let total_cell_cost = (base + history) * params.pres_fac_fp / 1000u;
// The /1000 compensates for the double scaling (base*1000 * pres*1000)
```

This encoding is identical to what `05-wgpu-implementation.md` specifies
and is shared between Bellman-Ford and GAMER.

### 4.4 PathFinder Integration

The PathFinder loop in `crates/autopcb-router/src/pathfinder/mod.rs`
calls `router.route_subnet()` for each subnet. The GPU GAMER router:

1. Receives `(&RoutingWorkspace, &Subnet, NetId, Option<&[f64]>)`.
2. Converts source/target `PointMm` to grid coords using
   `workspace.grid.to_grid(subnet.source)`.
3. Uploads grid params (source, target, costs) to uniform buffer.
4. Dispatches sweep passes until convergence.
5. Downloads predecessor array to CPU.
6. Traces back the path on CPU (same as Bellman-Ford).
7. Converts grid path to `Vec<PathSegment>` and returns.

The `history_costs: Option<&[f64]>` parameter is handled by uploading
the history array to the GPU history buffer. The HistoryArray in
`crates/autopcb-router/src/pathfinder/history.rs` uses CPU linearization
`x * (height * layers) + y * layers + layer`. For GPU, this is remapped
to `layer * (width * height) + y * width + x` during upload (a simple
reindex pass).

---

## 5. PcbIr Requirements

### 5.1 What GAMER Needs That Already Exists

Everything GAMER needs from PcbIr is already provided by the existing
crate infrastructure:

| Need | Source | Existing? |
|------|--------|-----------|
| Board bounding box | `PcbIr::board.bounds: BoundingBoxMm` | Yes |
| Pad positions + sizes | `PcbIr::components[*].pads[*].position: PointMm` | Yes |
| Net pin lists | `PcbIr::nets[*].pins: Vec<IrNetPin>` | Yes |
| Copper layer stack | `PcbIr::layer_stack.copper_layers: Vec<IrCopperLayer>` | Yes |
| Preferred directions | `IrCopperLayer::preferred_direction: Option<PreferredDirection>` | Yes |
| Keepout zones | `PcbIr::board.keepouts: Vec<IrKeepoutZone>` | Yes |
| Pre-routed tracks | `IrTrack::pre_routed: bool`, `IrTrack::layer: LayerId` | Yes |
| Pre-routed vias | `IrVia::pre_routed: bool`, `IrVia::from_layer/to_layer` | Yes |
| Design rules | `PcbIr::rules` via `build_policy()` | Yes |

### 5.2 What GAMER Needs That Corolla Does Not

GAMER does not require any PcbIr data beyond what Corolla needs. Both
approaches consume the same `RoutingWorkspace` (built by `build_workspace()`
from `PcbIr + RoutingConfig`). The difference is purely algorithmic -- how
the SSSP is computed, not what inputs it takes.

### 5.3 Additional Grid Metadata

GAMER benefits from one additional piece of metadata that the current
workspace does not compute:

**Per-layer preferred direction for sweep ordering**: When a layer has
`PreferredDirection::Horizontal`, the H-sweep should be applied first
on that layer (it will propagate distances farther in one sweep). When
`PreferredDirection::Vertical`, V-sweep first. This is a minor optimization
(affects convergence speed, not correctness).

This can be derived at workspace build time from `IrCopperLayer::preferred_direction`
and stored in `RoutingWorkspace` as a `Vec<SweepOrder>`:

```rust
#[derive(Debug, Clone, Copy)]
pub enum SweepOrder {
    HorizontalFirst,
    VerticalFirst,
}
```

Indexed by `layer.raw() as usize`, consistent with the Vec-based per-layer
collection pattern used throughout the workspace.

---

## 6. Performance Analysis

### 6.1 Theoretical Complexity

For an W x H grid with L layers and obstacle detour depth D:

| Operation | Bellman-Ford | GAMER |
|-----------|-------------|-------|
| One SSSP iteration | O(W*H*L) work, O(1) depth | O(W*H*L) work, O(log(max(W,H))) depth |
| Iterations | O(W+H) worst case | O(D) sweep pairs |
| Total work | O(W*H*L * (W+H)) | O(W*H*L * D) |
| Total parallel time | O(W+H) | O(D * log(max(W,H))) |
| GPU dispatches | O(W+H) per convergence check batch | O(D * 4) (H,via,V,via per pair) |

For a typical PCB routing grid: W=1000, H=1000, L=4, D=3 (sparse obstacles):
- Bellman-Ford: ~2000 iterations, ~2000 dispatches (or ~250 with 8-iteration batching)
- GAMER: ~3 sweep pairs = 12 dispatches, each with O(10) parallel steps

**Expected speedup over Bellman-Ford**: 100-200x fewer dispatches, each slightly
more expensive (prefix scan vs simple relaxation). Net speedup: 20-50x.

### 6.2 Expected Sweeps to Convergence

The number of H/V sweep pairs needed depends on obstacle layout:

| Scenario | Sweep pairs | Notes |
|----------|-------------|-------|
| Clear grid (no obstacles) | 1 | One H+V pair propagates everywhere |
| Sparse obstacles (typical PCB) | 2-3 | One extra pair per "level" of detour |
| Dense obstacles (BGA fanout) | 5-10 | Multiple detour levels |
| Maze-like (worst case) | O(W+H) | Degrades to Bellman-Ford speed |

For PCB routing, 2-5 sweep pairs is the expected norm. The "maze-like"
worst case is unrealistic for PCBs (no PCB has maze-like obstacle density).

### 6.3 Memory Access Patterns

**H-sweep (coalesced reads)**: Adjacent threads in a workgroup process
adjacent columns in the same row. Global memory reads of `distance[row * W + col]`
are coalesced -- optimal.

**V-sweep (strided reads)**: Adjacent threads process adjacent rows in the
same column. Global memory reads of `distance[row * W + col]` with `row` varying
are strided by W -- NOT coalesced.

**Mitigation options**:
1. **Accept the stride**: For 4-byte u32 values on a 1000-wide grid, the stride
   is 4000 bytes. Modern GPUs can handle this with L2 cache, but it is suboptimal.
2. **Grid transpose**: Before V-sweep, transpose the distance array so columns
   become rows. The V-sweep then reads coalesced. Transpose back before H-sweep.
   A GPU transpose of an N x N grid is O(N^2) work, O(1) depth with shared-memory
   tiling -- negligible cost.
3. **Two copies (shadow buffer)**: Maintain a second distance buffer in transposed
   layout. H-sweep reads/writes the row-major buffer; V-sweep reads/writes the
   column-major buffer. Cross-updates between them after each sweep. More memory
   but avoids transpose dispatch overhead.

**Recommendation**: Start with option 1 (accept stride). Profile, and if V-sweep
bandwidth is a bottleneck, implement option 2 (transpose). For PCB-scale grids
(< 2000 x 2000), the L2 cache should absorb most of the strided access penalty.

### 6.4 When to Recommend GAMER vs Bellman-Ford

| Board Characteristic | Recommended Backend |
|---------------------|-------------------|
| < 100 nets, small board | CPU A* (no GPU overhead) |
| 100-500 nets, moderate obstacles | GPU Bellman-Ford (lower constant factor) |
| > 500 nets, large open regions | GPU GAMER (asymptotic advantage) |
| Dense BGA areas (many obstacles) | GPU Bellman-Ford (GAMER needs more sweeps) |
| > 1000x1000 grid | GPU GAMER (log n advantage dominates) |

The auto-selection heuristic (section 3.4) handles per-subnet decisions within
a single board. A board may use both backends for different nets.

---

## 7. Rust Implementation Structure

### 7.1 Files (Unified GPU Module)

GAMER's dispatch logic lives in `gpu/sweep.rs` within the shared GPU module defined in Plan 01:

```
crates/autopcb-router/src/gpu/
├── mod.rs              // GpuRoutingEngine (shared)
├── engine.rs           // GpuRoutingEngine struct (shared)
├── buffers.rs          // Buffer types, upload/download (shared)
├── sweep.rs            // GAMER H/V sweep dispatch [this plan's dispatch logic]
├── bellman_ford.rs     // Corolla BF dispatch (Plan 01)
└── shaders/
    ├── horizontal_sweep.wgsl   // H-sweep with parallel prefix min
    ├── vertical_sweep.wgsl     // V-sweep with parallel prefix min
    ├── via_transition.wgsl     // Inter-layer via propagation
    ├── reset_dist.wgsl         // Reset distances to INFINITY (shared)
    ├── convergence.wgsl        // Change-flag check (shared)
    └── history_update.wgsl     // History increment (shared)
```

### 7.2 Key Structs

GAMER dispatch is implemented in `gpu/sweep.rs` as methods on the shared `GpuRoutingEngine` (defined in `gpu/engine.rs`, see Plan 01 for the full struct). The relevant dispatch methods:

```rust
// crates/autopcb-router/src/gpu/sweep.rs

impl GpuRoutingEngine {
    /// Route a subnet using the GAMER H/V sweep algorithm.
    pub fn route_gamer(
        &self,
        workspace: &RoutingWorkspace,
        subnet: &Subnet,
        net_id: NetId,
        history_costs: Option<&[f64]>,
    ) -> Result<Vec<PathSegment>, RoutingError> {
        // ... (dispatch sequence in section 7.3)
    }
}

impl DetailedRouter for GpuRoutingEngine {
    fn route_subnet(
        &self,
        workspace: &RoutingWorkspace,
        subnet: &Subnet,
        net_id: NetId,
        history_costs: Option<&[f64]>,
    ) -> Result<Vec<PathSegment>, RoutingError> {
        // backend_select.rs picks GAMER vs BF based on bbox size
        match self.select_backend(workspace, subnet) {
            GpuBackend::GamerSweep => self.route_gamer(workspace, subnet, net_id, history_costs),
            GpuBackend::BellmanFord => self.route_bellman_ford(workspace, subnet, net_id, history_costs),
        }
    }
}
```

### 7.3 GAMER Dispatch Sequence

```rust
impl GpuRouter {
    fn route_gamer(
        &self,
        workspace: &RoutingWorkspace,
        subnet: &Subnet,
        net_id: NetId,
        history_costs: Option<&[f64]>,
    ) -> Result<Vec<PathSegment>, RoutingError> {
        // 1. Upload history costs (if provided)
        if let Some(h) = history_costs {
            self.upload_history_reindexed(h, workspace);
        }

        // 2. Update uniform buffer with source/target
        let source = workspace.grid.to_grid(subnet.source);
        let target = workspace.grid.to_grid(subnet.target);
        self.update_params(source, target, workspace);

        // 3. Reset distances
        self.dispatch_reset();

        // 4. Set source distance to 0
        self.dispatch_set_sources(&[source]);

        // 5. Sweep loop
        let max_sweep_pairs = 20;  // safety cap
        for _ in 0..max_sweep_pairs {
            // Reset change flag
            self.queue.write_buffer(&self.change_flag_buf, 0, &0u32.to_le_bytes());

            let mut encoder = self.device.create_command_encoder(&Default::default());

            // H-sweep: one workgroup per (row, layer)
            {
                let mut pass = encoder.begin_compute_pass(&Default::default());
                pass.set_pipeline(&self.h_sweep_pipeline);
                pass.set_bind_group(0, &self.grid_bind_group, &[]);
                pass.set_bind_group(1, &self.params_bind_group, &[]);
                pass.dispatch_workgroups(1, self.grid_height, self.layer_count);
            }

            // Via transition
            {
                let mut pass = encoder.begin_compute_pass(&Default::default());
                pass.set_pipeline(&self.via_transition_pipeline);
                pass.set_bind_group(0, &self.grid_bind_group, &[]);
                pass.set_bind_group(1, &self.params_bind_group, &[]);
                pass.dispatch_workgroups(
                    (self.grid_width + 7) / 8,
                    (self.grid_height + 7) / 8,
                    1,
                );
            }

            // V-sweep: one workgroup per (column, layer)
            {
                let mut pass = encoder.begin_compute_pass(&Default::default());
                pass.set_pipeline(&self.v_sweep_pipeline);
                pass.set_bind_group(0, &self.grid_bind_group, &[]);
                pass.set_bind_group(1, &self.params_bind_group, &[]);
                pass.dispatch_workgroups(1, self.grid_width, self.layer_count);
            }

            // Via transition (again, after V-sweep)
            {
                let mut pass = encoder.begin_compute_pass(&Default::default());
                pass.set_pipeline(&self.via_transition_pipeline);
                pass.set_bind_group(0, &self.grid_bind_group, &[]);
                pass.set_bind_group(1, &self.params_bind_group, &[]);
                pass.dispatch_workgroups(
                    (self.grid_width + 7) / 8,
                    (self.grid_height + 7) / 8,
                    1,
                );
            }

            // Copy change flag to staging
            encoder.copy_buffer_to_buffer(
                &self.change_flag_buf, 0,
                &self.change_flag_staging, 0,
                4,
            );
            self.queue.submit(std::iter::once(encoder.finish()));

            // Check convergence
            let converged = self.read_change_flag() == 0;
            if converged { break; }
        }

        // 6. Download predecessor array
        let predecessors = self.download_predecessors();

        // 7. Trace back path on CPU
        let path = self.trace_back(&predecessors, source, target);

        Ok(path)
    }
}
```

---

## 8. Testing

### 8.1 Known-Answer Tests (CPU GAMER vs CPU A*)

Implement a CPU reference of the GAMER sweep algorithm for validation:

```rust
// crates/autopcb-router/src/gpu/tests.rs

/// CPU reference implementation of GAMER H/V sweep.
/// Used to validate GPU output without requiring GPU hardware.
fn cpu_gamer_sweep(
    obstacles: &[u32],     // packed bitmask per 2D cell
    width: u32,
    height: u32,
    layer_count: u32,
    sources: &[(u32, u32, u32)],  // (x, y, layer)
    base_cost: u32,
    via_cost: u32,
) -> (Vec<u32>, Vec<u32>) {
    // Returns (distance, predecessor) in GPU-layout linearization
    // ...
}

#[test]
fn cpu_gamer_matches_astar_empty_grid() {
    // 10x10 grid, no obstacles, single source -> single target
    let (gamer_dist, _) = cpu_gamer_sweep(&obstacles, 10, 10, 1, &[(0,0,0)], 1000, 10000);
    let astar_cost = run_cpu_astar(10, 10, 1, (0,0,0), (9,9,0), &obstacles);
    assert_eq!(gamer_dist[cell_index(9, 9, 0)], astar_cost);
}

#[test]
fn cpu_gamer_routes_around_obstacle() {
    // 10x10 grid, wall obstacle at col=5 rows 0-8
    // Source (0,5,0), target (9,5,0)
    // Expected: route goes around the wall via row 9
}

#[test]
fn cpu_gamer_multi_layer_via() {
    // 5x5 grid, 2 layers, obstacle blocking direct path on layer 0
    // Must route via layer 1
}
```

### 8.2 GPU vs CPU Reference Tests

```rust
#[cfg(feature = "gpu-tests")]
#[test]
fn gpu_gamer_matches_cpu_reference() {
    let adapter = match pollster::block_on(try_get_adapter()) {
        Some(a) => a,
        None => { eprintln!("Skipping: no GPU adapter"); return; }
    };

    let gpu_result = gpu_gamer_sweep(&adapter, &obstacles, 50, 50, 2, &[(0,0,0)], 1000, 10000);
    let cpu_result = cpu_gamer_sweep(&obstacles, 50, 50, 2, &[(0,0,0)], 1000, 10000);

    // Distance arrays must be identical (fixed-point, deterministic)
    assert_eq!(gpu_result.distances, cpu_result.distances);
}
```

### 8.3 Grid Sizes Where GAMER Outperforms Bellman-Ford

Benchmark both backends on identical grids of increasing size:

```rust
#[cfg(feature = "gpu-tests")]
#[test]
fn gamer_faster_than_bellman_ford_on_large_grid() {
    // 500x500 grid, sparse obstacles
    let gamer_time = bench_gamer(500, 500, 4);
    let bf_time = bench_bellman_ford(500, 500, 4);

    // GAMER should be significantly faster
    // (do not hard-assert ratio; just log for analysis)
    eprintln!("GAMER: {gamer_time:?}, BF: {bf_time:?}, ratio: {:.1}x",
              bf_time.as_secs_f64() / gamer_time.as_secs_f64());
}
```

Expected crossover: GAMER wins at ~100x100+ grid sizes. Below that,
Bellman-Ford's simpler kernel has lower overhead.

### 8.4 Convergence Tests

```rust
#[test]
fn gamer_converges_obstacle_free() {
    // Verify convergence in exactly 1 sweep pair on obstacle-free grid
    let sweeps = cpu_gamer_sweep_count(&[], 100, 100, 1, &[(0,0,0)]);
    assert_eq!(sweeps, 1, "obstacle-free grid should converge in 1 sweep pair");
}

#[test]
fn gamer_converges_with_wall() {
    // Wall across middle of grid with one gap
    // Should converge in 2-3 sweep pairs
    let sweeps = cpu_gamer_sweep_count(&wall_obstacles, 100, 100, 1, &[(0,50,0)]);
    assert!(sweeps <= 5, "wall obstacle should converge in <= 5 sweeps, got {sweeps}");
}
```

### 8.5 Test Feature Gates

Following the project's test infrastructure conventions:

```toml
# crates/autopcb-router/Cargo.toml
[features]
gpu-tests = ["wgpu"]  # GPU tests require wgpu as a dev-dependency
```

CPU reference tests run without any feature flag. GPU tests are gated behind
`gpu-tests` and gracefully skip when no adapter is available.

---

## 9. Implementation Milestones

### Phase 1: CPU Reference Implementation
- Implement `cpu_gamer_sweep()` in pure Rust
- Validate against existing A* on known test cases
- Verify convergence properties (sweep count vs obstacle density)
- **Files**: `crates/autopcb-router/src/gpu/gamer_cpu.rs`

### Phase 2: GPU Infrastructure (shared with Bellman-Ford)
- `GpuRouter` struct with device/queue initialization
- Buffer allocation for SoA distance/predecessor/obstacle/history arrays
- Uniform buffer for `GridParams`
- Obstacle bitmap packing from `Vec<ObstacleMap>`
- Staging buffers for predecessor readback
- Pipeline creation skeleton
- **Files**: `crates/autopcb-router/src/gpu/{mod,buffers,dispatch}.rs`

### Phase 3: GAMER WGSL Shaders
- `types.wgsl` (shared constants and functions)
- `reset.wgsl` and `set_sources.wgsl`
- `horizontal_sweep.wgsl` with parallel prefix min
- `vertical_sweep.wgsl` with parallel prefix min
- `via_transition.wgsl`
- **Files**: `crates/autopcb-router/src/gpu/shaders/*.wgsl`

### Phase 4: GAMER GPU Dispatch + Integration
- `GpuRouter::route_gamer()` dispatch sequence
- Convergence detection (change flag readback)
- Predecessor download and CPU trace-back
- History array upload with reindexing (CPU layout -> GPU layout)
- `DetailedRouter` trait implementation
- **Files**: `crates/autopcb-router/src/gpu/gamer.rs`

### Phase 5: Auto-Selection + Config
- `GpuBackendConfig` enum in `RoutingConfig`
- `select_backend()` heuristic
- Feature-gated `wgpu` dependency
- CLI flag for backend selection
- **Files**: `crates/autopcb-router/src/config.rs`, `crates/autopcb-router/Cargo.toml`

### Phase 6: Benchmarking + Tuning
- Benchmark suite: varying grid sizes, obstacle densities, layer counts
- Workgroup size tuning via override constants
- V-sweep coalescing optimization (transpose) if needed
- Document crossover points

---

## 10. Open Questions

1. **Parallel prefix min with non-uniform costs**: The basic GAMER scan assumes
   uniform step cost (all edges cost 1). With history and congestion costs,
   each edge has a different weight. The prefix min becomes:
   `dist[i] = min(dist[i], dist[i-1] + cost[i])` where `cost[i]` varies.
   This is a segmented parallel scan, still O(log n) but with higher constants.
   Need to benchmark whether the cost overhead is acceptable.

2. **Predecessor tracking in sweeps**: During a prefix min scan, updating the
   predecessor requires knowing which propagation path led to the minimum.
   In a parallel prefix reduction, the predecessor must be carried alongside
   the distance through the reduction tree. This doubles the shared memory
   requirement per row/column.

3. **Diagonal movement**: GAMER's H/V decomposition naturally handles Manhattan
   (4-way) movement. For 8-way movement (diagonal), an additional diagonal
   sweep pass would be needed, or diagonal moves can be handled by the via
   transition pass (treating diagonal as a "pseudo-via" with cost sqrt(2)).
   The current `MovementStyle::EightWay` in `config.rs` would need special
   handling.

4. **Workgroup size vs row width**: If grid width exceeds the maximum workgroup
   shared memory (16 KB / 8 bytes per element = 2048 cells), the row must be
   split into tiles. This adds complexity for inter-tile communication. For
   boards wider than 200mm at 0.1mm resolution (2000 cells), this may be needed.
   Alternative: use 0.2mm resolution for the GAMER sweep and interpolate.

---

## References

- GAMER: GPU-Accelerated Maze Routing -- https://ieeexplore.ieee.org/document/9799536
- Corolla: GPU-Accelerated FPGA Routing -- https://dl.acm.org/doi/10.1145/3020078.3021732
- Blelloch parallel prefix sum -- https://developer.nvidia.com/gpugems/gpugems3/part-vi-gpu-computing/chapter-39-parallel-prefix-sum-scan-cuda
- wgpu Implementation Patterns -- `docs/notes/autorouter-gpu/05-wgpu-implementation.md`
- GPU Acceleration Research Report -- `docs/plans/router/gpu-acceleration-research.md`
- Router Plan (Milestones 5-7) -- `docs/plans/router/README.md`
- PathFinder (McMurchie & Ebeling 1995) -- https://dl.acm.org/doi/10.1145/201310.201328

---

## See Also

| Plan | Role | Relationship to GAMER |
|------|------|----------------------|
| **01 — Corolla** (`01-corolla-bellman-ford.md`) | Alternative GPU SSSP backend | Interchangeable at step 3. Defines `GpuRoutingEngine` (see that plan for the full struct). Corolla is preferred for small subnets; GAMER for large. Same integration points. |
| **03 — X-Check** (`03-xcheck-gpu-drc.md`) | GPU DRC, runs after step 3 | Consumes `segment_buffer` filled by GAMER/Corolla. Writes DRC violation penalties back to `history_costs`. |
| **04 — Cypress** (`04-cypress-congestion-feedback.md`) | Post-routing congestion feedback | Independent of GAMER at the per-iteration level. Reads final `history_costs` after routing converges. |
| **05 — InstantGR** (`05-instantgr-net-batching.md`) | Net batching, runs before step 3 | Partitions nets into batches that GAMER routes simultaneously. Owns the interleaved buffer layout that GAMER uses for multi-net parallel sweeps. |
