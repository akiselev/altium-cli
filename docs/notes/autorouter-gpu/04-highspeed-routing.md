# GPU-Accelerated High-Speed PCB Routing

Research notes on differential pair routing, length matching, bus routing,
impedance-controlled routing, and DDR-specific routing -- all in the context
of GPU acceleration via wgpu compute shaders and LLM-authored spec constraints.

Builds on the foundations from `00-overview.md` (Bellman-Ford on GPU, PathFinder
architecture, wgpu capabilities, LLM-authored spec features).

---

## Table of Contents

1. [Differential Pair Routing on GPU](#1-differential-pair-routing-on-gpu)
2. [Length Matching on GPU](#2-length-matching-on-gpu)
3. [Bus Routing on GPU](#3-bus-routing-on-gpu)
4. [Impedance-Controlled Routing](#4-impedance-controlled-routing)
5. [DDR-Specific Routing](#5-ddr-specific-routing)
6. [What the LLM Can Pre-Compute](#6-what-the-llm-can-pre-compute)
7. [Proposed Architecture](#7-proposed-architecture)
8. [References](#8-references)

---

## 1. Differential Pair Routing on GPU

### 1.1 Problem Statement

Differential pairs are two nets (P and N) that must be routed together with
controlled spacing (gap), matched lengths (low skew), and coupled geometry for
impedance control. The router must:

- Maintain a target gap between P and N traces (typically 5-15 mil)
- Minimize uncoupled length (where gap exceeds tolerance)
- Minimize intra-pair skew (length difference between P and N)
- Handle corners, vias, and obstacles that force temporary uncoupling
- Respect per-pair impedance targets derived from stackup

### 1.2 How Commercial Tools Handle It

**Altium Designer** routes differential pairs interactively with a "Keep Coupled"
mode. Both traces are placed simultaneously with the router attempting to maintain
the configured gap. When the pair encounters an obstacle or bend, the traces
temporarily decouple and recouple afterward. Altium tracks coupled vs. uncoupled
length and reports intra-pair skew. Design rules specify width, gap, and coupled
tolerance (the gap range within which the pair is considered "coupled"). Altium
uses the `DiffPairsRouting` rule kind with parameters for primary gap, positive/
negative tolerance, max uncoupled length, and max skew.

**Cadence Allegro** uses a similar model with "coupled tolerance" to account for
geometric impossibilities at bends. The gap at a bend cannot maintain the exact
primary gap due to inner/outer radius differences. Allegro classifies segments
as coupled or uncoupled based on whether the edge-to-edge gap falls within the
tolerance window. Allegro also supports differential pair escape routing from
BGAs with ordered pin assignment.

**Key insight from both tools**: Differential pair routing is fundamentally a
**two-net coupled shortest path problem**, not two independent shortest paths
with a post-hoc gap check. The coupling constraint must be embedded in the
search itself.

### 1.3 Coupled Bellman-Ford: Routing P/N Simultaneously

The core idea: **double the grid state space** so that each search node
represents the positions of *both* traces simultaneously.

#### State Space

For single-net routing, a node is `(x, y, layer)`. For differential pair
routing, a node is `(xP, yP, layerP, xN, yN, layerN)` -- the joint position
of both traces. In practice, the coupling constraint dramatically reduces the
live state space: when the pair is coupled, `xN = xP + gap_offset_x` and
`yN = yP + gap_offset_y` (depending on routing direction), so the coupled
state can be encoded as `(x, y, layer, direction, coupling_mode)`.

#### Encoding for GPU

```
Coupled node:   (x, y, layer, dir)     -- N position is implicit from gap + dir
Uncoupled node: (xP, yP, xN, yN, layer) -- both positions explicit
```

For the GPU, store two distance arrays:

- `dist_coupled[x][y][layer][dir]` -- cost when pair is coupled, direction
  determines N offset. `dir` in {H, V} for horizontal/vertical routing.
- `dist_uncoupled[xP][yP][xN][yN][layer]` -- cost when pair is decoupled.
  This is O(grid^4) so must be bounded: uncoupled segments are short
  (typically < 20 grid cells), so only allocate a local window around the
  decouple point.

#### Bellman-Ford Relaxation

Each GPU thread processes one coupled node `(x, y, layer, dir)`:

```
// Pseudocode for coupled differential pair Bellman-Ford step
fn relax_coupled(x, y, layer, dir):
    let (xN, yN) = offset(x, y, dir, gap)
    let current = dist_coupled[x][y][layer][dir]
    if current == INFINITY: return

    // Try advancing both traces in the same direction (stay coupled)
    for next_dir in [same_dir, perpendicular]:
        let (nx, ny) = advance(x, y, next_dir)
        let (nxN, nyN) = offset(nx, ny, next_dir, gap)
        if !blocked(nx, ny, layer) && !blocked(nxN, nyN, layer):
            let cost = base_cost + gap_penalty(actual_gap, target_gap)
            atomicMin(&dist_coupled[nx][ny][layer][next_dir], current + cost)

    // Try decoupling (obstacle in N path, or bend required)
    if obstacle_forces_decouple(xN, yN, layer, dir):
        // Transition to uncoupled state
        atomicMin(&dist_uncoupled[x][y][xN][yN][layer], current + decouple_penalty)
```

A second kernel handles uncoupled nodes: each trace advances independently,
with a **recoupling bonus** when the gap returns to target. The uncoupled
penalty accumulates per grid step to incentivize short uncoupled segments.

#### Gap Enforcement as a Cost Function Term

Rather than hard-blocking non-gap-compliant positions, model the gap as a
**continuous cost term**:

```
gap_cost(actual, target, tolerance) =
    if |actual - target| <= tolerance: 0
    else: GAP_WEIGHT * (|actual - target| - tolerance)^2
```

This allows the search to find routes through narrow channels where the gap
must temporarily deviate, while strongly preferring coupled routing. The
quadratic penalty ensures small deviations are cheap but large deviations
are prohibitive.

#### Skew Minimization

Intra-pair skew = |length_P - length_N|. During coupled routing, skew is
zero by construction (both traces advance the same distance). Skew
accumulates during uncoupled segments and at bends (inner trace is shorter
than outer trace).

**GPU approach**: Track cumulative length of P and N as additional state in
the uncoupled distance array. At each uncoupled node, the cost includes a
skew penalty:

```
skew_penalty = SKEW_WEIGHT * |cumulative_length_P - cumulative_length_N|
```

This makes the search prefer uncoupling strategies that minimize skew
(e.g., alternating which trace takes the inner corner).

#### Complexity Analysis

- Coupled state space: `W * H * L * 2` (2 directions) -- same order as
  single-net routing.
- Uncoupled state space: bounded by `W * H * R^2 * L` where `R` is the
  max uncoupled radius (typically 10-20 cells). For R=20, this is 400x
  the single-net space per decouple region, but these regions are sparse.
- GPU parallelism: coupled relaxation has the same thread count as
  single-net BF. Uncoupled relaxation runs in a separate kernel over
  active decouple regions only.

### 1.4 Alternative: Sequential P-then-N with Offset

A simpler (but less optimal) approach used by some tools:

1. Route the P trace using standard single-net Bellman-Ford
2. Generate the N trace by offsetting the P trace by the gap distance
3. Check for obstacles on the N trace; if blocked, locally reroute N
4. Insert length compensation (serpentine) to match skew

**GPU mapping**: Step 1 is standard GPU BF. Step 2 is embarrassingly
parallel (one thread per segment). Step 3 requires local re-routing
(small GPU BF in a bounding box). Step 4 is the length-matching problem
(see Section 2).

**Trade-off**: Simpler to implement, but produces lower-quality routes.
The coupled approach finds globally optimal joint paths; the sequential
approach can get stuck in local optima where P's path makes N unroutable.

**Recommendation**: Implement sequential P-then-N first (Milestone 8), then
upgrade to coupled BF as an optimization. The sequential approach is adequate
for most PCB designs; coupled BF is needed for dense BGA escape routing.

### 1.5 Uncoupled Length Tracking

Track coupled vs. uncoupled length per differential pair for reporting and
DRC. On GPU, this is a parallel reduction after path reconstruction:

```
for each segment in diff_pair_path:
    if gap_within_tolerance(segment): coupled_length += segment.length
    else: uncoupled_length += segment.length
```

This runs as a post-processing kernel, one workgroup per differential pair.
Report `uncoupled_ratio = uncoupled_length / total_length` and flag
violations where `uncoupled_length > max_uncoupled_length` (from the
DiffPairsRouting rule).

---

## 2. Length Matching on GPU

### 2.1 Problem Statement

Length matching requires all nets in a group to have the same electrical
length (within tolerance). The target is typically the longest net in the
group. Shorter nets must be extended by inserting serpentine (accordion/
trombone/sawtooth) patterns.

Common length-matching groups:
- DDR data byte lanes (DQ0-DQ7 matched to DQS0)
- DDR address/command bus (matched to CK)
- Differential pair P/N (intra-pair matching)
- Parallel buses (USB, LVDS, etc.)

### 2.2 Length Calculation as Parallel Reduction

Computing the total routed length of a net is a textbook parallel
reduction. Each trace segment has a length; sum them.

**GPU kernel** (one workgroup per net):

```wgsl
@compute @workgroup_size(64)
fn compute_net_length(@builtin(global_invocation_id) gid: vec3<u32>) {
    let net_idx = gid.x / WORKGROUP_SIZE;
    let local_idx = gid.x % WORKGROUP_SIZE;

    let seg_start = net_segment_offsets[net_idx];
    let seg_end = net_segment_offsets[net_idx + 1];
    let seg_count = seg_end - seg_start;

    // Each thread sums its portion of segments
    var local_sum: f32 = 0.0;
    for (var i = local_idx; i < seg_count; i += WORKGROUP_SIZE) {
        let seg = segments[seg_start + i];
        local_sum += segment_length(seg);
    }

    // Workgroup reduction using shared memory
    shared_mem[local_idx] = local_sum;
    workgroupBarrier();

    // Tree reduction
    for (var stride = WORKGROUP_SIZE / 2u; stride > 0u; stride >>= 1u) {
        if (local_idx < stride) {
            shared_mem[local_idx] += shared_mem[local_idx + stride];
        }
        workgroupBarrier();
    }

    if (local_idx == 0u) {
        net_lengths[net_idx] = shared_mem[0];
    }
}
```

For N nets with M segments average, this runs in O(M/64 + log(64))
per net, all nets in parallel. Trivially fast on GPU.

### 2.3 Group-Based Length Targeting

After computing all net lengths in the group, the target length is
`max(lengths) + tolerance_margin`. This comparison runs on CPU (one
scalar per net) since the group sizes are small (8-16 nets typically).

The length deficit per net is:

```
deficit[net] = target_length - current_length[net]
```

Nets with `deficit < tolerance` need no adjustment. Nets with positive
deficit need serpentine insertion to add exactly `deficit` length.

### 2.4 Serpentine/Accordion Insertion

Serpentine insertion adds meander patterns to increase trace length. A
serpentine pattern is characterized by:
- **Amplitude** (A): perpendicular distance from the trace centerline
- **Pitch** (P): distance between meander peaks along the trace
- **Style**: accordion (90-degree bends), sawtooth (45-degree bends),
  or trombone (U-turn bends)

The length added per meander period is approximately:
```
accordion:  4 * A            (two perpendicular segments of length A each way)
sawtooth:   2 * sqrt(A^2 + (P/2)^2) - P   (two diagonals minus straight)
trombone:   2 * A + gap      (U-turn with gap between parallel segments)
```

#### Can Serpentine Insertion Be Parallelized?

**Partially.** The algorithm has three phases:

1. **Region selection** (CPU): Identify segments where serpentine can be
   inserted -- straight segments with sufficient clearance on both sides.
   This requires spatial queries (R-tree) and is sequential per net.

2. **Pattern computation** (GPU-parallelizable): Given a segment and a
   target added-length, compute the number of meander periods, amplitude,
   and pitch. This is arithmetic per segment, embarrassingly parallel.

3. **Geometry generation** (GPU-parallelizable): Convert meander
   parameters to actual trace segments. One thread per meander period,
   each producing 3-5 trace segments.

4. **DRC validation** (CPU or GPU): Check that generated serpentine
   segments don't violate clearance to neighbors. Can be GPU-parallelized
   as a batch obstacle query.

**Key constraint**: Serpentine segments should be inserted **close to the
source of the mismatch** (near the shorter end), not arbitrarily. This is
a heuristic decision best made on CPU.

**Practical GPU benefit**: For large matched groups (e.g., 32-bit DDR bus
with 40+ nets), computing serpentine geometry for all nets simultaneously
on GPU saves significant time vs. sequential CPU computation.

#### DAC 2024 Paper: Obstacle-Aware Length-Matching

Fang et al. (DAC 2024) present the first length-matching algorithm for
**any-direction traces** (not restricted to Manhattan grid). Their approach:

1. Assign non-overlapping routing regions to each trace
2. Meander traces within their regions to reach target length
3. Use Multi-Scale Dynamic Time Warping (MSDTW) for differential pair
   length matching (handling common decoupled problems)
4. Combine greedy, dynamic programming, and computational geometry

This is the state-of-the-art in automatic serpentine insertion. The region
assignment phase is sequential, but the meandering computation within each
region is independent per trace -- directly parallelizable on GPU.

### 2.5 Real-Time Length Display During Routing

The GPU naturally supports real-time length visualization:

1. After each PathFinder iteration (or even each net route), run the
   `compute_net_length` kernel to update all net lengths
2. Map the length buffer to CPU for the viewer to read
3. Viewer displays a length comparison table (similar to Altium's
   interactive length tuning display)

Since the length computation kernel is trivially fast (~0.1ms for 1000
nets), this adds negligible overhead and provides live feedback during
GPU-accelerated routing.

For the viewer integration, the `RoutingIterationSnapshot` already captures
per-net data. Add a `net_lengths: BTreeMap<NetId, f64>` field to each
snapshot for playback visualization.

---

## 3. Bus Routing on GPU

### 3.1 Problem Statement

Bus routing requires routing multiple nets simultaneously as a group with:
- Consistent ordering (no crossing between bus members)
- Uniform spacing between adjacent members
- Length matching within the group (see Section 2)
- Topology matching (all members follow the same routing topology)

Examples: DDR data buses, parallel address buses, LVDS bus pairs.

### 3.2 Member Ordering to Minimize Crossings

Bus member ordering is an **assignment problem**: given source pins and
destination pins, find the assignment of nets to pin pairs that minimizes
crossings. This is equivalent to finding the minimum-cost permutation.

**Exact solution**: For N nets, there are N! permutations. For N <= 12,
brute-force is feasible on CPU. For larger N, use the **Hungarian algorithm**
(O(N^3)) to find the minimum-cost assignment where cost = number of
crossings.

**GPU acceleration**: The Hungarian algorithm is inherently sequential, but
the **cost matrix computation** (pairwise crossing count) is parallelizable.
For N nets, the cost matrix has N^2 entries, each requiring a crossing test
between two net paths. Launch N^2 threads, each computing one entry:

```wgsl
fn crossing_cost(net_i_path, net_j_path) -> u32 {
    // Count segment-segment intersections between paths
    var crossings: u32 = 0;
    for seg_a in net_i_path:
        for seg_b in net_j_path:
            if segments_cross(seg_a, seg_b): crossings += 1;
    return crossings;
}
```

After the cost matrix is computed on GPU, run the Hungarian algorithm on CPU
(O(N^3) with N typically 8-32 -- fast enough).

### 3.3 Channel Routing on GPU

Channel routing routes multiple nets through a constrained corridor (channel)
between two rows of pins. Classic algorithms (Yoshimura-Kuh, Left-Edge) assign
nets to tracks within the channel.

**Left-Edge Algorithm** (parallelizable):
1. Sort nets by left endpoint
2. Assign each net to the lowest available track where it fits
3. Track occupancy per track

Step 2 is inherently sequential (greedy scan), but can be reformulated as a
**parallel prefix problem**: compute track availability as a function of
position using prefix-max scan on GPU.

**For PCB bus routing**: Channel routing is most useful for routing a bus
through a narrow corridor between components. The channel width (available
tracks) is determined by clearance rules and component placement.

**GPU strategy**: Use channel routing for initial assignment, then refine
with per-net detailed routing (Bellman-Ford) within the channel boundaries.
The channel assignment runs on CPU (small problem); the per-net BF within
channels runs on GPU with the channel boundaries as additional obstacles.

### 3.4 Parallel Multi-Net Routing for Bus Members

Within a bus, the nets share routing resources (same channel) so they
cannot be routed fully independently. However, adjacent nets in the bus
ordering can be routed in a **wavefront** pattern:

1. Route net 0 (outermost)
2. Route nets 1, 2 in parallel (their paths are constrained by net 0
   and the channel boundary -- they don't conflict with each other if
   spacing is sufficient)
3. Route nets 3, 4 in parallel
4. Continue until all nets routed

This gives O(N/2) parallelism for N bus members. On GPU, each parallel
pair gets its own Bellman-Ford dispatch (or batched into one dispatch with
separate distance arrays).

### 3.5 Spacing Preservation

Once the bus is routed, enforce uniform spacing by treating the bus as a
**rigid body** with flexible joints at bends. Each member's path is offset
from the bus centerline by `member_index * spacing`.

**GPU kernel for spacing enforcement**:
```
for each segment in bus_centerline:
    for each member (in parallel):
        offset_segment = parallel_offset(segment, member_index * spacing)
        if !blocked(offset_segment):
            member_path.push(offset_segment)
        else:
            // Flag for local re-routing
```

### 3.6 Topology-Aware Bus Routing (State of the Art)

Zhu et al. (2021) formulate topology-aware bus routing as an **unsplittable
flow problem (UFP)**: all bus bits must follow the same routing topology
through the board. Their algorithm integrates this into negotiation-based
global routing, achieving best scores in the ICCAD CAD Contest on
Obstacle-Aware On-Track Bus Routing.

**GPU mapping**: The UFP formulation produces per-net region constraints
that restrict the Bellman-Ford search space per bus member. These
constraints are naturally expressed as additional obstacle bitmaps per net
-- no algorithmic change to the GPU BF kernel, just different obstacle
data per dispatch.

---

## 4. Impedance-Controlled Routing

### 4.1 Trace Width from Stackup

Impedance-controlled routing adjusts trace width based on the target
impedance and the layer stackup properties:

- **Single-ended**: Z0 is a function of (trace_width, dielectric_thickness,
  dielectric_constant, copper_thickness). Target: typically 50 ohm.
- **Differential**: Zdiff depends on (trace_width, gap, dielectric_thickness,
  Dk, Cu_thickness). Target: typically 90-100 ohm.

The relationship is given by transmission line formulas (microstrip or
stripline depending on layer position):

```
Microstrip (outer layer):
Z0 ~= (87 / sqrt(Er + 1.41)) * ln(5.98 * H / (0.8 * W + T))

Stripline (inner layer):
Z0 ~= (60 / sqrt(Er)) * ln(4 * H / (0.67 * (0.8 * W + T)))

where:
  W = trace width, H = dielectric height to reference plane
  T = copper thickness, Er = dielectric constant
```

**For the router**: Pre-compute a **width-per-layer lookup table** from the
stackup definition. When routing a net with an impedance target, use the
width for the current layer. This table is computed once (CPU) and uploaded
to GPU as a uniform buffer.

```
impedance_width[layer][impedance_class] -> trace_width_mm
```

The GPU BF kernel uses this to determine clearance requirements per cell:
wider traces need more clearance, which translates to inflated obstacles.

### 4.2 Reference Plane Awareness

Every high-speed signal needs a continuous reference plane (ground or power)
on an adjacent layer for return current. Breaks in the reference plane
(splits, voids, cutouts) cause impedance discontinuities.

**GPU approach**: Model reference plane continuity as a **per-cell cost term**
in the Bellman-Ford kernel. Pre-compute a "reference plane quality" map per
signal layer:

```
ref_plane_quality[x][y][signal_layer] =
    if solid_plane_below(x, y): 0           // ideal
    elif split_crossing(x, y): HIGH_PENALTY  // plane split underneath
    elif void(x, y): VERY_HIGH_PENALTY       // no reference plane
```

Upload as a read-only storage buffer. The BF kernel adds this to edge cost:

```
edge_cost += ref_plane_penalty[neighbor] * REF_PLANE_WEIGHT
```

This steers traces away from reference plane discontinuities automatically.

### 4.3 Via Transitions and Impedance Discontinuities

Every via is an impedance discontinuity. The severity depends on:
- Via geometry (drill size, pad size, antipad size)
- Via stub length (portion of via barrel beyond the target layer)
- Whether the via transitions between different reference planes

**Cost model for GPU**:

```
via_impedance_cost(from_layer, to_layer, via_type) =
    BASE_VIA_COST
    + stub_penalty(via_type, from_layer, to_layer)
    + reference_plane_transition_penalty(from_layer, to_layer)
```

The `stub_penalty` is highest for through-hole vias used on inner layers
(long stubs act as resonant antennas). Back-drilled or blind/buried vias
have lower stub penalties.

The `reference_plane_transition_penalty` applies when the signal's
reference plane changes at the via (e.g., from GND on L2 to PWR on L3).
The return current must find a path between the planes, typically through
decoupling capacitors. This penalty is pre-computed from the stackup.

### 4.4 Impedance as a GPU Cost Function Term

Combining all impedance factors into the BF edge cost:

```
edge_cost(src, dst, net_class) =
    base_distance_cost
    + history_cost * pres_fac
    + gap_penalty(dst)              // differential pair gap (Section 1)
    + ref_plane_penalty(dst)        // reference plane continuity
    + width_clearance_penalty(dst)  // insufficient clearance for required width
    + via_impedance_cost(...)       // if this is a via transition
```

All terms are pre-computed and stored in GPU buffers. The BF kernel is
unchanged -- it just reads richer cost data per cell. This is the key
advantage of the cost-function approach: **new routing constraints add
data, not code**.

### 4.5 Layer-Specific Trace Width Handling

When a net transitions between layers via a via, the trace width may
change (different dielectric thickness -> different required width for
same impedance). The GPU grid must handle variable-width traces:

**Option A: Width-aware obstacle inflation**. Inflate obstacles differently
per net class per layer. Pre-compute `obstacle_bitmap[layer][width_class]`
and select the right bitmap in the BF kernel based on net class.

**Option B: Width as additional state dimension**. Add width to the search
node: `(x, y, layer, width)`. This multiplies the state space by the
number of distinct width values (typically 2-4). Manageable on GPU.

**Recommendation**: Option A (width-aware inflation) for simplicity. The
number of distinct impedance classes is small (2-5 typically), so
pre-computing a few obstacle bitmaps per layer is feasible.

---

## 5. DDR-Specific Routing

### 5.1 DDR Memory Topology

DDR memory (DDR3/DDR4/DDR5) uses **fly-by topology** for clock, address,
and command signals: signals start at the controller and daisy-chain
through each DRAM in series. Data signals (DQ, DQS, DM) connect
point-to-point between controller byte lanes and DRAM byte lanes.

Signal groups:
- **Clock (CK/CK#)**: Differential pair, fly-by topology
- **Address/Command (A0-A13, BA0-BA1, RAS#, CAS#, WE#)**: Single-ended,
  fly-by topology, length-matched to CK
- **Data (DQ0-DQ7)**: Single-ended, point-to-point per byte lane
- **Data Strobe (DQS/DQS#)**: Differential pair, one per byte lane
- **Data Mask (DM)**: Single-ended, one per byte lane

### 5.2 DDR Routing Constraints

Constraints from JEDEC specifications and vendor datasheets (AMD/Xilinx
UG583, Intel FPGA guidelines):

#### DDR4 Constraints (typical, from AMD UG583 Table 2-17/2-18)

| Signal Group | Matching Requirement | Skew Tolerance |
|---|---|---|
| DQ within byte lane | Match to DQS | +/- 10 ps (~1.5 mm) |
| DQS to DQ (same lane) | Match to DQS | +/- 10 ps |
| DM to DQS (same lane) | Match to DQS | +/- 10 ps |
| Address/Command to CK | Match to CK | +/- 8 ps (~1.2 mm) |
| CK to CK# (diff pair) | Intra-pair match | +/- 2 ps (~0.3 mm) |
| DQS to DQS# (diff pair) | Intra-pair match | +/- 2 ps (~0.3 mm) |
| Byte lane to byte lane | **Not required** | N/A |

Note: ps-to-mm conversion assumes Dk ~= 4 (FR-4), propagation delay
~6.4 ps/mm. Actual conversion depends on substrate material.

#### DDR5 Additional Constraints

DDR5 adds tighter constraints and new features:
- Higher data rates require tighter length matching
- Maximum of 2 transition vias per signal
- Differential data strobe only (no single-ended DQS option)
- Write-leveling compensates for CK-to-DQS skew on PCB

### 5.3 T-Topology (Fly-By) for Address/Command

Fly-by topology routes address/command signals as a single trace that
passes each DRAM in sequence:

```
Controller --[A0]--> DRAM0 --[A0]--> DRAM1 --[A0]--> DRAM2 --> ...
```

The trace must visit DRAMs in a specific order with short stubs from the
main trunk to each DRAM's pin.

**GPU routing approach**: Route fly-by signals as a **Steiner tree**
problem with ordered terminals. Decompose into ordered 2-pin subnets:

1. Controller -> DRAM0 (first segment)
2. DRAM0 -> DRAM1 (second segment)
3. DRAM1 -> DRAM2 (third segment)
4. ...

Each segment is a standard 2-pin route (GPU BF). The ordering constraint
means segments must be routed sequentially (each segment starts where the
previous one ends). However, multiple fly-by signals (A0, A1, ..., A13)
that share the same source-destination ordering can be routed as a **bus**
(Section 3), gaining inter-net parallelism.

### 5.4 Byte-Lane Grouping and Length Matching

Each DDR byte lane is an independent length-matching group:

```
Byte Lane 0: DQ0, DQ1, DQ2, DQ3, DQ4, DQ5, DQ6, DQ7, DQS0, DQS0#, DM0
Byte Lane 1: DQ8, DQ9, DQ10, DQ11, DQ12, DQ13, DQ14, DQ15, DQS1, DQS1#, DM1
...
```

Within each byte lane, all signals must be length-matched to the strobe
(DQS). **Between byte lanes, no matching is required** -- this is a
crucial constraint that enables parallelism.

**GPU strategy**: Route each byte lane as an independent routing partition.
Since byte lanes don't share routing resources (they connect to different
DRAM pins in different board regions), all byte lanes can be routed
**simultaneously** on GPU with zero conflict:

```
// Route all byte lanes in parallel
for lane in byte_lanes:  // parallel on GPU
    route_bus(lane.signals, lane.constraints)
    length_match(lane.signals, target=max_length(lane))
```

This is where the LLM-authored spec shines: the LLM declares byte lanes
as `independent_groups` in the spec, and the GPU router exploits this
declaration for maximum parallelism without runtime conflict detection.

### 5.5 DQ/DQS Matching Requirements

Within a byte lane, DQ signals must match to DQS (not to each other
directly, though matching to DQS transitively matches them). The matching
target is the DQS length (or the longest signal in the group).

**Algorithm**:
1. Route DQS first (it's the reference signal)
2. Route all DQ signals in parallel (they're in the same byte lane but
   connect different pin pairs -- mostly non-conflicting)
3. Compute length deficits: `deficit[DQi] = length(DQS) - length(DQi)`
4. Insert serpentine on each DQ to eliminate deficit
5. Route DM with same matching target

Step 2 is the main GPU win: 8 DQ routes per byte lane, all in parallel.
Step 3 is the parallel reduction from Section 2.2. Step 4 uses the
serpentine insertion from Section 2.4.

---

## 6. What the LLM Can Pre-Compute

The spec language is LLM-authored, so the LLM can pre-compute and declare
detailed constraints that traditional autorouters must discover at runtime.
This section describes what the LLM should generate for high-speed routing.

### 6.1 Signal Integrity Pre-Analysis from Schematic

The LLM can analyze the schematic and component datasheets to determine:

- **Impedance targets**: Based on interface standard (USB = 90 ohm diff,
  HDMI = 100 ohm diff, DDR4 = 40 ohm single-ended / 80 ohm diff)
- **Timing budgets**: From protocol specs (DDR4 data setup/hold, USB
  eye diagram requirements)
- **Driver/receiver characteristics**: Output impedance, input capacitance
  (affects impedance target calculation)

**Spec output**:
```
net_class "DDR4_DQ" {
    impedance_single_ended: 40ohm +/- 10%
    max_length: 150mm
    max_via_transitions: 2
    preferred_layers: [L3, L6]   // stripline layers
}
```

### 6.2 Constraint Generation from Component Datasheets

The LLM reads DDR timing specifications and generates length constraints:

**Input** (from DDR4 DRAM datasheet):
```
tDQSQ (DQS-DQ skew): max 120ps
tDQSS (DQS-CK skew): -0.27 to +0.27 tCK
tIS (input setup, address): 95ps
tIH (input hold, address): 120ps
```

**LLM-generated spec constraints**:
```
// Assuming Dk=4.2, prop_delay=6.5ps/mm
// tDQSQ=120ps -> max skew = 120ps / 6.5ps/mm = 18.5mm
match_group "byte_lane_0" {
    reference: NET_DQS0
    members: [NET_DQ0, NET_DQ1, ..., NET_DQ7, NET_DM0]
    max_skew: 18mm    // from tDQSQ with margin
    tolerance: 1mm
}

diff_pair "DQS0" {
    positive: NET_DQS0_P
    negative: NET_DQS0_N
    gap: 5mil
    max_intra_pair_skew: 2ps   // ~0.3mm
}
```

### 6.3 Automatic Differential Pair Identification

The LLM identifies differential pairs from schematic net names using
naming conventions:

| Convention | Example | Detection Rule |
|---|---|---|
| `_P` / `_N` suffix | `USB_D_P`, `USB_D_N` | Strip suffix, match base names |
| `+` / `-` suffix | `CLK+`, `CLK-` | Strip suffix, match base names |
| `_T` / `_C` (true/complement) | `DQS0_T`, `DQS0_C` | Strip suffix, match base names |
| Schematic differential pair markers | -- | Read from schematic metadata |

**Spec output**:
```
diff_pair "USB_D" {
    positive: USB_D_P
    negative: USB_D_N
    impedance: 90ohm
    gap: 7mil
}
```

This is trivial for the LLM but tedious for humans. The LLM can also
validate that the identified pairs make electrical sense (same source IC,
complementary pins, known differential interface).

### 6.4 Bus Group Inference from Naming Conventions

The LLM groups bus signals by name pattern:

```
DDR_DQ[0..15]  -> 2 byte lanes of 8
DDR_A[0..13]   -> address bus
DDR_BA[0..1]   -> bank address bus
SPI_D[0..3]    -> SPI data bus
```

**Spec output**:
```
bus "DDR_DATA_LANE0" {
    members: [DDR_DQ0, DDR_DQ1, DDR_DQ2, DDR_DQ3,
              DDR_DQ4, DDR_DQ5, DDR_DQ6, DDR_DQ7]
    strobe: DDR_DQS0
    mask: DDR_DM0
    match_to: DDR_DQS0
    max_skew: 18mm
}

bus "DDR_ADDR" {
    members: [DDR_A0, DDR_A1, ..., DDR_A13]
    topology: fly_by
    visit_order: [U_DRAM0, U_DRAM1]
    match_to: DDR_CK
    max_skew: 12mm
}

independent_groups [
    ["DDR_DATA_LANE0", "DDR_DATA_LANE1"],  // byte lanes are independent
]
```

### 6.5 Impact on GPU Router

With LLM-pre-computed constraints, the GPU router receives:

| What | Before (traditional) | After (LLM spec) |
|---|---|---|
| Diff pair identification | Runtime net-name parsing | Declared in spec |
| Bus grouping | Runtime heuristic | Declared in spec |
| Length targets | Computed after routing | Pre-computed from datasheets |
| Byte-lane independence | Must analyze connectivity | Declared as `independent_groups` |
| Impedance targets | Must look up design rules | Pre-computed per net class |
| Fly-by visit order | Must analyze placement | Declared with `visit_order` |
| Serpentine budget | Unknown until routing | Pre-computed from timing budget |

The GPU router skips all discovery work and goes straight to parallel
execution on pre-partitioned, pre-constrained work. This is the
fundamental advantage of the LLM-authored spec approach.

---

## 7. Proposed Architecture

### 7.1 High-Speed Routing Pipeline

```
LLM Spec Declarations
    |
    v
RoutingConfig + HighSpeedConfig
    |
    +-- DiffPairConfig[]        (gap, impedance, max_skew)
    +-- MatchGroup[]            (nets, reference, tolerance)
    +-- BusGroup[]              (members, ordering, topology)
    +-- ImpedanceConfig[]       (per net class, per layer widths)
    +-- IndependenceDecl[]      (groups that can route in parallel)
    |
    v
High-Speed Routing Pipeline (CPU orchestration, GPU execution)
    |
    +-- Phase 1: Impedance setup
    |     Pre-compute width-per-layer tables
    |     Generate width-aware obstacle bitmaps
    |     Compute reference plane quality maps
    |     [CPU: table lookups | GPU: bitmap generation]
    |
    +-- Phase 2: Bus ordering
    |     Compute crossing cost matrices (GPU: N^2 threads per bus)
    |     Run Hungarian assignment (CPU: O(N^3))
    |     [GPU: cost matrix | CPU: assignment]
    |
    +-- Phase 3: Parallel routing of independent groups
    |     For each independent group (in parallel on GPU):
    |       Route diff pairs (coupled BF or sequential P-then-N)
    |       Route bus members (channel routing + per-net BF)
    |       Route single-ended signals (standard BF)
    |     [GPU: all BF routing | CPU: orchestration]
    |
    +-- Phase 4: Length matching
    |     Compute net lengths (GPU: parallel reduction)
    |     Compute deficits per match group (CPU: comparison)
    |     Insert serpentine patterns (GPU: geometry generation)
    |     Validate clearance (GPU: batch DRC)
    |     [GPU: computation | CPU: region selection]
    |
    +-- Phase 5: Validation
    |     Check all skew constraints
    |     Check all impedance constraints
    |     Check all spacing constraints
    |     Report violations
    |     [GPU: batch checking | CPU: report generation]
    |
    v
RouteSolution + HighSpeedReport
```

### 7.2 GPU Buffer Layout for High-Speed Routing

In addition to the base routing buffers (obstacles, dist, history,
predecessor from `00-overview.md`), high-speed routing adds:

```
Buffer 4: ref_plane_quality    (u32 per cell per signal layer, read-only)
    Penalty for missing/broken reference plane underneath signal trace

Buffer 5: impedance_width_lut  (f32[MAX_LAYERS][MAX_IMPEDANCE_CLASSES])
    Trace width in mm for each (layer, impedance_class) combination

Buffer 6: diff_pair_state      (per coupled-node state for active diff pair)
    dist_coupled[x][y][layer][dir], dist_uncoupled[local window]

Buffer 7: net_lengths          (f32 per net, read-write)
    Updated by parallel reduction kernel, read by length-matching logic

Buffer 8: serpentine_params    (per-segment meander parameters)
    amplitude, pitch, count for each segment needing length compensation
```

### 7.3 Compute Shader Pipeline for High-Speed Routing

```
Per PathFinder iteration:
  -- Standard routing (from 00-overview.md) --
  1. history_update.wgsl
  2. For each net batch:
     a. reset_dist.wgsl
     b. bellman_ford.wgsl (or coupled_bellman_ford.wgsl for diff pairs)
     c. CPU: reconstruct path

  -- High-speed additions --
  3. compute_net_lengths.wgsl     -- parallel reduction per net
  4. CPU: compute length deficits per match group
  5. compute_serpentine.wgsl      -- geometry generation for meanders
  6. validate_clearance.wgsl      -- batch DRC on serpentine segments
  7. CPU: read back violations, iterate if needed
```

### 7.4 Integration with Router Milestones

From the router plan (`docs/plans/router/README.md`):

- **Milestone 8** (Trace Optimization + High-Speed) already plans:
  - `high_speed/diff_pair.rs` -- Differential pair routing
  - `high_speed/bus.rs` -- Bus routing
  - `optimize/serpentine.rs` -- Serpentine insertion

The GPU acceleration slots in as an optional backend:

```
DiffPairRouter trait
    +-- CpuDiffPairRouter     (sequential P-then-N, CPU A*)    [M8]
    +-- GpuCoupledRouter      (coupled BF on GPU)              [future]

LengthMatcher trait
    +-- CpuLengthMatcher      (sequential serpentine insertion) [M8]
    +-- GpuLengthMatcher      (parallel reduction + geometry)  [future]

BusRouter trait
    +-- CpuBusRouter          (sequential channel routing)     [M8]
    +-- GpuBusRouter          (parallel member routing)        [future]
```

The CPU implementations come first (M8). GPU backends share the same
trait interface and are selected via `RoutingConfig.gpu_acceleration`.

### 7.5 When GPU High-Speed Routing is Worth It

| Scenario | CPU | GPU | Winner |
|---|---|---|---|
| 1-2 diff pairs | < 1s | Overhead dominates | CPU |
| 8+ diff pairs (DDR byte lanes) | 5-30s | 0.5-3s | GPU |
| 32-bit DDR bus (40+ nets) | 30-120s | 2-10s | GPU |
| Length matching 4 nets | < 1s | Overhead | CPU |
| Length matching 40+ nets | 5-20s | 0.5-2s | GPU |
| Single bus (8 members) | 2-5s | 0.5-1s | Marginal |
| Multiple independent buses | 10-60s | 1-5s | GPU |

The GPU wins when there are **many independent routing tasks** (byte lanes,
independent buses) or **many nets to length-match simultaneously**. For
small designs with a few diff pairs, CPU is faster due to GPU dispatch
overhead.

---

## 8. References

### Papers: Differential Pair Routing

| Paper | Year | Contribution |
|---|---|---|
| [A Unified PCB Routing Algorithm with Complicated Constraints and Differential Pairs](https://dl.acm.org/doi/10.1145/3394885.3431568) | 2021 | Full-board routing with diff pair support using triangular grid + max flow. ASPDAC. |
| [Length-Constrained Escape Routing of Differential Pairs](https://www.sciencedirect.com/science/article/abs/pii/S0167926014000571) | 2014 | Two-phase escape routing with length matching for diff pairs. |
| [Modelling and Optimisation Algorithm for Length-Matching Escape Routing of Differential Pairs](https://ietresearch.onlinelibrary.wiley.com/doi/full/10.1049/el.2019.0522) | 2019 | One-stage optimization for simultaneous median-point and path computation. |
| [An Improved MCTS Algorithm for Ordered Escape Routing of Differential Pair](https://dl.acm.org/doi/10.1145/3760776) | 2024 | Monte Carlo Tree Search for ordered diff pair escape routing. |
| [Ordered Escape Routing with Consideration of Differential Pair and Blockage](https://dl.acm.org/doi/10.1145/3185783) | 2018 | MMCF approach for ordered escape routing. ACM TODAES. |
| [Two-Stage Ordered Escape Routing Combined with LP and Heuristic Algorithm for Large-Scale PCB](https://www.sciencedirect.com/science/article/abs/pii/S0167926024001342) | 2024 | LP + heuristic for large-scale PCB ordered escape routing. |

### Papers: Length Matching

| Paper | Year | Contribution |
|---|---|---|
| [Obstacle-Aware Length-Matching Routing for Any-Direction Traces in PCB](https://arxiv.org/abs/2407.19195) | 2024 | First any-direction length matching. MSDTW for diff pairs. DAC 2024. |
| [Obstacle-Aware Length-Matching Bus Routing](https://www.researchgate.net/publication/220915557_Obstacle-aware_length-matching_bus_routing) | 2011 | Early work on obstacle-aware bus length matching. |

### Papers: Bus Routing

| Paper | Year | Contribution |
|---|---|---|
| [Topology-Aware Bus Routing in Complex Networks of VLSI](https://onlinelibrary.wiley.com/doi/10.1155/2021/8843271) | 2021 | UFP formulation for topology-matching bus routing. ICCAD contest winner. |
| [A DAG-Based Algorithm for Obstacle-Aware Topology-Matching](https://baloneymath.github.io/files/DAC19_bus.pdf) | 2019 | DAG-based bus routing with topology constraints. DAC. |
| [A Parallel Algorithm for Channel Routing Problems (VLSI)](https://ieeexplore.ieee.org/document/125094/) | 1990 | Classic parallel channel routing on multilayer channels. |
| [Channel Routing Problems](https://link.springer.com/chapter/10.1007/978-1-4615-3642-0_5) | -- | Comprehensive overview of channel routing algorithms. |

### Papers: PCB Routing (General, Recent)

| Paper | Year | Contribution |
|---|---|---|
| [PCB Routing on Unstructured Meshes with Conflict-Based Search](https://link.springer.com/article/10.1007/s11227-025-07569-0) | 2025 | MAPF-inspired CBS on Delaunay grids for PCB routing. J. Supercomputing. |
| [Multi-Agent Based Minimal-Layer Via Routing for PCB Design](https://www.sciencedirect.com/science/article/abs/pii/S0167926025001907) | 2025 | Multi-agent routing minimizing via count. |

### Papers: GPU Parallel Primitives

| Paper | Year | Contribution |
|---|---|---|
| [Prefix Sum on Vulkan (Raph Levien)](https://raphlinus.github.io/gpu/2020/04/30/prefix-sum.html) | 2020 | Practical prefix sum on GPU compute shaders. |
| [GPUPrefixSums](https://github.com/b0nes164/GPUPrefixSums) | -- | Collection of prefix sum algorithms in WGPU, CUDA, D3D12. |
| [Reduce and Scan (Modern GPU)](https://moderngpu.github.io/scan.html) | -- | Reference implementations of parallel reduce/scan. |

### DDR Design Guides

| Document | Source | Content |
|---|---|---|
| [DDR4 SDRAM Routing Constraints (UG583)](https://docs.amd.com/r/en-US/ug583-ultrascale-pcb-design/DDR4-SDRAM-Routing-Constraints) | AMD/Xilinx | DDR4 skew tables, routing topology, constraint values. |
| [DDR5 Routing Guidelines](https://www.intel.com/content/www/us/en/docs/programmable/772538/24-1-6-1-0/routing-guidelines-for-ddr5-memory-down.html) | Intel FPGA | DDR5 routing constraints for Intel Agilex. |
| [DDR4 Routing Guidelines (Altium)](https://resources.altium.com/p/pcb-routing-guidelines-ddr4-memory-devices) | Altium | Practical DDR4 routing with impedance and length matching. |
| [Fly-By Topology for DDR3/DDR4 (Altium)](https://resources.altium.com/p/fly-topology-routing-ddr3-and-ddr4-memory) | Altium | Fly-by routing guidelines with T-topology details. |
| [DDR Timing Constraints with Allegro X](https://resources.pcb.cadence.com/blog/ddr-timing-constraints-with-allegro-x-cadence) | Cadence | DDR constraint setup in Allegro. |
| [Hardware and Layout Design Considerations for DDR (AN2582)](https://www.nxp.com/docs/en/application-note/AN2582.pdf) | NXP | General DDR layout guidelines with constraint tables. |
| [Field Guide to DDR Signal Integrity Analysis](https://www.ema-eda.com/wp-content/uploads/2024/01/Field-Guide-to-DDR-Signal-Integrity-Analysis.pdf) | EMA Design | Comprehensive DDR SI analysis guide. |

### Commercial Tool Documentation

| Document | Source | Content |
|---|---|---|
| [Differential Pair Routing (Altium)](https://www.altium.com/documentation/altium-designer/pcb/high-speed-design/interactively-routing-differential-pairs) | Altium | Coupled routing, gap tolerance, skew tracking. |
| [Controlled Impedance Routing (Altium)](https://www.altium.com/documentation/altium-designer/pcb/high-speed-design/interactively-routing-controlled-impedance) | Altium | Impedance-controlled routing implementation. |
| [Impedance Management Through Stackup Design (Altium)](https://resources.altium.com/p/impedance-management-through-pcb-stackup-design-reference-planes) | Altium | Stackup-driven impedance management. |
| [Length Matching: Trombone, Accordion, Sawtooth (Altium)](https://resources.altium.com/p/length-matching-high-speed-signals-trombone-accordion-and-sawtooth-tuning) | Altium | Length tuning patterns and guidelines. |
| [Switchback vs Serpentine Routing (Altium)](https://resources.altium.com/p/switchback-routing-vs-serpentine-routing-maximum-density) | Altium | Comparison of tuning pattern density. |

### Open-Source Implementations

| Project | Description |
|---|---|
| [FreeRouting](https://github.com/freerouting/freerouting) | Open-source PCB autorouter (Java). Diff pair support is partial (issue #358). Uses Specctra DSN format. |
| [OrthoRoute](https://github.com/bbenchoff/OrthoRoute) | GPU-accelerated PCB autorouter (Python/CuPy/KiCad). No diff pair or length matching support yet. |
| [PCB Auto Router](https://www.pcbautorouter.top/) | GPU-accelerated commercial autorouter. Claims diff pair and serpentine support. Closed source. |
| [GPUPrefixSums](https://github.com/b0nes164/GPUPrefixSums) | WGPU prefix sum implementations (needed for parallel length computation). |
