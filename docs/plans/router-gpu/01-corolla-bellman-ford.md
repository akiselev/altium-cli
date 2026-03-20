# GPU-Accelerated PathFinder Routing: Corolla Bellman-Ford Implementation Plan

## Overview

This plan details the implementation of `GpuGridRouter`, a GPU-accelerated
detailed routing backend that replaces the CPU A*-based `GridRouter` for the
inner single-source shortest-path (SSSP) computation within the PathFinder
negotiation loop. The approach is based on the Corolla paper (Shen & Luo, FPGA
2017), adapted from FPGA routing resource graphs to PCB routing grids.

**Key insight from Corolla**: Bellman-Ford is inherently GPU-friendly (no
priority queue, all edges relaxed in parallel via `atomicMin`) but has worse
worst-case complexity than Dijkstra/A*. Corolla solves this by running
Bellman-Ford on a *subgraph* extracted around each net's pin bounding box,
keeping the active vertex count small enough that BF converges fast.

**Key adaptation for PCB**: Our routing grid is a regular 3D lattice
`(x, y, layer)` with uniform connectivity (4-way or 8-way + via transitions),
which is far more regular than an FPGA routing resource graph. This regularity
makes GPU parallelization more efficient: workgroup dispatch maps directly to
grid tiles, memory access is coalesced, and the graph structure is implicit
(no adjacency list needed).

### What changes vs the existing router

| Component | Before (CPU) | After (GPU) |
|-----------|-------------|-------------|
| SSSP algorithm | A* via `pathfinding::astar` | Bellman-Ford via wgpu compute shaders |
| Data structure | Priority queue (binary heap) | Flat distance/predecessor arrays + `atomicMin` |
| Search space | Full grid (guided by heuristic) | Subgraph around net bbox (Corolla extraction) |
| Execution model | Sequential per-net | CPU orchestrates outer loop, GPU dispatches BF inner loop |
| Cost encoding | `f64` | Fixed-point `u32` (scale factor 1024, 0.001 precision, max ~4.2M) |
| History array | `Vec<f64>` in `HistoryArray` | `wgpu::Buffer` of `u32` (fixed-point), same linearization |

---

## Pipeline Integration

Corolla (this plan) implements the GPU SSSP kernel — the innermost compute step of each PathFinder iteration.

```
PathFinder Iteration:
  1. Rip-up (CPU)
  2. InstantGR (05) → batch nets into independent groups
  3. For each batch:
     Corolla (01) [this plan] OR GAMER (02) → GPU SSSP per net in batch
  4. X-Check (03) → GPU DRC, violations → history
  5. History update (GPU kernel)
  6. Convergence check (CPU)

After routing:
  Cypress (04) → congestion feedback → placement SA
```

**This plan's role**: Corolla BF is one of two GPU SSSP backends (step 3). It receives a batch of nets from InstantGR (05), runs Bellman-Ford on each net's subgraph using the shared `GpuRoutingEngine`, and returns `Vec<PathSegment>` per net. X-Check (03) consumes the routed segments afterward.

### Shared Buffer Access

| Buffer | Access | Notes |
|--------|--------|-------|
| `obstacle_bitmap` | Read | Built by workspace, never modified during routing |
| `history_costs` | Read | Written by `history_update` kernel and X-Check DRC violations |
| `distance` | Read/Write | Reset per net/batch; BF writes via `atomicMin` |
| `predecessor` | Write | BF writes parent direction; traced back on CPU |
| `routing_params` | Read | Grid dims, costs, source/target per net |
| `clearance_matrix` | Read | Per-net-class clearance lookup used by DRC (03) |

---

## Architecture

### How GpuGridRouter Slots Into the Existing Codebase

`GpuGridRouter` implements the existing `DetailedRouter` trait from
`crates/autopcb-router/src/detailed/grid.rs`:

```rust
pub trait DetailedRouter {
    fn route_subnet(
        &self,
        workspace: &RoutingWorkspace,
        subnet: &crate::global::steiner::Subnet,
        net_id: NetId,
        history_costs: Option<&[f64]>,
    ) -> Result<Vec<PathSegment>, RoutingError>;
}
```

The PathFinder loop in `crates/autopcb-router/src/pathfinder/mod.rs` currently
instantiates a `GridRouter` (CPU A*). It will be extended with a runtime
selection:

```rust
// crates/autopcb-router/src/pathfinder/mod.rs
let router: Box<dyn DetailedRouter> = if config.gpu_enabled {
    Box::new(GpuGridRouter::new(workspace, config)?)
} else {
    Box::new(GridRouter::new(via_cost, config.movement))
};
```

### Module Structure

All five GPU router plans share a single module:

```
crates/autopcb-router/src/gpu/
├── mod.rs              // GpuRoutingEngine (shared device, queue, buffers, pipelines)
├── engine.rs           // GpuRoutingEngine struct, initialization, buffer management
├── buffers.rs          // Buffer types, layout, upload/download helpers
├── bellman_ford.rs     // Corolla BF dispatch (01) [this file's dispatch logic]
├── sweep.rs            // GAMER H/V sweep dispatch (02)
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

### Shared `GpuRoutingEngine`

All five GPU routing algorithms share a single engine instance, created once per routing invocation and reused across all PathFinder iterations:

```rust
/// Shared GPU infrastructure for all routing algorithms.
/// Created once per routing invocation, reused across all PathFinder iterations.
pub struct GpuRoutingEngine {
    // wgpu primitives
    device: wgpu::Device,
    queue: wgpu::Queue,

    // Persistent buffers (lifetime = full routing run)
    obstacle_bitmap: wgpu::Buffer,   // per-layer obstacles, read-only after build
    history_costs: wgpu::Buffer,     // PathFinder history, updated per iteration
    clearance_matrix: wgpu::Buffer,  // per-net-class-pair clearances, read-only
    routing_params: wgpu::Buffer,    // uniform: grid dims, costs, etc.

    // Per-batch buffers (reset between batches)
    distance: wgpu::Buffer,          // atomic<u32>, interleaved for batch routing
    predecessor: wgpu::Buffer,       // u32, interleaved for batch routing

    // DRC buffers
    segment_buffer: wgpu::Buffer,    // routed segments for DRC
    violation_buffer: wgpu::Buffer,  // DRC violation output

    // Congestion buffers (for Cypress)
    congestion_grid: wgpu::Buffer,   // demand/capacity per cell

    // Compiled pipelines (created once, reused)
    bf_pipeline: wgpu::ComputePipeline,
    sweep_h_pipeline: wgpu::ComputePipeline,
    sweep_v_pipeline: wgpu::ComputePipeline,
    via_transition_pipeline: wgpu::ComputePipeline,
    reset_pipeline: wgpu::ComputePipeline,
    convergence_pipeline: wgpu::ComputePipeline,
    history_update_pipeline: wgpu::ComputePipeline,
    drc_sweepline_pipeline: wgpu::ComputePipeline,
    drc_short_pipeline: wgpu::ComputePipeline,
    congestion_rudy_pipeline: wgpu::ComputePipeline,
    congestion_overflow_pipeline: wgpu::ComputePipeline,

    // Configuration
    grid_config: GpuGridConfig,
    max_batch_size: u32,
}
```

The `GpuRoutingEngine` is defined in `gpu/engine.rs` and constructed in `gpu/mod.rs`. Plans 02–05 reference this struct by name and list only the fields they use.

### GPU Device/Queue Initialization

Two paths, determined at `GpuGridRouter::new()` time:

**Path 1 - Headless (CLI `altium routing solve`)**:
```rust
// crates/autopcb-router/src/gpu/device.rs
pub async fn create_headless_device() -> Result<(wgpu::Device, wgpu::Queue), RoutingError> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        flags: wgpu::InstanceFlags::VALIDATION,
        ..Default::default()
    });
    let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,          // headless: no surface
        force_fallback_adapter: false,
    }).await
    .ok_or_else(|| RoutingError::RoutingFailed("no GPU adapter found".into()))?;

    let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("autopcb-router"),
        required_features: wgpu::Features::empty(), // no special features needed
        required_limits: wgpu::Limits {
            max_storage_buffer_binding_size: 256 * 1024 * 1024, // 256 MiB
            max_buffer_size: 256 * 1024 * 1024,
            ..wgpu::Limits::default()
        },
        memory_hints: wgpu::MemoryHints::Performance,
    }, None).await
    .map_err(|e| RoutingError::RoutingFailed(format!("GPU device request failed: {e}")))?;

    Ok((device, queue))
}
```

**Path 2 - Shared with viewer (future, when routing runs inside the viewer)**:
The viewer in `crates/autopcb-viewer/src/view3d.rs` gets its `wgpu::Device` via
egui-wgpu's `RenderState`. The GPU router could accept an `Arc<wgpu::Device>`
and `Arc<wgpu::Queue>` to share the device. This is deferred -- headless-first
is simpler and avoids lifecycle issues.

### RoutingConfig Extension

Add to `crates/autopcb-router/src/config.rs`:

```rust
/// Enable GPU-accelerated Bellman-Ford routing. Falls back to CPU A* if no
/// GPU adapter is available. Default false.
#[serde(default)]
pub gpu_enabled: bool,

/// Number of Bellman-Ford iterations between convergence checks. Higher
/// values reduce CPU-GPU round-trips but may overshoot. Default 8.
#[serde(default = "default_bf_batch_size")]
pub bf_batch_size: u32,

/// Initial subgraph expansion factor (Corolla Delta_C). The net pin
/// bounding box is expanded by `expansion_factor * sqrt(grid_area)` on
/// each side. Default 0.021 (from Corolla Table 1).
#[serde(default = "default_subgraph_expansion_factor")]
pub subgraph_expansion_factor: f64,

/// Dynamic expansion increment in grid cells (Corolla Delta_D). When a
/// routed path touches the subgraph boundary, the subgraph is expanded
/// by this many cells on each side for the next iteration. Default 1.
#[serde(default = "default_subgraph_dynamic_increment")]
pub subgraph_dynamic_increment: u32,
```

---

## Buffer Layout for the Routing Grid

All GPU buffers use Structure-of-Arrays (SoA) layout for coalesced memory
access (as specified in `docs/notes/autorouter-gpu/05-wgpu-implementation.md`
section 1.1).

### Linearization

GPU uses layer-major, row-major order for coalesced access:

```
index = layer * (width * height) + y * width + x
```

This differs from the CPU's `HistoryArray` which uses x-major order:

```
index = x * (height * layer_count) + y * layer_count + layer
```

A remapping function converts between CPU and GPU index spaces during
upload/download. Since the history array is the only shared data structure
between CPU PathFinder state and GPU buffers, this is a single conversion
point.

### Buffer Inventory

| Buffer | Type | Size (cells) | Access | Changes per... | Purpose |
|--------|------|-------------|--------|----------------|---------|
| `distance_buf` | `storage<read_write>` atomic u32 | subgraph cells | GPU R/W | Reset per net | Fixed-point shortest distances from source |
| `predecessor_buf` | `storage<read_write>` u32 | subgraph cells | GPU R/W, CPU read | Reset per net | Packed `(x, y, layer)` of parent in shortest path tree |
| `obstacle_buf` | `storage<read>` u32 | `width * height` | GPU R | Per workspace build | Per-cell layer bitmask: bit `i` = blocked on layer `i` |
| `history_buf` | `storage<read>` u32 | full grid cells | GPU R | Per PF iteration | Fixed-point history congestion costs |
| `occupancy_buf` | `storage<read_write>` u32 | full grid cells | GPU R/W | Per net route/rip-up | Per-cell net count (for conflict detection) |
| `change_flag_buf` | `storage<read_write>` atomic u32 | 1 | GPU R/W, CPU read | Per BF batch | Convergence detection: 0 = no changes |
| `params_buf` | `uniform` | 1 struct (64B) | GPU R | Per net | Grid dimensions, costs, source/target cells |

### Fixed-Point u32 Encoding

Costs are encoded as `u32` with scale factor 1024:

```
fp_value = (f64_cost * 1024.0).round() as u32
```

- Precision: 1/1024 ~= 0.001
- Maximum representable cost: `u32::MAX / 1024 = 4,194,303`
- INFINITY sentinel: `0xFFFF_FFFF` (u32::MAX)
- NONE predecessor sentinel: `0xFFFF_FFFF`

Scale factor 1024 (2^10) is chosen because:
1. Shift-based multiplication is faster than arbitrary division on GPU
2. 0.001 precision is more than adequate for routing costs (base move cost = 1.0 = 1024 in FP)
3. Maximum cost of ~4.2M far exceeds any realistic path cost (1000x1000 grid diagonal = ~2000 moves = 2M in FP)

### Predecessor Encoding

Pack `(x, y, layer)` into a single `u32`:

```
bits [0..11]  = x      (supports grids up to 2048)
bits [11..22] = y      (supports grids up to 2048)
bits [22..26] = layer  (supports up to 16 layers)
bits [26..31] = reserved
```

For boards requiring larger grids (>2048 cells per dimension), the bit
allocation can be widened. PCB boards rarely exceed 1000x1000 at 0.1mm
resolution (100mm x 100mm = 4" x 4"), so 2048 provides ample headroom. A
24" board at 0.1mm resolution = 6096 cells; this would require 13 bits for
x and y. In that case, use: `x[0..13], y[13..26], layer[26..30]`.

### GridParams Uniform Buffer

```rust
// crates/autopcb-router/src/gpu/buffers.rs
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GridParams {
    pub width: u32,              // subgraph width in cells
    pub height: u32,             // subgraph height in cells
    pub layer_count: u32,        // number of copper layers
    pub total_cells: u32,        // width * height * layer_count

    pub source_cell: u32,        // linearized source index (GPU order)
    pub target_cell: u32,        // linearized target index (GPU order)
    pub base_cost_fp: u32,       // fixed-point base move cost (1024 = 1.0)
    pub via_cost_fp: u32,        // fixed-point via transition cost

    pub pres_fac_fp: u32,        // fixed-point present congestion factor
    pub history_fac_fp: u32,     // fixed-point history factor multiplier
    pub diagonal_cost_fp: u32,   // fixed-point diagonal cost (1448 = sqrt(2) * 1024)
    pub movement_four_way: u32,  // 1 = 4-way, 0 = 8-way

    // Subgraph offset within the full grid (for history buffer indexing)
    pub subgraph_origin_x: u32,  // x offset of subgraph in full grid
    pub subgraph_origin_y: u32,  // y offset of subgraph in full grid
    pub full_grid_width: u32,    // full grid width (for history index calculation)
    pub full_grid_height: u32,   // full grid height
}
// Total: 64 bytes (4 x vec4<u32> = 16 u32s), aligned to 16 bytes
```

---

## WGSL Shaders

### Shared Preamble: `types.wgsl`

```wgsl
// ---- Fixed-point constants ----
const FP_SCALE: u32 = 1024u;
const FP_INFINITY: u32 = 0xFFFFFFFFu;
const PRED_NONE: u32 = 0xFFFFFFFFu;

// ---- Grid params (uniform) ----
struct GridParams {
    width: u32,
    height: u32,
    layer_count: u32,
    total_cells: u32,
    source_cell: u32,
    target_cell: u32,
    base_cost_fp: u32,
    via_cost_fp: u32,
    pres_fac_fp: u32,
    history_fac_fp: u32,
    diagonal_cost_fp: u32,
    movement_four_way: u32,
    subgraph_origin_x: u32,
    subgraph_origin_y: u32,
    full_grid_width: u32,
    full_grid_height: u32,
}

@group(0) @binding(0) var<storage, read_write> distance: array<atomic<u32>>;
@group(0) @binding(1) var<storage, read_write> predecessor: array<u32>;
@group(0) @binding(2) var<storage, read>       obstacle: array<u32>;
@group(0) @binding(3) var<storage, read>       history: array<u32>;
@group(1) @binding(0) var<uniform>             params: GridParams;
@group(1) @binding(1) var<storage, read_write> change_flag: atomic<u32>;

// ---- Helper: GPU-order linearization ----
fn cell_index(x: u32, y: u32, layer: u32) -> u32 {
    return layer * (params.width * params.height) + y * params.width + x;
}

// ---- Helper: decode cell index back to (x, y, layer) ----
fn decode_cell(idx: u32) -> vec3<u32> {
    let plane_size = params.width * params.height;
    let layer = idx / plane_size;
    let rem = idx % plane_size;
    let y = rem / params.width;
    let x = rem % params.width;
    return vec3<u32>(x, y, layer);
}

// ---- Helper: check if cell is blocked on a layer ----
fn is_blocked(x: u32, y: u32, layer: u32) -> bool {
    // obstacle buffer is 2D (one u32 bitmask per cell, layer packed in bits)
    let idx_2d = y * params.width + x;
    return (obstacle[idx_2d] & (1u << layer)) != 0u;
}

// ---- Helper: get history cost for a subgraph cell ----
// Maps subgraph-local (x, y, layer) to full-grid history index
fn get_history(x: u32, y: u32, layer: u32) -> u32 {
    let full_x = x + params.subgraph_origin_x;
    let full_y = y + params.subgraph_origin_y;
    // CPU linearization: x * (height * layers) + y * layers + layer
    let idx = full_x * (params.full_grid_height * params.layer_count)
            + full_y * params.layer_count
            + layer;
    return history[idx];
}

// ---- Helper: encode predecessor ----
fn encode_pred(x: u32, y: u32, layer: u32) -> u32 {
    return x | (y << 11u) | (layer << 22u);
}
```

### `reset.wgsl` -- Initialize Distance Array

**Purpose**: Set all distances to INFINITY and predecessors to NONE. Set
source cell distance to 0.

**Workgroup size**: `(64, 1, 1)` -- 1D dispatch over total cells.

**Dispatch**: `ceil(total_cells / 64)` workgroups in x.

**Algorithm**:
1. Compute global thread ID.
2. If ID >= total_cells, return (boundary guard).
3. Set `distance[id] = FP_INFINITY`.
4. Set `predecessor[id] = PRED_NONE`.
5. If `id == source_cell`, set `distance[id] = 0`.

```wgsl
@compute @workgroup_size(64)
fn reset_distance(@builtin(global_invocation_id) gid: vec3<u32>) {
    let id = gid.x;
    if id >= params.total_cells { return; }

    atomicStore(&distance[id], FP_INFINITY);
    predecessor[id] = PRED_NONE;

    if id == params.source_cell {
        atomicStore(&distance[id], 0u);
    }
}
```

### `bellman_ford.wgsl` -- Parallel Edge Relaxation

**Purpose**: Each thread processes one cell. For each neighbor of that cell,
attempt to relax the edge: if `my_dist + edge_cost < neighbor_dist`, update
neighbor's distance via `atomicMin` and write predecessor.

**Workgroup size**: `(8, 8, 1)` -- 2D dispatch over `(width, height)`, one
dispatch per layer (z = layer index in dispatch).

**Dispatch**: `(ceil(width/8), ceil(height/8), layer_count)`.

**Algorithm pseudocode**:
```
for each cell (x, y, layer) assigned to this thread:
    if cell is out of bounds or blocked: skip
    my_dist = atomicLoad(distance[cell_index])
    if my_dist == FP_INFINITY: skip (unreachable cell)

    for each neighbor (nx, ny, nlayer) of (x, y, layer):
        if (nx, ny, nlayer) is out of bounds or blocked: skip

        // Compute edge cost
        edge_cost = base_cost
        if same layer:
            if diagonal: edge_cost = diagonal_cost
            // Add direction penalty (simplified: all directions equal for now)
        else:
            edge_cost = via_cost  // layer transition

        // Add history cost (from full-grid history buffer)
        hist = get_history(nx, ny, nlayer)
        edge_cost += (hist * pres_fac_fp) / FP_SCALE

        new_dist = my_dist + edge_cost
        old_dist = atomicMin(&distance[neighbor_index], new_dist)

        if new_dist < old_dist:
            predecessor[neighbor_index] = encode_pred(x, y, layer)
            atomicStore(&change_flag, 1u)
```

**Predecessor race condition**: Multiple threads may `atomicMin` the same
neighbor simultaneously. If two threads produce the same minimum distance,
the predecessor write is a race. This is acceptable: the *distance* is
deterministic (atomicMin guarantees the minimum wins), but the *predecessor*
(and thus the path) may vary between runs. The path cost is always correct.

**WGSL outline**:

```wgsl
@compute @workgroup_size(8, 8, 1)
fn bellman_ford_relax(@builtin(global_invocation_id) gid: vec3<u32>) {
    let x = gid.x;
    let y = gid.y;
    let layer = gid.z;

    if x >= params.width || y >= params.height || layer >= params.layer_count {
        return;
    }
    if is_blocked(x, y, layer) { return; }

    let idx = cell_index(x, y, layer);
    let my_dist = atomicLoad(&distance[idx]);
    if my_dist == FP_INFINITY { return; }

    // Cardinal neighbors: (x-1,y), (x+1,y), (x,y-1), (x,y+1)
    let offsets = array<vec2<i32>, 4>(
        vec2<i32>(-1, 0), vec2<i32>(1, 0),
        vec2<i32>(0, -1), vec2<i32>(0, 1)
    );

    for (var i = 0u; i < 4u; i = i + 1u) {
        let nx = i32(x) + offsets[i].x;
        let ny = i32(y) + offsets[i].y;
        if nx < 0 || ny < 0 || u32(nx) >= params.width || u32(ny) >= params.height {
            continue;
        }
        let unx = u32(nx);
        let uny = u32(ny);
        if is_blocked(unx, uny, layer) { continue; }

        let nidx = cell_index(unx, uny, layer);
        let hist = get_history(unx, uny, layer);
        let edge_cost = params.base_cost_fp + (hist * params.pres_fac_fp) / FP_SCALE;
        let new_dist = my_dist + edge_cost;

        let old = atomicMin(&distance[nidx], new_dist);
        if new_dist < old {
            predecessor[nidx] = encode_pred(x, y, layer);
            atomicStore(&change_flag, 1u);
        }
    }

    // 8-way diagonal neighbors (if enabled)
    if params.movement_four_way == 0u {
        let diag_offsets = array<vec2<i32>, 4>(
            vec2<i32>(-1, -1), vec2<i32>(1, -1),
            vec2<i32>(-1, 1), vec2<i32>(1, 1)
        );
        for (var i = 0u; i < 4u; i = i + 1u) {
            let nx = i32(x) + diag_offsets[i].x;
            let ny = i32(y) + diag_offsets[i].y;
            if nx < 0 || ny < 0 || u32(nx) >= params.width || u32(ny) >= params.height {
                continue;
            }
            let unx = u32(nx);
            let uny = u32(ny);
            if is_blocked(unx, uny, layer) { continue; }

            let nidx = cell_index(unx, uny, layer);
            let hist = get_history(unx, uny, layer);
            let edge_cost = params.diagonal_cost_fp + (hist * params.pres_fac_fp) / FP_SCALE;
            let new_dist = my_dist + edge_cost;

            let old = atomicMin(&distance[nidx], new_dist);
            if new_dist < old {
                predecessor[nidx] = encode_pred(x, y, layer);
                atomicStore(&change_flag, 1u);
            }
        }
    }

    // Via transitions to all other layers
    for (var l = 0u; l < params.layer_count; l = l + 1u) {
        if l == layer { continue; }
        if is_blocked(x, y, l) { continue; }

        let nidx = cell_index(x, y, l);
        let hist = get_history(x, y, l);
        let edge_cost = params.via_cost_fp + (hist * params.pres_fac_fp) / FP_SCALE;
        let new_dist = my_dist + edge_cost;

        let old = atomicMin(&distance[nidx], new_dist);
        if new_dist < old {
            predecessor[nidx] = encode_pred(x, y, layer);
            atomicStore(&change_flag, 1u);
        }
    }
}
```

### `convergence.wgsl` -- Detect If Any Distances Changed

No separate shader needed. The `change_flag` atomic is written by the BF
kernel. The CPU reads it back after each batch of BF iterations:

```rust
// CPU-side (crates/autopcb-router/src/gpu/dispatch.rs)
fn is_converged(device: &wgpu::Device, change_flag_staging: &wgpu::Buffer) -> bool {
    change_flag_staging.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    device.poll(wgpu::Maintain::Wait);
    let data = change_flag_staging.slice(..).get_mapped_range();
    let flag = u32::from_le_bytes(data[0..4].try_into().unwrap());
    drop(data);
    change_flag_staging.unmap();
    flag == 0
}
```

### `history_update.wgsl` -- Update History Costs

**Purpose**: After a full PathFinder iteration (all nets routed), increment
history costs for cells where `occupancy > 1` (oversubscribed).

**Workgroup size**: `(64, 1, 1)` -- 1D dispatch over full grid cells.

**Algorithm**:
```
for each cell (x, y, layer):
    if occupancy[cell] > 1:
        history[cell] += history_increment_fp
```

```wgsl
@group(0) @binding(4) var<storage, read>       occupancy: array<u32>;
@group(0) @binding(3) var<storage, read_write>  history_rw: array<u32>;

@compute @workgroup_size(64)
fn update_history(@builtin(global_invocation_id) gid: vec3<u32>) {
    let id = gid.x;
    if id >= params.total_cells { return; }

    if occupancy[id] > 1u {
        history_rw[id] = history_rw[id] + params.history_fac_fp;
    }
}
```

Note: `history_update.wgsl` uses a *different* bind group from the BF kernel
because the history buffer must be `read_write` here but is `read` in BF.
This requires two bind group layouts, or rebinding with a different access mode.
The simplest approach: define two bind groups and swap between BF and history
update dispatches.

### `rip_up.wgsl` -- Clear Occupancy for Ripped-Up Nets

**Purpose**: When a net is ripped up, decrement the occupancy count for all
cells that net occupied.

**Implementation choice**: This is simpler on CPU. The CPU traces the net's
path (a short list of `PathSegment`s, typically 10-100 cells) and writes
occupancy decrements. Uploading a rip-up worklist to GPU and dispatching a
kernel for 10-100 cells adds more overhead than it saves.

**Decision**: Rip-up is CPU-side. The CPU maintains occupancy counts in
`HashMap<(u32, u32, u16), u16>` (cell -> net count) and uploads the updated
occupancy buffer to the GPU before each BF dispatch batch.

If profiling shows occupancy upload is a bottleneck (unlikely for typical PCB
boards), a GPU-side rip-up kernel can be added later using an indirect dispatch
over a worklist buffer.

---

## Corolla-Specific Techniques

### Subgraph Extraction

**Concept**: Instead of running Bellman-Ford over the entire routing grid
(potentially 1000x1000x4 = 4M cells), extract a rectangular subgraph around
each net's pin bounding box. For a typical net spanning 10mm on a 0.1mm grid,
the subgraph is ~100x100x4 = 40K cells -- 100x smaller.

**Implementation** (`crates/autopcb-router/src/gpu/subgraph.rs`):

```rust
pub struct SubgraphRegion {
    /// Top-left corner of the subgraph in full-grid coordinates.
    pub origin_x: u32,
    pub origin_y: u32,
    /// Dimensions of the subgraph.
    pub width: u32,
    pub height: u32,
}

/// Extract a subgraph region for a subnet using the Corolla initial coverage
/// strategy.
///
/// Expansion: bbox of net pins, expanded by Delta_C on all sides.
/// Delta_C = ceil(expansion_factor * sqrt(grid_width * grid_height))
///
/// Corolla Table 1 gives expansion_factor = 0.021 for 98.5% coverage.
pub fn extract_subgraph(
    grid: &GridConfig,
    subnet: &Subnet,
    config: &RoutingConfig,
) -> SubgraphRegion {
    let (src_gx, src_gy) = grid.to_grid(subnet.source);
    let (tgt_gx, tgt_gy) = grid.to_grid(subnet.target);

    let min_x = src_gx.min(tgt_gx);
    let max_x = src_gx.max(tgt_gx);
    let min_y = src_gy.min(tgt_gy);
    let max_y = src_gy.max(tgt_gy);

    let grid_area = (grid.width_cells as f64) * (grid.height_cells as f64);
    let delta_c = (config.subgraph_expansion_factor * grid_area.sqrt()).ceil() as u32;

    SubgraphRegion {
        origin_x: min_x.saturating_sub(delta_c),
        origin_y: min_y.saturating_sub(delta_c),
        width: (max_x - min_x + 2 * delta_c + 1).min(grid.width_cells),
        height: (max_y - min_y + 2 * delta_c + 1).min(grid.height_cells),
    }
}
```

### Dynamic Expansion

Per Corolla Section 4.3: if the routed path touches the subgraph boundary
(any predecessor cell is on the boundary row/column), expand the subgraph by
`Delta_D` cells (default 1) on all sides and re-route.

**Detection**: After CPU path traceback, check if any cell in the path has
`x == 0`, `x == subgraph.width - 1`, `y == 0`, or `y == subgraph.height - 1`
(in subgraph-local coordinates).

```rust
fn needs_expansion(path: &[GridNode], subgraph: &SubgraphRegion) -> bool {
    path.iter().any(|node| {
        let local_x = node.x - subgraph.origin_x;
        let local_y = node.y - subgraph.origin_y;
        local_x == 0 || local_x == subgraph.width - 1
            || local_y == 0 || local_y == subgraph.height - 1
    })
}

fn expand_subgraph(
    subgraph: &mut SubgraphRegion,
    delta_d: u32,
    grid: &GridConfig,
) {
    subgraph.origin_x = subgraph.origin_x.saturating_sub(delta_d);
    subgraph.origin_y = subgraph.origin_y.saturating_sub(delta_d);
    subgraph.width = (subgraph.width + 2 * delta_d).min(grid.width_cells - subgraph.origin_x);
    subgraph.height = (subgraph.height + 2 * delta_d).min(grid.height_cells - subgraph.origin_y);
}
```

**Re-routing after expansion**: The subgraph buffers (distance, predecessor)
are re-allocated (or the existing buffers are large enough if pre-sized to
the maximum expected expansion), obstacle data is re-uploaded for the new
region, and BF is re-run.

### Mapping to LLM-Declared Routing Corridors

The spec language supports routing constraints like:

```
routing {
    corridor net="USB_D+" layers=[1, 2] region="(10, 20) to (30, 40)"
}
```

These LLM-declared corridors map directly to Corolla subgraphs: the
corridor's region defines the initial subgraph bbox, superseding the
pin-bounding-box-based extraction. The dynamic expansion strategy still
applies if the corridor is too tight.

### SNP vs DEP: Which to Use

Corolla explores three parallelism strategies:
- **SNP** (Static Node Parallelism): Every node gets a thread, active or not.
- **DNP** (Dynamic Node Parallelism): Only active nodes get threads (worklist).
- **DEP** (Dynamic Edge Parallelism): Active edges get threads (prefix scan).

**Our choice: SNP with per-cell early exit.** Rationale:

1. WGSL/wgpu has no dynamic parallelism (cannot launch kernels from kernels).
   DNP and DEP require worklist management, which needs either shared-memory
   atomic worklists or CPU-driven indirect dispatch.
2. Our grid is regular -- SNP maps perfectly to a 2D dispatch over (x, y) with
   a third dimension for layers. Every thread checks one cell.
3. The early-exit pattern (`if my_dist == FP_INFINITY { return; }`) achieves
   most of DNP's benefit: cells that haven't been reached yet are skipped in
   O(1). Only cells within the wavefront do real work.
4. Corolla shows SNP achieves 4.15x average speedup alone (Figure 9), and the
   hybrid SNP+DEP gets 10.86x. The extra complexity of DEP on wgpu (no CUDA
   dynamic parallelism, 16 KiB workgroup memory limit) is not justified for
   PCB-scale graphs.
5. For nets with < 3 sinks (the vast majority in PCB routing, as in FPGA),
   SNP is faster than DEP (Corolla Figure 7b).

**Future optimization**: If profiling shows that large nets (many sinks,
spanning the full board) are bottlenecked by SNP's wasted threads, add an
active-flag buffer: each BF iteration writes a per-cell "active" flag if it
relaxed any neighbor, and the next iteration skips cells whose flag is 0.
This is a simple form of DNP without worklists.

---

## Integration with PathFinder Loop

### CPU Orchestrates Outer Loop, GPU Handles Inner SSSP

The PathFinder negotiation loop in `crates/autopcb-router/src/pathfinder/mod.rs`
remains CPU-driven. The GPU replaces only the `route_subnet()` call:

```
for iteration in 0..max_iterations:
    rip_up_all_nets()                          // CPU
    for net_id in net_order:
        for subnet in net.subnets:
            subgraph = extract_subgraph(subnet) // CPU
            loop:
                upload_params(source, target)   // CPU -> GPU
                reset_distance()                // GPU dispatch
                loop:                           // BF convergence loop
                    dispatch BF x bf_batch_size // GPU dispatches
                    check convergence           // CPU reads back flag
                    if converged: break
                readback predecessor array     // GPU -> CPU
                path = trace_back(predecessor) // CPU
                if needs_expansion(path):
                    expand_subgraph()
                    continue                   // re-route with larger subgraph
                else:
                    break
            update_occupancy(path)             // CPU
    update_history()                           // GPU dispatch (or CPU)
    if count_conflicts() == 0: break           // CPU
```

### Per-Net Dispatch Pattern

For each subnet:

1. **CPU**: Extract subgraph region, compute source/target in subgraph-local
   coordinates.
2. **CPU -> GPU**: Upload `GridParams` uniform (48 bytes via
   `queue.write_buffer`). Upload obstacle subset for subgraph region if not
   already cached.
3. **GPU**: Dispatch `reset` kernel: `ceil(total_subgraph_cells / 64)` workgroups.
4. **GPU**: Dispatch BF relaxation in batches of `bf_batch_size` (default 8):
   - Reset `change_flag` to 0 via `queue.write_buffer`.
   - Encode `bf_batch_size` compute passes in one command encoder.
   - Dispatch each as `(ceil(width/8), ceil(height/8), layer_count)`.
   - Copy `change_flag` to staging buffer.
   - Submit.
5. **CPU**: Read back `change_flag`. If 0, converged. If not, repeat step 4.
6. **CPU**: Read back predecessor array (subgraph-sized, not full grid).
7. **CPU**: Trace back path from target to source using predecessor chain.
8. **CPU**: Check for boundary touching; expand subgraph if needed, goto step 2.

### Amortized Convergence Detection

As detailed in `docs/notes/autorouter-gpu/05-wgpu-implementation.md` section
2.2: run `bf_batch_size` BF iterations between CPU readbacks of the change
flag. This reduces CPU-GPU round trips from ~D (grid diameter) to ~D/8.

For a 100x100 subgraph (typical net), D~200, so ~25 readbacks instead of ~200.
Each readback costs ~50-200us (staging buffer map + poll), so this saves
~35ms per net.

**Tuning**: `bf_batch_size = 8` is a good default. For very small subgraphs
(<50x50), 4 is better. For large subgraphs (>500x500), 16 is better. The
`RoutingConfig` exposes this as a tunable.

### Path Reconstruction: CPU

Path reconstruction (tracing the predecessor chain from target to source) is
inherently sequential and fast on CPU (~1us for a 100-cell path). GPU-side
path reconstruction is possible but complex and offers no benefit for our
path lengths.

After readback of the predecessor array (subgraph-sized):

```rust
fn trace_back(
    predecessors: &[u32],
    source_idx: u32,
    target_idx: u32,
    subgraph: &SubgraphRegion,
) -> Vec<GridNode> {
    let mut path = Vec::new();
    let mut cell = target_idx;
    while cell != source_idx {
        let pred = predecessors[cell as usize];
        if pred == PRED_NONE {
            // No path found (target unreachable within subgraph)
            return Vec::new();
        }
        let x = pred & 0x7FF;
        let y = (pred >> 11) & 0x7FF;
        let layer = (pred >> 22) & 0xF;
        path.push(GridNode {
            x: x + subgraph.origin_x,
            y: y + subgraph.origin_y,
            layer: LayerId(layer as u16),
        });
        cell = cell_index_from_pred(x, y, layer, subgraph);
    }
    path.reverse();
    path
}
```

---

## PcbIr Extensions Needed

The existing PcbIr types are already well-suited for GPU routing. The
following fields are present and sufficient:

| Routing Need | PcbIr Source | File | Status |
|-------------|-------------|------|--------|
| Net connectivity (pins + positions) | `IrNet.pins: Vec<IrNetPin>` | `crates/autopcb-ir/src/net.rs` | Present |
| Pad positions | `IrComponentPad.world_position: PointMm` | `crates/autopcb-ir/src/component.rs` | Present |
| Pad layer set | `IrComponentPad.layer_set: Vec<LayerId>` | `crates/autopcb-ir/src/component.rs` | Present |
| Layer stack | `IrLayerStack.copper_layers: Vec<IrCopperLayer>` | `crates/autopcb-ir/src/layer_stack.rs` | Present |
| Preferred direction | `IrCopperLayer.preferred_direction: Option<PreferredDirection>` | `crates/autopcb-ir/src/layer_stack.rs` | Present |
| Obstacles (pads) | `IrComponent.pads + PadShapeInfo` | `crates/autopcb-ir/src/component.rs` | Present |
| Keepouts | `IrBoardGeometry.keepouts: Vec<IrKeepoutZone>` | `crates/autopcb-ir/src/board.rs` | Present |
| Pre-routed traces | `IrTrack.locked + .pre_routed` | `crates/autopcb-ir/src/copper.rs` | Present |
| Pre-routed vias | `IrVia.locked + .pre_routed + .from_layer + .to_layer` | `crates/autopcb-ir/src/copper.rs` | Present |
| Design rules (clearance, width) | `IrDesignRule + IrRuleParams::Clearance/Width/...` | `crates/autopcb-ir/src/rule.rs` | Present |
| Net class | `IrNet.net_class: Option<String>` | `crates/autopcb-ir/src/net.rs` | Present |
| Diff pair | `IrNet.diff_pair_partner: Option<NetId>` | `crates/autopcb-ir/src/net.rs` | Present |
| Board bounds | `IrBoardGeometry.bounds: BoundingBoxMm` | `crates/autopcb-ir/src/board.rs` | Present |

**No PcbIr extensions are needed for the GPU router.** All routing-critical
fields already exist (they were added in Milestone 2 of the router plan).

### New Types Needed in `autopcb-router`

| Type | File | Purpose |
|------|------|---------|
| `GpuGridRouter` | `crates/autopcb-router/src/gpu/mod.rs` | GPU routing backend implementing `DetailedRouter` |
| `GpuDevice` | `crates/autopcb-router/src/gpu/device.rs` | Wrapper for `wgpu::Device` + `wgpu::Queue` |
| `GpuBuffers` | `crates/autopcb-router/src/gpu/buffers.rs` | All GPU buffer handles + staging buffers |
| `GpuPipelines` | `crates/autopcb-router/src/gpu/pipelines.rs` | All compute pipeline handles + bind group layouts |
| `SubgraphRegion` | `crates/autopcb-router/src/gpu/subgraph.rs` | Corolla subgraph bounding box |
| `GridParams` | `crates/autopcb-router/src/gpu/buffers.rs` | `#[repr(C)] bytemuck::Pod` uniform data |

---

## Performance Estimates

### Memory Budget

| Board Size | Grid (0.1mm) | Layers | Cells | Distance | Predecessor | Obstacle | History | Total |
|-----------|-------------|--------|-------|----------|-------------|----------|---------|-------|
| 50x50mm (small) | 500x500 | 4 | 1M | 4 MB | 4 MB | 1 MB | 4 MB | ~13 MB |
| 100x100mm (medium) | 1000x1000 | 4 | 4M | 16 MB | 16 MB | 4 MB | 16 MB | ~52 MB |
| 200x200mm (large) | 2000x2000 | 8 | 32M | 128 MB | 128 MB | 16 MB | 128 MB | ~400 MB |

**Subgraph sizes** are much smaller. A net spanning 10mm on a 0.1mm grid with
`Delta_C = 2` cells expansion:

- Subgraph: ~104x104 = 10,816 cells per layer, x 4 layers = 43,264 total cells
- Distance: 43K * 4 = 169 KB
- Predecessor: 169 KB
- Total per-net GPU memory: ~340 KB

This means the GPU buffers can be pre-allocated at the maximum expected
subgraph size (e.g., 512x512x8 = 2M cells = 16 MB) and reused across nets
without reallocation.

### Expected BF Iterations Per Net

Bellman-Ford on a graph with diameter D converges in at most D iterations.
For a subgraph of width W and height H:

| Subgraph | Diameter | BF Iterations (worst) | BF Batches (batch=8) |
|----------|----------|----------------------|---------------------|
| 50x50 | ~100 | 100 | 13 |
| 100x100 | ~200 | 200 | 25 |
| 200x200 | ~400 | 400 | 50 |
| 500x500 | ~1000 | 1000 | 125 |

In practice, BF converges much faster than the theoretical worst case because:
1. The subgraph is a regular grid (short diameter relative to vertex count).
2. Most paths are nearly straight (small detour ratio).
3. The early-exit optimization (`my_dist == INFINITY: skip`) limits active
   cells to the wavefront.

Empirical expectation: convergence in 1-3x the path length (not the grid
diameter). For a 100-cell path on a 100x100 grid, ~100-300 BF iterations.

### When GPU Is Faster vs CPU

The GPU has two overheads the CPU does not:
1. **Per-net setup**: buffer upload, pipeline dispatch, staging readback.
   Approximately 0.5-2ms per net.
2. **BF iteration overhead vs A* efficiency**: A* visits O(path_length) cells
   with a good heuristic; BF visits O(subgraph_size) cells per iteration.

**Crossover analysis**:

| Factor | CPU A* | GPU BF |
|--------|--------|--------|
| Per-net setup | ~0 | ~1ms |
| Per-cell work per iteration | ~50ns (heap ops) | ~2ns (parallel relaxation) |
| Cells visited per "iteration" | ~1 (dequeue + expand) | subgraph_size (all cells) |
| Total iterations | path_length | ~2x path_length |
| Total cell-visits | ~path_length^2 (with obstacles) | ~subgraph_size * 2 * path_length |

For a 100-cell path on a 100x100x4 subgraph:
- CPU A*: ~10,000 cell-visits * 50ns = 0.5ms
- GPU BF: 40,000 cells * 200 iterations * 2ns = 16ms BUT each "iteration" is
  massively parallel (40K cells / 64 threads per WG = 625 WGs dispatched
  simultaneously)
- GPU wall time: ~200 dispatches * ~5us/dispatch = 1ms + 1ms setup = ~2ms

**Crossover**: GPU becomes faster than CPU when subgraph size exceeds ~200x200
(40K cells per layer), because the massive parallelism amortizes the setup
cost. For nets shorter than ~50 grid cells, CPU A* is faster.

**Strategy**: Use GPU for nets whose pin bounding box exceeds a threshold
(e.g., > 100x100 grid cells). Fall back to CPU A* for small nets. This is
analogous to Corolla's hybrid approach (SNP for small nets, DEP for large).

```rust
fn should_use_gpu(subnet: &Subnet, grid: &GridConfig, threshold: u32) -> bool {
    let (sx, sy) = grid.to_grid(subnet.source);
    let (tx, ty) = grid.to_grid(subnet.target);
    let bbox_width = sx.abs_diff(tx);
    let bbox_height = sy.abs_diff(ty);
    bbox_width > threshold || bbox_height > threshold
}
```

Default threshold: 100 cells (10mm at 0.1mm resolution).

---

## Testing Strategy

### CPU Reference Implementation

A pure-Rust Bellman-Ford implementation that produces identical distance arrays
to the GPU version. Uses the same fixed-point encoding, same linearization,
same cost function.

```rust
// crates/autopcb-router/src/gpu/cpu_reference.rs

/// CPU reference Bellman-Ford for testing GPU correctness.
///
/// Same algorithm, same cost function, same fixed-point encoding.
/// Returns (distance, predecessor) arrays with GPU linearization order.
pub fn cpu_bellman_ford(
    obstacle_mask: &[u32],         // 2D, one bitmask per cell
    history: &[u32],               // full-grid history (CPU linearization)
    width: u32,
    height: u32,
    layer_count: u32,
    source: (u32, u32, u32),       // (x, y, layer)
    target: (u32, u32, u32),
    params: &GridParams,
) -> (Vec<u32>, Vec<u32>) {
    let total = (width * height * layer_count) as usize;
    let mut dist = vec![u32::MAX; total];
    let mut pred = vec![u32::MAX; total];

    let src_idx = cell_index_gpu(source.0, source.1, source.2, width, height);
    dist[src_idx as usize] = 0;

    loop {
        let mut changed = false;
        for layer in 0..layer_count {
            for y in 0..height {
                for x in 0..width {
                    // Same relaxation logic as WGSL shader
                    // ...
                    if new_dist < dist[nidx] {
                        dist[nidx] = new_dist;
                        pred[nidx] = encode_pred(x, y, layer);
                        changed = true;
                    }
                }
            }
        }
        if !changed { break; }
    }

    (dist, pred)
}
```

### Determinism Testing

| Property | Deterministic? | Explanation |
|----------|---------------|-------------|
| Distance array | Yes | `atomicMin` on `u32` is fully deterministic |
| Total path cost | Yes | Determined by distance at target cell |
| Predecessor array | **No** | Multiple threads may set predecessor for same minimum distance |
| Path identity | **No** | Different predecessors may yield different paths with same cost |

**Test assertions**:
- `gpu_distances == cpu_distances` (exact match, u32 fixed-point)
- `gpu_path_cost == cpu_path_cost` (exact match)
- `gpu_predecessors != cpu_predecessors` (allowed to differ; do NOT assert equality)

### Synthetic Test Boards

| Test | Grid | Layers | Nets | What It Tests |
|------|------|--------|------|---------------|
| `empty_grid_straight_path` | 20x20 | 1 | 1 | Basic BF correctness, path is a straight line |
| `single_obstacle_detour` | 20x20 | 1 | 1 | Path routes around a central obstacle |
| `multi_layer_via` | 20x20 | 2 | 1 | Source on layer 0, target on layer 1, via placed |
| `two_crossing_nets` | 30x30 | 2 | 2 | PathFinder convergence with history/congestion |
| `subgraph_boundary_expansion` | 100x100 | 2 | 1 | Source and target far apart, initial subgraph too small |
| `blocked_target` | 20x20 | 1 | 1 | Target surrounded by obstacles, returns empty path |
| `dense_obstacles_serpentine` | 50x50 | 1 | 1 | Many obstacles force a winding path |
| `large_grid_stress` | 500x500 | 4 | 100 | Performance benchmark, verify no OOM or dispatch limits |
| `same_cell_source_target` | 10x10 | 1 | 1 | Trivial case: distance = 0, empty path |
| `history_steers_away` | 30x30 | 1 | 2 | High history on one path forces second net to detour |

### Test Feature Gate

GPU tests are gated behind the `gpu-tests` feature:

```toml
# crates/autopcb-router/Cargo.toml
[features]
gpu-tests = ["wgpu"]  # requires GPU hardware or software rasterizer
```

Tests that cannot acquire a GPU adapter print a skip message and return
successfully (do not fail CI):

```rust
#[cfg(feature = "gpu-tests")]
#[test]
fn gpu_bellman_ford_matches_cpu() {
    let adapter = match pollster::block_on(try_get_adapter()) {
        Some(a) => a,
        None => {
            eprintln!("Skipping: no GPU adapter");
            return;
        }
    };
    // ... run GPU BF, run CPU reference, compare distances
}
```

CPU reference tests run unconditionally (no feature gate):

```rust
#[test]
fn cpu_reference_bellman_ford_straight_path() {
    // Uses cpu_reference::cpu_bellman_ford, no GPU needed
}
```

---

## Implementation Milestones

### Phase 1: Foundation (no GPU yet)

**Files**: `crates/autopcb-router/src/gpu/mod.rs`, `subgraph.rs`, `cpu_reference.rs`

1. Implement `SubgraphRegion` extraction with Corolla initial coverage.
2. Implement dynamic expansion detection.
3. Implement CPU Bellman-Ford reference with fixed-point encoding.
4. Add `gpu_enabled`, `bf_batch_size`, `subgraph_expansion_factor`,
   `subgraph_dynamic_increment` to `RoutingConfig`.
5. Unit tests for subgraph extraction, expansion, and CPU BF correctness.

### Phase 2: GPU Infrastructure

**Files**: `device.rs`, `buffers.rs`, `pipelines.rs`, all `shaders/*.wgsl`

1. Implement headless device initialization.
2. Implement `GridParams` uniform struct with `bytemuck`.
3. Implement obstacle bitmap packing (per-layer bits into u32 bitmask).
4. Write all WGSL shaders.
5. Create compute pipelines and bind group layouts.
6. Implement buffer allocation with pre-sized maximum subgraph.
7. Validation test: `GridParams` Rust struct size matches WGSL struct size.

### Phase 3: GPU Dispatch Loop

**Files**: `dispatch.rs`, `mod.rs`

1. Implement the per-net dispatch sequence: upload params -> reset -> BF loop
   -> convergence check -> readback predecessor -> trace back path.
2. Implement amortized convergence detection (batch N iterations).
3. Implement subgraph expansion retry loop.
4. Implement `GpuGridRouter` struct implementing `DetailedRouter`.
5. Integration test: GPU BF produces identical distances to CPU reference.

### Phase 4: PathFinder Integration

**Files**: `crates/autopcb-router/src/pathfinder/mod.rs`, `gpu/mod.rs`

1. Add runtime selection between `GridRouter` and `GpuGridRouter` based on
   `config.gpu_enabled`.
2. Implement hybrid strategy: GPU for large nets, CPU for small nets.
3. Implement history buffer upload from CPU `HistoryArray` to GPU buffer
   (with CPU-to-GPU linearization remapping).
4. Implement occupancy tracking for GPU-routed nets.
5. End-to-end test: `route_board()` with `gpu_enabled: true` produces valid
   `RouteSolution`.

### Phase 5: Optimization and Profiling

1. Add `wgpu-profiler` integration for per-kernel timing.
2. Tune `bf_batch_size` based on profiling data.
3. Tune subgraph pre-allocation size.
4. Implement double-buffered predecessor readback (overlap CPU traceback of
   net N with GPU routing of net N+1, per
   `docs/notes/autorouter-gpu/05-wgpu-implementation.md` section 3.2).
5. Benchmark GPU vs CPU on synthetic boards of various sizes.
6. Determine and document crossover threshold.

---

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| No GPU adapter available | Medium (CI, headless servers) | Cannot run GPU tests | Feature-gated tests, CPU fallback is always available |
| `atomicMin` not available on some backends | Low (standard WGSL) | Shader compilation fails | Validate at device creation; fall back to CPU |
| Predecessor non-determinism breaks tests | High | Test flakiness | Only assert distance equality, not path identity |
| Subgraph too small for complex nets | Medium | Dynamic expansion loop runs many times | Pre-size with generous expansion factor; cap expansion iterations |
| History buffer upload/download overhead | Low | Per-PF-iteration GPU stall | History buffer changes slowly; upload incrementally if needed |
| wgpu validation errors in CI | Medium | CI failures | Run validation in debug only; add validation-layer error tests |
| Fixed-point overflow for extreme costs | Low | Incorrect distances | Max cost ~4.2M exceeds any realistic path; add overflow detection in CPU reference |

---

## References

- Shen & Luo, "Corolla: GPU-Accelerated FPGA Routing Based on Subgraph
  Dynamic Expansion", FPGA 2017. (`docs/notes/router-gpu/corolla-gpu-fpga-routing-2017.md`)
- McMurchie & Ebeling, "PathFinder: A Negotiation-Based Performance-Driven
  Router for FPGAs", FPGA 1995.
- GPU implementation patterns: `docs/notes/autorouter-gpu/05-wgpu-implementation.md`
- GPU overview and resource limits: `docs/notes/autorouter-gpu/00-overview.md`
- Router plan (Milestones 6, 7): `docs/plans/router/README.md`
- CPU A* router: `crates/autopcb-router/src/detailed/grid.rs`
- PathFinder loop: `crates/autopcb-router/src/pathfinder/mod.rs`
- History array: `crates/autopcb-router/src/pathfinder/history.rs`
- Grid config: `crates/autopcb-router/src/workspace.rs` (`GridConfig`)
- Routing config: `crates/autopcb-router/src/config.rs` (`RoutingConfig`)
- Route solution types: `crates/autopcb-routes/src/lib.rs`
- PcbIr types: `crates/autopcb-ir/src/` (`net.rs`, `component.rs`, `copper.rs`,
  `layer_stack.rs`, `board.rs`, `rule.rs`, `handles.rs`)
- Viewer wgpu setup: `crates/autopcb-viewer/src/view3d.rs`
- Obstacle maps: `crates/autopcb-router/src/obstacles.rs`
- Spatial index: `crates/autopcb-router/src/spatial.rs`

---

## See Also

| Plan | Role | Relationship to Corolla |
|------|------|------------------------|
| **02 — GAMER** (`02-gamer-sweep-routing.md`) | Alternative GPU SSSP backend | Interchangeable at step 3; `backend_select.rs` picks Corolla for small subnets, GAMER for large. Both consume the same `GpuRoutingEngine` fields and produce identical `Vec<PathSegment>` output. |
| **03 — X-Check** (`03-xcheck-gpu-drc.md`) | GPU DRC, runs after step 3 | Consumes `segment_buffer` filled by Corolla/GAMER. Writes DRC violation penalties into `history_costs`, which Corolla reads on the next iteration. |
| **04 — Cypress** (`04-cypress-congestion-feedback.md`) | Post-routing congestion feedback | Reads `history_costs` after the full routing run completes. Feeds congestion back to placement SA. Independent of Corolla at the per-iteration level. |
| **05 — InstantGR** (`05-instantgr-net-batching.md`) | Net batching, runs before step 3 | Partitions nets into `Vec<RoutingBatch>` that Corolla routes simultaneously. Owns the interleaved `distance`/`predecessor` buffer layout that Corolla uses. |
