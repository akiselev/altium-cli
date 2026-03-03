# Rotation & Ratsnest: The Hard Problems

How to model discrete component rotation and net-aware placement optimization
within solverang's continuous least-squares framework.


## 1. The Coupled Problem

A placed component has:
- **Position**: `(cx, cy)` — continuous, solvable
- **Rotation**: `θ` — discrete `{0°, 90°, 180°, 270°}`, problematic
- **Pad list**: `{(lx_j, ly_j)}` — fixed local coordinates per pad
- **Bounding box**: `(hw, hh)` — half-width, half-height in local frame

Each **net** connects a set of pads across different components. The total wire
length (HPWL) depends on the **world positions** of all pads, which depend on
both `(cx, cy)` AND `θ`:

```
pad_world_x = cx + lx·cos(θ) - ly·sin(θ)
pad_world_y = cy + lx·sin(θ) + ly·cos(θ)
```

This coupling means rotation affects wire length, and optimal position depends
on rotation. We can't solve them independently.


## 2. Pad World Position Math

### Fixed Rotation (θ is a constant)

At the four cardinal rotations, the trig simplifies to exact integers:

| θ    | cos(θ) | sin(θ) | pad_world_x        | pad_world_y        |
|------|--------|--------|--------------------|--------------------|
| 0°   | 1      | 0      | `cx + lx`          | `cy + ly`          |
| 90°  | 0      | 1      | `cx - ly`          | `cy + lx`          |
| 180° | -1     | 0      | `cx - lx`          | `cy - ly`          |
| 270° | 0      | -1     | `cx + ly`          | `cy - lx`          |

**Key property**: When θ is fixed, pad world position is **linear** in `(cx, cy)`:
```
∂(pad_world_x)/∂cx = 1      (always, regardless of θ)
∂(pad_world_x)/∂cy = 0
∂(pad_world_y)/∂cx = 0
∂(pad_world_y)/∂cy = 1      (always, regardless of θ)
```

The pad offset `(lx·cos(θ) - ly·sin(θ), lx·sin(θ) + ly·cos(θ))` is just a
constant translation. This makes the Jacobian trivial.

### Continuous Rotation (θ is a solver variable)

When θ is a free variable, pad position is **nonlinear** in θ:
```
∂(pad_world_x)/∂θ = -lx·sin(θ) - ly·cos(θ)
∂(pad_world_y)/∂θ =  lx·cos(θ) - ly·sin(θ)
```

These are smooth and well-defined everywhere. The Jacobian is more complex but
presents no numerical difficulty for LM.


## 3. Three Approaches to Rotation

### Approach A: Enumerate + Solve (Fixed Rotation)

**Strategy**: For each combination of component rotations, solve the continuous
placement problem (x, y only), then pick the best.

```
for each rotation_assignment in enumerate_rotations(components):
    fix all θ_i to rotation_assignment[i]
    solve for (x_i, y_i) only
    compute HPWL
    if HPWL < best: save solution
```

**Complexity**: `Π K_i` where `K_i` = number of allowed rotations for component i.

| Component Type | Typical K | Note |
|---------------|-----------|------|
| Edge connector | 1 | Rotation fixed by edge |
| Through-hole connector | 1-2 | Usually fixed |
| QFP/BGA IC | 4 | 0°/90°/180°/270° |
| SOT-23/SOIC | 2 | 0°/180° (symmetric-ish) |
| 2-pin passive (0402, 0603) | 2 | 0°/90° (symmetric) |
| LED | 2 | 0°/180° (polarity matters) |

**Worst case**: 20 ICs × 4 options = 4²⁰ ≈ 10¹² — too many.

**Mitigation**: Greedy or beam search:
1. Sort components by net connectivity (most-connected first)
2. For each component, try all K rotations, pick the one that minimizes HPWL
3. Fix that rotation, move to next component
4. Optionally: 2-opt improvement (flip pairs)

Greedy complexity: `Σ K_i` solves ≈ 40-80 solves × 5ms ≈ 200-400ms. Acceptable.

**Pros**: Simple, exact (each sub-solve is a well-conditioned continuous problem).
**Cons**: Greedy may miss globally optimal rotation assignment.

### Approach B: Continuous Relaxation + Discretization Constraint

**Strategy**: Make θ a continuous solver variable and add a constraint that
penalizes non-90° angles.

**The sin(2θ) trick**:
```
sin(2θ) = 0  ⟺  θ ∈ {0, π/2, π, 3π/2}
```

This is an equality constraint with **exactly** the right zeros. Residual and
Jacobian:
```
residual = sin(2θ)
jacobian = 2·cos(2θ)    (w.r.t. θ)
```

The solver naturally converges to the nearest 90° angle.

Practical caveat: at `θ = 45° + k·90°`, residual is non-zero but the Jacobian is zero,
which can stall gradient-based steps. Treat this as an experimental relaxation, not the
default production path.

**Entity with 3 parameters**:
```rust
pub struct PcbComponentContinuous {
    id: EntityId,
    x: ParamId,       // center X
    y: ParamId,       // center Y
    theta: ParamId,   // rotation (radians, continuous)
    pads: Vec<PadLocal>,
    half_width: f64,
    half_height: f64,
    designator: String,
    params: [ParamId; 3],  // [x, y, θ]
}

struct PadLocal {
    local_x: f64,
    local_y: f64,
    net_index: Option<usize>,
}
```

**Discretization constraint**:
```rust
pub struct RotationDiscretize {
    id: ConstraintId,
    entity: EntityId,
    theta: ParamId,
    params: [ParamId; 1],
}

impl Constraint for RotationDiscretize {
    fn id(&self) -> ConstraintId { self.id }
    fn name(&self) -> &str { "RotationDiscretize" }
    fn entity_ids(&self) -> &[EntityId] { std::slice::from_ref(&self.entity) }
    fn param_ids(&self) -> &[ParamId] { &self.params }
    fn equation_count(&self) -> usize { 1 }

    fn residuals(&self, store: &ParamStore) -> Vec<f64> {
        let theta = store.get(self.theta);
        vec![f64::sin(2.0 * theta)]
    }

    fn jacobian(&self, store: &ParamStore) -> Vec<(usize, ParamId, f64)> {
        let theta = store.get(self.theta);
        vec![(0, self.theta, 2.0 * f64::cos(2.0 * theta))]
    }
}
```

**Bounding box under continuous rotation**:
The AABB of a rotated rectangle with half-size (hw, hh):
```
world_hw = |hw·cos(θ)| + |hh·sin(θ)|
world_hh = |hw·sin(θ)| + |hh·cos(θ)|
```

At 90° increments, this is exact (just swaps hw/hh). At intermediate angles,
the AABB is larger (worst case at 45°: both dimensions equal `(hw+hh)/√2`).

For clearance constraints, use the smooth AABB:
```
world_hw(θ) = hw·|cos(θ)| + hh·|sin(θ)|
```

The derivative of `|cos(θ)|` is `-sign(cos(θ))·sin(θ)`, which is discontinuous
at 90° boundaries. But since sin(2θ)=0 forces us to those boundaries, the
clearance constraints only need to work AT 0°/90°/180°/270° where the derivative
is well-defined.

**Pros**: Solver jointly optimizes position and rotation; HPWL directly
influences rotation choice.
**Cons**: Nonlinear, more variables (3N vs 2N), potential convergence to wrong
local minimum.

### Approach C: Two-Phase (Recommended)

**Strategy**: Combine A and B. Use continuous θ to find a good rotation
assignment, then refine with fixed rotations.

```
Phase 1: Continuous relaxation
    Variables: (x_i, y_i, θ_i) per component — 3N variables
    Constraints: all placement constraints + sin(2θ_i)=0 per component
    Objective: HPWL (soft)
    Initialize: θ_i = 0 (or random from allowed set)
    → Produces approximate (x, y, θ) where θ ≈ some multiple of 90°

Phase 2: Snap + refine
    For each component i:
        Round θ_i to nearest allowed rotation from user spec
    Fix all θ_i (remove from variable set)
    Re-solve with 2N variables (x_i, y_i only)
    → Produces final placement

Phase 3 (optional): Local improvement
    For each component i (sorted by most-connected-to-unplaced):
        Try each allowed rotation:
            Fix θ_i to candidate
            Re-solve x, y for all components
            Record HPWL
        Keep best rotation for component i
    Repeat until no improvement (or max iterations)
```

**Why two-phase works well**:
- Phase 1's continuous θ lets the HPWL gradient "vote" for the best rotation
- The sin(2θ)=0 constraint ensures θ converges to a cardinal angle
- Phase 2's fixed-θ solve is fast (2N variables, all linear pad offsets)
- Phase 3 is optional polish, ~2-4 extra solves per component

**Default implementation order**: ship fixed-rotation + discrete rotation search first;
add continuous-θ relaxation only as an optional advanced mode.

**Expected performance** (50 components, 200 nets):
- Phase 1: 150 variables, ~1000 constraints → ~50ms
- Phase 2: 100 variables, ~800 constraints → ~10ms
- Phase 3: ~100 extra solves × 10ms → ~1s (optional)


## 4. HPWL: Smooth Half-Perimeter Wire Length

### The Problem

For net N with pins at world positions `{(wx_1, wy_1), ..., (wx_k, wy_k)}`:
```
HPWL(N) = (max(wx_i) - min(wx_i)) + (max(wy_i) - min(wy_i))
```

`min` and `max` are not differentiable (derivative undefined at the transition
point where two values tie for the max/min).

### Log-Sum-Exp (LSE) Smooth Approximation

```
LSE_max(v, γ) = (1/γ) · ln( Σ exp(γ · v_i) )
LSE_min(v, γ) = -(1/γ) · ln( Σ exp(-γ · v_i) )
```

where `γ > 0` is the **sharpness** parameter:
- `γ → ∞`: approaches true max/min (exact but non-smooth)
- `γ → 0`: approaches arithmetic mean (smooth but inaccurate)
- Practical range: `γ ∈ [2, 20]`

**Smooth HPWL**:
```
HPWL_smooth(N) = LSE_max(wx, γ) - LSE_min(wx, γ)
               + LSE_max(wy, γ) - LSE_min(wy, γ)
```

**Numerical stability**: Use the log-sum-exp trick to avoid overflow:
```
LSE_max(v, γ) = v_max + (1/γ) · ln( Σ exp(γ · (v_i - v_max)) )
```
where `v_max = max(v_i)` is the hard max (for stability, not differentiability).

### Derivatives (Softmax Weights)

The derivative of `LSE_max` w.r.t. `v_j` is the **softmax** weight:

```
∂LSE_max(v, γ)/∂v_j = exp(γ · v_j) / Σ exp(γ · v_i) = softmax(v, γ)_j
```

The derivative of `LSE_min` w.r.t. `v_j` is the **negative softmax**:
```
∂LSE_min(v, γ)/∂v_j = exp(-γ · v_j) / Σ exp(-γ · v_i)
```

**Total HPWL gradient w.r.t. pin world position `wx_j`**:
```
∂HPWL_smooth/∂wx_j = softmax_max(wx, γ)_j - softmax_min(wx, γ)_j
```

This is elegant: each pin gets a "pull force" proportional to how close it is
to the net's bounding box edge. Pins near the max are pulled inward (positive
gradient = solver wants to decrease), pins near the min are pushed inward too.

### Gamma (γ) Selection

| γ Value | Approximation Quality | Smoothness | Use Case |
|---------|----------------------|------------|----------|
| 2 | Poor (mean-like) | Very smooth | Initial iterations |
| 5 | Moderate | Smooth | General use |
| 10 | Good | Moderate | Refinement |
| 20 | Excellent | Sharp | Final polish |

**Adaptive γ**: Start with γ=2 for Phase 1 (helps convergence from poor initial
guess), increase to γ=10 for Phase 2 (more accurate HPWL when near solution).
This is standard in VLSI analytical placement (ePlace, DREAMPlace use this).


## 5. Full HPWL Constraint with Rotation

### Fixed Rotation (θ constant)

When rotation is fixed, pad world positions are linear in component position:

```rust
pub struct SmoothHPWL {
    id: ConstraintId,
    /// For each pin in the net: (component_x_param, component_y_param, pad_offset_x, pad_offset_y)
    pins: Vec<PinInfo>,
    gamma: f64,
    weight: f64,
    entity_ids: Vec<EntityId>,
    param_ids: Vec<ParamId>,
    /// Note: Consider using #[auto_jacobian] macro for simpler constraints.
    /// For SmoothHPWL the hand-coded Jacobian is preferred due to the
    /// softmax structure, which auto_jacobian may not optimize well.
}

struct PinInfo {
    comp_x: ParamId,
    comp_y: ParamId,
    // Precomputed pad offset in world frame (depends on fixed θ)
    offset_x: f64,   // lx·cos(θ) - ly·sin(θ)
    offset_y: f64,   // lx·sin(θ) + ly·cos(θ)
}

impl Constraint for SmoothHPWL {
    fn id(&self) -> ConstraintId { self.id }
    fn name(&self) -> &str { "SmoothHPWL" }
    fn entity_ids(&self) -> &[EntityId] { &self.entity_ids }
    fn param_ids(&self) -> &[ParamId] { &self.param_ids }
    fn equation_count(&self) -> usize { 2 }  // X-span + Y-span

    fn residuals(&self, store: &ParamStore) -> Vec<f64> {
        // Compute world positions of all pins
        let wxs: Vec<f64> = self.pins.iter()
            .map(|p| store.get(p.comp_x) + p.offset_x)
            .collect();
        let wys: Vec<f64> = self.pins.iter()
            .map(|p| store.get(p.comp_y) + p.offset_y)
            .collect();

        let x_span = lse_max(&wxs, self.gamma) - lse_min(&wxs, self.gamma);
        let y_span = lse_max(&wys, self.gamma) - lse_min(&wys, self.gamma);

        vec![self.weight * x_span, self.weight * y_span]
    }

    fn jacobian(&self, store: &ParamStore) -> Vec<(usize, ParamId, f64)> {
        let wxs: Vec<f64> = self.pins.iter()
            .map(|p| store.get(p.comp_x) + p.offset_x)
            .collect();
        let wys: Vec<f64> = self.pins.iter()
            .map(|p| store.get(p.comp_y) + p.offset_y)
            .collect();

        let sm_max_x = softmax(&wxs, self.gamma);
        let sm_min_x = softmax_neg(&wxs, self.gamma);
        let sm_max_y = softmax(&wys, self.gamma);
        let sm_min_y = softmax_neg(&wys, self.gamma);

        let mut jac = Vec::new();
        for (i, pin) in self.pins.iter().enumerate() {
            // ∂HPWL_x/∂comp_x = softmax_max_x[i] - softmax_min_x[i]
            // (because ∂wx/∂comp_x = 1)
            let dx = self.weight * (sm_max_x[i] - sm_min_x[i]);
            if dx.abs() > 1e-15 {
                jac.push((0, pin.comp_x, dx));
            }

            // ∂HPWL_y/∂comp_y = softmax_max_y[i] - softmax_min_y[i]
            let dy = self.weight * (sm_max_y[i] - sm_min_y[i]);
            if dy.abs() > 1e-15 {
                jac.push((1, pin.comp_y, dy));
            }
        }
        jac
    }

    fn weight(&self) -> f64 { self.weight }
    fn is_soft(&self) -> bool { true }
}
```

**Key insight**: Because `∂wx/∂comp_x = 1` (constant), the HPWL Jacobian
w.r.t. component position is just the softmax weight difference. No chain
rule complexity.

Multiple pins on the same component contribute additively:
```
∂HPWL/∂cx_k = Σ_{j ∈ pads_on_k ∩ net} (softmax_max_x[j] - softmax_min_x[j])
```

This has a beautiful physical interpretation: the **net pull force** on component
k is the sum of the pull forces on its individual pins. Pins near the max edge
of the net's bounding box pull the component toward the center; pins near the
min edge pull from the other side.


### Continuous Rotation (θ is a variable)

With θ as a parameter, the chain rule adds terms for `∂wx/∂θ`:

```rust
pub struct SmoothHPWLWithRotation {
    id: ConstraintId,
    pins: Vec<PinInfoWithRotation>,
    gamma: f64,
    weight: f64,
    entity_ids: Vec<EntityId>,
    param_ids: Vec<ParamId>,
}

struct PinInfoWithRotation {
    comp_x: ParamId,
    comp_y: ParamId,
    comp_theta: ParamId,
    local_x: f64,   // pad position in local frame
    local_y: f64,
}

impl Constraint for SmoothHPWLWithRotation {
    fn id(&self) -> ConstraintId { self.id }
    fn name(&self) -> &str { "SmoothHPWLWithRotation" }
    fn entity_ids(&self) -> &[EntityId] { &self.entity_ids }
    fn param_ids(&self) -> &[ParamId] { &self.param_ids }
    fn equation_count(&self) -> usize { 2 }

    fn residuals(&self, store: &ParamStore) -> Vec<f64> {
        let (wxs, wys) = self.compute_world_positions(store);
        let x_span = lse_max(&wxs, self.gamma) - lse_min(&wxs, self.gamma);
        let y_span = lse_max(&wys, self.gamma) - lse_min(&wys, self.gamma);
        vec![self.weight * x_span, self.weight * y_span]
    }

    fn jacobian(&self, store: &ParamStore) -> Vec<(usize, ParamId, f64)> {
        let (wxs, wys) = self.compute_world_positions(store);

        let sm_max_x = softmax(&wxs, self.gamma);
        let sm_min_x = softmax_neg(&wxs, self.gamma);
        let sm_max_y = softmax(&wys, self.gamma);
        let sm_min_y = softmax_neg(&wys, self.gamma);

        let mut jac = Vec::new();

        for (i, pin) in self.pins.iter().enumerate() {
            let theta = store.get(pin.comp_theta);
            let (sin_t, cos_t) = theta.sin_cos();

            // ── X-span residual (row 0) ──

            // ∂HPWL_x/∂wx_i · ∂wx_i/∂comp_x = (sm_max - sm_min) · 1
            let dHx_dwx = sm_max_x[i] - sm_min_x[i];
            jac.push((0, pin.comp_x, self.weight * dHx_dwx));

            // ∂HPWL_x/∂wx_i · ∂wx_i/∂θ
            // where ∂wx/∂θ = -lx·sin(θ) - ly·cos(θ)
            let dwx_dtheta = -pin.local_x * sin_t - pin.local_y * cos_t;
            let dHx_dtheta = dHx_dwx * dwx_dtheta;
            if dHx_dtheta.abs() > 1e-15 {
                jac.push((0, pin.comp_theta, self.weight * dHx_dtheta));
            }

            // ── Y-span residual (row 1) ──

            let dHy_dwy = sm_max_y[i] - sm_min_y[i];
            jac.push((1, pin.comp_y, self.weight * dHy_dwy));

            // ∂wy/∂θ = lx·cos(θ) - ly·sin(θ)
            let dwy_dtheta = pin.local_x * cos_t - pin.local_y * sin_t;
            let dHy_dtheta = dHy_dwy * dwy_dtheta;
            if dHy_dtheta.abs() > 1e-15 {
                jac.push((1, pin.comp_theta, self.weight * dHy_dtheta));
            }
        }
        jac
    }
}

impl SmoothHPWLWithRotation {
    fn compute_world_positions(&self, store: &ParamStore) -> (Vec<f64>, Vec<f64>) {
        let mut wxs = Vec::with_capacity(self.pins.len());
        let mut wys = Vec::with_capacity(self.pins.len());
        for pin in &self.pins {
            let cx = store.get(pin.comp_x);
            let cy = store.get(pin.comp_y);
            let theta = store.get(pin.comp_theta);
            let (sin_t, cos_t) = theta.sin_cos();
            wxs.push(cx + pin.local_x * cos_t - pin.local_y * sin_t);
            wys.push(cy + pin.local_x * sin_t + pin.local_y * cos_t);
        }
        (wxs, wys)
    }
}
```


## 6. Aggregate HPWL: Total Wire Length Objective

Create one `SmoothHPWL` constraint per net. The solver minimizes:
```
total_residual² = Σ_constraints r_i² = Σ_nets (weight · HPWL_x(net))²
                                      + Σ_nets (weight · HPWL_y(net))²
                                      + Σ hard_constraints r_j²
```

**Weighting**: The HPWL weight controls the trade-off between feasibility
(hard constraints = 0) and optimality (low wire length):

| Weight | Behavior |
|--------|----------|
| 0.0 | No wire length optimization (just satisfy constraints) |
| 0.001 | Gentle optimization (won't fight hard constraints) |
| 0.01 | Moderate optimization (default) |
| 0.1 | Aggressive optimization (may slow convergence) |

The solver's LM algorithm naturally handles this: when hard constraint
residuals dominate, LM steps toward feasibility. When hard constraints are
nearly satisfied, LM steps toward HPWL reduction.


## 7. Component Symmetry and Rotation Equivalence

Not all 4 rotations are distinct for every component:

| Component Type | Distinct Rotations | Why |
|---------------|-------------------|-----|
| 2-pin passive (R, C) | {0°, 90°} | Pads are interchangeable → 0°≡180°, 90°≡270° |
| LED (2-pin, polarized) | {0°, 90°, 180°, 270°} | Anode/cathode distinguish 0° from 180° |
| SOT-23 (3-pin) | {0°, 90°, 180°, 270°} | All 4 are distinct |
| QFP IC | {0°, 90°, 180°, 270°} | Pin 1 position distinguishes all 4 |
| BGA IC | {0°, 90°, 180°, 270°} | Pin A1 position distinguishes all 4 |
| Barrel jack | {0°} or {0°, 180°} | Depends on layout |

For symmetric components, the solver may oscillate between equivalent rotations.
The sin(2θ)=0 constraint doesn't distinguish them. Solutions:
1. Restrict allowed rotations in the spec (agent says `rotation: 0 | 90`)
2. Break symmetry by initializing θ deterministically
3. For passives, don't model rotation at all (just use 0° and let the
   router handle pad-to-trace angles)


## 8. Clearance Under Rotation

When θ is a solver variable, the bounding box changes with rotation.
The elliptical exclusion zone from `constraint-types.md` needs rotation-aware
half-sizes:

```rust
pub struct ComponentClearanceWithRotation {
    id: ConstraintId,
    entities: [EntityId; 2],
    x1: ParamId, y1: ParamId, theta1: ParamId,
    x2: ParamId, y2: ParamId, theta2: ParamId,
    hw1: f64, hh1: f64,  // local half-sizes of component 1
    hw2: f64, hh2: f64,  // local half-sizes of component 2
    gap: f64,
    params: [ParamId; 6],
}

impl Constraint for ComponentClearanceWithRotation {
    fn equation_count(&self) -> usize { 1 }

    fn residuals(&self, store: &ParamStore) -> Vec<f64> {
        let theta1 = store.get(self.theta1);
        let theta2 = store.get(self.theta2);

        // World-frame AABB half-sizes
        let whw1 = self.hw1 * theta1.cos().abs() + self.hh1 * theta1.sin().abs();
        let whh1 = self.hw1 * theta1.sin().abs() + self.hh1 * theta1.cos().abs();
        let whw2 = self.hw2 * theta2.cos().abs() + self.hh2 * theta2.sin().abs();
        let whh2 = self.hw2 * theta2.sin().abs() + self.hh2 * theta2.cos().abs();

        let combined_hw = whw1 + whw2 + self.gap;
        let combined_hh = whh1 + whh2 + self.gap;

        let dx = store.get(self.x2) - store.get(self.x1);
        let dy = store.get(self.y2) - store.get(self.y1);

        let nx = dx / combined_hw;
        let ny = dy / combined_hh;

        // Elliptical exclusion: must be ≥ 0
        vec![nx * nx + ny * ny - 1.0]
    }

    // Jacobian: chain rule through world_hw(θ) and world_hh(θ)
    // More complex — 6 partial derivatives
    // Omitted for brevity; computed analytically from the expressions above
}
```

**In practice**: For Phase 2 (fixed rotation), the rotation-aware version
reduces to the simple version from constraint-types.md since whw/whh become
constants.


## 9. Recommended Implementation Order

### Step 1: Fixed-rotation placement (MVP)
- Entity: `PcbComponent` with params `[x, y]` only
- Rotation specified by user in spec, fixed during solve
- HPWL: `SmoothHPWL` with precomputed pad offsets
- Clearance: `ComponentClearance` with precomputed AABB half-sizes
- Greedy rotation search: try each component at each allowed rotation

### Step 2: Continuous rotation (Phase 1 of two-phase)
- Entity: `PcbComponentContinuous` with params `[x, y, θ]`
- Add `RotationDiscretize` constraint (sin(2θ)=0)
- HPWL: `SmoothHPWLWithRotation` with rotation-dependent Jacobian
- Clearance: `ComponentClearanceWithRotation`
- Two-phase solve: continuous → snap → refine

### Step 3: Adaptive γ
- Start with γ=2, solve to rough convergence
- Increase to γ=10, re-solve from previous solution
- Optionally γ=20 for final polish

### Step 4: Multi-start
- Run placement from multiple random initial positions
- Keep the solution with lowest total HPWL
- Parallelize using solverang's `ParallelSolver` (requires `parallel` feature,
  uses `rayon` internally). `ParallelSolver` decomposes problems into independent
  clusters; for multi-start, spawn separate `ConstraintSystem` instances per
  initial condition and solve in parallel via `rayon::par_iter()`.


## 10. Pin Position Aggregation

When multiple pins from the same component are in the same net, their HPWL
contributions are coupled through the component position. The Jacobian handles
this correctly because each pin's Jacobian row references the component's
ParamId.

**Example**: Component U1 has pins 1, 7, 14 on net VCC. The HPWL constraint
for net VCC has 3 pin entries, all referencing `u1_x` and `u1_y`. The Jacobian
will have 3 entries for `(row_0, u1_x, ...)` which solverang sums automatically
when building the Jacobian matrix.

This means: if U1 is the "widest" component on net VCC (its pins span the most
in X), the net force on U1 will be near-zero (max-side pins and min-side pins
cancel out). The net force on smaller components will be stronger, pulling them
toward U1. This is physically correct — big central ICs tend to anchor the
placement while small passives gravitate toward them.
