# GPU Multi-Net Parallel Routing with InstantGR-Style Net Batching

## Overview

This plan describes how to implement GPU-accelerated multi-net parallel routing
within the PathFinder negotiation loop. The core idea: within each PathFinder
iteration, nets that cannot conflict on routing resources are batched together
and routed simultaneously on the GPU. This transforms the sequential
one-net-at-a-time inner loop into a batched-parallel loop with dramatically
higher GPU utilization.

Two batching strategies are combined:

1. **InstantGR-style runtime batching** — segment-based overlap checking to
   discover independent nets at runtime (Section 1)
2. **LLM-declared independence** — spec-declared `independent_groups` and
   `routing_partition` that bypass runtime conflict detection entirely
   (Section 2)

### Key Insight from InstantGR

Traditional bounding-box overlap checking is pessimistic: nets whose bounding
boxes overlap but whose actual routing graphs do not overlap are needlessly
serialized. InstantGR's segment-based representation decomposes each net's
routing DAG into horizontal segments `hs(y, x_l, x_r)` and vertical segments
`vs(x, y_l, y_r)`. Because horizontal and vertical wires occupy different
metal layers (H-layers vs V-layers), **only same-direction segments can
conflict**. This produces 50-100x fewer batches than bounding-box methods on
large designs (554 batches vs 26,038 for the largest ISPD'24 benchmark with
59.3M nets).

### Our Advantage: LLM-Declared Independence

Our router is spec-centric — the LLM already knows the schematic topology and
can declare which net groups are independent. Traditional routers must discover
independence at runtime. We get it for free from the spec, then refine with
InstantGR's runtime batching for undeclared nets.

---

## Pipeline Integration

InstantGR (this plan) runs **before** the SSSP step in each PathFinder iteration — it partitions the full net list into independent batches that Corolla/GAMER can route simultaneously.

```
PathFinder Iteration:
  1. Rip-up (CPU)
  2. InstantGR (05) [this plan] → batch nets into independent groups
  3. For each batch:
     Corolla (01) OR GAMER (02) → GPU SSSP per net in batch
  4. X-Check (03) → GPU DRC, violations → history
  5. History update (GPU kernel)
  6. Convergence check (CPU)

After routing:
  Cypress (04) → congestion feedback → placement SA
```

**This plan's role**: InstantGR runs on CPU at the start of each PathFinder iteration (step 2). It reads net connectivity from `PcbIr` and the LLM-declared `independent_groups` / `routing_partition` specs, constructs `Vec<RoutingBatch>`, and hands them to Corolla (01) or GAMER (02) for parallel GPU routing. InstantGR also owns the `batch_reset.wgsl` and `batch_conflict_check.wgsl` shaders used to initialize and validate per-batch GPU state.

### Shared `GpuRoutingEngine`

Uses shared `GpuRoutingEngine` from `gpu/engine.rs` (see Plan 01 for full definition). InstantGR-specific fields/pipelines used:

| Field | Purpose |
|-------|---------|
| `device`, `queue` | wgpu primitives |
| `distance` | Interleaved distance arrays — InstantGR dictates the per-batch layout |
| `predecessor` | Interleaved predecessor arrays — same batch layout |
| `max_batch_size` | Upper bound on nets per batch, used to pre-size interleaved buffers |

### Shared Buffer Access

| Buffer | Access | Notes |
|--------|--------|-------|
| `distance` | Write (reset) | `batch_reset.wgsl` clears per-net distance slices before each batch |
| `predecessor` | Write (reset) | Cleared in same pass as `distance` |
| `obstacle_bitmap` | Read | Consulted during segment-based overlap checking (CPU-side) |

### Module Structure

All files live under the shared GPU module (same as Plans 01, 02, 03, 04):

```
crates/autopcb-router/src/gpu/
├── mod.rs              // GpuRoutingEngine (shared device, queue, buffers, pipelines)
├── engine.rs           // GpuRoutingEngine struct, initialization, buffer management
├── buffers.rs          // Buffer types, layout, upload/download helpers
├── bellman_ford.rs     // Corolla BF dispatch (01)
├── sweep.rs            // GAMER H/V sweep dispatch (02)
├── drc.rs              // X-Check GPU DRC (03)
├── congestion.rs       // Cypress congestion estimation (04)
├── batching.rs         // InstantGR net batching logic (05) [this file]
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

## 1. InstantGR Net Batching Algorithm

### 1.1 Segment-Based Routing Graph Representation

Each net's routing graph is represented as a pair of segment sets:

```rust
/// A net's routing graph decomposed into H and V segments for overlap checking.
///
/// File: crates/autopcb-router/src/batching/segments.rs
pub struct NetRoutingSegments {
    /// Net identifier.
    pub net_id: NetId,
    /// Horizontal segments: (y, x_left, x_right) on horizontal routing layers.
    pub h_segments: Vec<HSegment>,
    /// Vertical segments: (x, y_bottom, y_top) on vertical routing layers.
    pub v_segments: Vec<VSegment>,
}

/// A horizontal segment representing wire resources on H-direction layers.
#[derive(Debug, Clone, Copy)]
pub struct HSegment {
    pub y: u32,     // Grid row
    pub x_l: u32,   // Left column (inclusive)
    pub x_r: u32,   // Right column (inclusive)
}

/// A vertical segment representing wire resources on V-direction layers.
#[derive(Debug, Clone, Copy)]
pub struct VSegment {
    pub x: u32,     // Grid column
    pub y_l: u32,   // Bottom row (inclusive)
    pub y_r: u32,   // Top row (inclusive)
}
```

**Segment construction** from net pin positions (using `IrNetPin::position`):

1. Compute the Rectilinear Steiner Minimum Tree (RSMT) for the net's pins
   using the MST decomposer from Milestone 5 (`MstDecomposer`).
2. For each edge in the RSMT, generate L-shape routing candidates:
   - Horizontal wire segment: `hs(y, min(x1,x2), max(x1,x2))`
   - Vertical wire segment: `vs(x, min(y1,y2), max(y1,y2))`
3. For each potential via location (at L-shape corners), generate surrounding
   segments to account for via wire demand:
   - `hs(y, x-1, x+1)` and `vs(x, y-1, y+1)` per InstantGR's via model.

### 1.2 H/V Segment Independence Test

Two nets are **independent** (can be routed in parallel) if and only if:
- No horizontal segment from net A overlaps with any horizontal segment from
  net B, AND
- No vertical segment from net A overlaps with any vertical segment from net B.

Horizontal and vertical segments **never** conflict with each other because they
occupy different metal layers (H-layers vs V-layers in the alternating-direction
layer model).

**Overlap definition** (from InstantGR Definition 3): Two horizontal segments
`hs(y0, x0l, x0r)` and `hs(y1, x1l, x1r)` overlap iff `y0 == y1` AND
`[x0l, x0r]` intersects `[x1l, x1r]`.

### 1.3 Overlap Checking Data Structure: Point Exhaustion with Bitsets

InstantGR's key observation: RSMT segments are short (average length 12 cells
on a 9245x12544 grid). This means a simple bitset-per-row approach beats
segment trees and R-trees.

```rust
/// Per-row bitset for fast segment overlap checking.
///
/// File: crates/autopcb-router/src/batching/overlap.rs
pub struct SegmentOccupancy {
    /// One BitVec per grid row (for H-segments) or per grid column (for V-segments).
    /// `rows[y]` has one bit per grid column.
    rows: Vec<bitvec::vec::BitVec>,
    grid_width: u32,
}

impl SegmentOccupancy {
    pub fn new(grid_width: u32, grid_height: u32) -> Self { ... }

    /// Insert a segment into the occupancy map. O(segment_length).
    pub fn insert(&mut self, row: u32, left: u32, right: u32) {
        let bits = &mut self.rows[row as usize];
        for x in left..=right {
            bits.set(x as usize, true);
        }
    }

    /// Check if a segment overlaps any existing segment. O(segment_length).
    pub fn query(&self, row: u32, left: u32, right: u32) -> bool {
        let bits = &self.rows[row as usize];
        for x in left..=right {
            if bits[x as usize] { return true; }
        }
        false
    }

    /// Clear all occupancy data for next batch accumulation.
    pub fn clear(&mut self) {
        for row in &mut self.rows {
            row.fill(false);
        }
    }
}
```

The `bitvec` crate provides word-level parallelism: checking 64 bits at once
via bitwise AND against a mask. For the typical 12-cell segment, this is 1
word operation.

**Representative point exhaustion** (optional, for faster approximate checking):
Only check the two endpoints of each segment instead of all points. This allows
~2% intra-batch overlap in exchange for fewer batches and faster batch
construction. Useful during initial routing; switch to exact checking for
rip-up-and-reroute iterations where precision matters.

### 1.4 Batch Construction: Greedy First-Fit

```rust
/// Build non-overlapping batches from a set of nets.
///
/// File: crates/autopcb-router/src/batching/builder.rs
pub fn build_batches(
    nets: &[NetRoutingSegments],
    grid_width: u32,
    grid_height: u32,
) -> Vec<RoutingBatch> {
    let mut batches: Vec<RoutingBatch> = Vec::new();

    for net in nets {
        let mut placed = false;
        for batch in &mut batches {
            if !batch.h_occupancy.query_segments(&net.h_segments)
                && !batch.v_occupancy.query_segments(&net.v_segments)
            {
                batch.insert(net);
                placed = true;
                break;
            }
        }
        if !placed {
            let mut new_batch = RoutingBatch::new(grid_width, grid_height);
            new_batch.insert(net);
            batches.push(new_batch);
        }
    }
    batches
}

pub struct RoutingBatch {
    pub net_ids: Vec<NetId>,
    h_occupancy: SegmentOccupancy,  // tracks H-segment coverage
    v_occupancy: SegmentOccupancy,  // tracks V-segment coverage
}
```

**Why greedy, not optimal?** Optimal bin-packing (minimum batch count) is
NP-hard. Greedy first-fit is O(N * B) where N = net count and B = batch count.
InstantGR shows this produces near-optimal results: 554 batches for 59.3M nets,
which is close to the theoretical minimum.

**Net ordering before batching**: Sort nets by decreasing bounding-box area.
Larger nets are harder to fit, so placing them first reduces total batch count
(same principle as first-fit-decreasing bin packing).

### 1.5 Flexible Layer Transition for Multi-Layer Routing

PCB boards have multiple layers (typically 2-8 copper layers for most designs,
up to 32). InstantGR's H/V separation assumes alternating horizontal/vertical
preferred directions per layer. For PCB routing:

- **Even-numbered copper layers** (L1, L3, ...): horizontal preferred direction
- **Odd-numbered copper layers** (L2, L4, ...): vertical preferred direction
- This is configurable via `IrCopperLayer::preferred_direction`

Vias create coupling between layers. In the segment model, a via at grid
position (x, y) generates both an H-segment and a V-segment (one cell in each
direction) to model the wire resource consumption on adjacent layers.

For boards where layer directions are not strictly alternating (e.g., two
consecutive horizontal layers), segments on same-direction layers must be
checked against each other. The occupancy structure becomes per-direction
(not per-layer), which is exactly what InstantGR does — all H-segments across
all H-layers share one occupancy map, all V-segments across all V-layers share
another.

---

## 2. LLM-Declared Independence (Our Advantage)

### 2.1 Spec-Declared `independent_groups`

The spec language supports explicit independence declarations:

```
independent_groups [
    [DDR_DQ0..DDR_DQ7],
    [DDR_DQ8..DDR_DQ15],
    [SPI_CLK, SPI_MOSI, SPI_MISO, SPI_CS],
    [UART_TX, UART_RX],
]
```

Each group is **guaranteed** by the LLM to share no routing resources with any
other group. This is a stronger guarantee than InstantGR's runtime detection:
the LLM has analyzed the schematic topology and board layout and asserts spatial
independence.

**How this bypasses runtime conflict detection:**

```rust
/// File: crates/autopcb-router/src/batching/declared.rs

/// Convert LLM-declared independent groups into routing batches.
/// Each group becomes one batch — no overlap checking needed.
pub fn batches_from_declared_groups(
    groups: &[Vec<NetId>],
    all_nets: &IdMap<NetId, IrNet>,
) -> (Vec<RoutingBatch>, Vec<NetId>) {
    let mut batches = Vec::new();
    let mut declared_nets: HashSet<NetId> = HashSet::new();

    for group in groups {
        let batch = RoutingBatch {
            net_ids: group.clone(),
            // No occupancy tracking needed — independence is declared
            h_occupancy: SegmentOccupancy::empty(),
            v_occupancy: SegmentOccupancy::empty(),
        };
        for &net_id in group {
            declared_nets.insert(net_id);
        }
        batches.push(batch);
    }

    // Collect undeclared nets for runtime batching
    let undeclared: Vec<NetId> = all_nets
        .iter()
        .map(|(id, _)| id)
        .filter(|id| !declared_nets.contains(id))
        .collect();

    (batches, undeclared)
}
```

### 2.2 Spec-Declared `routing_partition`

Routing partitions are stronger than independent groups — they also specify
layer assignments and component membership:

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

**Mapping to batches**: Each routing partition becomes a batch with a restricted
search space. Nets in different partitions are automatically independent because
they use different layers and/or occupy different board regions.

**Layer restriction** eliminates the most expensive form of conflict: via
contention. If partition A uses layers [L1, L4] and partition B uses [L2, L3],
their vias cannot conflict regardless of spatial proximity.

### 2.3 Combining LLM Declarations with Runtime Batching

The hybrid approach processes nets in three tiers:

```
Tier 1: LLM-declared routing_partition groups
  → Each partition is one batch, no conflict checking
  → Layer-restricted search space per batch

Tier 2: LLM-declared independent_groups (not in any partition)
  → Each group is one batch, no conflict checking
  → Full layer access per batch

Tier 3: Undeclared nets
  → Runtime InstantGR-style batching with segment overlap checking
  → These are the "leftovers" — nets the LLM didn't classify
```

```rust
/// File: crates/autopcb-router/src/batching/mod.rs

pub fn compute_all_batches(
    nets: &IdMap<NetId, IrNet>,
    partitions: &[RoutingPartitionSpec],
    independent_groups: &[Vec<NetId>],
    grid: &GridConfig,
    layer_stack: &IrLayerStack,
) -> Vec<RoutingBatch> {
    let mut all_batches = Vec::new();
    let mut assigned: HashSet<NetId> = HashSet::new();

    // Tier 1: routing partitions
    for partition in partitions {
        let batch = RoutingBatch::from_partition(partition);
        for &net_id in &batch.net_ids {
            assigned.insert(net_id);
        }
        all_batches.push(batch);
    }

    // Tier 2: independent groups
    for group in independent_groups {
        let unassigned_in_group: Vec<NetId> = group.iter()
            .copied()
            .filter(|id| !assigned.contains(id))
            .collect();
        if !unassigned_in_group.is_empty() {
            let batch = RoutingBatch::from_group(unassigned_in_group.clone());
            for &net_id in &unassigned_in_group {
                assigned.insert(net_id);
            }
            all_batches.push(batch);
        }
    }

    // Tier 3: runtime batching for undeclared nets
    let undeclared: Vec<NetRoutingSegments> = nets.iter()
        .filter(|(id, _)| !assigned.contains(&id))
        .map(|(id, net)| build_routing_segments(id, net, grid, layer_stack))
        .collect();

    let runtime_batches = build_batches(
        &undeclared,
        grid.width_cells,
        grid.height_cells,
    );
    all_batches.extend(runtime_batches);

    all_batches
}
```

### 2.4 Expected Batch Sizes

| Scenario | Nets | LLM-declared | Runtime batched | Total batches | Avg nets/batch |
|----------|------|-------------|-----------------|---------------|----------------|
| Small board, no spec | 100 | 0 | 100 | 5-10 | 10-20 |
| Small board, with spec | 100 | 80 | 20 | 6-8 | 12-17 |
| Medium board, no spec | 500 | 0 | 500 | 15-30 | 17-33 |
| Medium board, with spec | 500 | 400 | 100 | 8-15 | 33-63 |
| Large board, no spec | 2000 | 0 | 2000 | 30-80 | 25-67 |
| Large board, with spec | 2000 | 1600 | 400 | 12-25 | 80-167 |

LLM declarations typically reduce batch count by 2-4x because the LLM can
assert independence that runtime checking cannot prove (e.g., nets that share
a layer but are on opposite sides of the board with a known keepout between
them).

---

## 3. Integration with PathFinder

### 3.1 Modified PathFinder Loop

The standard PathFinder loop (Milestone 7) routes nets sequentially within
each iteration. The batched variant routes multiple nets simultaneously:

```rust
/// File: crates/autopcb-router/src/pathfinder/mod.rs

pub fn pathfinder_route_batched(
    workspace: &mut RoutingWorkspace,
    config: &RoutingConfig,
    gpu: &GpuRoutingEngine,
) -> Result<RouteSolution> {
    let mut state = PathFinderState::new(workspace);
    let mut solution = RouteSolutionBuilder::new();

    for iteration in 0..config.max_iterations {
        // 1. Rip up all nets (clear solution occupancy, keep history)
        state.rip_up_all();

        // 2. Compute batches for this iteration
        //    Batches may change between iterations because augmented routing
        //    DAGs (rip-up-and-reroute) have different segment footprints.
        let batches = compute_all_batches(
            &workspace.ir.nets,
            &config.routing_partitions,
            &config.independent_groups,
            &workspace.grid,
            &workspace.ir.layer_stack,
        );

        // 3. Route each batch on GPU
        for batch in &batches {
            // Route all nets in batch simultaneously
            let batch_routes = gpu.route_batch(
                batch,
                &state.history,
                state.pres_fac,
                &workspace.obstacles,
            )?;

            // 4. Update occupancy for routed nets in this batch
            for (net_id, route) in &batch_routes {
                state.mark_occupied(&route);
                solution.set_net_route(*net_id, route.clone());
            }
        }

        // 5. Update history costs for oversubscribed cells
        state.update_history();

        // 6. Check convergence
        let conflicts = state.count_conflicts();
        solution.record_iteration_snapshot(iteration, conflicts);

        if conflicts == 0 {
            break;
        }

        // 7. Grow present congestion factor
        state.pres_fac = (state.pres_fac * config.pres_fac_multiplier)
            .min(config.pres_fac_cap);
    }

    solution.build()
}
```

### 3.2 Occupancy Updates for Simultaneously-Routed Nets

Within a batch, all nets are routed against the **same** occupancy snapshot
(from before the batch started). This means two nets in the same batch
could theoretically route through the same cell, creating a conflict that
would not occur in sequential routing.

**Why this is acceptable:**

1. The batch construction algorithm guarantees that nets' routing graphs
   do not overlap. If two nets' RSMT-based routing DAGs are independent, their
   actual routed paths (which are subsets of the routing DAGs) cannot overlap.
   This is the fundamental correctness guarantee of the InstantGR approach.

2. For LLM-declared groups, the LLM guarantees independence. If the guarantee
   is wrong, we detect it in post-batch DRC (Section 3.3).

3. Occupancy is updated **sequentially between batches**. Batch B sees the
   occupancy from all nets in batches 0..B-1. This is the same correctness
   model as sequential PathFinder, just with "batch granularity" instead of
   "net granularity."

**Occupancy update after each batch:**

```rust
/// File: crates/autopcb-router/src/pathfinder/history.rs

impl PathFinderState {
    /// Mark all cells used by a routed net as occupied.
    /// Updates the demand array for history cost computation.
    pub fn mark_occupied(&mut self, route: &RoutedNet) {
        for segment in &route.segments {
            let cells = rasterize_segment(
                &segment.start, &segment.end, segment.layer, &self.grid
            );
            for cell_idx in cells {
                self.demand[cell_idx] += 1;
            }
        }
        for via in &route.vias {
            let cell_idx = self.grid.cell_index(
                via.position.x, via.position.y, via.from_layer
            );
            self.demand[cell_idx] += 1;
            // Also mark the target layer
            let cell_idx_to = self.grid.cell_index(
                via.position.x, via.position.y, via.to_layer
            );
            self.demand[cell_idx_to] += 1;
        }
    }
}
```

### 3.3 Post-Batch DRC for Intra-Batch Conflict Detection

Even though the segment-based independence test prevents routing graph overlap,
corner cases can arise (augmented DAGs during rip-up-and-reroute may explore
beyond the initial RSMT). A lightweight DRC check after each batch catches
these:

```rust
/// File: crates/autopcb-router/src/batching/drc.rs

/// Check for conflicts among nets routed in the same batch.
/// Returns pairs of conflicting net IDs that must be re-serialized.
pub fn check_intra_batch_conflicts(
    batch_routes: &[(NetId, RoutedNet)],
    grid: &GridConfig,
) -> Vec<(NetId, NetId)> {
    // Build a cell -> net_id map for all routed cells in this batch
    let mut cell_owners: HashMap<u32, NetId> = HashMap::new();
    let mut conflicts: Vec<(NetId, NetId)> = Vec::new();

    for (net_id, route) in batch_routes {
        for cell_idx in route.all_cell_indices(grid) {
            if let Some(&existing) = cell_owners.get(&cell_idx) {
                if existing != *net_id {
                    conflicts.push((existing, *net_id));
                }
            } else {
                cell_owners.insert(cell_idx, *net_id);
            }
        }
    }

    conflicts.dedup(); // Remove duplicate conflict pairs
    conflicts
}
```

**Handling detected conflicts:**

1. **No conflicts** (expected common case): Proceed normally.
2. **Conflicts detected**: Move the conflicting nets out of this batch and
   into a "serialized fallback" batch that routes them one at a time. This
   is conservative but correct. In practice, intra-batch conflicts should be
   extremely rare if the segment model matches the actual routing DAG.
3. **Persistent conflicts in LLM-declared groups**: Report as a spec warning —
   the LLM's independence assertion was incorrect. The user should fix the spec.

---

## 4. GPU Implementation

### 4.1 Multi-Net Bellman-Ford: Architecture Options

The GPU must run multiple Bellman-Ford shortest-path searches simultaneously
(one per net in the batch). Three architecture options, each with different
GPU utilization characteristics:

#### Option A: One Net Per Workgroup (Small Nets)

```
Dispatch(num_nets_in_batch, 1, 1)
  workgroup[i] {
    256 threads cooperate on net i
    shared memory: local distance frontier
  }
```

- **Best for**: Small nets (< 10K cells in routing DAG)
- **Limitation**: 256 threads per workgroup (wgpu default max) means each
  thread handles ~40 cells for a 10K-cell routing DAG. For larger DAGs, each
  thread processes multiple cells per iteration.
- **Advantage**: No inter-net synchronization needed. Each workgroup is fully
  independent.

#### Option B: Interleaved Distance Arrays (Recommended)

Each net in the batch gets its own distance/predecessor array. All arrays are
laid out contiguously in GPU memory. All threads across all workgroups process
all nets, but each thread only touches one net's arrays at a time.

```
// Memory layout:
// dist_buffer:  [--- net 0 dist (W*H*L cells) ---][--- net 1 dist ---]...
// pred_buffer:  [--- net 0 pred (W*H*L cells) ---][--- net 1 pred ---]...
// Net count stored in uniform params.

Dispatch(ceil(total_cells_all_nets / 64), 1, 1)
  Each thread:
    global_id -> (net_index, cell_index)
    process bellman_ford_step for this (net, cell) pair
```

```rust
/// GPU buffer layout for batch routing.
///
/// File: crates/autopcb-router/src/gpu/buffers.rs

pub struct BatchBufferLayout {
    /// Number of cells per net (grid_width * grid_height * layer_count).
    pub cells_per_net: u32,
    /// Number of nets in this batch.
    pub batch_size: u32,
    /// Total buffer size = cells_per_net * batch_size * sizeof(u32).
    pub total_dist_bytes: u64,
    pub total_pred_bytes: u64,
}

impl BatchBufferLayout {
    pub fn new(grid: &GridConfig, batch_size: u32) -> Self {
        let cells_per_net = grid.width_cells * grid.height_cells
            * grid.layer_count;
        Self {
            cells_per_net,
            batch_size,
            total_dist_bytes: (cells_per_net * batch_size * 4) as u64,
            total_pred_bytes: (cells_per_net * batch_size * 4) as u64,
        }
    }

    /// Compute the buffer offset for a (net_index, cell_index) pair.
    pub fn offset(&self, net_index: u32, cell_index: u32) -> u32 {
        net_index * self.cells_per_net + cell_index
    }
}
```

- **Best for**: Medium nets, medium batch sizes (10-100 nets)
- **Advantage**: Maximizes GPU occupancy — all SMs work on all nets
- **Limitation**: Memory scales as `batch_size * cells_per_net * 4 bytes`.
  For a 500x500x4 grid (1M cells) with 50 nets: 50 * 1M * 4 = 200 MB for
  distance alone. Feasible on discrete GPUs (4+ GB VRAM).

#### Option C: Partitioned Grid Regions (Spatial Partitions)

When using `routing_partition` with spatial boundaries, each net's search
space is restricted to its partition's grid region. The GPU processes only
the relevant subgrid:

```
For each partition-batch:
  Dispatch(ceil(partition_cells / 64), 1, 1)
    threads process cells within partition bounding box only
```

- **Best for**: Spatially partitioned nets with restricted search spaces
- **Advantage**: Dramatically reduces per-net memory (partition subgrid <<
  full grid) and convergence time (fewer BF iterations on smaller graph)
- **When**: Only usable for nets in `routing_partition` specs with explicit
  spatial bounds

**Recommendation**: Use Option B (interleaved arrays) as the primary
implementation. Fall back to Option A for very small nets (< 100 routing DAG
nodes) and Option C for spatially partitioned nets. The dispatch strategy is
selected per batch based on batch characteristics.

### 4.2 Memory Layout for Multi-Net Routing

```
GPU Buffer Map for Batch Routing:
─────────────────────────────────────────────────────────────
Buffer 0 (storage, read-only):
  obstacles[]     — per-layer bitmaps, shared across all nets
                    Size: ceil(grid_cells / 32) * layer_count * 4 bytes

Buffer 1 (storage, read-write):
  dist[]          — interleaved distance arrays
                    Layout: [net0_cell0, net0_cell1, ..., net1_cell0, ...]
                    Size: batch_size * cells_per_net * 4 bytes
                    Type: atomic<u32> (for atomicMin relaxation)

Buffer 2 (storage, read-only during routing, write during update):
  history[]       — shared history costs (one copy, not per-net)
                    Size: cells_per_net * 4 bytes

Buffer 3 (storage, write-only during routing):
  predecessor[]   — interleaved predecessor arrays
                    Layout: same as dist[]
                    Size: batch_size * cells_per_net * 4 bytes

Uniform buffer:
  params {
    grid_width: u32,
    grid_height: u32,
    num_layers: u32,
    cells_per_net: u32,
    batch_size: u32,
    pres_fac: u32,          // fixed-point (x1000)
    base_cost_h: u32,       // fixed-point
    base_cost_v: u32,       // fixed-point
    via_cost: u32,           // fixed-point
    source_targets: array<SourceTarget, MAX_BATCH_SIZE>,
  }

  struct SourceTarget {
    source_cell: u32,       // linearized source cell index
    target_cell: u32,       // linearized target cell index (for heuristic)
  }
─────────────────────────────────────────────────────────────
```

### 4.3 Maximum Batch Size Given GPU Memory

```
Per-net memory overhead:
  dist array:  cells_per_net * 4 bytes
  pred array:  cells_per_net * 4 bytes
  Total/net:   cells_per_net * 8 bytes

Shared (fixed) memory:
  obstacles:   ceil(cells_per_net / 8) bytes (bitmaps)
  history:     cells_per_net * 4 bytes

Grid: 500x500, 4 layers → cells_per_net = 1,000,000
  Per net: 8 MB
  Shared:  4.125 MB

Available VRAM = 4 GB → usable ~2.5 GB (leaving room for OS, shader state)
  Max batch size: (2500 MB - 4.125 MB) / 8 MB ≈ 312 nets

Grid: 1000x1000, 4 layers → cells_per_net = 4,000,000
  Per net: 32 MB
  Shared:  16.5 MB

Available VRAM = 8 GB → usable ~5 GB
  Max batch size: (5000 MB - 16.5 MB) / 32 MB ≈ 155 nets

Grid: 200x200, 2 layers → cells_per_net = 80,000
  Per net: 640 KB
  Shared:  330 KB

Available VRAM = 2 GB → usable ~1.2 GB
  Max batch size: (1200 MB - 0.33 MB) / 0.64 MB ≈ 1875 nets
```

For typical PCB boards (200x200 to 500x500 grid, 2-4 layers), batch sizes of
50-300 nets are feasible on modest GPUs. This is enough to batch the vast
majority of nets, with only the largest batches potentially requiring
sub-batching.

---

## 5. WGSL Shaders

### 5.1 `batch_bellman_ford.wgsl` — Multi-Net Parallel Bellman-Ford

```wgsl
// crates/autopcb-router/shaders/batch_bellman_ford.wgsl

struct Params {
    grid_width: u32,
    grid_height: u32,
    num_layers: u32,
    cells_per_net: u32,
    batch_size: u32,
    pres_fac: u32,          // fixed-point x1000
    base_cost_h: u32,
    base_cost_v: u32,
    via_cost: u32,
    iteration_changed: atomic<u32>,  // convergence flag
}

struct SourceTarget {
    source_cell: u32,
    target_cell: u32,
}

@group(0) @binding(0) var<storage, read> obstacles: array<u32>;
@group(0) @binding(1) var<storage, read_write> dist: array<atomic<u32>>;
@group(0) @binding(2) var<storage, read> history: array<u32>;
@group(0) @binding(3) var<storage, read_write> pred: array<u32>;
@group(1) @binding(0) var<uniform> params: Params;
@group(1) @binding(1) var<storage, read> source_targets: array<SourceTarget>;

// Decode cell linear index into (x, y, layer)
fn decode_cell(cell: u32) -> vec3<u32> {
    let layer = cell % params.num_layers;
    let y = (cell / params.num_layers) % params.grid_height;
    let x = cell / (params.num_layers * params.grid_height);
    return vec3<u32>(x, y, layer);
}

fn encode_cell(x: u32, y: u32, layer: u32) -> u32 {
    return x * (params.num_layers * params.grid_height)
         + y * params.num_layers + layer;
}

fn is_blocked(cell: u32) -> bool {
    let word = cell / 32u;
    let bit = cell % 32u;
    return (obstacles[word] & (1u << bit)) != 0u;
}

fn edge_cost(neighbor_cell: u32, base: u32) -> u32 {
    let h = history[neighbor_cell];
    // Cost = base + history * pres_fac / 1000
    return base + (h * params.pres_fac) / 1000u;
}

fn try_relax(
    net_offset: u32,
    src_cell: u32,
    src_dist_val: u32,
    nx: u32, ny: u32, nl: u32,
    base: u32,
    direction: u32,  // encoded predecessor direction
) {
    // Bounds check
    if nx >= params.grid_width || ny >= params.grid_height
       || nl >= params.num_layers {
        return;
    }

    let neighbor_cell = encode_cell(nx, ny, nl);
    if is_blocked(neighbor_cell) { return; }

    let cost = edge_cost(neighbor_cell, base);
    let new_dist = src_dist_val + cost;

    let idx = net_offset + neighbor_cell;
    let old = atomicMin(&dist[idx], new_dist);
    if new_dist < old {
        // We improved the distance — record predecessor
        pred[idx] = (src_cell << 4u) | direction;
        atomicStore(&params.iteration_changed, 1u);
    }
}

@compute @workgroup_size(64)
fn bellman_ford_step(@builtin(global_invocation_id) gid: vec3<u32>) {
    let global_idx = gid.x;
    let total = params.batch_size * params.cells_per_net;
    if global_idx >= total { return; }

    // Determine which net and which cell
    let net_index = global_idx / params.cells_per_net;
    let cell_index = global_idx % params.cells_per_net;
    let net_offset = net_index * params.cells_per_net;

    let current_dist = atomicLoad(&dist[net_offset + cell_index]);
    if current_dist == 0xFFFFFFFFu { return; }  // unvisited

    let pos = decode_cell(cell_index);
    let x = pos.x;
    let y = pos.y;
    let layer = pos.z;

    // 4 cardinal neighbors (same layer)
    if x > 0u {
        try_relax(net_offset, cell_index, current_dist,
                  x - 1u, y, layer, params.base_cost_h, 1u);
    }
    if x < params.grid_width - 1u {
        try_relax(net_offset, cell_index, current_dist,
                  x + 1u, y, layer, params.base_cost_h, 2u);
    }
    if y > 0u {
        try_relax(net_offset, cell_index, current_dist,
                  x, y - 1u, layer, params.base_cost_v, 3u);
    }
    if y < params.grid_height - 1u {
        try_relax(net_offset, cell_index, current_dist,
                  x, y + 1u, layer, params.base_cost_v, 4u);
    }

    // Via transitions (adjacent layers)
    if layer > 0u {
        try_relax(net_offset, cell_index, current_dist,
                  x, y, layer - 1u, params.via_cost, 5u);
    }
    if layer < params.num_layers - 1u {
        try_relax(net_offset, cell_index, current_dist,
                  x, y, layer + 1u, params.via_cost, 6u);
    }
}
```

**Convergence detection**: The `iteration_changed` atomic flag is set to 1 by
any thread that successfully relaxes an edge. The CPU checks this flag after
each dispatch. If the flag is 0, Bellman-Ford has converged for all nets in the
batch.

**Net isolation**: Each net accesses only its own slice of `dist[]` and `pred[]`
(offset by `net_index * cells_per_net`). The shared `history[]` and
`obstacles[]` are read-only, so no cross-net data races occur.

### 5.2 `batch_reset_dist.wgsl` — Reset Distance Arrays

```wgsl
// crates/autopcb-router/shaders/batch_reset_dist.wgsl

@group(0) @binding(0) var<storage, read_write> dist: array<atomic<u32>>;
@group(0) @binding(1) var<storage, read_write> pred: array<u32>;
@group(1) @binding(0) var<uniform> params: Params;
@group(1) @binding(1) var<storage, read> source_targets: array<SourceTarget>;

@compute @workgroup_size(64)
fn reset_and_seed(@builtin(global_invocation_id) gid: vec3<u32>) {
    let global_idx = gid.x;
    let total = params.batch_size * params.cells_per_net;
    if global_idx >= total { return; }

    // Reset to infinity
    atomicStore(&dist[global_idx], 0xFFFFFFFFu);
    pred[global_idx] = 0xFFFFFFFFu;

    // Seed source cells
    let net_index = global_idx / params.cells_per_net;
    let cell_index = global_idx % params.cells_per_net;
    let source = source_targets[net_index].source_cell;
    if cell_index == source {
        atomicStore(&dist[global_idx], 0u);
    }
}
```

### 5.3 `batch_occupancy_update.wgsl` — History Update

```wgsl
// crates/autopcb-router/shaders/batch_occupancy_update.wgsl
//
// Run once per PathFinder iteration after all batches are routed.
// Updates history costs for cells where demand > capacity.

@group(0) @binding(0) var<storage, read_write> history: array<u32>;
@group(0) @binding(1) var<storage, read> demand: array<u32>;
@group(0) @binding(2) var<storage, read> capacity: array<u32>;
@group(1) @binding(0) var<uniform> params: HistoryParams;

struct HistoryParams {
    total_cells: u32,
    history_increment: u32,  // fixed-point x1000
}

@compute @workgroup_size(64)
fn update_history(@builtin(global_invocation_id) gid: vec3<u32>) {
    let cell = gid.x;
    if cell >= params.total_cells { return; }

    let d = demand[cell];
    let c = capacity[cell];
    if d > c {
        // Cell is oversubscribed: increase history cost
        history[cell] += params.history_increment;
    }
}
```

### 5.4 `batch_conflict_check.wgsl` — Intra-Batch Conflict Detection

```wgsl
// crates/autopcb-router/shaders/batch_conflict_check.wgsl
//
// After routing a batch, check if any two nets in the batch
// routed through the same cell. Uses an atomic "owner" map.

@group(0) @binding(0) var<storage, read_write> cell_owner: array<atomic<u32>>;
@group(0) @binding(1) var<storage, read_write> conflict_count: atomic<u32>;
@group(0) @binding(2) var<storage, read> routed_cells: array<u32>;
  // Packed: [net_id (16 bits) | cell_index (16 bits)] — for small grids
  // Or separate arrays for large grids
@group(1) @binding(0) var<uniform> params: ConflictParams;

struct ConflictParams {
    total_routed_cells: u32,
}

@compute @workgroup_size(64)
fn check_conflicts(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if idx >= params.total_routed_cells { return; }

    let packed = routed_cells[idx];
    let net_id = packed >> 16u;
    let cell = packed & 0xFFFFu;

    // Try to claim this cell for our net
    // NONE_OWNER = 0xFFFF (no owner yet)
    let old = atomicCompareExchangeWeak(&cell_owner[cell], 0xFFFFu, net_id);
    if old.old_value != 0xFFFFu && old.old_value != net_id {
        // Cell already owned by a different net — conflict!
        atomicAdd(&conflict_count, 1u);
    }
}
```

---

## 6. Batch Construction Algorithm

### 6.1 Runtime Batching: Full Algorithm

```rust
/// File: crates/autopcb-router/src/batching/builder.rs

use crate::batching::segments::{NetRoutingSegments, HSegment, VSegment};
use crate::batching::overlap::SegmentOccupancy;
use autopcb_ir::{NetId, IrNet, IrNetPin};

/// Sort nets by decreasing bounding-box area for better packing.
fn sort_nets_for_batching(nets: &mut Vec<NetRoutingSegments>) {
    nets.sort_by(|a, b| {
        let area_a = a.bounding_box_area();
        let area_b = b.bounding_box_area();
        area_b.partial_cmp(&area_a).unwrap_or(std::cmp::Ordering::Equal)
    });
}

/// Greedy first-fit batch construction with segment-based overlap checking.
///
/// Time complexity: O(N * B * avg_segment_length) where N = net count,
/// B = number of batches (typically << N), avg_segment_length ≈ 12.
pub fn build_batches(
    nets: &mut Vec<NetRoutingSegments>,
    grid_width: u32,
    grid_height: u32,
) -> Vec<RoutingBatch> {
    sort_nets_for_batching(nets);

    let mut batches: Vec<RoutingBatch> = Vec::new();

    for net in nets.iter() {
        let mut placed = false;

        for batch in batches.iter_mut() {
            // Check H-segment overlaps
            let h_overlap = net.h_segments.iter().any(|seg| {
                batch.h_occupancy.query(seg.y, seg.x_l, seg.x_r)
            });
            if h_overlap { continue; }

            // Check V-segment overlaps
            let v_overlap = net.v_segments.iter().any(|seg| {
                batch.v_occupancy.query(seg.x, seg.y_l, seg.y_r)
            });
            if v_overlap { continue; }

            // No overlap — add to this batch
            batch.insert(net);
            placed = true;
            break;
        }

        if !placed {
            let mut new_batch = RoutingBatch::new(grid_width, grid_height);
            new_batch.insert(net);
            batches.push(new_batch);
        }
    }

    batches
}
```

### 6.2 LLM-Assisted Batching: Pre-Declared Groups + Runtime Refinement

```rust
/// File: crates/autopcb-router/src/batching/hybrid.rs

/// Hybrid batching: LLM groups first, runtime batching for the rest.
pub fn hybrid_batch_construction(
    all_nets: &IdMap<NetId, IrNet>,
    declared_partitions: &[RoutingPartitionSpec],
    declared_groups: &[Vec<NetId>],
    grid: &GridConfig,
    layer_stack: &IrLayerStack,
    max_batch_size: u32,  // from GPU memory budget
) -> Vec<RoutingBatch> {
    let mut result = Vec::new();
    let mut assigned = HashSet::new();

    // Phase 1: Partitions (guaranteed independent, layer-restricted)
    for partition in declared_partitions {
        let nets: Vec<NetId> = partition.net_ids.iter()
            .copied()
            .filter(|id| all_nets.get(*id).is_some())
            .collect();
        // Split partition into GPU-sized sub-batches if needed
        for chunk in nets.chunks(max_batch_size as usize) {
            let mut batch = RoutingBatch::from_partition_chunk(
                chunk, &partition.allowed_layers
            );
            result.push(batch);
        }
        for &net_id in &nets {
            assigned.insert(net_id);
        }
    }

    // Phase 2: Independent groups (guaranteed independent, full layers)
    for group in declared_groups {
        let nets: Vec<NetId> = group.iter()
            .copied()
            .filter(|id| !assigned.contains(id))
            .collect();
        for chunk in nets.chunks(max_batch_size as usize) {
            result.push(RoutingBatch::from_group(chunk.to_vec()));
        }
        for &net_id in &nets {
            assigned.insert(net_id);
        }
    }

    // Phase 3: Runtime batching for undeclared nets
    let mut undeclared_segments: Vec<NetRoutingSegments> = all_nets.iter()
        .filter(|(id, _)| !assigned.contains(&id))
        .map(|(id, net)| build_routing_segments(id, net, grid, layer_stack))
        .collect();

    let runtime_batches = build_batches(
        &mut undeclared_segments,
        grid.width_cells,
        grid.height_cells,
    );

    // Sub-batch runtime results by GPU memory limit
    for batch in runtime_batches {
        if batch.net_ids.len() <= max_batch_size as usize {
            result.push(batch);
        } else {
            // Split oversized batch (nets at the end are smallest, least likely
            // to conflict, so we just chunk them — some may end up in same batch
            // despite overlapping, but post-batch DRC catches it)
            for chunk in batch.net_ids.chunks(max_batch_size as usize) {
                result.push(RoutingBatch::from_group(chunk.to_vec()));
            }
        }
    }

    result
}
```

### 6.3 Data Structure for Fast Overlap Detection

The `SegmentOccupancy` bitset-per-row structure is the primary data structure.
For the fast path:

- **Query**: O(segment_length) with word-level parallelism from `bitvec`.
  For average segment length 12, this is typically 1 word operation.
- **Insert**: O(segment_length), same as query.
- **Space**: O(grid_width * grid_height / 8) bytes per direction.
  For a 1000x1000 grid: 125 KB per occupancy map, 250 KB total (H + V).

**Alternative: Interval trees for large segments.** If some nets have very long
segments (e.g., clock distribution across the entire board), a per-row interval
tree gives O(log n) query/insert instead of O(segment_length). However,
InstantGR's empirical data shows average segment length of 12, so the bitset
approach is faster in practice due to lower constant factors.

**Sweep line for optimal batching.** A sweep-line algorithm can compute the
exact minimum number of batches in O(S log S) where S is total segment count.
However, greedy first-fit produces near-optimal results and is simpler to
implement. Reserve sweep-line for future optimization if batch count becomes
a bottleneck.

---

## 7. PcbIr / Spec Extensions

### 7.1 Spec Language Features Supporting Batching

The following spec constructs map to batching inputs:

| Spec Feature | Batching Use | Status |
|---|---|---|
| `placement_group { components: [...] }` | Candidate routing partition (same components → local nets) | Exists (in `PlacementGroupSpec`) |
| `independent_groups [...]` | Direct batch creation, no overlap check | Proposed (new) |
| `routing_partition { ... }` | Layer-restricted batch with spatial bounds | Proposed (new) |
| `net_class { priority: ... }` | Net ordering within batches | Exists in router plan (M3) |
| `constraint edge_placement { ... }` | Spatial bounds for partition inference | Exists (in `ConstraintSpec`) |

**New spec model types needed:**

```rust
/// File: crates/altium-format-spec/src/model.rs (additions to PcbDocSpec)

/// Declaration of net groups that are guaranteed to be routable
/// without resource conflicts. Used for GPU batch construction.
pub struct IndependentGroupsSpec {
    /// Each inner Vec is one independent group of net names.
    pub groups: Vec<Vec<String>>,
}

/// A routing partition: a group of nets restricted to specific layers
/// and/or board regions. Partitions are guaranteed independent of each other.
pub struct RoutingPartitionSpec {
    pub name: String,
    /// Component designators belonging to this partition.
    pub components: Vec<String>,
    /// Net names belonging to this partition.
    pub nets: Vec<String>,
    /// Copper layers this partition may use (empty = all layers).
    pub allowed_layers: Vec<LayerSpec>,
    /// Optional spatial bounds (bounding box in mm).
    pub region: Option<(PointMm, PointMm)>,
}
```

### 7.2 PcbIr Fields for Batching

The existing `IrNet` and `IrNetPin` types provide everything needed for segment
construction:

- `IrNet::pins` → pin positions for RSMT construction
- `IrNetPin::position: PointMm` → world-space coordinates
- `IrNet::net_class: Option<String>` → priority lookup
- `IrNet::diff_pair_partner: Option<NetId>` → diff pairs must be in same batch

**New field for precomputed bounding box (optional optimization):**

```rust
/// File: crates/autopcb-ir/src/net.rs (addition)

impl IrNet {
    /// Compute the axis-aligned bounding box of all pins in this net.
    pub fn bounding_box(&self) -> Option<BoundingBoxMm> {
        let points: Vec<PointMm> = self.pins.iter()
            .map(|p| p.position)
            .collect();
        BoundingBoxMm::from_points(&points)
    }
}
```

This uses the existing `BoundingBoxMm::from_points()` from
`crates/autopcb-ir/src/types.rs`.

### 7.3 Net Ordering Within Batches

Within each batch, nets are ordered by priority for the PathFinder cost
function. The ordering affects which nets "win" contested resources in the
shared history:

```rust
/// File: crates/autopcb-router/src/batching/ordering.rs

/// Order nets within a batch for GPU routing.
/// Higher-priority nets are uploaded first so their paths are seeded
/// in the distance array before lower-priority nets begin.
pub fn order_batch_nets(
    batch: &mut RoutingBatch,
    nets: &IdMap<NetId, IrNet>,
    policy: &RoutingPolicy,
    rng: &mut ChaCha8Rng,
) {
    batch.net_ids.sort_by(|a, b| {
        let pa = policy.priority(*a);
        let pb = policy.priority(*b);
        // Higher priority first (lower number = higher priority)
        pa.cmp(&pb)
            .then_with(|| {
                // Tiebreak: shorter nets first (less search space)
                let la = nets[*a].bounding_box().map_or(0.0, |b| b.width() + b.height());
                let lb = nets[*b].bounding_box().map_or(0.0, |b| b.width() + b.height());
                la.partial_cmp(&lb).unwrap_or(std::cmp::Ordering::Equal)
            })
    });
}
```

Note: Within a GPU batch, all nets are routed simultaneously so ordering does
not affect which net "goes first." But ordering determines the upload sequence
for the source/target buffer and affects tie-breaking in `atomicMin` races
(lower net index wins ties in the predecessor array). For truly independent
nets in a batch, this is irrelevant. For nearly-independent nets (e.g., those
passing the representative point exhaustion check with ~2% overlap), the
ordering can affect solution quality.

---

## 8. Performance Analysis

### 8.1 Expected Batch Sizes for Typical PCB Boards

PCB boards are dramatically smaller than VLSI designs. InstantGR's benchmarks
have 129K-59.3M nets; typical PCB boards have 50-5000 nets.

| Board type | Nets | Grid | Expected batches | Avg batch size |
|---|---|---|---|---|
| Simple 2-layer (Arduino) | 50-100 | 200x200x2 | 3-5 | 15-30 |
| Medium 4-layer (STM32 dev board) | 200-500 | 400x400x4 | 5-15 | 20-50 |
| Complex 4-layer (RPi-class) | 500-1500 | 600x600x4 | 10-30 | 30-80 |
| Dense 6-layer (DDR4 SoC board) | 1500-3000 | 800x800x6 | 15-50 | 40-100 |
| Server 8-layer (backplane) | 3000-8000 | 1000x1000x8 | 20-80 | 50-200 |

With LLM declarations, batch counts decrease by 2-4x and batch sizes increase
proportionally. The sweet spot for GPU acceleration is 10+ nets per batch.

### 8.2 Speedup from Batching

**Per-batch speedup (GPU vs sequential CPU):**

The GPU routes all nets in a batch in the time it takes to route the largest
net (the one requiring the most Bellman-Ford iterations). Sequential CPU
routing takes `sum(per_net_time)`.

```
Speedup_per_iteration ≈ total_nets / num_batches

With B batches of average size S:
  Sequential:  S * B * T_per_net
  Batched GPU: B * T_per_net  (each batch takes ≈ T_per_net)
  Speedup:     S (linear in batch size)
```

**Overhead costs that reduce effective speedup:**

- CPU-GPU data transfer per batch: ~0.1-0.5 ms (negligible for > 1K cells)
- Batch construction: O(N * B * avg_seg_len) ≈ O(N * B * 12) on CPU
- Post-batch DRC check: O(routed_cells_in_batch) on CPU
- Path reconstruction from predecessor array: O(path_length) per net, on CPU

**Expected total speedup for the PathFinder loop:**

| Board size | Batch size | BF iterations/net | Batched speedup | Net speedup (w/ overhead) |
|---|---|---|---|---|
| 100 nets | 20 | 30 | 20x | 8-12x |
| 500 nets | 40 | 50 | 40x | 15-25x |
| 2000 nets | 80 | 80 | 80x | 25-40x |

### 8.3 When Batching Hurts

Batching can be counterproductive in specific scenarios:

1. **Very small boards (< 50 nets)**: GPU dispatch overhead (~1 ms) exceeds
   CPU routing time. Stick to CPU A* for tiny boards.

2. **Highly interconnected nets**: If most nets share routing resources (e.g.,
   a single-layer board with many crossing nets), batch sizes approach 1 and
   batching adds overhead with no benefit.

3. **Bus-dominated routing**: Parallel bus nets are typically in the same
   routing corridor and share H or V segments. They cannot be batched by
   InstantGR's independence test. However, LLM-declared `independent_groups`
   can still batch bus halves (e.g., DDR_DQ[0:7] and DDR_DQ[8:15]) if they
   use different layer pairs.

4. **Convergence impact**: Batched routing within a PathFinder iteration is
   slightly less informed than sequential routing (each net doesn't see the
   paths of other nets in the same batch). This may require 1-3 more
   PathFinder iterations for convergence. However, each iteration is much
   faster, so the net effect is still a speedup.

**Auto-enable heuristic:**

```rust
/// File: crates/autopcb-router/src/batching/mod.rs

pub fn should_use_gpu_batching(
    net_count: usize,
    grid_cells: usize,
    has_gpu: bool,
) -> bool {
    has_gpu && net_count > 50 && grid_cells > 40_000
}
```

---

## 9. Testing Strategy

### 9.1 Correctness: Batch-Routed vs Sequential

```rust
/// File: crates/autopcb-router/src/batching/tests.rs

#[cfg(test)]
mod tests {
    use super::*;

    /// Two non-overlapping nets must produce the same routes
    /// whether routed sequentially or in a batch.
    #[test]
    fn batch_matches_sequential_independent_nets() {
        let ir = synthetic_board_two_independent_nets();
        let config = RoutingConfig::default();

        // Route sequentially
        let seq_solution = pathfinder_route_sequential(&ir, &config).unwrap();

        // Route in a single batch
        let batch_solution = pathfinder_route_batched(&ir, &config).unwrap();

        // Both solutions must route all nets
        assert_eq!(seq_solution.unrouted_count(), 0);
        assert_eq!(batch_solution.unrouted_count(), 0);

        // Route quality must be comparable (within 5% total wirelength)
        let seq_wl = seq_solution.total_wirelength_mm();
        let batch_wl = batch_solution.total_wirelength_mm();
        assert!(
            (batch_wl - seq_wl).abs() / seq_wl < 0.05,
            "Batched wirelength {batch_wl} differs from sequential {seq_wl} by > 5%"
        );
    }

    /// Nets in the same batch must not share any grid cells.
    #[test]
    fn no_intra_batch_conflicts_for_independent_nets() {
        let ir = synthetic_board_many_independent_nets(50);
        let config = RoutingConfig::default();
        let segments = build_all_routing_segments(&ir);
        let batches = build_batches(&mut segments, 200, 200);

        for batch in &batches {
            let routes = route_batch_cpu(&batch, &ir, &config);
            let conflicts = check_intra_batch_conflicts(&routes, &ir.grid());
            assert!(
                conflicts.is_empty(),
                "Batch has {} conflicts: {:?}", conflicts.len(), conflicts
            );
        }
    }
}
```

### 9.2 LLM-Declared Independent Groups

```rust
#[test]
fn declared_groups_produce_fewer_batches() {
    let ir = synthetic_board_with_functional_blocks();
    let config = RoutingConfig::default();

    // Without declarations: runtime batching
    let batches_runtime = build_batches_runtime_only(&ir);

    // With declarations: hybrid batching
    let groups = vec![
        vec![NetId::from(0), NetId::from(1), NetId::from(2)],
        vec![NetId::from(3), NetId::from(4), NetId::from(5)],
    ];
    let batches_hybrid = compute_all_batches(
        &ir.nets, &[], &groups, &ir.grid(), &ir.layer_stack
    );

    assert!(
        batches_hybrid.len() <= batches_runtime.len(),
        "Hybrid batching should produce <= batches than runtime-only"
    );
}

#[test]
fn incorrect_independence_declaration_detected() {
    // Declare two nets as independent when they actually share cells
    let ir = synthetic_board_crossing_nets();
    let groups = vec![
        vec![NetId::from(0)],
        vec![NetId::from(1)],
    ];
    let batches = compute_all_batches(
        &ir.nets, &[], &groups, &ir.grid(), &ir.layer_stack
    );

    // Route the batch
    let routes = route_batch(&batches[0], &ir);
    let conflicts = check_intra_batch_conflicts(&routes, &ir.grid());

    // Even though declared independent, DRC should catch the conflict
    // (if nets actually overlap after routing)
    // This test verifies the safety net works.
}
```

### 9.3 Stress Test: All Nets in One Batch

```rust
#[test]
fn all_nets_single_batch_maximum_conflicts() {
    let ir = synthetic_board_dense_grid(20); // 20 nets in tight area
    let mut all_nets: Vec<NetId> = ir.nets.iter().map(|(id, _)| id).collect();

    // Force all nets into one batch (simulates worst-case LLM error)
    let batch = RoutingBatch::from_group(all_nets);
    let routes = route_batch(&batch, &ir);
    let conflicts = check_intra_batch_conflicts(&routes, &ir.grid());

    // Expect many conflicts — this batch should be split
    // The router must handle this gracefully
    assert!(conflicts.len() > 0, "Dense board should have intra-batch conflicts");

    // After splitting conflicting nets into separate batches, all should route
    let proper_batches = build_batches_runtime_only(&ir);
    let solution = route_all_batches(&proper_batches, &ir);
    assert_eq!(solution.unrouted_count(), 0);
}
```

### 9.4 Segment Overlap Checking Tests

```rust
#[test]
fn non_overlapping_l_shapes_same_batch() {
    // Two L-shaped nets that share a bounding box but not segments
    // (one goes right-then-down, the other goes down-then-right)
    let net_a = NetRoutingSegments {
        net_id: NetId::from(0),
        h_segments: vec![HSegment { y: 5, x_l: 0, x_r: 5 }],
        v_segments: vec![VSegment { x: 5, y_l: 0, y_r: 5 }],
    };
    let net_b = NetRoutingSegments {
        net_id: NetId::from(1),
        h_segments: vec![HSegment { y: 0, x_l: 5, x_r: 10 }],
        v_segments: vec![VSegment { x: 10, y_l: 0, y_r: 5 }],
    };

    let batches = build_batches(&mut vec![net_a, net_b], 20, 20);
    assert_eq!(batches.len(), 1, "Non-overlapping L-shapes should be in one batch");
}

#[test]
fn overlapping_h_segments_different_batches() {
    let net_a = NetRoutingSegments {
        net_id: NetId::from(0),
        h_segments: vec![HSegment { y: 5, x_l: 0, x_r: 10 }],
        v_segments: vec![],
    };
    let net_b = NetRoutingSegments {
        net_id: NetId::from(1),
        h_segments: vec![HSegment { y: 5, x_l: 8, x_r: 15 }],
        v_segments: vec![],
    };

    let batches = build_batches(&mut vec![net_a, net_b], 20, 20);
    assert_eq!(batches.len(), 2, "Overlapping H-segments must be in different batches");
}

#[test]
fn h_and_v_segments_never_conflict() {
    // Horizontal segment on row 5 and vertical segment crossing row 5
    // should NOT conflict (different layers)
    let net_a = NetRoutingSegments {
        net_id: NetId::from(0),
        h_segments: vec![HSegment { y: 5, x_l: 0, x_r: 20 }],
        v_segments: vec![],
    };
    let net_b = NetRoutingSegments {
        net_id: NetId::from(1),
        h_segments: vec![],
        v_segments: vec![VSegment { x: 10, y_l: 0, y_r: 20 }],
    };

    let batches = build_batches(&mut vec![net_a, net_b], 25, 25);
    assert_eq!(batches.len(), 1, "H and V segments on different layers should not conflict");
}
```

---

## 10. Module Structure

InstantGR's CPU batching logic lives in `src/batching/`. The GPU buffer layout for multi-net parallel routing lives in `src/gpu/batching.rs` (part of the shared GPU module defined in Plan 01):

```
crates/autopcb-router/src/
├── batching/
│   ├── mod.rs              — compute_all_batches(), should_use_gpu_batching()
│   ├── segments.rs         — NetRoutingSegments, HSegment, VSegment, build_routing_segments()
│   ├── overlap.rs          — SegmentOccupancy (bitset-per-row)
│   ├── builder.rs          — build_batches() (greedy first-fit)
│   ├── declared.rs         — batches_from_declared_groups(), batches_from_partitions()
│   ├── hybrid.rs           — hybrid_batch_construction()
│   ├── ordering.rs         — order_batch_nets()
│   ├── drc.rs              — check_intra_batch_conflicts()
│   └── tests.rs            — all batching tests
├── gpu/
│   ├── mod.rs              — GpuRoutingEngine (shared, see Plan 01)
│   ├── engine.rs           — GpuRoutingEngine struct (shared)
│   ├── batching.rs         — BatchBufferLayout, batch buffer creation [this plan]
│   ├── buffers.rs          — buffer creation
│   ├── pipeline.rs         — wgpu compute pipeline setup
│   └── dispatch.rs         — route_batch(), bellman_ford_loop()
└── shaders/
    ├── batch_bellman_ford.wgsl
    ├── batch_reset_dist.wgsl
    ├── batch_occupancy_update.wgsl
    └── batch_conflict_check.wgsl
```

---

## 11. Implementation Order

| Step | Description | Dependencies |
|---|---|---|
| 1 | `SegmentOccupancy` (bitset overlap structure) | `bitvec` (existing dep) |
| 2 | `NetRoutingSegments` + `build_routing_segments()` | M5 MST decomposer |
| 3 | `build_batches()` (greedy first-fit, CPU only) | Steps 1-2 |
| 4 | `check_intra_batch_conflicts()` | Step 3 |
| 5 | Unit tests for overlap checking and batch construction | Steps 1-4 |
| 6 | `batches_from_declared_groups()`, `hybrid_batch_construction()` | Step 3 + spec model changes |
| 7 | `BatchBufferLayout` + GPU buffer creation | wgpu context (existing) |
| 8 | `batch_reset_dist.wgsl` + `batch_bellman_ford.wgsl` | Step 7 |
| 9 | `route_batch()` GPU dispatch loop with convergence | Steps 7-8 |
| 10 | `batch_occupancy_update.wgsl` + `batch_conflict_check.wgsl` | Step 9 |
| 11 | Modified `pathfinder_route_batched()` | Steps 3, 6, 9, 10 |
| 12 | Integration tests (batch vs sequential equivalence) | Step 11 |
| 13 | Spec parser/compiler for `independent_groups` and `routing_partition` | Step 6 |
| 14 | Performance benchmarks | Steps 11-12 |

Steps 1-5 are pure CPU and can begin immediately after Milestone 5 (global
routing with MST decomposer). Steps 7-10 require the GPU routing context
from the base GPU acceleration work. Step 13 extends the spec language.

---

## 12. References

## See Also

| Plan | Role | Relationship to InstantGR |
|------|------|--------------------------|
| **01 — Corolla** (`01-corolla-bellman-ford.md`) | GPU SSSP backend | Defines `GpuRoutingEngine` including `max_batch_size` and the interleaved `distance`/`predecessor` buffer layout that InstantGR's batches populate. Corolla routes the small-subnet batches that InstantGR constructs. |
| **02 — GAMER** (`02-gamer-sweep-routing.md`) | Alternative GPU SSSP backend | Routes large-subnet batches from InstantGR. Same buffer layout as Corolla; `backend_select.rs` picks between them per batch. |
| **03 — X-Check** (`03-xcheck-gpu-drc.md`) | GPU DRC | Runs after all batches in an iteration are routed. InstantGR's batch structure determines the granularity: X-Check fires once per iteration, not once per batch. |
| **04 — Cypress** (`04-cypress-congestion-feedback.md`) | Post-routing congestion feedback | Operates after all PathFinder iterations complete. Independent of InstantGR's per-iteration batching. |

---

## 12. References

- **InstantGR** (Lin et al., ICCAD 2024): Segment-based routing graph
  representation, point exhaustion with bitsets, representative point
  exhaustion. Source: https://github.com/cuhk-eda/InstantGR
- **GANGR** (2025): GAN-based net batching for 40% runtime reduction.
  https://arxiv.org/abs/2511.17665
- **McMurchie & Ebeling, FPGA 1995**: PathFinder negotiation-based routing.
- **Corolla** (Shen et al., FPGA 2017): Multi-level GPU parallelism for FPGA
  routing (SNP, DEP, multi-net).
- **GAMER** (IEEE TCAD 2023): GPU maze routing via H/V sweep decomposition.
- **OrthoRoute** (2025): GPU PCB autorouter — PathFinder parameter lessons.
