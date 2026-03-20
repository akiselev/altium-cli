# GPU-Accelerated Congestion Estimation and Placement-Router Co-Optimization

Implementation plan for replacing the CPU-based congestion estimation in the
placement SA with a GPU-accelerated pipeline inspired by the Cypress paper
(Zhang et al., ISPD 2025), adapted for our wgpu/WGSL stack.

## Table of Contents

1. [Cypress Algorithm for PCB](#1-cypress-algorithm-for-pcb)
2. [GPU Congestion Estimation](#2-gpu-congestion-estimation)
3. [Integration with Existing SA Placer](#3-integration-with-existing-sa-placer)
4. [WGSL Shaders](#4-wgsl-shaders)
5. [Bottleneck Extraction (Router to Placement)](#5-bottleneck-extraction-router-to-placement)
6. [PcbIr / Placement Extensions](#6-pcbir--placement-extensions)
7. [Performance](#7-performance)
8. [Testing](#8-testing)
9. [Implementation Milestones](#9-implementation-milestones)

---

## Pipeline Integration

Cypress (this plan) runs **after** the PathFinder routing loop completes, feeding congestion results back to the placement SA for the next outer co-optimization round.

```
PathFinder Iteration:
  1. Rip-up (CPU)
  2. InstantGR (05) → batch nets into independent groups
  3. For each batch:
     Corolla (01) OR GAMER (02) → GPU SSSP per net in batch
  4. X-Check (03) → GPU DRC, violations → history
  5. History update (GPU kernel)
  6. Convergence check (CPU)

After routing:
  Cypress (04) [this plan] → congestion feedback → placement SA
```

**This plan's role**: Cypress is independent of the PathFinder iteration loop. It runs once after routing converges (or times out). It reads the `history_costs` buffer (accumulated congestion from all PathFinder iterations) and the `congestion_grid`, computes per-component congestion attribution, and returns `Vec<PlacementNudge>` to the placement SA for the next outer co-optimization iteration.

### Shared `GpuRoutingEngine`

Uses shared `GpuRoutingEngine` from `gpu/engine.rs` (see Plan 01 for full definition). Cypress-specific fields/pipelines used:

| Field | Purpose |
|-------|---------|
| `device`, `queue` | wgpu primitives |
| `history_costs` | Read after routing completes; high values indicate congestion hotspots |
| `congestion_grid` | Written by RUDY kernel; read by overflow and score attribution kernels |
| `congestion_rudy_pipeline` | Per-net demand accumulation |
| `congestion_overflow_pipeline` | Compute overflow per cell |

### Shared Buffer Access

| Buffer | Access | Notes |
|--------|--------|-------|
| `history_costs` | Read | Populated by PathFinder iterations (01-03); Cypress reads final state |
| `congestion_grid` | Read/Write | Cleared and rebuilt by RUDY kernel per SA move evaluation |
| `routing_params` | Read | Grid dimensions needed for congestion grid alignment |

### Module Structure

All files live under the shared GPU module (same as Plans 01, 02, 03, 05):

```
crates/autopcb-router/src/gpu/
├── mod.rs              // GpuRoutingEngine (shared device, queue, buffers, pipelines)
├── engine.rs           // GpuRoutingEngine struct, initialization, buffer management
├── buffers.rs          // Buffer types, layout, upload/download helpers
├── bellman_ford.rs     // Corolla BF dispatch (01)
├── sweep.rs            // GAMER H/V sweep dispatch (02)
├── drc.rs              // X-Check GPU DRC (03)
├── congestion.rs       // Cypress congestion estimation (04) [this file]
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

## 1. Cypress Algorithm for PCB

### 1.1 How Cypress Estimates Routing Congestion from Placement

Cypress (Zhang et al., ISPD 2025) formulates PCB placement as a multi-objective
optimization problem with three cost functions computed via gradient descent on
GPU:

1. **Wirelength** (orientation-aware log-sum-exp HPWL approximation)
2. **Density** (electrostatic potential via 2D FFT, DREAMPlace-style)
3. **Net crossing** (pin-pair line segment intersection penalty)

The combined cost function is:

```
L(x, y, theta) = WL(x, y, theta) + lambda_D * D(x, y, theta) + lambda_NC * NC(x, y, theta)
```

Critically, Cypress **does not use RUDY** for PCB congestion estimation. The
paper explicitly argues that RUDY is a flawed model for PCB routability (Section
4.2.1, Figure 3): two net pairs with identical RUDY congestion scores can have
vastly different routing conflicts because RUDY ignores source-sink topology. PCB
routing operates with far fewer metal layers than VLSI, so the specific
pin-to-pin connections (not just bounding-box overlap) determine whether nets
conflict.

Instead, Cypress uses **net crossing** as the primary routability proxy:
- Decompose each multi-pin net into source-sink pin pairs (one line segment per
  sink pin)
- Compute crossing scores between all pairs of line segments on the same layer
  using Bezier-parameter intersection (Eq. 2-3)
- Apply a smooth bell-shaped function (Eq. 4-5) to produce a differentiable
  penalty

### 1.2 RUDY for Our Use Case: Pragmatic Compromise

Despite Cypress's criticism, RUDY remains useful for our SA inner loop because:

1. **Speed**: RUDY is O(nets) on GPU with simple atomicAdd. Net crossing is
   O(pin_pairs^2) -- quadratic in the number of pin pairs on the same layer.
2. **Incremental evaluation**: RUDY demand can be incrementally updated when one
   component moves (subtract old net bboxes, add new ones). Net crossing requires
   re-evaluating all crossings involving any net connected to the moved component.
3. **Our SA is not gradient-based**: Cypress uses gradient descent where the
   differentiable bell-shaped crossing function is essential. Our SA uses
   Metropolis accept/reject on a scalar cost delta, so differentiability is
   irrelevant.

**Our approach**: Use RUDY on GPU for the SA inner loop (fast, incremental), and
optionally compute net crossing as a separate validation metric at snapshot
boundaries or after SA completes. This matches how the existing
`compute_congestion_metrics()` already works (bounding-box demand spreading) but
moves it to GPU for speed.

### 1.3 How RUDY Differs from Cypress's Net Crossing for VLSI vs PCB

| Property | RUDY | Net Crossing (Cypress) |
|----------|------|------------------------|
| Granularity | Bounding-box-level | Pin-pair-level |
| Accuracy for PCB | Overestimates (ignores topology) | More accurate (captures actual conflicts) |
| Complexity | O(nets * bbox_cells) | O(pin_pairs^2) per layer |
| GPU parallelism | One thread per net, atomicAdd | One thread per pair, fully parallel |
| Differentiable | No (discrete grid) | Yes (bell-shaped function) |
| SA suitability | Good (fast scalar cost) | Expensive (full re-evaluation) |

### 1.4 Per-Layer Capacity Estimation

The existing CPU `compute_congestion_metrics()` in `simulated_annealing.rs`
(line 445) uses a single-layer model:

```rust
let congestion_capacity = ir.layer_stack.copper_layer_count.max(1) as f64 * congestion_cell_mm;
```

And the router's `congestion_oracle()` in `coopt.rs` (line 247-255) uses:

```rust
let capacity = cell_area / (typical_pitch * typical_pitch);
```

Both are crude. The GPU implementation should compute per-layer capacity from
the `IrLayerStack`:

```
capacity(layer, cell) = usable_cell_width / trace_pitch
```

Where:
- `usable_cell_width` = `cell_size - obstacle_width_in_cell` (from obstacle map)
- `trace_pitch` = `trace_width + clearance` (from pcb-toolkit impedance tables
  or design rules)
- Multi-layer capacity is the sum across all allowed routing layers for a given
  net class

For the SA inner loop, a simplified model suffices:
```
capacity(cell) = cell_size_mm / avg_trace_pitch * num_copper_layers
```

### 1.5 Congestion-to-Placement Feedback Loop

```
                    ┌─────────────────────┐
                    │   Placement SA      │
                    │   (CPU: Metropolis)  │
                    └──────┬──────────────┘
                           │ component positions
                           ▼
                    ┌─────────────────────┐
                    │  GPU: RUDY kernel   │   < 1ms
                    │  (congestion_rudy)  │
                    └──────┬──────────────┘
                           │ congestion grid
                           ▼
                    ┌─────────────────────┐
                    │  GPU: overflow      │   < 0.1ms
                    │  (congestion_overflow) │
                    └──────┬──────────────┘
                           │ overflow penalty
                           ▼
                    ┌─────────────────────┐
                    │  GPU: component     │   < 0.1ms
                    │  score attribution  │
                    └──────┬──────────────┘
                           │ per-component congestion scores
                           ▼
                    ┌─────────────────────┐
                    │  CPU: SA cost delta │
                    │  congestion_weight  │
                    └─────────────────────┘
```

---

## 2. GPU Congestion Estimation

### 2.1 Input Data

The GPU congestion kernel requires:

**From PcbIr / placement state** (uploaded once, updated incrementally):
- Component positions: `Vec<(f32, f32)>` indexed by component index
- Component rotations: `Vec<f32>` (degrees)
- Pad offsets in component-local coordinates: CSR of `(net_idx, local_x, local_y)`

**From net connectivity** (uploaded once, static):
- Net-to-pin CSR (Compressed Sparse Row):
  - `net_pin_offsets: Vec<u32>` -- `net_pin_offsets[net_idx]` is the start index
    into `net_pins` for net `net_idx`
  - `net_pins: Vec<PinEntry>` where `PinEntry = { comp_idx: u32, local_x: f32, local_y: f32 }`

**From design rules** (uploaded once, static):
- Per-net trace width (from pcb-toolkit impedance tables): `Vec<f32>`
- Default trace pitch (width + clearance): `f32` uniform

### 2.2 Output

The GPU pipeline produces:

- `CongestionGrid` -- per-cell demand/capacity/overflow values
  - `demand: Vec<f32>` -- total routing demand per cell
  - `overflow: Vec<f32>` -- `max(0, demand - capacity)` per cell
  - `total_overflow_penalty: f32` -- sum of squared overflow
- `component_scores: Vec<f32>` -- per-component congestion attribution
  (sum of overflow in cells touched by the component's nets)

These map directly to the existing `CongestionMetrics` struct at
`simulated_annealing.rs:108`:

```rust
struct CongestionMetrics {
    penalty: f64,
    component_scores: Vec<f64>,
}
```

### 2.3 GPU RUDY Implementation

One thread per net computes the net's pin bounding box in world coordinates
(applying component position + rotation to each pin's local offset), then
atomicAdds the RUDY demand to each grid cell within the bbox.

**Net bounding box computation** (per-thread):

```
For net n with pins p_0..p_k:
  For each pin p_i on component c_i:
    world_x = comp_x[c_i] + local_x * cos(rot[c_i]) - local_y * sin(rot[c_i])
    world_y = comp_y[c_i] + local_x * sin(rot[c_i]) + local_y * cos(rot[c_i])
  bbox = (min_x, min_y, max_x, max_y) across all world pin positions
  hpwl = (max_x - min_x) + (max_y - min_y)
  area = max((max_x - min_x) * (max_y - min_y), cell_size^2 * 0.25)
  demand = hpwl / area  (RUDY formula)
```

This matches the CPU implementation at `simulated_annealing.rs:500-504`:

```rust
let demand = (span_w + span_h) / (span_w * span_h).max(cell_size * cell_size * 0.25);
```

### 2.4 Per-Net Trace Width Handling

Nets with wider traces consume more routing capacity. The RUDY demand should be
scaled by the trace width relative to the default:

```
demand_scaled = demand * (trace_width[net] / default_trace_width)
```

Trace widths come from pcb-toolkit impedance tables (see
`docs/notes/autorouter-gpu/06-pcb-toolkit-integration.md`). The router's rules
bridge (M3) pre-computes a `width_table: BTreeMap<(LayerId, ImpedanceClass), TraceWidthMm>`.
For the placement SA, we use the net's default trace width (from the `Width`
design rule resolved by `RoutingPolicy::trace_width()`).

For the initial implementation, use a uniform trace width. The per-net width
buffer can be populated later when the pcb-toolkit integration is complete.

### 2.5 Grid Resolution

The congestion grid resolution should match the SA placer's `congestion_cell_mm`
parameter (default 5.0 mm in `SAConfig`). The GPU grid is configured from:

```rust
let cell_size = config.congestion_cell_mm.max(0.5);
let cols = ((board_width / cell_size).ceil() as u32).max(1);
let rows = ((board_height / cell_size).ceil() as u32).max(1);
```

This matches the CPU implementation at `simulated_annealing.rs:457-459`.

For a typical 100mm x 100mm board with 5mm cells: 20x20 = 400 cells.
For a large 300mm x 200mm board with 2mm cells: 150x100 = 15,000 cells.

Both are trivially small for GPU computation.

---

## 3. Integration with Existing SA Placer

### 3.1 Current Congestion Code

The CPU congestion estimation lives in `simulated_annealing.rs`:

| Function | Location | Purpose |
|----------|----------|---------|
| `compute_congestion_metrics()` | line 445-584 | Full congestion grid computation |
| `build_move_bias_context()` | line 586-622 | Combines criticality + congestion for move selection weights |
| `delta_cost()` | line 1177-1331 | Cost delta including congestion penalty (clones components for "before/after") |
| `component_criticality()` | line 412-443 | Per-component HPWL-based criticality scores |

Key observations:
1. `compute_congestion_metrics()` is called at **every** `delta_cost()` when
   `congestion_enabled` is true (line 1178-1184). It clones the entire
   components array to evaluate the "after" state. This is O(nets * cells) per
   SA move trial.
2. `build_move_bias_context()` calls `compute_congestion_metrics()` once per
   temperature step (line 588) to weight move selection.
3. The `Placement` struct stores `congestion_weight`, `congestion_cell_mm`,
   `congestion_capacity`, and `congestion_enabled` (lines 88-91).

### 3.2 GPU Replacement Strategy

Replace `compute_congestion_metrics()` with GPU dispatch while preserving the
existing API contract:

**Phase 1: Full recomputation on GPU** (simplest, replaces CPU hot path)

```rust
// New: GpuCongestionEstimator wraps wgpu device/pipeline/buffers
struct GpuCongestionEstimator {
    device: wgpu::Device,
    queue: wgpu::Queue,
    rudy_pipeline: wgpu::ComputePipeline,
    overflow_pipeline: wgpu::ComputePipeline,
    score_pipeline: wgpu::ComputePipeline,
    // GPU buffers
    comp_positions: wgpu::Buffer,     // [f32; num_comps * 2]
    comp_rotations: wgpu::Buffer,     // [f32; num_comps]
    net_pin_offsets: wgpu::Buffer,    // [u32; num_nets + 1]
    net_pins: wgpu::Buffer,           // [PinEntry; total_pins]
    congestion_grid: wgpu::Buffer,    // [atomic<u32>; rows * cols]  (fixed-point demand)
    overflow_grid: wgpu::Buffer,      // [f32; rows * cols]
    component_scores: wgpu::Buffer,   // [f32; num_comps]
    readback_penalty: wgpu::Buffer,   // [f32; 1] (mapped for CPU read)
    readback_scores: wgpu::Buffer,    // [f32; num_comps] (mapped for CPU read)
    // Grid params
    rows: u32,
    cols: u32,
    cell_size_mm: f32,
    origin_x: f32,
    origin_y: f32,
}
```

The estimator is built once at SA initialization (in `refine_with_sa()` at
line 1877) and reused across all temperature steps.

**Calling convention** (replaces `compute_congestion_metrics()`):

```rust
fn gpu_compute_congestion(
    estimator: &GpuCongestionEstimator,
    components: &[ComponentState],
) -> CongestionMetrics {
    // 1. Upload component positions/rotations to GPU
    estimator.upload_positions(components);
    // 2. Clear congestion grid (zero-fill)
    estimator.clear_grid();
    // 3. Dispatch RUDY kernel: one thread per net
    estimator.dispatch_rudy();
    // 4. Dispatch overflow kernel: one thread per cell
    estimator.dispatch_overflow();
    // 5. Dispatch score attribution kernel: one thread per net
    estimator.dispatch_score_attribution();
    // 6. Readback penalty + component scores
    estimator.readback()
}
```

### 3.3 Incremental Updates

The full recomputation approach above runs 3 GPU dispatches per `delta_cost()`
call. For the SA inner loop at 100 moves per temperature step, this is 300 GPU
dispatches per step. At ~0.1ms per dispatch, total GPU time is ~30ms per step --
acceptable but not ideal.

**Incremental optimization** (Phase 2):

When a single component moves (the common case for `Move::Displace` and
`Move::Rotate`), only the nets connected to that component change their bounding
boxes. Instead of recomputing the entire grid:

1. **Subtract** the old demand contribution of affected nets from the grid
   (GPU kernel: one thread per affected net, atomicSub)
2. **Add** the new demand contribution with updated component position
   (GPU kernel: one thread per affected net, atomicAdd)
3. Recompute overflow only for cells that changed (sparse update)

This reduces work from O(all_nets * bbox_cells) to O(affected_nets * bbox_cells)
per move trial. For a component with ~10 nets, each spanning ~20 cells, this is
~200 atomicOps vs ~20,000 for the full grid.

**Implementation**: Store a `net_demand_contribution` buffer on GPU. For each
net, record which cells received demand and how much. On incremental update,
read back the affected cells, subtract old, add new.

**Note**: Incremental updates introduce complexity (maintaining per-net demand
records, handling race conditions). Implement full recomputation first, profile,
then add incremental only if the full approach is too slow.

### 3.4 Batch Evaluation

SA move evaluation is inherently sequential (each move is accepted/rejected
before the next is tried). However, we can evaluate **K candidate moves in
parallel** on GPU and pick the best one:

```rust
// Generate K candidate moves
let candidates: Vec<Move> = (0..K)
    .filter_map(|_| generate_move(&placement, temperature, &bias, &mut rng))
    .collect();

// Evaluate all K congestion deltas in parallel on GPU
let congestion_deltas = gpu_batch_evaluate(&estimator, &placement, &candidates);

// Pick best candidate and apply Metropolis criterion
let best = candidates.iter()
    .zip(congestion_deltas.iter())
    .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap());
```

This requires uploading K sets of modified component positions and running the
RUDY kernel K times (or with K-way parallelism in a single dispatch). However,
this changes the SA semantics (evaluating multiple moves per trial rather than
one) and would require careful analysis of its effect on convergence.

**Recommendation**: Defer batch evaluation to a later optimization pass. The
single-move full-recomputation approach is simpler and likely fast enough.

---

## 4. WGSL Shaders

### 4.1 `congestion_rudy.wgsl` -- Per-Net Demand Accumulation

```wgsl
// Uniforms
struct Params {
    num_nets: u32,
    num_pins_total: u32,
    grid_cols: u32,
    grid_rows: u32,
    cell_size: f32,
    origin_x: f32,
    origin_y: f32,
    default_trace_pitch: f32,
}

struct PinEntry {
    comp_idx: u32,
    local_x: f32,
    local_y: f32,
    _pad: f32,  // alignment
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> comp_positions: array<vec2<f32>>;
@group(0) @binding(2) var<storage, read> comp_rotations: array<f32>;
@group(0) @binding(3) var<storage, read> net_pin_offsets: array<u32>;
@group(0) @binding(4) var<storage, read> net_pins: array<PinEntry>;
@group(0) @binding(5) var<storage, read_write> congestion_grid: array<atomic<u32>>;
// Optional: per-net trace width scaling
@group(0) @binding(6) var<storage, read> net_trace_widths: array<f32>;

const DEMAND_SCALE: u32 = 1024u;  // fixed-point scaling for atomicAdd

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let net_idx = gid.x;
    if (net_idx >= params.num_nets) { return; }

    let pin_start = net_pin_offsets[net_idx];
    let pin_end = net_pin_offsets[net_idx + 1u];
    let pin_count = pin_end - pin_start;
    if (pin_count < 2u) { return; }

    // Compute bounding box of all pins in world coordinates
    var min_x = 1e10f;
    var min_y = 1e10f;
    var max_x = -1e10f;
    var max_y = -1e10f;

    for (var i = pin_start; i < pin_end; i++) {
        let pin = net_pins[i];
        let pos = comp_positions[pin.comp_idx];
        let rot = comp_rotations[pin.comp_idx];
        let cos_r = cos(radians(rot));
        let sin_r = sin(radians(rot));

        let world_x = pos.x + pin.local_x * cos_r - pin.local_y * sin_r;
        let world_y = pos.y + pin.local_x * sin_r + pin.local_y * cos_r;

        min_x = min(min_x, world_x);
        min_y = min(min_y, world_y);
        max_x = max(max_x, world_x);
        max_y = max(max_y, world_y);
    }

    // RUDY demand: HPWL / bbox_area
    let span_w = max(max_x - min_x, params.cell_size * 0.5);
    let span_h = max(max_y - min_y, params.cell_size * 0.5);
    let area = max(span_w * span_h, params.cell_size * params.cell_size * 0.25);
    let hpwl = span_w + span_h;
    var demand = hpwl / area;

    // Scale by trace width ratio (wider traces use more capacity)
    let trace_width = net_trace_widths[net_idx];
    if (trace_width > 0.0 && params.default_trace_pitch > 0.0) {
        demand *= trace_width / params.default_trace_pitch;
    }

    // Spread demand uniformly across bbox cells
    let col0 = clamp(i32(floor((min_x - params.origin_x) / params.cell_size)), 0i, i32(params.grid_cols) - 1i);
    let col1 = clamp(i32(floor((max_x - params.origin_x) / params.cell_size)), 0i, i32(params.grid_cols) - 1i);
    let row0 = clamp(i32(floor((min_y - params.origin_y) / params.cell_size)), 0i, i32(params.grid_rows) - 1i);
    let row1 = clamp(i32(floor((max_y - params.origin_y) / params.cell_size)), 0i, i32(params.grid_rows) - 1i);

    let n_cells = f32(max((col1 - col0 + 1i) * (row1 - row0 + 1i), 1i));
    let demand_per_cell = u32(demand / n_cells * f32(DEMAND_SCALE));

    for (var row = row0; row <= row1; row++) {
        for (var col = col0; col <= col1; col++) {
            let idx = u32(row) * params.grid_cols + u32(col);
            atomicAdd(&congestion_grid[idx], demand_per_cell);
        }
    }
}
```

### 4.2 `congestion_overflow.wgsl` -- Compute Overflow Penalty Per Cell

```wgsl
struct OverflowParams {
    total_cells: u32,
    capacity_fixed: u32,  // capacity * DEMAND_SCALE
}

@group(0) @binding(0) var<uniform> params: OverflowParams;
@group(0) @binding(1) var<storage, read> congestion_grid: array<u32>;
@group(0) @binding(2) var<storage, read_write> overflow_grid: array<f32>;
@group(0) @binding(3) var<storage, read_write> total_penalty: array<atomic<u32>>;

const DEMAND_SCALE_F: f32 = 1024.0;
const PENALTY_SCALE: u32 = 1024u;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let cell = gid.x;
    if (cell >= params.total_cells) { return; }

    let demand = f32(congestion_grid[cell]) / DEMAND_SCALE_F;
    let capacity = f32(params.capacity_fixed) / DEMAND_SCALE_F;
    let overflow = max(demand - capacity, 0.0);
    overflow_grid[cell] = overflow;

    // Squared overflow penalty (matches CPU: .max(0.0).powi(2))
    let penalty = overflow * overflow;
    let penalty_fixed = u32(penalty * f32(PENALTY_SCALE));
    atomicAdd(&total_penalty[0], penalty_fixed);
}
```

### 4.3 `congestion_component_score.wgsl` -- Attribute Overflow to Components

This kernel attributes overflow back to components by iterating each net's
bounding box and summing the overflow in the covered cells:

```wgsl
struct ScoreParams {
    num_nets: u32,
    grid_cols: u32,
    grid_rows: u32,
    cell_size: f32,
    origin_x: f32,
    origin_y: f32,
}

@group(0) @binding(0) var<uniform> params: ScoreParams;
@group(0) @binding(1) var<storage, read> comp_positions: array<vec2<f32>>;
@group(0) @binding(2) var<storage, read> comp_rotations: array<f32>;
@group(0) @binding(3) var<storage, read> net_pin_offsets: array<u32>;
@group(0) @binding(4) var<storage, read> net_pins: array<PinEntry>;
@group(0) @binding(5) var<storage, read> overflow_grid: array<f32>;
@group(0) @binding(6) var<storage, read_write> component_scores: array<atomic<u32>>;

const SCORE_SCALE: u32 = 1024u;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let net_idx = gid.x;
    if (net_idx >= params.num_nets) { return; }

    let pin_start = net_pin_offsets[net_idx];
    let pin_end = net_pin_offsets[net_idx + 1u];
    if ((pin_end - pin_start) < 2u) { return; }

    // Recompute bounding box (same as RUDY kernel)
    var min_x = 1e10f;
    var min_y = 1e10f;
    var max_x = -1e10f;
    var max_y = -1e10f;
    var unique_comps: array<u32, 64>;  // max 64 components per net
    var num_unique_comps = 0u;

    for (var i = pin_start; i < pin_end; i++) {
        let pin = net_pins[i];
        let pos = comp_positions[pin.comp_idx];
        let rot = comp_rotations[pin.comp_idx];
        let cos_r = cos(radians(rot));
        let sin_r = sin(radians(rot));

        let world_x = pos.x + pin.local_x * cos_r - pin.local_y * sin_r;
        let world_y = pos.y + pin.local_x * sin_r + pin.local_y * cos_r;

        min_x = min(min_x, world_x);
        min_y = min(min_y, world_y);
        max_x = max(max_x, world_x);
        max_y = max(max_y, world_y);

        // Track unique components
        var found = false;
        for (var j = 0u; j < num_unique_comps; j++) {
            if (unique_comps[j] == pin.comp_idx) { found = true; break; }
        }
        if (!found && num_unique_comps < 64u) {
            unique_comps[num_unique_comps] = pin.comp_idx;
            num_unique_comps++;
        }
    }

    // Sum overflow in the net's bbox cells
    let col0 = clamp(i32(floor((min_x - params.origin_x) / params.cell_size)), 0i, i32(params.grid_cols) - 1i);
    let col1 = clamp(i32(floor((max_x - params.origin_x) / params.cell_size)), 0i, i32(params.grid_cols) - 1i);
    let row0 = clamp(i32(floor((min_y - params.origin_y) / params.cell_size)), 0i, i32(params.grid_rows) - 1i);
    let row1 = clamp(i32(floor((max_y - params.origin_y) / params.cell_size)), 0i, i32(params.grid_rows) - 1i);

    var overflow_sum = 0.0f;
    for (var row = row0; row <= row1; row++) {
        for (var col = col0; col <= col1; col++) {
            let idx = u32(row) * params.grid_cols + u32(col);
            overflow_sum += overflow_grid[idx];
        }
    }

    if (overflow_sum <= 0.0) { return; }

    // Attribute overflow equally to all components in this net
    let score_fixed = u32(overflow_sum * f32(SCORE_SCALE));
    for (var j = 0u; j < num_unique_comps; j++) {
        atomicAdd(&component_scores[unique_comps[j]], score_fixed);
    }
}
```

### 4.4 Buffer Layout Summary

| Buffer | Type | Size | Binding | Updated |
|--------|------|------|---------|---------|
| `comp_positions` | `storage, read` | `num_comps * 8` bytes | 0:1 | Every `delta_cost()` |
| `comp_rotations` | `storage, read` | `num_comps * 4` bytes | 0:2 | Every `delta_cost()` |
| `net_pin_offsets` | `storage, read` | `(num_nets + 1) * 4` bytes | 0:3 | Once at init |
| `net_pins` | `storage, read` | `total_pins * 16` bytes | 0:4 | Once at init (unless pin swap) |
| `congestion_grid` | `storage, read_write` | `rows * cols * 4` bytes | 0:5 | Cleared + written each eval |
| `net_trace_widths` | `storage, read` | `num_nets * 4` bytes | 0:6 | Once at init |
| `overflow_grid` | `storage, read_write` | `rows * cols * 4` bytes | 1:0 | Written each eval |
| `total_penalty` | `storage, read_write` | `4` bytes | 1:1 | Readback each eval |
| `component_scores` | `storage, read_write` | `num_comps * 4` bytes | 1:2 | Readback each eval |

For a board with 1000 components, 500 nets, 5000 pins, 400 grid cells:
- Static buffers: ~100 KB
- Dynamic buffers: ~10 KB per evaluation
- Total GPU memory: < 1 MB

---

## 5. Bottleneck Extraction (Router to Placement)

### 5.1 Current Implementation

The existing `extract_bottlenecks()` in `coopt.rs` (line 284-344):
1. Runs `congestion_oracle()` to build a `CongestionGrid`
2. Iterates all cells, finds those with `severity > 1.0`
3. For each oversubscribed cell, does an O(components * pads) search to find
   nearby components
4. Returns `Vec<Bottleneck>` sorted by severity

### 5.2 Router-Informed Bottleneck Extraction

After routing fails or partially converges, the PathFinder state contains
**history costs** -- accumulated penalties at grid cells that were persistently
oversubscribed across iterations. These are far more informative than the
placement-only RUDY estimate:

```rust
pub fn extract_routing_bottlenecks(
    pathfinder_state: &PathFinderState,
    workspace: &RoutingWorkspace,
    ir: &PcbIr,
) -> Vec<Bottleneck> {
    // 1. Identify cells where history cost exceeds threshold
    //    (high history = persistently fought over by multiple nets)
    let threshold = pathfinder_state.history.iter()
        .copied()
        .fold(0.0_f64, f64::max) * 0.5;  // top 50% of history

    // 2. Map oversubscribed grid cells back to coarse congestion grid cells
    //    (routing grid is finer than congestion grid)

    // 3. For each coarse cell with high history: find components whose pads
    //    are within search_radius

    // 4. Score each component by sum of history costs of its pad cells

    // 5. Generate placement nudge suggestions:
    //    - Direction: away from the highest-history neighbor
    //    - Magnitude: proportional to overflow severity
}
```

### 5.3 Placement Nudge Suggestions

When routing identifies a bottleneck, it can generate nudge suggestions that
the placement SA uses as biased moves:

```rust
pub struct PlacementNudge {
    pub component: ComponentId,
    pub dx_mm: f64,
    pub dy_mm: f64,
    pub severity: f64,
}
```

These nudges feed into the SA move generator as biased displacements:
- With probability proportional to `severity`, generate a `Move::Displace`
  in the suggested direction
- The SA Metropolis criterion still governs acceptance

### 5.4 Outer Co-Optimization Loop

```
┌──────────────────────────────────────────────────────┐
│                 Outer Loop (3-5 iterations)           │
│                                                      │
│  1. Placement SA (with GPU congestion feedback)      │
│     → PlacementResult                                │
│                                                      │
│  2. Global Route (fast, CPU)                         │
│     → GlobalRoutePlan                                │
│     → CongestionGrid (more accurate than RUDY)       │
│                                                      │
│  3. If routing converged: DONE                       │
│                                                      │
│  4. Extract bottlenecks from routing state           │
│     → Vec<PlacementNudge>                            │
│                                                      │
│  5. Feed nudges back to placement SA (next iteration) │
│     → Bias move generator toward nudge directions     │
│                                                      │
└──────────────────────────────────────────────────────┘
```

---

## 6. PcbIr / Placement Extensions

### 6.1 New Types

**In `autopcb-placement`** (new file: `src/gpu_congestion.rs`):

```rust
/// GPU-accelerated congestion estimator.
///
/// Wraps wgpu device, pipelines, and buffers for RUDY-based congestion
/// estimation. Built once at SA initialization, reused for all evaluations.
pub struct GpuCongestionEstimator {
    device: wgpu::Device,
    queue: wgpu::Queue,
    // ... pipelines and buffers as described in Section 4.4
}

impl GpuCongestionEstimator {
    /// Build from PcbIr and SA config. Uploads static data to GPU.
    pub fn new(ir: &PcbIr, config: &SAConfig) -> Result<Self, PlacementError>;

    /// Compute congestion metrics for the given component state.
    pub fn evaluate(&self, components: &[ComponentState]) -> CongestionMetrics;

    /// Update a single component's position/rotation on GPU.
    pub fn update_component(&self, comp_idx: usize, x: f64, y: f64, rotation: f64);
}
```

**In `autopcb-placement`** (extend `Placement` struct):

```rust
struct Placement {
    // ... existing fields ...
    /// GPU congestion estimator (None if GPU unavailable or congestion disabled).
    gpu_congestion: Option<GpuCongestionEstimator>,
}
```

### 6.2 CongestionOracle Interface

The existing `CongestionOracle` trait in `congestion.rs` (line 48-53) provides
the stable interface between placement and router:

```rust
pub trait CongestionOracle {
    fn congestion_penalty_at(&self, x_mm: f64, y_mm: f64) -> f64;
}
```

The GPU estimator's output grid should implement this trait:

```rust
impl CongestionOracle for GpuCongestionGrid {
    fn congestion_penalty_at(&self, x_mm: f64, y_mm: f64) -> f64 {
        self.overflow_at(x_mm, y_mm)
    }
}
```

This preserves the existing integration path where the placement SA can accept
`Option<&dyn CongestionOracle>` for external congestion feedback from the
router's `CongestionGrid`.

### 6.3 Congestion Grid Flow Between Crates

```
autopcb-placement                        autopcb-router
┌───────────────────────┐               ┌──────────────────────────┐
│ GpuCongestionEstimator│               │ CongestionGrid (coopt.rs)│
│ (internal GPU RUDY)   │               │ (CPU RUDY, coarser)      │
│                       │               │                          │
│ evaluate() → CongestionMetrics        │ congestion_oracle()      │
│ (per-move, <1ms)      │               │ (per-routing-iter, ~10ms)│
│                       │               │                          │
│ Implements:           │  ◄─────────── │ CongestionGrid also      │
│ CongestionOracle trait│  feeds into   │ implements CongestionOracle│
└───────────────────────┘  SA via       └──────────────────────────┘
                          apply_external_congestion_penalty()
```

The two congestion models serve different purposes:
- **Internal GPU RUDY** (`GpuCongestionEstimator`): Fast, runs every SA move,
  uses the SA's own grid resolution
- **External router CongestionGrid** (`coopt.rs`): More accurate, runs after
  a placement round completes, uses the router's grid resolution

Both implement `CongestionOracle`. The SA cost function sums both contributions
(when the external oracle is available).

---

## 7. Performance

### 7.1 Expected Speedup Over CPU

Current CPU `compute_congestion_metrics()` performance for a 1000-net board:
- Per call: ~2-5ms (iterates all nets, all pins, all bbox cells)
- Per SA step: ~200-500ms (100 moves/step * 2 calls per move for before/after)
- Per SA run: ~100-250 seconds (500 steps * 200-500ms)

GPU RUDY kernel for the same board:
- Per call: < 0.5ms (1000 threads, ~400 atomicAdds each)
- Per SA step: ~50ms (100 moves/step * 0.5ms per call)
- Per SA run: ~25 seconds

**Expected speedup: 4-10x** for the congestion evaluation alone. The overall SA
speedup is smaller because HPWL and overlap evaluation remain on CPU.

### 7.2 GPU Memory Requirements

| Item | Size (1000 components, 500 nets) |
|------|----------------------------------|
| Component positions + rotations | 12 KB |
| Net-to-pin CSR | 100 KB |
| Congestion grid (400 cells) | 1.6 KB |
| Overflow grid | 1.6 KB |
| Component scores | 4 KB |
| Shader pipelines + bind groups | ~50 KB |
| **Total** | **< 200 KB** |

For a large board (5000 components, 5000 nets, 15,000 cells):
- Total GPU memory: < 2 MB

This is negligible. The viewer already uses wgpu for rendering, so device
initialization overhead is amortized.

### 7.3 Latency Requirements

The SA inner loop requires congestion evaluation latency < 1ms to avoid
dominating the per-move cost. Budget breakdown:

| Operation | Budget | Expected |
|-----------|--------|----------|
| Upload positions | 0.05ms | 0.02ms |
| Clear grid | 0.02ms | 0.01ms |
| RUDY kernel | 0.3ms | 0.1ms |
| Overflow kernel | 0.1ms | 0.05ms |
| Score attribution | 0.3ms | 0.1ms |
| Readback penalty + scores | 0.2ms | 0.1ms |
| **Total** | **< 1ms** | **~0.4ms** |

The readback is the bottleneck (GPU → CPU synchronization). To minimize it:
- Only read back `total_penalty` (4 bytes) per move
- Read back `component_scores` only at move bias recomputation time (once per
  temperature step, not per move)

With this optimization, per-move latency drops to ~0.3ms.

### 7.4 When GPU Acceleration is Not Worth It

For small boards (< 50 components, < 100 nets), the overhead of GPU dispatch
(~0.1ms minimum) exceeds the CPU computation time (~0.05ms). The implementation
should fall back to CPU for small boards:

```rust
const GPU_CONGESTION_THRESHOLD: usize = 100; // minimum nets for GPU benefit

let use_gpu = ir.nets.len() >= GPU_CONGESTION_THRESHOLD
    && gpu_device_available();
```

---

## 8. Testing

### 8.1 Determinism

GPU floating-point arithmetic is not bitwise identical to CPU f64. The test
strategy accounts for this:

```rust
#[test]
fn gpu_congestion_is_deterministic() {
    let ir = make_test_ir();
    let config = SAConfig::default();
    let estimator = GpuCongestionEstimator::new(&ir, &config).unwrap();
    let components = make_test_components();

    let result_a = estimator.evaluate(&components);
    let result_b = estimator.evaluate(&components);

    // Exact equality: same GPU, same inputs, same order → same results
    assert_eq!(result_a.penalty, result_b.penalty);
    assert_eq!(result_a.component_scores, result_b.component_scores);
}
```

GPU RUDY uses `atomicAdd` on `u32` (fixed-point), which is order-independent
(commutative). Multiple threads may execute in different orders across runs, but
since addition is commutative and we use fixed-point (integer) arithmetic, the
result is **exactly deterministic** regardless of thread scheduling.

### 8.2 GPU vs CPU Comparison

The GPU RUDY result should closely match the CPU `compute_congestion_metrics()`:

```rust
#[test]
fn gpu_matches_cpu_congestion_within_tolerance() {
    let ir = make_test_ir_with_nets(200);
    let config = SAConfig { congestion_cell_mm: 5.0, ..Default::default() };
    let components = make_test_components(&ir);

    let cpu_result = compute_congestion_metrics(/* ... */);
    let gpu_result = GpuCongestionEstimator::new(&ir, &config)
        .unwrap()
        .evaluate(&components);

    // Tolerance accounts for f32 vs f64 precision
    let penalty_err = (gpu_result.penalty - cpu_result.penalty).abs();
    assert!(
        penalty_err < cpu_result.penalty * 0.01,
        "GPU penalty {:.4} vs CPU penalty {:.4}, error {:.6}",
        gpu_result.penalty, cpu_result.penalty, penalty_err
    );

    for (i, (gpu, cpu)) in gpu_result.component_scores.iter()
        .zip(cpu_result.component_scores.iter())
        .enumerate()
    {
        let err = (gpu - cpu).abs();
        assert!(
            err < cpu.abs() * 0.01 + 1e-6,
            "component {i}: GPU {gpu:.4} vs CPU {cpu:.4}"
        );
    }
}
```

Expected differences:
- f32 vs f64 precision: up to ~1e-4 relative error
- Fixed-point quantization: up to 1/1024 absolute error per cell

### 8.3 Synthetic Boards with Known Bottleneck Areas

```rust
#[test]
fn known_bottleneck_detected() {
    // Create a board with a narrow channel: 10mm gap between two keepouts
    // Force 50 nets through the gap → congestion in the channel cells
    let ir = make_bottleneck_board(
        board_size_mm: 100.0,
        gap_x: 45.0..55.0,  // 10mm channel
        gap_y: 0.0..100.0,
        num_nets_through_gap: 50,
    );

    let config = SAConfig { congestion_cell_mm: 5.0, ..Default::default() };
    let estimator = GpuCongestionEstimator::new(&ir, &config).unwrap();
    let result = estimator.evaluate(&make_components(&ir));

    // The channel cells (columns 9-10 out of 20) should have the highest overflow
    let channel_cols = 9..=10;
    let max_overflow_in_channel = result.overflow_grid.iter()
        .enumerate()
        .filter(|(idx, _)| channel_cols.contains(&(idx % 20)))
        .map(|(_, v)| *v)
        .fold(0.0_f64, f64::max);

    let max_overflow_outside = result.overflow_grid.iter()
        .enumerate()
        .filter(|(idx, _)| !channel_cols.contains(&(idx % 20)))
        .map(|(_, v)| *v)
        .fold(0.0_f64, f64::max);

    assert!(
        max_overflow_in_channel > max_overflow_outside * 2.0,
        "channel should have significantly higher overflow"
    );
}
```

### 8.4 Fallback Testing

```rust
#[test]
fn cpu_fallback_when_gpu_unavailable() {
    // When GpuCongestionEstimator cannot be created (no GPU, wgpu error),
    // the SA must fall back to CPU compute_congestion_metrics() seamlessly.
    let ir = make_test_ir();
    let config = SAConfig {
        congestion_weight: 1.0,
        ..Default::default()
    };

    // Force CPU path
    let result = refine_with_sa_cpu_only(&initial, &ir, &config, &designators, 0.5);
    assert!(result.is_ok());
}
```

---

## 9. Implementation Milestones

### M1: GPU Congestion Infrastructure (1 week)

**Files**:
- `crates/autopcb-placement/src/gpu_congestion.rs` (new)
- `crates/autopcb-placement/Cargo.toml` (add `wgpu` dependency)

**Tasks**:
1. Add `wgpu` as optional dependency behind `gpu` feature flag
2. Create `GpuCongestionEstimator` struct with wgpu device/queue initialization
3. Implement `congestion_rudy.wgsl` shader
4. Implement buffer upload/download for component positions and net connectivity
5. Test: determinism, GPU vs CPU comparison on synthetic board

### M2: SA Integration (3 days)

**Files**:
- `crates/autopcb-placement/src/simulated_annealing.rs` (modify `delta_cost`,
  `build_move_bias_context`, `refine_with_sa`)

**Tasks**:
1. Create `GpuCongestionEstimator` in `refine_with_sa()` when `gpu` feature
   enabled and board exceeds size threshold
2. Replace `compute_congestion_metrics()` calls in `delta_cost()` with
   `gpu_estimator.evaluate()`
3. Read back only `total_penalty` per move; read `component_scores` only at
   bias recomputation
4. Implement CPU fallback when GPU unavailable
5. Test: SA produces valid placement with GPU congestion

### M3: Overflow and Score Attribution Shaders (3 days)

**Files**:
- `crates/autopcb-placement/shaders/congestion_overflow.wgsl` (new)
- `crates/autopcb-placement/shaders/congestion_component_score.wgsl` (new)
- `crates/autopcb-placement/src/gpu_congestion.rs` (extend)

**Tasks**:
1. Implement `congestion_overflow.wgsl`
2. Implement `congestion_component_score.wgsl`
3. Chain the three shaders in the `evaluate()` method
4. Test: component scores match CPU within tolerance

### M4: Incremental Updates (3 days)

**Files**:
- `crates/autopcb-placement/src/gpu_congestion.rs` (extend)
- `crates/autopcb-placement/shaders/congestion_incremental.wgsl` (new)

**Tasks**:
1. Track per-net demand contributions in a GPU buffer
2. Implement incremental subtract/add for single-component moves
3. Benchmark: incremental vs full recomputation
4. Use incremental for `Move::Displace` and `Move::Rotate`, full for `Move::Swap`

### M5: Bottleneck Extraction and Nudge Generation (2 days)

**Files**:
- `crates/autopcb-router/src/coopt.rs` (extend `extract_bottlenecks`)
- `crates/autopcb-placement/src/simulated_annealing.rs` (extend move generator)

**Tasks**:
1. Extend `extract_bottlenecks()` to use PathFinder history when available
2. Define `PlacementNudge` type
3. Generate nudge suggestions from routing bottlenecks
4. Integrate nudges into SA move generator as biased displacements

### M6: Per-Net Trace Width Integration (2 days)

**Files**:
- `crates/autopcb-placement/src/gpu_congestion.rs` (extend)
- Depends on pcb-toolkit width table (M3 of router plan)

**Tasks**:
1. Accept per-net trace widths from `RoutingPolicy::trace_width()`
2. Upload to GPU as `net_trace_widths` buffer
3. Scale RUDY demand by trace width ratio in shader
4. Test: wider traces produce higher congestion in affected cells

---

## References

- Zhang et al., "Cypress: VLSI-Inspired PCB Placement with GPU Acceleration,"
  ISPD 2025 (Best Paper). Full text in `docs/notes/router-gpu/cypress-pcb-placement-gpu-2025.md`.
- Spindler & Johannes, "Fast and Accurate Routing Demand Estimation," DATE 2007
  (RUDY). Referenced in `docs/notes/autorouter-gpu/03-gpu-cost-functions.md`.
- wgpu implementation patterns: `docs/notes/autorouter-gpu/05-wgpu-implementation.md`.
- pcb-toolkit integration: `docs/notes/autorouter-gpu/06-pcb-toolkit-integration.md`.
- Router plan M11 (Placement-Router Co-Optimization): `docs/plans/router/README.md`.
- Existing CPU congestion: `crates/autopcb-placement/src/simulated_annealing.rs:445-584`.
- CongestionOracle trait: `crates/autopcb-placement/src/congestion.rs`.
- Router CongestionGrid: `crates/autopcb-router/src/coopt.rs`.

---

## See Also

| Plan | Role | Relationship to Cypress |
|------|------|------------------------|
| **01 — Corolla** (`01-corolla-bellman-ford.md`) | GPU SSSP backend | Defines `GpuRoutingEngine` including `history_costs` and `congestion_grid` buffers that Cypress reads after routing completes. |
| **02 — GAMER** (`02-gamer-sweep-routing.md`) | Alternative GPU SSSP backend | Same integration point as Corolla. Cypress is agnostic to which SSSP backend ran — it reads the final `history_costs` regardless of how they were accumulated. |
| **03 — X-Check** (`03-xcheck-gpu-drc.md`) | GPU DRC, per-iteration violations | Writes DRC violation penalties into `history_costs` throughout the PathFinder loop. Cypress reads the final state of `history_costs` — a combined signal from occupancy conflicts and DRC violations across all iterations. |
| **05 — InstantGR** (`05-instantgr-net-batching.md`) | Net batching | Operates inside the PathFinder iteration loop, before Cypress. Cypress runs after all iterations complete and is independent of per-iteration batching decisions. |
