# Spatial Partitioning for GPU-Parallel PCB Routing

Research notes on partitioning a PCB board into regions that can be routed
independently on separate GPU workgroups, in the context of a wgpu-based
PathFinder router.

---

## Table of Contents

1. [Board Partitioning Strategies](#1-board-partitioning-strategies)
2. [Boundary Routing Between Partitions](#2-boundary-routing-between-partitions)
3. [GPU Workgroup Mapping](#3-gpu-workgroup-mapping)
4. [Room-Based Routing (Altium Rooms)](#4-room-based-routing-altium-rooms)
5. [Multi-Solution Exploration Within Partitions](#5-multi-solution-exploration-within-partitions)
6. [Recommended Architecture for Our Router](#6-recommended-architecture-for-our-router)
7. [References](#7-references)

---

## 1. Board Partitioning Strategies

The goal of board partitioning is to decompose the routing problem into
sub-problems that can be solved independently (and thus in parallel). The
central tension is: **smaller partitions enable more parallelism, but
inter-partition nets create coupling that requires sequential stitching.**

### 1.1 Geometric / Spatial Decomposition

#### Quadtree Decomposition

Recursively subdivide the board bounding box into four quadrants. Each leaf
cell becomes a routing partition.

**Strengths:**
- Simple, deterministic construction (no graph algorithms)
- Naturally adaptive: dense regions subdivide further, sparse regions stay coarse
- Board corners and edges handled cleanly
- O(n log n) construction where n = component count

**Weaknesses:**
- Net-agnostic: a quadtree split can bisect a critical bus, creating many
  boundary crossings
- Poor match for non-square boards or boards with highly irregular component
  distribution
- Splitting criterion is spatial (component density), not routing-related
  (congestion, net topology)

**Variant -- adaptive quadtree**: Subdivide only when cell congestion exceeds
a threshold (estimated via RUDY or net bounding-box overlap). This produces
fewer, larger cells in sparse areas and fine-grained cells in congested areas.

#### KD-Tree Decomposition

Binary space partition with alternating axis splits. Each split chooses the
median along the current axis, producing balanced sub-regions.

**Strengths:**
- Balanced by construction (equal component count per partition)
- Median split minimizes worst-case partition size disparity
- Works well for non-square boards

**Weaknesses:**
- Same net-agnostic problem as quadtree
- Axis-alternating constraint can produce elongated partitions with poor
  aspect ratios, causing long boundary routing channels

**Best use:** Initial coarse decomposition, refined by net-aware methods.

#### Regular Grid / Tile Decomposition

The simplest approach: overlay a regular rectangular grid on the board and
assign each tile as a partition. This is what VLSI global routers use (the
"global routing tiles" model).

**Strengths:**
- Trivial to implement
- Direct mapping to GPU dispatch: tile (i,j) -> workgroup (i,j)
- Well-studied in VLSI (CUGR, GAMER, InstantGR all use tile grids)

**Weaknesses:**
- Fixed granularity -- some tiles are empty, others are packed
- Oblivious to net structure

**This is the ISPD/DAC contest standard.** The ISPD global routing contests
(2008, 2018, 2019, 2024, 2025) all use a regular tile-based decomposition
for the global routing graph. The routing region is partitioned into an
array of rectangular tiles, with edges between adjacent tiles representing
routing capacity. Global routing finds tile-to-tile paths; detailed routing
fills in the geometry within each tile.

### 1.2 Net-Connectivity-Based Partitioning (Graph Partitioning)

Instead of splitting spatially, partition the **netlist hypergraph**: vertices
are components (or pads), hyperedges are nets. The goal is to find k partitions
that minimize the number of nets cut across partition boundaries (min-cut
partitioning), since cut nets require inter-partition routing.

#### METIS / hMETIS

[METIS](https://github.com/KarypisLab/METIS) is the standard tool for graph
partitioning. [hMETIS](https://karypis.github.io/glaros/software/metis/overview.html)
extends it to hypergraphs, which is the natural representation for circuit
netlists (a net connects 2+ pins).

**Algorithm:** Multilevel approach with three phases:
1. **Coarsen**: collapse vertices into super-vertices, producing a hierarchy
   of progressively smaller graphs
2. **Initial partition**: partition the coarsest graph (small enough for
   exact/near-exact algorithms)
3. **Uncoarsen + refine**: project the partition back through the hierarchy,
   running Kernighan-Lin or Fiduccia-Mattheyses refinement at each level

**Performance:** hMETIS produces partitions that are 10-50% better (fewer cut
edges) than spectral methods, and is 1-2 orders of magnitude faster than other
widely used algorithms. It can partition hypergraphs with 100K+ vertices in
under 3 minutes.

**For PCB routing:**
- Build a hypergraph where each component is a vertex and each net is a
  hyperedge connecting all its pins' components
- Run hMETIS with k = desired partition count (e.g., 8-64 for GPU workgroups)
- Add spatial constraints: the partition must be geometrically contiguous
  (hMETIS doesn't guarantee this -- need post-processing)

**Rust integration:** No Rust crate wraps METIS/hMETIS directly, but the C
library is callable via FFI. Alternatively, implement the multilevel KL/FM
algorithm natively -- it is well-documented and the PCB netlist sizes (~1K-10K
components) are small enough that performance is not critical.

#### Spectral Partitioning

Compute the Fiedler vector (second-smallest eigenvector of the graph Laplacian)
and split vertices into two sets by the sign of their Fiedler vector entry.
Recursively bisect for k partitions.

**Strengths:** Globally optimal bisection in continuous relaxation.
**Weaknesses:** O(n^3) eigenvector computation; significantly outperformed by
multilevel heuristics (METIS) in practice.

**Verdict:** Not recommended for our use case. METIS/hMETIS dominates on both
quality and speed.

### 1.3 Component-Cluster-Based Partitioning (Placement Groups)

Our spec language already supports `placement_group` declarations, which group
components by functional block (power supply, DDR bus, analog front-end, etc.).
These groups are a natural partitioning unit because:

- Components within a group are typically placed near each other (the placer
  respects group constraints)
- Nets within a group tend to be short and local
- Inter-group nets are typically longer and less numerous
- The LLM already understands the schematic hierarchy and declares groups
  based on functional analysis

**This is our strongest advantage over traditional routers.** A traditional
autorouter must discover independent regions by analyzing the netlist graph.
Our LLM-authored spec declares them explicitly, eliminating the partitioning
computation entirely.

**Proposed approach:**
1. Each `placement_group` becomes a candidate routing partition
2. Estimate partition routing complexity: net count, pin count, area, estimated
   congestion (RUDY)
3. Merge small groups into larger partitions (to avoid boundary overhead)
4. Split large groups into sub-partitions (to increase parallelism)
5. Assign inter-group nets to boundary routing channels

### 1.4 Layer-Based Partitioning

In a multilayer PCB, different layers serve different purposes:

| Layer type | Routing strategy |
|------------|-----------------|
| Signal layers (e.g., L2, L3) | Dense signal routing, preferred H/V direction |
| Power planes (e.g., L1, L4) | Copper floods with anti-pads, minimal routing |
| Mixed layers | Signal routing + power distribution |

**Layer partitioning opportunities:**

1. **Power vs. signal separation**: Power planes are not "routed" in the
   traditional sense -- they are copper floods with clearance cuts. The router
   can handle power net connectivity separately (plane connection checks) and
   focus GPU resources on signal layer routing.

2. **Layer-pair independence**: On a 4-layer board, signals on L2 (horizontal)
   and L3 (vertical) only interact through vias. Within a single PathFinder
   iteration, nets assigned to different layer pairs can be routed independently
   if their via transitions don't conflict.

3. **Analog/digital layer isolation**: Mixed-signal boards often dedicate
   specific layers to analog vs. digital signals. These can be partitioned
   into separate routing domains with independent obstacle maps.

**Interaction with spatial partitioning:** Layer partitioning is orthogonal to
spatial partitioning. A partition can be defined as (spatial_region, layer_set),
giving a 3D decomposition of the routing problem.

### 1.5 How ISPD/DAC Routing Contests Handle Partitioning

The routing contest community has converged on several key approaches:

**ISPD 2008-2019 (Global Routing):**
- Regular tile grid decomposition (standard model)
- Negotiation-based (PathFinder) or LP-based global routing on tile graph
- Detailed routing within individual tiles
- Parallelism via multi-threaded tile processing

**ISPD 2024 (GPU/ML-Enhanced Global Routing):**
- Explicit GPU acceleration requirement
- InstantGR (ICCAD 2024 winner) uses **net batching** based on spatial
  independence: nets whose rectilinear Steiner minimum trees (RSMTs) have
  non-overlapping H and V segments are batched together for simultaneous
  GPU routing
- "Representative point exhaustion" algorithm for fast overlap checking
- Large nets (>12 pins) broken into independent subnets before batching
- Key finding: in the largest ISPD'24 design, average horizontal RSMT segment
  length is only 12 cells on a 9245x12544 grid -- most nets are local

**ISPD 2025 (Performance-Driven Global Routing):**
- Continued emphasis on parallelization and scalability
- Focus on timing-driven objectives alongside congestion

**TritonRoute (OpenROAD, 2024-2026):**
- Partition-based parallel detailed routing is an active research area
- The approach partitions the design into regions, routes each region as an
  independent sub-problem, and stitches results together
- Uses TritonPart for hypergraph-based partitioning with constraints-driven
  coarsening and V-cycle refinement
- Process-level parallelism (separate processes per partition) rather than
  thread-level, to avoid shared-state contention

**GAMER (ICCAD 2021, IEEE TCAD 2023):**
- GPU-accelerated maze routing applied to CUGR global router
- Decomposes multisource-multidestination shortest path into H/V sweep
  operations, each parallelizable
- 16x speedup on coarsened maze routing stage
- Does NOT partition the board; instead parallelizes within a single net's
  shortest-path search

**Corolla (FPGA 2017):**
- Multi-net parallelism via recursive spatial partitioning (RPTT)
- Three parallelism levels: single-net node parallelism (SNP), single-net
  edge parallelism (DEP), and multi-net parallelism
- Average 18.7x speedup

**Key takeaway from contest community:** The dominant trend is net-level
batching (InstantGR, GANGR) rather than spatial partitioning of the board.
Partition the *nets* into independent batches, not the *board* into
independent tiles. This avoids boundary routing complexity entirely.

---

## 2. Boundary Routing Between Partitions

When a net spans multiple partitions, the portions of the net in each
partition must be connected. This is the "boundary routing" or "stitching"
problem.

### 2.1 Virtual Pins at Partition Boundaries

The standard technique from VLSI hierarchical routing:

1. Each partition boundary is a line segment (2D) or plane (3D)
2. For each net crossing a boundary, place a "virtual pin" at the crossing
   point on the boundary
3. Within each partition, route from the net's real pins to the virtual pins
   on the partition boundary
4. After intra-partition routing, connect virtual pins across boundaries
   (channel routing)

**Virtual pin placement strategies:**
- **Fixed grid**: Place virtual pins at regular intervals along the boundary.
  Each boundary becomes a channel with fixed capacity. Simple but may waste
  resources (empty pin slots) or create congestion (all nets funneled to
  nearest pin).
- **Demand-driven**: Place virtual pins where inter-partition nets actually
  need to cross. Requires a global routing pass first to determine crossing
  points.
- **Negotiated**: Start with estimated crossing points, refine through
  PathFinder iterations. The virtual pin positions themselves become
  negotiable resources.

**For GPU routing:** Virtual pins transform each partition into a self-contained
routing problem with known source/target pins. This is ideal for GPU
workgroups: each workgroup routes one partition, reading only local obstacle
maps and pin positions.

### 2.2 Multi-Pass Approach: Route Partitions, Then Stitch

**Phase 1 -- Global routing across partitions:**
- Run coarse global routing on the full board to determine which partitions
  each net crosses and approximate crossing points
- Assign virtual pins at boundary crossings

**Phase 2 -- Intra-partition detailed routing (GPU parallel):**
- Each partition routes independently on a GPU workgroup
- Source/target pins include both real component pins and virtual boundary pins
- PathFinder negotiation runs within each partition

**Phase 3 -- Boundary channel routing:**
- Route the short segments connecting virtual pins across partition boundaries
- This is a 1D channel routing problem (well-studied, fast)
- Can use classic channel routing algorithms (left-edge, greedy, or
  constraint-based)

**Phase 4 -- Global conflict resolution:**
- Run a final PathFinder iteration on the full board to resolve any remaining
  conflicts at partition boundaries
- This is typically a small number of nets (only those crossing boundaries)

### 2.3 Nets Spanning 3+ Partitions

When a net spans three or more partitions, the routing problem becomes a
Steiner tree problem across partition boundaries:

```
Partition A          Partition B          Partition C
  [Pin1] ---> [VP_AB] ---> [Pin2]
                 |
                 v
              [VP_BC] ---> [Pin3]
```

**Approaches:**

1. **Global Steiner tree first**: Compute the Steiner tree on the coarse
   partition graph. This determines which partition boundaries the net
   crosses and in what topology. Then route each segment within its partition.

2. **Cascaded routing**: Route the net in partition A first (from Pin1 to
   VP_AB). Then route in partition B (from VP_AB to Pin2 and VP_BC). Then
   route in partition C (from VP_BC to Pin3). This is sequential per net but
   each partition's internal routing is GPU-parallel.

3. **Multi-commodity flow**: Model inter-partition routing as a multi-commodity
   flow problem on the partition boundary graph. Solve the LP to get
   approximate fractional flows, then round to integer assignments. This
   handles capacity constraints on boundary channels.

**Practical recommendation:** For PCB-scale boards (<10K nets), most nets are
local to a single partition (especially with placement-group-based
partitioning). The ~5-15% of nets that span multiple partitions can be routed
in a separate sequential pass after intra-partition routing completes. This
avoids the complexity of multi-partition Steiner tree routing.

### 2.4 Channel Routing at Partition Boundaries

Each partition boundary becomes a routing channel -- a rectangular region
between two partitions where inter-partition nets must be routed.

**Channel routing is a well-solved problem in VLSI:**
- The channel has a fixed width (determined by the gap between partitions in
  the routing grid)
- Pins on the top and bottom of the channel are the virtual pins from the
  adjacent partitions
- Classic algorithms: left-edge (optimal for non-crossing nets), greedy
  channel router, Yoshimura-Kuh (dogleg), constraint-based

**For GPU routing:** Channel routing is fast (O(n log n) where n = nets in
channel) and sequential. Run it on CPU after GPU intra-partition routing
completes. The channel routing results are then fed back as constraints for
the next PathFinder iteration.

---

## 3. GPU Workgroup Mapping

### 3.1 One Partition Per Workgroup vs. One Partition Per Dispatch

**Option A: One partition per workgroup (single dispatch)**

```
dispatch(num_partitions, 1, 1)
  workgroup[i] routes partition i
```

- All partitions processed simultaneously in one dispatch
- Workgroup shared memory used for local BFS frontier / Bellman-Ford state
- Severe constraint: 256 max invocations per workgroup (wgpu default)
- For a partition with 500x500 grid = 250K cells, only 256 threads can
  process them -> each thread handles ~1000 cells per Bellman-Ford iteration
- **Limited benefit**: the intra-workgroup parallelism is too small for
  meaningful routing speedup per partition

**Option B: One partition per dispatch (sequential dispatches)**

```
for partition in partitions:
    dispatch(ceil(partition.cells / 64), 1, 1)
      all workgroups cooperate on routing this partition
```

- Full GPU parallelism applied to one partition at a time
- Millions of threads for large partitions
- Sequential across partitions -- no inter-partition parallelism
- Simple: same shader as non-partitioned routing, just smaller grid

**Option C: Batched independent partitions (recommended)**

```
// Group partitions that are spatially independent
for batch in independent_partition_batches:
    // Each partition gets its own grid section in a shared buffer
    dispatch(total_cells_in_batch / 64, 1, 1)
```

- Multiple independent partitions share a single dispatch
- Each thread checks which partition its cell belongs to
- Partition data laid out contiguously in buffers (partition A cells, then
  partition B cells, etc.)
- Maximizes GPU utilization while respecting independence constraints

**Recommendation:** Option C (batched independent partitions) for
inter-partition parallelism, combined with the existing Bellman-Ford
parallelism for intra-partition routing. This gives two levels of parallelism:
- Level 1: Multiple independent partitions in one dispatch
- Level 2: All cells within each partition processed in parallel

### 3.2 Load Balancing

Partitions have wildly different routing complexity:
- A power supply partition (few nets, large copper pours) might have 50 nets
- A DDR memory partition (many parallel data lines) might have 200+ nets
- A microcontroller BGA escape partition might have 500+ nets in a small area

**Static load balancing (at partitioning time):**
- Estimate per-partition complexity: `complexity ~ net_count * avg_net_span * congestion_factor`
- Use this estimate to split large partitions and merge small ones
- Target equal complexity per partition, not equal area

**Dynamic load balancing (at dispatch time):**
- Assign partition->workgroup mapping based on estimated work
- Large partitions get more workgroups (Option B behavior)
- Small partitions are batched into a single dispatch (Option C behavior)
- Use `atomicAdd` on a global work counter for work-stealing between
  workgroups (advanced, may not be worth the complexity)

**Practical approach for PCB routing:**
PCB-scale boards have relatively few partitions (8-32 typical). The overhead
of sub-optimal load balancing is small compared to the routing computation
itself. Start with static balancing (equal-complexity partitions), profile,
and add dynamic balancing only if profiling shows significant load imbalance.

### 3.3 Memory Layout for Multi-Partition GPU Routing

Each partition needs its own:
- Distance array (u32 per cell, reset per net)
- History array (u32 per cell, persistent across nets)
- Obstacle bitmap (1 bit per cell per layer)
- Predecessor array (u32 per cell, for path reconstruction)

**Layout option 1: Concatenated buffers**

```
Buffer:  [--- Partition 0 ---][--- Partition 1 ---][--- Partition 2 ---]
Offset:  0                    P0.cells             P0.cells + P1.cells
```

Each thread computes its global cell index as
`partition_offset[partition_id] + local_cell_index`. Partition offsets stored
in a uniform buffer.

- Pro: Single buffer allocation, single bind group
- Con: Non-uniform partition sizes cause irregular access patterns

**Layout option 2: Separate buffer per partition**

```
Buffer 0: [--- Partition 0 ---]
Buffer 1: [--- Partition 1 ---]
Buffer 2: [--- Partition 2 ---]
```

Each dispatch binds the buffer for its target partition.

- Pro: Clean addressing (local cell index only), no offset calculation
- Con: Limited to 8 storage buffers per shader stage (wgpu limit) -- only
  2 partitions per dispatch if each needs 4 buffers (dist, history, obstacles,
  predecessor)

**Layout option 3: Uniform-sized partitions with padding**

```
Buffer:  [--P0--pad][--P1--pad][--P2--pad]
         |<- S  ->| |<- S  ->| |<- S  ->|
```

All partitions padded to the same size S. Thread computes cell as
`partition_id * S + local_cell_index`. No offset lookup needed.

- Pro: Regular access pattern, simple addressing, good for GPU cache
- Con: Wasted memory from padding (up to 2x if partition sizes vary widely)

**Recommendation:** Option 1 (concatenated) for production, Option 3
(uniform-padded) as a simpler starting point. The 8-buffer-per-stage limit
makes Option 2 impractical for more than 2 concurrent partitions.

### 3.4 Partition Sizing: The Central Trade-off

```
Too small partitions:
  + More parallelism (more independent units)
  - More inter-partition nets (boundary routing overhead)
  - More virtual pins (memory + routing complexity)
  - Small partitions may have fewer cells than GPU threads (waste)

Too large partitions:
  + Fewer boundary crossings
  + Amortized overhead
  - Less inter-partition parallelism
  - GPU underutilized if only 1-2 partitions active
```

**Empirical guidance from the literature:**

- **GAMER** uses global routing tiles of ~100x100 cells. At our default
  0.1mm grid resolution, that is 10mm x 10mm physical area.
- **InstantGR** notes that average net RSMT segment length is ~12 tiles on
  a 9245x12544 grid. Most nets are local.
- **TritonRoute** partition-based routing uses regions sized to give each
  process sufficient work (roughly 1000-5000 nets per partition).
- **RPTT (Corolla)** recursively partitions until sub-regions contain
  ~50-200 nets each.

**Recommended partition sizing heuristic for PCB boards:**

| Board size | Target partitions | Partition area | Nets/partition |
|------------|-------------------|----------------|----------------|
| Small (<200 nets) | 1-2 | Full board | All |
| Medium (200-1000 nets) | 4-8 | 15-30mm square | 50-200 |
| Large (1000-5000 nets) | 8-32 | 10-20mm square | 100-500 |
| Very large (5000+ nets) | 32-64 | 5-15mm square | 100-300 |

These are starting points. Actual partition sizes should be tuned based on:
- Net locality (more local nets -> smaller partitions OK)
- Congestion distribution (congested areas need finer partitions)
- GPU memory (each partition's arrays must fit in VRAM)
- GPU occupancy (need enough total cells across all concurrent partitions
  to saturate the GPU)

---

## 4. Room-Based Routing (Altium Rooms)

### 4.1 How Altium Designer Handles Rooms

Altium Designer's "Room" is a spatial constraint that groups components into
a defined PCB region. Key characteristics:

- **Creation**: Rooms can be created manually or automatically from
  hierarchical schematic sheets. Each sheet in a hierarchical design can
  generate a corresponding room.
- **Nesting**: Rooms can be nested (child rooms within parent rooms),
  creating a hierarchy that mirrors the schematic hierarchy.
- **Design rules**: Rooms support room-specific design rules (clearance,
  width, routing layers, etc.). Rules scoped to a room override global rules.
- **Multi-channel replication**: In multi-channel designs (e.g., 8 identical
  ADC channels), one room is designed and then "replicated" -- placement and
  routing are copied to identical rooms with different component designators.
- **Routing containment**: Altium's interactive router can be configured to
  keep routes within a room's boundary, though the autorouter does not
  strictly enforce room containment for inter-room nets.

### 4.2 LLM-Defined Rooms for Routing Partitions

Our spec language can declare rooms based on schematic hierarchy analysis.
The LLM examines the schematic and identifies functional blocks:

```
room "power_supply" {
  components: [U1, C1..C8, L1, D1]
  area: rect(5mm, 5mm, 25mm, 20mm)
  routing_rules {
    min_width: 0.3mm
    preferred_layers: [L1, L4]
  }
}

room "ddr_interface" {
  components: [U2, R1..R16, C9..C24]
  area: rect(30mm, 10mm, 60mm, 40mm)
  routing_rules {
    min_width: 0.1mm
    preferred_layers: [L2, L3]
    matched_length_tolerance: 0.5mm
  }
}
```

**Mapping rooms to GPU routing partitions:**
1. Each room becomes a routing partition
2. Room-internal nets (all pins within the room) are routed on the GPU
   workgroup assigned to that room
3. Inter-room nets are identified and routed in a separate pass
4. Room-specific routing rules become per-partition configuration

**Advantages of LLM-defined rooms:**
- Zero computational cost for partitioning (declared, not discovered)
- Semantically meaningful boundaries (functional blocks, not arbitrary
  spatial cuts)
- Room rules provide partition-specific constraints that improve routing
  quality
- Multi-channel rooms enable "route once, replicate" -- a single GPU routing
  solution applied to all identical channels

### 4.3 Room-to-Room Routing Channels

The space between rooms is the "channel" where inter-room nets are routed.
This mirrors the VLSI channel routing paradigm:

```
+------------------+    channel    +------------------+
|   Room A         |<------------>|   Room B         |
|  (power supply)  |    5mm gap   |  (DDR interface) |
+------------------+              +------------------+
        |                                   |
        |          channel (3mm gap)        |
        v                                   v
+------------------+              +------------------+
|   Room C         |              |   Room D         |
|  (analog input)  |              |  (digital I/O)   |
+------------------+              +------------------+
```

**Channel routing strategy:**
1. After intra-room routing, each room exports its boundary connection
   points (virtual pins on room edges)
2. Channel routing connects virtual pins between adjacent rooms
3. Channels have limited capacity (tracks that fit in the gap between rooms)
4. If a channel is over-capacity, the partitioner must either:
   a. Widen the channel (adjust room placement)
   b. Route some nets through other rooms (detour)
   c. Use additional layers

**GPU acceleration for channel routing:** Channels are small (few hundred
cells) and sequential. CPU channel routing is fast enough. The GPU benefit
is in the intra-room routing, which is the bulk of the work.

---

## 5. Multi-Solution Exploration Within Partitions

### 5.1 Running N Parallel Route Attempts Per Partition

For each partition, launch N independent routing attempts with different
parameters:

```
Partition P, Attempt 0: via_cost=1.0, layer_bias=strong, pres_fac_start=0.5
Partition P, Attempt 1: via_cost=2.0, layer_bias=weak,   pres_fac_start=1.0
Partition P, Attempt 2: via_cost=0.5, layer_bias=none,   pres_fac_start=0.5
Partition P, Attempt 3: via_cost=1.0, layer_bias=strong, pres_fac_start=2.0
```

Each attempt runs independently on the GPU. After all attempts complete (or
a time budget expires), select the best solution per partition.

**What can vary between attempts:**
- Via cost multiplier (trade via count vs. wirelength)
- Layer direction bias strength (strict H/V vs. flexible)
- PathFinder pressure factor initial value and growth rate
- Net ordering (different RNG seeds)
- Layer assignment (different starting layer preferences)
- Pin escape direction (different BGA fanout strategies)

**What should NOT vary between attempts:**
- Obstacle maps (same physical constraints)
- Design rules (clearance, width -- these are hard constraints)
- Net connectivity (same nets in each attempt)

### 5.2 Selecting the Best Solution Per Partition

**Scoring function for partition solutions:**

```
score = w1 * total_wirelength
      + w2 * total_vias
      + w3 * unrouted_net_penalty
      + w4 * congestion_hotspot_penalty
      + w5 * timing_violation_penalty
```

Lower score is better. The weights can be tuned per partition (e.g., DDR
partitions weight timing higher, power partitions weight via count higher).

**Selection happens on CPU** after reading back all attempt results. This is
fast (O(N) where N = attempt count, typically 4-16).

### 5.3 GPU Memory Budget for Multi-Solution Exploration

Each routing attempt needs its own set of arrays:

| Array | Size per attempt (500x500 grid, 4 layers) |
|-------|-------------------------------------------|
| Distance | 500 * 500 * 4 * 4 bytes = 4 MB |
| History | 4 MB |
| Predecessor | 4 MB |
| Obstacles | 500 * 500 * 4 / 8 = 125 KB (shared, read-only) |

Per attempt: ~12 MB. Per partition with N=8 attempts: ~96 MB.

**For a board with 8 partitions, each with 8 parallel attempts:**
64 simultaneous routing instances * 12 MB = 768 MB.

This fits comfortably in discrete GPU VRAM (4-24 GB typical). Even with
larger grids (1000x1000), 64 instances would use ~3 GB -- still feasible on
most discrete GPUs.

**For integrated GPUs (shared memory, 1-4 GB available):**
Reduce to 4 partitions * 4 attempts = 16 instances * 12 MB = 192 MB.
Still feasible.

**Practical number of parallel attempts:**

| GPU VRAM | Grid size | Max attempts/partition | Max concurrent |
|----------|-----------|------------------------|----------------|
| 2 GB | 500x500x4 | 8 | 16 partitions * 8 = 128 |
| 4 GB | 500x500x4 | 16 | 16 * 16 = 256 |
| 8 GB | 1000x1000x4 | 8 | 8 * 8 = 64 |
| 16 GB | 1000x1000x4 | 16 | 8 * 16 = 128 |

These are conservative estimates (pure array memory, not counting GPU
overhead, shader state, etc.). In practice, allocate 60-70% of available
VRAM for routing arrays and use the rest for overhead.

### 5.4 When Multi-Solution Exploration is Worth It

Multi-solution exploration adds value when:
- **The solution space is rugged**: different parameter settings produce
  significantly different routing quality (common for congested partitions)
- **Critical partitions**: DDR, high-speed serial, dense BGA escape --
  partitions where routing quality directly impacts signal integrity
- **Convergence is uncertain**: if PathFinder might not converge with one
  parameter set, trying multiple simultaneously hedges the risk

Multi-solution exploration is NOT worth it when:
- **The partition is simple**: few nets, no congestion, trivial routing
- **Memory is constrained**: integrated GPUs with limited shared memory
- **Time is more valuable than quality**: quick-and-dirty routing for
  feasibility checks

**Recommendation:** Enable multi-solution exploration only for partitions
marked as `priority: critical` in the spec. For non-critical partitions,
route once with default parameters. The spec's priority annotations
naturally control GPU resource allocation.

---

## 6. Recommended Architecture for Our Router

Based on the research above, here is the recommended partitioning architecture
for our GPU-accelerated PathFinder router:

### 6.1 Partitioning Pipeline

```
PcbIr + RoutingConfig + Spec
        |
        v
1. LLM-declared rooms/groups -> candidate partitions
   (zero cost, semantically meaningful)
        |
        v
2. Complexity estimation per partition
   (net count, pin count, RUDY congestion estimate)
        |
        v
3. Partition refinement
   - Merge partitions with < 20 nets (boundary overhead > benefit)
   - Split partitions with > 500 nets (enable parallelism)
   - Balance: target equal complexity, not equal area
        |
        v
4. Inter-partition net analysis
   - Identify nets spanning 2+ partitions
   - Compute virtual pin positions via global routing
   - Estimate channel capacity requirements
        |
        v
5. GPU resource allocation
   - Assign workgroup batches to partitions
   - Allocate per-partition buffer regions
   - Assign multi-solution attempt count per partition
     (more for critical/congested, fewer for simple)
```

### 6.2 Routing Execution

```
Phase 1: Intra-partition routing (GPU parallel)
  For each PathFinder iteration:
    For each batch of independent partitions:
      dispatch GPU Bellman-Ford for all partitions in batch
    Update per-partition history arrays (GPU)
    Check per-partition convergence (CPU)

Phase 2: Boundary channel routing (CPU)
  Extract virtual pins from partition solutions
  Run channel routing for each inter-partition boundary
  Resolve conflicts at boundary crossings

Phase 3: Global refinement (optional, GPU)
  Run 1-3 PathFinder iterations on full board
  Focus on boundary region nets only
  Verify no inter-partition DRC violations

Phase 4: Solution selection (if multi-solution)
  Score each partition's best attempt
  Stitch selected solutions together
  Final DRC check on assembled solution
```

### 6.3 Key Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Primary partitioning | Spec-declared rooms/groups | Zero cost, semantically correct, LLM advantage |
| Fallback partitioning | hMETIS on netlist hypergraph | Best quality for automated partitioning |
| Partition granularity | 50-500 nets per partition | Balances parallelism vs. boundary overhead |
| GPU mapping | Batched independent partitions | Maximizes GPU utilization |
| Buffer layout | Concatenated with offset table | Single allocation, flexible partition sizes |
| Boundary routing | CPU channel routing after GPU | Channels are small, sequential is fine |
| Multi-solution | Critical partitions only | GPU memory budget, diminishing returns |
| Inter-partition nets | Global route first, then stitch | Determines virtual pin positions upfront |

---

## 7. References

### Papers: Partitioning and Parallel Routing

- **METIS**: Karypis & Kumar, "A Fast and High Quality Multilevel Scheme for Partitioning Irregular Graphs," SIAM J. Sci. Comput. 1998.
  https://www.cs.utexas.edu/~pingali/CS395T/2009fa/papers/metis.pdf
- **hMETIS**: Karypis & Kumar, "Multilevel Hypergraph Partitioning," DAC 1997.
  https://dl.acm.org/doi/pdf/10.1145/266021.266273
  - Manual: https://course.ece.cmu.edu/~ee760/760docs/hMetisManual.pdf
  - Software: https://karypis.github.io/glaros/software/metis/overview.html
- **RPTT (Recursive Partitioning Ternary Tree)**: Zang et al., "Accelerate FPGA Routing with Parallel Recursive Partitioning," FPGA 2023.
  https://ceca.pku.edu.cn/media/lw/f8937474bc4352fa545e32f55ae8e7be.pdf
  - Open source: https://github.com/xszang/parallel-routing
- **Bamboo**: Shen et al., "Dependency-Aware Parallel Routing for Large-Scale FPGAs," ICCAD 2017.
  https://ceca.pku.edu.cn/media/lw/bbdead22b825f1afe2a210c44a124640.pdf
  - IEEE: https://ieeexplore.ieee.org/document/8119218/
- **InstantGR**: Lin et al., "InstantGR: Scalable GPU Parallelization for Global Routing," ICCAD 2024.
  https://dl.acm.org/doi/10.1145/3676536.3676787
  - GitHub: https://github.com/cuhk-eda/InstantGR
  - PDF: https://shijulin.github.io/files/1239_Final_Manuscript.pdf
- **GANGR**: "GAN-Assisted Scalable and Efficient Global Routing Parallelization," 2025.
  https://arxiv.org/abs/2511.17665
- **GAMER**: "GPU Accelerated Maze Routing," IEEE TCAD 2023.
  https://ieeexplore.ieee.org/document/9799536
- **Corolla**: Shen et al., "GPU-Accelerated FPGA Routing Based on Subgraph Dynamic Expansion," FPGA 2017.
  https://dl.acm.org/doi/10.1145/3020078.3021732
  - PDF: https://ceca.pku.edu.cn/media/lw/137e5df7dec627f988e07d54ff222857.pdf
- **TritonRoute**: Kahng et al., "TritonRoute: The Open Source Detailed Router," IEEE TCAD 2021.
  https://vlsicad.ucsd.edu/Publications/Journals/j133.pdf
  - Partition-based parallel routing: https://dev.to/wiowiztech/parallel-region-based-routing-on-openroad-scaling-beyond-multithreading-3jf6
- **Parallel VLSI routing (survey)**: "Challenges and Approaches in VLSI Routing."
  http://www.or.uni-bonn.de/~held/publications/VLSI_Routing.pdf
- **Multi-net routing lecture**: ECE6133 Physical Design Automation.
  https://limsk.ece.gatech.edu/course/ece6133/slides/multi-net.pdf
- **VLSI channel routing**: CMU channel routing reference.
  https://www.cs.cmu.edu/~jab/pubs/propo/node19.html
- **Hypergraph partitioning survey**: Papa & Markov.
  https://web.eecs.umich.edu/~imarkov/pubs/book/part_survey.pdf
- **(Hyper)Graph Partitioning Advances**: Gottesburen et al., 2022.
  https://arxiv.org/pdf/2205.13202
- **Sphynx**: Parallel multi-GPU graph partitioner.
  https://www.sciencedirect.com/science/article/abs/pii/S0167819121000272

### Papers: GPU Shortest Path and Graph Algorithms

- **Parallel Bellman-Ford SSSP**: PPoPP 2021.
  https://www.cs.utexas.edu/~lin/papers/ppopp21.pdf
- **Delta-stepping on compute shaders**: Niklaus.
  https://www.execfoo.de/blog/deltastep_shader.html
- **GPU graph partitioning**: Burtscher et al., HCW 2016.
  https://userweb.cs.txstate.edu/~burtscher/papers/hcw16.pdf
- **All-pairs shortest path on GPU**: Bulucc et al.
  https://www.sciencedirect.com/science/article/abs/pii/S0743731515001069
- **GPU SSSP for road networks**: 2024.
  https://www.tandfonline.com/doi/abs/10.1080/13658816.2024.2394651
- **NVIDIA shortest path overview**.
  https://developer.nvidia.com/discover/shortest-path-problem
- **Parallel shortest path via graph partitioning and iterative correcting**.
  https://www.researchgate.net/publication/224332373

### Projects and Tools

- **METIS** (graph partitioning): https://github.com/KarypisLab/METIS
- **hMETIS** (hypergraph partitioning): https://karypis.github.io/glaros/software/metis/overview.html
- **InstantGR** (GPU global router): https://github.com/cuhk-eda/InstantGR
- **CUGR** (VLSI global router): https://github.com/cuhk-eda/cu-gr
- **OrthoRoute** (GPU PCB autorouter): https://github.com/bbenchoff/OrthoRoute
- **OpenROAD** (full RTL-to-GDS): https://github.com/The-OpenROAD-Project/OpenROAD
- **TritonRoute** (detailed router): https://github.com/The-OpenROAD-Project/TritonRoute

### Routing Contests

- **ISPD 2024 GPU/ML Routing Contest**: https://liangrj2014.github.io/ISPD24_contest/
- **ISPD 2025 Performance-Driven Routing**: https://dl.acm.org/doi/10.1145/3698364.3715706
- **ISPD 2019 Detailed Routing**: https://www.ispd.cc/contests/19/
- **ISPD 2008 Global Routing**: http://www.ispd.cc/contests/08/ispd08rc.html

### Altium Designer Documentation

- **Working with Rooms**: https://www.altium.com/documentation/altium-designer/pcb/rooms
- **Multi-Channel Design**: https://resources.altium.com/p/multi-channel-design-with-a-flat-project
- **Group Components into Rooms**: https://resources.altium.com/p/group-components-into-rooms-for-more-efficient-layout
- **Connection Rooms**: https://resources.altium.com/p/connection-room
