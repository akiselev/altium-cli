# Manufacturing-Aware Cost Functions and Multi-Objective Optimization in PCB Autorouters

Research report for the autopcb-router PathFinder implementation.

**Date**: 2026-03-23
**Context**: Grid-based PathFinder (McMurchie-Ebeling '95) with cost function:
```
C(n) = base * dir_penalty * corridor_penalty
       + hist_weight * history[n]
       + pres_fac * max(0, usage[n] - 1)
```

---

## Table of Contents

1. [Multi-Objective Cost Functions in PCB/VLSI Routing](#1-multi-objective-cost-functions)
2. [Via Minimization Strategies](#2-via-minimization-strategies)
3. [Congestion-Driven Routing with DFM Awareness](#3-congestion-driven-routing-with-dfm)
4. [Post-Route DFM Optimization Passes](#4-post-route-dfm-optimization)
5. [Machine Learning Approaches](#5-machine-learning-approaches)
6. [Open-Source Router DFM Analysis](#6-open-source-router-dfm)
7. [Redundant Via Insertion](#7-redundant-via-insertion)
8. [Recommended Augmentations for Our Router](#8-recommendations)

---

## 1. Multi-Objective Cost Functions

### 1.1 The VPR/PathFinder Canonical Cost Function

The VPR (Verilog-to-Routing) project maintains the most mature open-source PathFinder
implementation. Its cost function per routing resource node `n` for connection `i->j` is:

```
C(n, i, j) = alpha_ij * d(n) + (1 - alpha_ij) * (b(n) + h(n,t)) * p(n)
```

Where:
- `alpha_ij` = criticality of connection (timing-driven term, 0.0 for pure congestion)
- `d(n)` = delay through node n (timing-driven component)
- `b(n)` = base cost of node n (intrinsic delay, demand-normalized, or demand-only)
- `h(n,t)` = historical congestion cost accumulated over iterations 0..t
- `p(n)` = present congestion penalty = `1 + max(0, (1 + occupancy(n) - capacity(n))) * pres_fac`

**Key VPR parameters and their defaults:**
| Parameter | Default | Description |
|-----------|---------|-------------|
| `first_iter_pres_fac` | 0.0 | No congestion in first iteration (shortest-path discovery) |
| `initial_pres_fac` | 0.5 | Starting present penalty for subsequent iterations |
| `pres_fac_mult` | 1.3 | Exponential growth per iteration |
| `max_pres_fac` | 1000.0 | Cap to prevent overflow |
| `acc_fac` | 1.0 | Historical congestion increment (unchanged across iterations) |
| `bend_cost` | 1.0 (global) / 0.0 (detailed) | Turn penalty |
| `bb_factor` | 3 | Bounding box expansion for search |
| `astar_fac` | 1.2 | A* lookahead aggressiveness |

**Our router comparison**: Our implementation closely follows VPR but lacks the timing-driven
alpha term. Our `dir_penalty` and `corridor_penalty` are multiplicative modifiers on base cost
rather than separate additive terms. Our `hist_weight` corresponds to what VPR handles
implicitly in `(b(n) + h(n,t))`.

References:
- McMurchie & Ebeling, "PathFinder: A Negotiation-Based Performance-Driven Router for FPGAs," FPGA 1995
- VPR documentation: https://docs.verilogtorouting.org/en/latest/vpr/command_line_usage/

### 1.2 ISPD Contest Scoring: The Industry Multi-Objective Formula

The ISPD 2018/2019 Initial Detailed Routing Contests defined the standard multi-objective
scoring formula used by the VLSI community:

```
Score = w_wl * (wirelength / M2_pitch) + w_via * via_count + w_short_area * short_area + w_short_count * short_count + ... (per DRC type)
```

**ISPD 2018 weights (verified from contest specification):**
| Component | Weight | Unit |
|-----------|--------|------|
| Wire length | 0.5 | Per Metal2 pitch |
| Via count | 2.0 | Per via |
| Short violations | 500 | Per violation AND per area unit |
| Other DRC violations | Varies by type | Per violation |

Key insight: **vias are weighted 4x more than wirelength** per equivalent unit, reflecting
their outsized impact on manufacturing yield and routing congestion. DRC violations carry
extreme penalties (500x), making the formula inherently lexicographic: DRC-free first, then
optimize quality.

The 2019 contest added more realistic design rules: spacing tables, cut spacing,
end-of-line spacing, and minimum area rules -- reflecting real DFM constraints.

References:
- ISPD 2018 Contest: https://www.ispd.cc/contests/18/
- ISPD 2019 Contest: https://www.ispd.cc/contests/19/

### 1.3 Negotiation-Based Track Assignment Cost Function

The ICCAD 2018 multithreaded detailed router uses a negotiation-based cost for track
assignment with an explicit multi-objective formulation:

```
C_t = beta_wl * C_wl + beta_ol * C_ol + beta_bk * C_bk + beta_his * C_his
```

Where:
- `C_wl` = wirelength cost
- `C_ol` = overlap cost (current congestion)
- `C_bk` = via/blockage cost
- `C_his` = historical congestion cost

This is a **weighted linear combination** (scalarization) of multiple objectives, which is
the standard approach in negotiated congestion frameworks because it integrates naturally
into the A* priority queue -- you cannot have a Pareto frontier inside a single-objective
shortest-path search.

Reference: "A Multithreaded Initial Detailed Routing Algorithm Considering Global Routing Guides," ICCAD 2018

### 1.4 How to Navigate the Pareto Frontier

In practice, PCB/VLSI routers do NOT explicitly compute Pareto frontiers. Instead they use:

1. **Weighted sum scalarization** (as above) -- fast, integrates into A*
2. **Lexicographic ordering** -- DRC violations > connectivity > congestion > wirelength > vias
3. **Iterative weight adjustment** -- increase congestion penalty over iterations (PathFinder's core mechanism)
4. **Constraint-based** -- hard constraints for DRC, soft penalties for optimization objectives

**Recommended approach for our router**: Use weighted sum with iteration-adaptive weights.
DFM terms should be additive penalty terms in the cost function, not multiplicative (to
avoid distorting the congestion negotiation mechanism).

### 1.5 Proposed Extended Cost Function

```
C(n) = base * dir_penalty * corridor_penalty
       + hist_weight * (history[n] + edge_history[n])
       + pres_fac * max(0, usage[n] - 1)
       + via_penalty(n)           // manufacturing-aware via cost
       + spacing_penalty(n)       // DFM spacing margin
       + acid_trap_penalty(n)     // acute angle avoidance
```

Where the new DFM terms are detailed in sections below.

---

## 2. Via Minimization Strategies

### 2.1 The Manufacturing Cost of Vias

Vias are the single largest manufacturing cost driver in PCB routing that the router can
control. Quantitative data from PCB fabricators:

| Factor | Impact |
|--------|--------|
| Drilling = 30-40% of total PCB manufacturing cost | Hole count dominates drilling time |
| Each drill size reduction below 0.3mm | +10-20% cost increase |
| Via density doubling (100 -> 200 holes/in^2) | +50% drilling cost |
| Blind vias vs. through-hole | +20-50% cost increase |
| Microvias (laser-drilled) | +$0.05-0.15 per via |
| Multiple drill size setups | +20% production time |
| Small drill (<6 mil) tool wear | Faster consumption, higher cost |
| High aspect ratio (>6:1) | Plating uniformity problems |
| Annular ring < 5 mil | 18.7% breakout probability at +/-25um drill wander |

**Key insight for cost function**: Via cost should NOT be a flat penalty. It should
reflect the actual manufacturing impact:

```rust
fn manufacturing_via_cost(via: &Via, board: &Board) -> f64 {
    let mut cost = BASE_VIA_COST;  // e.g., 10.0

    // Aspect ratio penalty (drilling difficulty)
    let aspect_ratio = board.thickness_mm / via.drill_diameter_mm;
    if aspect_ratio > 6.0 {
        cost *= 1.0 + (aspect_ratio - 6.0) * 0.3;  // 30% per unit above 6:1
    }

    // Small drill penalty (tool wear)
    if via.drill_diameter_mm < 0.3 {
        cost *= 1.0 + (0.3 - via.drill_diameter_mm) / 0.1 * 0.15;  // 15% per 0.1mm below 0.3
    }

    // Annular ring reliability penalty
    let annular_ring = (via.pad_diameter_mm - via.drill_diameter_mm) / 2.0;
    if annular_ring < 0.15 {  // 6 mil
        cost *= 1.5;  // significant reliability risk
    }

    // Via-in-pad penalty (requires fill+cap process)
    if via.is_in_pad {
        cost *= 2.0;  // doubles manufacturing complexity
    }

    cost
}
```

### 2.2 Layer Assignment for Via Minimization

The formal via minimization problem: given a routing solution with wire segments, assign
each segment to a metal layer to minimize the total number of layer transitions (vias).

**Two-layer case: Maximum Cut on planar graph (polynomial)**

For two layers, via minimization reduces to MAX-CUT on a planar conflict graph:
- Vertices = wire segments
- Edges = crossings between segments
- Partition into two sets (layers) maximizing cut edges = minimizing vias

This is solvable in polynomial time for planar graphs (unlike general MAX-CUT which is
NP-hard). The algorithm runs in O(V^(3/2) * log(V)) time.

**Multi-layer case: NP-hard, requires heuristics**

For k > 2 layers, the problem generalizes to k-coloring which is NP-hard. Practical
approaches:

1. **Greedy layer assignment**: Assign each segment to the layer with fewest crossings
   with already-assigned segments. O(n * k) per segment.

2. **Graph contractibility heuristic**: Build crossing graph, iteratively contract edges
   to merge segments onto same layer. Proven effective in practice.

3. **Maximum Independent Set (MIS) based**: Used by MLV-CBS (2025). Build conflict graph,
   find MIS to identify non-conflicting layer assignments.

**V-GR (ASP-DAC 2024)**: Novel via-aware 3D global router that integrates via minimization
into the routing cost function itself rather than treating it as a post-process. Key insight:
edges with higher wire density have higher probability of producing vias, so the routing
cost is modified to account for this correlation.

References:
- "Via Minimization in VLSI Chip Design - Application of a Planar Max-Cut Algorithm"
- V-GR: "3D Global Routing with Via Minimization and Multi-Strategy Rip-up and Rerouting," ASP-DAC 2024
- "Multi-agent based minimal-layer via routing algorithm for PCB design," 2025

### 2.3 Via Minimization in PathFinder: Practical Approaches

Within our negotiated congestion framework, via minimization can be addressed at
multiple levels:

**Level 1: Cost function penalty (current approach)**
Our current `via_cost_base = 10.0` is a flat penalty. This should be replaced with the
manufacturing-aware cost from section 2.1.

**Level 2: Layer assignment during global routing**
Before detailed routing, assign preferred layers to each net segment based on the
global routing topology. This is where the MAX-CUT / graph coloring approaches apply.

**Level 3: Post-route via optimization**
After convergence, sweep through the solution looking for unnecessary vias:
- **Via stacking reduction**: If a path goes L1->L2->L1, try to reroute the L2 segment
  on L1 (or L3) to eliminate both vias.
- **Via migration**: Move vias to locations where they cause less DRC stress.

**Level 4: Same-layer rerouting**
During PathFinder negotiation, when a net is ripped up and rerouted, prefer staying on
the current layer even at the cost of slightly longer wirelength. This can be achieved
by making the via penalty iteration-adaptive:

```
via_penalty(iteration) = via_base * (1.0 + 0.5 * iteration / max_iterations)
```

This progressively discourages vias as the solution stabilizes.

---

## 3. Congestion-Driven Routing with DFM Awareness

### 3.1 Congestion Estimation: State of the Art

Modern VLSI routers use sophisticated congestion estimation:

**CUGR (CUHK Global Router)**: Uses a probability-based cost scheme where routing cost is
sensitive to resource changes. A logistic function maps congestion to cost, creating a
smooth gradient that guides routing away from congested areas:

```
congestion_cost(edge) = 1 / (1 + exp(-k * (demand(edge) - capacity(edge))))
```

Where k controls the steepness. This is smoother than PathFinder's sharp `max(0, usage-1)`
and provides gradient even below capacity.

**MEDUSA (2023)**: Uses ML-based multi-resolution congestion estimation, combining 2D
and 3D congestion maps at different granularities.

**FastRoute**: Uses adaptive cost functions based on logistic functions to direct routing
to find less congested paths.

References:
- CUGR: "Detailed-Routability-Driven 3D Global Routing with Probabilistic Resource Model"
- FastRoute: https://onlinelibrary.wiley.com/doi/10.1155/2012/608362

### 3.2 DFM-Aware Congestion: Key Manufacturing Constraints

The following DFM constraints should be integrated into congestion-driven routing:

**1. Minimum Spacing Margins**
Design to 80% of manufacturer capability. If fabricator limit is 4 mil spacing, route
with 5 mil minimum. Encode as a soft penalty in the cost function:

```
spacing_margin_penalty(n) = if actual_spacing < preferred_spacing {
    dfm_weight * (preferred_spacing - actual_spacing) / preferred_spacing
} else {
    0.0
}
```

**2. Acid Trap Avoidance**
Acute angles (<90 degrees) between traces trap etchant. The grid-based router naturally
avoids this with 45-degree movement, but trace-to-pad junctions can create acid traps.
Add a penalty for nodes adjacent to pads where the approach angle would be acute.

**3. Copper Balance**
Uneven copper distribution causes warpage. Track copper density per region and add a
soft penalty for routing in already copper-dense areas:

```
copper_density_penalty(n) = if copper_density(region(n)) > target_density {
    balance_weight * (copper_density(region(n)) - target_density)
} else {
    0.0
}
```

**4. Solder Mask Slivers**
Copper features < 4 mil wide can create floating slivers. The router should avoid
creating narrow gaps between traces and pads.

**5. Trace Width Consistency**
Necking (trace width reduction) near pads is sometimes necessary but degrades
manufacturability. Minimize the length of necked-down segments.

### 3.3 Integration into PathFinder

The key challenge is integrating DFM penalties without disrupting PathFinder's convergence.

**Approach 1: Additive DFM penalty (recommended)**
Add DFM terms as separate additive costs that don't interact with the congestion
negotiation mechanism:

```
C(n) = [base * dir_penalty * corridor_penalty]     // routing preference
     + [hist_weight * history[n]]                    // negotiation: history
     + [pres_fac * max(0, usage[n] - 1)]            // negotiation: present
     + [dfm_weight * dfm_penalty(n)]                 // DFM: manufacturing
```

Where `dfm_weight` is a constant (not iteration-adaptive) so it doesn't interfere with
convergence. Suggested starting value: 1.0-2.0.

**Approach 2: DFM-aware history (advanced)**
Inject DFM violations into the history array after each iteration, similar to how our
router already injects DRC violations. This allows PathFinder to "learn" DFM hotspots:

```rust
// After each iteration, augment history with DFM penalties
for violation in dfm_check(&solution) {
    let (col, row) = grid.to_grid(violation.location);
    state.history.increment(col, row, violation.layer, DFM_PENALTY);
}
```

This is exactly what our router already does with DRC violations (see pathfinder/mod.rs
line 371-383). DFM violations can use the same mechanism.

---

## 4. Post-Route DFM Optimization Passes

### 4.1 Commercial Tool Post-Route Passes

Cadence Allegro PCB Router DFM features (from the Allegro X platform):
1. **Automatic trace spreading** -- widen spacing between traces for manufacturing margin
2. **Automatic via reduction** -- eliminate unnecessary vias from the solution
3. **Automatic miter 90-to-45** -- convert 90-degree corners to 45-degree chamfers
4. **DFM rule checking** -- real-time validation against manufacturer capabilities
5. **DesignTrue DFM** -- manufacturer rules applied during routing, not just post-check

### 4.2 Via Reduction Pass

**Algorithm: Greedy Via Elimination**
```
for each via V in solution (ordered by cost savings):
    segments_before = trace segments connected to V
    # Try to reroute the connected segments on a single layer
    for each candidate_layer in available_layers:
        new_segments = reroute_on_single_layer(segments_before, candidate_layer)
        if new_segments is valid (no DRC violations):
            if total_cost(new_segments) < total_cost(segments_before) + via_cost(V):
                replace segments_before with new_segments
                remove V
                break
```

**FreeRouting's via optimization**: Processes vias before traces in a left-to-right spatial
scan during optimization passes. For each via, attempts to reroute connected segments on
fewer layers.

### 4.3 Trace Smoothing / Pull-Tight

**FreeRouting's PullTightAlgo**:
After every trace insertion, a pull-tight pass:
1. Removes unnecessary corners
2. Straightens diagonal segments
3. Reduces total trace length
4. Operates on the local area affected by the insertion

**Algorithm: Iterative Corner Elimination**
```
loop:
    improved = false
    for each corner point P in trace:
        # Try to shortcut: connect P-1 directly to P+1
        direct_segment = line(P.prev, P.next)
        if direct_segment has no DRC violations:
            replace [P.prev -> P -> P.next] with direct_segment
            improved = true
    if not improved:
        break
```

**45-Degree Chamfering**:
Convert grid-aligned routes (H-V-H) to chamfered routes (H-45-V or H-45-H):
```
for each 90-degree corner at point P:
    # Insert a 45-degree segment
    chamfer_length = min(segment_before.length, segment_after.length, max_chamfer)
    replace corner with two 135-degree bends separated by chamfer_length
```

This eliminates acid traps and improves signal integrity. Our router already has
`CornerStyle::FortyFiveDegree` as the default post-route corner style.

### 4.4 Trace Spreading / Width Optimization

**Algorithm: Iterative Space Utilization**
```
for each trace segment S:
    available_space = min_distance_to_nearest_obstacle(S) - clearance
    if available_space > current_spacing_margin:
        # Either widen the trace or increase spacing
        if trace_width < preferred_width:
            widen trace up to preferred_width (constrained by available_space)
        else:
            center trace in available space (maximize spacing to neighbors)
```

This is a post-route pass that doesn't change topology, only geometry.

### 4.5 Recommended Post-Route DFM Pass Pipeline

Execute in this order after PathFinder convergence:

```
1. Corner chamfering     (45-degree mitering, acid trap elimination)
2. Via reduction          (eliminate unnecessary layer transitions)
3. Trace pull-tight       (shorten traces, remove unnecessary corners)
4. Trace spreading        (maximize spacing margins)
5. Redundant via insertion (add reliability vias where space permits)
6. DFM validation         (final check against manufacturer rules)
```

---

## 5. Machine Learning Approaches

### 5.1 Google AlphaChip (Circuit Training)

Google's reinforcement learning approach to chip placement (Nature 2021, used in TPU-v5):

**Architecture**: RL agent places macros sequentially, then standard cells via
force-directed method.

**Reward function (proxy cost)**:
```
reward = -1 * (w_wl * approx_HPWL + w_cong * approx_congestion + w_density * density_penalty)
```

A weighted sum of approximate wirelength (half-perimeter wire length), approximate
routing congestion, and component density. The approximations enable fast evaluation
during training (millions of iterations).

**Current status (2024-2025)**: Independent evaluations (CACM 2024) show ML-based
placement techniques lag behind optimization-based approaches like RePlAce and
DREAMPlace for standard benchmarks. The approach works best as a warm-start for
traditional optimizers rather than an end-to-end replacement.

References:
- Mirhoseini et al., "A graph placement methodology for fast chip design," Nature 2021
- "Reevaluating Google's Reinforcement Learning for IC Macro Placement," CACM 2024

### 5.2 Deep Reinforcement Learning for Global Routing

**DRL-Router (2019, updated 2023)**: Uses Double Deep Q-Network (DDQN) for global routing.

**State representation**: Congestion heat map of the routing grid
**Action space**: Route a two-pin net through one of several candidate paths
**Reward function**:
```
reward = -alpha * overflow(action) - beta * wirelength(action) + gamma * wire_sharing_bonus
```

The wire-sharing bonus encourages the agent to route nets through shared corridors,
reducing overall congestion. The DDQN architecture avoids Q-value overestimation.

**Results**: 26% reduction in DRC violations vs. baseline A* maze routing.

**Asynchronous RL with Transfer Learning** (DATE 2021): Distributed training across
multiple routing instances, with transfer learning between different netlists. Achieved
1.2% reduction in total costs.

References:
- "A Deep Reinforcement Learning Approach for Global Routing," arXiv:1906.08809
- "An Enhanced Deep Reinforcement Learning-Based Global Router for VLSI Design," 2023

### 5.3 DeepPCB: RL-Based PCB Routing

DeepPCB (by InstaDeep) uses RL specifically for PCB routing:

**Training infrastructure**: High-speed simulation engine on Cloud TPUs, achieving 235x
throughput increase and 90% cost reduction vs. CPU training.

**Reward signal**: Encodes DRC pass/fail, DFM constraint satisfaction, length matching
requirements, and via count minimization as a composite reward.

**Limitation**: The system is an "interpolation engine" -- it performs well on problems
within its training distribution but may fail on novel designs (e.g., a model trained on
consumer electronics may not work for aerospace).

Reference: https://deeppcb.ai/

### 5.4 Quilter: Physics-Driven RL for PCB

Quilter (Series B, $25M in Oct 2025) takes a different approach:

**Key differentiator**: RL trained on fundamental physics rather than human design
patterns. The system generates multiple complete layout candidates in parallel, each
representing a different valid topology/via strategy.

**Optimization objectives**: Timing margins, impedance conformance, crosstalk budgets,
PDN stability, EMI risk indicators, and manufacturer constraints -- all treated as
first-class objectives, not just guardrails.

**Manufacturing integration**: Manufacturer DFM rules are encoded as constraints that
the RL agent must satisfy, not just checked post-hoc.

Reference: https://www.quilter.ai/

### 5.5 Conflict-Based Search (CBS) from Multi-Agent Path Finding

A fundamentally different approach: treat PCB nets as agents in a Multi-Agent Path
Finding (MAPF) problem and use CBS (Conflict-Based Search) to find conflict-free routes.

**MLV-CBS (2025)**: Minimal Layer Via CBS method.
- Treats each net as an agent
- CBS provides optimal conflict resolution (no heuristic negotiation needed)
- Incorporates layer assignment via improved MIS (Maximum Independent Set) algorithm
- Two efficiency strategies: adaptive heatmap partitioning and congestion-negotiated
  routing order
- Results: favorable compared to commercial software on open-source benchmarks

**Relevance**: CBS is an alternative to PathFinder's negotiated congestion. For small
net counts it can find optimal solutions, but it does not scale as well as PathFinder
for large designs (CBS is exponential in the number of conflicts).

References:
- "Multi-agent based minimal-layer via routing algorithm for PCB design," 2025
- "PCB routing on unstructured meshes with conflict-based search," 2025

### 5.6 Practical ML for Our Router: Cost Function Tuning

Rather than replacing PathFinder with ML, the most practical approach is to use ML for
**parameter tuning**:

```
parameters = [
    pres_fac_multiplier,
    initial_pres_fac,
    history_increment,
    history_decay,
    via_cost_base,
    dfm_weight,
    dir_penalty,
    corridor_penalty,
    hist_weight,
    stagnation_threshold,
]

# Bayesian optimization over router parameters
optimizer = BayesianOptimizer(
    objective = lambda params: -router_score(board, params),
    parameter_space = parameter_bounds,
    n_iterations = 100,
)
best_params = optimizer.optimize()
```

This is orders of magnitude cheaper than training an RL agent and can be done per-board
or per-board-class. The AutoDMP framework uses Multi-objective Tree-structured Parzen
Estimator (MOTPE) and NSGA-II for similar parameter optimization in placement.

---

## 6. Open-Source Router DFM Analysis

### 6.1 FreeRouting

**Architecture**: Free-space (convex polygon) based A* routing, NOT grid-based.

**DFM-relevant features**:
| Feature | Description |
|---------|-------------|
| Minkowski sum clearance | Obstacles inflated by clearance radius; all paths inherently clearance-compliant |
| Three angle modes | 90-degree, 45-degree, free-angle routing |
| Forced shove | Push existing traces aside (20 recursion levels for traces, 5 for vias) |
| Pull-tight smoothing | Corner elimination and trace shortening after each insertion |
| Via optimization | Post-route via reduction pass (vias processed before traces in spatial scan) |
| Neckdown | Automatic trace width reduction near smaller pins |
| Optimization strategies | Global optimal, greedy, and hybrid post-route optimization |
| Scoring system | v2.1.0: Total trace length + via count + completion percentage |

**DFM gaps**:
- No acid trap detection
- No copper balance optimization
- No spacing margin optimization (uses minimum clearance, not preferred)
- No redundant via insertion
- No manufacturing cost model for vias

**Rip-up cost escalation**:
```
ripup_cost(pass_n) = base_ripup_cost * pass_number
```
Progressive escalation mimics simulated annealing over the routing solution space.

### 6.2 KiCad Interactive Router (PNS)

**Architecture**: Push-and-shove interactive router in the PNS namespace.

**Three modes**:
1. **Highlight (Mark Obstacles)** -- collision visualization only
2. **Shove** -- recursive displacement of blocking items
3. **Walk Around** -- route around obstacles without moving them

**DFM-relevant features**:
| Feature | Description |
|---------|-------------|
| Push-and-shove | Maintains clearance by moving existing traces |
| Differential pair routing | Coupled routing with impedance matching |
| Length tuning | Meander generation for length matching |
| Skew tuning | Differential pair skew control |
| 45-degree constraint | H/V/45 segments in Shove and Walk Around modes |
| DRC integration | Real-time design rule checking during routing |
| Neckdown | Automatic width reduction at smaller pads |

**DFM gaps**:
- Interactive only (no full autorouter for all nets)
- No manufacturing cost optimization
- No post-route DFM passes
- No copper balance or acid trap checking

### 6.3 Horizon EDA

Uses the same PNS (Push-and-Shove) router core from KiCad/CERN. Same capabilities
and same DFM gaps as KiCad.

### 6.4 TritonRoute (OpenROAD)

**Architecture**: VLSI detailed router (not PCB-specific, but algorithmically relevant).

**Key components**:
1. Pin access analysis
2. Track assignment
3. Initial detailed routing (maze routing on 3D grid graph)
4. Search and repair (iterative DRC fixing)
5. Built-in DRC engine

**DFM-relevant features**:
- Comprehends complex DRC rules (spacing tables, cut spacing, end-of-line spacing, min-area)
- Search-and-repair mechanism iteratively fixes DRC violations
- Via count optimization (up to 16.1% improvement over other academic routers)
- Wire length optimization (up to 0.8% improvement)

**Key insight**: TritonRoute's search-and-repair is conceptually similar to our
DRC-injection-into-history approach. Both use iterative refinement to push routes
away from DRC-violating regions.

Reference: "TritonRoute: The Open Source Detailed Router," IEEE TCAD 2020

### 6.5 Lessons for Our Router

| Feature | FreeRouting | KiCad | TritonRoute | Ours (current) | Ours (proposed) |
|---------|-------------|-------|-------------|-----------------|-----------------|
| Via cost model | Flat | Flat | ISPD-weighted | Flat | Manufacturing-aware |
| Post-route via reduction | Yes | No | Yes | No | Yes |
| Trace smoothing | Pull-tight | 45-degree | - | Corner chamfering | Pull-tight + chamfer |
| Trace spreading | No | No | No | No | Yes |
| Acid trap avoidance | No | 45-only | DRC rules | 45 movement | DFM penalty |
| DRC during routing | No | Yes | Yes | Yes | Yes |
| Redundant via insertion | No | No | No | No | Yes |

---

## 7. Redundant Via Insertion

### 7.1 Problem Formulation

Redundant vias (also called "double-cut vias" in VLSI) are additional vias placed
adjacent to existing vias for reliability improvement. Via failure during manufacturing
can cause opens; a redundant via provides a backup connection.

**Formulation as Maximum Independent Set (MIS)**:

1. Build a conflict graph G = (V, E):
   - Each vertex v in V = a candidate location for a redundant via
   - Edge (u,v) in E if placing both u and v would violate design rules OR
     if u and v are candidates for the same original via

2. Find maximum independent set of G = maximum number of redundant vias
   that can be placed without conflicts.

3. MIS on general graphs is NP-hard, but efficient heuristics exist.

**Alternative: 0-1 Integer Linear Programming (ILP)**:
```
maximize sum(x_i)  for all candidate locations i
subject to:
    x_i + x_j <= 1  for all conflicting pairs (i,j)
    x_i in {0, 1}
```

Results: ILP achieves optimal solutions with up to 73.98x speedup over heuristic
algorithms (due to modern ILP solver efficiency).

### 7.2 Redundant Via Types

```
      Original Via    On-Track Redundant     Off-Track Redundant
      ┌────┐          ┌────┐  ┌────┐        ┌────┐
      │ V  │          │ V  │──│ R  │        │ V  │
      └────┘          └────┘  └────┘        └────┘
                                               │
                                            ┌────┐
                                            │ R  │
                                            └────┘
```

- **On-track**: Redundant via placed along the trace direction. Requires less routing
  modification but may not always have space.
- **Off-track**: Redundant via placed perpendicular to trace direction. Requires a
  short stub trace but more placement options.

### 7.3 Timing-Aware Redundant Via Insertion

Extra vias change parasitic capacitance and resistance. For timing-critical nets:
- Each redundant via adds ~2-5 fF capacitance and reduces resistance by ~50%
- Net effect is usually positive (lower resistance dominates) but must be verified
- Use incremental timing analysis to verify no timing violations are introduced

### 7.4 Integration with PathFinder

**During routing (proactive)**:
Modify the cost function to prefer via locations that leave space for redundant vias:
```
via_cost(n) = base_via_cost * (1.0 - redundant_via_bonus * has_redundant_space(n))
```

Where `has_redundant_space(n)` checks if any of the 4 neighboring locations are free
for a redundant via. This incentivizes the router to place vias in locations where
redundant vias can be added later.

**Post-route (reactive)**:
Run the MIS/ILP algorithm after routing convergence to insert the maximum number of
redundant vias without DRC violations.

### 7.5 PCB-Specific Considerations

In PCB (vs. VLSI), redundant vias are less common but still valuable for:
- High-reliability designs (aerospace, automotive, medical)
- Power delivery networks (reduced via resistance)
- Signal integrity (reduced via inductance from parallel vias)

**Via density constraint**: PCB fabricators have maximum via density limits. The
redundant via insertion algorithm must respect these:
```
subject to:
    via_density(region_r) <= max_density_r  for all regions r
```

References:
- "Post-Routing Redundant Via Insertion for Yield/Reliability Improvement," ASP-DAC 2006
- "Optimal post-routing redundant via insertion," IEEE/ACM 2008
- "Redundant via Insertion: Removing Design Rule Conflicts and Balancing via Density"

---

## 8. Recommended Augmentations for Our Router

### 8.1 Phase 1: Cost Function Enhancement (Low Effort, High Impact)

**A. Manufacturing-aware via cost model**

Replace flat `via_cost_base = 10.0` with the model from Section 2.1 that accounts for
drill size, aspect ratio, annular ring, and via-in-pad. Expose via cost parameters in
`ViaCostConfig`.

**B. Spacing margin penalty**

Add an additive DFM term for traces that are close to the minimum spacing:
```rust
// In A* edge cost calculation:
let spacing_to_nearest = workspace.nearest_obstacle_distance(gx, gy, layer);
let dfm_spacing_penalty = if spacing_to_nearest < preferred_spacing {
    DFM_SPACING_WEIGHT * (1.0 - spacing_to_nearest / preferred_spacing)
} else {
    0.0
};
```

**C. DFM history injection**

Extend the existing DRC-history-injection mechanism (pathfinder/mod.rs lines 371-383)
to also inject DFM soft violations:
```rust
for dfm_issue in dfm_check(&iter_solution) {
    let (col, row) = grid.to_grid(dfm_issue.location);
    state.history.increment(col, row, dfm_issue.layer, DFM_HISTORY_PENALTY);
}
```

This requires implementing a `dfm_check()` function that identifies:
- Traces closer than preferred spacing (not DRC violation, but DFM risk)
- Via locations with insufficient redundant via space
- Acid trap geometries
- Trace segments in copper-dense regions

### 8.2 Phase 2: Post-Route Optimization (Medium Effort, High Impact)

**A. Via reduction pass**

After PathFinder convergence, attempt to eliminate each via by rerouting its
connected segments on a single layer. Use the same A* infrastructure but with
infinite via cost to force single-layer solutions.

**B. Trace pull-tight**

Iterative corner elimination on the final solution. For each intermediate point
in a trace path, check if directly connecting the predecessor to the successor
is DRC-clean. If so, remove the intermediate point.

**C. Trace spreading**

For each trace segment, compute the distance to the nearest parallel obstacle.
If there is excess margin, shift the trace to center it in the available channel.
This maximizes spacing margins without changing topology.

### 8.3 Phase 3: Advanced DFM (Higher Effort)

**A. Redundant via insertion**

Implement the MIS-based algorithm from Section 7.1. Build conflict graph from
candidate redundant via locations, solve MIS, insert winners.

**B. Copper balance optimization**

Divide the board into regions, track copper density per region, add soft penalty
for routing in already-dense regions. This encourages even copper distribution.

**C. Iteration-adaptive via penalty**

Increase via cost as iterations progress to discourage late-stage via addition:
```rust
let adaptive_via_cost = config.via_cost_base
    * (1.0 + 0.5 * state.iteration as f64 / config.max_iterations as f64);
```

### 8.4 Phase 4: ML-Based Parameter Tuning (Research)

Use Bayesian optimization (e.g., `botorch` or custom) to tune router parameters
per-board-class. The objective function runs the router and evaluates:
```
score = w1 * completion_rate + w2 * total_wirelength + w3 * via_count
      + w4 * drc_violations + w5 * dfm_score
```

Suggested starting weights based on ISPD contest:
| Objective | Weight | Rationale |
|-----------|--------|-----------|
| Completion rate | 10000 | Lexicographically dominant |
| DRC violations | 500 | Per ISPD 2018 |
| Via count | 2.0 | Per ISPD 2018 |
| Wirelength | 0.5 | Per ISPD 2018 |
| DFM score | 1.0 | Start conservative |

### 8.5 Weight Tuning Guidance

Based on ISPD contest data and literature review, recommended starting weights for
the extended cost function:

```rust
pub struct DfmCostConfig {
    /// Weight for spacing margin penalty. Default 1.5.
    pub spacing_margin_weight: f64,

    /// Weight for acid trap penalty. Default 3.0.
    pub acid_trap_weight: f64,

    /// Weight for copper density balance. Default 0.5.
    pub copper_balance_weight: f64,

    /// DFM violation history penalty (injected into PathFinder history). Default 2.0.
    pub dfm_history_penalty: f64,

    /// Bonus for via locations with redundant via space. Default 0.8
    /// (i.e., 20% cost reduction for vias with redundant space).
    pub redundant_via_bonus: f64,
}
```

The key principle: **DFM penalties should be significant enough to influence routing
decisions but not so large that they prevent convergence**. A good rule of thumb is that
DFM penalties should be comparable to (not larger than) the base movement cost, while
congestion penalties should dominate when there are actual resource conflicts.

---

## Appendix A: Key References

### Foundational Algorithms
- McMurchie & Ebeling, "PathFinder: A Negotiation-Based Performance-Driven Router for FPGAs," FPGA 1995
  https://dl.acm.org/doi/10.1145/201310.201328
- VPR Documentation: https://docs.verilogtorouting.org/en/latest/vpr/command_line_usage/

### VLSI Routing Contests
- ISPD 2018 Initial Detailed Routing Contest: https://www.ispd.cc/contests/18/
- ISPD 2019 Contest: https://www.ispd.cc/contests/19/
- ISPD 2025 Performance-Driven Large Scale Global Routing Contest: https://dl.acm.org/doi/10.1145/3698364.3715706

### Multi-Objective Optimization
- "A Multithreaded Initial Detailed Routing Algorithm Considering Global Routing Guides," ICCAD 2018
- "Net Separation-Oriented Printed Circuit Board Placement via Margin Maximization," 2022

### Via Minimization
- "Via Minimization in VLSI Chip Design - Application of a Planar Max-Cut Algorithm"
  http://e-archive.informatik.uni-koeln.de/630/
- V-GR: "3D Global Routing with Via Minimization," ASP-DAC 2024
  https://dl.acm.org/doi/abs/10.1109/ASP-DAC58780.2024.10473939
- "Constrained via minimization with practical considerations," IEEE
  https://www.researchgate.net/publication/3933936
- "Multi-agent based minimal-layer via routing algorithm for PCB design," 2025
  https://www.sciencedirect.com/science/article/abs/pii/S0167926025001907

### Congestion-Driven Routing
- CUGR: https://github.com/cuhk-eda/cu-gr
- FastRoute: https://onlinelibrary.wiley.com/doi/10.1155/2012/608362
- "Optimizing FPGA Routing with Explainable Co-Learning of Congestion and Wirelength," 2025
  https://dl.acm.org/doi/10.1145/3728467

### Open-Source Routers
- TritonRoute: https://github.com/The-OpenROAD-Project/TritonRoute
  "TritonRoute: The Open Source Detailed Router," IEEE TCAD 2020
- FreeRouting: https://github.com/freerouting/freerouting
  Math of routing: https://tinycomputers.io/posts/the-mathematics-of-pcb-trace-routing.html
- KiCad PNS: https://deepwiki.com/KiCad/kicad-source-mirror/2.4-interactive-router

### Redundant Via Insertion
- "Post-Routing Redundant Via Insertion for Yield/Reliability Improvement," ASP-DAC 2006
  https://www.cecs.uci.edu/~papers/aspdac06/pdf/p303_3C-1.pdf
- "Optimal post-routing redundant via insertion," IEEE/ACM
  https://www.researchgate.net/publication/220915472
- "Redundant via Insertion: Removing Design Rule Conflicts and Balancing via Density"
  https://www.researchgate.net/publication/220241472

### Machine Learning Approaches
- Google AlphaChip: https://research.google/blog/chip-design-with-deep-reinforcement-learning/
- "Reevaluating Google's RL for IC Macro Placement," CACM 2024
  https://cacm.acm.org/research/reevaluating-googles-reinforcement-learning-for-ic-macro-placement/
- DRL Global Routing: https://arxiv.org/abs/1906.08809
- DeepPCB: https://deeppcb.ai/
- Quilter: https://www.quilter.ai/
- CBS for PCB: https://link.springer.com/article/10.1007/s11227-025-07569-0

### PCB Manufacturing Cost Data
- Drill cost analysis: https://www.allpcb.com/allelectrohub/the-hidden-costs-in-your-pcb-drill-hole-size-and-density-impacts
- Via design rules: https://www.topfastpcb.com/blog/pcb-via-design-rules-for-reliable-manufacturing/
- DFM guide: https://www.protoexpress.com/blog/dfm-issues-pcb-manufacturing/
- Fabrication cost factors: https://resources.pcb.cadence.com/blog/2023-10-factors-in-pcb-fabrication-cost-estimation

### DFM Manufacturing Constraints
- Acid traps: https://pcbsync.com/acid-traps-pcb/
- PCB DFM rules: https://yourpcb.com/tools/reference/dfm-design-rules
- Altium via sizing: https://resources.altium.com/p/pcb-size-and-pad-size-guidelines
- Annular ring design: https://www.elepcb.com/blog/pcb-knowledge/annular-ring-pcb-design/
