# Simulated Annealing Implementation Specification (Phase 3)

## 1. Overview

Simulated annealing (SA) is Phase 3 of the 6-phase PCB autoplacer pipeline:

```
Phase 0: Clustering (pre-processing)
Phase 1: Solverang analytical (continuous optimization, existing)
Phase 2: Legalization (snap rotations, resolve overlaps, existing)
Phase 3: SA detailed placement  ← THIS SPEC
Phase 4: Final Solverang refinement (fixed rotations, existing)
Phase 5: DRC verification
```

Phases 1, 2, and 4 are already implemented in
`crates/autopcb-placement/src/lib.rs`. Phase 3 must be implemented as a new
module in the same crate: `crates/autopcb-placement/src/simulated_annealing.rs`.

SA's role: take Phase 2's legal placement (feasible, no overlaps, integer
rotations) and improve it by exploring discrete moves that gradient-based
Levenberg-Marquardt cannot perform — component swaps, 90° rotations, and
single-axis slides. SA uses the Metropolis acceptance criterion to escape local
minima in the HPWL + routability landscape.

### Why SA after Phase 2 (not instead of it)

Phase 2 output is a legal, constraint-satisfying placement. SA starts warm from
this state rather than from random initial conditions, which means SA explores a
much smaller region of the search space and converges in far fewer temperature
steps than cold-start SA would require.

### Pipeline numbering alignment

The spec document uses "Phase 3" to mean SA. The existing `solve_placement()`
function in `lib.rs` runs what the pipeline calls Phases 1 and 2 internally —
it produces the legalized placement. The implementing agent should not renumber
anything: SA is injected between the call to `solve_placement()` and the
optional call to the Phase 4 refinement pass.

---

## 2. Existing Codebase Integration

### 2.1 Types SA Consumes (from `autopcb-ir`)

All types are re-exported from `autopcb_ir::*`. Relevant items:

**`PcbIr`** (`crates/autopcb-ir/src/extract.rs`):
```
PcbIr {
    board: IrBoardGeometry,
    layer_stack: IrLayerStack,
    components: IdMap<ComponentId, IrComponent>,
    nets: IdMap<NetId, IrNet>,
    rules: IdMap<RuleId, IrDesignRule>,
    free_copper: FreeCopperGeometry,
    polygons: IdMap<PolygonId, IrPolygon>,
}
```

**`IrBoardGeometry`** (`crates/autopcb-ir/src/board.rs`):
- `bounds: BoundingBoxMm` — AABB of the board outline (min/max x/y in mm)
- `outline: Vec<PointMm>` — tessellated polygon
- `keepouts: Vec<IrKeepoutZone>`

**`IrComponent`** (`crates/autopcb-ir/src/component.rs`):
- `id: ComponentId`
- `designator: String`
- `position: PointMm` — world position (mm)
- `rotation: f64` — degrees
- `local_bounds: BoundingBoxMm` — bounding box in component-local coords
- `world_bounds: BoundingBoxMm` — bounding box in world coords
- `pads: Vec<IrComponentPad>`

**`IrComponentPad`**:
- `id: PadId`
- `name: String`
- `local_position: PointMm` — position relative to component origin
- `world_position: PointMm` — world position
- `net: Option<NetId>`

**`IrNet`** (`crates/autopcb-ir/src/net.rs`):
- `id: NetId`
- `name: String`
- `pins: Vec<IrNetPin>` — all pads on this net
- `component_count: usize`

**`IrNetPin`**:
- `pad: PadId`
- `component: ComponentId`
- `position: PointMm`

**`BoundingBoxMm`** (`crates/autopcb-ir/src/types.rs`):
- `min: PointMm`, `max: PointMm`
- Methods: `width()`, `height()`, `center()`

**`PointMm`**: `x: f64`, `y: f64`

**`ComponentId`, `NetId`, `PadId`** (`crates/autopcb-ir/src/handles.rs`):
- Newtype wrappers around `u32`
- `.raw() -> u32` for use as `HashMap` keys

### 2.2 Types SA Produces (existing in `autopcb-placement`)

SA must return a `PlacementResult` identical in shape to what `solve_placement()`
already produces. All these types live in `lib.rs`.

```rust
pub struct PlacementResult {
    pub status: String,
    pub total_iterations: usize,
    pub duration_ms: u128,
    pub components: Vec<PlacementComponentState>,
    pub snapshots: Vec<PlacementIterationSnapshot>,
    pub hpwl_estimate_mm: f64,
    pub overlap_violations: usize,
}

pub struct PlacementComponentState {
    pub designator: String,
    pub x_mm: f64,
    pub y_mm: f64,
    pub rotation_deg: f64,
}

pub struct PlacementIterationSnapshot {
    pub phase: String,
    pub components: Vec<PlacementComponentState>,
    pub note: Option<String>,
}
```

### 2.3 Existing `PlacementConfig`

```rust
pub struct PlacementConfig {
    pub gamma_start: f64,              // 2.0
    pub gamma_end: f64,                // 10.0
    pub max_iters: usize,              // 250
    pub ratsnest_weight: f64,          // 0.01
    pub default_clearance_mm: f64,     // 0.5
    pub board_edge_clearance_mm: f64,  // 0.0
    pub grid_snap_mm: Option<f64>,
}
```

SA introduces its own `SAConfig` (see Section 3.1). The integration point is
`solve_placement()` gaining an optional `sa_config: Option<SAConfig>` parameter,
or a separate `refine_with_sa()` entry point callable after `solve_placement()`.
The spec mandates a separate entry point (see Section 9.1) to keep `solve_placement()`
signature stable.

### 2.4 Existing Constraint Types

The eight constraint types defined in `lib.rs` are used by Phases 1 and 2 only.
SA does not use the Solverang `ConstraintSystem` at all — SA operates on a
self-contained `Placement` struct with its own cost function. The constraints
inform which moves are hard-rejected (board bounds), but SA does not call
`.residuals()` on any `Constraint` implementor.

### 2.5 Existing HPWL

`estimate_hpwl()` in `lib.rs` computes exact Manhattan HPWL from a list of
`PlacementComponentState` values and a net-to-pin map. SA's internal cost
function uses an equivalent computation directly on its `Placement` struct (see
Section 5.1). The existing `estimate_hpwl()` is reused for the final
`hpwl_estimate_mm` field in the returned `PlacementResult`.

---

## 3. New Module: `crates/autopcb-placement/src/simulated_annealing.rs`

Create this file. Add `pub mod simulated_annealing;` to `lib.rs`. Re-export the
public entry point and config type from `lib.rs`:

```rust
pub use simulated_annealing::{refine_with_sa, SAConfig};
```

### 3.1 `SAConfig`

```rust
#[derive(Debug, Clone)]
pub struct SAConfig {
    // Temperature schedule
    pub t_initial: Option<f64>,        // None = auto-initialize from sample moves
    pub t_frozen: f64,                 // 0.01: stop when acceptance ratio < 1%
    pub alpha: f64,                    // 0.95: base cooling rate per temperature step

    // Move budget
    pub moves_per_temp: usize,         // 10 * N_components per temperature step
    pub displacement_range_mm: f64,    // initial max displacement; shrinks proportionally with T

    // Move type probabilities (must sum <= 1.0; remainder goes to Slide)
    pub p_displace: f64,               // 0.4
    pub p_swap: f64,                   // 0.3
    pub p_rotate: f64,                 // 0.2
    pub p_slide: f64,                  // 0.1

    // Cost weights
    pub w_hpwl: f64,                   // 1.0
    pub w_overlap: f64,                // 10.0
    pub w_constraint: f64,             // 100.0
    pub w_net_crossing: f64,           // 0.5 (only used when enable_net_crossings = true)
    pub enable_net_crossings: bool,    // false: skip expensive crossing computation

    // Termination
    pub max_temperature_steps: usize,  // 500: hard cap regardless of acceptance

    // Clearance for overlap detection (matches PlacementConfig::default_clearance_mm)
    pub default_clearance_mm: f64,     // 0.5
}

impl Default for SAConfig {
    fn default() -> Self {
        SAConfig {
            t_initial: None,
            t_frozen: 0.01,
            alpha: 0.95,
            moves_per_temp: 0,   // 0 = auto: 10 * N at call time
            displacement_range_mm: 10.0,
            p_displace: 0.4,
            p_swap: 0.3,
            p_rotate: 0.2,
            p_slide: 0.1,
            w_hpwl: 1.0,
            w_overlap: 10.0,
            w_constraint: 100.0,
            w_net_crossing: 0.5,
            enable_net_crossings: false,
            max_temperature_steps: 500,
            default_clearance_mm: 0.5,
        }
    }
}
```

### 3.2 Internal Placement State

SA operates on a `Placement` struct that is completely independent of the Solverang
`ConstraintSystem`. It is built once at the start of `refine_with_sa()` from the
`PlacementResult` returned by `solve_placement()` combined with structural data from
`PcbIr`.

```rust
// Private to simulated_annealing.rs — not pub

#[derive(Debug, Clone)]
struct PadState {
    local_x: f64,
    local_y: f64,
    net_index: Option<usize>,   // index into Placement::nets
}

#[derive(Debug, Clone)]
struct ComponentState {
    comp_id: ComponentId,
    designator: String,
    x: f64,
    y: f64,
    theta_deg: f64,
    half_w: f64,   // component-local half-width (from IrComponent::local_bounds)
    half_h: f64,   // component-local half-height (from IrComponent::local_bounds)
    pads: Vec<PadState>,
    fixed: bool,   // true if a FixedPosition user constraint locks this component
}

#[derive(Debug, Clone)]
struct NetState {
    name: String,
    pin_refs: Vec<PinRef>,  // component_index + pad_index within that component
}

#[derive(Debug, Clone, Copy)]
struct PinRef {
    comp_idx: usize,
    pad_idx: usize,
}

struct Placement {
    components: Vec<ComponentState>,
    nets: Vec<NetState>,
    comp_id_to_index: HashMap<ComponentId, usize>,
    net_comp_index: NetComponentIndex,
    spatial_grid: SpatialGrid,
    board_min_x: f64,
    board_min_y: f64,
    board_max_x: f64,
    board_max_y: f64,
}
```

#### `NetComponentIndex`

For each component, tracks which net indices it participates in. Used for O(k)
incremental HPWL computation.

```rust
struct NetComponentIndex {
    // comp_to_nets[comp_idx] = list of net indices the component has pads on
    comp_to_nets: Vec<Vec<usize>>,
}

impl NetComponentIndex {
    fn build(components: &[ComponentState], nets: &[NetState]) -> Self {
        let mut comp_to_nets: Vec<Vec<usize>> = vec![Vec::new(); components.len()];
        for (net_idx, net) in nets.iter().enumerate() {
            let mut seen = HashSet::new();
            for pin_ref in &net.pin_refs {
                if seen.insert(pin_ref.comp_idx) {
                    comp_to_nets[pin_ref.comp_idx].push(net_idx);
                }
            }
        }
        NetComponentIndex { comp_to_nets }
    }

    fn nets_for_component(&self, comp_idx: usize) -> &[usize] {
        &self.comp_to_nets[comp_idx]
    }

    fn nets_for_pair(&self, a: usize, b: usize) -> impl Iterator<Item = usize> + '_ {
        let a_nets: HashSet<usize> = self.comp_to_nets[a].iter().copied().collect();
        self.comp_to_nets[a]
            .iter()
            .copied()
            .chain(self.comp_to_nets[b].iter().copied().filter(move |n| !a_nets.contains(n)))
    }
}
```

#### `SpatialGrid`

A uniform grid for O(k) neighbor overlap detection. Cell size is chosen so that
only the immediately adjacent cells need to be checked.

```rust
struct SpatialGrid {
    cell_size: f64,
    cols: usize,
    rows: usize,
    board_min_x: f64,
    board_min_y: f64,
    // Each cell holds the set of component indices whose AABB overlaps that cell
    cells: Vec<Vec<usize>>,
}

impl SpatialGrid {
    fn build(
        components: &[ComponentState],
        board_min_x: f64,
        board_min_y: f64,
        board_max_x: f64,
        board_max_y: f64,
        clearance: f64,
    ) -> Self {
        // Choose cell_size = largest component diagonal + clearance
        // This guarantees that two components can only overlap if they are in
        // adjacent or same cells.
        let max_half = components.iter().fold(0.0_f64, |acc, c| {
            acc.max(c.half_w).max(c.half_h)
        });
        let cell_size = (max_half * 2.0 + clearance).max(1.0);

        let board_w = board_max_x - board_min_x;
        let board_h = board_max_y - board_min_y;
        let cols = ((board_w / cell_size).ceil() as usize).max(1);
        let rows = ((board_h / cell_size).ceil() as usize).max(1);

        let mut grid = SpatialGrid {
            cell_size,
            cols,
            rows,
            board_min_x,
            board_min_y,
            cells: vec![Vec::new(); cols * rows],
        };

        for (idx, comp) in components.iter().enumerate() {
            grid.insert(idx, comp);
        }
        grid
    }

    fn cell_index(&self, cx: f64, cy: f64) -> Option<usize> {
        let col = ((cx - self.board_min_x) / self.cell_size) as isize;
        let row = ((cy - self.board_min_y) / self.cell_size) as isize;
        if col < 0 || row < 0 || col >= self.cols as isize || row >= self.rows as isize {
            return None;
        }
        Some(row as usize * self.cols + col as usize)
    }

    fn insert(&mut self, comp_idx: usize, comp: &ComponentState) {
        // Insert comp_idx into all cells its AABB overlaps
        let (hw, hh) = world_half_extents(comp);
        let x_min_col = (((comp.x - hw - self.board_min_x) / self.cell_size).floor() as isize).max(0) as usize;
        let x_max_col = (((comp.x + hw - self.board_min_x) / self.cell_size).ceil() as isize).min(self.cols as isize - 1).max(0) as usize;
        let y_min_row = (((comp.y - hh - self.board_min_y) / self.cell_size).floor() as isize).max(0) as usize;
        let y_max_row = (((comp.y + hh - self.board_min_y) / self.cell_size).ceil() as isize).min(self.rows as isize - 1).max(0) as usize;

        for row in y_min_row..=y_max_row {
            for col in x_min_col..=x_max_col {
                let idx = row * self.cols + col;
                if !self.cells[idx].contains(&comp_idx) {
                    self.cells[idx].push(comp_idx);
                }
            }
        }
    }

    fn remove(&mut self, comp_idx: usize, comp: &ComponentState) {
        let (hw, hh) = world_half_extents(comp);
        let x_min_col = (((comp.x - hw - self.board_min_x) / self.cell_size).floor() as isize).max(0) as usize;
        let x_max_col = (((comp.x + hw - self.board_min_x) / self.cell_size).ceil() as isize).min(self.cols as isize - 1).max(0) as usize;
        let y_min_row = (((comp.y - hh - self.board_min_y) / self.cell_size).floor() as isize).max(0) as usize;
        let y_max_row = (((comp.y + hh - self.board_min_y) / self.cell_size).ceil() as isize).min(self.rows as isize - 1).max(0) as usize;

        for row in y_min_row..=y_max_row {
            for col in x_min_col..=x_max_col {
                let idx = row * self.cols + col;
                self.cells[idx].retain(|&i| i != comp_idx);
            }
        }
    }

    fn neighbors(&self, comp: &ComponentState, clearance: f64) -> Vec<usize> {
        let (hw, hh) = world_half_extents(comp);
        let search_hw = hw + clearance;
        let search_hh = hh + clearance;

        let x_min_col = (((comp.x - search_hw - self.board_min_x) / self.cell_size).floor() as isize).max(0) as usize;
        let x_max_col = (((comp.x + search_hw - self.board_min_x) / self.cell_size).ceil() as isize).min(self.cols as isize - 1).max(0) as usize;
        let y_min_row = (((comp.y - search_hh - self.board_min_y) / self.cell_size).floor() as isize).max(0) as usize;
        let y_max_row = (((comp.y + search_hh - self.board_min_y) / self.cell_size).ceil() as isize).min(self.rows as isize - 1).max(0) as usize;

        let mut result = Vec::new();
        for row in y_min_row..=y_max_row {
            for col in x_min_col..=x_max_col {
                let idx = row * self.cols + col;
                for &ci in &self.cells[idx] {
                    if !result.contains(&ci) {
                        result.push(ci);
                    }
                }
            }
        }
        result
    }
}
```

### 3.3 Move Types

```rust
#[derive(Debug, Clone, Copy)]
pub enum Axis {
    X,
    Y,
}

#[derive(Debug, Clone, Copy)]
enum Move {
    Displace { comp_idx: usize, dx: f64, dy: f64 },
    Swap { a_idx: usize, b_idx: usize },
    Rotate { comp_idx: usize, delta_deg: i32 },  // +90, +180, +270 only
    Slide { comp_idx: usize, axis: Axis, delta: f64 },
}
```

---

## 4. Building `Placement` from `PlacementResult` + `PcbIr`

The `Placement` struct is built in `refine_with_sa()` before the SA loop starts.
The algorithm:

1. For each `PlacementComponentState` in `initial.components`, look up the
   matching `IrComponent` by designator from `ir.components`.

2. Compute `half_w` and `half_h` from `ir_comp.local_bounds`:
   ```
   half_w = ir_comp.local_bounds.width() * 0.5
   half_h = ir_comp.local_bounds.height() * 0.5
   ```

3. Collect pads for each component: for each `IrComponentPad` in `ir_comp.pads`,
   record `local_position.x`, `local_position.y`, and resolve `pad.net` to a
   `net_index` in the `nets` vec (built from `ir.nets`).

4. Build `nets: Vec<NetState>`: iterate `ir.nets`, for each `IrNet` with
   `component_count >= 2`, build a `NetState` with `pin_refs` pointing into the
   `components` vec.

5. Build `NetComponentIndex` and `SpatialGrid` from the assembled vecs.

The `fixed` flag on a `ComponentState` is set to `true` if any `UserConstraint::FixedPosition`
constraint targets that designator. Fixed components are never chosen as the
subject of a move. Because `UserConstraint` is defined in `lib.rs` and is not
passed to `refine_with_sa()`, the fixed-component detection is opt-in: the caller
may pass a `HashSet<String>` of fixed designators, or `refine_with_sa()` may mark
no components as fixed. The entry point signature handles this (see Section 9.1).

---

## 5. Cost Function

### 5.1 HPWL (exact Manhattan)

SA does not need smooth HPWL — no gradients are required. Use exact computation.

For a net with pins at world positions `(wx_i, wy_i)`:
```
HPWL(net) = (max_i wx_i - min_i wx_i) + (max_i wy_i - min_i wy_i)
```

World position of a pad given its component state and local offset:
```
wx = comp.x + local_x * cos(theta) - local_y * sin(theta)
wy = comp.y + local_x * sin(theta) + local_y * cos(theta)
```
where `theta = comp.theta_deg.to_radians()`.

Total HPWL across all nets: `sum of HPWL(net) for each net with >= 2 pins`.

Implementation (computes HPWL for a single net given current component states):

```rust
fn net_hpwl(net: &NetState, components: &[ComponentState]) -> f64 {
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for pin_ref in &net.pin_refs {
        let comp = &components[pin_ref.comp_idx];
        let pad = &comp.pads[pin_ref.pad_idx];
        let theta = comp.theta_deg.to_radians();
        let (sin_t, cos_t) = theta.sin_cos();
        let wx = comp.x + pad.local_x * cos_t - pad.local_y * sin_t;
        let wy = comp.y + pad.local_x * sin_t + pad.local_y * cos_t;
        min_x = min_x.min(wx);
        max_x = max_x.max(wx);
        min_y = min_y.min(wy);
        max_y = max_y.max(wy);
    }

    if min_x.is_finite() {
        (max_x - min_x) + (max_y - min_y)
    } else {
        0.0
    }
}
```

Total HPWL: `placement.nets.iter().map(|n| net_hpwl(n, &placement.components)).sum::<f64>()`.

### 5.2 Overlap Penalty

Two components overlap when their world AABBs (including clearance gap) intersect.

World half-extents under rotation (conservative AABB bound):
```
half_w_world = half_w * |cos(theta)| + half_h * |sin(theta)|
half_h_world = half_w * |sin(theta)| + half_h * |cos(theta)|
```

The function `world_half_extents(comp: &ComponentState) -> (f64, f64)` returns
`(half_w_world, half_h_world)` using the formula above.

Two components `a` and `b` overlap (including required clearance `gap`) when:
```
|a.x - b.x| < half_w_a + half_w_b + gap
AND
|a.y - b.y| < half_h_a + half_h_b + gap
```

Overlap penalty = `(number of overlapping pairs among neighbors) * config.w_overlap`.

For the spatial grid optimization, only check `neighbors()` of the moved component,
not all N components.

### 5.3 Board Containment (hard rejection)

A component violates board bounds when:
```
comp.x - half_w_world < board_min_x
OR comp.x + half_w_world > board_max_x
OR comp.y - half_h_world < board_min_y
OR comp.y + half_h_world > board_max_y
```

Moves that produce bound violations are **hard-rejected** before computing
delta-cost. The Metropolis criterion is not applied; the move is simply discarded.

### 5.4 Net Crossing Count (optional)

Only computed when `config.enable_net_crossings == true`.

#### Step 1: Decompose multi-pin nets into 2-pin segments via Prim's MST

For a net with pins at world positions `P[0..k]`:

```rust
fn net_mst_segments(net: &NetState, components: &[ComponentState]) -> Vec<(usize, usize)> {
    // Returns pairs of pin indices (i, j) forming the MST
    let positions: Vec<(f64, f64)> = net.pin_refs.iter().map(|pr| {
        let comp = &components[pr.comp_idx];
        let pad = &comp.pads[pr.pad_idx];
        let theta = comp.theta_deg.to_radians();
        let (sin_t, cos_t) = theta.sin_cos();
        let wx = comp.x + pad.local_x * cos_t - pad.local_y * sin_t;
        let wy = comp.y + pad.local_x * sin_t + pad.local_y * cos_t;
        (wx, wy)
    }).collect();

    let n = positions.len();
    if n < 2 {
        return Vec::new();
    }

    // Prim's algorithm: grow MST from vertex 0
    let mut in_mst = vec![false; n];
    let mut min_edge: Vec<(f64, usize)> = vec![(f64::INFINITY, 0); n];
    min_edge[0] = (0.0, 0);
    let mut edges = Vec::with_capacity(n - 1);

    for _ in 0..n {
        // Find vertex with minimum key not yet in MST
        let u = (0..n)
            .filter(|&v| !in_mst[v])
            .min_by(|&a, &b| min_edge[a].0.partial_cmp(&min_edge[b].0).unwrap())
            .unwrap();
        in_mst[u] = true;

        if min_edge[u].1 != u || u != 0 {
            edges.push((min_edge[u].1, u));
        }

        // Update keys for neighbors of u
        for v in 0..n {
            if !in_mst[v] {
                let dx = positions[u].0 - positions[v].0;
                let dy = positions[u].1 - positions[v].1;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist < min_edge[v].0 {
                    min_edge[v] = (dist, u);
                }
            }
        }
    }

    edges
}
```

#### Step 2: Count segment intersections

Two line segments (A1→A2) and (B1→B2) intersect if and only if A1 and A2 lie
on opposite sides of line B1B2, AND B1 and B2 lie on opposite sides of line A1A2.
Uses the CCW orientation test:

```rust
fn cross2d(ox: f64, oy: f64, ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    (ax - ox) * (by - oy) - (ay - oy) * (bx - ox)
}

fn segments_intersect(
    a1x: f64, a1y: f64, a2x: f64, a2y: f64,
    b1x: f64, b1y: f64, b2x: f64, b2y: f64,
) -> bool {
    let d1 = cross2d(b1x, b1y, b2x, b2y, a1x, a1y);
    let d2 = cross2d(b1x, b1y, b2x, b2y, a2x, a2y);
    let d3 = cross2d(a1x, a1y, a2x, a2y, b1x, b1y);
    let d4 = cross2d(a1x, a1y, a2x, a2y, b2x, b2y);

    if ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0))
        && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0))
    {
        return true;
    }

    // Collinear cases: treat as non-crossing to avoid double-counting
    false
}
```

Total crossing count: iterate all pairs of segments across all nets (O(E²) where
E = total MST edges). For PCB-scale boards with N < 500, E < ~2000 and E² < 4M,
which is acceptable for the optional-enable path.

### 5.5 Total Cost

```rust
fn total_cost(
    placement: &Placement,
    config: &SAConfig,
) -> f64 {
    let hpwl: f64 = placement.nets.iter()
        .map(|n| net_hpwl(n, &placement.components))
        .sum();

    let overlap = count_all_overlaps(&placement.components, config.default_clearance_mm)
        as f64;

    let crossings = if config.enable_net_crossings {
        total_net_crossings(placement) as f64
    } else {
        0.0
    };

    config.w_hpwl * hpwl
        + config.w_overlap * overlap
        + config.w_net_crossing * crossings
}
```

---

## 6. Incremental Cost Evaluation

Full cost re-evaluation is O(N²) for overlaps and O(nets) for HPWL. SA applies
O(N × T_steps) moves, making full re-evaluation too slow. Instead, compute the
cost delta for each move incrementally.

### 6.1 HPWL Delta

Only the nets connected to moved component(s) can change their HPWL.

For a `Displace` or `Slide` or `Rotate` move on `comp_idx`:
```
delta_hpwl = sum over nets in net_comp_index.nets_for_component(comp_idx)
               of (net_hpwl_after(net) - net_hpwl_before(net))
```

For a `Swap` of `a_idx` and `b_idx`:
```
delta_hpwl = sum over union of nets_for_component(a_idx) and nets_for_component(b_idx)
               of (net_hpwl_after(net) - net_hpwl_before(net))
```

To compute `net_hpwl_after`: temporarily apply the move to the component state
in a scratch copy, compute HPWL, then revert.

Pattern for incremental delta:
```rust
fn delta_hpwl_for_component_move(
    placement: &Placement,
    comp_idx: usize,
    new_x: f64,
    new_y: f64,
    new_theta_deg: f64,
) -> f64 {
    let nets = placement.net_comp_index.nets_for_component(comp_idx);
    let mut delta = 0.0;
    for &net_idx in nets {
        let net = &placement.nets[net_idx];
        let before = net_hpwl(net, &placement.components);
        let after = net_hpwl_with_override(net, &placement.components, comp_idx, new_x, new_y, new_theta_deg);
        delta += after - before;
    }
    delta
}

fn net_hpwl_with_override(
    net: &NetState,
    components: &[ComponentState],
    override_idx: usize,
    override_x: f64,
    override_y: f64,
    override_theta_deg: f64,
) -> f64 {
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for pin_ref in &net.pin_refs {
        let comp = &components[pin_ref.comp_idx];
        let pad = &comp.pads[pin_ref.pad_idx];
        let (x, y, theta_deg) = if pin_ref.comp_idx == override_idx {
            (override_x, override_y, override_theta_deg)
        } else {
            (comp.x, comp.y, comp.theta_deg)
        };
        let theta = theta_deg.to_radians();
        let (sin_t, cos_t) = theta.sin_cos();
        let wx = x + pad.local_x * cos_t - pad.local_y * sin_t;
        let wy = y + pad.local_x * sin_t + pad.local_y * cos_t;
        min_x = min_x.min(wx);
        max_x = max_x.max(wx);
        min_y = min_y.min(wy);
        max_y = max_y.max(wy);
    }

    if min_x.is_finite() { (max_x - min_x) + (max_y - min_y) } else { 0.0 }
}
```

For `Swap`, call this for both components with swapped positions, then also
call it for the union of their net sets. Use `net_hpwl_with_swap_override()`
which takes two component overrides simultaneously.

### 6.2 Overlap Delta

Only check components in the spatial grid neighbors of the moved component(s):

```rust
fn delta_overlap(
    placement: &Placement,
    comp_idx: usize,
    new_x: f64,
    new_y: f64,
    new_theta_deg: f64,
    clearance: f64,
) -> f64 {
    let old_comp = &placement.components[comp_idx];
    let new_comp = ComponentState { x: new_x, y: new_y, theta_deg: new_theta_deg, ..*old_comp };

    // Count overlaps before and after, only with neighbors
    let neighbors_old = placement.spatial_grid.neighbors(old_comp, clearance);
    let neighbors_new = placement.spatial_grid.neighbors(&new_comp, clearance);

    let all_neighbors: HashSet<usize> = neighbors_old.into_iter()
        .chain(neighbors_new.into_iter())
        .filter(|&i| i != comp_idx)
        .collect();

    let mut before = 0i32;
    let mut after = 0i32;
    for &ni in &all_neighbors {
        let neighbor = &placement.components[ni];
        if aabb_overlaps(old_comp, neighbor, clearance) { before += 1; }
        if aabb_overlaps(&new_comp, neighbor, clearance) { after += 1; }
    }
    (after - before) as f64
}

fn aabb_overlaps(a: &ComponentState, b: &ComponentState, clearance: f64) -> bool {
    let (ahw, ahh) = world_half_extents(a);
    let (bhw, bhh) = world_half_extents(b);
    (a.x - b.x).abs() < ahw + bhw + clearance
        && (a.y - b.y).abs() < ahh + bhh + clearance
}
```

### 6.3 Total Delta Cost for a Move

```rust
fn delta_cost(
    mv: Move,
    placement: &Placement,
    config: &SAConfig,
) -> f64 {
    match mv {
        Move::Displace { comp_idx, dx, dy } => {
            let comp = &placement.components[comp_idx];
            let nx = comp.x + dx;
            let ny = comp.y + dy;
            let d_hpwl = delta_hpwl_for_component_move(placement, comp_idx, nx, ny, comp.theta_deg);
            let d_overlap = delta_overlap(placement, comp_idx, nx, ny, comp.theta_deg, config.default_clearance_mm);
            config.w_hpwl * d_hpwl + config.w_overlap * d_overlap
        }
        Move::Rotate { comp_idx, delta_deg } => {
            let comp = &placement.components[comp_idx];
            let new_theta = (comp.theta_deg + delta_deg as f64).rem_euclid(360.0);
            let d_hpwl = delta_hpwl_for_component_move(placement, comp_idx, comp.x, comp.y, new_theta);
            let d_overlap = delta_overlap(placement, comp_idx, comp.x, comp.y, new_theta, config.default_clearance_mm);
            config.w_hpwl * d_hpwl + config.w_overlap * d_overlap
        }
        Move::Slide { comp_idx, axis, delta } => {
            let comp = &placement.components[comp_idx];
            let (nx, ny) = match axis {
                Axis::X => (comp.x + delta, comp.y),
                Axis::Y => (comp.x, comp.y + delta),
            };
            let d_hpwl = delta_hpwl_for_component_move(placement, comp_idx, nx, ny, comp.theta_deg);
            let d_overlap = delta_overlap(placement, comp_idx, nx, ny, comp.theta_deg, config.default_clearance_mm);
            config.w_hpwl * d_hpwl + config.w_overlap * d_overlap
        }
        Move::Swap { a_idx, b_idx } => {
            let a = &placement.components[a_idx];
            let b = &placement.components[b_idx];
            let (ax, ay, at) = (a.x, a.y, a.theta_deg);
            let (bx, by, bt) = (b.x, b.y, b.theta_deg);

            let d_hpwl = delta_hpwl_for_swap(placement, a_idx, b_idx, bx, by, bt, ax, ay, at);
            // Overlap delta for swap: a moves to b's position, b moves to a's position
            let d_ov_a = delta_overlap(placement, a_idx, bx, by, bt, config.default_clearance_mm);
            let d_ov_b = delta_overlap(placement, b_idx, ax, ay, at, config.default_clearance_mm);
            config.w_hpwl * d_hpwl + config.w_overlap * (d_ov_a + d_ov_b)
        }
    }
}
```

Net crossing delta (when enabled) recomputes total crossings before and after
the move. Because crossing computation is O(E²), it is always full re-evaluation
even in incremental mode — but it is only called when `enable_net_crossings =
true`. If needed for performance, crossing delta can be approximated by only
recomputing segments attached to moved component(s).

---

## 7. Move Generation

```rust
fn generate_move(
    placement: &Placement,
    config: &SAConfig,
    t: f64,
    rng: &mut impl Rng,
) -> Option<Move> {
    let movable: Vec<usize> = (0..placement.components.len())
        .filter(|&i| !placement.components[i].fixed)
        .collect();
    if movable.is_empty() {
        return None;
    }

    let p: f64 = rng.gen();
    let p_swap_thresh = config.p_displace + config.p_swap;
    let p_rot_thresh = p_swap_thresh + config.p_rotate;

    if p < config.p_displace {
        // Displacement: shrink range proportionally with T
        let range = config.displacement_range_mm * (t / config.t_initial.unwrap_or(1.0)).max(0.01);
        let comp_idx = movable[rng.gen_range(0..movable.len())];
        let dx = rng.gen_range(-range..=range);
        let dy = rng.gen_range(-range..=range);
        Some(Move::Displace { comp_idx, dx, dy })

    } else if p < p_swap_thresh {
        // Swap: pick two distinct movable components
        if movable.len() < 2 {
            return None;
        }
        let i = rng.gen_range(0..movable.len());
        let mut j = rng.gen_range(0..movable.len() - 1);
        if j >= i { j += 1; }
        Some(Move::Swap { a_idx: movable[i], b_idx: movable[j] })

    } else if p < p_rot_thresh {
        // Rotate: 90, 180, or 270 degrees added
        let comp_idx = movable[rng.gen_range(0..movable.len())];
        let delta_deg = [90i32, 180, 270][rng.gen_range(0..3)];
        Some(Move::Rotate { comp_idx, delta_deg })

    } else {
        // Slide: single axis, smaller range (fine-tuning)
        let comp_idx = movable[rng.gen_range(0..movable.len())];
        let axis = if rng.gen::<bool>() { Axis::X } else { Axis::Y };
        let range = config.displacement_range_mm
            * (t / config.t_initial.unwrap_or(1.0)).max(0.01)
            * 0.2;
        let delta = rng.gen_range(-range..=range);
        Some(Move::Slide { comp_idx, axis, delta })
    }
}
```

The `rng` parameter is `rand::rngs::SmallRng` seeded from a fixed u64 (or from
`rand::SeedableRng::from_entropy()` for non-deterministic mode). Add `rand` to
`Cargo.toml` for `autopcb-placement`:

```toml
rand = { version = "0.8", features = ["small_rng"] }
```

---

## 8. Applying and Reverting Moves

Moves are applied directly to `placement.components`. On rejection, the move
is manually reverted. The spatial grid is updated on acceptance only.

```rust
fn apply_move(placement: &mut Placement, mv: Move) {
    match mv {
        Move::Displace { comp_idx, dx, dy } => {
            placement.spatial_grid.remove(comp_idx, &placement.components[comp_idx]);
            placement.components[comp_idx].x += dx;
            placement.components[comp_idx].y += dy;
            placement.spatial_grid.insert(comp_idx, &placement.components[comp_idx]);
        }
        Move::Rotate { comp_idx, delta_deg } => {
            placement.spatial_grid.remove(comp_idx, &placement.components[comp_idx]);
            placement.components[comp_idx].theta_deg =
                (placement.components[comp_idx].theta_deg + delta_deg as f64).rem_euclid(360.0);
            placement.spatial_grid.insert(comp_idx, &placement.components[comp_idx]);
        }
        Move::Slide { comp_idx, axis, delta } => {
            placement.spatial_grid.remove(comp_idx, &placement.components[comp_idx]);
            match axis {
                Axis::X => placement.components[comp_idx].x += delta,
                Axis::Y => placement.components[comp_idx].y += delta,
            }
            placement.spatial_grid.insert(comp_idx, &placement.components[comp_idx]);
        }
        Move::Swap { a_idx, b_idx } => {
            let (ax, ay, at) = {
                let a = &placement.components[a_idx];
                (a.x, a.y, a.theta_deg)
            };
            let (bx, by, bt) = {
                let b = &placement.components[b_idx];
                (b.x, b.y, b.theta_deg)
            };
            placement.spatial_grid.remove(a_idx, &placement.components[a_idx]);
            placement.spatial_grid.remove(b_idx, &placement.components[b_idx]);
            placement.components[a_idx].x = bx;
            placement.components[a_idx].y = by;
            placement.components[a_idx].theta_deg = bt;
            placement.components[b_idx].x = ax;
            placement.components[b_idx].y = ay;
            placement.components[b_idx].theta_deg = at;
            placement.spatial_grid.insert(a_idx, &placement.components[a_idx]);
            placement.spatial_grid.insert(b_idx, &placement.components[b_idx]);
        }
    }
}

fn revert_move(placement: &mut Placement, mv: Move) {
    // Revert by applying the inverse move (no spatial grid update needed —
    // revert is called before grid was updated)
    match mv {
        Move::Displace { comp_idx, dx, dy } => {
            placement.components[comp_idx].x -= dx;
            placement.components[comp_idx].y -= dy;
        }
        Move::Rotate { comp_idx, delta_deg } => {
            placement.components[comp_idx].theta_deg =
                (placement.components[comp_idx].theta_deg - delta_deg as f64).rem_euclid(360.0);
        }
        Move::Slide { comp_idx, axis, delta } => {
            match axis {
                Axis::X => placement.components[comp_idx].x -= delta,
                Axis::Y => placement.components[comp_idx].y -= delta,
            }
        }
        Move::Swap { a_idx, b_idx } => {
            // Swap is its own inverse
            let (ax, ay, at) = {
                let a = &placement.components[a_idx];
                (a.x, a.y, a.theta_deg)
            };
            let (bx, by, bt) = {
                let b = &placement.components[b_idx];
                (b.x, b.y, b.theta_deg)
            };
            placement.components[a_idx].x = bx;
            placement.components[a_idx].y = by;
            placement.components[a_idx].theta_deg = bt;
            placement.components[b_idx].x = ax;
            placement.components[b_idx].y = ay;
            placement.components[b_idx].theta_deg = at;
        }
    }
}
```

Note: the pattern used here — apply move, then revert if rejected — avoids needing
a scratch copy. The spatial grid is only updated on acceptance (inside `apply_move`).
For rejection, `revert_move` restores component positions without touching the grid.
This means the grid and component positions are always in sync after each iteration.

---

## 9. Cooling Schedule

### 9.1 Temperature Auto-Initialization

If `config.t_initial == None`, sample 50 random moves, compute mean of
`|delta_cost|` for each, and set:
```
T_init = -mean_abs_delta / ln(0.8)
```
This ensures the initial acceptance probability is approximately 80%, which is
the standard SA starting condition (highly exploratory).

```rust
fn auto_init_temperature(
    placement: &Placement,
    config: &SAConfig,
    rng: &mut impl Rng,
) -> f64 {
    let n_samples = 50.max(placement.components.len());
    let mut total = 0.0;
    let mut count = 0usize;

    // Use a high dummy temperature so generate_move picks full-range displacements
    let dummy_t = f64::INFINITY;
    for _ in 0..n_samples {
        // Temporarily use infinity for range calculation in generate_move
        // by passing a config clone with t_initial set
        if let Some(mv) = generate_move_with_t(placement, config, dummy_t, rng) {
            let dc = delta_cost(mv, placement, config).abs();
            if dc > 0.0 {
                total += dc;
                count += 1;
            }
        }
    }

    if count == 0 {
        return 1.0;
    }
    let mean_delta = total / count as f64;
    -mean_delta / 0.2f64.ln()  // ln(0.8) ≈ -0.2231; this yields ~80% acceptance
}
```

`generate_move_with_t` is `generate_move` with the temperature parameter used
only to scale displacement range. For the sampling phase, pass `dummy_t = config.displacement_range_mm`
(i.e., full-range moves).

### 9.2 Adaptive Cooling

After each temperature step, measure the acceptance rate over the moves taken
at that temperature. Adjust `alpha` for the next step:

```rust
fn adaptive_alpha(acceptance_rate: f64) -> f64 {
    if acceptance_rate > 0.8 {
        0.90   // too hot: cool faster
    } else if acceptance_rate > 0.5 {
        0.95   // normal: standard rate
    } else if acceptance_rate > 0.1 {
        0.98   // getting cold: cool slowly
    } else {
        0.99   // nearly frozen: very slow cooling
    }
}
```

Apply: `t = adaptive_alpha(acceptance_rate) * t` after each temperature step.

### 9.3 Stopping Criteria

Stop when any of the following is true:

1. `t < config.t_frozen`
2. `temperature_step_count > config.max_temperature_steps`
3. Acceptance rate was below 1% (`< 0.01`) for 5 consecutive temperature steps

Track consecutive-low-acceptance count as a `usize` in the SA loop. Reset to 0
whenever acceptance rate rises above 1%.

---

## 10. Main SA Loop

```rust
pub fn refine_with_sa(
    initial: &PlacementResult,
    ir: &PcbIr,
    config: &SAConfig,
    fixed_designators: &HashSet<String>,
) -> Result<PlacementResult, PlacementError> {
    let start = std::time::Instant::now();
    let n = initial.components.len();
    if n == 0 {
        return Err(PlacementError::NoComponents);
    }

    let bounds = ir.board.bounds;
    let mut placement = build_placement(initial, ir, fixed_designators, config)?;

    let moves_per_temp = if config.moves_per_temp == 0 {
        10 * n
    } else {
        config.moves_per_temp
    };

    let mut rng = rand::rngs::SmallRng::seed_from_u64(0x5A_5A_5A_5A_5A_5A_5A_5Au64);

    let mut t = match config.t_initial {
        Some(t0) => t0,
        None => auto_init_temperature(&placement, config, &mut rng),
    };

    let t_init = t;
    // Store t_init back into a local variable used by generate_move for range scaling
    // (displacement_range * t / t_init)

    let mut current_cost = total_cost(&placement, config);
    let mut best_placement = placement.components.clone();
    let mut best_cost = current_cost;

    let mut snapshots = initial.snapshots.clone();
    let mut total_iters = initial.total_iterations;

    let mut temp_step = 0usize;
    let mut consecutive_frozen = 0usize;
    let snapshot_interval = (config.max_temperature_steps / 10).max(1);

    while t > config.t_frozen
        && temp_step < config.max_temperature_steps
        && consecutive_frozen < 5
    {
        let mut accepted = 0usize;
        let mut attempted = 0usize;

        for _ in 0..moves_per_temp {
            let Some(mv) = generate_move_with_t_init(&placement, config, t, t_init, &mut rng)
            else { continue };

            // Hard rejection: outside board bounds
            if violates_board_bounds(&placement, mv, bounds.min.x, bounds.min.y, bounds.max.x, bounds.max.y) {
                continue;
            }

            let dc = delta_cost(mv, &placement, config);
            attempted += 1;

            let accept = if dc < 0.0 {
                true
            } else {
                let prob = (-dc / t).exp();
                rng.gen::<f64>() < prob
            };

            if accept {
                apply_move(&mut placement, mv);
                current_cost += dc;
                accepted += 1;

                if current_cost < best_cost {
                    best_cost = current_cost;
                    best_placement = placement.components.clone();
                }
            } else {
                revert_move(&mut placement, mv);
            }

            total_iters += 1;
        }

        let acceptance_rate = if attempted > 0 {
            accepted as f64 / attempted as f64
        } else {
            0.0
        };

        if acceptance_rate < 0.01 {
            consecutive_frozen += 1;
        } else {
            consecutive_frozen = 0;
        }

        t *= adaptive_alpha(acceptance_rate);
        temp_step += 1;

        if temp_step % snapshot_interval == 0 {
            snapshots.push(PlacementIterationSnapshot {
                phase: "sa".to_string(),
                components: placement_to_states(&placement),
                note: Some(format!(
                    "T={:.4} accept={:.1}% cost={:.2}",
                    t,
                    acceptance_rate * 100.0,
                    current_cost,
                )),
            });
        }
    }

    // Restore best placement
    placement.components = best_placement;

    // Compute final HPWL and overlap violations
    let final_states = placement_to_states(&placement);
    let final_hpwl = compute_final_hpwl(&final_states, &placement.nets, &placement.components);
    let overlap_violations = count_all_overlaps(&placement.components, config.default_clearance_mm);

    snapshots.push(PlacementIterationSnapshot {
        phase: "sa_final".to_string(),
        components: final_states.clone(),
        note: Some(format!("best_cost={:.4} temp_steps={}", best_cost, temp_step)),
    });

    Ok(PlacementResult {
        status: "SA_Converged".to_string(),
        total_iterations: total_iters,
        duration_ms: initial.duration_ms + start.elapsed().as_millis(),
        components: final_states,
        snapshots,
        hpwl_estimate_mm: final_hpwl,
        overlap_violations,
    })
}
```

#### `violates_board_bounds`

```rust
fn violates_board_bounds(
    placement: &Placement,
    mv: Move,
    min_x: f64, min_y: f64, max_x: f64, max_y: f64,
) -> bool {
    match mv {
        Move::Displace { comp_idx, dx, dy } => {
            let c = &placement.components[comp_idx];
            let (hw, hh) = world_half_extents_at(c.half_w, c.half_h, c.theta_deg);
            let nx = c.x + dx;
            let ny = c.y + dy;
            nx - hw < min_x || nx + hw > max_x || ny - hh < min_y || ny + hh > max_y
        }
        Move::Slide { comp_idx, axis, delta } => {
            let c = &placement.components[comp_idx];
            let (hw, hh) = world_half_extents_at(c.half_w, c.half_h, c.theta_deg);
            let (nx, ny) = match axis {
                Axis::X => (c.x + delta, c.y),
                Axis::Y => (c.x, c.y + delta),
            };
            nx - hw < min_x || nx + hw > max_x || ny - hh < min_y || ny + hh > max_y
        }
        Move::Rotate { comp_idx, delta_deg } => {
            let c = &placement.components[comp_idx];
            let new_theta = (c.theta_deg + delta_deg as f64).rem_euclid(360.0);
            let (hw, hh) = world_half_extents_at(c.half_w, c.half_h, new_theta);
            c.x - hw < min_x || c.x + hw > max_x || c.y - hh < min_y || c.y + hh > max_y
        }
        Move::Swap { a_idx, b_idx } => {
            // Both components stay inside because they're exchanging positions with each
            // other; if both were legal before, both remain legal after swap.
            // However, if rotations differ, the world extents change. Check both.
            let a = &placement.components[a_idx];
            let b = &placement.components[b_idx];
            // a moves to b's position with a's rotation
            let (ahw, ahh) = world_half_extents_at(a.half_w, a.half_h, a.theta_deg);
            // b moves to a's position with b's rotation
            let (bhw, bhh) = world_half_extents_at(b.half_w, b.half_h, b.theta_deg);
            (b.x - ahw < min_x || b.x + ahw > max_x || b.y - ahh < min_y || b.y + ahh > max_y)
            || (a.x - bhw < min_x || a.x + bhw > max_x || a.y - bhh < min_y || a.y + bhh > max_y)
        }
    }
}

fn world_half_extents(comp: &ComponentState) -> (f64, f64) {
    world_half_extents_at(comp.half_w, comp.half_h, comp.theta_deg)
}

fn world_half_extents_at(half_w: f64, half_h: f64, theta_deg: f64) -> (f64, f64) {
    let theta = theta_deg.to_radians();
    let (sin_t, cos_t) = theta.sin_cos();
    let hw_world = half_w * cos_t.abs() + half_h * sin_t.abs();
    let hh_world = half_w * sin_t.abs() + half_h * cos_t.abs();
    (hw_world, hh_world)
}
```

#### `placement_to_states`

Converts internal `Placement` back to the public `PlacementComponentState` vec,
sorted by designator (matching `solve_placement()` sort order):

```rust
fn placement_to_states(placement: &Placement) -> Vec<PlacementComponentState> {
    let mut states: Vec<PlacementComponentState> = placement.components.iter().map(|c| {
        PlacementComponentState {
            designator: c.designator.clone(),
            x_mm: c.x,
            y_mm: c.y,
            rotation_deg: c.theta_deg,
        }
    }).collect();
    states.sort_by(|a, b| a.designator.cmp(&b.designator));
    states
}
```

---

## 11. Integration Points

### 11.1 Entry Point (public API)

```rust
pub fn refine_with_sa(
    initial: &PlacementResult,
    ir: &PcbIr,
    config: &SAConfig,
    fixed_designators: &HashSet<String>,
) -> Result<PlacementResult, PlacementError>
```

Located in `simulated_annealing.rs`, re-exported from `lib.rs`:
```rust
pub use simulated_annealing::{refine_with_sa, SAConfig};
```

### 11.2 Usage Pattern After `solve_placement()`

The calling code (e.g. in `autopcb-shell` or CLI) would:
```rust
let phase_2_result = solve_placement(&ir, &user_constraints, &config)?;

if let Some(sa_config) = &placement_config.sa_config {
    let sa_result = refine_with_sa(&phase_2_result, &ir, sa_config, &fixed_desigs)?;
    // sa_result can then be fed to a Phase 4 Solverang refinement pass
    // or returned directly to the caller
}
```

### 11.3 Post-SA Verification

After SA converges, verify constraint satisfaction:
- Check `overlap_violations` in returned `PlacementResult`
- If `overlap_violations > 0`, the caller should optionally run another
  `greedy_legalize_overlaps` pass (the existing function in `lib.rs`) or re-run
  Phase 4 refinement

SA's heavy overlap penalty (`w_overlap = 10.0`) strongly discourages violations,
but does not hard-guarantee them. Starting from Phase 2's legal state and using
hard board-bounds rejection means violations are rare in practice.

---

## 12. Performance Targets

| N components | Target time | moves_per_temp | Temperature steps |
|---|---|---|---|
| 50 | < 500 ms | 500 | up to 200 |
| 100 | < 2 s | 1000 | up to 300 |
| 200 | < 5 s | 2000 | up to 400 |

These targets assume `enable_net_crossings = false` (the default). With net
crossings enabled, expect 3-10× slower at N=100.

The spatial grid and incremental cost evaluation are essential to meeting these
targets. Without them, each of the `moves_per_temp * temp_steps` iterations
would be O(N²) (full overlap check) rather than O(k) (neighbor check).

---

## 13. Testing Strategy

### 13.1 Unit Tests (no feature flag required)

All in `simulated_annealing.rs` under `#[cfg(test)]`:

**Move generation validity:**
```rust
#[test]
fn displace_move_stays_in_bounds() {
    // Build a minimal Placement with one component at board center
    // Generate 1000 displaces, verify none exceed board bounds
    // (hard rejection must catch them)
}

#[test]
fn swap_move_exchanges_positions() {
    // Build Placement with two components A and B
    // Apply swap, verify A is at old B position and vice versa
    // Revert swap, verify original positions restored
}

#[test]
fn rotate_move_snaps_to_90_degree_multiples() {
    // Component at theta=0, apply Rotate{delta_deg:90}
    // Verify theta_deg == 90.0 after apply
    // Revert, verify theta_deg == 0.0
}
```

**HPWL correctness:**
```rust
#[test]
fn net_hpwl_two_pins_horizontal() {
    // Net with two pins at (0,0) and (5,0), no rotation
    // Expected HPWL = 5.0
}

#[test]
fn net_hpwl_with_rotation() {
    // Component at origin, rotation=90deg
    // Pad at local (1, 0) → world (0, 1) after 90deg rotation
    // Verify pin world position is computed correctly
}

#[test]
fn net_hpwl_with_override_matches_manual() {
    // Build net with two pins, verify net_hpwl_with_override returns
    // same value as net_hpwl after manually applying the override
}
```

**Metropolis acceptance probability:**
```rust
#[test]
fn metropolis_downhill_always_accepted() {
    // delta_cost < 0 must always accept
}

#[test]
fn metropolis_large_uphill_rarely_accepted() {
    // delta_cost >> T must have acceptance << 0.01
    // Sample 10000, verify count < 50
}
```

**World half-extents formula:**
```rust
#[test]
fn world_half_extents_at_zero_rotation() {
    let (hw, hh) = world_half_extents_at(3.0, 2.0, 0.0);
    assert!((hw - 3.0).abs() < 1e-10);
    assert!((hh - 2.0).abs() < 1e-10);
}

#[test]
fn world_half_extents_at_90_rotation() {
    // 90-degree rotation swaps w and h
    let (hw, hh) = world_half_extents_at(3.0, 2.0, 90.0);
    assert!((hw - 2.0).abs() < 1e-9, "hw={hw}");
    assert!((hh - 3.0).abs() < 1e-9, "hh={hh}");
}
```

**AABB overlap detection:**
```rust
#[test]
fn aabb_overlap_touching_is_not_overlap() {
    // Two 2mm×2mm components: A at (0,0), B at (2.5,0), clearance=0.5
    // Combined half-widths = 1+1+0.5 = 2.5, |dx| = 2.5 → NOT overlapping
}

#[test]
fn aabb_overlap_intersecting() {
    // A at (0,0), B at (1.5,0), clearance=0.5
    // |dx|=1.5 < 2.5 → overlapping
}
```

### 13.2 Integration Tests (behind `test-fixtures` feature)

In `crates/autopcb-placement/src/lib.rs` or a dedicated test file:

```rust
#[cfg(feature = "test-fixtures")]
#[test]
fn sa_improves_or_maintains_hpwl_vs_phase2() {
    // Load a fixture PcbDoc from data/pcbdoc/
    // Run solve_placement() to get Phase 2 result
    // Run refine_with_sa() on that result
    // Assert sa_result.hpwl_estimate_mm <= phase2_result.hpwl_estimate_mm * 1.05
    //   (SA should not significantly worsen HPWL; allow 5% margin for randomness)
}

#[cfg(feature = "test-fixtures")]
#[test]
fn sa_produces_zero_overlap_violations() {
    // Run SA on a Phase 2 result from a fixture
    // Assert sa_result.overlap_violations == 0
}
```

### 13.3 Regression Approach

When SA produces unexpected output on a fixture, extract a minimal reproduction
case as a standalone test with a hardcoded seed. Use `SmallRng::seed_from_u64(seed)`
to get deterministic behavior. Commit the regression test to
`crates/altium-format/proptest-regressions/` (or equivalent in `autopcb-placement`).

---

## 14. Full Net Crossing Implementation Detail

For completeness, the full incremental net-crossing delta when one component moves:

```rust
fn delta_net_crossings_for_component_move(
    placement: &Placement,
    comp_idx: usize,
    new_x: f64,
    new_y: f64,
    new_theta_deg: f64,
) -> i64 {
    // All MST segments in the design:
    let all_segments_before = compute_all_mst_segments(placement);

    // Apply the override
    let mut tmp_components = placement.components.clone();
    tmp_components[comp_idx].x = new_x;
    tmp_components[comp_idx].y = new_y;
    tmp_components[comp_idx].theta_deg = new_theta_deg;

    let all_segments_after: Vec<Segment> = placement.nets.iter().flat_map(|net| {
        net_mst_segments_for_components(net, &tmp_components)
    }).collect();

    count_intersections(&all_segments_after) as i64
        - count_intersections(&all_segments_before) as i64
}

struct Segment {
    x1: f64, y1: f64, x2: f64, y2: f64,
}

fn count_intersections(segments: &[Segment]) -> usize {
    let mut count = 0;
    for i in 0..segments.len() {
        for j in (i + 1)..segments.len() {
            let a = &segments[i];
            let b = &segments[j];
            if segments_intersect(a.x1, a.y1, a.x2, a.y2, b.x1, b.y1, b.x2, b.y2) {
                count += 1;
            }
        }
    }
    count
}
```

This recomputes all MST segments from scratch for each move evaluation when net
crossings are enabled. For designs with many nets (> 200), pre-compute and cache
the segments for the current placement, only recomputing segments for nets
connected to the moved component.

---

## 15. Cargo.toml Changes

Add `rand` to `crates/autopcb-placement/Cargo.toml`:

```toml
[dependencies]
autopcb-ir = { version = "0.2.0", path = "../autopcb-ir" }
solverang = { path = "/home/kiselev/git/solverang/crates/solverang", default-features = true }
serde = { version = "1.0", features = ["derive"] }
thiserror = "2"
rand = { version = "0.8", features = ["small_rng"] }
```

No other crate dependencies are required. The spatial grid, MST, and segment
intersection logic are all self-contained in `simulated_annealing.rs`.

---

## 16. Module File Structure

Files to create or modify:

- **Create**: `crates/autopcb-placement/src/simulated_annealing.rs` — all SA code
- **Modify**: `crates/autopcb-placement/src/lib.rs` — add `pub mod simulated_annealing;` and re-exports
- **Modify**: `crates/autopcb-placement/Cargo.toml` — add `rand` dependency

No other files need modification.

---

## 17. Future Enhancements (Out of Scope for This Spec)

These are documented for awareness but must NOT be implemented as part of this spec:

- **Multi-start**: Run multiple SA instances in parallel with different seeds
  (requires `rayon` dependency), keep best result.
- **Sweep-line crossing detection**: O(E log E) algorithm using event-queue;
  necessary for N > 500. See Bentley-Ottmann algorithm.
- **RUDY congestion estimation**: Assign each net a weighted congestion map cell
  contribution; sum gives per-cell demand. Cheaper than full routing for cost.
- **Board-side assignment**: Add `Mirror` move type that flips a component to the
  opposite board side (requires `side` field in `ComponentState` and side-specific
  bounds checking).
- **Weighted-average HPWL**: Alternative to LSE for Phase 1 that may yield better
  SA starting points for very asymmetric nets.
- **Pareto multi-objective**: Track a Pareto front over HPWL and net crossings;
  return the knee-point solution.
