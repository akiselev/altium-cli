# Solverang PCB Constraint Types

Concrete constraint type designs for `solverang-pcb`. Each constraint follows
the solverang v3 `Constraint` trait pattern: entity IDs, param IDs, residuals,
and sparse Jacobians.

## Solverang v3 Constraint Trait (Current API)

```rust
/// Full trait signature — all constraint implementations must provide these.
pub trait Constraint: Send + Sync {
    fn id(&self) -> ConstraintId;
    fn name(&self) -> &str;                                    // human-readable name
    fn entity_ids(&self) -> &[EntityId];                       // which entities are involved
    fn param_ids(&self) -> &[ParamId];                         // which params are read
    fn equation_count(&self) -> usize;                         // number of residual equations
    fn residuals(&self, store: &ParamStore) -> Vec<f64>;       // F(x) — must equal zero
    fn jacobian(&self, store: &ParamStore) -> Vec<(usize, ParamId, f64)>;  // sparse (row, param, value)
    fn weight(&self) -> f64 { 1.0 }                            // for soft constraints
    fn is_soft(&self) -> bool { false }
}
```

Note: `ParamStore::alloc(value, owner: EntityId) -> ParamId` uses generational
indices for use-after-free safety. Parameters are allocated with an owner entity.

Solverang also provides `#[auto_jacobian]` proc macro for automatic Jacobian
derivation from residual expressions — useful for simpler constraints where
hand-coding the Jacobian is tedious.

## Conventions

- All distances in **mm** (f64) internally — converted from Altium internal units
  (10,000 per mil) at the boundary
- **Squared distance formulations** where possible — avoids `sqrt()` in residuals,
  gives smooth Jacobians everywhere (no singularity at distance=0)
- **Slack variable inequalities**: `g(x) ≥ 0` → `g(x) - s² = 0` where `s` is
  an extra solver variable. Solverang's `InequalityConstraint` trait handles this.
  Also available: `BoundsConstraint` and `ClearanceConstraint` helpers.
- **Bounding box distance** approximates component shape as axis-aligned rectangle
  (with rotation support via OBB overlap test)


## Entity Types

### PcbComponent

The primary entity for placement. Represents a placed component with solvable
position and fixed footprint geometry.

```rust
pub struct PcbComponent {
    id: EntityId,
    x: ParamId,            // center X (solvable)
    y: ParamId,            // center Y (solvable)
    // Rotation is NOT a continuous parameter — handled discretely
    rotation: f64,          // fixed for a given solve pass (0, 90, 180, 270)
    // Bounding box in local coordinates (from footprint data)
    half_width: f64,        // half-width of bounding box
    half_height: f64,       // half-height of bounding box
    designator: String,     // e.g. "U1", "J1", "C3"
    params: [ParamId; 2],   // [x, y]
}

impl Entity for PcbComponent {
    fn id(&self) -> EntityId { self.id }
    fn params(&self) -> &[ParamId] { &self.params }
    fn name(&self) -> &str { &self.designator }
}
```

**Bounding box in world coordinates** (accounting for rotation):
```
For rotation 0°/180°: world_hw = half_width,  world_hh = half_height
For rotation 90°/270°: world_hw = half_height, world_hh = half_width
```

### PcbBoardOutline

Fixed entity representing the board boundary. Not solvable — all params are fixed.

```rust
pub struct PcbBoardOutline {
    id: EntityId,
    // Board represented as a convex polygon or AABB for now
    // Full polygon support needed for complex board shapes
    min_x: f64, min_y: f64,
    max_x: f64, max_y: f64,
    // No solvable params — this is a fixed reference
}
```

For complex board outlines (non-rectangular), distance-to-polygon computations
are needed. For initial implementation, AABB (axis-aligned bounding box) is sufficient.

### PcbNet

Not a solver entity — metadata used during constraint generation to identify
which pads belong to which net for HPWL computation.

```rust
pub struct PcbNet {
    name: String,
    pad_positions: Vec<(EntityId, ParamId, ParamId)>,  // (component, pad_x, pad_y)
}
```


## Placement Constraints (Hard)

### 1. BoardContainment

Component bounding box must be entirely inside the board outline.

**4 inequality constraints per component** (one per edge):

```rust
pub struct BoardContainment {
    id: ConstraintId,
    entity: EntityId,
    x: ParamId, y: ParamId,
    half_w: f64, half_h: f64,  // rotated bounding box half-sizes
    board_min_x: f64, board_min_y: f64,
    board_max_x: f64, board_max_y: f64,
    params: [ParamId; 2],
}

// Residuals (all must be ≥ 0, use slack variables):
// r0 = (x - half_w) - board_min_x        ← left edge inside
// r1 = board_max_x - (x + half_w)        ← right edge inside
// r2 = (y - half_h) - board_min_y        ← bottom edge inside
// r3 = board_max_y - (y + half_h)        ← top edge inside

impl Constraint for BoardContainment {
    fn id(&self) -> ConstraintId { self.id }
    fn name(&self) -> &str { "BoardContainment" }
    fn entity_ids(&self) -> &[EntityId] { std::slice::from_ref(&self.entity) }
    fn param_ids(&self) -> &[ParamId] { &self.params }
    fn equation_count(&self) -> usize { 4 }

    fn residuals(&self, store: &ParamStore) -> Vec<f64> {
        let x = store.get(self.x);
        let y = store.get(self.y);
        vec![
            (x - self.half_w) - self.board_min_x,
            self.board_max_x - (x + self.half_w),
            (y - self.half_h) - self.board_min_y,
            self.board_max_y - (y + self.half_h),
        ]
    }

    fn jacobian(&self, _store: &ParamStore) -> Vec<(usize, ParamId, f64)> {
        vec![
            (0, self.x, 1.0),   // dr0/dx = 1
            (1, self.x, -1.0),  // dr1/dx = -1
            (2, self.y, 1.0),   // dr2/dy = 1
            (3, self.y, -1.0),  // dr3/dy = -1
        ]
    }
}
```

### 2. ComponentClearance

Minimum distance between two component bounding boxes.

**Approach**: Use the **signed distance between AABBs**. For two axis-aligned
rectangles, the separation along each axis is:
```
sep_x = |cx_A - cx_B| - (hw_A + hw_B)
sep_y = |cy_A - cy_B| - (hh_A + hh_B)
```
The gap is `max(sep_x, sep_y)` when separated, and `max(sep_x, sep_y)` is
negative when overlapping.

**Problem**: `max()` is not differentiable. Use **smooth max** (log-sum-exp):
```
smooth_max(a, b) = (1/γ) * ln(exp(γa) + exp(γb))    where γ ≈ 10
```

Or simpler: use **squared overlap penalty** for each axis independently.

```rust
pub struct ComponentClearance {
    id: ConstraintId,
    entities: [EntityId; 2],
    x1: ParamId, y1: ParamId,  // component A center
    x2: ParamId, y2: ParamId,  // component B center
    combined_hw: f64,           // hw_A + hw_B + gap
    combined_hh: f64,           // hh_A + hh_B + gap
    params: [ParamId; 4],
}

// Two inequality residuals (both must be ≥ 0):
// Either X-separation OR Y-separation must be ≥ 0
// But we can't express OR with equalities...
//
// Alternative: use L∞ distance (Chebyshev) formulation:
// max(|dx| - combined_hw, |dy| - combined_hh) ≥ 0
//
// Simplest correct approach: squared-distance penalty
// r = (dx² + dy²) - min_dist²     [≥ 0]
// where min_dist is the minimum center-to-center distance
//
// But this is center-to-center, not edge-to-edge...
//
// For AABB clearance, the correct formulation uses
// TWO constraints (one per axis):
// r0 = |x2 - x1| - combined_hw    [≥ 0 via slack]
// r1 = |y2 - y1| - combined_hh    [≥ 0 via slack]
//
// These are checked as: at least one must be ≥ 0.
// This is a disjunctive constraint — not directly expressible.
//
// PRACTICAL APPROACH: Use a smooth penalty function.
// See objectives.rs for the actual formulation.
```

**Important**: An ellipse-based exclusion residual is a **smooth heuristic**, not an
exact non-overlap proof for rectangles. It can admit some AABB-overlap cases.

Use it only as a soft guide during global optimization; enforce legality with an
exact rectangle overlap test in legalization/SA hard-reject steps.

Exact legality check for axis-aligned boxes:
```
overlap_x = |dx| < (hw_A + hw_B + gap)
overlap_y = |dy| < (hh_A + hh_B + gap)
illegal = overlap_x && overlap_y
```

Smooth heuristic residual (optional soft term):

```rust
// Elliptical exclusion: the center-to-center vector must lie outside
// an ellipse with semi-axes (combined_hw, combined_hh).
//
// r = 1 - (dx/combined_hw)² - (dy/combined_hh)²    [≤ 0]
// Equivalently: (dx/combined_hw)² + (dy/combined_hh)² - 1 ≥ 0
//
// This is smooth and differentiable, but not equivalent to AABB non-overlap.

impl Constraint for ComponentClearance {
    fn id(&self) -> ConstraintId { self.id }
    fn name(&self) -> &str { "ComponentClearance" }
    fn entity_ids(&self) -> &[EntityId] { &self.entities }
    fn param_ids(&self) -> &[ParamId] { &self.params }
    fn equation_count(&self) -> usize { 1 }

    fn residuals(&self, store: &ParamStore) -> Vec<f64> {
        let dx = store.get(self.x2) - store.get(self.x1);
        let dy = store.get(self.y2) - store.get(self.y1);
        let nx = dx / self.combined_hw;
        let ny = dy / self.combined_hh;
        // Must be ≥ 0 (use slack variable wrapping)
        vec![nx * nx + ny * ny - 1.0]
    }

    fn jacobian(&self, store: &ParamStore) -> Vec<(usize, ParamId, f64)> {
        let dx = store.get(self.x2) - store.get(self.x1);
        let dy = store.get(self.y2) - store.get(self.y1);
        let hw2 = self.combined_hw * self.combined_hw;
        let hh2 = self.combined_hh * self.combined_hh;
        vec![
            (0, self.x1, -2.0 * dx / hw2),
            (0, self.y1, -2.0 * dy / hh2),
            (0, self.x2, 2.0 * dx / hw2),
            (0, self.y2, 2.0 * dy / hh2),
        ]
    }
}
```

### 3. BoardEdgeClearance

Minimum distance from component bounding box to board edge.
Same as BoardContainment but with `gap` added to the margin.

```rust
// Identical to BoardContainment but board bounds shrunk by `gap`:
// effective_min_x = board_min_x + gap
// effective_max_x = board_max_x - gap
// etc.
```

### 4. EdgePlacement

Pin a component to a board edge with optional inset and alignment.

```rust
pub struct EdgePlacement {
    id: ConstraintId,
    entity: EntityId,
    x: ParamId, y: ParamId,
    edge: BoardEdge,           // Top, Bottom, Left, Right
    inset: f64,                // distance from edge
    board_min_x: f64, board_min_y: f64,
    board_max_x: f64, board_max_y: f64,
    half_w: f64, half_h: f64,
    params: [ParamId; 2],
}

pub enum BoardEdge { Top, Bottom, Left, Right }

// 1 equality constraint (pins the perpendicular axis)
// + optional equality for alignment (pins the parallel axis)

impl Constraint for EdgePlacement {
    fn id(&self) -> ConstraintId { self.id }
    fn name(&self) -> &str { "EdgePlacement" }
    fn entity_ids(&self) -> &[EntityId] { std::slice::from_ref(&self.entity) }
    fn param_ids(&self) -> &[ParamId] { &self.params }
    fn equation_count(&self) -> usize { 1 }

    fn residuals(&self, store: &ParamStore) -> Vec<f64> {
        match self.edge {
            // Component's outer edge should be `inset` from board edge
            BoardEdge::Top => {
                let y = store.get(self.y);
                vec![y + self.half_h - (self.board_max_y - self.inset)]
            }
            BoardEdge::Bottom => {
                let y = store.get(self.y);
                vec![y - self.half_h - (self.board_min_y + self.inset)]
            }
            BoardEdge::Left => {
                let x = store.get(self.x);
                vec![x - self.half_w - (self.board_min_x + self.inset)]
            }
            BoardEdge::Right => {
                let x = store.get(self.x);
                vec![x + self.half_w - (self.board_max_x - self.inset)]
            }
        }
    }

    fn jacobian(&self, _store: &ParamStore) -> Vec<(usize, ParamId, f64)> {
        match self.edge {
            BoardEdge::Top | BoardEdge::Bottom => vec![(0, self.y, 1.0)],
            BoardEdge::Left | BoardEdge::Right => vec![(0, self.x, 1.0)],
        }
    }
}
```

### 5. RegionContainment

Component center must be within a specified region (rectangle, circle, or polygon).

```rust
pub struct RectRegionContainment {
    id: ConstraintId,
    entity: EntityId,
    x: ParamId, y: ParamId,
    region_min_x: f64, region_min_y: f64,
    region_max_x: f64, region_max_y: f64,
    params: [ParamId; 2],
}

// 4 inequality constraints (same pattern as BoardContainment)
// r0 = x - region_min_x ≥ 0
// r1 = region_max_x - x ≥ 0
// r2 = y - region_min_y ≥ 0
// r3 = region_max_y - y ≥ 0
```

**Named region shortcuts** (computed from board dimensions):
- `center` → central 50% of board area
- `top_half`, `bottom_half`, `left_half`, `right_half`
- `quadrant_tl`, `quadrant_tr`, `quadrant_bl`, `quadrant_br`

### 6. DirectionalOrdering

One component must be to the left/right/above/below another with minimum gap.

```rust
pub struct DirectionalOrdering {
    id: ConstraintId,
    entities: [EntityId; 2],
    // Component A must be {direction} of component B
    a_x: ParamId, a_y: ParamId,
    b_x: ParamId, b_y: ParamId,
    direction: Direction,      // LeftOf, RightOf, Above, Below
    gap: f64,                  // minimum gap between bounding box edges
    a_half_w: f64, a_half_h: f64,
    b_half_w: f64, b_half_h: f64,
    params: [ParamId; 4],
}

pub enum Direction { LeftOf, RightOf, Above, Below }

// 1 inequality constraint:
// LeftOf:  b_x - b_hw - gap - (a_x + a_hw) ≥ 0
// RightOf: a_x - a_hw - gap - (b_x + b_hw) ≥ 0
// Above:   a_y - a_hh - gap - (b_y + b_hh) ≥ 0
// Below:   b_y - b_hh - gap - (a_y + a_hh) ≥ 0
```

### 7. NearConstraint

Component A must be within max_distance of component B (center-to-center).

```rust
pub struct NearConstraint {
    id: ConstraintId,
    entities: [EntityId; 2],
    a_x: ParamId, a_y: ParamId,
    b_x: ParamId, b_y: ParamId,
    max_distance_sq: f64,
    params: [ParamId; 4],
}

// 1 inequality: max_distance² - (dx² + dy²) ≥ 0
```

### 8. GroupSeparation

Minimum distance between the convex hulls of two component groups.

```rust
pub struct GroupSeparation {
    id: ConstraintId,
    // All component param IDs in group A and group B
    group_a: Vec<(ParamId, ParamId)>,  // [(x, y), ...]
    group_b: Vec<(ParamId, ParamId)>,
    min_gap: f64,
    // ... entity_ids and param_ids computed from groups
}

// Approximation: minimum pairwise center-to-center distance
// between any component in A and any component in B must ≥ min_gap.
// This generates O(|A| × |B|) inequality constraints.
//
// Better approximation: distance between group centroids ≥ min_gap + sum_of_radii
```

### 9. FixedPosition

Pin a component to an exact position (used for connectors with known locations).

```rust
pub struct FixedPosition {
    id: ConstraintId,
    entity: EntityId,
    x: ParamId, y: ParamId,
    target_x: f64, target_y: f64,
    params: [ParamId; 2],
}

// 2 equality constraints:
// r0 = x - target_x = 0
// r1 = y - target_y = 0
```


## Optimization Objectives (Soft)

### 10. SmoothHPWL (Half-Perimeter Wire Length)

The primary placement quality metric. For each net, HPWL estimates the minimum
routing length as: `(max_x - min_x) + (max_y - min_y)` over all pins in the net.

**Problem**: `min` and `max` are not differentiable.

**Solution**: Log-sum-exp (LSE) smooth approximation:
```
smooth_max(x_1, ..., x_n) = (1/γ) · ln(Σ exp(γ · x_i))
smooth_min(x_1, ..., x_n) = -(1/γ) · ln(Σ exp(-γ · x_i))
```
where `γ` is a sharpness parameter (higher = closer to true min/max, but
less smooth). Typical: `γ = 5` to `20`.

```rust
pub struct SmoothHPWL {
    id: ConstraintId,
    // Pin positions (component center + pad offset, accounting for rotation)
    pin_xs: Vec<ParamId>,  // x-coordinates of all pins in the net
    pin_ys: Vec<ParamId>,  // y-coordinates
    gamma: f64,             // sharpness (default: 10.0)
    weight: f64,            // soft constraint weight (default: 0.01)
    entity_ids: Vec<EntityId>,
    param_ids: Vec<ParamId>,
}

impl Constraint for SmoothHPWL {
    fn id(&self) -> ConstraintId { self.id }
    fn name(&self) -> &str { "SmoothHPWL" }
    fn entity_ids(&self) -> &[EntityId] { &self.entity_ids }
    fn param_ids(&self) -> &[ParamId] { &self.param_ids }
    fn equation_count(&self) -> usize { 2 }  // x-span + y-span

    fn residuals(&self, store: &ParamStore) -> Vec<f64> {
        let xs: Vec<f64> = self.pin_xs.iter().map(|p| store.get(*p)).collect();
        let ys: Vec<f64> = self.pin_ys.iter().map(|p| store.get(*p)).collect();

        let x_span = lse_max(&xs, self.gamma) - lse_min(&xs, self.gamma);
        let y_span = lse_max(&ys, self.gamma) - lse_min(&ys, self.gamma);

        // Return weighted residuals (soft objective)
        vec![self.weight * x_span, self.weight * y_span]
    }

    fn jacobian(&self, store: &ParamStore) -> Vec<(usize, ParamId, f64)> {
        let xs: Vec<f64> = self.pin_xs.iter().map(|p| store.get(*p)).collect();
        let ys: Vec<f64> = self.pin_ys.iter().map(|p| store.get(*p)).collect();

        let mut jac = Vec::new();

        // d(smooth_max)/dx_i = exp(γ·x_i) / Σ exp(γ·x_j)  (softmax)
        // d(smooth_min)/dx_i = exp(-γ·x_i) / Σ exp(-γ·x_j)

        let max_weights = softmax(&xs, self.gamma);
        let min_weights = softmax_neg(&xs, self.gamma);
        for (i, pid) in self.pin_xs.iter().enumerate() {
            // d(x_span)/dx_i = max_weight_i - min_weight_i
            jac.push((0, *pid, self.weight * (max_weights[i] - min_weights[i])));
        }

        let max_weights_y = softmax(&ys, self.gamma);
        let min_weights_y = softmax_neg(&ys, self.gamma);
        for (i, pid) in self.pin_ys.iter().enumerate() {
            jac.push((1, *pid, self.weight * (max_weights_y[i] - min_weights_y[i])));
        }

        jac
    }

    fn weight(&self) -> f64 { self.weight }
    fn is_soft(&self) -> bool { true }
}

fn lse_max(vals: &[f64], gamma: f64) -> f64 {
    let max_v = vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let sum: f64 = vals.iter().map(|&v| ((v - max_v) * gamma).exp()).sum();
    max_v + sum.ln() / gamma
}

fn lse_min(vals: &[f64], gamma: f64) -> f64 {
    -lse_max(&vals.iter().map(|&v| -v).collect::<Vec<_>>(), gamma)
}

fn softmax(vals: &[f64], gamma: f64) -> Vec<f64> {
    let max_v = vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let exps: Vec<f64> = vals.iter().map(|&v| ((v - max_v) * gamma).exp()).collect();
    let sum: f64 = exps.iter().sum();
    exps.iter().map(|&e| e / sum).collect()
}

fn softmax_neg(vals: &[f64], gamma: f64) -> Vec<f64> {
    let min_v = vals.iter().cloned().fold(f64::INFINITY, f64::min);
    let exps: Vec<f64> = vals.iter().map(|&v| (-(v - min_v) * gamma).exp()).collect();
    let sum: f64 = exps.iter().sum();
    exps.iter().map(|&e| e / sum).collect()
}
```

**Pin position computation**: Pads have offsets from component center. When
the component rotates, the pad offset rotates too:
```
pad_world_x = comp_x + pad_local_x * cos(θ) - pad_local_y * sin(θ)
pad_world_y = comp_y + pad_local_x * sin(θ) + pad_local_y * cos(θ)
```
Since rotation is fixed (discrete), `cos(θ)` and `sin(θ)` are constants,
making the Jacobian simple (just `∂pad_world_x/∂comp_x = 1`).


## DRC Verification Constraints

These constraints are used in DRC mode where all positions are fixed.
The residual value tells us the **violation magnitude** (negative = violation).

### 11. CopperClearance (Rule ID 0)

```rust
pub struct CopperClearance {
    // Between two copper objects (tracks, pads, vias, fills)
    // Distance computation depends on object type pair:
    // - pad-pad: center distance - (radius_A + radius_B)
    // - track-pad: point-to-segment distance - (half_width + radius)
    // - track-track: segment-to-segment distance - (half_width_A + half_width_B)
    // etc.
    object_a: CopperObject,
    object_b: CopperObject,
    min_gap: f64,
}

pub enum CopperObject {
    Pad { x: f64, y: f64, radius: f64 },
    Via { x: f64, y: f64, radius: f64 },
    TrackSegment { x1: f64, y1: f64, x2: f64, y2: f64, half_width: f64 },
    // Fill, Region (polygon) — more complex distance functions
}
```

### 12. TrackWidthBounds (Rule ID 2)

```rust
pub struct TrackWidthBounds {
    width: f64,         // actual track width
    min_width: f64,     // MINLIMIT
    max_width: f64,     // MAXLIMIT
}
// r0 = width - min_width ≥ 0
// r1 = max_width - width ≥ 0
```

### 13. AnnularRingMin (Rule ID 19)

```rust
pub struct AnnularRingMin {
    via_diameter: f64,
    hole_diameter: f64,
    min_ring: f64,
}
// r = (via_diameter - hole_diameter) / 2 - min_ring ≥ 0
```


## Constraint Generation Pipeline

```rust
fn generate_constraints(
    placement_model: &PlacementModel,
    pcbdoc: &PcbDoc,
) -> ConstraintSystem {
    let mut system = ConstraintSystem::new();
    // Or with custom config:
    // let mut system = ConstraintSystem::with_config(SystemConfig {
    //     lm_config: LMConfig::robust(),
    //     solver_config: SolverConfig::default(),
    // });

    // 1. Create entities for each component
    // Each entity's params are allocated via system.alloc_param(value, owner_entity_id)
    let components = create_component_entities(&mut system, placement_model, pcbdoc);

    // 2. Board containment (every component)
    let board = extract_board_outline(pcbdoc);
    for comp in &components {
        system.add_constraint(Box::new(BoardContainment::new(
            system.alloc_constraint_id(),
            comp, &board,
        )));
    }

    // 3. Component clearance (pairwise, O(n²) — use spatial indexing for large N)
    let clearance = placement_model.default_clearance;
    for (i, a) in components.iter().enumerate() {
        for b in &components[i+1..] {
            system.add_constraint(Box::new(ComponentClearance::new(
                system.alloc_constraint_id(),
                a, b, clearance,
            )));
        }
    }

    // 4. User constraints from spec
    for constraint in &placement_model.constraints {
        match constraint {
            SpecConstraint::EdgePlace(designator, edge, inset) => { ... }
            SpecConstraint::LeftOf(a, b, gap) => { ... }
            SpecConstraint::Near(a, b, max_dist) => { ... }
            SpecConstraint::Region(designator, region) => { ... }
            // ...
        }
    }

    // 5. HPWL objectives (per net)
    if placement_model.optimize_ratsnest {
        let netlist = extract_netlist(pcbdoc);
        for net in &netlist {
            if net.pins.len() >= 2 {
                system.add_constraint(Box::new(SmoothHPWL::new(
                    system.alloc_constraint_id(),
                    &net, &components,
                    placement_model.ratsnest_weight,
                )));
            }
        }
    }

    // 6. Solve — returns SystemResult with per-cluster results
    // let result = system.solve();
    // match result.status {
    //     SystemStatus::Solved => { /* read final values from system.params() */ }
    //     SystemStatus::PartiallySolved => { /* some clusters didn't converge */ }
    //     SystemStatus::DiagnosticFailure(issues) => { /* redundant/conflicting constraints */ }
    // }

    system
}
```


## Open Design Questions

### Q1: Oriented Bounding Boxes vs AABB

Components can be at 0°, 90°, 180°, 270°. For clearance checking between
rotated components, should we:
- (a) Use AABB of the rotated component (simpler but wasteful for 45° cases)
- (b) Use OBB (oriented bounding box) with separating axis theorem
- (c) Use the convex hull of the footprint pads

**Recommendation**: (a) for initial implementation — 90° rotation just swaps
width/height, so AABB is exact for 0°/90° cases.

### Q2: Large Board Scalability

Pairwise clearance is O(n²). For 500 components, that's 125,000 constraints.
Options:
- Spatial indexing (R-tree) to prune distant pairs
- Hierarchical: check group-to-group first, then pairwise within close groups
- Grid-based: partition board into cells, only check same/adjacent cells

### Q3: Thermal Grouping Objective

Beyond HPWL, thermal placement wants heat-generating components (voltage
regulators, power FETs) distributed rather than clustered. Formulate as:
```
r = weight * (1 / dist(hot_A, hot_B))    [soft, minimize]
```
This pushes hot components apart. Needs user annotation of which components
are heat sources.

### Q4: Obstacle Avoidance

Board may have keepout zones, mounting holes, cutouts. These are fixed
obstacles that components must avoid. Represent as additional containment
constraints with inverted sense (component center must be OUTSIDE the
obstacle polygon + clearance).
