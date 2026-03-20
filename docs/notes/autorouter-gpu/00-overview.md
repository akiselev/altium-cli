# GPU Acceleration for AutoPCB Placement & Routing (wgpu)

Research report on applying GPU compute (via wgpu) to our autoplacer and autorouter.

## Context

Our router is **spec-centric**: `pcbdoc-spec` is the sole entry point. The router
receives `PcbIr` + `RoutingConfig` and produces a `RouteSolution`. The spec language
is primarily LLM-authored, meaning we can add arbitrarily detailed constraint
declarations that would be tedious for humans but trivial for LLM agents. This
fundamentally changes the GPU parallelism model compared to traditional EDA tools.

We already use wgpu in the viewer crate, so there is no new dependency.

---

## Table of Contents

1. [Why wgpu (not CUDA)](#1-why-wgpu-not-cuda)
2. [wgpu Compute Capabilities & Limits](#2-wgpu-compute-capabilities--limits)
3. [Placement: GPU-Accelerated SA](#3-placement-gpu-accelerated-sa)
4. [Routing: GPU-Accelerated PathFinder](#4-routing-gpu-accelerated-pathfinder)
5. [LLM-Authored Spec Features for GPU Parallelism](#5-llm-authored-spec-features-for-gpu-parallelism)
6. [Proposed Architecture](#6-proposed-architecture)
7. [What NOT to GPU-Accelerate](#7-what-not-to-gpu-accelerate)
8. [Rust GPU Ecosystem](#8-rust-gpu-ecosystem)
9. [References](#9-references)

---

## 1. Why wgpu (not CUDA)

| Factor | wgpu | CUDA |
|--------|------|------|
| GPU vendor support | NVIDIA, AMD, Intel, Apple Silicon | NVIDIA only |
| Platform | Vulkan, Metal, DX12, WebGPU | Linux/Windows (NVIDIA) |
| Existing dependency | Yes (viewer crate) | No |
| Perf vs raw Vulkan | ~5-10% overhead | N/A (native) |
| Perf vs CUDA (NVIDIA) | ~2-4x slower for equivalent kernels | Baseline |
| Atomic f32 | Not in WGSL spec (workaround: fixed-point i32) | Native |
| Subgroup ops | Available (native feature, experimental) | Warp intrinsics (mature) |

**Decision**: wgpu. The 2-4x perf gap vs CUDA is acceptable because (a) we're not
competing with NVIDIA's EDA tools, (b) cross-platform support matters more, (c) we
already have wgpu, and (d) for PCB-scale boards (<10K nets) the absolute runtimes are
small enough that 2-4x doesn't matter.

---

## 2. wgpu Compute Capabilities & Limits

### Resource Limits (guaranteed minimums)

| Limit | Value | Implication |
|-------|-------|-------------|
| Max storage buffer binding | 128 MiB | 32M f32 values; plenty for PCB grids |
| Max buffer size | 256 MiB | |
| Max storage buffers/stage | 8 | Pack data or use multiple bind groups |
| Max bind groups | 4 | |
| Max workgroup size (total) | 256 invocations | 64 is a good default |
| Max workgroup shared memory | 16 KiB | 4096 u32 values for local queues |
| Max workgroups/dimension | 65,535 | |

### Atomic Operations (WGSL standard)

Available on `atomic<i32>` and `atomic<u32>` in storage and workgroup memory:
- `atomicAdd`, `atomicSub`, `atomicMin`, `atomicMax`
- `atomicAnd`, `atomicOr`, `atomicXor`
- `atomicExchange`, `atomicCompareExchangeWeak`

**No f32 atomics in WGSL spec.** Workaround: fixed-point arithmetic. Multiply costs by
1000, use `atomicMin` on `atomic<u32>`. This gives 0.001 resolution up to ~4.2M -- more
than sufficient for routing costs.

Native-only extension `SHADER_FLOAT32_ATOMIC` provides f32 `atomicAdd` (Vulkan/Metal)
but NOT f32 `atomicMin`/`atomicMax`. Not portable enough to rely on.

### Subgroup Operations (native feature, experimental)

Available via `Features::SUBGROUP` on Vulkan, DX12, Metal:
- `subgroupMin`/`subgroupMax`/`subgroupAdd` -- warp-level reductions
- `subgroupBallot` -- which lanes satisfy a predicate
- `subgroupShuffle` -- exchange between lanes (equiv. CUDA `__shfl_sync`)
- `subgroupExclusiveAdd` -- prefix sum within subgroup

These are the WGSL equivalents of CUDA warp intrinsics. All key primitives are
available. Missing: cooperative groups (cross-workgroup sync) -- use multiple
dispatches instead.

### What's NOT Available (vs CUDA)

- **No f32 `atomicMin`/`atomicMax`** -- use u32 fixed-point
- **No cooperative groups** -- no cross-workgroup barriers; use CPU-driven multi-dispatch
- **No dynamic parallelism** -- can't launch kernels from kernels; CPU dispatches
- **No unified memory** -- explicit CPU-GPU transfers required
- **No native FFT** -- would need to implement in WGSL or CPU fallback
- **No tensor cores** -- not relevant for routing
- **16 KiB workgroup memory** -- CUDA offers 48-96 KiB; limits local queue sizes
- **Single queue** -- no concurrent compute + transfer (tracked in wgpu #5576)

### Memory Budget for Routing

A 2000x2000 grid with 8 layers = 32M cells:
- Distance array: 32M x 4 bytes = **128 MB** (fits in one buffer)
- History array: 32M x 4 bytes = **128 MB**
- Obstacle bitmaps: 32M / 8 = **4 MB** per layer, 32 MB total
- Predecessor array: 32M x 4 bytes = **128 MB**

Total: ~416 MB for a large board. Well within discrete GPU VRAM (4-24 GB typical).

For typical PCB boards (<1000x1000 grid, 4 layers), total is ~26 MB.

### f32 vs f64 Precision

wgpu only supports f32 in shaders. For PCB placement:
- Board dimensions in mm with 0.001mm precision: 24" board = 610mm, needs ~20 bits.
  f32 has 23 bits of mantissa. **Sufficient.**
- Altium internal units (10000 per mil): 24" = 2.4 billion units. **Exceeds f32.**
  Work in mm on GPU, convert at boundaries.
- SA acceptance criterion `exp(-delta/T)`: f32 is fine. SA is inherently noise-tolerant.

---

## 3. Placement: GPU-Accelerated SA

### Current Bottlenecks (from codebase analysis)

The SA placer runs ~500K moves (5000 temperature steps x 100 moves/step). Per move:

1. **HPWL evaluation** (highest cost): For each affected net, iterate all components
   and pads, compute rotation + bounding box. O(components x pads) per net, called
   for 2-10 nets per move.
2. **Congestion metrics**: Build congestion grid, accumulate per-net demand, compute
   overflow penalty. O(nets x grid_cells).
3. **Overlap penalty**: AABB overlap checks against spatial grid neighbors.

The outer SA loop (Metropolis acceptance) is inherently sequential -- each move's
accept/reject depends on the previous state.

### GPU Strategy: Batch Move Evaluation

Instead of evaluating one move at a time, batch K candidates:

```
for step in 0..max_steps:
    candidates = [generate_move() for _ in 0..K]  // CPU
    delta_costs = gpu_evaluate_batch(candidates)    // GPU: parallel
    for (move, dc) in zip(candidates, delta_costs):
        if accept(dc, temperature):
            apply_move(move)                        // CPU: sequential
            break  // or continue with updated state
```

**GPU kernels needed:**

1. **HPWL kernel**: One workgroup per net. Pin positions in storage buffer,
   net-to-pin CSR mapping. Parallel min/max reduction in workgroup shared memory.
   Output: per-net HPWL.

2. **Density/overlap kernel**: One thread per component. Each thread `atomicAdd`
   (fixed-point i32) component area to overlapping grid bins. Second pass: parallel
   reduction for total penalty.

3. **Congestion kernel**: One thread per net. Each net spreads routing demand
   (RUDY model) across bounding box bins via `atomicAdd`.

### Analytical Placement on GPU (DREAMPlace approach)

For a more ambitious approach, replace SA global placement with gradient-based
analytical placement:

- Model placement as nonlinear optimization (Nesterov's method)
- Wirelength: differentiable log-sum-exp approximation (not HPWL)
- Density: electrostatics analogy -- solve Poisson equation via FFT on grid
- All operations are embarrassingly parallel

DREAMPlace (UT Austin) achieves **30x speedup** over CPU placement using this approach.
However, it requires FFT which has no native wgpu support -- would need CPU fallback
or custom WGSL FFT implementation.

**Recommendation**: GPU-accelerate the existing SA cost function first (low risk, high
ROI). Consider analytical placement as a future Phase 1 replacement.

### Key Papers: Placement

| Paper | Year | Contribution |
|-------|------|-------------|
| [DREAMPlace](https://github.com/limbo018/DREAMPlace) | 2019 | GPU analytical placer, 30x speedup. [DOI](https://doi.org/10.1145/3316781.3317803) |
| [DREAMPlace 4.0](https://doi.org/10.23919/DATE54114.2022.9774725) | 2022 | Timing-driven, momentum net weighting |
| [ePlace](https://doi.org/10.1145/2699873) | 2015 | Electrostatics-based density model |
| [RePlAce](https://github.com/The-OpenROAD-Project/RePlAce) | 2019 | Open-source Nesterov global placer |
| [Xplace](https://github.com/cuhk-eda/Xplace) | 2022 | Extremely fast GPU global placement |

---

## 4. Routing: GPU-Accelerated PathFinder

### Core Insight: Bellman-Ford Replaces A* on GPU

A* requires a priority queue (sequential). **Bellman-Ford relaxes all edges in
parallel** using `atomicMin` -- a natural GPU primitive. This is the key insight from
Corolla (FPGA 2017) and is used by OrthoRoute (the only existing GPU PCB autorouter).

On a routing grid, adjacency is implicit from (x, y, layer) coordinates -- no CSR
graph storage needed. Each thread computes its 4-6 neighbors directly.

```wgsl
// Core Bellman-Ford step (WGSL)
@compute @workgroup_size(64)
fn bellman_ford_step(@builtin(global_invocation_id) gid: vec3<u32>) {
    let cell = gid.x;
    if (cell >= params.total_cells) { return; }

    let current_dist = atomicLoad(&dist[cell]);
    if (current_dist == 0xFFFFFFFFu) { return; }

    // Decode (x, y, layer) from linear index
    let layer = cell % params.num_layers;
    let y = (cell / params.num_layers) % params.grid_height;
    let x = cell / (params.num_layers * params.grid_height);

    // Try 4 cardinal neighbors + via transitions
    try_relax(cell, x - 1, y, layer, current_dist, COST_HORIZONTAL);
    try_relax(cell, x + 1, y, layer, current_dist, COST_HORIZONTAL);
    try_relax(cell, x, y - 1, layer, current_dist, COST_VERTICAL);
    try_relax(cell, x, y + 1, layer, current_dist, COST_VERTICAL);
    // Via transitions to adjacent layers...
}

fn try_relax(src: u32, nx: u32, ny: u32, nl: u32, src_dist: u32, base_cost: u32) {
    if (out_of_bounds(nx, ny, nl)) { return; }
    let neighbor = nx * params.num_layers * params.grid_height
                 + ny * params.num_layers + nl;
    if (is_blocked(neighbor)) { return; }

    let edge_cost = base_cost + history[neighbor] * params.pres_fac;
    let new_dist = src_dist + edge_cost;
    atomicMin(&dist[neighbor], new_dist);
}
```

Bellman-Ford needs O(V) iterations worst case but converges much faster on grid graphs
(typically O(sqrt(V))). Each iteration = one GPU dispatch. CPU checks convergence
between dispatches.

### PathFinder Loop with GPU

```
CPU                                     GPU
────                                    ────
for iteration in 0..max_iterations:
  rip_up(nets)
  for net_batch in batched_nets:
    upload source/target coords    →    reset distance array to 0xFFFFFFFF
                                        set dist[source] = 0
                                        repeat bellman_ford_step() until converge
                                   ←    download predecessor array
    reconstruct path (CPU)
    update occupancy
  update history                   →    history_update kernel (1 thread/cell)
                                   ←    (history stays on GPU)
  check convergence
```

### Multi-Net Parallelism

The PathFinder loop routes nets sequentially within an iteration (each net's path
affects subsequent nets' costs). But nets that **don't share routing resources** can
be routed simultaneously.

Strategies for batching:
- **Spatial partitioning**: Divide board into regions; nets confined to one region
  are independent. This maps directly to placement groups from the spec.
- **H/V segment independence** (InstantGR approach): Nets whose horizontal and
  vertical bounding boxes don't overlap can't conflict.
- **Spec-declared independence**: LLM declares `independent_groups` in the spec,
  eliminating the need for runtime conflict detection entirely.

### Multi-Solution Exploration

For critical nets, launch N parallel Bellman-Ford searches with different:
- Via placement seeds (force via at different grid points)
- Layer assignment choices (restrict to different layer subsets)
- Cost weight variations (trade via count vs wirelength)

Pick the lowest-cost solution. The spec's priority annotations tell us which nets
are worth the extra GPU time.

### Key Papers: Routing

| Paper | Year | Contribution |
|-------|------|-------------|
| [GAMER](https://ieeexplore.ieee.org/document/9799536) | 2023 | GPU maze routing via H/V sweep decomposition, 16x speedup |
| [Corolla](https://dl.acm.org/doi/10.1145/3020078.3021732) | 2017 | First GPU-accelerated FPGA PathFinder routing, 18.7x speedup |
| [OrthoRoute](https://github.com/bbenchoff/OrthoRoute) | 2025 | Only existing GPU PCB autorouter (Python/CuPy/KiCad) |
| [InstantGR](https://github.com/cuhk-eda/InstantGR) | 2024 | GPU global router, 2.16x over ISPD'24 winner. [DOI](https://dl.acm.org/doi/10.1145/3676536.3676787) |
| [GANGR](https://arxiv.org/abs/2511.17665) | 2025 | GAN-based net batching for 40% runtime reduction |
| [Delta-stepping on compute shaders](https://www.execfoo.de/blog/deltastep_shader.html) | -- | Vulkan compute SSSP, directly translatable to WGSL |
| [Parallel SSSP (PPoPP 2021)](https://www.cs.utexas.edu/~lin/papers/ppopp21.pdf) | 2021 | Work-efficient GPU shortest paths |
| [GPU wavefront BFS](https://arxiv.org/abs/1209.3314) | 2013 | Irregular wavefront propagation on GPU |

### OrthoRoute: Lessons from the Only GPU PCB Router

OrthoRoute (2025) is a Python/CuPy KiCad plugin using PathFinder with CUDA parallel
Dijkstra. Key lessons:

- Routed an **8,192-net backplane** (32 layers, 33.5 GB VRAM, 41 hours on A100)
- PathFinder parameter tuning is critical:
  - `history_decay=0.995` caused exponential growth -- removed entirely
  - Fixed hotset of 100 nets (adaptive sizing caused oscillation)
  - Pressure factor capped at `pres_fac_max=8.0`
- The GPU wins on SSSP inner loop, not on outer negotiation loop
- Python/CuPy overhead is significant -- Rust/wgpu should be faster for CPU-side work

---

## 5. LLM-Authored Spec Features for GPU Parallelism

The spec language is primarily authored by LLM agents, which can generate arbitrarily
detailed constraint declarations. This is a unique advantage over traditional EDA tools
where routing constraints are limited by what humans are willing to type.

Traditional routers spend significant compute **discovering** what the LLM already
knows: which nets are independent, where congestion will occur, which layer each signal
should use. If the LLM declares this in the spec, the GPU router skips discovery and
goes straight to parallel execution on pre-partitioned work.

### Proposed Spec Extensions

**Routing partitions** -- LLM analyzes schematic topology and declares independent zones:
```
routing_partition "power_section" {
  components: [U1, U2, C1..C12]
  nets: [VCC_3V3, GND, VCC_1V8]
  layers: [L1, L4]
}
routing_partition "ddr_bus" {
  components: [U3, U4]
  nets: [DDR_DQ0..DDR_DQ15, DDR_A0..DDR_A13]
  layers: [L2, L3]
}
```
Each partition -> one GPU workgroup, zero coordination between them.

**Per-net routing corridors** -- LLM specifies approximate path regions:
```
net DDR_DQ0 {
  corridor: rect(10mm, 20mm, 45mm, 25mm)
  preferred_layer: L3
  priority: critical
}
```
Constrains A*/Bellman-Ford search space per net -> smaller grid per GPU thread.

**Independence declarations** -- LLM explicitly marks net groups that share no resources:
```
independent_groups [
  [DDR_DQ0..DDR_DQ7],
  [DDR_DQ8..DDR_DQ15],
  [SPI_CLK, SPI_MOSI, SPI_MISO],
]
```
GPU routes all independent groups simultaneously with zero conflict detection overhead.

**Layer assignment hints** -- LLM suggests based on stackup analysis:
```
layer_hints {
  net_class "high_speed" { prefer: [L2, L3], avoid: [L1, L4] }
  net_class "power"      { prefer: [L1, L4] }
  net_class "analog"     { prefer: [L2], isolate_from: "digital" }
}
```
Eliminates layer assignment ILP. Validate hint, use it, report violations.

**Pin escape strategies**:
```
component U1 {
  escape_strategy: dog_bone
  escape_layers: [L1, L2, L3]
  fanout_pitch: 0.8mm
}
```

**Congestion predictions** -- LLM pre-identifies bottleneck regions:
```
congestion_warning {
  region: rect(30mm, 40mm, 35mm, 50mm)
  reason: "narrow channel between U1 and J1"
  max_tracks: 4
}
```
Pre-seeds GPU congestion grid -> PathFinder converges faster.

### Impact on GPU Architecture

With LLM-declared partitions and independence:
- **Partition discovery** -> zero cost (declared in spec)
- **Layer assignment** -> zero cost (hinted in spec)
- **Search space per net** -> dramatically reduced (corridors)
- **Conflict detection** -> eliminated within independent groups
- **Net ordering** -> informed by priority + criticality annotations

The GPU router becomes a **parallel verification + refinement engine** rather than a
search engine. The LLM does strategic thinking, the GPU does geometric execution.

---

## 6. Proposed Architecture

### Integration with Router Plan

The GPU backend slots behind the existing `DetailedRouter` trait (Milestone 6):

```
DetailedRouter trait
    |-- GridRouter       (CPU, pathfinding::astar)     [M6]
    |-- GpuGridRouter    (GPU, wgpu Bellman-Ford)      [future]
    |-- ShapeRouter      (CPU, geometry-based)          [M6]
```

`RoutingConfig` gains `gpu_acceleration: bool` (default false).

### GPU Buffer Layout

```
Buffer 0: obstacles      (per-layer bitmaps, read-only)
           packed u32, one bit per cell per layer
Buffer 1: dist           (atomic<u32>, read-write, reset per net)
           0xFFFFFFFF = unvisited, fixed-point cost otherwise
Buffer 2: history        (u32, read during routing, write during update)
           persistent across nets within a PathFinder iteration
Buffer 3: predecessor    (u32, write-only during routing)
           encoded parent direction for path reconstruction
Uniform:  params         (grid dims, layer count, costs, source/target)
```

### Compute Shader Pipeline

```
Per PathFinder iteration:
  1. history_update.wgsl    -- 1 dispatch, 1 thread/cell
     Updates history costs for oversubscribed cells

  Per net (or per net batch):
    2. reset_dist.wgsl      -- 1 dispatch, 1 thread/cell
       Set all dist to 0xFFFFFFFF, set dist[source]=0
    3. bellman_ford.wgsl    -- N dispatches until convergence
       Parallel edge relaxation with atomicMin
    4. (CPU) read back predecessor array, reconstruct path
```

### Placement GPU Pipeline

```
Per SA temperature step:
  1. (CPU) generate K candidate moves
  2. (CPU) upload candidate data to GPU

  Per candidate batch:
    3. hpwl_eval.wgsl       -- 1 dispatch, 1 workgroup/net
       Parallel min/max reduction over pin positions
    4. density_eval.wgsl    -- 1 dispatch, 1 thread/component
       atomicAdd area to grid bins (fixed-point i32)
    5. (CPU) read back per-candidate cost deltas
    6. (CPU) Metropolis accept/reject, apply moves
```

### When GPU is Worth It

| Metric | CPU-only | GPU-accelerated |
|--------|----------|-----------------|
| Nets < 100 | Fast | Overhead dominates, slower |
| Nets 100-500 | Moderate | Break-even |
| Nets > 500 | Slow | 10-30x routing speedup |
| Grid < 500x500 | Fast | Break-even |
| Grid > 1000x1000 | Slow | 10-50x per-net speedup |
| SA with < 1K components | Fast | Marginal benefit |
| SA with > 5K components | Slow | 5-15x cost eval speedup |

**Recommendation**: Default off. Auto-enable heuristic based on net count + grid size.

---

## 7. What NOT to GPU-Accelerate

| Task | Why CPU is better |
|------|-------------------|
| PathFinder outer loop | Sequential by nature (iteration N depends on N-1) |
| Net ordering/batching | Small data, decision logic, needs spec access |
| Global routing | Coarse grid, fast on CPU, complex data dependencies |
| Trace optimization | Sequential per-net, geometry-heavy (rubber-banding) |
| DRC checking | Spatial queries better with CPU R-tree |
| Move generation (SA) | O(1) random sampling, negligible cost |
| Path reconstruction | Sequential backtracing through predecessor array |
| Spec parsing/compilation | Text processing, no parallelism |

---

## 8. Rust GPU Ecosystem

### Recommended: wgpu (direct WGSL)

We already depend on wgpu for the viewer. Write compute shaders directly in WGSL.
Maximum control over memory layout, atomics, and dispatch patterns.

- Repository: https://github.com/gfx-rs/wgpu
- Learning: https://github.com/d4mr/wgpu-puzzles (14 progressive compute puzzles)
- Profiling: https://github.com/Wumpf/wgpu-profiler (timestamp-based GPU profiling)

### Alternative: CubeCL (multi-backend)

Write kernels in Rust with `#[cube]` macro, compile to WGSL/CUDA/ROCm.
Higher-level abstraction -- less control but multi-backend portability.
Used by Burn (ML framework).

- Repository: https://github.com/tracel-ai/cubecl
- **Consideration**: If we later want NVIDIA-specific perf, CubeCL lets us write
  once and run on both wgpu and CUDA. Worth evaluating after the initial wgpu
  implementation is working.

### Alternative: rust-gpu (Rust as shader language)

Compile Rust to SPIR-V. Share types between CPU and GPU code. Requires nightly Rust
and custom codegen backend. More complex build setup.

- Repository: https://github.com/Rust-GPU/rust-gpu

### Notable wgpu Compute Projects

| Project | Description |
|---------|-------------|
| [Vello](https://github.com/linebender/vello) | GPU 2D renderer using prefix-scan in WGSL |
| [Burn](https://burn.dev/) | ML framework with wgpu backend (competitive with LibTorch) |
| [GPUPrefixSums](https://github.com/b0nes164/GPUPrefixSums) | wgpu prefix sum implementations |

---

## 9. References

### Papers: Routing

- McMurchie & Ebeling, "PathFinder: A Negotiation-Based Performance-Driven Router for FPGAs," FPGA 1995. https://dl.acm.org/doi/10.1145/201310.201328
- GAMER: GPU-Accelerated Maze Routing, IEEE TCAD 2023. https://ieeexplore.ieee.org/document/9799536
- Corolla: GPU-Accelerated FPGA Routing, ACM FPGA 2017. https://dl.acm.org/doi/10.1145/3020078.3021732
- InstantGR: GPU Global Routing, ICCAD 2024. https://dl.acm.org/doi/10.1145/3676536.3676787
- GANGR: GAN-Assisted Routing Parallelization, 2025. https://arxiv.org/abs/2511.17665
- Fast Work-Efficient SSSP for GPUs, PPoPP 2021. https://www.cs.utexas.edu/~lin/papers/ppopp21.pdf
- Work-Efficient Parallel GPU SSSP. https://escholarship.org/content/qt8qr166v2/qt8qr166v2.pdf
- Efficient Irregular Wavefront Propagation on GPUs. https://arxiv.org/abs/1209.3314
- GPU Multi-agent Path Planning. https://www.sciencedirect.com/science/article/pii/S1877050912003249
- Delta-stepping on Vulkan Compute Shaders. https://www.execfoo.de/blog/deltastep_shader.html

### Papers: Placement

- DREAMPlace: GPU-Accelerated VLSI Placement, DAC 2019. https://doi.org/10.1145/3316781.3317803
- DREAMPlace 4.0: Timing-Driven Placement, DATE 2022. https://doi.org/10.23919/DATE54114.2022.9774725
- ePlace: Electrostatics-Based Placement, ACM TODAES 2015. https://doi.org/10.1145/2699873
- RUDY: Routing Demand Estimation, DATE 2007. (Spindler & Johannes)
- Xplace: Extremely Fast GPU Placement, 2022. https://github.com/cuhk-eda/Xplace

### Projects

- OrthoRoute (GPU PCB autorouter): https://github.com/bbenchoff/OrthoRoute
- InstantGR (GPU global router): https://github.com/cuhk-eda/InstantGR
- DREAMPlace: https://github.com/limbo018/DREAMPlace
- RePlAce (Nesterov placer): https://github.com/The-OpenROAD-Project/RePlAce
- OpenROAD (full RTL-to-GDS): https://github.com/The-OpenROAD-Project/OpenROAD

### wgpu & Rust GPU

- wgpu: https://github.com/gfx-rs/wgpu
- CubeCL: https://github.com/tracel-ai/cubecl
- rust-gpu: https://github.com/Rust-GPU/rust-gpu
- Vello: https://github.com/linebender/vello
- Burn: https://burn.dev/
- wgpu-profiler: https://github.com/Wumpf/wgpu-profiler
- wgpu-puzzles: https://github.com/d4mr/wgpu-puzzles
- GPUPrefixSums: https://github.com/b0nes164/GPUPrefixSums

### Specifications

- WGSL Spec: https://www.w3.org/TR/WGSL/
- WGSL Atomics Reference: https://webgpu.rocks/wgsl/functions/synchronization-atomic/
- WGSL Subgroups Proposal: https://github.com/gpuweb/gpuweb/blob/main/proposals/subgroups.md
- wgpu Limits: https://docs.rs/wgpu/latest/wgpu/struct.Limits.html
- wgpu Features: https://docs.rs/wgpu/latest/wgpu/struct.FeaturesWGPU.html
- f32 Atomics Discussion: https://github.com/gpuweb/gpuweb/issues/4894
- ISPD 2024 GPU Routing Contest: https://liangrj2014.github.io/ISPD24_contest/
