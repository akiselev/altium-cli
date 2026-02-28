# Placement Algorithm Survey & Integration Strategy

Comprehensive survey of placement algorithms, how they compare to our
solverang-based approach, and a multi-stage pipeline design that combines
the best of each.


## 1. Algorithm Taxonomy

```
┌─────────────────────────────────────────────────────────────────┐
│                    PLACEMENT ALGORITHMS                         │
├──────────────────┬──────────────────┬───────────────────────────┤
│   CONSTRUCTIVE   │   ITERATIVE      │   LEARNING-BASED          │
│   (build from    │   (refine from   │   (learn from data)       │
│    scratch)      │    initial)      │                           │
├──────────────────┼──────────────────┼───────────────────────────┤
│ • Partitioning   │ • Simulated      │ • RL (Google, 2021)       │
│   (min-cut)      │   Annealing      │ • Diffusion models (2024) │
│ • Quadratic      │ • Force-directed │ • GNN-based               │
│ • Analytical     │ • Local search   │ • Genetic/Evolutionary    │
│   (ePlace, etc.) │   (swap, slide)  │                           │
│ • Constraint     │                  │                           │
│   solving ←US    │                  │                           │
└──────────────────┴──────────────────┴───────────────────────────┘
```

## 2. Simulated Annealing (SA)

**The classic**: Used by TimberWolf (1985), VPR (1997), and Altium's own
placer. Still the gold standard for detailed placement.

### Core Algorithm

```
T = T_initial
placement = random_initial()
best = placement

while T > T_frozen:
    for i in 0..moves_per_temp:
        move = generate_random_move(placement)    // swap, displace, rotate
        ΔC = cost(apply(placement, move)) - cost(placement)
        if ΔC < 0 or random() < exp(-ΔC / T):   // Metropolis criterion
            accept(placement, move)
            if cost(placement) < cost(best):
                best = placement
    T = α * T    // cooling: α ∈ [0.85, 0.99]

return best
```

### Move Types for PCB

| Move | Description | When Useful |
|------|-------------|-------------|
| **Displacement** | Move component to random position | High temperature (exploration) |
| **Swap** | Exchange two component positions | Medium temperature |
| **Rotation** | Rotate component 90°/180°/270° | All temperatures |
| **Slide** | Move component along one axis | Low temperature (fine-tuning) |
| **Mirror** | Flip component to other board side | Rare, for dual-sided boards |

### Cost Function

```
Cost = w₁ · HPWL                    // total wire length
     + w₂ · Σ overlap_penalty       // overlap between components
     + w₃ · Σ constraint_violation   // hard constraint penalties
     + w₄ · congestion_estimate      // routing congestion
     + w₅ · net_crossing_count       // PCB-specific: signal integrity
```

### Strengths & Weaknesses

| Strengths | Weaknesses |
|-----------|------------|
| Handles discrete variables naturally (rotation, side) | Slow for large N (O(N²) cost eval per move) |
| Provably converges to global optimum (given infinite time) | No gradient information — blind search |
| Simple to implement | Hard to tune (T_initial, α, moves_per_temp) |
| Handles arbitrary cost functions | Constraint satisfaction via penalty = unreliable |
| Excellent for PCB-scale problems (N < 500) | Doesn't scale to VLSI (N > 100K) |

### Fits Into Our Pipeline As: **Phase 3 (Detailed Placement)**

SA excels at local refinement after analytical placement gives a good initial
solution. It naturally handles discrete rotation, component swaps, and board-side
assignment. Using solverang's solution as the initial placement instead of random
dramatically reduces SA runtime (starts near optimum, needs less exploration).


## 3. Analytical Placement (ePlace/DREAMPlace/RePlace)

**State of the art for VLSI**: Formulates placement as continuous optimization.
This is closest to what we're doing with solverang.

### Formulation

```
minimize    W(x, y)                  // smooth wirelength (LSE or WA)
          + λ · D(x, y)             // density penalty
subject to  x, y ∈ placement_region
```

**Wirelength** `W`: Log-sum-exp smooth HPWL (what we use) or weighted-average (WA).

**Density penalty** `D`: Prevents all components from clustering in one spot.
ePlace uses an electrostatic analogy — each component is a positive charge, and
the density cost is the electric potential energy. Solved via Poisson's equation
using FFT:

```
∇²ψ(x,y) = ρ(x,y) - ρ_target     // Poisson's equation
D = ∫∫ ρ(x,y) · ψ(x,y) dx dy     // electrostatic energy
∇D = ρ(x,y) · E(x,y)             // force = charge × electric field
```

**Optimizer**: Nesterov's accelerated gradient descent (not LM). ePlace showed
this converges faster than conjugate gradient for placement.

### Key Difference from Solverang

| Property | ePlace/DREAMPlace | Solverang |
|----------|-------------------|-----------|
| Formulation | Unconstrained optimization with penalty | Constrained least-squares |
| Constraints | Penalty functions (soft) | Equality/inequality (hard) |
| Density | Electrostatic FFT | Not needed at PCB scale |
| Optimizer | Nesterov gradient descent | Levenberg-Marquardt |
| Variables | Millions (VLSI) | Tens to hundreds (PCB) |
| GPU | Essential (DREAMPlace) | Not needed at PCB scale |

### Fits Into Our Pipeline As: **Inspiration for Phase 1**

We're already doing analytical placement with solverang. The key insight from
ePlace/DREAMPlace we should adopt:
1. **Adaptive γ**: Start with small γ (smooth LSE), increase during solve
2. **Nesterov momentum**: Could be added to solverang as an alternative solver
3. **Density penalty**: Probably NOT needed for PCB (too few components), but
   available if components cluster pathologically


## 4. Force-Directed Placement (Kraftwerk2)

**Quadratic wirelength minimization with spreading forces**.

### Formulation

```
minimize    x^T · Q · x              // quadratic wirelength (HPWL approximation)
subject to  spreading constraints      // prevent overlap
```

The connectivity matrix `Q` encodes net connections. Solving `Qx = b` gives
the minimum-wirelength positions, but all components pile on top of each other.

**Spreading**: Add "forces" that push overlapping components apart:
- **Hold force**: Pulls component toward its current position (anchor)
- **Move force**: Pushes component away from dense regions

Each iteration: solve quadratic system → compute density → add forces → repeat.

### Key Ideas

The **Bound2Bound net model** models each multi-pin net as a set of 2-pin
connections between the leftmost/rightmost (topmost/bottommost) pins. This
gives an exact quadratic approximation to HPWL for the current pin ordering.

### Relevance to Solverang

Kraftwerk2's force model maps beautifully to solverang constraints:
- **Net springs**: Our HPWL residuals are essentially spring forces
- **Hold forces**: Our `FixedPosition` constraints
- **Move forces**: Could be modeled as `ComponentClearance` constraints

The main difference: Kraftwerk2 uses a quadratic system (linear solve) while
solverang uses nonlinear least-squares (LM). For PCB-scale problems, the
difference is negligible. For VLSI-scale, the linear solve is much faster.


## 5. Partitioning-Based (Min-Cut)

**Recursive bisection**: Divide the board into regions, assign components to
regions, recurse.

```
function partition(components, region):
    if |components| ≤ threshold:
        place_within_region(components, region)
        return

    (left, right) = min_cut_partition(components, netlist)
    (region_L, region_R) = split(region)
    partition(left, region_L)
    partition(right, region_R)
```

**Min-cut**: Minimize the number of nets crossing the partition boundary
(= minimize wire length between regions). Uses hMETIS or similar graph
partitioning algorithms.

### Relevance

For PCB, partitioning maps naturally to our **group/separate** constraints:
```
group analog { components: [U5, R10, C20] }
group digital { components: [U1, U2, U3] }
separate $analog, $digital { gap: 10mm }
```

The LLM agent's high-level grouping specification IS a manual partition.
We could auto-generate groups from netlist analysis (heavily connected
components should be grouped).

### Fits Into Our Pipeline As: **Pre-processing (Phase 0)**

Auto-detect component clusters from netlist connectivity and generate
initial group assignments. Feed these as `NearConstraint` groups to solverang.


## 6. Google's RL Chip Placer (Nature 2021)

**Macro placement via reinforcement learning**: Train a policy network to
place components sequentially.

### Architecture

```
State:   (netlist_graph, current_partial_placement, metadata)
Action:  (grid_cell_x, grid_cell_y) for next component
Reward:  -w₁·HPWL - w₂·congestion   (after all components placed)
Network: Edge-based GNN → embeddings → policy + value heads
```

### Key Insights

1. **Sequential placement**: Place components one at a time, largest first
2. **Transfer learning**: Pre-train on many chip designs, fine-tune on new chip
3. **Standard cells**: After RL places macros, a force-directed method places
   standard cells (same as our analytical approach)
4. **Learned heuristics**: The network learns that "macros go near edges with
   convex space in center" — exactly what a human designer does

### Relevance to Our System

Google's approach validates our LLM-agent-driven specification:
- The RL agent learns the SAME heuristics humans use
- Our approach lets LLM agents specify these heuristics DIRECTLY
- "U1 in center, connectors on edges" IS the learned policy, stated declaratively

**We don't need RL because the LLM IS the policy**. The LLM encodes design
knowledge from its training data. The solver just makes it precise.

### Could Integrate As: **Alternative to LLM specification**

Instead of an LLM writing a placement spec, train an RL policy to generate
constraint sets. But this is more complex and less interpretable than our
approach.


## 7. Diffusion Models for Placement (2024)

**Latest research** (UC Berkeley, arxiv 2407.12282): Use a denoising diffusion
model to generate placements in a single forward pass.

### Architecture

```
Input:  Netlist graph (nodes = components, edges = nets)
Model:  Interleaved graph convolutions + multi-headed attention
Train:  On (netlist, optimal_placement) pairs
Infer:  Start from noise → denoise → placement
Guide:  During sampling, add gradient from HPWL/congestion cost
```

### Key Innovation: Synthetic Data Generation

The inverse problem — "given a placement, generate a plausible netlist" —
is easy. This enables unlimited training data without needing real designs
or commercial tools.

### Strengths

- **Zero-shot**: Places new circuits without fine-tuning
- **Fast inference**: Single forward pass ≈ milliseconds
- **Guided sampling**: Can optimize any differentiable cost during generation

### Relevance

Interesting research direction, but not practical for us now because:
1. Requires training infrastructure (GPU, large datasets)
2. PCB netlists are small enough that exact solving is fast
3. Our constraint-based approach gives guarantees; diffusion doesn't

**Could integrate as**: Initial placement generator (instead of random init)
that gives solverang a good starting point. But random + solverang is likely
fast enough for PCB-scale problems.


## 8. Genetic/Evolutionary Algorithms

**Encoding**: Represent placement as a chromosome (component order + rotations).
**Crossover**: Combine regions from two parent placements.
**Mutation**: Random move/swap/rotate.
**Selection**: Tournament/roulette based on fitness (cost function).

### PCB-Specific Variants

- **Self-Organizing Genetic Algorithm (SOGA)**: Groups related components
  during evolution, maintaining functional clusters
- **Island model**: Parallel populations with occasional migration
- **Hybrid GA+SA**: GA for global search, SA for local refinement

### Relevance

GA is dominated by SA for detailed placement and by analytical methods for
global placement. Not recommended as primary algorithm. Could be useful for:
- Exploring the discrete rotation assignment space
- Multi-objective optimization (Pareto front of HPWL vs. area vs. thermal)


## 9. Cypress: VLSI-Inspired PCB Placement (ISPD 2025 Best Paper)

The most directly relevant work. **GPU-accelerated PCB placement built on
DREAMPlace** with PCB-specific adaptations.

### PCB-Specific Adaptations

1. **Board outline constraints**: Non-rectangular placement regions
2. **Component clearance**: Much larger clearance rules than VLSI
3. **Mixed rotation**: 0°/90°/180°/270° with different component symmetries
4. **Net crossing metric**: PCB routability proxy (limited routing layers)
5. **Hyperparameter tuning**: Automated search for cost function weights

### Results

- 1–5.9× higher routability than prior PCB placement methods
- 1–19.7× shorter routed track lengths on fully routed designs
- Open-source benchmark with 10 synthesized PCB designs

### Relevance

Cypress validates analytical placement for PCB. Their approach is very similar
to ours (analytical + gradient + density) but GPU-accelerated. For our
PCB-scale problems (50-500 components), we don't need GPU — solverang on CPU
is fast enough. But their cost functions and PCB adaptations are directly
applicable.


## 10. The Critical Insight: Density Control

Every analytical placer must solve the **overlap problem**: minimizing wire
length alone causes all components to pile on top of each other (the optimal
HPWL position for a fully connected graph is all-at-one-point).

### How Others Solve It

| Approach | Algorithm | Mechanism |
|----------|-----------|-----------|
| **Electrostatic** | ePlace, DREAMPlace | Components = charges, FFT Poisson solve |
| **Bell-shaped** | NTUPlace, SimPL | Gaussian density smoothing per bin |
| **Spreading forces** | Kraftwerk2 | Explicit push forces from dense regions |
| **Penalty function** | SA-based | Add overlap area to cost |
| **Hard constraints** | **Solverang (us)** | Pairwise clearance constraints |

### Our Approach: Pairwise Clearance (Is It Enough?)

We use `ComponentClearance` constraints (elliptical exclusion zones). For N
components, this generates O(N²) constraints. At PCB scale:

| N | Pairs | Constraints | Acceptable? |
|---|-------|-------------|-------------|
| 20 | 190 | 190 | Yes |
| 50 | 1,225 | 1,225 | Yes |
| 100 | 4,950 | 4,950 | Yes |
| 200 | 19,900 | 19,900 | Marginal |
| 500 | 124,750 | 124,750 | Too many |

**For N > 200**: Use spatial indexing (R-tree or grid) to prune distant pairs.
Components more than `max_component_size + clearance` apart can never violate
clearance. In practice, this reduces to O(N · k) where k ≈ 10-20 neighbors.

**Alternative for large N**: Add a lightweight density penalty term (like
ePlace's electrostatic model) instead of pairwise constraints. This converts
O(N²) constraints into O(1) FFT-based evaluation. But for PCB-scale problems,
pairwise is fine.


## 11. PCB vs. VLSI: Why Our Approach Works

PCB placement is fundamentally different from VLSI placement:

| Property | VLSI | PCB |
|----------|------|-----|
| Components | 100K–10M standard cells + macros | 50–500 components |
| Component sizes | Mostly uniform (std cells) | Highly varied (0402 to QFP-144) |
| Layers | 5-15 metal layers | 2-8 copper layers |
| Routing resources | Abundant (many layers) | Scarce (few layers) |
| Board shape | Rectangular die | Irregular outlines, cutouts |
| Mechanical | None | Mounting holes, connectors, height limits |
| Constraints | Timing, power | Design rules, thermal, EMC |
| Cost of iteration | Minutes (digital flow) | Hours–days (fabrication) |

**Implications for algorithm choice**:

1. **N is small enough for exact constraint solving** → Solverang's O(N²) pairwise
   clearance is fine. No need for FFT-based density.

2. **Components have diverse sizes** → Bounding box clearance (not bin-based density)
   is more natural.

3. **Mechanical constraints dominate** → Connectors MUST be on specific edges. Mounting
   holes are fixed obstacles. These are naturally expressed as solverang constraints.

4. **Discrete decisions matter more** → Board side (top/bottom), rotation, layer
   assignment. SA handles these naturally.

5. **Routability is critical** → With few layers, bad placement means unroutable design.
   Net crossing minimization (Cypress-style) is essential.


## 12. Recommended Multi-Stage Pipeline

```
╔══════════════════════════════════════════════════════════════════╗
║  PHASE 0: Pre-Processing                                        ║
║  Input: PcbDoc + placement spec                                  ║
║  • Extract netlist, footprint BBs, board outline                ║
║  • Auto-detect component clusters from connectivity             ║
║  • Generate initial grouping constraints                         ║
║  Method: Graph partitioning (hMETIS or spectral clustering)     ║
║  Output: Component groups + initial constraint set               ║
╠══════════════════════════════════════════════════════════════════╣
║  PHASE 1: Global Placement (Solverang)                          ║
║  Input: Components + constraints + netlist                       ║
║  • Continuous (x, y, θ) optimization                            ║
║  • Hard constraints: board containment, clearance, edges, groups║
║  • Soft objective: smooth HPWL (LSE, adaptive γ)                ║
║  • sin(2θ) = 0 for rotation discretization                     ║
║  Method: Levenberg-Marquardt (solverang AutoSolver)             ║
║  Output: Approximate placement (continuous)                      ║
╠══════════════════════════════════════════════════════════════════╣
║  PHASE 2: Legalization                                           ║
║  Input: Continuous placement from Phase 1                        ║
║  • Snap rotations to nearest allowed value                      ║
║  • Resolve any remaining overlaps (greedy shifting)             ║
║  • Verify all hard constraints satisfied                         ║
║  Method: Greedy + Solverang re-solve with fixed θ               ║
║  Output: Legal placement (integer rotations, no overlaps)       ║
╠══════════════════════════════════════════════════════════════════╣
║  PHASE 3: Detailed Placement (Simulated Annealing)              ║
║  Input: Legal placement from Phase 2                             ║
║  • Swap, displace, rotate moves                                 ║
║  • Cost = w₁·HPWL + w₂·net_crossings + w₃·constraint_penalty  ║
║  • Adaptive cooling schedule                                     ║
║  Method: SA with Metropolis acceptance                           ║
║  Output: Optimized placement                                     ║
╠══════════════════════════════════════════════════════════════════╣
║  PHASE 4: Final Refinement (Solverang)                          ║
║  Input: SA result                                                ║
║  • Fix rotations (from SA)                                       ║
║  • Continuous (x, y) optimization with tight γ                  ║
║  • Fine-tune positions within clearance envelopes               ║
║  Method: Solverang with fixed θ, high γ                         ║
║  Output: Final placement                                         ║
╠══════════════════════════════════════════════════════════════════╣
║  PHASE 5: DRC Verification                                       ║
║  Input: Final placement                                          ║
║  • Evaluate all design rules as constraint residuals            ║
║  • Report violations with exact distances                        ║
║  Method: Solverang single-evaluation (no solve, just residuals) ║
║  Output: DRC report                                              ║
╚══════════════════════════════════════════════════════════════════╝
```

### Why This Pipeline

- **Phase 1 (Solverang)** gives constraint-satisfying initial placement
  (something SA alone can't guarantee)
- **Phase 3 (SA)** explores discrete moves and escapes local minima
  (something gradient-based solvers can't do)
- **Phase 4 (Solverang)** fine-tunes continuous positions after SA's discrete
  decisions are locked in
- **Each phase starts from a good solution**, so convergence is fast


## 13. Simulated Annealing: Detailed Design for Phase 3

### SA Parameters for PCB

```rust
pub struct SAConfig {
    // Temperature
    t_initial: f64,         // auto-set: ~20× average cost change
    t_frozen: f64,          // stop when acceptance rate < 1%
    alpha: f64,             // cooling rate: 0.95 (PCB-scale)

    // Moves
    moves_per_temp: usize,  // 10 × N (components)
    displacement_range: f64, // starts at board_diagonal, shrinks with T

    // Move probabilities (adapt with temperature)
    p_displace: f64,        // 0.4 (high T) → 0.2 (low T)
    p_swap: f64,            // 0.3 (all T)
    p_rotate: f64,          // 0.2 (all T)
    p_slide: f64,           // 0.1 (high T) → 0.3 (low T)

    // Cost weights
    w_hpwl: f64,            // 1.0
    w_overlap: f64,         // 10.0 (heavy penalty)
    w_constraint: f64,      // 100.0 (must satisfy)
    w_net_crossing: f64,    // 0.5 (routability)
}
```

### Move Generation

```rust
fn generate_move(placement: &Placement, config: &SAConfig, rng: &mut Rng) -> Move {
    let p = rng.gen::<f64>();
    if p < config.p_displace {
        // Random displacement within shrinking window
        let comp = rng.choose(&placement.components);
        let range = config.displacement_range * (config.t_current / config.t_initial);
        let dx = rng.gen_range(-range..range);
        let dy = rng.gen_range(-range..range);
        Move::Displace(comp, dx, dy)
    } else if p < config.p_displace + config.p_swap {
        // Swap two components (same-size preferred for better acceptance)
        let (a, b) = rng.choose_pair(&placement.components);
        Move::Swap(a, b)
    } else if p < config.p_displace + config.p_swap + config.p_rotate {
        // Rotate by 90° (check allowed rotations)
        let comp = rng.choose(&placement.components);
        let delta = *rng.choose(&[90, 180, 270]);
        Move::Rotate(comp, delta)
    } else {
        // Slide along one axis (fine-tuning)
        let comp = rng.choose(&placement.components);
        let axis = rng.choose(&[Axis::X, Axis::Y]);
        let delta = rng.gen_range(-1.0..1.0) * config.displacement_range * 0.1;
        Move::Slide(comp, axis, delta)
    }
}
```

### Incremental Cost Evaluation

SA makes O(N × T_steps) moves. Full cost evaluation per move is too expensive.
Use **incremental** evaluation:

- **HPWL**: Only recompute nets connected to moved component(s). O(k) where
  k = average pins per component.
- **Overlap**: Only check moved component against nearby components. O(k)
  with spatial index.
- **Constraint violation**: Only check constraints involving moved component.

This makes each move evaluation O(k) instead of O(N²).

### Constraint Handling in SA

SA uses **penalty functions** for constraints, which don't guarantee satisfaction.
But because Phase 1 (solverang) gives a feasible initial placement, SA starts
from a legal state. We add:

1. **Hard rejection**: Moves that place components outside the board are
   rejected immediately (no Metropolis).
2. **Heavy penalty**: Overlap and constraint violations have weight >> HPWL,
   so SA strongly prefers feasible solutions.
3. **Feasibility check**: After SA converges, verify all constraints. If any
   violated, re-run Phase 2 (solverang re-solve).


## 14. Net Crossing: PCB Routability Metric

HPWL alone doesn't capture routability. On a 2-layer PCB, crossing nets require
vias, which are expensive. **Net crossing count** is a better routability proxy.

### Definition

For each pair of 2-pin net segments (A₁→A₂) and (B₁→B₂), count whether
the segments would cross if routed as straight lines:

```
crossing(A, B) = segments_intersect(A₁, A₂, B₁, B₂) ? 1 : 0
```

Total crossings: `Σ_{all net segment pairs} crossing(A, B)`

For multi-pin nets, decompose into minimum spanning tree edges (or star topology).

### Integration

Net crossing is expensive to compute (O(E²) where E = total net segments).
Use spatial index or sweep-line for O(E · log E).

In SA cost function: `w_net_crossing × total_crossings`.
Not used in solverang (not differentiable), but SA handles it naturally.


## 15. What We Should NOT Implement

| Algorithm | Why Skip |
|-----------|----------|
| **FFT density** (ePlace) | Only needed for N > 10K. PCB has N < 500. |
| **GPU acceleration** (DREAMPlace/Cypress) | Not needed at PCB scale. CPU solverang is <100ms. |
| **RL training** (Google) | Massive infrastructure cost, LLM already encodes design knowledge. |
| **Diffusion models** | Training infra, no constraint guarantees, PCB too small to need it. |
| **Genetic algorithms** | Dominated by SA for discrete + analytical for continuous. |
| **Quadratic solver** (Kraftwerk2) | Solverang's LM handles nonlinear objectives better than linearized HPWL. |

## 16. Implementation Priority

### Milestone 1: Solverang-Only Placer (MVP)
- Phases 1 + 2 only (analytical + legalization)
- Fixed rotation from user spec
- Pairwise clearance constraints
- Smooth HPWL with adaptive γ
- **Estimated effort**: 2-3 weeks

### Milestone 2: Add Simulated Annealing
- Phase 3 (SA detailed placement)
- Move generation: displace, swap, rotate, slide
- Incremental cost evaluation
- Adaptive cooling schedule
- **Estimated effort**: 1-2 weeks

### Milestone 3: Full Pipeline
- Phase 0 (netlist clustering → auto-groups)
- Phase 4 (final refinement)
- Phase 5 (DRC verification)
- Net crossing metric
- **Estimated effort**: 2-3 weeks

### Milestone 4: Advanced Features
- Multi-start with parallel solves
- Thermal-aware placement
- 3D clearance (component height)
- Automatic rotation exploration (Phase 1 continuous θ)
- **Estimated effort**: 2-4 weeks
