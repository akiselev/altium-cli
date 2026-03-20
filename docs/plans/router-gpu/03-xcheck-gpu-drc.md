# GPU-Accelerated Design Rule Checking for the AutoPCB Router

## Overview

This plan describes how to implement GPU-accelerated DRC for the autorouter's PathFinder
loop, drawing on the X-Check parallel sweepline algorithm (He, Ma, Yu -- ICCAD 2022),
OpenDRC's hierarchical GPU acceleration (DAC 2023), and PDRC's non-Manhattan extension
(DAC 2024). DRC serves two purposes in the router:

1. **Routing-time DRC**: Clearance/short violations drive PathFinder cost updates and
   rip-up decisions. Runs every iteration. Must be fast.
2. **Final validation DRC**: After routing converges, a comprehensive check before
   emitting the `RouteSolution`. Can be slower and more thorough.

The key insight is that routing-time DRC does not need the full generality of a standalone
DRC engine. The router already knows all segment geometries, net assignments, and clearance
rules. We exploit this to build a simpler, faster GPU pipeline than a general-purpose
checker.

---

## Pipeline Integration

X-Check (this plan) implements the GPU DRC pass — step 4 of each PathFinder iteration, immediately after all nets in a batch are routed.

```
PathFinder Iteration:
  1. Rip-up (CPU)
  2. InstantGR (05) → batch nets into independent groups
  3. For each batch:
     Corolla (01) OR GAMER (02) → GPU SSSP per net in batch
  4. X-Check (03) [this plan] → GPU DRC, violations → history
  5. History update (GPU kernel)
  6. Convergence check (CPU)

After routing:
  Cypress (04) → congestion feedback → placement SA
```

**This plan's role**: X-Check runs after all nets in the current iteration are routed by Corolla/GAMER (step 3). It receives the `segment_buffer` and `violation_buffer` from the shared `GpuRoutingEngine`, finds clearance/short violations, and writes DRC violation costs back into the `history_costs` buffer. These updated history costs are then used by Corolla/GAMER in the next PathFinder iteration.

### Shared `GpuRoutingEngine`

Uses shared `GpuRoutingEngine` from `gpu/engine.rs` (see Plan 01 for full definition). X-Check-specific fields/pipelines used:

| Field | Purpose |
|-------|---------|
| `device`, `queue` | wgpu primitives |
| `segment_buffer` | Routed segments written by Corolla/GAMER; read by sweepline DRC |
| `violation_buffer` | DRC violation output; written by sweepline, read back to CPU |
| `history_costs` | Written by `drc_history_update` kernel after violations are found |
| `clearance_matrix` | Per-net-class clearance rules for violation filtering |
| `drc_sweepline_pipeline` | Parallel sweepline clearance check |
| `drc_short_pipeline` | Short-circuit detection |

### Shared Buffer Access

| Buffer | Access | Notes |
|--------|--------|-------|
| `segment_buffer` | Read | Filled by Corolla (01) / GAMER (02) after routing each batch |
| `violation_buffer` | Write | DRC violations written here; compacted for CPU readback |
| `history_costs` | Read/Write | Read for current penalties; incremented at violation locations |
| `clearance_matrix` | Read | Symmetric per-net-class matrix, uploaded once per routing run |

### Module Structure

All files live under the shared GPU module (same as Plans 01, 02, 04, 05):

```
crates/autopcb-router/src/gpu/
├── mod.rs              // GpuRoutingEngine (shared device, queue, buffers, pipelines)
├── engine.rs           // GpuRoutingEngine struct, initialization, buffer management
├── buffers.rs          // Buffer types, layout, upload/download helpers
├── bellman_ford.rs     // Corolla BF dispatch (01)
├── sweep.rs            // GAMER H/V sweep dispatch (02)
├── drc.rs              // X-Check GPU DRC (03) [this file]
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

## 1. X-Check Algorithm Adapted for PCB DRC

### 1.1 Sequential Sweepline Recap

The classic distance-check sweepline (Algorithm 1 in X-Check) sweeps a vertical line
across horizontal segments sorted by x-coordinate. For each left endpoint, it inserts the
segment into a BST keyed by y-coordinate and queries for segments within clearance delta.
For each right endpoint, it removes the segment. This is O(n log n + k) where k is the
number of violations.

### 1.2 Parallel Sweepline via Prefix Computation

X-Check's core contribution (Section 3.3, Equation 4) is showing that the sweepline can
be decomposed into a parallel prefix computation:

```
PSW = {P, <, T, t_0, h, rho, f}
```

Where:
- `P` = event points (segment endpoints)
- `<` = total order on coordinates
- `T` = prefix structures (active segment sets)
- `t_0` = empty set
- `h` = update function (insert/delete segment)
- `rho` = fold function (batch-process a block of events)
- `f` = combine function (merge two prefix structures, associative)

The three-step parallel algorithm:
1. **Batching**: Sort events, split into `b` blocks. Each GPU thread computes a partial
   prefix structure for its block via `rho`.
2. **Sweeping**: A single thread (or small group) sweeps the `b` partial prefix structures
   using `f` to produce complete prefix structures at block boundaries.
3. **Refining**: Each block refines its internal prefix structures in parallel, using the
   block-boundary prefix as a starting point.

### 1.3 Vertical vs Horizontal Sweeping

X-Check proposes two sweeping strategies. For segments sorted by y-coordinate (vertical
sweep), the prefix structure is the set of segments within clearance delta below the
current segment. This yields:
- Work: O(n * polylog(n))
- Depth: O(sqrt(n) * polylog(n))

The vertical sweep is preferred for GPU because:
- Better theoretical depth (polynomial improvement over horizontal sweep)
- Simpler implementation (prefix structures use 1D binary search, not set operations)
- Each segment has exactly one y-coordinate, forming a clean total order

### 1.4 Adapting from VLSI (Rectilinear) to PCB (45-Degree Traces)

VLSI DRC assumes Manhattan geometry -- all edges are axis-aligned. PCB routing supports
45-degree traces (and the PDRC paper addresses arbitrary angles). Adaptations needed:

**Segment decomposition for 45-degree traces**: A 45-degree trace segment from (x1,y1)
to (x2,y2) is decomposed into its axis-aligned bounding box for the sweepline, then the
actual distance computation uses the true segment geometry. This matches PDRC's approach
(Section 4.2: decompose non-Manhattan segments into convex parts).

**Distance computation between arbitrary segments**: Replace the simple y-distance check
(`|y1 - y2| < delta`) with a general segment-to-segment distance computation:

```
fn segment_distance(s1: Segment, s2: Segment) -> f64 {
    // Compute minimum distance between two line segments
    // This is a standard computational geometry operation:
    // 1. Check if segments intersect (distance = 0)
    // 2. Compute point-to-segment distances for all 4 endpoints
    // 3. Return minimum
}
```

On GPU, this is ~20 ALU ops per pair -- negligible compared to memory latency.

**Practical simplification**: For routing-time DRC, we can use the bounding-box inflation
approach (Section 5 below) instead of exact segment-to-segment distance. Exact distance
checking is reserved for final validation DRC.

### 1.5 Handling Per-Net-Class Clearance Rules

X-Check assumes a single clearance value `delta`. PCB boards have per-net-class clearance
matrices (e.g., "power nets need 0.3mm clearance from signal nets, signal nets need 0.2mm
from each other").

**Approach**: Use the maximum clearance from the rule matrix as `delta` for the sweepline
query range. This produces a superset of candidate pairs. Then filter each candidate pair
against the actual clearance rule for that net-class combination:

```wgsl
// In violation check kernel
let actual_clearance = clearance_matrix[net_class_a * num_classes + net_class_b];
let distance = segment_distance(seg_a, seg_b);
if (distance < actual_clearance) {
    report_violation(seg_a, seg_b, distance, actual_clearance);
}
```

The clearance matrix is small (max 32 net classes = 1KB) and fits in uniform buffer.

---

## 2. GPU DRC Architecture

### 2.1 Position in PathFinder Loop

```
PathFinder iteration:
  1. Rip-up (all nets or hot-set)
  2. Order nets
  3. Route each net (GPU Bellman-Ford)        <-- segments produced here
  4. Update occupancy / detect shorts          <-- existing GPU kernel
  5. *** GPU DRC pass ***                      <-- NEW: clearance + short check
  6. Update history costs from DRC violations  <-- violations feed back here
  7. Check convergence
```

DRC runs after all nets are routed in an iteration, not after each individual net. This
allows batch processing of all segments together, which is far more efficient on GPU than
incremental checking.

**When to skip DRC**: In early PathFinder iterations (iterations 1-3), many conflicts are
expected and the routing is unstable. Running full DRC wastes GPU cycles. Options:
- **Skip entirely** for iterations 1-3, rely on occupancy-based conflict detection only
- **Run lightweight DRC** (shorts only, no clearance) for iterations 1-5
- **Run full DRC** from iteration 4 onward (or when conflict count drops below threshold)

The convergence metric transitions from "occupancy conflicts" (cheap, sufficient for early
iterations) to "DRC violations" (precise, needed for final convergence).

### 2.2 Data Flow

```
Input (already on GPU from routing):
  ├── routed_segments[]     // {net_id, layer, x1, y1, x2, y2, width}
  ├── routed_vias[]         // {net_id, x, y, from_layer, to_layer, diameter}
  ├── fixed_obstacles[]     // pads, keepouts, board edge (uploaded once)
  └── clearance_matrix[]    // net_class x net_class -> clearance (uniform buffer)

GPU Pipeline:
  ├── Per-layer segment extraction
  ├── Sort segments by y-coordinate (GPU radix sort)
  ├── Parallel sweepline (batching -> sweeping -> refining)
  ├── Candidate pair filtering (actual clearance check)
  ├── Short-circuit detection (overlapping same-resource segments)
  └── Violation compaction (parallel stream compaction)

Output (GPU -> CPU readback):
  ├── violation_count: u32
  ├── violations[]: {seg_a_idx, seg_b_idx, net_a, net_b, distance, required, location}
  └── violation_heatmap[]: per-grid-cell violation density (u8)
```

### 2.3 Violation Representation

```rust
/// A DRC violation detected by the GPU pipeline.
struct GpuDrcViolation {
    /// Index of first segment/via in the GPU buffer.
    object_a: u32,
    /// Index of second segment/via in the GPU buffer.
    object_b: u32,
    /// Net IDs of the two objects.
    net_a: u32,  // NetId.raw()
    net_b: u32,  // NetId.raw()
    /// Violation type.
    kind: DrcViolationKind,
    /// Actual distance between objects (mm, fixed-point).
    actual_distance_fp: u32,
    /// Required clearance (mm, fixed-point).
    required_clearance_fp: u32,
    /// Location of violation center (grid coordinates).
    location_x: u32,
    location_y: u32,
    layer: u32,
}

enum DrcViolationKind {
    ClearanceViolation = 0,
    ShortCircuit = 1,
    WidthViolation = 2,
}
```

### 2.4 How Violations Feed Back into PathFinder

DRC violations update two PathFinder data structures:

1. **History cost increment**: For each violation at grid cell (x, y, layer), increment
   `history[cell_index(x, y, layer)]` by `DRC_VIOLATION_HISTORY_INCREMENT`. This
   discourages future nets from routing through violation locations.

2. **Per-net violation count**: Track how many violations each net participates in. Nets
   with high violation counts are prioritized for rip-up in the next iteration (added to
   the hot-set).

```wgsl
@compute @workgroup_size(64)
fn update_history_from_violations(@builtin(global_invocation_id) gid: vec3<u32>) {
    let viol_idx = gid.x;
    if (viol_idx >= params.violation_count) { return; }

    let v = violations[viol_idx];
    let cell = cell_index(v.location_x, v.location_y, v.layer);
    atomicAdd(&history[cell], DRC_VIOLATION_HISTORY_INCREMENT);

    // Mark both nets as violating
    atomicAdd(&net_violation_counts[v.net_a], 1u);
    atomicAdd(&net_violation_counts[v.net_b], 1u);
}
```

**Convergence metric**: `drc_violation_count == 0` is the true convergence condition. The
occupancy-based conflict count is a necessary but not sufficient proxy.

---

## 3. WGSL Shader Pipeline

### 3.1 `segment_extract.wgsl` -- Per-Layer Segment Extraction

Before sweepline, extract segments belonging to each layer into contiguous arrays. This
enables per-layer sweepline passes (intra-layer clearance) and avoids cross-layer segment
comparisons.

```wgsl
struct Segment {
    net_id: u32,
    net_class: u32,
    x1: i32,  // fixed-point grid coordinates
    y1: i32,
    x2: i32,
    y2: i32,
    half_width: u32,  // half trace width in grid units
}

@group(0) @binding(0) var<storage, read> all_segments: array<Segment>;
@group(0) @binding(1) var<storage, read> segment_layers: array<u32>;
@group(0) @binding(2) var<storage, read_write> layer_counts: array<atomic<u32>>;
@group(0) @binding(3) var<storage, read_write> layer_offsets: array<u32>;
@group(0) @binding(4) var<storage, read_write> sorted_segments: array<Segment>;

// Pass 1: count segments per layer
@compute @workgroup_size(256)
fn count_per_layer(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= params.num_segments) { return; }
    let layer = segment_layers[idx];
    atomicAdd(&layer_counts[layer], 1u);
}

// Pass 2: scatter segments into per-layer buckets (after prefix sum on layer_counts)
@compute @workgroup_size(256)
fn scatter_to_layers(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= params.num_segments) { return; }
    let layer = segment_layers[idx];
    let write_pos = layer_offsets[layer] + atomicAdd(&layer_write_heads[layer], 1u);
    sorted_segments[write_pos] = all_segments[idx];
}
```

### 3.2 `segment_sort.wgsl` -- Sort Segments by Y-Coordinate

For the vertical sweeping algorithm, segments must be sorted by y-coordinate. We use the
GPU radix sort approach from X-Check's Copy-Sort-Permute (CSP) strategy (Section 5.2):

1. Extract y-coordinate keys from segments
2. GPU radix sort on keys (with associated index permutation)
3. Permute segments according to sorted indices

```wgsl
// Extract sort keys (y-coordinate of segment midpoint)
@compute @workgroup_size(256)
fn extract_sort_keys(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= params.num_layer_segments) { return; }
    let seg = layer_segments[idx];
    // Use midpoint y for sorting; min(y1,y2) also works
    sort_keys[idx] = u32(min(seg.y1, seg.y2) + 0x7FFFFFFF);  // offset to unsigned
    sort_indices[idx] = idx;
}

// Radix sort is implemented as a sequence of prefix-sum + scatter passes
// (4 passes for 32-bit keys, each pass handles 8 bits)
// Standard GPU radix sort -- use wgpu-sort crate or implement per Merrill 2010
```

**Sort strategy selection** (following X-Check Section 5.2): Use CSP (radix sort on
extracted keys + permute) for arrays > 8000 elements. For smaller arrays, use a simple
bitonic sort or fall back to CPU sort + upload.

### 3.3 `sweepline_check.wgsl` -- Parallel Clearance Checking

The core algorithm. Implements the vertical sweeping algorithm (X-Check Section 4.1):

```wgsl
struct PrefixRange {
    start: u32,  // index into sorted segment array
    end: u32,    // exclusive end
}

// Step 1: Batching -- compute partial prefix structures per block
@compute @workgroup_size(256)
fn sweepline_batch(@builtin(global_invocation_id) gid: vec3<u32>) {
    let block_idx = gid.x;
    if (block_idx >= params.num_blocks) { return; }

    let block_start = block_idx * params.block_size;
    let block_end = min(block_start + params.block_size, params.num_segments);

    // For the highest segment in this block, binary search for the lowest
    // segment within max_clearance_delta
    let highest_y = sorted_segments[block_end - 1u].y_key();
    let lowest_valid_y = highest_y - params.max_clearance_cells;

    // Binary search in sorted array for lowest_valid_y
    var lo = 0u;
    var hi = block_start;
    while (lo < hi) {
        let mid = (lo + hi) / 2u;
        if (sorted_segments[mid].y_key() < lowest_valid_y) {
            lo = mid + 1u;
        } else {
            hi = mid;
        }
    }

    // Store partial prefix: range of segments that could interact with this block
    block_prefixes[block_idx] = PrefixRange { start: lo, end: block_end };
}

// Step 2: Sequential sweep of block prefixes (small -- runs on GPU with 1 thread)
@compute @workgroup_size(1)
fn sweepline_sweep() {
    for (var b = 1u; b < params.num_blocks; b++) {
        // Combine prefix from previous block boundary with current block
        let prev_end = block_prefixes[b - 1u].end;
        let prev_prefix_start = block_prefixes[b - 1u].start;
        // The refined prefix for block b starts from the lowest segment
        // still within delta of block b's first segment
        let first_y = sorted_segments[b * params.block_size].y_key();
        let lowest_valid_y = first_y - params.max_clearance_cells;

        // Update start to account for segments that have scrolled out of range
        var new_start = prev_prefix_start;
        while (new_start < prev_end && sorted_segments[new_start].y_key() < lowest_valid_y) {
            new_start++;
        }
        block_prefixes[b].start = new_start;
    }
}

// Step 3: Refining + violation checking (the hot kernel -- maximum parallelism)
@compute @workgroup_size(256)
fn sweepline_check(@builtin(global_invocation_id) gid: vec3<u32>) {
    let seg_idx = gid.x;
    if (seg_idx >= params.num_segments) { return; }

    let seg = sorted_segments[seg_idx];
    let seg_y = seg.y_key();
    let block_idx = seg_idx / params.block_size;

    // Iterate over candidate segments from prefix structure
    let prefix = block_prefixes[block_idx];
    for (var i = prefix.start; i < seg_idx; i++) {
        let candidate = sorted_segments[i];

        // Skip if y-distance exceeds max clearance
        let dy = seg_y - candidate.y_key();
        if (dy > params.max_clearance_cells) { continue; }
        if (dy < 0) { break; }  // sorted, so no more candidates

        // Skip same-net segments (no self-check)
        if (seg.net_id == candidate.net_id) { continue; }

        // Check x-overlap (horizontal projection must be nonempty)
        let x_overlap = has_x_overlap(seg, candidate);
        if (!x_overlap) { continue; }

        // Compute actual distance between segments (including width)
        let distance = segment_pair_distance(seg, candidate);
        let required = clearance_matrix[seg.net_class * params.num_classes + candidate.net_class];
        let total_required = required + seg.half_width + candidate.half_width;

        if (distance < total_required) {
            // Report violation
            let write_idx = atomicAdd(&violation_count, 1u);
            if (write_idx < MAX_VIOLATIONS) {
                violations[write_idx] = DrcViolation {
                    object_a: seg_idx,
                    object_b: i,
                    net_a: seg.net_id,
                    net_b: candidate.net_id,
                    kind: select(CLEARANCE_VIOLATION, SHORT_CIRCUIT, distance == 0u),
                    actual_distance_fp: distance,
                    required_clearance_fp: total_required,
                    location_x: (seg.x1 + candidate.x1) / 2,
                    location_y: (seg_y + candidate.y_key()) / 2,
                    layer: params.current_layer,
                };
            }
        }
    }
}
```

**Kernel granularity** (following X-Check Section 5.3): Each GPU thread handles one
segment and iterates over its prefix structure. This is option (3) from X-Check -- not the
finest granularity (option 4) but simpler to implement. For PCB-scale data (thousands of
segments, not millions), this provides sufficient parallelism without the complexity of
global offset calculation.

### 3.4 `short_check.wgsl` -- Short-Circuit Detection

Short circuits are a special case: two segments from different nets that overlap (distance
= 0). The sweepline already catches these as clearance violations with distance < delta.
However, a dedicated short-detection pass is useful for early iterations where we skip full
clearance checking:

```wgsl
@compute @workgroup_size(256)
fn detect_shorts(@builtin(global_invocation_id) gid: vec3<u32>) {
    let cell = gid.x;
    if (cell >= params.total_cells) { return; }

    let occupancy = occupancy_map[cell];
    let capacity = capacity_map[cell];

    if (occupancy > capacity) {
        // Multiple nets share this routing resource
        let write_idx = atomicAdd(&short_count, 1u);
        if (write_idx < MAX_SHORTS) {
            short_locations[write_idx] = cell;
        }
    }
}
```

This kernel is already described in `docs/notes/autorouter-gpu/03-gpu-cost-functions.md`
(Section 3.4). It runs on the occupancy grid (already maintained by the PathFinder loop)
and completes in < 1ms for typical PCB grids.

### 3.5 `violation_compact.wgsl` -- Compact Violations into Output Buffer

After the sweepline check, violations are scattered into the output buffer with gaps (due
to atomicAdd contention and MAX_VIOLATIONS cap). Compact them into a dense array for CPU
readback:

```wgsl
// Standard parallel stream compaction:
// 1. Prefix sum on violation_valid flags
// 2. Scatter valid violations to compacted positions

@compute @workgroup_size(256)
fn compact_violations(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= params.raw_violation_count) { return; }

    // If this slot has a valid violation, write to compacted position
    if (violations[idx].kind != INVALID) {
        let compact_idx = prefix_sum[idx];
        compacted_violations[compact_idx] = violations[idx];
    }
}
```

For routing-time DRC, we may skip compaction entirely and just read `violation_count`
(a single u32) plus the first N violations for hot-set identification. Full compaction
is only needed for final validation DRC where we report all violations.

### 3.6 Variable Clearance Handling in Shaders

The clearance matrix is uploaded as a uniform buffer (small, fits in constant cache):

```wgsl
// In bind group 0
@group(0) @binding(5) var<uniform> clearance_matrix: array<u32, 1024>;
// 32 x 32 net classes = 1024 entries, each a fixed-point clearance value

fn lookup_clearance(class_a: u32, class_b: u32) -> u32 {
    return clearance_matrix[class_a * params.num_net_classes + class_b];
}
```

The sweepline uses `max_clearance_cells` (the maximum entry in the matrix) as the sweep
range. This over-reports candidates, but the per-pair check in the refine step filters
false positives using the actual clearance for that net-class pair. Since PCB boards
typically have 2-5 net classes, the over-reporting ratio is modest (at most 2-3x).

---

## 4. Obstacle Inflation Alternative

### 4.1 Baking Clearance into Obstacle Maps

Instead of post-route sweepline DRC, we can bake clearance requirements into the obstacle
maps used during routing. This is the Minkowski sum approach: inflate each obstacle
(pad, keepout, board edge, existing trace) by the clearance distance, then treat any
overlap with a routed trace as a violation.

**GPU implementation** (from `03-gpu-cost-functions.md` Section 3.2):

```wgsl
@compute @workgroup_size(64)
fn inflate_obstacles(@builtin(global_invocation_id) gid: vec3<u32>) {
    let cell = gid.x;
    if (cell >= params.total_cells) { return; }
    if (is_obstacle(cell)) { return; }

    let x = cell_x(cell);
    let y = cell_y(cell);
    let radius = params.clearance_cells;

    for (var dx = -i32(radius); dx <= i32(radius); dx++) {
        for (var dy = -i32(radius); dy <= i32(radius); dy++) {
            if (u32(abs(dx) + abs(dy)) > radius) { continue; }
            let nx = i32(x) + dx;
            let ny = i32(y) + dy;
            if (out_of_bounds(nx, ny)) { continue; }
            if (is_raw_obstacle(u32(nx), u32(ny))) {
                set_inflated_obstacle(cell);
                return;
            }
        }
    }
}
```

### 4.2 Pros and Cons

| Aspect | Obstacle Inflation | Explicit DRC (Sweepline) |
|--------|-------------------|--------------------------|
| Speed during routing | Zero-cost: routing avoids inflated cells naturally | O(n sqrt(n)) per iteration |
| Accuracy | Grid-quantized (limited by grid resolution) | Exact segment-to-segment distance |
| Per-net-class clearance | Multiple obstacle maps (2-4 maps, ~16MB total) | Single pass with clearance matrix lookup |
| Memory | K maps x grid_size bytes | Segment arrays + prefix structures |
| Diagonal traces | Inflation is axis-aligned (overestimates for diagonals) | Correct for arbitrary segment angles |
| Short detection | Not covered (inflation prevents routes, not detects shorts) | Naturally reports overlaps |
| Violation reporting | No report -- just blocked cells | Full violation details (nets, locations, distances) |

### 4.3 Recommended Hybrid Strategy

Use **both** approaches for different purposes:

1. **Obstacle inflation for routing cost** (Milestone 4: workspace/obstacles): Inflate
   fixed obstacles (pads, keepouts, board edges) by the routing net's clearance class.
   This prevents the router from placing traces too close to fixed objects. Runs once
   at workspace build time. Uses Approach A from `03-gpu-cost-functions.md` Section 3.3:
   one inflated map per distinct clearance value.

2. **Explicit DRC for trace-to-trace checking** (this plan): After each iteration, run
   the sweepline DRC to find clearance violations between routed traces from different
   nets. Inflation alone cannot catch trace-to-trace violations because traces are not
   obstacles to each other during routing -- they are added incrementally as nets are
   routed.

3. **Explicit DRC for final validation**: After routing converges, run a comprehensive
   DRC pass that checks all primitive pairs (trace-trace, trace-pad, trace-via, via-via,
   trace-keepout). This produces the `drc_violation_count` in `RoutingMetrics`.

---

## 5. PcbIr Design Rules for DRC

### 5.1 Relevant `IrDesignRule` / `IrRuleParams` Types

The following rule types from `crates/autopcb-ir/src/rule.rs` are consumed by the DRC
pipeline:

| `IrRuleParams` variant | DRC usage | GPU representation |
|------------------------|-----------|-------------------|
| `Clearance { gap_mm }` | Minimum copper-to-copper clearance | `clearance_matrix[class_a * N + class_b]` |
| `Width { min_mm, max_mm, preferred_mm }` | Minimum/maximum trace width | Per-net min/max width in `net_params[]` buffer |
| `BoardOutlineClearance { gap_mm }` | Clearance from board edge | Inflated board edge obstacle map |
| `HoleToHoleClearance { gap_mm }` | Via-to-via drill clearance | Via pair distance check |
| `MinimumAnnularRing { min_mm }` | Via annular ring size | Via validation (per-via check) |
| `ComponentClearance { gap_mm }` | Component-to-trace clearance | Inflated component courtyard obstacle map |

Rules NOT consumed by routing-time DRC (but needed for final validation):
- `SolderMaskExpansion` / `PasteMaskExpansion` -- manufacturing rules, not routing
- `RoutingTopology` / `RoutingPriority` / `RoutingLayers` / `RoutingCornerStyle` /
  `RoutingViaStyle` -- routing strategy rules, not DRC
- `DiffPairsRouting` / `MatchedLengths` -- checked separately in high-speed validation

### 5.2 GPU-Friendly Rule Lookup Table

Build a clearance matrix from `IrDesignRule` entries at workspace construction time.
The matrix is indexed by net class ID pairs:

```rust
/// Built from IrDesignRule entries with kind == RuleKind::Clearance.
/// Indexed by (net_class_a, net_class_b) -> clearance in grid cells.
struct ClearanceMatrix {
    /// Flattened NxN matrix (N = number of distinct net classes).
    entries: Vec<u32>,  // fixed-point: clearance_mm * COST_SCALE
    num_classes: u32,
}

impl ClearanceMatrix {
    fn build(rules: &[IrDesignRule], nets: &IdMap<NetId, IrNet>, grid_resolution: f64) -> Self {
        // 1. Collect distinct net class names -> assign class IDs (0..N)
        // 2. For each Clearance rule, determine which net-class pairs it applies to
        //    (from rule scope / filter expressions -- IrDesignRule.name pattern matching)
        // 3. Apply rule priority: lower priority number = higher priority, first match wins
        // 4. Convert clearance_mm to grid cells: ceil(gap_mm / grid_resolution)
        // 5. Ensure matrix is symmetric: clearance(A,B) == clearance(B,A)
        // 6. Default clearance for unspecified pairs: use the global/default clearance rule
    }

    fn upload_to_gpu(&self, device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("clearance_matrix"),
            contents: bytemuck::cast_slice(&self.entries),
            usage: wgpu::BufferUsages::UNIFORM,
        })
    }
}
```

**Net class assignment**: Each `IrNet` has `net_class: Option<String>`. Nets without an
explicit class are assigned to a default class (class ID 0). The class-to-ID mapping is
built during workspace construction:

```rust
fn assign_net_classes(nets: &IdMap<NetId, IrNet>) -> (HashMap<String, u32>, Vec<u32>) {
    let mut class_map: HashMap<String, u32> = HashMap::new();
    class_map.insert("default".into(), 0);
    let mut next_id = 1u32;

    let net_class_ids: Vec<u32> = nets.values().map(|net| {
        match &net.net_class {
            Some(name) => *class_map.entry(name.clone()).or_insert_with(|| {
                let id = next_id;
                next_id += 1;
                id
            }),
            None => 0,  // default class
        }
    }).collect();

    (class_map, net_class_ids)
}
```

### 5.3 Per-Net-Class Clearance Matrix

Example for a typical PCB with 3 net classes:

```
Net classes:
  0 = "default"   (signal nets)
  1 = "power"     (VCC, GND)
  2 = "high_speed" (USB, DDR)

Clearance matrix (mm):
         default  power  high_speed
default   0.20    0.30    0.25
power     0.30    0.40    0.35
high_speed 0.25   0.35    0.20

GPU representation (fixed-point, scale=1024):
  [205, 307, 256, 307, 410, 358, 256, 358, 205]
```

This 3x3 matrix (9 entries, 36 bytes) trivially fits in uniform buffer. Even with 32 net
classes (1024 entries, 4KB), it fits comfortably within wgpu's uniform buffer limits
(64KB minimum guaranteed).

---

## 6. Integration with PathFinder

### 6.1 DRC Violations -> History Cost Updates

After the DRC pass, violations are read back to CPU (just `violation_count` + first N
violation locations) and fed into two update mechanisms:

**GPU-side update** (preferred, avoids readback latency):

```wgsl
@compute @workgroup_size(256)
fn drc_history_update(@builtin(global_invocation_id) gid: vec3<u32>) {
    let viol_idx = gid.x;
    if (viol_idx >= violation_count) { return; }

    let v = violations[viol_idx];

    // Increment history at violation location
    let cell = cell_index(v.location_x, v.location_y, v.layer);
    atomicAdd(&history[cell], params.drc_history_increment);

    // Also increment in a neighborhood around the violation
    // (spreads the penalty to discourage nearby routing too)
    for (var dx = -1; dx <= 1; dx++) {
        for (var dy = -1; dy <= 1; dy++) {
            if (dx == 0 && dy == 0) { continue; }
            let nx = i32(v.location_x) + dx;
            let ny = i32(v.location_y) + dy;
            if (!out_of_bounds(nx, ny)) {
                let neighbor_cell = cell_index(u32(nx), u32(ny), v.layer);
                atomicAdd(&history[neighbor_cell], params.drc_history_increment / 2u);
            }
        }
    }
}
```

**CPU-side hot-set update**: Read back `net_violation_counts[]` and add nets with
violations > threshold to the hot-set for the next iteration's rip-up.

### 6.2 DRC Violation Count as Convergence Metric

```rust
fn check_convergence(state: &PathFinderState) -> bool {
    // Primary: no DRC violations
    if state.drc_violation_count == 0 && state.occupancy_conflicts == 0 {
        return true;  // fully converged
    }

    // Secondary: max iterations
    if state.iteration >= state.config.max_iterations {
        return true;  // timeout
    }

    // Tertiary: plateau detection on DRC violation count
    if state.drc_plateau_detected(window_size: 8) {
        return true;  // stuck
    }

    false
}
```

The `drc_violation_count` is recorded in `RoutingMetrics` (from `autopcb-routes`) and in
each `RoutingIterationSnapshot` for viewer playback.

### 6.3 When to Skip DRC

| Iteration | DRC mode | Rationale |
|-----------|----------|-----------|
| 1-2 | None (occupancy only) | Routing is completely unstable, DRC would report thousands of violations |
| 3-5 | Short-circuit only | Quick GPU kernel, catches gross routing errors without sweepline cost |
| 6+ | Full DRC (clearance + shorts) | Routing is stabilizing, DRC violations drive fine-grained convergence |
| Last iteration | Comprehensive DRC | Final validation including width, annular ring, board edge clearance |

The transition thresholds are configurable in `RoutingConfig`:

```rust
struct DrcConfig {
    /// First iteration to run short-circuit detection (default: 3).
    short_check_start_iteration: u32,
    /// First iteration to run full clearance DRC (default: 6).
    full_drc_start_iteration: u32,
    /// History cost increment per DRC violation (default: 5.0).
    drc_history_increment: f64,
    /// Neighborhood radius for history spreading around violations (default: 1).
    drc_history_spread_radius: u32,
}
```

---

## 7. Performance Analysis

### 7.1 Expected Speedup vs CPU DRC

**PCB board characteristics** (contrast with VLSI):

| Metric | Typical PCB | Typical VLSI (X-Check) |
|--------|-------------|------------------------|
| Segments per layer | 1,000 - 10,000 | 100,000 - 50,000,000 |
| Total segments | 5,000 - 50,000 | 500,000 - 100,000,000 |
| Net classes | 2-5 | 1-2 |
| Layers | 2-8 (copper) | 5-15 (metal) |
| Clearance rules | 5-20 | 50-500 |
| Segment angles | 0, 45, 90, 135 degrees | 0, 90 degrees only |

PCB boards have far fewer segments than VLSI designs. X-Check shows 45-60x speedup at
scale (millions of segments), but PCB-scale data may not saturate GPU parallelism.

**Expected performance by board size**:

| Board complexity | Segments | CPU DRC time | GPU DRC time | Speedup |
|-----------------|----------|-------------|-------------|---------|
| Simple (2L, 100 nets) | ~2,000 | < 1ms | ~2ms (overhead-dominated) | < 1x |
| Medium (4L, 500 nets) | ~10,000 | ~5ms | ~2ms | ~2.5x |
| Complex (6L, 2000 nets) | ~50,000 | ~50ms | ~5ms | ~10x |
| Dense (8L, 5000 nets) | ~200,000 | ~500ms | ~10ms | ~50x |

For simple boards, GPU DRC is slower than CPU due to kernel launch and data transfer
overhead. The crossover point is around 5,000-10,000 segments. Below this, use CPU DRC.

**Dynamic algorithm selection** (following X-Check Section 5.1): If `total_segments <
DRC_GPU_THRESHOLD` (default 5000), run DRC on CPU using the rstar R-tree spatial index
already built for the workspace.

### 7.2 Memory Requirements

| Data structure | Size formula | Typical (4L, 500 nets, 10K segments) |
|---------------|-------------|--------------------------------------|
| Segment buffer | 28 bytes/segment | 280 KB |
| Sort key buffer | 8 bytes/segment | 80 KB |
| Prefix structures | 8 bytes/block (b = sqrt(n)) | 800 bytes |
| Clearance matrix | 4 bytes * N^2 | 100 bytes (5 classes) |
| Violation buffer | 40 bytes * MAX_VIOLATIONS | 40 KB (1000 max) |
| Violation heatmap | 1 byte/cell | 4 MB (2000x2000 grid) |
| **Total GPU memory** | | **~5 MB** |

This is negligible compared to the routing grid itself (which is 10-100 MB for a typical
board).

### 7.3 Sorting Overhead

Sorting is the dominant cost for small segment counts. Options:

1. **GPU radix sort** (via `wgpu-sort` or custom implementation): O(n) work, but kernel
   launch overhead dominates for n < 10,000. Best for n > 50,000.

2. **CPU sort + upload**: `segments.sort_by_key(|s| s.y)` on CPU, then upload sorted
   buffer to GPU. For n = 10,000, this is ~100us (sort) + ~50us (upload) = 150us total.
   Simpler, no GPU sort infrastructure needed.

3. **Pre-sorted maintenance**: If segments are added incrementally (net-by-net), maintain
   a sorted buffer using insertion. This amortizes sorting cost across the routing loop.

**Recommendation**: CPU sort + upload for initial implementation. GPU sort only if
profiling shows sorting as a bottleneck on large boards.

---

## 8. Testing Strategy

### 8.1 Synthetic Board Tests (Unit Tests)

```rust
#[cfg(test)]
mod tests {
    /// Two parallel traces with known clearance -- no violation.
    #[test]
    fn parallel_traces_clearance_ok() {
        let segments = vec![
            Segment::new(net_a, 0.0, 0.0, 10.0, 0.0, 0.2),  // horizontal, width 0.2mm
            Segment::new(net_b, 0.0, 0.5, 10.0, 0.5, 0.2),  // parallel, 0.3mm gap
        ];
        let rules = ClearanceMatrix::uniform(0.25);  // 0.25mm clearance
        let violations = run_drc(&segments, &rules);
        assert_eq!(violations.len(), 0);  // gap = 0.3mm > 0.25mm required
    }

    /// Two parallel traces too close -- clearance violation reported.
    #[test]
    fn parallel_traces_clearance_violation() {
        let segments = vec![
            Segment::new(net_a, 0.0, 0.0, 10.0, 0.0, 0.2),
            Segment::new(net_b, 0.0, 0.3, 10.0, 0.3, 0.2),  // 0.1mm gap
        ];
        let rules = ClearanceMatrix::uniform(0.25);
        let violations = run_drc(&segments, &rules);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].kind, DrcViolationKind::ClearanceViolation);
    }

    /// Two overlapping traces from different nets -- short circuit.
    #[test]
    fn overlapping_traces_short() {
        let segments = vec![
            Segment::new(net_a, 0.0, 0.0, 10.0, 0.0, 0.2),
            Segment::new(net_b, 5.0, 0.0, 15.0, 0.0, 0.2),  // overlaps at x=5-10
        ];
        let rules = ClearanceMatrix::uniform(0.25);
        let violations = run_drc(&segments, &rules);
        assert!(violations.iter().any(|v| v.kind == DrcViolationKind::ShortCircuit));
    }

    /// Same-net segments should not trigger violations.
    #[test]
    fn same_net_no_violation() {
        let segments = vec![
            Segment::new(net_a, 0.0, 0.0, 10.0, 0.0, 0.2),
            Segment::new(net_a, 5.0, 0.1, 15.0, 0.1, 0.2),  // same net, very close
        ];
        let rules = ClearanceMatrix::uniform(0.25);
        let violations = run_drc(&segments, &rules);
        assert_eq!(violations.len(), 0);
    }

    /// Per-net-class clearance: power nets need more clearance.
    #[test]
    fn per_class_clearance() {
        // class 0 = signal (0.2mm), class 1 = power (0.3mm between power-signal)
        let matrix = ClearanceMatrix::from_entries(2, &[0.20, 0.30, 0.30, 0.40]);
        let segments = vec![
            Segment::new_with_class(net_a, 0, 0.0, 0.0, 10.0, 0.0, 0.2),  // signal
            Segment::new_with_class(net_b, 1, 0.0, 0.35, 10.0, 0.35, 0.2), // power
            // Gap = 0.15mm, required for signal-power = 0.30mm -> violation
        ];
        let violations = run_drc(&segments, &matrix);
        assert_eq!(violations.len(), 1);
    }

    /// 45-degree trace clearance check.
    #[test]
    fn diagonal_trace_clearance() {
        let segments = vec![
            Segment::new(net_a, 0.0, 0.0, 10.0, 10.0, 0.2),  // 45-degree
            Segment::new(net_b, 0.0, 0.2, 10.0, 10.2, 0.2),  // parallel 45-degree, 0.14mm gap
        ];
        let rules = ClearanceMatrix::uniform(0.20);
        let violations = run_drc(&segments, &rules);
        // Perpendicular distance between parallel 45-degree lines with dy=0.2
        // = 0.2 * cos(45) = 0.141mm, which is < 0.20mm required
        assert_eq!(violations.len(), 1);
    }
}
```

### 8.2 CPU/GPU Result Comparison (Property Tests)

```rust
#[cfg(feature = "proptest")]
proptest! {
    /// GPU DRC and CPU DRC produce identical violation counts.
    #[test]
    fn gpu_cpu_drc_agreement(
        segments in prop::collection::vec(arb_segment(), 10..500),
        clearance in 0.1f64..1.0f64,
    ) {
        let rules = ClearanceMatrix::uniform(clearance);
        let cpu_violations = cpu_drc(&segments, &rules);
        let gpu_violations = gpu_drc(&segments, &rules);

        // Same violation count
        prop_assert_eq!(cpu_violations.len(), gpu_violations.len());

        // Same violation pairs (order may differ)
        let cpu_pairs: BTreeSet<_> = cpu_violations.iter()
            .map(|v| (v.net_a.min(v.net_b), v.net_a.max(v.net_b)))
            .collect();
        let gpu_pairs: BTreeSet<_> = gpu_violations.iter()
            .map(|v| (v.net_a.min(v.net_b), v.net_a.max(v.net_b)))
            .collect();
        prop_assert_eq!(cpu_pairs, gpu_pairs);
    }
}
```

### 8.3 Integration Tests

- Route a synthetic board with known clearance violations, verify DRC finds them
- Route a violation-free board, verify DRC reports zero violations
- Verify that DRC violations cause PathFinder history cost increases at violation locations
- Verify convergence: DRC violation count decreases across iterations
- Benchmark: measure DRC time as percentage of total PathFinder iteration time

---

## 9. Implementation Milestones

This GPU DRC plan integrates with the existing router milestone structure from
`docs/plans/router/README.md`. It spans three milestones:

### Phase 1: CPU DRC Baseline (part of Milestone 7: PathFinder)

**Files**:
- `crates/autopcb-router/src/drc.rs`

**Scope**:
- CPU-only DRC using rstar R-tree spatial queries
- Clearance checking between routed segments (trace-trace, trace-pad, trace-via)
- Short-circuit detection via occupancy map
- `ClearanceMatrix` construction from `IrDesignRule` entries
- Integration into PathFinder loop (after all nets routed per iteration)
- Violations feed back into history cost array

**Acceptance criteria**:
- DRC correctly identifies clearance violations on synthetic boards
- PathFinder converges using DRC violation count as metric
- CPU DRC time < 100ms for boards with < 10,000 segments

### Phase 2: GPU DRC Pipeline (new milestone, after Milestone 7)

**Files**:
- `crates/autopcb-router/src/gpu/drc.rs`
- `crates/autopcb-router/src/gpu/shaders/segment_extract.wgsl`
- `crates/autopcb-router/src/gpu/shaders/segment_sort.wgsl`
- `crates/autopcb-router/src/gpu/shaders/sweepline_check.wgsl`
- `crates/autopcb-router/src/gpu/shaders/short_check.wgsl`
- `crates/autopcb-router/src/gpu/shaders/violation_compact.wgsl`
- `crates/autopcb-router/src/gpu/shaders/drc_history_update.wgsl`

**Scope**:
- WGSL shader pipeline implementing parallel sweepline DRC
- GPU radix sort for segments (or CPU sort + upload)
- Dynamic algorithm selection (CPU vs GPU based on segment count)
- GPU-side history cost updates from violations
- Violation readback for hot-set identification

**Acceptance criteria**:
- GPU DRC produces identical results to CPU DRC on all test boards
- GPU DRC is faster than CPU for boards with > 5,000 segments
- Total DRC overhead < 20% of PathFinder iteration time

### Phase 3: Advanced DRC Features (part of Milestone 8: Optimization)

**Files**:
- `crates/autopcb-router/src/drc.rs` (extend)
- `crates/autopcb-router/src/gpu/drc.rs` (extend)

**Scope**:
- Width violation checking (min/max trace width)
- Via-to-via drill clearance
- Board outline clearance (against inflated board edge)
- Comprehensive final validation DRC (all rule types)
- DRC violation reporting in `RouteSolution` / `RoutingMetrics`

---

## 10. File and Type Cross-References

### Codebase Files

| Path | Relevance |
|------|-----------|
| `crates/autopcb-ir/src/rule.rs` | `IrDesignRule`, `IrRuleParams` -- source of clearance rules |
| `crates/autopcb-ir/src/copper.rs` | `IrTrack`, `IrVia` -- segment geometry types |
| `crates/autopcb-ir/src/net.rs` | `IrNet` -- net class assignment |
| `crates/autopcb-ir/src/handles.rs` | `NetId`, `LayerId` -- typed handles |
| `crates/autopcb-ir/src/board.rs` | `IrBoardGeometry`, `IrKeepoutZone` -- fixed obstacles |
| `crates/autopcb-ir/src/component.rs` | `IrComponentPad` -- pad obstacles |
| `crates/autopcb-ir/src/layer_stack.rs` | `IrLayerStack`, `IrCopperLayer` -- per-layer DRC |

### Research Papers

| Paper | Key Technique Used |
|-------|-------------------|
| X-Check (ICCAD 2022) | Parallel sweepline via prefix computation, vertical sweep algorithm, CSP sort, dynamic algorithm selection, kernel granularity |
| OpenDRC (DAC 2023) | Hierarchical layout, adaptive row-based partition, sequential/parallel mode selection |
| PDRC (DAC 2024) | Non-Manhattan segment handling, hierarchical interval lists, iterative parallel sweepline |
| GPU Minkowski Sum (Li & McMains, CAD 2011) | Obstacle inflation via voxelized Minkowski sum |

### Router Plan Cross-References

| Router plan section | DRC integration point |
|--------------------|----------------------|
| Milestone 3 (Rules Bridge) | `ClearanceMatrix` built from `RoutingPolicy` |
| Milestone 4 (Workspace) | Obstacle inflation for fixed objects |
| Milestone 7 (PathFinder) | DRC in iteration loop, history cost updates, convergence |
| Milestone 8 (Optimization) | Final validation DRC, DRC after trace optimization |
| `03-gpu-cost-functions.md` Section 3 | Obstacle inflation, short detection, per-class clearance |

---

## 11. Open Questions

1. **GPU sort library for wgpu/WGSL**: No mature GPU radix sort exists for wgpu (unlike
   CUDA's thrust). Options: (a) port a radix sort to WGSL, (b) use CPU sort + upload,
   (c) use a wgpu compute library like `wgpu-sort` if one matures. Decision deferred to
   Phase 2 implementation.

2. **Violation buffer sizing**: `MAX_VIOLATIONS` must be chosen at pipeline creation time
   (WGSL storage buffer size). If actual violations exceed this, some are silently dropped.
   For routing-time DRC this is acceptable (we only need the count + a sample). For final
   validation, resize and re-run if overflow is detected.

3. **Inter-layer DRC**: The plan focuses on intra-layer clearance (trace-to-trace on the
   same layer). Inter-layer DRC (e.g., via-to-trace clearance on adjacent layers) requires
   cross-layer segment pairs in the sweepline. This can be handled by treating via
   projections on each layer as segments and including them in the per-layer sweep.

4. **Polygon pour DRC**: Copper polygon pours (from `IrPolygon` in
   `crates/autopcb-ir/src/polygon.rs`) are complex shapes that require polygon-to-segment
   distance checks. For routing-time DRC, treat polygon outlines as a set of segments.
   For final validation, use the full polygon containment check.

5. **Determinism**: GPU atomic operations have non-deterministic ordering. The violation
   list order may differ between runs. For deterministic `RouteSolution` output, sort
   violations by (layer, y, x, net_a, net_b) on CPU after readback.

---

## See Also

| Plan | Role | Relationship to X-Check DRC |
|------|------|----------------------------|
| **01 — Corolla** (`01-corolla-bellman-ford.md`) | GPU SSSP backend | Defines `GpuRoutingEngine` including `segment_buffer` and `violation_buffer` that X-Check reads and writes. Corolla routed segments are the primary input to the DRC pipeline. |
| **02 — GAMER** (`02-gamer-sweep-routing.md`) | Alternative GPU SSSP backend | Same integration point as Corolla. X-Check is agnostic to which SSSP backend ran — it only reads `segment_buffer`. |
| **04 — Cypress** (`04-cypress-congestion-feedback.md`) | Post-routing congestion feedback | Reads `history_costs` after routing converges. X-Check writes DRC violation penalties into `history_costs` throughout the PathFinder loop, which Cypress then uses to identify bottleneck regions. |
| **05 — InstantGR** (`05-instantgr-net-batching.md`) | Net batching, runs before SSSP | Operates before X-Check in the iteration sequence. Batch granularity (not per-net) determines when X-Check's DRC pass fires — once per iteration after all batches are routed. |
