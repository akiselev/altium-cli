# Routability Metrics Implementation Specification

Self-contained implementation reference for routability metrics used across the
PCB autoplacer pipeline. An implementing agent needs no other document to build
these modules.

**Target crate**: `crates/autopcb-placement/src/routability.rs`
**Dependencies on**: `autopcb-ir` (for `PcbIr`, `IrNet`, `IrComponent`, `ComponentId`,
`NetId`, `PadId`), `std` only — no external numeric libraries required.


---

## 1. Overview

Three metrics are computed in this module:

| Metric | Phase used in | Purpose |
|--------|--------------|---------|
| **HPWL** (Half-Perimeter Wire Length) | Phase 1 (smooth), Phase 3 (exact) | Primary optimization objective |
| **Net crossing count** | Phase 3 (SA cost) | PCB routability proxy — fewer crossings means fewer vias needed on limited-layer boards |
| **Grid-based congestion** | Phase 3 (optional penalty) | Identifies routing bottlenecks, enables area-specific penalty |

These three metrics feed into a unified `CostBreakdown` struct consumed by the
SA engine in Phase 3 and by evaluation hooks in Phase 1 (HPWL only).

**Coordinate system**: all values in millimetres (`f64`). Conversion from Altium
internal units (10,000 per mil, 100,000 per mm) happens at the `autopcb-ir`
extraction boundary — this module never touches raw `Coord` integers.


---

## 2. Core Types

Define these types at the top of `routability.rs` before any function definitions.

```rust
/// World-space point in millimetres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

/// Axis-aligned bounding box in millimetres.
#[derive(Debug, Clone, Copy)]
pub struct BBox {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl BBox {
    pub fn width(&self) -> f64  { self.max_x - self.min_x }
    pub fn height(&self) -> f64 { self.max_y - self.min_y }
    pub fn hpwl(&self) -> f64   { self.width() + self.height() }

    /// Expand the box to include a point.
    pub fn expand(&mut self, p: Point) {
        if p.x < self.min_x { self.min_x = p.x; }
        if p.x > self.max_x { self.max_x = p.x; }
        if p.y < self.min_y { self.min_y = p.y; }
        if p.y > self.max_y { self.max_y = p.y; }
    }

    /// Construct from an initial point (zero-area box).
    pub fn from_point(p: Point) -> Self {
        BBox { min_x: p.x, max_x: p.x, min_y: p.y, max_y: p.y }
    }
}

/// A directed or undirected line segment in world space.
#[derive(Debug, Clone, Copy)]
pub struct Segment {
    pub a: Point,
    pub b: Point,
    /// Which net this segment belongs to (used when filtering same-net crossings).
    pub net_id: NetId,
}

/// Cached per-net routing state. Updated incrementally during SA.
pub struct NetState {
    pub id: NetId,
    /// Pin designators participating in this net (stable identifiers).
    pub pins: Vec<PadId>,
    /// Current world positions of each pin, parallel to `pins`.
    pub pin_positions: Vec<Point>,
    /// Axis-aligned bounding box of all pin positions.
    pub bounds: BBox,
    /// Exact HPWL: bounds.width() + bounds.height().
    pub hpwl: f64,
    /// MST edge decomposition: list of (from_pin_idx, to_pin_idx, segment).
    pub segments: Vec<Segment>,
    /// Crossing count contributed by this net's segments vs all others.
    /// Maintained incrementally during SA.
    pub crossing_count: usize,
    /// Set to true when a component move has invalidated cached values.
    pub dirty: bool,
}

/// Bidirectional index between components and the nets they participate in.
pub struct NetComponentIndex {
    /// net_id → list of (component_id, local pad indices within that component).
    pub net_to_comps: HashMap<NetId, Vec<(ComponentId, Vec<usize>)>>,
    /// comp_id → list of net IDs that contain at least one pad from this component.
    pub comp_to_nets: HashMap<ComponentId, Vec<NetId>>,
}

/// Grid-based routing demand/capacity model (RUDY-style).
pub struct CongestionGrid {
    /// Routing demand per cell (horizontal-edge demand + vertical-edge demand).
    /// Indexed as `cells[row * cols + col]`.
    pub cells: Vec<f64>,
    pub rows: usize,
    pub cols: usize,
    pub cell_size: f64,      // mm per cell edge
    pub origin: Point,       // world coordinate of cell (0,0) corner
    /// Number of cells where demand exceeds capacity.
    pub overflow_count: usize,
    /// Per-cell routing capacity (routing_layers × cell_size).
    pub capacity: f64,
}

/// Full cost breakdown returned after each SA cost evaluation.
#[derive(Debug, Clone, Copy, Default)]
pub struct CostBreakdown {
    pub hpwl: f64,
    pub overlap: f64,
    pub constraint: f64,
    pub crossings: f64,
    pub congestion: f64,
    pub total: f64,
}
```


---

## 3. HPWL Computation

### 3.1 Exact HPWL (Phase 3 — Simulated Annealing)

SA does not need gradients. Use Manhattan HPWL directly: the half-perimeter of the
bounding box of all pin world positions for each net.

```
HPWL(net) = (max_x − min_x) + (max_y − min_y)
```

Complexity: O(|pins|) per net, O(Σ|pins|) = O(E) over all nets where E is total
pin count across all nets.

```rust
/// Compute exact HPWL for all nets given the current placement.
///
/// `placement` maps ComponentId to current (x, y) in mm.
/// Returns total HPWL in mm.
pub fn compute_hpwl_exact(
    nets: &[NetState],
) -> f64 {
    nets.iter()
        .filter(|n| n.pins.len() >= 2)
        .map(|n| n.hpwl)
        .sum()
}

/// Recompute the bounding box and HPWL for a single net from its pin positions.
/// Call this after updating `net_state.pin_positions`.
pub fn refresh_net_hpwl(net_state: &mut NetState) {
    let mut it = net_state.pin_positions.iter();
    let first = match it.next() {
        Some(p) => *p,
        None => { net_state.hpwl = 0.0; return; }
    };
    let mut bounds = BBox::from_point(first);
    for p in it {
        bounds.expand(*p);
    }
    net_state.bounds = bounds;
    net_state.hpwl = bounds.hpwl();
}
```

**Incremental path for SA**: When component `c` moves, only the nets connected to `c`
need their `pin_positions` and `hpwl` refreshed. Use `NetComponentIndex::comp_to_nets`
to enumerate those nets. See Section 5 for the full incremental update protocol.

### 3.2 Smooth HPWL (Phase 1 — Analytical Solver / Solverang)

The analytical solver requires a differentiable approximation to HPWL because the
true max/min are not differentiable everywhere.

**Log-Sum-Exp (LSE) approximation:**

```
smooth_max(v₁, …, vₙ; γ) = (1/γ) · ln( Σᵢ exp(γ · vᵢ) )
smooth_min(v₁, …, vₙ; γ) = −smooth_max(−v₁, …, −vₙ; γ)

smooth_HPWL(net; γ) = smooth_max(x_pins; γ) − smooth_min(x_pins; γ)
                    + smooth_max(y_pins; γ) − smooth_min(y_pins; γ)
```

**Gradient** (required for Jacobian in solverang constraint):

```
∂smooth_max/∂xᵢ = exp(γ · xᵢ) / Σⱼ exp(γ · xⱼ)   (softmax weights)
```

The gradient is fully differentiable everywhere. As γ → ∞, the approximation
converges to exact HPWL; as γ → 0, it converges to a constant (mean of values).

**Implementation note**: This is already implemented as `SmoothHpwlConstraint` in
`crates/autopcb-placement/src/constraints.rs`. The routability module does not
re-implement smooth HPWL — it only implements exact HPWL for SA cost evaluation.
Cross-reference that file for the full solverang `Constraint` impl.

### 3.3 Adaptive Gamma Schedule

Gamma controls the sharpness of the smooth approximation during Phase 1:

| Schedule step | γ value | Effect |
|--------------|---------|--------|
| Start (iter 0) | 2.0 | Very smooth — helps escape poor initial placements; gradients are gentle |
| Middle (iter N/2) | 5.0–7.0 | Intermediate accuracy |
| End (iter N) | 10.0 | Near-accurate HPWL; gradients sharper near extremal pins |

**Current implementation**: Two-phase jump: γ = 2.0 for the first half of iterations,
γ = 10.0 for the second half. Implemented in the solver loop, not in this module.

**Future enhancement**: Smooth 4-step schedule `[2.0, 5.0, 7.0, 10.0]` applied at
iteration quartiles. This avoids the abrupt gradient change at the midpoint and
empirically improves convergence for large nets (> 10 pins).


---

## 4. Net Crossing Detection

### 4.1 Why Net Crossings Matter for PCB

On a 2-layer PCB, crossing net segments require at least one via to route. Vias
cost board area, impedance discontinuities, and manufacturing complexity. On dense
boards with 4–6 layers, crossings drive layer assignment difficulty. HPWL alone
does not capture this: two placements with identical HPWL can differ by 10× in
crossing count.

**Cypress (ISPD 2025)** showed that minimizing net crossings alongside HPWL gives
1–5.9× higher routability vs HPWL-only placement on real PCB benchmarks.

### 4.2 Multi-Pin Net Decomposition

A net with n ≥ 3 pins must be decomposed into 2-pin segments before crossing tests.
Three strategies with different accuracy/cost tradeoffs:

| Strategy | Segments per net | Accuracy | Complexity | Recommendation |
|----------|-----------------|----------|------------|----------------|
| **MST** | n−1 | High (realistic topology) | O(n²) per net | Use this |
| **Star** | n | Medium (overcounts ~2×) | O(n) per net | Acceptable for n > 20 |
| **Complete graph** | n(n−1)/2 | Overcounts badly | O(n²) per net | Avoid |
| Steiner tree | < n−1 (optimal) | Highest | NP-hard (FLUTE approx.) | Future |

**Recommendation**: Use MST for all nets. At PCB scale (typical net fan-out 2–8 pins),
MST is fast enough and gives the most realistic crossing prediction.

**MST algorithm (Prim's, Manhattan distance):**

```
Input: pin_positions: [(f64, f64)]  (n points)
Output: mst_edges: Vec<(usize, usize)>  (n-1 edges as index pairs)

mst_prim(pins):
    n = pins.len()
    if n <= 1: return []

    in_mst = [false; n]
    min_dist = [f64::MAX; n]   // cheapest edge into MST for each out-node
    parent = [0usize; n]       // which MST node each out-node connects to

    min_dist[0] = 0.0
    edges = []

    repeat n times:
        // Pick cheapest out-node
        u = argmin over i where !in_mst[i]: min_dist[i]
        in_mst[u] = true

        if u != 0:  // not the seed
            edges.push((parent[u], u))

        // Update neighbours
        for v in 0..n where !in_mst[v]:
            d = manhattan_dist(pins[u], pins[v])
            if d < min_dist[v]:
                min_dist[v] = d
                parent[v] = u

    return edges
```

Where `manhattan_dist((x1,y1), (x2,y2)) = |x1−x2| + |y1−y2|`.

```rust
pub fn decompose_net_mst(pin_positions: &[Point]) -> Vec<Segment> {
    let n = pin_positions.len();
    if n < 2 { return vec![]; }

    let mut in_mst = vec![false; n];
    let mut min_dist = vec![f64::MAX; n];
    let mut parent = vec![0usize; n];
    min_dist[0] = 0.0;

    let mut edges: Vec<Segment> = Vec::with_capacity(n - 1);

    for _ in 0..n {
        // Pick cheapest out-node
        let u = (0..n)
            .filter(|&i| !in_mst[i])
            .min_by(|&a, &b| min_dist[a].partial_cmp(&min_dist[b]).unwrap())
            .unwrap();
        in_mst[u] = true;

        if u != 0 || min_dist[u] < f64::MAX {
            if edges.len() < n - 1 {
                // Segment is stored with a placeholder NetId filled in by caller
                edges.push(Segment {
                    a: pin_positions[parent[u]],
                    b: pin_positions[u],
                    net_id: NetId::INVALID,  // caller fills this in
                });
            }
        }

        for v in 0..n {
            if in_mst[v] { continue; }
            let p = pin_positions[u];
            let q = pin_positions[v];
            let d = (p.x - q.x).abs() + (p.y - q.y).abs();
            if d < min_dist[v] {
                min_dist[v] = d;
                parent[v] = u;
            }
        }
    }

    edges
}
```

**Caller responsibility**: After calling `decompose_net_mst`, fill in `net_id` on
each returned segment before adding it to the global segment list. The MST function
is kept net-agnostic to allow testing in isolation.

### 4.3 Segment Intersection Test

Use the classic CCW (counter-clockwise) orientation test. This handles the general
case (proper intersection) and collinear cases via the boundary condition.

```rust
/// Sign of the cross product (q−p) × (r−p).
/// Returns >0 if CCW, <0 if CW, 0 if collinear.
fn cross_sign(p: Point, q: Point, r: Point) -> f64 {
    (q.x - p.x) * (r.y - p.y) - (q.y - p.y) * (r.x - p.x)
}

/// True if point r lies on segment [p, q] (collinear case).
fn on_segment(p: Point, q: Point, r: Point) -> bool {
    r.x <= p.x.max(q.x) && r.x >= p.x.min(q.x)
        && r.y <= p.y.max(q.y) && r.y >= p.y.min(q.y)
}

/// Returns true if segments (a1→a2) and (b1→b2) properly intersect.
///
/// Segments sharing an endpoint are NOT counted as intersecting (they
/// belong to the same net tree and share a pin position). Collinear
/// overlapping segments ARE counted.
pub fn segments_intersect(a1: Point, a2: Point, b1: Point, b2: Point) -> bool {
    let d1 = cross_sign(b1, b2, a1);
    let d2 = cross_sign(b1, b2, a2);
    let d3 = cross_sign(a1, a2, b1);
    let d4 = cross_sign(a1, a2, b2);

    if ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0))
        && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0))
    {
        return true;
    }

    // Collinear cases
    if d1 == 0.0 && on_segment(b1, b2, a1) { return true; }
    if d2 == 0.0 && on_segment(b1, b2, a2) { return true; }
    if d3 == 0.0 && on_segment(a1, a2, b1) { return true; }
    if d4 == 0.0 && on_segment(a1, a2, b2) { return true; }

    false
}
```

**Shared-endpoint rule**: Segments from the same net MST tree share pin positions.
Two segments from **different nets** that happen to share a pin position (co-located
pads, which should not occur in valid netlists) would be counted as a collinear
intersection. Filter these during `count_crossings_naive` using the `net_id` field.

### 4.4 Naive Crossing Count (O(E²))

For PCB scale (typical E = 20–200 net segments total), O(E²) is fully acceptable:
at E = 200, that is 19,900 segment-pair tests, each O(1) with the CCW algorithm.

```rust
/// Count the number of intersecting segment pairs across all nets.
///
/// Segments from the same net are never counted as crossing each other
/// (they share pin positions by construction of the MST).
///
/// `all_segments` should be the concatenated MST segments of all nets,
/// with `net_id` set correctly on each segment.
pub fn count_crossings_naive(all_segments: &[Segment]) -> usize {
    let n = all_segments.len();
    let mut count = 0;

    for i in 0..n {
        for j in (i + 1)..n {
            let s1 = &all_segments[i];
            let s2 = &all_segments[j];

            // Segments from the same net never cross each other by definition
            if s1.net_id == s2.net_id {
                continue;
            }

            if segments_intersect(s1.a, s1.b, s2.a, s2.b) {
                count += 1;
            }
        }
    }

    count
}
```

### 4.5 Full Net Crossing Computation

```rust
/// Decompose all nets into MST segments, assign net IDs, count crossings.
///
/// Returns the total crossing count and updates each NetState with its
/// current MST segments.
pub fn compute_all_crossings(nets: &mut [NetState]) -> usize {
    // Step 1: rebuild MST segments for any dirty nets
    for net in nets.iter_mut() {
        if net.dirty || net.segments.is_empty() {
            let mut segs = decompose_net_mst(&net.pin_positions);
            for s in &mut segs {
                s.net_id = net.id;
            }
            net.segments = segs;
        }
    }

    // Step 2: collect all segments
    let all_segments: Vec<Segment> = nets.iter()
        .flat_map(|n| n.segments.iter().copied())
        .collect();

    // Step 3: count
    count_crossings_naive(&all_segments)
}
```

### 4.6 Sweep-Line Algorithm (Future Enhancement)

Bentley-Ottmann sweep-line computes all intersections in O((E + K) log E) where
K = number of actual crossings. This outperforms O(E²) when K is small, but adds
significant implementation complexity (event queue, balanced BST of active segments,
numerical robustness for near-degenerate configurations).

**Decision threshold**: At PCB scale with E ≤ 500 segments, O(E²) = 250,000 ops
per SA move evaluation. If profiling shows crossing computation is a bottleneck
(> 10% of total SA time), implement Bentley-Ottmann. For the initial implementation,
use the naive O(E²) algorithm.

**Not implemented in this spec.** Document as a known future optimization.


---

## 5. Incremental Updates

Recomputing all metrics from scratch on every SA move is too expensive. When a
component moves, only a small subset of nets are affected.

### 5.1 NetComponentIndex Construction

Build this once before SA begins. It does not change during SA (the netlist is
fixed; only positions change).

```rust
impl NetComponentIndex {
    pub fn build(ir: &PcbIr) -> Self {
        let mut net_to_comps: HashMap<NetId, Vec<(ComponentId, Vec<usize>)>> = HashMap::new();
        let mut comp_to_nets: HashMap<ComponentId, Vec<NetId>> = HashMap::new();

        for (net_id, net) in ir.nets.iter() {
            let mut comp_pads: HashMap<ComponentId, Vec<usize>> = HashMap::new();
            for (pin_idx, pin) in net.pins.iter().enumerate() {
                comp_pads.entry(pin.component).or_default().push(pin_idx);
            }
            for (comp_id, pad_indices) in comp_pads {
                net_to_comps.entry(net_id).or_default().push((comp_id, pad_indices));
                comp_to_nets.entry(comp_id).or_default().push(net_id);
            }
        }

        Self { net_to_comps, comp_to_nets }
    }
}
```

### 5.2 Per-Move Incremental Update Protocol

This is the critical inner loop for SA. Every call to accept a move must:

1. **Identify affected nets**: `NetComponentIndex::comp_to_nets[moved_comp_id]`
   (and `comp_to_nets[other_comp_id]` for swap moves, which affect two components).

2. **Save old cost for affected nets** before applying the move:
   ```rust
   let old_hpwl: f64 = affected_nets.iter().map(|&nid| nets[nid].hpwl).sum();
   let old_crossings = count_crossings_for_nets(affected_nets, &all_segments);
   ```

3. **Update pin positions** for the moved component in all affected nets:
   ```rust
   for &net_id in &affected_net_ids {
       let net = &mut nets[net_id];
       for (comp_id, pad_indices) in &index.net_to_comps[net_id] {
           if *comp_id == moved_comp_id {
               for &pad_idx in pad_indices {
                   net.pin_positions[pad_idx] = new_pad_world_pos(moved_comp, pad_idx);
               }
           }
       }
       refresh_net_hpwl(net);

       let mut segs = decompose_net_mst(&net.pin_positions);
       for s in &mut segs { s.net_id = net.id; }
       net.segments = segs;
       net.dirty = false;
   }
   ```

4. **Recompute crossings for affected nets only**:
   ```rust
   // Remove old segments of affected nets from all_segments,
   // insert new segments, recount only pairs involving affected nets.
   let new_crossings = count_crossings_affected_nets(
       &affected_net_ids, &nets, &all_segments
   );
   ```

5. **Compute delta cost**:
   ```rust
   let delta_hpwl = new_hpwl - old_hpwl;
   let delta_crossings = new_crossings as f64 - old_crossings as f64;
   let delta_cost = w_hpwl * delta_hpwl + w_net_crossing * delta_crossings;
   ```

6. **Accept or reject** via Metropolis criterion. If rejected, restore old positions
   and segments.

### 5.3 Crossing Count for Affected Nets Only

```rust
/// Count crossing pairs where at least one segment belongs to an affected net.
///
/// This is used during incremental SA update to compute the crossing delta
/// without reprocessing the entire segment list.
pub fn count_crossings_affected_nets(
    affected_net_ids: &[NetId],
    nets: &[NetState],
    all_segments: &[Segment],
) -> usize {
    let affected_set: HashSet<NetId> = affected_net_ids.iter().copied().collect();
    let mut count = 0;

    let affected_segs: Vec<&Segment> = all_segments.iter()
        .filter(|s| affected_set.contains(&s.net_id))
        .collect();

    // Pairs where both segments are in affected nets (avoid double-counting)
    for i in 0..affected_segs.len() {
        for j in (i+1)..affected_segs.len() {
            let s1 = affected_segs[i];
            let s2 = affected_segs[j];
            if s1.net_id == s2.net_id { continue; }
            if segments_intersect(s1.a, s1.b, s2.a, s2.b) {
                count += 1;
            }
        }
    }

    // Pairs where exactly one segment is in an affected net
    let unaffected_segs: Vec<&Segment> = all_segments.iter()
        .filter(|s| !affected_set.contains(&s.net_id))
        .collect();

    for a_seg in &affected_segs {
        for u_seg in &unaffected_segs {
            if segments_intersect(a_seg.a, a_seg.b, u_seg.a, u_seg.b) {
                count += 1;
            }
        }
    }

    count
}
```

**Note on the `all_segments` buffer**: Maintain a single `Vec<Segment>` that is the
concatenation of all net MST segments. After accepting a move, splice out the old
segments for affected nets and splice in the new ones. The segment list is in net-order
so splicing is O(affected_net_segments) using `retain` + extend.


---

## 6. Grid-Based Congestion Estimation (RUDY-Style)

### 6.1 Motivation

HPWL and crossings are net-level metrics. A placement can have low HPWL but still
be congested in one corner of the board where many nets are routed through a narrow
channel. RUDY (Routing Utilization Density Yardstick) captures this.

### 6.2 Algorithm

```
Board divided into M×N grid cells of size cell_size × cell_size mm.
For each net n:
    Compute net bounding box [x_min, x_max] × [y_min, y_max] in mm.
    demand_n = 1 / (n_pins − 1)   (normalized per-segment demand)
    For each grid cell (r, c) whose edges lie within the net bounding box:
        horizontal_demand[r][c] += demand_n × (min(x_max, cell_right) − max(x_min, cell_left)) / cell_size
        vertical_demand[r][c]   += demand_n × (min(y_max, cell_top)  − max(y_min, cell_bot))  / cell_size

capacity = routing_layers × cell_size  (mm of routable width per cell)
overflow[r][c] = max(0, demand[r][c] − capacity)
total_congestion_penalty = Σ overflow[r][c]²
overflow_count = number of cells where overflow > 0
```

**Demand formula**: RUDY approximates how much routing demand a net places on a cell
by assuming uniform distribution of wires within the net's bounding box. The `demand_n`
factor normalizes by net complexity.

**Cell size choice**: 5mm is a reasonable default for PCB-scale boards. Smaller cells
give finer resolution but more computation; larger cells miss local bottlenecks.
Expose as a parameter in `CongestionGrid::compute`.

### 6.3 Implementation

```rust
impl CongestionGrid {
    /// Build a congestion grid for the given placement and net states.
    ///
    /// `board_bounds`: AABB of the board outline.
    /// `cell_size`: grid cell edge length in mm (recommended: 5.0).
    /// `routing_layers`: number of copper signal layers available for routing.
    pub fn compute(
        nets: &[NetState],
        board_bounds: &BBox,
        cell_size: f64,
        routing_layers: usize,
    ) -> Self {
        let cols = ((board_bounds.width() / cell_size).ceil() as usize).max(1);
        let rows = ((board_bounds.height() / cell_size).ceil() as usize).max(1);
        let mut cells = vec![0.0f64; rows * cols];
        let origin = Point { x: board_bounds.min_x, y: board_bounds.min_y };

        let capacity = routing_layers as f64 * cell_size;

        for net in nets {
            if net.pins.len() < 2 { continue; }
            let bb = &net.bounds;
            let demand_per_net = 1.0 / (net.pins.len() - 1) as f64;

            // Column range covered by this net's bounding box
            let c_start = ((bb.min_x - origin.x) / cell_size).floor() as isize;
            let c_end   = ((bb.max_x - origin.x) / cell_size).ceil()  as isize;
            let r_start = ((bb.min_y - origin.y) / cell_size).floor() as isize;
            let r_end   = ((bb.max_y - origin.y) / cell_size).ceil()  as isize;

            for r in r_start.max(0) as usize..r_end.min(rows as isize) as usize {
                for c in c_start.max(0) as usize..c_end.min(cols as isize) as usize {
                    let cell_left  = origin.x + c as f64 * cell_size;
                    let cell_bot   = origin.y + r as f64 * cell_size;
                    let cell_right = cell_left + cell_size;
                    let cell_top   = cell_bot  + cell_size;

                    let h_frac = (bb.max_x.min(cell_right) - bb.min_x.max(cell_left))
                        .max(0.0) / cell_size;
                    let v_frac = (bb.max_y.min(cell_top)   - bb.min_y.max(cell_bot))
                        .max(0.0) / cell_size;

                    cells[r * cols + c] += demand_per_net * (h_frac + v_frac);
                }
            }
        }

        let overflow_count = cells.iter()
            .filter(|&&d| d > capacity)
            .count();

        Self { cells, rows, cols, cell_size, origin, overflow_count, capacity }
    }

    /// Squared overflow penalty: Σ max(0, demand − capacity)².
    /// This is convex and penalises highly congested cells more than lightly
    /// congested ones, encouraging the solver to flatten the demand distribution.
    pub fn overflow_penalty(&self) -> f64 {
        self.cells.iter()
            .map(|&d| (d - self.capacity).max(0.0).powi(2))
            .sum()
    }
}
```

### 6.4 Integration with SA Cost Function

Congestion adds a term to the SA cost:

```rust
let congestion_penalty = if w_congestion > 0.0 {
    let grid = CongestionGrid::compute(nets, board_bounds, cell_size, routing_layers);
    grid.overflow_penalty()
} else {
    0.0
};
```

**When to enable**: Congestion is optional. Compute a baseline `overflow_count` at
the start of SA. Enable the congestion penalty only if `overflow_count > 0` on the
initial legal placement. This avoids unnecessary computation on uncongested boards.

**Phase 1**: Congestion is NOT used in the analytical Phase 1 solver. It is not
differentiable in the form presented here. Density constraints (component clearance)
in solverang serve as the Phase 1 substitute for congestion control.


---

## 7. Cost Function Integration

### 7.1 Weights

| Component | Default weight | Rationale |
|-----------|---------------|-----------|
| HPWL | 1.0 | Baseline objective; all other weights are relative to this |
| Overlap | 10.0 | Must avoid; heavy penalty drives SA to reject overlapping moves |
| Constraint violation | 100.0 | Must satisfy; near-infinite penalty ensures feasibility |
| Net crossings | 0.05 | Routability; secondary to HPWL. Adjusted per auto-tuning heuristic |
| Congestion | 0.0 (disabled) | Enable if initial overflow_count > 0; start at 0.1 |

**Why overlap weight is 10× HPWL**: An overlap of 1mm area should cost more than
1mm of extra wire length. The factor of 10 ensures SA strongly prefers legal placements
and only permits temporary overlaps at high temperature.

**Why constraint weight is 100×**: Constraints represent physical impossibilities
(board edge, fixed components). 100× makes them effectively hard — SA will almost
never accept a constraint-violating move except at the highest temperatures.

### 7.2 Cost Function Formula

```rust
pub fn compute_total_cost(
    breakdown: &CostBreakdown,
    w_hpwl: f64,
    w_overlap: f64,
    w_constraint: f64,
    w_net_crossing: f64,
    w_congestion: f64,
) -> f64 {
    w_hpwl       * breakdown.hpwl
        + w_overlap     * breakdown.overlap
        + w_constraint  * breakdown.constraint
        + w_net_crossing * breakdown.crossings
        + w_congestion  * breakdown.congestion
}
```

### 7.3 Auto-Tuning Net Crossing Weight

Heuristic to adjust `w_net_crossing` based on observed crossing density:

```rust
/// Ratio of observed crossings to the theoretical maximum for this netlist.
/// crossing_ratio ∈ [0, 1], where 1 means every segment pair crosses.
fn crossing_ratio(total_crossings: usize, total_segments: usize) -> f64 {
    if total_segments < 2 { return 0.0; }
    let max_crossings = total_segments * (total_segments - 1) / 2;
    total_crossings as f64 / max_crossings as f64
}

/// Suggest a net crossing weight given current placement statistics.
///
/// If most segment pairs already cross, increasing the weight will dominate
/// HPWL and may produce worse placements. Cap at 0.1.
pub fn suggest_crossing_weight(
    total_crossings: usize,
    total_segments: usize,
    base_weight: f64,
) -> f64 {
    let ratio = crossing_ratio(total_crossings, total_segments);
    if ratio > 0.5 {
        // Very congested: increase weight to drive improvements
        (base_weight * 2.0).min(0.1)
    } else if ratio < 0.05 {
        // Already very few crossings: reduce weight to focus on HPWL
        base_weight * 0.5
    } else {
        base_weight
    }
}
```

Apply this adjustment once at the start of the SA cooling schedule (not per move).


---

## 8. Module Structure

Full public API for `crates/autopcb-placement/src/routability.rs`:

```rust
// Types
pub struct Point { pub x: f64, pub y: f64 }
pub struct BBox { pub min_x, max_x, min_y, max_y: f64 }
pub struct Segment { pub a: Point, pub b: Point, pub net_id: NetId }
pub struct NetState { ... }   // see Section 2
pub struct NetComponentIndex { ... }   // see Section 5.1
pub struct CongestionGrid { ... }   // see Section 6.3
pub struct CostBreakdown { ... }   // see Section 2

// HPWL
pub fn compute_hpwl_exact(nets: &[NetState]) -> f64
pub fn refresh_net_hpwl(net_state: &mut NetState)

// MST decomposition
pub fn decompose_net_mst(pin_positions: &[Point]) -> Vec<Segment>

// Intersection test
pub fn segments_intersect(a1: Point, a2: Point, b1: Point, b2: Point) -> bool

// Crossing count
pub fn count_crossings_naive(all_segments: &[Segment]) -> usize
pub fn compute_all_crossings(nets: &mut [NetState]) -> usize
pub fn count_crossings_affected_nets(
    affected_net_ids: &[NetId],
    nets: &[NetState],
    all_segments: &[Segment],
) -> usize

// Congestion
impl CongestionGrid {
    pub fn compute(nets, board_bounds, cell_size, routing_layers) -> Self
    pub fn overflow_penalty(&self) -> f64
}

// Index
impl NetComponentIndex {
    pub fn build(ir: &PcbIr) -> Self
}

// Cost
pub fn compute_total_cost(breakdown, w_hpwl, w_overlap, w_constraint,
                          w_net_crossing, w_congestion) -> f64
pub fn suggest_crossing_weight(total_crossings, total_segments, base_weight) -> f64
```

**Private helpers** (not pub):

```rust
fn cross_sign(p: Point, q: Point, r: Point) -> f64
fn on_segment(p: Point, q: Point, r: Point) -> bool
fn crossing_ratio(total_crossings: usize, total_segments: usize) -> f64
```


---

## 9. Test Cases

All tests go in `#[cfg(test)]` blocks within `routability.rs`. No fixture files
are required — all tests use programmatically constructed inputs.

### 9.1 HPWL Tests

**Test: 4-pin net at corners of 10×10mm square**

```
pins: (0,0), (10,0), (0,10), (10,10)
bounding box: x ∈ [0,10], y ∈ [0,10]
HPWL = (10-0) + (10-0) = 20.0 mm
```

```rust
#[test]
fn hpwl_square_corners() {
    let pins = vec![
        Point { x: 0.0, y: 0.0 },
        Point { x: 10.0, y: 0.0 },
        Point { x: 0.0, y: 10.0 },
        Point { x: 10.0, y: 10.0 },
    ];
    let mut net = NetState {
        id: NetId::from_raw(0),
        pins: vec![],
        pin_positions: pins,
        bounds: BBox::from_point(Point { x: 0.0, y: 0.0 }),
        hpwl: 0.0,
        segments: vec![],
        crossing_count: 0,
        dirty: true,
    };
    refresh_net_hpwl(&mut net);
    assert!((net.hpwl - 20.0).abs() < 1e-9);
}
```

**Test: single-pin net has HPWL = 0**

```
pins: (5.0, 7.3)
HPWL = 0.0
```

**Test: collinear pins**

```
pins: (0,5), (5,5), (10,5)
bounding box: x ∈ [0,10], y ∈ [5,5]
HPWL = 10.0 + 0.0 = 10.0
```

### 9.2 Crossing Tests

**Test: two perpendicular segments cross**

```
seg A: (0,5) → (10,5)   (horizontal)
seg B: (5,0) → (5,10)   (vertical)
Expected: segments_intersect = true
```

```rust
#[test]
fn crossing_perpendicular() {
    let a1 = Point { x: 0.0, y: 5.0 };
    let a2 = Point { x: 10.0, y: 5.0 };
    let b1 = Point { x: 5.0, y: 0.0 };
    let b2 = Point { x: 5.0, y: 10.0 };
    assert!(segments_intersect(a1, a2, b1, b2));
}
```

**Test: two parallel segments do not cross**

```
seg A: (0,3) → (10,3)
seg B: (0,7) → (10,7)
Expected: segments_intersect = false
```

**Test: two segments that share one endpoint do not cross**

```
seg A: (0,0) → (5,5)
seg B: (5,5) → (10,0)
Expected: segments_intersect = false
```

This case arises when two MST segments from the same net share a pin (which is the
expected case and should NOT be counted as a crossing).

**Test: two crossing diagonal segments**

```
seg A: (0,0) → (10,10)
seg B: (0,10) → (10,0)
Expected: segments_intersect = true
```

**Test: two non-crossing diagonal segments (same quadrant)**

```
seg A: (0,0) → (4,4)
seg B: (6,0) → (10,4)
Expected: segments_intersect = false
```

**Test: count_crossings_naive with one crossing pair**

```
net_0 segment: (0,5) → (10,5)   net_id=0
net_1 segment: (5,0) → (5,10)   net_id=1
Expected: count = 1
```

**Test: count_crossings_naive ignores same-net pairs**

```
net_0 seg A: (0,5) → (10,5)   net_id=0
net_0 seg B: (5,0) → (5,10)   net_id=0   (same net, perpendicular)
Expected: count = 0
```

### 9.3 MST Tests

**Test: 4-pin square net produces 3 MST edges**

```
pins: (0,0), (10,0), (0,10), (10,10)
MST has 4-1 = 3 edges.
All edges should have total length ≤ any other spanning tree.
```

```rust
#[test]
fn mst_square_has_three_edges() {
    let pins = vec![
        Point { x: 0.0, y: 0.0 },
        Point { x: 10.0, y: 0.0 },
        Point { x: 0.0, y: 10.0 },
        Point { x: 10.0, y: 10.0 },
    ];
    let segs = decompose_net_mst(&pins);
    assert_eq!(segs.len(), 3);
}
```

**Test: 2-pin net produces 1 MST edge**

```
pins: (0,0), (5,5)
Expected: 1 segment from (0,0) to (5,5)
```

**Test: 1-pin net produces 0 segments**

**Test: MST total length ≤ complete-graph total length**

For any 4-pin net, verify that the MST Manhattan length is ≤ the sum of all six
pairwise Manhattan distances. This is a sanity check that Prim's gives a valid
spanning tree.

### 9.4 Congestion Tests

**Test: single net crossing the center of a 2×2 grid**

```
Board: 10mm × 10mm, origin (0,0)
cell_size = 5mm → 2×2 grid (4 cells)
Net bounding box: (2,2) to (8,8) — passes through all 4 cells
routing_layers = 2 → capacity = 2 × 5.0 = 10.0 mm

Demand per cell (approximate):
  h_frac for cell (0,0): overlap in x = min(8,5)-max(2,0) = 3 → 3/5 = 0.6
  v_frac for cell (0,0): overlap in y = min(8,5)-max(2,0) = 3 → 3/5 = 0.6
  demand[0][0] ≈ 1 × (0.6 + 0.6) = 1.2

With demand = 1.2 and capacity = 10.0: overflow = 0 (no overflow).

For a highly congested scenario with capacity = 0 (routing_layers=0):
  overflow_count = 4 (all cells have demand > 0)
```

```rust
#[test]
fn congestion_single_net_no_overflow() {
    let mut net = NetState { /* 2-pin net with bounds (2,2)→(8,8) */ ... };
    refresh_net_hpwl(&mut net);
    let board = BBox { min_x: 0.0, max_x: 10.0, min_y: 0.0, max_y: 10.0 };
    let grid = CongestionGrid::compute(&[net], &board, 5.0, 2);
    assert_eq!(grid.overflow_count, 0);
}

#[test]
fn congestion_single_net_zero_capacity_has_overflow() {
    let mut net = NetState { /* same */ ... };
    refresh_net_hpwl(&mut net);
    let board = BBox { min_x: 0.0, max_x: 10.0, min_y: 0.0, max_y: 10.0 };
    let grid = CongestionGrid::compute(&[net], &board, 5.0, 0);
    // All 4 cells have positive demand but zero capacity
    assert!(grid.overflow_count > 0);
}
```

**Test: overflow_penalty is zero when no overflow**

```
grid = CongestionGrid with all cells ≤ capacity
Expected: grid.overflow_penalty() == 0.0
```

**Test: overflow_penalty is positive and proportional to squared overflow**

```
Single cell with demand = 5.0, capacity = 3.0
overflow = 5.0 - 3.0 = 2.0
expected penalty = 2.0² = 4.0
```


---

## 10. Relationship to Pipeline Phases

This table shows exactly which functions from this module are called in each pipeline phase:

| Pipeline phase | Functions used | Notes |
|----------------|---------------|-------|
| Phase 0 (pre-processing) | `NetComponentIndex::build` | Build once; reused in all subsequent phases |
| Phase 1 (analytical — solverang) | None from this module | Phase 1 uses `SmoothHpwlConstraint` from `constraints.rs` |
| Phase 2 (legalization) | `compute_hpwl_exact`, `compute_all_crossings` | Evaluate solution quality after legalization |
| Phase 3 (SA) | All functions in this module | Incremental updates on every accepted move |
| Phase 4 (final refinement — solverang) | `compute_hpwl_exact` | Quality evaluation before/after Phase 4 |
| Phase 5 (DRC) | None | DRC uses constraint residuals, not routability metrics |

**Data ownership during SA**: The SA engine owns `Vec<NetState>`, `NetComponentIndex`,
and the global `Vec<Segment>` buffer. The `CongestionGrid` is recomputed at temperature
checkpoints (e.g., every `moves_per_temp` moves) rather than per-move, because it is
not used incrementally.


---

## 11. Numerical Considerations

### 11.1 Floating-Point Precision in Intersection Test

The CCW test uses exact floating-point arithmetic without epsilon guards. This is
correct for the implementation: PCB coordinates in mm with f64 have ≈15 significant
digits of precision, and component positions do not require sub-nanometre accuracy.
Near-collinear cases (where `cross_sign` returns a very small value) are correctly
handled by the `on_segment` collinear branch.

**Do NOT add epsilon guards** to `cross_sign` or `segments_intersect`. Epsilons
introduce correctness bugs for segments that are genuinely parallel or coincident.
If a near-degenerate case causes an incorrect crossing count, diagnose the root cause
(typically a numerical issue upstream in coordinate conversion) rather than masking it.

### 11.2 HPWL Accumulation

Summing many f64 HPWL values: at PCB scale (≤ 500 nets, total HPWL ≤ 10,000mm),
naive summation with f64 is accurate to better than 0.001mm. Kahan summation is not
necessary.

### 11.3 Congestion Grid Boundary Conditions

Components and nets that extend slightly outside the board bounds (due to floating-point
rounding in coordinate conversion) are clamped to the grid boundary using `.max(0)` and
`.min(rows/cols)` in `CongestionGrid::compute`. This avoids out-of-bounds indexing
without silently discarding data — the demand is still counted in the boundary cells.
