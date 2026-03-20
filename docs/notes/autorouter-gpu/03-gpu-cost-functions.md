# GPU Cost Functions, Congestion Estimation, and Optimization Strategies

Research notes on GPU-efficient cost functions, congestion models, DRC, optimization
passes, and convergence acceleration for our wgpu-based PCB autorouter.

---

## 1. PathFinder Cost Function on GPU

### 1.1 Standard Cost Function

The canonical PathFinder cost function (McMurchie & Ebeling 1995) for routing node `n` is:

```
C(n) = (b_n + h_n) * p_n
```

Where:
- `b_n` = base cost of using node `n` (intrinsic delay/distance)
- `h_n` = accumulated history cost (how many past iterations had overuse at `n`)
- `p_n` = present congestion penalty: `1 + max(0, occupancy_n - capacity_n) * pres_fac`

VPR's timing-driven variant extends this with a criticality blend:

```
C(n, i, j) = alpha_ij * d_n + (1 - alpha_ij) * (b_n + h_n) * p_n
```

where `d_n` is delay and `alpha_ij` is the criticality of net segment i->j. For
PCB routing (no timing-driven requirement in initial implementation), we use the
simpler congestion-only form.

**Reference**: [Quantifying and Reducing Delay Noise in VPR-PathFinder](https://ic.ese.upenn.edu/pdf/pathfinder_noise_fpga2011.pdf)
(Rubin & DeHon, FPGA 2011)

### 1.2 Fixed-Point u32 Encoding for atomicMin

WGSL provides `atomicMin` on `atomic<u32>` but not on `f32`. We encode costs as
fixed-point u32 values. The design space:

**Scaling factor selection**:

| Scale | Resolution | Max representable cost | Bits used |
|-------|-----------|----------------------|-----------|
| x100  | 0.01      | 42,949,672           | ~26 bits  |
| x1000 | 0.001     | 4,294,967            | ~22 bits  |
| x4096 | ~0.000244 | 1,048,575            | ~20 bits  |
| x8192 | ~0.000122 | 524,287              | ~19 bits  |

**Recommendation: scale factor = 1024 (2^10)**.

Rationale:
- Power-of-2 scaling means multiply/divide is a bit shift (free on GPU)
- Resolution of ~0.001 (1/1024 = 0.000977) is sufficient for routing costs
- Max representable cost: 4,194,303 -- a board diagonal of 600mm at 0.1mm grid is
  6000 cells; with 8 layers and generous cost multipliers, worst-case path cost is
  well under 1M. Factor of 4x headroom.
- Avoids any rounding bias that non-power-of-2 scales introduce in repeated
  add-then-compare operations

**Encoding in WGSL**:

```wgsl
const COST_SCALE: u32 = 1024u;  // 2^10, shift instead of multiply
const INFINITY: u32 = 0xFFFFFFFFu;

fn encode_cost(cost_fp: f32) -> u32 {
    return u32(cost_fp * f32(COST_SCALE));
}

fn try_relax(src: u32, neighbor: u32, src_dist: u32, edge_cost: u32) {
    let new_dist = src_dist + edge_cost;
    // Overflow guard: if addition wrapped, don't relax
    if (new_dist < src_dist) { return; }
    atomicMin(&dist[neighbor], new_dist);
}
```

**Predecessor encoding**: Pack predecessor info into u32 alongside cost updates.
Two options:

1. **Separate predecessor buffer** (recommended): `atomicMin` on dist, then check
   if we won the race and write predecessor. Race-free because the thread that
   achieves the minimum also writes the correct predecessor.

   ```wgsl
   let old = atomicMin(&dist[neighbor], new_dist);
   if (new_dist < old) {
       // We won -- write predecessor direction
       predecessor[neighbor] = encode_direction(src, neighbor);
   }
   ```

   Note: this has a subtle TOCTOU race when two threads have the same `new_dist`.
   Both may think they won. This is benign -- either predecessor is valid since
   both yield the same optimal cost. For determinism, break ties with thread ID
   or source index.

2. **Packed cost+predecessor in u64** (if available via `SHADER_INT64_ATOMIC_MIN_MAX`):
   Encode cost in upper 32 bits, predecessor in lower 32. Single `atomicMin` on u64
   is atomic w.r.t. both fields. Requires native feature -- not portable.

### 1.3 Additional Cost Terms

The beauty of the grid-based Bellman-Ford approach is that additional cost terms
are just arithmetic on the edge weight computation -- no algorithmic changes needed.
Each term adds a few ALU ops per thread per edge relaxation. On GPU, ALU is
essentially free compared to memory latency.

#### Layer Preference Penalty

Penalize routing on non-preferred layers to steer signals toward their assigned
layer from global routing:

```wgsl
fn layer_penalty(layer: u32, net_layer_pref: u32) -> u32 {
    if (layer == net_layer_pref) { return 0u; }
    return LAYER_PREF_PENALTY;  // e.g., 50 * COST_SCALE
}
```

Implementation: Store per-net preferred layer in a small uniform/storage buffer
indexed by net ID. Cost: 1 buffer read + 1 comparison per edge.

#### Direction Bias (H/V Preferred Direction per Layer)

PCB convention: even layers route horizontally, odd layers route vertically (or
vice versa, configurable per stackup). Penalize movement against preferred direction:

```wgsl
fn direction_penalty(layer: u32, dx: i32, dy: i32) -> u32 {
    let preferred_h = (layer % 2u) == 0u;  // even = horizontal
    if (preferred_h && dy != 0) {
        return DIRECTION_BIAS_PENALTY;  // e.g., 20 * COST_SCALE
    }
    if (!preferred_h && dx != 0) {
        return DIRECTION_BIAS_PENALTY;
    }
    return 0u;
}
```

Implementation: Encode preferred direction per layer in a small array (max 32
layers = 32 bytes). Cost: 1 byte read + 1 branch per edge. The branch is coherent
within a layer (all threads on same layer take same branch), so no divergence penalty.

#### Via Proximity Penalty

Discourage vias from clustering together (manufacturing concern, crosstalk). For
each via transition, add a penalty based on nearby existing vias:

```wgsl
fn via_proximity_penalty(x: u32, y: u32, layer: u32) -> u32 {
    // Read via_density map (u8 per grid cell, count of vias in 3x3 neighborhood)
    let density = via_density_map[cell_index(x, y, layer)];
    return u32(density) * VIA_PROXIMITY_SCALE;  // linear penalty
}
```

Implementation: Maintain a via density map (u8 per cell) updated on CPU after each
net is routed. Upload to GPU as a read-only storage buffer. Cost: 1 byte read + 1
multiply per via transition edge. Only evaluated for layer-change edges (6 edges
out of ~10 per node), so amortized cost is low.

#### Differential Pair Coupling Penalty

For diff-pair routing, the positive net is routed first, then the negative net is
routed with a coupling bonus for cells adjacent to the positive net's path:

```wgsl
fn coupling_bonus(x: u32, y: u32, layer: u32, partner_path_map: ptr<storage>) -> u32 {
    // Check if partner net occupies adjacent cell at correct spacing
    let gap_cells = diff_pair_gap / grid_resolution;
    let left  = partner_path_map[cell_index(x - gap_cells, y, layer)];
    let right = partner_path_map[cell_index(x + gap_cells, y, layer)];
    let above = partner_path_map[cell_index(x, y - gap_cells, layer)];
    let below = partner_path_map[cell_index(x, y + gap_cells, layer)];
    if (left != 0u || right != 0u || above != 0u || below != 0u) {
        return COUPLING_BONUS;  // negative cost (subtract from edge weight)
    }
    return 0u;
}
```

Implementation: After routing the positive net, mark its path in a bitmap. When
routing the negative net, the GPU reads this bitmap to compute coupling bonuses.
Cost: 4 buffer reads per edge (only for diff-pair nets -- skip for regular nets
via a uniform flag).

#### Crosstalk Penalty (Adjacent-Net Awareness)

Penalize routing adjacent to high-speed or sensitive nets. This requires a per-cell
"aggressor map" that records which nets occupy each cell:

```wgsl
fn crosstalk_penalty(x: u32, y: u32, layer: u32, current_net_class: u32) -> u32 {
    // Check 4 adjacent cells for nets in incompatible classes
    var penalty = 0u;
    for (var dir = 0u; dir < 4u; dir++) {
        let adj = adjacent_cell(x, y, layer, dir);
        let occupant_class = occupant_map[adj];
        if (occupant_class != 0u && is_sensitive_pair(current_net_class, occupant_class)) {
            penalty += CROSSTALK_PENALTY;
        }
    }
    return penalty;
}
```

Implementation: Store a net-class ID (u8) per cell in an occupant map. The
`is_sensitive_pair` check can be a lookup table (32x32 = 1KB for 32 net classes)
stored in uniform buffer. Cost: 4 reads + 4 table lookups per edge. This is the
most expensive additional term but still ALU-bound on GPU.

**Total cost function**:

```wgsl
fn edge_cost(src: u32, neighbor: u32, layer: u32, dx: i32, dy: i32, is_via: bool) -> u32 {
    var cost = base_cost;                                    // 1 * COST_SCALE
    cost += history[neighbor] * params.pres_fac;             // PathFinder h_n * p_n
    cost += present_congestion_penalty(neighbor);            // PathFinder occupancy
    cost += direction_penalty(layer, dx, dy);                // H/V bias
    cost += layer_penalty(layer, params.net_layer_pref);     // Layer preference
    if (is_via) {
        cost += params.via_base_cost;                        // Via base penalty
        cost += via_proximity_penalty(x, y, layer);          // Via clustering
    }
    cost += crosstalk_penalty(x, y, layer, params.net_class); // Crosstalk
    // Diff-pair coupling (only when routing partner net)
    if (params.is_diff_pair_partner) {
        cost -= coupling_bonus(x, y, layer);  // Reward coupling
    }
    return cost;
}
```

**Performance estimate**: The base Bellman-Ford step does 1 atomicLoad + 4-6
atomicMin + ~20 ALU ops per thread. Adding all cost terms brings it to ~60-80
ALU ops and ~10-15 buffer reads per thread. On a modern GPU (1000+ ALU units,
100+ GB/s bandwidth), this is still memory-bound, not compute-bound. The extra
cost terms are essentially free.

---

## 2. Congestion Estimation on GPU

### 2.1 RUDY (Rectangular Uniform wire DensitY)

RUDY (Spindler & Johannes, DATE 2007) estimates routing demand without actually
routing. For each net, it spreads a uniform wire density over the net's bounding box:

```
demand(cell) += wire_density_contribution(net)
             = L(net) / (W_bbox * H_bbox)
```

Where `L(net)` is the estimated wirelength (half-perimeter of bounding box), and
`W_bbox * H_bbox` is the bounding box area. Each cell within the bounding box
receives an equal share of the net's routing demand.

**GPU implementation**:

```wgsl
@compute @workgroup_size(64)
fn rudy_kernel(@builtin(global_invocation_id) gid: vec3<u32>) {
    let net_idx = gid.x;
    if (net_idx >= params.num_nets) { return; }

    let bbox = net_bboxes[net_idx];
    let width = bbox.x_max - bbox.x_min;
    let height = bbox.y_max - bbox.y_min;
    let area = max(width * height, 1u);  // avoid div-by-zero
    let hpwl = width + height;
    // Fixed-point: density * COST_SCALE
    let density = (hpwl * COST_SCALE) / area;

    // Spread over bounding box cells
    for (var x = bbox.x_min; x <= bbox.x_max; x++) {
        for (var y = bbox.y_min; y <= bbox.y_max; y++) {
            let idx = x * params.grid_height + y;
            atomicAdd(&congestion_grid[idx], density);
        }
    }
}
```

**Performance**: One thread per net. For N nets with average bounding box of AxA
cells, total work is O(N * A^2). For 1000 nets with average 20x20 bbox = 400K
atomicAdds. At ~1 billion atomicAdds/second on a modern GPU, this completes in
< 1ms.

**Limitation**: RUDY is a 2D model -- it does not account for layer assignment.
For multi-layer congestion, scale demand by `1/num_allowed_layers` per net.

**PinRUDY extension**: Weight density by pin locations within the bounding box.
Cells near pins get higher demand. This improves accuracy for nets with clustered
pins.

**Reference**: [Fast and Accurate Routing Demand Estimation (DATE 2007)](https://past.date-conference.com/proceedings-archive/2007/DATE07/PDFFILES/08.7_1.PDF)

**Rudys (2025)**: A recent extension that accounts for macrocell area occupancy
and pin demand with GPU acceleration, achieving more accurate congestion estimates
for placement-driven design.

**Reference**: [Rudys: A highly efficient routing demand estimator](https://www.sciencedirect.com/science/article/abs/pii/S1879239125003777)

### 2.2 Probabilistic Congestion Models

Beyond RUDY, probabilistic models estimate congestion by computing the probability
that a net's optimal route passes through each grid cell:

**Probability-based model**: For a 2-pin net from (x1,y1) to (x2,y2), the
probability that an optimal (shortest) route passes through cell (x,y) is:

```
P(x,y) = C(dx_left + dy_above, dx_left) * C(dx_right + dy_below, dx_right)
         / C(dx_total + dy_total, dx_total)
```

where `C(n,k)` is binomial coefficient, and dx_left/dx_right/dy_above/dy_below
are Manhattan distances from (x,y) to source/target.

**GPU implementation**: Each thread handles one (net, cell) pair. The binomial
coefficients can be precomputed in a lookup table (bounded by grid dimensions).
This is embarrassingly parallel.

**Advantage over RUDY**: Concentrates demand along likely routing corridors rather
than uniformly filling bounding boxes. Produces more accurate congestion maps,
especially for nets where the bbox is much larger than the actual routing path.

### 2.3 Real-Time Congestion Heatmap Generation

For viewer playback, we need to generate congestion heatmaps from routing state:

**Per-iteration heatmap** (for PathFinder playback):

```wgsl
@compute @workgroup_size(64)
fn generate_heatmap(@builtin(global_invocation_id) gid: vec3<u32>) {
    let cell = gid.x;
    if (cell >= params.total_cells_2d) { return; }

    // Sum occupancy across all layers for this (x,y)
    var total_demand = 0u;
    var total_capacity = 0u;
    for (var layer = 0u; layer < params.num_layers; layer++) {
        let idx_3d = cell * params.num_layers + layer;
        total_demand += demand[idx_3d];
        total_capacity += capacity[idx_3d];
    }

    // Congestion ratio: demand / capacity (fixed-point)
    let ratio = (total_demand * 256u) / max(total_capacity, 1u);
    heatmap[cell] = min(ratio, 255u);  // u8 color intensity
}
```

This kernel runs once per PathFinder iteration. Output is a 2D u8 texture that
the viewer renders as a color overlay. At 2000x2000 grid = 4M cells, one dispatch
completes in < 0.1ms.

**Streaming to viewer**: The heatmap buffer can be mapped for CPU readback and
stored in the `RoutingIterationSnapshot`. For live playback, the GPU writes directly
to a texture that the viewer's render pipeline samples.

### 2.4 How DREAMPlace Computes Routing Congestion

DREAMPlace (Lin et al., DAC 2019) uses RUDY-based congestion estimation within
its GPU-accelerated analytical placement loop:

1. **Cell inflation**: After each placement iteration, estimate congestion via RUDY
2. **Inflate cells**: In congested regions, increase cell area (virtual padding)
   to spread density
3. **Re-place**: Run another iteration of gradient-based placement with inflated cells
4. **Converge**: Repeat until congestion is acceptable

The GPU kernels for congestion estimation in DREAMPlace:
- Compute net bounding boxes (one thread per net, parallel min/max reduction)
- RUDY demand spreading (one thread per net, atomicAdd to grid)
- Pin density computation (one thread per pin, atomicAdd to grid)
- Congestion ratio computation (one thread per grid cell)

**Reference**: [Global Placement with Deep Learning-Enabled Explicit Routability Optimization](https://www.cse.cuhk.edu.hk/~byu/papers/C112-DATE2021-DREAMPlace-Cong.pdf)

### 2.5 Cypress: GPU PCB Placement with Congestion Feedback (ISPD 2025 Best Paper)

Cypress (Zhang et al., ISPD 2025) is directly relevant -- it applies VLSI-inspired
GPU-accelerated placement to PCB, including routability feedback:

- **Net crossing metric**: Decomposes multi-pin nets into pin pairs with line
  segments; counts crossings as a proxy for routing congestion
- **Macro halo technique**: Temporarily enlarges component footprints during
  placement to create buffer zones, preventing overlap and improving routability
- **Results**: 1-5.9x higher routability, up to 492x speedup with GPU acceleration

This validates the approach of feeding GPU-computed congestion estimates back into
placement optimization for PCB-scale designs.

**Reference**: [Cypress: VLSI-Inspired PCB Placement with GPU Acceleration](https://www.csl.cornell.edu/~zhiruz/pdfs/cypress-ispd2025.pdf)

### 2.6 GPU Congestion Feedback into Placement SA

**Yes, GPU congestion estimates can feed back into our placement SA**. The pipeline:

```
Placement SA iteration
  |
  v
GPU: RUDY congestion estimation  (< 1ms for 1000 nets)
  |
  v
CPU: Congestion penalty term in SA cost function
  |
  v
CPU: Metropolis accept/reject
```

The RUDY kernel is fast enough to run at every SA temperature step (or every K
moves). The congestion grid stays on GPU between evaluations -- only the changed
net bounding boxes need updating.

**Integration with existing SA cost function**:

```rust
// In SA cost evaluation
let hpwl_cost = evaluate_hpwl(candidate);
let overlap_cost = evaluate_overlap(candidate);
let congestion_cost = gpu_rudy_evaluate(candidate);  // GPU dispatch
let total_cost = w_hpwl * hpwl_cost
               + w_overlap * overlap_cost
               + w_congestion * congestion_cost;  // New term
```

The congestion weight `w_congestion` should be low in early SA (high temperature,
focus on global optimization) and increase in late SA (low temperature, focus on
routability refinement).

---

## 3. GPU-Friendly DRC (Design Rule Check)

### 3.1 X-Check: GPU-Accelerated DRC via Parallel Sweepline

X-Check (He, Ma, Yu -- ICCAD 2022) demonstrates that GPU DRC is viable and fast:

- **Parallel sweepline algorithm**: Decompose 2D clearance checking into 1D sweep
  operations, parallelized across GPU threads
- **Performance**: Average 61x speedup for space checking, up to 1258x on large
  designs vs. single-threaded CPU DRC
- **Three DRC tasks accelerated**: width checking, space (clearance) checking,
  enclosure checking

**Reference**: [X-Check: GPU-Accelerated Design Rule Checking via Parallel Sweepline Algorithms](https://www.cse.cuhk.edu.hk/~byu/papers/C149-ICCAD2022-GPU-DRC.pdf)

OpenDRC (DAC 2023) extends this with hierarchical GPU acceleration and achieves
3.2-12x speedup over sequential DRC:

**Reference**: [OpenDRC: An Efficient Open-Source Design Rule Checking Engine](https://www.cse.cuhk.edu.hk/~byu/papers/C172-DAC2023-OpenDRC.pdf)

### 3.2 Obstacle Inflation (Baking Clearance into the Grid)

The simplest GPU-friendly DRC approach for grid-based routing: **inflate obstacles
by the clearance distance**, then treat any overlap as a violation. This is the
Minkowski sum of the obstacle geometry with a disc of radius = clearance.

On a grid, inflation is a morphological dilation:

```wgsl
@compute @workgroup_size(64)
fn inflate_obstacles(@builtin(global_invocation_id) gid: vec3<u32>) {
    let cell = gid.x;
    if (cell >= params.total_cells) { return; }

    // Skip if cell is already an obstacle
    if (is_obstacle(cell)) { return; }

    let x = cell_x(cell);
    let y = cell_y(cell);
    let radius = params.clearance_cells;  // clearance / grid_resolution

    // Check all cells within clearance radius
    for (var dx = -i32(radius); dx <= i32(radius); dx++) {
        for (var dy = -i32(radius); dy <= i32(radius); dy++) {
            // Use Manhattan or Chebyshev distance (cheaper than Euclidean)
            if (u32(abs(dx) + abs(dy)) > radius) { continue; }
            let nx = i32(x) + dx;
            let ny = i32(y) + dy;
            if (out_of_bounds(nx, ny)) { continue; }
            if (is_raw_obstacle(u32(nx), u32(ny))) {
                // Mark this cell as blocked (within clearance of an obstacle)
                set_inflated_obstacle(cell);
                return;
            }
        }
    }
}
```

**Performance**: One thread per grid cell. For a 2000x2000 grid with 3-cell
clearance radius, each thread checks ~25 neighbors. Total: 4M * 25 = 100M reads.
Completes in < 10ms.

**Advantage**: After inflation, DRC during routing is trivial -- just check the
inflated obstacle map. No clearance computation needed per edge relaxation.

**Limitation**: A single inflation radius assumes uniform clearance for all nets.
For per-net-class clearance, we need a different approach.

### 3.3 Per-Net-Class Clearance

Three approaches for handling different clearance requirements per net class:

**Approach A: Multiple obstacle maps** (recommended for small number of classes):

Generate one inflated obstacle map per clearance value. During routing, select the
map matching the current net's clearance class.

- Memory: K maps for K distinct clearance values. Typical PCB: 2-4 classes.
  At 2000x2000 grid with 4 layers = 32M bits per map = 4MB per map.
  4 classes = 16MB total. Acceptable.
- GPU: Generate all maps in parallel (one dispatch per class, or batch).
- Routing: Index into correct map via net class ID.

```wgsl
fn is_blocked_for_net(cell: u32, net_class: u32) -> bool {
    let map_offset = net_class * params.cells_per_map;
    return obstacle_maps[map_offset + cell] != 0u;
}
```

**Approach B: Parameterized cost** (recommended for many classes):

Store raw (uninflated) obstacles. During routing, compute clearance violation
dynamically:

```wgsl
fn clearance_cost(cell: u32, net_clearance: u32) -> u32 {
    let min_dist = nearest_obstacle_distance[cell];  // precomputed distance transform
    if (min_dist >= net_clearance) { return 0u; }
    return (net_clearance - min_dist) * CLEARANCE_VIOLATION_PENALTY;
}
```

This requires a precomputed distance transform (distance to nearest obstacle per
cell), which can itself be computed on GPU using jump flooding algorithm (JFA).
JFA runs in O(log N) passes on an NxN grid.

**Approach C: Hybrid**: Use Approach A for the 2-3 most common clearance classes,
Approach B for rare overrides.

### 3.4 Short-Circuit Detection on GPU

A short circuit occurs when two different nets share a routing resource. During
PathFinder, shorts are detected via the occupancy array -- any cell with
occupancy > capacity has multiple nets routed through it.

**GPU kernel for short detection**:

```wgsl
@compute @workgroup_size(64)
fn detect_shorts(@builtin(global_invocation_id) gid: vec3<u32>) {
    let cell = gid.x;
    if (cell >= params.total_cells) { return; }

    let occupancy = occupancy_map[cell];
    if (occupancy > capacity_map[cell]) {
        // Short or overuse detected
        atomicAdd(&short_count, 1u);
        // Record location for debugging
        let write_idx = atomicAdd(&short_list_len, 1u);
        if (write_idx < MAX_SHORTS) {
            short_locations[write_idx] = cell;
        }
    }
}
```

For full short-circuit detection (are two specific nets connected that shouldn't
be?), this requires connected component analysis on the routing graph. GPU-parallel
connected component algorithms exist (ECL-CC, LACC) but are overkill for our
PathFinder loop where occupancy checking is sufficient.

**Reference**: [High Performance Detection of Strongly Connected Components on GPUs](https://dl.acm.org/doi/10.1145/3026937.3026941) (PMAM 2017)

---

## 4. Optimization Passes on GPU

### 4.1 Staircase Elimination

**Problem**: Grid-based routing produces staircase patterns (alternating H-V
segments) where a single diagonal segment would suffice.

**Pattern**: A staircase is a sequence of segments: H(dx1)-V(dy1)-H(dx2)-V(dy2)...
where all H segments go in the same direction and all V segments go in the same
direction. This can be replaced by a 45-degree diagonal + short H or V tail.

**Can it run on GPU?** Yes, with caveats.

**GPU approach -- parallel pattern scan**:

```
1. Represent each net's path as an array of segment directions
   (encoded as u8: N=0, E=1, S=2, W=3)
2. GPU kernel: each thread examines one segment and its neighbors
3. Mark segments that form staircase patterns (consecutive H-V or V-H pairs
   where the pair can be replaced by a diagonal)
4. CPU: apply marked replacements (sequential, since replacements may interact)
```

The detection phase is embarrassingly parallel (one thread per segment, check
2 neighbors). The application phase should remain on CPU because replacements
can overlap -- removing one staircase step may invalidate the next.

**Estimated speedup from GPU**: Modest (2-5x). The bottleneck is the CPU
application phase, not detection. For boards with < 100K segments, CPU-only
staircase elimination runs in < 100ms -- GPU acceleration is not justified
unless combined with other optimization passes.

**Recommendation**: CPU-only initially. Revisit if profiling shows it as a
bottleneck on large boards.

### 4.2 Rubber-Banding

**Problem**: Pull routed traces tight against obstacles to free up routing
resources and reduce wirelength.

**Algorithm**: Iteratively pull each vertex of a trace toward its optimal
position (minimizing total wirelength) subject to clearance constraints. This
is essentially a constrained optimization per vertex.

**Can it run on GPU?** Partially.

**GPU-amenable component**: The optimal vertex position computation (geometric
calculation per vertex, clearance query against spatial index) can be parallelized.
Each thread handles one vertex independently.

**Sequential dependency**: Pulling vertex V_i affects the optimal position of
V_{i-1} and V_{i+1}. Converging to a global optimum requires iterative
Gauss-Seidel-style updates. On GPU, use Jacobi-style updates (all vertices
update simultaneously from previous positions), which converges slower but is
fully parallel.

```wgsl
@compute @workgroup_size(64)
fn rubber_band_step(@builtin(global_invocation_id) gid: vec3<u32>) {
    let vertex_idx = gid.x;
    if (vertex_idx >= params.total_vertices) { return; }

    let prev = vertices[vertex_idx - 1];  // neighbor
    let curr = vertices[vertex_idx];
    let next = vertices[vertex_idx + 1];  // neighbor

    // Optimal position: on the line segment from prev to next
    let target = closest_point_on_line(prev, next, curr);

    // Clamp to clearance from obstacles
    let clamped = clamp_to_clearance(target, curr.layer, params.clearance);

    // Write to output buffer (double-buffered, read old / write new)
    new_vertices[vertex_idx] = clamped;
}
```

Run for K iterations (typically 5-10 converge). Each iteration is one GPU
dispatch.

**Estimated speedup**: 5-20x for boards with > 50K vertices. The geometric
computations (line intersection, point-to-segment distance) are ALU-heavy and
map well to GPU.

**Clearance checking on GPU**: The `clamp_to_clearance` function needs to query
nearby obstacles. Using the precomputed distance transform (from Section 3.3
Approach B), this is a single buffer read -- very fast.

### 4.3 Serpentine/Accordion Insertion

**Problem**: After routing, some nets in matched-length groups are shorter than
the target length. Insert serpentine meanders to add the required length.

**CPU vs GPU split**:

| Phase | Where | Why |
|-------|-------|-----|
| Compute target lengths | GPU | Parallel per-net length measurement: sum segment lengths |
| Compute length deficits | GPU | Subtract actual from target, per group |
| Find insertion points | CPU | Sequential scan along path for uncongested segments |
| Generate meander geometry | CPU | Complex geometry (arc/chamfer options, DRC checking) |
| DRC validate meanders | GPU | Parallel clearance checking of inserted geometry |

**GPU target length computation**:

```wgsl
@compute @workgroup_size(64)
fn compute_net_lengths(@builtin(global_invocation_id) gid: vec3<u32>) {
    let net_idx = gid.x;
    if (net_idx >= params.num_nets) { return; }

    let seg_start = net_segment_offsets[net_idx];
    let seg_end = net_segment_offsets[net_idx + 1];
    var total_length = 0u;  // fixed-point
    for (var i = seg_start; i < seg_end; i++) {
        let seg = segments[i];
        let dx = abs(i32(seg.x2) - i32(seg.x1));
        let dy = abs(i32(seg.y2) - i32(seg.y1));
        // Manhattan length (or Euclidean via integer sqrt)
        total_length += u32(dx + dy) * COST_SCALE;
    }
    net_lengths[net_idx] = total_length;
}
```

**Recommendation**: Compute lengths on GPU, everything else on CPU. Serpentine
geometry generation involves complex clearance-aware decisions (where to insert,
amplitude, pitch, DRC validation) that don't benefit from GPU parallelism.

---

## 5. Convergence Acceleration Techniques

### 5.1 Adaptive Pressure Factor

The standard PathFinder increases `pres_fac` by a fixed multiplier each iteration:

```
pres_fac(t) = pres_fac_init * multiplier^(t-1)
```

OrthoRoute uses `multiplier = 1.3` and caps at `pres_fac_max = 8.0`.
VPR uses `pres_fac_mult` (configurable, default varies by architecture).

**Problems with fixed exponential growth**:
- Too aggressive: causes oscillation in late iterations (nets bounce between
  resources as penalty grows faster than they can adapt)
- Too conservative: wastes iterations converging on easy boards
- OrthoRoute found that `history_decay = 0.995` caused exponential growth --
  removed it entirely

**Adaptive strategy**: Adjust `pres_fac` growth based on convergence rate:

```rust
fn adaptive_pres_fac(
    pres_fac: &mut f64,
    prev_conflicts: usize,
    curr_conflicts: usize,
    iteration: usize,
) {
    let improvement_ratio = 1.0 - (curr_conflicts as f64 / prev_conflicts.max(1) as f64);

    if improvement_ratio > 0.1 {
        // Good progress -- gentle increase
        *pres_fac *= 1.05;
    } else if improvement_ratio > 0.0 {
        // Some progress -- moderate increase
        *pres_fac *= 1.15;
    } else {
        // No progress or regression -- aggressive increase
        *pres_fac *= 1.5;
    }

    // Cap to prevent oscillation
    *pres_fac = pres_fac.min(PRES_FAC_MAX);
}
```

**Reference**: The "Revisiting PathFinder" paper (Zha & Li, FPGA 2022) found that
the traditional pressure update strategy has fundamental issues. They propose a
**constant cost gap** strategy instead of exponential growth, reducing critical
path delay variation by up to 96.2%.

**Reference**: [Revisiting PathFinder Routing Algorithm (FPGA 2022)](https://dl.acm.org/doi/10.1145/3490422.3502356)

**"Guaranteed Yet Hard to Find"** (2024) further explores cases where PathFinder
has guaranteed convergence but struggles to find solutions. The paper constructs
adversarial cases revealing inherent algorithmic limitations.

**Reference**: [Guaranteed Yet Hard to Find: Uncovering FPGA Routing Convergence Paradox](https://ieeexplore.ieee.org/document/11008981/)

### 5.2 Hot-Set Tracking on GPU

Instead of ripping up all nets each iteration, track the "hot set" -- nets that
pass through oversubscribed cells. Only rip up and reroute these nets.

**GPU kernel for hot-set identification**:

```wgsl
@compute @workgroup_size(64)
fn identify_hot_cells(@builtin(global_invocation_id) gid: vec3<u32>) {
    let cell = gid.x;
    if (cell >= params.total_cells) { return; }

    let demand = demand_map[cell];
    let capacity = capacity_map[cell];
    if (demand > capacity) {
        hot_cell_bitmap[cell / 32u] |= (1u << (cell % 32u));
        atomicAdd(&hot_cell_count, 1u);
    }
}
```

**Net-to-hot-cell mapping** (GPU):

```wgsl
@compute @workgroup_size(64)
fn mark_hot_nets(@builtin(global_invocation_id) gid: vec3<u32>) {
    let net_idx = gid.x;
    if (net_idx >= params.num_nets) { return; }

    // Scan net's path for hot cells
    let seg_start = net_segment_offsets[net_idx];
    let seg_end = net_segment_offsets[net_idx + 1];
    for (var i = seg_start; i < seg_end; i++) {
        let cell = path_cells[i];
        if (is_hot_cell(cell)) {
            hot_net_flags[net_idx] = 1u;
            return;
        }
    }
}
```

**CPU**: Read back `hot_net_flags`, build hot-set list, rip up only those nets.

OrthoRoute uses a fixed hot-set size of 100 nets (found adaptive sizing caused
oscillation). Our approach: start with all nets in hot-set for first few
iterations (full rip-up), then transition to tracked hot-set after iteration 5.

**Estimated benefit**: For a 1000-net board where only 50 nets have conflicts in
later iterations, hot-set routing is 20x faster per iteration than full rip-up.
The GPU identification of hot nets is < 1ms.

### 5.3 Early Termination Heuristics

**Convergence detection**:

```rust
fn should_terminate(state: &PathFinderState) -> bool {
    // 1. No conflicts -- converged
    if state.current_conflicts == 0 {
        return true;
    }

    // 2. Max iterations reached
    if state.iteration >= state.config.max_iterations {
        return true;
    }

    // 3. Plateau detection: no improvement for N iterations
    if state.iterations_without_improvement >= PLATEAU_THRESHOLD {
        return true;
    }

    // 4. Oscillation detection: conflicts alternating up/down
    if state.is_oscillating(window_size: 6) {
        // Halve pres_fac and continue (or terminate)
        return false;  // try recovery first
    }

    false
}
```

**Plateau detection**: Track conflict count over a sliding window. If the
standard deviation of conflict counts in the window is < 5% of the mean,
we're in a plateau. Options:
- Terminate and report unrouted nets
- Increase pres_fac aggressively (2x) to force convergence
- Switch to full rip-up (if using hot-set)

**Oscillation detection**: Track whether conflict count alternates between
two values (or a small set). This indicates pres_fac is too high -- nets are
bouncing between equivalent resources.

### 5.4 Congestion Trend Prediction

**Can we predict convergence from congestion trends?**

**ML-based approach** (Hua & Bhatt, MLCAD 2022): Train a model to forecast
congestion costs from early iterations, pre-loading this information into the
router to avoid exploring highly congested regions. Achieved 43% reduction in
routing iterations and 28.6% reduction in runtime.

**Reference**: [Faster FPGA Routing by Forecasting and Pre-Loading Congestion Information](https://dl.acm.org/doi/10.1145/3551901.3556492)

**Simpler heuristic for our use**: Track three metrics across iterations:

1. **Conflict count** `C(t)`: Should decrease monotonically (on average)
2. **Total overuse** `O(t) = sum(max(0, demand - capacity))`: Should decrease
3. **Max single-cell overuse** `M(t) = max(demand - capacity)`: Indicates
   worst bottleneck

Fit exponential decay to `C(t)`: `C(t) ~ C(0) * exp(-k*t)`. Extrapolate to
predict how many more iterations needed for `C(t) < 1`:

```rust
fn predict_remaining_iterations(history: &[usize]) -> Option<usize> {
    if history.len() < 5 { return None; }

    // Fit log-linear model: log(C(t)) = a - k*t
    // k = -(log(C_last) - log(C_first)) / (t_last - t_first)
    let first = history.first()?.max(&1);
    let last = history.last()?.max(&1);
    let n = history.len() as f64;

    let k = ((*first as f64).ln() - (*last as f64).ln()) / n;
    if k <= 0.0 { return None; }  // Not converging

    // Predict iterations until C(t) < 1
    let remaining = (*last as f64).ln() / k;
    Some(remaining.ceil() as usize)
}
```

This prediction can inform:
- User-facing progress bars ("estimated 12 more iterations")
- Dynamic resource allocation (if predicted iterations > 3x budget, abort early
  and report bottleneck)
- Adaptive pres_fac scheduling (if converging fast, keep gentle; if slow, escalate)

### 5.5 Congestion Pre-Loading from Spec

Our spec-centric architecture provides a unique advantage: the LLM can declare
expected congestion regions in the spec, which the router uses to pre-seed the
history array:

```
congestion_warning {
    region: rect(30mm, 40mm, 35mm, 50mm)
    reason: "narrow channel between U1 and J1"
    max_tracks: 4
}
```

**GPU implementation**: Before the first PathFinder iteration, run a kernel that
writes initial history values for cells within declared congestion regions:

```wgsl
@compute @workgroup_size(64)
fn preseed_history(@builtin(global_invocation_id) gid: vec3<u32>) {
    let cell = gid.x;
    if (cell >= params.total_cells) { return; }

    let x = cell_x(cell);
    let y = cell_y(cell);

    // Check if cell is in any congestion warning region
    for (var r = 0u; r < params.num_warning_regions; r++) {
        let region = warning_regions[r];
        if (x >= region.x_min && x <= region.x_max &&
            y >= region.y_min && y <= region.y_max) {
            history[cell] = region.initial_history;  // e.g., 10 * COST_SCALE
        }
    }
}
```

This steers nets away from known bottlenecks from the very first iteration,
potentially saving 5-10 iterations of discovery.

---

## 6. Summary: Implementation Priority

| Feature | Priority | GPU benefit | Complexity |
|---------|----------|------------|------------|
| Fixed-point cost encoding | P0 (required) | Enables atomicMin | Low |
| Base PathFinder cost function | P0 (required) | Core algorithm | Low |
| Direction bias penalty | P1 (early) | Quality improvement | Low |
| Layer preference penalty | P1 (early) | Quality improvement | Low |
| Obstacle inflation | P1 (early) | Simplifies routing DRC | Medium |
| RUDY congestion estimation | P1 (early) | Placement feedback | Low |
| Hot-set tracking | P2 (optimization) | 5-20x iteration speedup | Medium |
| Adaptive pres_fac | P2 (optimization) | Faster convergence | Low |
| Via proximity penalty | P2 (optimization) | Manufacturing quality | Low |
| Convergence prediction | P2 (optimization) | User experience | Low |
| Multiple obstacle maps | P2 (optimization) | Per-class clearance | Medium |
| Diff-pair coupling | P3 (high-speed) | Signal integrity | Medium |
| Crosstalk penalty | P3 (high-speed) | Signal integrity | Medium |
| Rubber-banding on GPU | P3 (post-route) | Wirelength reduction | High |
| Serpentine length compute | P3 (post-route) | Length matching | Low |
| Staircase elimination | P4 (post-route) | Trace quality | Low (CPU-only) |
| Congestion pre-loading | P4 (spec integration) | Faster convergence | Low |

---

## References

### Cost Functions and PathFinder

- [PathFinder: A Negotiation-Based Performance-Driven Router for FPGAs](https://dl.acm.org/doi/10.1145/201310.201328) -- McMurchie & Ebeling, FPGA 1995. Original cost function definition.
- [Quantifying and Reducing Delay Noise in VPR-PathFinder](https://ic.ese.upenn.edu/pdf/pathfinder_noise_fpga2011.pdf) -- Rubin & DeHon, FPGA 2011. VPR timing-driven cost function analysis.
- [Revisiting PathFinder Routing Algorithm](https://dl.acm.org/doi/10.1145/3490422.3502356) -- Zha & Li, FPGA 2022. Constant cost gap strategy, 96.2% variation reduction.
- [Guaranteed Yet Hard to Find: Uncovering FPGA Routing Convergence Paradox](https://ieeexplore.ieee.org/document/11008981/) -- 2024. Adversarial cases for PathFinder.
- [Corolla: GPU-Accelerated FPGA Routing](https://dl.acm.org/doi/10.1145/3020078.3021732) -- FPGA 2017. GPU Bellman-Ford for PathFinder inner loop.
- [OrthoRoute](https://bbenchoff.github.io/pages/OrthoRoute.html) -- Benchoff, 2025. GPU PCB PathFinder with practical convergence lessons.

### Congestion Estimation

- [RUDY: Fast and Accurate Routing Demand Estimation](https://past.date-conference.com/proceedings-archive/2007/DATE07/PDFFILES/08.7_1.PDF) -- Spindler & Johannes, DATE 2007.
- [Rudys: Highly Efficient Routing Demand Estimator](https://www.sciencedirect.com/science/article/abs/pii/S1879239125003777) -- 2025. GPU-accelerated RUDY extension.
- [DREAMPlace Congestion Optimization](https://www.cse.cuhk.edu.hk/~byu/papers/C112-DATE2021-DREAMPlace-Cong.pdf) -- DATE 2021.
- [Cypress: VLSI-Inspired PCB Placement with GPU Acceleration](https://www.csl.cornell.edu/~zhiruz/pdfs/cypress-ispd2025.pdf) -- Zhang et al., ISPD 2025 Best Paper.
- [OpenROAD RUDY Congestion Heatmap Discussion](https://github.com/The-OpenROAD-Project/OpenROAD/issues/6287)

### GPU DRC

- [X-Check: GPU-Accelerated DRC via Parallel Sweepline](https://www.cse.cuhk.edu.hk/~byu/papers/C149-ICCAD2022-GPU-DRC.pdf) -- He, Ma, Yu, ICCAD 2022. 61x average speedup.
- [OpenDRC: Open-Source GPU-Accelerated DRC](https://www.cse.cuhk.edu.hk/~byu/papers/C172-DAC2023-OpenDRC.pdf) -- DAC 2023.
- [PDRC: Package DRC via GPU-Accelerated Geometric Operations](http://www.cse.cuhk.edu.hk/~byu/papers/C219-DAC2024-PDRC.pdf) -- DAC 2024.

### Convergence Acceleration

- [Acceleration Techniques for Modified PathFinder](https://ieeexplore.ieee.org/document/9755536/) -- IEEE 2022. 38% routing time reduction.
- [Faster FPGA Routing by Forecasting Congestion](https://dl.acm.org/doi/10.1145/3551901.3556492) -- MLCAD 2022. ML-based congestion forecasting, 43% fewer iterations.
- [Accelerating FPGA Routing Using Architecture-Adaptive A*](https://people.ece.uw.edu/hauck/publications/A_star.pdf) -- Hauck et al.

### GPU Algorithms

- [GPU-Based Voxelized Minkowski Sum Computation](https://mcmains.me.berkeley.edu/pubs/KrishnamurthyMcMainsCAD11correctedproof.pdf) -- Li & McMains. GPU obstacle inflation.
- [High Performance Connected Components on GPUs](https://dl.acm.org/doi/10.1145/3026937.3026941) -- PMAM 2017. Short detection.
- [WGSL Atomics Reference](https://webgpu.rocks/wgsl/functions/synchronization-atomic/)
- [wgpu Limits](https://docs.rs/wgpu/latest/wgpu/struct.Limits.html)

### Routing and Placement

- [GAMER: GPU-Accelerated Maze Routing](https://ieeexplore.ieee.org/document/9799536) -- IEEE TCAD 2023.
- [InstantGR: GPU Global Routing](https://dl.acm.org/doi/10.1145/3676536.3676787) -- ICCAD 2024.
- [DREAMPlace: GPU Analytical Placement](https://doi.org/10.1145/3316781.3317803) -- DAC 2019.
- [ISPD 2024 GPU/ML Global Routing Contest](https://liangrj2014.github.io/ISPD24_contest/)
- [The Mathematics of PCB Trace Routing](https://tinycomputers.io/posts/the-mathematics-of-pcb-trace-routing.html) -- Freerouting optimization description.
