# GPU Acceleration for PCB Autorouting: Research Report

## Executive Summary

GPU acceleration for PCB autorouting is viable and has been demonstrated in both academic
research and real-world projects. The key insight from the literature is that **the
parallelism lives inside single-net shortest-path search** (intra-net parallelism) and
in **batching non-conflicting nets for simultaneous routing** (inter-net parallelism).
The negotiation loop itself (PathFinder's history/congestion update) remains sequential
across iterations, but each iteration's routing work is highly parallelizable.

wgpu is a reasonable platform for this work. It provides the compute shader primitives
needed (atomics, workgroup shared memory, storage buffers, subgroup operations), runs on
all major GPU vendors (Vulkan/Metal/DX12), and avoids NVIDIA lock-in. The main trade-off
versus CUDA is the loss of some advanced features (cooperative groups, dynamic parallelism)
and ~5-10% abstraction overhead from the WebGPU validation layer.

**Recommendation**: Implement CPU-first (already planned in the router milestones), then
add GPU acceleration as an optional backend for the two hottest paths: (1) single-net
shortest-path search using parallel Bellman-Ford or wavefront BFS, and (2) congestion map
updates. This matches the architecture in the existing router plan where `GridRouter` and
`ShapeRouter` are behind a `DetailedRouter` trait -- a `GpuGridRouter` can slot in behind
the same trait.

---

## 1. GPU-Accelerated Maze Routing and Pathfinding

### 1.1 GAMER: GPU-Accelerated Maze Routing (2021/2023)

**Paper**: "GAMER: GPU-Accelerated Maze Routing" -- IEEE TCAD 2023, Vol. 42, Issue 2, pp. 583-593.
- IEEE: https://ieeexplore.ieee.org/document/9799536
- ICCAD 2021 conference version: https://ieeexplore.ieee.org/document/9643563

**Algorithm**: Decomposes multisource-multidestination shortest path into alternating
vertical and horizontal sweep operations. Each sweep is parallelized from O(n^2) to
O(log2 n) on an n x n grid.

**Key results**:
- 16x average speedup on ICCAD 2019 global routing benchmarks (coarsened maze routing stage)
- Applied to CUGR open-source global router: 19.85x speedup on coarse routing, 2.59x on fine routing, 2.7x overall
- No quality loss

**wgpu mapping**: The sweep decomposition maps well to compute shaders. Each sweep
processes a row or column in parallel -- this is a natural 1D workgroup dispatch. No
CUDA-specific features (warp shuffle, cooperative groups) are required. The alternating
H/V sweep pattern can be implemented as two separate compute passes with a barrier between
them.

**Relevance**: HIGH. This is the most directly applicable technique for accelerating our
A*-based detailed router. The sweep decomposition replaces the priority-queue-based
expansion in standard Dijkstra/A* with a fully data-parallel operation.

### 1.2 Parallel Bellman-Ford on GPU

**Papers**:
- "Bellman-Ford Single Source Shortest Path Algorithm on GPU using CUDA" -- https://towardsdatascience.com/bellman-ford-single-source-shortest-path-algorithm-on-gpu-using-cuda-a358da20144b/
- "Work-Efficient Parallel GPU Methods for Single-Source Shortest Paths" -- https://escholarship.org/content/qt8qr166v2/qt8qr166v2.pdf
- "A Fast Work-Efficient SSSP Algorithm for GPUs" (PPoPP 2021) -- https://www.cs.utexas.edu/~lin/papers/ppopp21.pdf

**Algorithm**: Unlike Dijkstra (inherently sequential due to priority queue), Bellman-Ford
relaxes ALL edges in parallel each iteration. This makes it naturally GPU-friendly.
Optimized variants use delta-stepping to reduce work: group vertices into distance buckets,
process each bucket in parallel.

**Key implementation pattern**:
```
for each iteration until convergence:
    for each vertex v (in parallel):
        for each neighbor u of v:
            new_dist = dist[v] + weight(v, u)
            atomicMin(&dist[u], new_dist)  // atomic update
```

**Performance**: 34x average speedup over serial Dijkstra. Delta-stepping variants achieve
13x-220x depending on graph structure.

**wgpu mapping**: EXCELLENT. The core operation is `atomicMin` on `u32` in storage buffers,
which is natively supported in WGSL. The graph is stored in CSR format (compressed sparse
row) in storage buffers. Each thread processes one vertex. The main loop runs on CPU,
dispatching GPU compute passes until convergence.

**Key consideration**: For our routing grid graph (regular 3D grid), we don't even need
CSR format -- the adjacency is implicit from grid coordinates. Each thread computes its
4-6 neighbors from `(x, y, layer)` coordinates, making the memory access pattern very
regular and cache-friendly.

**Relevance**: HIGH. This is what Corolla (below) uses instead of Dijkstra, and what
OrthoRoute implements as "parallel Dijkstra" (it's actually closer to Bellman-Ford).

### 1.3 Delta-Stepping on Compute Shaders

**Blog/Implementation**: Patrick Niklaus -- https://www.execfoo.de/blog/deltastep_shader.html

**Implementation details** (Vulkan/GLSL compute shaders, directly applicable to WGSL):
- Graph stored in CSR format across three read-only storage buffers
- Distance buffer marked `coherent` for cross-workgroup visibility
- Uses `atomicMin(dist[v], alt)` for safe parallel distance updates
- Workgroup size: 64 threads (local_size_x = 64)
- Change tracking via bit-vector buffers with atomic min/max on changed node ranges
- Z-order (Morton code) node reordering for cache locality (significant optimization)

**Performance**: On AMD RX480, delta-stepping achieves ~1.1x speedup over parallel
Dijkstra. Near-Far variant achieves 1.48x. The gains are modest for road networks but
may be larger for regular grid graphs (like our routing grid).

**wgpu mapping**: DIRECT. This was implemented in Vulkan compute shaders. The translation
to WGSL is mechanical -- `atomicMin` on `atomic<u32>` is directly available.

### 1.4 GPU-Accelerated A* and Wavefront BFS

**Papers**:
- "GPU Accelerated Multi-agent Path Planning Based on Grid Space Decomposition" -- https://www.sciencedirect.com/science/article/pii/S1877050912003249
- "Efficient Irregular Wavefront Propagation Algorithms on Hybrid CPU-GPU Machines" -- https://arxiv.org/abs/1209.3314 / https://pmc.ncbi.nlm.nih.gov/articles/PMC3727669/
- "GPU-accelerated Conflict-based Search for Multi-agent Embodied Intelligence" -- Springer 2025

**Lee's Algorithm parallelization**: The wavefront BFS used in Lee's maze router maps
naturally to GPU. Each BFS level (wavefront) is processed in parallel -- all cells at
distance d are expanded simultaneously. The wavefront propagation pattern uses:
1. Read wavefront cells from one buffer
2. For each cell, check neighbors and atomically update distance
3. Build next wavefront using atomic append to output queue

**Multi-level queue optimization**: GPU implementations use a hierarchical queue structure
to improve fast-memory utilization and reduce synchronization overhead. This maps to using
workgroup shared memory as a local queue, then flushing to global storage buffer.

**Performance**: 46x+ speedup reported for multi-agent pathfinding (2025 paper).

**wgpu mapping**: GOOD. The wavefront BFS pattern uses:
- `atomicMin` on `atomic<u32>` in storage buffers (for distance updates)
- `atomicAdd` on `atomic<u32>` for queue append
- `workgroupBarrier()` for intra-workgroup synchronization
- All available in standard WGSL

### 1.5 Hadlock's Algorithm

**References**: https://sites.lafayette.edu/cadapps/main-page/maze-router-app/hadlocks-algorithm/

Hadlock's algorithm is a directed variant of Lee's BFS that biases search toward the
target using a "detour number" (count of steps away from target). It still guarantees
optimal paths but explores fewer cells on average.

**GPU parallelization**: No specific GPU implementation found in the literature. However,
Hadlock's can be viewed as a simplified A* with an integer heuristic (detour count), so
the same GPU parallelization techniques apply. The detour number can be computed locally
per cell without global state, making it thread-safe.

**Relevance**: MEDIUM. If we use wavefront BFS on GPU, the Hadlock bias is a simple
per-cell computation that adds negligible overhead.

---

## 2. Negotiation-Based Routing (PathFinder) on GPU

### 2.1 Corolla: GPU-Accelerated FPGA Routing (2017)

**Paper**: "Corolla: GPU-Accelerated FPGA Routing Based on Subgraph Dynamic Expansion" -- ACM/SIGDA FPGA 2017
- PDF: https://ceca.pku.edu.cn/media/lw/137e5df7dec627f988e07d54ff222857.pdf
- ACM: https://dl.acm.org/doi/10.1145/3020078.3021732

**Key contribution**: First work to demonstrate GPU-accelerated FPGA routing using
PathFinder negotiation. The critical insight is **replacing Dijkstra with Bellman-Ford**
for the inner-loop shortest path search, because Bellman-Ford is GPU-friendly while
Dijkstra is not.

**Architecture**:
1. **Problem size reduction**: Extract routing subgraph per net (not full routing resource
   graph). Limits search space so GPU-friendly Bellman-Ford is competitive with CPU Dijkstra.
2. **Dynamic expansion**: If subgraph is too small for convergence, expand it dynamically.
3. **Three levels of parallelism**:
   - **Single-net node parallelism (SNP)**: Each GPU thread processes one node in Bellman-Ford
   - **Single-net edge parallelism (DEP)**: Each thread processes one edge
   - **Multi-net parallelism**: Route non-conflicting nets simultaneously
4. **Hybrid static/dynamic parallelism**: Combines CUDA static kernel launches with dynamic parallelism for adaptive subgraph expansion.

**Performance**: 18.72x average speedup with tolerable quality loss.

**wgpu mapping**: GOOD with caveats.
- SNP and DEP map directly to compute shader workgroups
- Multi-net parallelism is just multiple independent dispatches (or batched into one)
- **CUDA dynamic parallelism is NOT available in wgpu** -- subgraph expansion must be
  driven from CPU (dispatch new compute pass when expansion needed)
- Subgraph extraction runs on CPU, GPU handles the Bellman-Ford inner loop

**Relevance**: HIGH. This directly validates the approach of GPU-accelerating PathFinder's
inner loop while keeping the outer negotiation loop on CPU.

### 2.2 OrthoRoute: GPU-Accelerated PCB Autorouter (2025)

**Project**: https://github.com/bbenchoff/OrthoRoute
**Write-up**: https://bbenchoff.github.io/pages/OrthoRoute.html

**The only existing GPU-accelerated PCB autorouter** (as of 2025). Written in Python
using CuPy (CUDA) as a KiCad plugin.

**Architecture**:
- Manhattan lattice routing graph (orthogonal grid, alternating H/V layers)
- PathFinder negotiation-based routing
- GPU acceleration via CUDA parallel Dijkstra (actually closer to parallel Bellman-Ford)
  for single-net shortest path
- **Parallelism is intra-net only** -- does not route multiple nets simultaneously
- Pad escape planner handles component-to-grid connections

**PathFinder implementation details**:
- Cost function: standard PathFinder `C(n) = (b_n + h_n) * p_n`
- History decay bug discovered: `history_decay=0.995` caused exponential growth instead
  of convergence -- fixed by removing decay
- Fixed hotset size of 100 nets (adaptive sizing caused oscillation)
- Pressure factor capped at `pres_fac_max=8.0` to prevent late-stage oscillation

**Performance on 8,192-net backplane**:
- 41 hours on 80GB A100 GPU
- 33.5 GB VRAM
- 44,233 vias, 68,975 track segments, 32 layers
- Early iterations: congestion reduced from 9,495 to 5,527 oversubscribed edges (42% improvement)

**Lessons for us**:
1. PathFinder parameter tuning is critical -- the OrthoRoute author spent significant
   time on convergence issues (history decay, oscillation, pressure cap)
2. VRAM usage can be substantial for large boards -- routing graph + distance arrays
3. The GPU wins on the SSSP inner loop, not on the outer negotiation loop
4. Python/CuPy overhead is significant -- a Rust/wgpu implementation should be faster
   for the CPU-side orchestration

**Relevance**: CRITICAL. This is a direct real-world validation of GPU-accelerated PCB
routing using PathFinder. Our implementation will follow the same high-level architecture
but in Rust with wgpu instead of Python with CuPy.

### 2.3 Parallelizing the Negotiation Loop

**Can the PathFinder outer loop be parallelized?**

The negotiation loop has this structure:
```
for iteration in 0..max_iterations:
    rip_up(nets)                          // sequential or parallel
    for net in order(nets):
        route(net)                        // GPU-accelerable (SSSP)
        update_occupancy(net.path)        // must be atomic
    update_history(congestion_map)         // embarrassingly parallel
    update_present_factor()               // scalar update
    if converged(): break
```

**Net routing within an iteration**: Can be parallelized IF nets don't share routing
resources. Multiple approaches exist:
- **Spatial partitioning** (RPPT): Divide board into regions, route non-overlapping
  region nets in parallel (CPU threads in practice)
- **GAN-based batching** (GANGR, 2025): Use WGAN to predict non-conflicting net groups,
  achieve up to 40% runtime reduction
- **Dependency detection** (Bamboo/InstantGR): Route nets simultaneously if their
  horizontal and vertical segments don't overlap

**History/congestion updates**: Embarrassingly parallel -- each cell's history is
independent. This is a perfect GPU workload: one thread per grid cell, update
`history[cell] += overuse_penalty` if `demand[cell] > capacity[cell]`.

**wgpu mapping for congestion updates**: EXCELLENT. A single compute dispatch over the
entire congestion grid. Each thread reads demand and capacity, writes updated history.
No atomics needed (each cell written by exactly one thread).

---

## 3. Global Routing on GPU

### 3.1 InstantGR: Scalable GPU Parallelization for Global Routing (ICCAD 2024)

**Paper**: https://dl.acm.org/doi/10.1145/3676536.3676787
**Code**: https://github.com/cuhk-eda/InstantGR (C++ with CUDA)

**Key contributions**:
- GPU-parallel global routing achieving state-of-the-art on ISPD'24 benchmarks
- 2.16x speedup and 1.6% quality improvement over ISPD'24 contest winner
- Net batching: routes nets simultaneously if their vertical segments don't overlap
  AND horizontal segments don't overlap (they use different routing resources)
- Flexible layer transition technique for DAG-based routing

**Relevance**: MEDIUM. Our global router (Milestone 5) operates on a coarse grid and is
not the bottleneck -- detailed routing (Milestone 6/7) dominates runtime. But the net
batching strategy is applicable to our PathFinder iterations.

### 3.2 GANGR: GAN-Assisted Net Batching (2025)

**Paper**: https://arxiv.org/abs/2511.17665

Uses Wasserstein GANs to learn net-interference patterns for optimal batching. Achieves
40% runtime reduction with only 0.002% quality degradation on ISPD'24 benchmarks.

**Relevance**: LOW for initial implementation (requires ML training infrastructure), but
the concept of intelligent net batching is important for GPU utilization.

### 3.3 ISPD 2024 GPU/ML-Enhanced Global Routing Contest

**Contest**: https://liangrj2014.github.io/ISPD24_contest/
**NVIDIA sponsorship**: https://research.nvidia.com/publication/2024-03_gpuml-enhanced-large-scale-global-routing-contest

Benchmarks with up to 50 million cells. Validates that GPU acceleration for global routing
is a mainstream research direction in EDA.

---

## 4. wgpu Compute Shader Specifics

### 4.1 Available Atomic Operations

**Standard WGSL (all platforms including web)**:
- Types: `atomic<i32>`, `atomic<u32>` ONLY. No f32 atomics in spec.
- Address spaces: `workgroup` and `storage` only
- Operations:
  - `atomicLoad`, `atomicStore`
  - `atomicAdd`, `atomicSub`
  - `atomicMax`, `atomicMin` -- **critical for SSSP**
  - `atomicAnd`, `atomicOr`, `atomicXor`
  - `atomicExchange`
  - `atomicCompareExchangeWeak` -- **CAS, usable for custom atomic ops**

**wgpu native-only extensions** (not available on web, but fine for us):
- `SHADER_FLOAT32_ATOMIC`: f32 atomic load, store, add, sub, exchange
  - Supported on Metal (MSL 3.0+) and Vulkan with extensions
  - **f32 atomicMin/atomicMax NOT available** -- must use CAS loop with bitcast
- `SHADER_INT64_ATOMIC_MIN_MAX`: i64/u64 atomic min/max
- `SHADER_INT64_ATOMIC_ALL_OPS`: full i64/u64 atomic suite

**Implication for routing**: Distances should be stored as `u32` (fixed-point
representation of cost), not `f32`. This avoids needing f32 atomics entirely.
Our cost function `C(n) = (b_n + h_n) * p_n` can be scaled to fixed-point with
sufficient precision (e.g., cost * 1000 as u32, giving 0.001 resolution up to ~4M).

### 4.2 Synchronization

- `workgroupBarrier()` -- synchronize threads within a workgroup
- `storageBarrier()` -- memory fence for storage buffer operations
- No global barrier across workgroups (same as CUDA -- must use multiple dispatches)

### 4.3 Subgroup Operations (wgpu native feature `SUBGROUP`)

Available on Vulkan, DX12, Metal. Enable with `enable subgroups;` in WGSL.

**Useful for routing**:
- `subgroupMin(value)` / `subgroupMax(value)` -- parallel reduction within subgroup
  (equivalent to CUDA warp-level reduction)
- `subgroupAdd(value)` -- sum reduction (for counting active wavefront cells)
- `subgroupBallot(pred)` -- which lanes satisfy predicate (for wavefront tracking)
- `subgroupShuffle(v, id)` -- exchange data between lanes (equivalent to CUDA `__shfl_sync`)
- `subgroupShuffleXor(v, mask)` -- butterfly reduction pattern
- `subgroupExclusiveAdd(v)` -- prefix sum (for compact queue building)
- `subgroupBroadcast(v, id)` -- broadcast from one lane

**These are the WGSL equivalents of CUDA warp-level primitives**. All the key ones are
available. The main CUDA feature NOT available is cooperative groups (cross-workgroup
synchronization without a new dispatch).

### 4.4 Resource Limits

| Limit | Default | Notes |
|-------|---------|-------|
| `max_compute_workgroup_size_x` | 256 | |
| `max_compute_workgroup_size_y` | 256 | |
| `max_compute_workgroup_size_z` | 64 | |
| `max_compute_invocations_per_workgroup` | 256 | Product of x*y*z |
| `max_compute_workgroup_storage_size` | 16,384 bytes | Workgroup shared memory |
| `max_storage_buffer_binding_size` | 128 MiB | Per-binding |
| `max_buffer_size` | 256 MiB | Total per buffer |
| `max_storage_buffers_per_shader_stage` | 8 | |
| `max_compute_workgroups_per_dimension` | 65,535 | Per dispatch axis |
| `max_bind_groups` | 4 | |

**Memory implications for routing**:
- A 1000x1000 grid with 4 layers = 4M cells. Distance array at 4 bytes/cell = 16 MB.
  Easily fits in one storage buffer.
- A 5000x5000 grid with 8 layers = 200M cells. Distance array = 800 MB. Exceeds single
  buffer limit. Would need to split across multiple buffers or reduce grid resolution.
- For typical PCB boards (< 2000 nets, < 2000x2000 grid), memory is not a concern.

**Workgroup shared memory** (16 KB): Useful for local wavefront queues. Can hold 4096
u32 values per workgroup -- enough for local BFS frontier.

### 4.5 Performance Characteristics

**Overhead**:
- Pipeline creation: expensive (shader compilation + validation). Create once, reuse.
- Buffer mapping (CPU readback): asynchronous, requires `device.poll()`. Avoid
  round-trips in inner loops.
- Dispatch overhead: non-trivial. Batch work into large dispatches rather than many small
  ones.
- Validation overhead: ~5-10% vs raw Vulkan/Metal. Acceptable for our use case.

**Best practices**:
- Chain multiple compute passes in one command encoder submission
- Use timestamp queries (`TIMESTAMP_QUERY` feature) for GPU-side profiling
- Workgroup size 64 is a good starting point (matches common subgroup sizes)
- Mark read-only buffers as `var<storage, read>` for driver optimization
- Use `StagingBelt` for frequent small uploads
- Avoid CPU-GPU round-trips for intermediate results

---

## 5. Rust GPU Compute Ecosystem

### 5.1 wgpu (Primary recommendation)

**Repository**: https://github.com/gfx-rs/wgpu
**Crate**: https://crates.io/crates/wgpu

- Pure Rust, cross-platform (Vulkan, Metal, DX12, OpenGL ES, WebGPU)
- Mature, well-maintained (gfx-rs team)
- Compute shaders written in WGSL
- Already used in our viewer crate -- no new dependency
- Validation layer adds safety but ~5-10% overhead

### 5.2 CubeCL (Alternative for multi-backend)

**Repository**: https://github.com/tracel-ai/cubecl
**Crate**: https://crates.io/crates/cubecl

- Write GPU kernels in Rust syntax with `#[cube]` proc macro
- Compiles to WGSL (via wgpu), CUDA, ROCm/HIP
- Automatic vectorization and autotuning
- Used by Burn (ML framework)
- Higher-level abstraction -- less control over memory layout

**Trade-off**: CubeCL would let us write kernels once and run on CUDA (faster on NVIDIA)
or wgpu (portable). But it adds another layer of abstraction and may not expose all the
low-level control we need for routing (custom atomic patterns, precise memory layout).

**Recommendation**: Start with raw wgpu/WGSL for maximum control. Consider CubeCL later
if multi-backend becomes important.

### 5.3 rust-gpu (Rust-to-SPIR-V compiler)

**Repository**: https://github.com/Rust-GPU/rust-gpu

- Compile Rust code directly to SPIR-V (Vulkan shaders)
- Now community-owned (Embark Studios archived original repo Oct 2025)
- ~90% of reference Vulkan shaders successfully ported (as of June 2025)
- Allows sharing code between CPU and GPU (same Rust types)

**Trade-off**: Compelling for code sharing, but adds nightly Rust dependency and build
complexity. WGSL is simpler for the compute kernels we need.

### 5.4 Notable wgpu compute projects

- **wgpu-puzzles**: 14 progressive GPU compute puzzles in WGSL -- good learning resource
  (https://github.com/d4mr/wgpu-puzzles)
- **webgpu-compute-exploration**: SPH fluids, molecular dynamics, boids in WGSL
  (https://github.com/scttfrdmn/webgpu-compute-exploration)
- **Burn**: ML framework using CubeCL/wgpu for GPU compute
  (https://burn.dev/)

---

## 6. Proposed GPU Acceleration Architecture

Based on this research, here is a concrete architecture for GPU-accelerating our router:

### 6.1 What to accelerate (in priority order)

1. **Single-net shortest path** (Milestone 6 `astar.rs`):
   Replace CPU A* with GPU parallel Bellman-Ford on the routing grid.
   Expected speedup: 10-30x per net on boards with > 500 nets.

2. **Congestion/history map updates** (Milestone 7 `history.rs`):
   GPU dispatch over entire 3D grid: one thread per cell.
   Expected speedup: trivial to implement, 50-100x for large grids.

3. **Obstacle bitmap generation** (Milestone 4 `obstacles.rs`):
   GPU rasterization of pad/keepout geometries onto per-layer bitmaps.
   Expected speedup: 5-10x, but only runs once per workspace build.

4. **Multi-net parallel routing** (future optimization):
   Batch non-conflicting nets and route simultaneously on GPU.
   Expected speedup: proportional to batch size (2-8x typical).

### 6.2 What to keep on CPU

- PathFinder outer loop (sequential by nature)
- Net ordering and batching decisions
- Global routing (coarse grid, fast on CPU)
- Trace optimization (sequential per-net, geometry-heavy)
- DRC checking (spatial queries better on CPU with R-tree)

### 6.3 Data layout for GPU

```
Storage Buffer 0: Grid obstacles (per-layer bitmaps, read-only)
    - Packed u32 words, one bit per grid cell per layer
    - Size: ceil(grid_width * grid_height / 32) * num_layers * 4 bytes

Storage Buffer 1: Distance array (read-write)
    - atomic<u32> per cell (fixed-point cost, 0xFFFFFFFF = unvisited)
    - Size: grid_width * grid_height * num_layers * 4 bytes

Storage Buffer 2: History costs (read-write for updates, read-only during routing)
    - u32 per cell (fixed-point)
    - Size: same as distance array

Storage Buffer 3: Predecessor array (write-only during routing)
    - u32 per cell (encoded parent direction for path reconstruction)
    - Size: same as distance array

Uniform Buffer: Routing parameters
    - Grid dimensions, layer count, via cost, present factor, target coords
```

### 6.4 Compute shader outline (WGSL)

```wgsl
// Bellman-Ford iteration for single-net SSSP
@group(0) @binding(0) var<storage, read> obstacles: array<u32>;
@group(0) @binding(1) var<storage, read_write> dist: array<atomic<u32>>;
@group(0) @binding(2) var<storage, read> history: array<u32>;
@group(0) @binding(3) var<storage, read_write> predecessor: array<u32>;
@group(1) @binding(0) var<uniform> params: RoutingParams;

@compute @workgroup_size(64)
fn bellman_ford_step(@builtin(global_invocation_id) gid: vec3<u32>) {
    let cell_idx = gid.x;
    if (cell_idx >= params.total_cells) { return; }

    let current_dist = atomicLoad(&dist[cell_idx]);
    if (current_dist == 0xFFFFFFFFu) { return; }  // unvisited

    // Decode 3D position from linear index
    let layer = cell_idx % params.num_layers;
    let temp = cell_idx / params.num_layers;
    let y = temp % params.grid_height;
    let x = temp / params.grid_height;

    // Try 4 cardinal neighbors + layer transitions
    // For each neighbor:
    //   1. Check obstacle bitmap
    //   2. Compute edge cost = base + history[neighbor] * pres_fac
    //   3. atomicMin(&dist[neighbor], current_dist + edge_cost)
    //   4. If updated, store predecessor direction
}
```

### 6.5 Integration with existing router plan

The GPU acceleration fits as an optional backend behind the existing trait structure:

```
DetailedRouter trait
    |
    +-- GridRouter (CPU, using pathfinding::astar)     [Milestone 6]
    |
    +-- GpuGridRouter (GPU, using wgpu Bellman-Ford)   [Future milestone]
    |
    +-- ShapeRouter (CPU, geometry-based)               [Milestone 6]
```

`RoutingConfig` gains a `gpu_acceleration: bool` field (default false).
When enabled, `build_workspace()` initializes GPU device, creates pipelines and buffers.
`route_single_net()` dispatches compute passes instead of calling `pathfinding::astar`.

---

## 7. Key Technical Decisions for Implementation

### 7.1 Fixed-point costs (not floating-point)

Store all costs as `u32` fixed-point (multiply by 1000). This avoids the need for f32
atomics (not in WGSL spec, native-only extension with limited support). `atomicMin` on
`atomic<u32>` is universally supported and sufficient.

### 7.2 Bellman-Ford over A* on GPU

A* requires a priority queue (inherently sequential). Bellman-Ford relaxes all edges in
parallel. On GPU, the parallelism of Bellman-Ford outweighs A*'s theoretical work
advantage. This is the key insight from Corolla.

### 7.3 Multiple dispatches per convergence

Bellman-Ford needs O(V) iterations in the worst case, but typically converges much faster
on grid graphs. Each iteration is one GPU dispatch. The CPU loop checks for convergence
(no distance updates in last iteration) and dispatches the next iteration. This avoids
needing cooperative groups or global barriers within a single dispatch.

### 7.4 CPU-GPU data transfer minimization

- Upload obstacles and history once per PathFinder iteration (not per net)
- Upload source/target per net (small uniform buffer update)
- Download path (predecessor array) only for final solution, not intermediate
- Keep distance array on GPU between nets (just reset to 0xFFFFFFFF between nets)

### 7.5 When GPU acceleration is NOT worth it

- Small boards (< 100 nets): CPU dispatch overhead dominates
- Trivial routing (most nets are short, straight connections): CPU A* is fast enough
- Systems without a discrete GPU: integrated graphics may be slower than CPU

The `gpu_acceleration` config flag should default to `false`, with a heuristic that
suggests enabling it for boards with > 500 nets and > 1000x1000 grid cells.

---

## 8. References

### Papers

- GAMER: GPU-Accelerated Maze Routing -- https://ieeexplore.ieee.org/document/9799536
- Corolla: GPU-Accelerated FPGA Routing -- https://dl.acm.org/doi/10.1145/3020078.3021732
- InstantGR: Scalable GPU Parallelization for Global Routing -- https://dl.acm.org/doi/10.1145/3676536.3676787
- GANGR: GAN-Assisted Global Routing Parallelization -- https://arxiv.org/abs/2511.17665
- PathFinder: Negotiation-Based Performance-Driven Router -- https://dl.acm.org/doi/10.1145/201310.201328
- Work-Efficient Parallel GPU Methods for SSSP -- https://escholarship.org/content/qt8qr166v2/qt8qr166v2.pdf
- Fast Work-Efficient SSSP for GPUs (PPoPP 2021) -- https://www.cs.utexas.edu/~lin/papers/ppopp21.pdf
- Efficient Irregular Wavefront Propagation on GPUs -- https://arxiv.org/abs/1209.3314
- GPU Accelerated Multi-agent Path Planning -- https://www.sciencedirect.com/science/article/pii/S1877050912003249
- FPGA-Accelerated Maze Routing Kernel -- https://yibolin.com/publications/papers/ROUTE_ASPDAC2022_Jiang.pdf
- Accelerating FPGA Routing Through Parallelization -- https://dl.acm.org/doi/10.1145/3406959
- Delta-Stepping on Compute Shaders -- https://www.execfoo.de/blog/deltastep_shader.html

### Projects and Tools

- OrthoRoute (GPU PCB autorouter for KiCad) -- https://github.com/bbenchoff/OrthoRoute
- InstantGR (GPU global router) -- https://github.com/cuhk-eda/InstantGR
- wgpu (Rust WebGPU implementation) -- https://github.com/gfx-rs/wgpu
- CubeCL (multi-platform GPU compute for Rust) -- https://github.com/tracel-ai/cubecl
- rust-gpu (Rust-to-SPIR-V compiler) -- https://github.com/Rust-GPU/rust-gpu
- wgpu-puzzles (GPU compute learning) -- https://github.com/d4mr/wgpu-puzzles

### Specifications

- WGSL Specification -- https://www.w3.org/TR/WGSL/
- WGSL Atomic/Synchronization Reference -- https://webgpu.rocks/wgsl/functions/synchronization-atomic/
- WGSL Subgroups Proposal -- https://github.com/gpuweb/gpuweb/blob/main/proposals/subgroups.md
- wgpu Limits -- https://docs.rs/wgpu/latest/wgpu/struct.Limits.html
- wgpu Features (native extensions) -- https://docs.rs/wgpu/latest/wgpu/struct.FeaturesWGPU.html
- f32 Atomics Discussion -- https://github.com/gpuweb/gpuweb/issues/4894
