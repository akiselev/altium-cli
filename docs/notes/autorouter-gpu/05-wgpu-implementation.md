# wgpu Implementation Patterns for GPU-Accelerated PCB Routing

Practical implementation guide for building a GPU-accelerated PathFinder router
using wgpu compute shaders (WGSL). Covers memory layout, pipeline management,
CPU-GPU synchronization, shader organization, profiling, and testing.

**Prerequisites**: Read `00-overview.md` first for the architectural context (why
wgpu, resource limits, fixed-point cost encoding, etc.).

---

## Table of Contents

1. [GPU Memory Layout for Routing Grids](#1-gpu-memory-layout-for-routing-grids)
2. [Compute Pipeline Patterns for Iterative Algorithms](#2-compute-pipeline-patterns-for-iterative-algorithms)
3. [CPU-GPU Synchronization Patterns](#3-cpu-gpu-synchronization-patterns)
4. [WGSL Shader Organization](#4-wgsl-shader-organization)
5. [Profiling and Debugging](#5-profiling-and-debugging)
6. [Testing Strategy](#6-testing-strategy)
7. [References](#7-references)

---

## 1. GPU Memory Layout for Routing Grids

### 1.1 Structure-of-Arrays vs Array-of-Structures

Our routing grid stores multiple per-cell values: distance (cost), predecessor,
obstacle mask, history cost, occupancy count. The key layout decision is SoA vs AoS.

**AoS (Array of Structures)** -- how our CPU code does it today:

```wgsl
// BAD for GPU: interleaved fields cause non-coalesced access
struct GridCell {
    distance: u32,      // fixed-point cost
    predecessor: u32,   // encoded (x, y, layer) of parent
    obstacle: u32,      // bitmask: 1 bit per layer
    history: u32,       // fixed-point accumulated history cost
}
@group(0) @binding(0) var<storage, read_write> grid: array<GridCell>;
```

When a Bellman-Ford kernel reads only `distance` for all cells, it loads 16 bytes
per cell but uses only 4. Threads in a workgroup access `grid[tid], grid[tid+1],
grid[tid+2]...` but the distance values are 16 bytes apart. This defeats memory
coalescing on every GPU architecture.

**SoA (Structure of Arrays)** -- the GPU-friendly layout:

```wgsl
// GOOD for GPU: each buffer accessed contiguously
@group(0) @binding(0) var<storage, read_write> distance: array<u32>;
@group(0) @binding(1) var<storage, read_write> predecessor: array<u32>;
@group(0) @binding(2) var<storage, read>       obstacle: array<u32>;
@group(0) @binding(3) var<storage, read_write> history: array<u32>;
```

With SoA, when 64 threads read `distance[tid..tid+63]`, those 256 bytes are
contiguous in memory -- a single cache line fetch on most GPUs. SoA typically
delivers 3-5x bandwidth improvement over AoS for kernels that access a subset
of fields (which is almost all of our kernels).

**Decision**: Use SoA. Each routing grid array is a separate `wgpu::Buffer` bound
to a separate binding slot.

### 1.2 3D Grid Linearization

Our grid is 3D: `(x, y, layer)`. The linearization order determines which
dimension gets coalesced access.

Our CPU code uses `x * (height * layers) + y * layers + layer` (from
`crates/autopcb-router/src/pathfinder/history.rs`). This is x-major order,
meaning incrementing x by 1 jumps `height * layers` elements -- bad for GPU
when adjacent threads process adjacent x values.

For GPU, we want adjacent threads to map to adjacent memory locations. In a
2D dispatch where `global_invocation_id.x` varies fastest within a workgroup:

```
index = layer * (width * height) + y * width + x
```

This is **layer-major, then row-major** order. Adjacent threads (varying in x)
access adjacent memory addresses within a single (layer, y) slice. Each layer
is a contiguous 2D plane in memory, which also simplifies per-layer operations.

```wgsl
fn cell_index(x: u32, y: u32, layer: u32) -> u32 {
    return layer * (grid_width * grid_height) + y * grid_width + x;
}
```

**Conversion between CPU and GPU linearization**: The CPU code can either be
updated to match, or a simple remapping step can be done during upload/download.
Since the CPU code only accesses the history array linearly (not with spatial
locality), the CPU ordering does not matter much -- match the GPU order everywhere.

### 1.3 Multiple Grids: One Buffer vs Separate Buffers

| Approach | Pros | Cons |
|----------|------|------|
| Separate buffers | Clear semantics, independent bind group slots, can mark read-only | More bind group entries (max 8 storage buffers/stage) |
| One large buffer | Single binding, manual offset arithmetic | Complex indexing, cannot mark subregions read-only |

**Decision**: Separate buffers. We have 4-6 grids, which fits within the 8
storage buffer limit per bind group. If we hit the limit, we can pack related
read-only arrays (obstacle + history) into one buffer with offset arithmetic.

### 1.4 WGSL Alignment and Padding

WGSL alignment rules (from the [WebGPU spec](https://www.w3.org/TR/WGSL/#alignment-and-size)):

| Type | Alignment | Size |
|------|-----------|------|
| `u32`, `i32`, `f32` | 4 bytes | 4 bytes |
| `vec2<u32>` | 8 bytes | 8 bytes |
| `vec3<u32>` | **16 bytes** | 12 bytes |
| `vec4<u32>` | 16 bytes | 16 bytes |
| `array<u32, N>` | 4 bytes | 4*N bytes |
| `array<vec3<u32>, N>` | 16 bytes | 16*N bytes (!) |

**Critical pitfall**: `vec3` has alignment 16, not 12. An `array<vec3<f32>>` has
stride 16 (4 bytes wasted per element). For our routing grids this doesn't matter
because we use flat `array<u32>` -- no padding, 4-byte stride, maximum density.

For uniform buffers (grid dimensions, iteration params), use `bytemuck` on the
Rust side with explicit padding fields:

```rust
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct GridParams {
    width: u32,
    height: u32,
    layer_count: u32,
    iteration: u32,           // 16 bytes, aligned
    pres_fac_fp: u32,         // fixed-point present congestion factor
    history_fac_fp: u32,      // fixed-point history factor
    base_cost_fp: u32,        // fixed-point base move cost
    via_cost_fp: u32,         // fixed-point via cost  -- 32 bytes total
    source_cell: u32,         // linearized source index
    target_cell: u32,         // linearized target index
    _pad: [u32; 2],           // pad to 48 bytes (multiple of 16)
}
```

**Rule of thumb**: Always make uniform buffer structs a multiple of 16 bytes.
Storage buffers are more lenient (4-byte alignment for `array<u32>`), but uniform
buffers require the struct's total size to be a multiple of its alignment.

### 1.5 Per-Layer Data Organization

For obstacle bitmaps, pack multiple layers into a single `u32` bitmask per cell:

```wgsl
// Each cell stores a 32-bit mask: bit i = blocked on layer i
// Supports up to 32 copper layers (more than any real PCB)
@group(0) @binding(2) var<storage, read> obstacle_mask: array<u32>;

fn is_blocked(x: u32, y: u32, layer: u32) -> bool {
    let idx = y * grid_width + x;  // 2D index (layer encoded in bitmask)
    return (obstacle_mask[idx] & (1u << layer)) != 0u;
}
```

This avoids per-layer buffer duplication for obstacles. The 2D obstacle bitmap
is `width * height` u32 values -- 4 bytes/cell regardless of layer count.

For per-layer arrays that need full u32 range (distance, predecessor, history),
use the 3D linearization from section 1.2.

---

## 2. Compute Pipeline Patterns for Iterative Algorithms

### 2.1 Bellman-Ford on GPU: Multi-Pass Architecture

The GPU Bellman-Ford algorithm parallelizes edge relaxation. For a grid graph,
every cell is a vertex with 4-6 neighbors (cardinal moves + via transitions).
Each iteration, every cell attempts to relax its neighbors in parallel.

**Kernel decomposition**:

| Kernel | Purpose | Frequency |
|--------|---------|-----------|
| `reset_distances` | Set all distances to INFINITY, predecessor to NONE | Once per net |
| `set_source` | Set source cell distance to 0 | Once per net |
| `bellman_ford_relax` | Relax all edges in parallel | N times per convergence check |
| `check_convergence` | Reduce change-flag buffer to single value | Once per N iterations |
| `update_history` | Increment history for oversubscribed cells | Once per PathFinder iteration |
| `rip_up` | Clear distance/predecessor for ripped-up nets | Once per rip-up |

### 2.2 Convergence Detection Without Per-Iteration CPU Readback

The naive approach -- read back a "changed" flag after every Bellman-Ford
iteration -- kills performance. Each readback requires:
1. `copy_buffer_to_buffer` (GPU storage -> staging)
2. `queue.submit()`
3. `staging.slice(..).map_async()`
4. `device.poll(Maintain::Wait)` (CPU blocks)

This pipeline stall costs 50-200us per readback, dominating iteration time for
small grids.

**Solution: Amortized convergence checking with atomic change flag.**

```wgsl
// Change flag: single atomic u32. 0 = no changes, 1 = at least one relaxation occurred.
@group(1) @binding(0) var<storage, read_write> change_flag: atomic<u32>;

@compute @workgroup_size(64)
fn bellman_ford_relax(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = cell_index(gid.x, gid.y, gid.z);
    if idx >= total_cells { return; }

    let my_dist = distance[idx];
    // For each neighbor...
    let neighbor_idx = /* compute neighbor */;
    let edge_cost = /* base + history + present congestion */;
    let new_dist = my_dist + edge_cost;

    if new_dist < distance[neighbor_idx] {
        // Fixed-point atomicMin for relaxation
        atomicMin(&distance_atomic[neighbor_idx], new_dist);
        // Signal that a change occurred
        atomicStore(&change_flag, 1u);
    }
}
```

**Dispatch pattern: run N iterations, then check.**

```rust
const ITERATIONS_BETWEEN_CHECKS: u32 = 8; // tunable
const MAX_TOTAL_ITERATIONS: u32 = 256;     // safety cap

let mut total_iters = 0u32;

loop {
    // Reset change flag to 0
    queue.write_buffer(&change_flag_buf, 0, &0u32.to_le_bytes());

    // Dispatch N relaxation iterations without readback
    let mut encoder = device.create_command_encoder(&Default::default());
    for _ in 0..ITERATIONS_BETWEEN_CHECKS {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&bellman_ford_pipeline);
        pass.set_bind_group(0, &grid_bind_group, &[]);
        pass.set_bind_group(1, &flag_bind_group, &[]);
        pass.dispatch_workgroups(
            (grid_width + 7) / 8,
            (grid_height + 7) / 8,
            layer_count,
        );
    }

    // Copy change flag to staging buffer
    encoder.copy_buffer_to_buffer(
        &change_flag_buf, 0,
        &change_flag_staging, 0,
        4,
    );
    queue.submit(std::iter::once(encoder.finish()));

    // Read back the flag (one stall per N iterations)
    let converged = read_back_u32(&device, &change_flag_staging) == 0;
    total_iters += ITERATIONS_BETWEEN_CHECKS;

    if converged || total_iters >= MAX_TOTAL_ITERATIONS {
        break;
    }
}
```

**Why this works**: Bellman-Ford on a grid with diameter D converges in at most D
iterations. For a 500x500 grid, D~1000. With `ITERATIONS_BETWEEN_CHECKS=8`, we
do ~125 readbacks instead of ~1000. The convergence check adds 1 stall per 8
iterations -- acceptable overhead.

**Alternative: GPU-side reduction**. Instead of `atomicStore(&change_flag, 1u)`,
use a hierarchical reduction (workgroup-local reduction via shared memory, then
global atomic). This is more complex but avoids false convergence from race
conditions. For our grid sizes, the simple atomic flag is sufficient.

### 2.3 Pipeline Caching and Reuse Across Nets

Creating a `wgpu::ComputePipeline` involves shader compilation -- expensive (1-50ms).
All pipelines must be created once at startup and reused for every net.

```rust
struct GpuRouter {
    device: wgpu::Device,
    queue: wgpu::Queue,

    // Pipelines (created once, reused for every net)
    reset_pipeline: wgpu::ComputePipeline,
    bellman_ford_pipeline: wgpu::ComputePipeline,
    update_history_pipeline: wgpu::ComputePipeline,
    trace_back_pipeline: wgpu::ComputePipeline,

    // Bind group layouts (shared across pipelines)
    grid_bgl: wgpu::BindGroupLayout,
    params_bgl: wgpu::BindGroupLayout,

    // Persistent GPU buffers (allocated for the board, reused across nets)
    distance_buf: wgpu::Buffer,
    predecessor_buf: wgpu::Buffer,
    obstacle_buf: wgpu::Buffer,
    history_buf: wgpu::Buffer,
    change_flag_buf: wgpu::Buffer,

    // Staging buffers for readback
    predecessor_staging: wgpu::Buffer,
    change_flag_staging: wgpu::Buffer,

    // Uniform buffer (updated per-net with source/target)
    params_buf: wgpu::Buffer,
}
```

### 2.4 Bind Group Management

Bind groups define which GPU buffers are visible to a shader dispatch. The key
question is: which bindings change per net vs per iteration?

| Resource | Changes per... | Binding strategy |
|----------|---------------|------------------|
| Distance array | Reset per net, modified per iteration | Group 0 (grid data) |
| Predecessor array | Reset per net, modified per iteration | Group 0 |
| Obstacle bitmap | Never (fixed for board) | Group 0 |
| History array | Once per PathFinder outer iteration | Group 0 |
| Change flag | Reset per convergence check batch | Group 1 (flags) |
| Grid params (source, target, costs) | Per net | Group 2 (uniform) |

```rust
// Group 0: grid data (same bind group for all kernels, all nets)
let grid_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
    layout: &grid_bgl,
    entries: &[
        wgpu::BindGroupEntry { binding: 0, resource: distance_buf.as_entire_binding() },
        wgpu::BindGroupEntry { binding: 1, resource: predecessor_buf.as_entire_binding() },
        wgpu::BindGroupEntry { binding: 2, resource: obstacle_buf.as_entire_binding() },
        wgpu::BindGroupEntry { binding: 3, resource: history_buf.as_entire_binding() },
    ],
    label: Some("grid_bind_group"),
});

// Group 2: params uniform (recreated per net, or use write_buffer to update)
// Using queue.write_buffer is cheaper than recreating the bind group:
queue.write_buffer(&params_buf, 0, bytemuck::bytes_of(&GridParams {
    source_cell: linearize(source),
    target_cell: linearize(target),
    // ...
}));
```

**Best practice**: Minimize bind group recreation. Use `queue.write_buffer()` to
update uniform contents rather than creating new bind groups. Bind group creation
is cheap but not free -- it allocates on the GPU timeline.

---

## 3. CPU-GPU Synchronization Patterns

### 3.1 The Core Problem: Minimizing Round-Trips

Every CPU-GPU synchronization point is a potential pipeline stall:

```
CPU: submit commands ──> idle (waiting) ──> read result ──> submit next
GPU:                  ──> execute        ──> idle          ──> execute
```

The idle gaps are the performance killer. Our PathFinder loop has inherent
serialization points (read back converged flag, read back path for trace
reconstruction), but we can minimize them.

**Pipeline of one net**:
1. Upload: source/target coords (tiny, ~48 bytes)
2. GPU: reset + N relaxation iterations + convergence check
3. Download: converged flag (4 bytes)
4. Repeat 2-3 until converged
5. Download: predecessor array for path reconstruction
6. CPU: trace back path, update occupancy

Steps 1 and 3 are tiny transfers. Step 5 is the largest: `width * height *
layer_count * 4` bytes. For a 500x500 grid with 4 layers = 4MB. At PCIe 3.0
bandwidth (~12 GB/s), this takes ~0.3ms -- fast enough.

### 3.2 Double-Buffered Overlapping

For the outer PathFinder loop (route net 0, then net 1, ...), we can overlap
CPU trace-back of net N with GPU routing of net N+1:

```rust
// Simplified double-buffer pipeline
let mut pending_readback: Option<(NetId, wgpu::Buffer)> = None;

for net_id in net_order {
    // If previous net's readback is ready, trace back on CPU
    if let Some((prev_net, staging)) = pending_readback.take() {
        let path = read_predecessor_and_trace_back(&device, &staging);
        update_occupancy(&mut occupancy, &path, prev_net);
        staging.unmap();
    }

    // Upload source/target for this net
    queue.write_buffer(&params_buf, 0, bytemuck::bytes_of(&net_params));

    // GPU: reset + Bellman-Ford + copy predecessor to staging
    let mut encoder = device.create_command_encoder(&Default::default());
    dispatch_reset(&mut encoder, ...);
    dispatch_bellman_ford_loop(&mut encoder, ...);
    encoder.copy_buffer_to_buffer(&predecessor_buf, 0, &staging_a, 0, pred_size);
    queue.submit(std::iter::once(encoder.finish()));

    // Queue the readback (don't wait yet)
    staging_a.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    pending_readback = Some((net_id, staging_a));

    // Swap staging buffers
    std::mem::swap(&mut staging_a, &mut staging_b);
}

// Drain the last readback
device.poll(wgpu::Maintain::Wait);
if let Some((prev_net, staging)) = pending_readback {
    let path = read_predecessor_and_trace_back(&device, &staging);
    update_occupancy(&mut occupancy, &path, prev_net);
}
```

This overlapping is only beneficial when CPU trace-back time is significant
compared to GPU routing time. For small nets it may not help; for large nets
with complex paths it hides the CPU work behind GPU compute.

### 3.3 map_async vs poll(Wait)

- **`buffer.slice(..).map_async(MapMode::Read, callback)`**: Queues a request
  to map the buffer. The callback fires when the GPU is done and the mapping is
  ready. Does NOT block.

- **`device.poll(Maintain::Wait)`**: Blocks the CPU thread until all pending GPU
  work AND pending map operations are complete. Required on native platforms to
  drive the callback.

- **`device.poll(Maintain::Poll)`**: Non-blocking check. Returns immediately,
  fires any callbacks whose GPU work has completed.

**Pattern for our router**:

```rust
// Non-blocking: check if previous net's readback is ready
device.poll(wgpu::Maintain::Poll);
if staging.slice(..).get_mapped_range_mut().is_some() {
    // Ready! Process it.
}
// ... otherwise, submit more GPU work and check later.

// Blocking: when we need the result NOW (e.g., final net, convergence check)
device.poll(wgpu::Maintain::Wait);
let data = staging.slice(..).get_mapped_range();
```

### 3.4 StagingBelt for Frequent Small Uploads

`wgpu::util::StagingBelt` maintains a pool of reusable staging buffers for
CPU-to-GPU uploads, avoiding per-upload allocation overhead.

```rust
use wgpu::util::StagingBelt;

let mut belt = StagingBelt::new(1024); // 1KB chunk size

// Per-net upload of source/target params
belt.write_buffer(
    &mut encoder,
    &params_buf,
    0,
    std::num::NonZeroU64::new(std::mem::size_of::<GridParams>() as u64).unwrap(),
    &device,
).copy_from_slice(bytemuck::bytes_of(&params));

// After submit
belt.finish();       // mark current chunks as submitted
belt.recall();       // reclaim chunks that the GPU has finished using
```

For our router, the per-net uploads are tiny (48-64 bytes), so `StagingBelt`
provides marginal benefit over `queue.write_buffer()`. It becomes valuable
if we batch many small updates (e.g., updating occupancy for multiple cells).

### 3.5 Reading Back the Predecessor Array for Path Reconstruction

After Bellman-Ford converges, we need the predecessor array on the CPU to
trace back the shortest path from target to source.

**Encoding**: Pack predecessor as `(x, y, layer)` into a single `u32`:

```wgsl
// Encoding: x in bits [0..11], y in bits [11..22], layer in bits [22..26]
// Supports grids up to 2048 x 2048 with 16 layers
const NONE: u32 = 0xFFFFFFFFu;

fn encode_predecessor(x: u32, y: u32, layer: u32) -> u32 {
    return x | (y << 11u) | (layer << 22u);
}

fn decode_x(pred: u32) -> u32 { return pred & 0x7FFu; }
fn decode_y(pred: u32) -> u32 { return (pred >> 11u) & 0x7FFu; }
fn decode_layer(pred: u32) -> u32 { return (pred >> 22u) & 0xFu; }
```

**Readback**:

```rust
// 1. Copy predecessor buffer to staging
encoder.copy_buffer_to_buffer(
    &predecessor_buf, 0, &predecessor_staging, 0,
    (total_cells * 4) as u64,
);
queue.submit(std::iter::once(encoder.finish()));

// 2. Map and read
predecessor_staging.slice(..).map_async(wgpu::MapMode::Read, |_| {});
device.poll(wgpu::Maintain::Wait);

let data = predecessor_staging.slice(..).get_mapped_range();
let predecessors: &[u32] = bytemuck::cast_slice(&data);

// 3. Trace back path on CPU (sequential, fast)
let mut path = Vec::new();
let mut cell = target_index;
while cell != source_index && predecessors[cell as usize] != NONE {
    path.push(cell);
    cell = predecessors[cell as usize];
}
path.push(source_index);
path.reverse();
```

**Optimization**: Only read back the portion of the predecessor array that was
actually modified. Track the bounding box of modified cells on the GPU (using
`atomicMin`/`atomicMax` on x/y bounds) and read back only that subregion.
For sparse paths on large grids, this can reduce readback by 10-100x. However,
the complexity may not be worth it for PCB-scale grids where the full readback
is already fast (<1ms).

---

## 4. WGSL Shader Organization

### 4.1 Shader File Structure

Organize compute shaders by function, one kernel per file:

```
crates/autopcb-router/src/gpu/
    shaders/
        types.wgsl          # shared type definitions (included by all)
        reset.wgsl           # reset_distances kernel
        bellman_ford.wgsl    # edge relaxation kernel
        convergence.wgsl     # change-flag reduction kernel
        history_update.wgsl  # increment history for congested cells
        trace_back.wgsl      # (optional) GPU-side path trace-back
    mod.rs                   # GpuRouter struct, pipeline creation
    buffers.rs               # buffer allocation and management
    dispatch.rs              # command encoding and dispatch logic
```

Each `.wgsl` file includes the shared type definitions via WGSL's preprocessor
or via string concatenation at load time (WGSL has no `#include`; the standard
approach is to concatenate shared preamble + kernel source in Rust):

```rust
fn load_shader(device: &wgpu::Device, kernel_source: &str) -> wgpu::ShaderModule {
    let preamble = include_str!("shaders/types.wgsl");
    let full_source = format!("{preamble}\n{kernel_source}");
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("router_shader"),
        source: wgpu::ShaderSource::Wgsl(full_source.into()),
    })
}
```

### 4.2 Shared Types Between CPU (Rust) and GPU (WGSL)

There is no automatic type sharing between Rust and WGSL. The approaches are:

**A. Manual sync (simplest, what Vello does)**:
Define matching structs in both languages and rely on tests to catch mismatches.

```rust
// Rust side
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct GridParams {
    width: u32,
    height: u32,
    layer_count: u32,
    iteration: u32,
    // ...
}
```

```wgsl
// WGSL side (types.wgsl)
struct GridParams {
    width: u32,
    height: u32,
    layer_count: u32,
    iteration: u32,
    // ...
}
```

**B. Code generation (what Burn/CubeCL does)**:
Generate WGSL from Rust types using a build script or proc macro. CubeCL's
`#[cube]` macro is the most advanced example -- it compiles Rust-like GPU
kernel code to WGSL at compile time. For our relatively simple shaders, this
is overkill.

**C. Constants as source of truth**:
Define layout constants (field offsets, buffer sizes) in Rust and embed them
into WGSL via string formatting:

```rust
let wgsl = format!(
    "const GRID_WIDTH: u32 = {w}u;\nconst GRID_HEIGHT: u32 = {h}u;\n{shader_body}",
    w = grid.width_cells,
    h = grid.height_cells,
    shader_body = include_str!("shaders/bellman_ford.wgsl"),
);
```

**Decision**: Use manual sync (option A) with a validation test that checks
`std::mem::size_of::<GridParams>()` matches the expected WGSL struct size.
Keep constant values (grid dimensions, costs) in the uniform buffer rather
than baking them into shader source, so pipelines don't need recompilation
when grid size changes.

### 4.3 Override Constants for Compile-Time Specialization

WGSL `override` declarations allow setting values at pipeline creation time
without recompiling the shader module:

```wgsl
// bellman_ford.wgsl
override WORKGROUP_SIZE_X: u32 = 8;
override WORKGROUP_SIZE_Y: u32 = 8;
override MOVEMENT_FOUR_WAY: bool = true;  // false = 8-way

@compute @workgroup_size(WORKGROUP_SIZE_X, WORKGROUP_SIZE_Y, 1)
fn bellman_ford_relax(@builtin(global_invocation_id) gid: vec3<u32>) {
    // ...
    if MOVEMENT_FOUR_WAY {
        // 4 neighbors
    } else {
        // 8 neighbors (diagonals)
    }
}
```

```rust
let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
    label: Some("bellman_ford"),
    layout: Some(&pipeline_layout),
    module: &shader_module,
    entry_point: Some("bellman_ford_relax"),
    compilation_options: wgpu::PipelineCompilationOptions {
        constants: &[
            ("WORKGROUP_SIZE_X".to_string(), 16.0),  // override for this GPU
            ("MOVEMENT_FOUR_WAY".to_string(), 0.0),  // false = 8-way
        ].into_iter().collect(),
        ..Default::default()
    },
    cache: None,
});
```

**Limitations**: Override constants can only be scalar values (bool, int, float).
No vectors, matrices, or arrays. They are applied per-pipeline, not per-dispatch.

**Use cases for our router**:
- Workgroup size tuning per GPU vendor
- Movement style (4-way vs 8-way) -- avoids runtime branch divergence
- Maximum layer count (allows compile-time loop unrolling for via checks)

### 4.4 Push Constants (Not Available in WebGPU)

Push constants (small per-draw/dispatch data without buffer allocation) are
available in Vulkan but NOT in the WebGPU/WGSL standard. wgpu supports them
only as a native-only feature via `Features::PUSH_CONSTANTS`.

**Alternative**: Use a small uniform buffer updated via `queue.write_buffer()`.
This is what we do for `GridParams` (source/target per net). The overhead is
negligible for our use case (<100 bytes per net).

---

## 5. Profiling and Debugging

### 5.1 wgpu-profiler for GPU Timestamp Profiling

[`wgpu-profiler`](https://github.com/Wumpf/wgpu-profiler) provides nested
GPU-side timestamp scopes with minimal overhead.

**Setup**:

```rust
use wgpu_profiler::{GpuProfiler, GpuProfilerSettings};

// Request timestamp features during device creation
let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
    required_features: wgpu::Features::TIMESTAMP_QUERY
        | wgpu::Features::TIMESTAMP_QUERY_INSIDE_PASSES
        | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS,
    ..Default::default()
}, None).await?;

let mut profiler = GpuProfiler::new(GpuProfilerSettings::default())
    .expect("Failed to create profiler");
```

**Scoped compute pass profiling**:

```rust
let mut encoder = device.create_command_encoder(&Default::default());

{
    let mut scope = profiler.scope("pathfinder_iteration", &mut encoder, &device);

    // Profile individual kernels
    {
        let mut pass = scope.scoped_compute_pass("bellman_ford", &device);
        pass.set_pipeline(&bf_pipeline);
        pass.set_bind_group(0, &grid_bg, &[]);
        pass.dispatch_workgroups(wg_x, wg_y, wg_z);
    }

    {
        let mut pass = scope.scoped_compute_pass("history_update", &device);
        pass.set_pipeline(&history_pipeline);
        pass.set_bind_group(0, &grid_bg, &[]);
        pass.dispatch_workgroups(wg_x, wg_y, 1);
    }
}

profiler.resolve_queries(&mut encoder);
queue.submit(std::iter::once(encoder.finish()));
profiler.end_frame().unwrap();

// Retrieve results (non-blocking -- returns None if GPU hasn't finished)
if let Some(results) = profiler.process_finished_frame(queue.get_timestamp_period()) {
    // Write Chrome trace format for chrome://tracing
    wgpu_profiler::chrometrace::write_chrometrace(
        Path::new("router_trace.json"),
        &results,
    );
}
```

The Chrome trace output can be loaded in `chrome://tracing` or
[Perfetto](https://ui.perfetto.dev/) for visual timeline analysis.

### 5.2 Debugging WGSL Compute Shaders

WGSL has no `printf`. Debugging strategies:

**A. Buffer-based printf (write-to-buffer debugging)**:

```wgsl
@group(2) @binding(0) var<storage, read_write> debug_buf: array<u32>;
@group(2) @binding(1) var<storage, read_write> debug_counter: atomic<u32>;

fn debug_print(value: u32) {
    let idx = atomicAdd(&debug_counter, 1u);
    if idx < arrayLength(&debug_buf) {
        debug_buf[idx] = value;
    }
}
```

Read back `debug_buf` on the CPU to inspect values. Gate behind a compile-time
flag to avoid overhead in release builds:

```wgsl
override DEBUG_ENABLED: bool = false;

fn debug_print(value: u32) {
    if DEBUG_ENABLED {
        let idx = atomicAdd(&debug_counter, 1u);
        if idx < arrayLength(&debug_buf) {
            debug_buf[idx] = value;
        }
    }
}
```

**B. Visualization debugging**:
Write intermediate results (distance field, history costs) to a texture and
render it in the viewer. This is extremely effective for routing -- you can see
the wavefront propagation, identify blocked regions, and spot cost anomalies.

```wgsl
// Write distance field as color to a texture for visualization
@group(2) @binding(0) var debug_texture: texture_storage_2d<rgba8unorm, write>;

fn debug_visualize_distance(x: u32, y: u32, dist: u32) {
    let normalized = f32(dist) / f32(MAX_DISTANCE);
    let color = vec4<f32>(normalized, 1.0 - normalized, 0.0, 1.0);
    textureStore(debug_texture, vec2<i32>(i32(x), i32(y)), color);
}
```

**C. Single-cell debugging**:
Set a "debug cell" in the uniform buffer. The shader writes detailed info
(all neighbor costs, relaxation decisions) only for that cell. Read back a
small fixed-size debug struct.

### 5.3 Validation Layer Messages

wgpu's validation layer catches many errors at submit time. Enable it via:

```rust
let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
    flags: wgpu::InstanceFlags::VALIDATION | wgpu::InstanceFlags::GPU_BASED_VALIDATION,
    ..Default::default()
});
```

**Common pitfalls**:
- Buffer too small for bind group binding (off-by-one in size calculation)
- Dispatch workgroup count exceeds `maxComputeWorkgroupsPerDimension` (65535)
- Storage buffer not aligned to 4 bytes
- Uniform buffer size not a multiple of its alignment
- Read-after-write hazard between passes (wgpu inserts barriers automatically
  for resources used in the same command encoder, but be aware of this)

### 5.4 How Vello and Burn Debug GPU Compute

**Vello's approach**:
- CPU reference implementations for every WGSL shader in `vello_shaders/src/cpu/`
- Run the same scene through both CPU and GPU paths, compare outputs
- The `CpuBinding` abstraction provides identical resource access patterns
- Useful for isolating whether a bug is in shader logic or GPU pipeline setup

**Burn/CubeCL's approach**:
- CubeCL compiles the same `#[cube]` kernel to both WGSL and native Rust
- The Rust version runs on CPU for debugging with standard tools (debugger, logs)
- Autotune benchmarks different kernel variants automatically
- Memory pool tracking detects leaks and double-frees

Both projects validate GPU outputs against CPU reference implementations.
This is the gold standard for GPU compute testing.

---

## 6. Testing Strategy

### 6.1 CPU Reference Implementation

Maintain a pure-Rust Bellman-Ford implementation that produces identical results
to the GPU version. This already exists in our codebase -- the A* router in
`crates/autopcb-router/src/detailed/grid.rs` and the history array in
`crates/autopcb-router/src/pathfinder/history.rs`.

For GPU validation, add a simpler Bellman-Ford reference (not A*, since the GPU
version is Bellman-Ford, not A*):

```rust
/// CPU reference Bellman-Ford for testing GPU correctness.
/// Same algorithm, same cost function, deterministic output.
fn cpu_bellman_ford(
    obstacle_mask: &[u32],
    history: &[u32],
    width: u32, height: u32, layer_count: u32,
    source: (u32, u32, u32),
    target: (u32, u32, u32),
    params: &GridParams,
) -> (Vec<u32>, Vec<u32>) {
    // Returns (distance, predecessor) arrays with identical layout to GPU
    // ...
}
```

**Test pattern**:

```rust
#[test]
fn gpu_matches_cpu_reference() {
    let gpu_result = gpu_bellman_ford(&grid, source, target, &params);
    let cpu_result = cpu_bellman_ford(&grid, source, target, &params);

    // Distance arrays must be identical (fixed-point, deterministic)
    assert_eq!(gpu_result.distances, cpu_result.distances);

    // Predecessor arrays may differ (multiple shortest paths exist),
    // but the path cost must be identical
    let gpu_path_cost = trace_back_cost(&gpu_result);
    let cpu_path_cost = trace_back_cost(&cpu_result);
    assert_eq!(gpu_path_cost, cpu_path_cost);
}
```

### 6.2 Determinism Testing

GPU Bellman-Ford with `atomicMin` is deterministic for the distance array
(minimum is unique) but NOT deterministic for the predecessor array when
multiple paths have the same cost. This is acceptable -- we care about path
quality, not path identity.

**What to test for determinism**:
- Same input -> same distance array (yes, deterministic via `atomicMin`)
- Same input -> same total path cost (yes)
- Same input -> same predecessor array (NO -- thread scheduling varies)

**Fixed-point encoding ensures exact comparison**: By using `u32` fixed-point
costs instead of `f32`, we avoid floating-point non-associativity issues.
`atomicMin` on `u32` is fully deterministic.

### 6.3 Edge Cases

| Edge case | What to test |
|-----------|-------------|
| Source == target | Distance = 0, empty path |
| Source or target blocked | Graceful failure, INFINITY distance |
| Fully blocked grid | All distances = INFINITY |
| Single-cell path | 1 cell, no predecessor chain |
| Grid boundary cells | Neighbors at edges don't wrap or OOB |
| Maximum grid size | Dispatch workgroup count within limits |
| All cells on same layer | No via transitions |
| Dense obstacles | Path exists but serpentine |
| Disconnected regions | Target unreachable, clean failure |

### 6.4 Handling GPU Unavailability (CI, Headless Servers)

wgpu can fail to find an adapter on headless CI machines without GPU hardware.
Strategies:

**A. Feature-gated GPU tests**:

```rust
#[cfg(feature = "gpu-tests")]
#[test]
fn gpu_bellman_ford_basic() {
    let adapter = pollster::block_on(try_get_adapter());
    let adapter = match adapter {
        Some(a) => a,
        None => {
            eprintln!("Skipping GPU test: no adapter available");
            return;
        }
    };
    // ...
}

async fn try_get_adapter() -> Option<wgpu::Adapter> {
    let instance = wgpu::Instance::default();
    instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    }).await
}
```

**B. CPU fallback for logic tests**:
Test the shader logic via the CPU reference implementation on all CI. Only test
GPU-specific behavior (performance, memory limits, pipeline correctness) with
the `gpu-tests` feature flag.

**C. Software rasterizer fallback**:
wgpu supports `force_fallback_adapter: true` to use a software Vulkan
implementation (lavapipe on Linux, SwiftShader via Vulkan). This is extremely
slow but can validate pipeline correctness without hardware GPU. However, not
all CI environments have lavapipe installed.

**Recommended CI strategy**:

```toml
# Cargo.toml
[features]
gpu-tests = []  # Run GPU tests (requires hardware or software GPU)
```

```yaml
# CI: run CPU tests always, GPU tests only on GPU runners
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - run: cargo test --workspace  # CPU tests only
  test-gpu:
    runs-on: [self-hosted, gpu]      # GPU runner
    steps:
      - run: cargo test --workspace --features gpu-tests
```

---

## 7. References

### wgpu and WebGPU

- [wgpu documentation](https://docs.rs/wgpu/) -- Official Rust API docs
- [WebGPU Fundamentals - Compute Shaders](https://webgpufundamentals.org/webgpu/lessons/webgpu-compute-shaders.html) -- Dispatch patterns, workgroup sizing
- [WebGPU Fundamentals - Memory Layout](https://webgpufundamentals.org/webgpu/lessons/webgpu-memory-layout.html) -- WGSL alignment and padding rules
- [Learn wgpu - Memory Layout](https://sotrh.github.io/learn-wgpu/showcase/alignment/) -- Rust/WGSL struct alignment
- [WebGPU Fundamentals - Shader Constants](https://webgpufundamentals.org/webgpu/lessons/webgpu-constants.html) -- Override constants
- [High Performance GPGPU with Rust and wgpu](https://dev.to/jaysmito101/high-performance-gpgpu-with-rust-and-wgpu-4l9i) -- Compute pipeline patterns
- [Rust wgpu Compute: Minimal Example and Performance Tips](https://tillcode.com/rust-wgpu-compute-minimal-example-buffer-readback-and-performance-tips/) -- Buffer readback, staging patterns

### GPU Profiling and Debugging

- [wgpu-profiler](https://github.com/Wumpf/wgpu-profiler) -- Timestamp-based GPU profiling with Chrome trace export
- [WebGPU Debugging and Errors](https://webgpufundamentals.org/webgpu/lessons/webgpu-debugging.html) -- Validation layer usage

### Project References (GPU Compute in Rust)

- [Vello](https://github.com/linebender/vello) -- GPU compute-centric 2D renderer; multi-pass pipeline, CPU shader fallbacks, `vello_shaders/` organization
- [Vello Architecture (DeepWiki)](https://deepwiki.com/linebender/vello/1.1-architecture) -- Pipeline stages, `ResourcePool`, `BindMap`
- [Burn](https://burn.dev/) -- Deep learning framework; wgpu backend, async compute, memory pooling
- [CubeCL](https://github.com/tracel-ai/cubecl) -- Burn's compute abstraction; compile Rust to WGSL, memory pools, autotuning
- [Burn-Compute Blog](https://burn.dev/blog/creating-high-performance-asynchronous-backends-with-burn-compute/) -- Async GPU execution, memory management

### GPU Bellman-Ford

- [Bellman-Ford on GPU using CUDA (Towards Data Science)](https://towardsdatascience.com/bellman-ford-single-source-shortest-path-algorithm-on-gpu-using-cuda-a358da20144b/) -- Parallelization strategy, active vertex flag pruning
- [Work-Efficient Parallel GPU Methods for Single-Source Shortest Paths](https://escholarship.org/content/qt8qr166v2/qt8qr166v2_noSplash_be4e051e39b35de331f67c483ccb78a7.pdf) -- Academic reference for GPU SSSP algorithms

### GPU Memory Layout

- [SoA vs AoS (NVIDIA Forums)](https://forums.developer.nvidia.com/t/structures-of-arrays-vs-arrays-of-structures/13581) -- Memory coalescing analysis
- [Raph Levien - Requiem for piet-gpu-hal](https://raphlinus.github.io/rust/gpu/2023/01/07/requiem-piet-gpu-hal.html) -- Why Vello moved to wgpu, lessons learned
