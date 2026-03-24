# autopcb-research

Autonomous optimization loop for the autopcb router and placer. Inspired by
[Karpathy's autoresearch](https://github.com/karpathy/autoresearch) — an agent
modifies algorithm code, routes a benchmark board, measures quality metrics,
keeps improvements, discards regressions, and repeats indefinitely.

## Overview

The autoresearch pattern: **modify → run → measure → keep/discard → repeat**.

In our context:
- `prepare.py` equivalent → the board IR, DRC rules, evaluation harness (FIXED)
- `train.py` equivalent → router/placer algorithm code and config (EDITABLE)
- `val_bpb` equivalent → composite routing score (MEASURABLE)

## Setup

To set up a new optimization run, work with the user to:

1. **Agree on a run tag**: propose a tag based on today's date and target
   (e.g. `mar23-router`, `mar23-placer`). The branch
   `autopcb-research/<tag>` must not already exist.
2. **Create the branch**: `git checkout -b autopcb-research/<tag>` from current
   master.
3. **Read the in-scope files**: Read the files listed in the "Scope" section
   below for full context on what you can modify.
4. **Select benchmark boards**: Identify which `.pcb` files to use as
   benchmarks. At minimum use the cobra board. Record the benchmark list in
   `results.tsv`.
5. **Establish baseline**: Route all benchmarks with the current code, record
   metrics in `results.tsv`.
6. **Confirm and go**: Confirm setup looks good.

Once you get confirmation, kick off the experimentation loop.

## Benchmark Boards

Boards are in `~/cadatomic/ee-template/`. All are real designs with JLCPCB
2-layer rules.

| Board | Components | Nets | Layers | Baseline Score | Notes |
|-------|-----------|------|--------|---------------|-------|
| **hub** | 32 | 25 | 2 | 6,000,000 (6 unrouted) | Primary benchmark. USB-C hub. |
| sensor | 56 | 40 | 2 | TBD | Battery charger + sensor |
| phec | 75 | 46 | 2 | TBD | Isolated analog, more nets |
| power | 151 | 113 | 2 | TBD | Mains power, 8mm isolation, hardest |

**Primary benchmark**: `hub` (32 components, 25 nets, ~30s routing time).
Use `--all` to run the full suite when testing major changes.

### Running benchmarks

The benchmark script handles everything: routing, metric extraction, scoring,
and results logging.

```bash
# Build first
cargo build -p altium-cli --release

# Quick score check (hub only, ~30s)
./scripts/bench.sh --board hub --quiet

# Full suite (~5 min)
./scripts/bench.sh --all --quiet

# Record to results.tsv with description
./scripts/bench.sh --board hub --quiet --record "reduce via_cost_base to 8.0"

# Machine-readable JSON output
./scripts/bench.sh --board hub --quiet --json
```

The script outputs:
```
==========================================
  BENCHMARK RESULTS
==========================================

  hub          score=6000000      completion=76.0%  unrouted=6  wl=977.42mm  vias=46  drc=148  (25529ms)

  AGGREGATE
  score:       6000000.00
  ...
==========================================
```

## The Scoring Function

**Lower is better.** Uses ISPD-style lexicographic priority: completion and
DRC correctness are hard prerequisites; only after those are satisfied do we
optimize quality metrics.

```
if completion_pct < 100.0:
    score = 1_000_000 × unrouted_count
elif drc_violations > 0:
    score = 100_000 + drc_violations × 1000
else:
    score = total_wirelength_mm + 50 × total_vias
```

The priority order is non-negotiable:
1. **Completion** (unrouted nets) — must reach zero
2. **DRC violations** — must reach zero
3. **Wirelength** — shorter is better (mm)
4. **Via count** — fewer is better (×50 weight: each via ≈ 50mm of trace cost)

When running multiple benchmarks, the aggregate score is the sum of per-board
scores. A regression on ANY board counts as a regression overall — never
sacrifice one board for another.

### Why this scoring

- **Completion dominates** because an unrouted net is a non-functional board.
  No amount of wirelength improvement compensates for a missing connection.
- **DRC dominates wirelength** because a DRC violation means the board may not
  manufacture correctly or may have signal integrity issues.
- **Via cost of 50mm** is a heuristic: each via adds capacitance (~0.5pF),
  inductance (~1nH), and a reliability weak point. The weight balances via
  reduction against the wirelength cost of avoiding layer changes.

## Scope: What You CAN Modify

### Router (primary target)

These are the files you edit. Everything is fair game: algorithm parameters,
cost functions, heuristics, data structures, search strategies.

| File | What it controls |
|------|-----------------|
| `crates/autopcb-router/src/config.rs` | All tuning parameters (grid resolution, via cost, pres_fac, history, stagnation) |
| `crates/autopcb-router/src/pathfinder/mod.rs` | PathFinder negotiation loop (rip-up/reroute, convergence) |
| `crates/autopcb-router/src/detailed/grid.rs` | A* detailed router (cost function, heuristics, ROI) |
| `crates/autopcb-router/src/detailed/fanout.rs` | Pad escape planning (3-tier breakout system) |
| `crates/autopcb-router/src/global/mod.rs` | Global routing (net decomposition, coarse congestion) |
| `crates/autopcb-router/src/global/congestion.rs` | Global congestion grid |
| `crates/autopcb-router/src/optimize/*.rs` | Post-route optimization (staircase, corners, rubber-band) |
| `crates/autopcb-router/src/coopt.rs` | Placement-router co-optimization |
| `crates/autopcb-router/src/detailed/via_cost.rs` | Via cost model |
| `crates/autopcb-router/src/obstacles.rs` | Obstacle map and access point computation |
| `crates/autopcb-router/src/workspace.rs` | Workspace construction and grid config |

### Placer (secondary target)

| File | What it controls |
|------|-----------------|
| `crates/autopcb-placement/src/simulated_annealing.rs` | SA algorithm, cost function, move types, cooling schedule |
| `crates/autopcb-placement/src/lib.rs` | Analytical solver, constraint system, legalization |
| `crates/autopcb-placement/src/clustering.rs` | Net clustering for hierarchical placement |
| `crates/autopcb-placement/src/congestion.rs` | Congestion oracle integration |

### What You CANNOT Modify

- `crates/autopcb-routes/src/lib.rs` — Solution format and metrics struct (this
  is the evaluation harness).
- `crates/autopcb-ir/` — The intermediate representation (this is the input
  format). Changing it would invalidate benchmarks.
- `crates/altium-format*/` — File format parsing. Not relevant to routing quality.
- `crates/altium-cli/` — CLI interface. Not relevant to routing quality.
- Benchmark `.pcb` files — these are the fixed inputs.
- The scoring function defined above.

### What You SHOULD NOT Do

- Install new crate dependencies (work with what's available).
- Add `unsafe` code.
- Break the `cargo check` / `cargo test` build.
- Modify test assertions to make failing tests pass.

## Key Router Parameters (Starting Points)

These are the most impactful parameters in `RoutingConfig`. Default values
are the current baseline — your job is to find better ones or better algorithms.

| Parameter | Default | Range | What it does |
|-----------|---------|-------|-------------|
| `grid_resolution_mm` | 0.25 | 0.05–1.0 | Grid cell size. Finer = tighter channels but more memory (quadratic) |
| `via_cost_base` | 10.0 | 1.0–100.0 | A* penalty for layer transitions |
| `pres_fac_multiplier` | 1.5 | 1.01–3.0 | Present congestion growth rate per iteration |
| `pres_fac_cap` | 500.0 | 50–5000 | Upper limit on present congestion factor |
| `history_increment` | 1.5 | 0.1–10.0 | History cost bump per congested-node-iteration |
| `initial_pres_fac` | 0.5 | 0.01–5.0 | Starting present congestion factor |
| `history_decay` | 1.0 | 0.5–1.0 | History forgetting factor (<1.0 helps avoid fossilization) |
| `hist_weight` | 1.0 | 0.1–10.0 | History cost multiplier in A* |
| `max_iterations` | 50 | 10–200 | PathFinder iteration cap |
| `roi_initial_radius` | 24 | 8–64 | A* search region (grid cells) |
| `roi_retry_multiplier` | 2 | 1–4 | ROI expansion on retry |
| `stagnation_threshold` | 5 | 2–20 | Iterations before escalation |
| `stagnation_max` | 10 | 5–50 | Iterations before early termination |
| `movement` | FourWay | FourWay/EightWay | Cardinal vs diagonal movement |

### SA Placer Parameters

| Parameter | Default | Range | What it does |
|-----------|---------|-------|-------------|
| `cooling_rate` | 0.95 | 0.8–0.999 | Geometric cooling factor |
| `moves_per_temp` | 100 | 10–1000 | Trials per temperature level |
| `max_steps` | 5000 | 100–50000 | Temperature step cap |
| `initial_acceptance` | 0.8 | 0.5–0.99 | T₀ calibration target |
| `congestion_weight` | 0.0 | 0.0–10.0 | Congestion penalty weight |
| `critical_net_boost` | 2.0 | 1.0–10.0 | High-HPWL net cost multiplier |

## Metrics You Can Compute

### Already available in `RoutingMetrics`

```rust
pub struct RoutingMetrics {
    pub total_length_mm: f64,    // Sum of all trace segment lengths
    pub total_vias: u32,         // Total via count
    pub unrouted_count: u32,     // Nets that failed to route
    pub completion_pct: f64,     // routed / (routed + unrouted) × 100
    pub drc_violations: u32,     // DRC violation count
}
```

### Metrics you could add (ideas for experiments)

These are NOT currently computed but could be added as optimization signals:

| Metric | How to compute | Why it matters |
|--------|---------------|---------------|
| **Detour factor** | actual_length / HPWL per net | Measures path efficiency; >2.0 suggests congestion-forced detours |
| **Max congestion** | max(usage/capacity) across grid cells | Identifies hotspots; reducing max congestion improves routability |
| **Congestion variance** | σ² of usage/capacity | Even distribution is better than peaks |
| **Layer balance** | std_dev(trace_length_per_layer) | Even layer usage improves manufacturability |
| **Per-net via count** | vias per net | Nets with many vias may indicate poor layer assignment |
| **Wirelength-weighted DRC** | DRC violations weighted by net criticality | Some violations matter more than others |

### Placer metrics

| Metric | How to compute | Why it matters |
|--------|---------------|---------------|
| **HPWL** | Σ (max_x - min_x + max_y - min_y) per net | Standard placement quality proxy |
| **Overlap** | Σ intersection_area for all component pairs | Zero overlap required |
| **Routability estimate** | Run global routing, measure overflow | Predicts routing difficulty |
| **Congestion penalty** | Bin-based density exceeding target | Identifies placement hotspots |

## Output Format

After routing completes, extract metrics from the `RouteSolution`. The CLI
prints a summary like:

```
Routing complete:
  completion:     100.0%
  unrouted:       0
  total_length:   847.32mm
  total_vias:     42
  drc_violations: 0
  iterations:     23
  runtime:        4.7s
```

Compute the score:
```
score = total_length_mm + 50 * total_vias
      = 847.32 + 50 * 42
      = 2947.32
```

## Logging Results

When an experiment is done, log it to `results.tsv` (tab-separated, NOT
comma-separated — commas break in descriptions).

The TSV has a header row and 9 columns:

```
commit	score	completion	unrouted	wirelength	vias	drc	status	description
```

1. `commit` — git commit hash (short, 7 chars)
2. `score` — composite score (the number above)
3. `completion` — completion percentage (e.g. 100.0)
4. `unrouted` — number of unrouted nets
5. `wirelength` — total wirelength in mm (e.g. 847.32)
6. `vias` — total via count
7. `drc` — DRC violation count
8. `status` — `keep`, `discard`, `crash`, or `build_fail`
9. `description` — short text description of what this experiment tried

Example:

```
commit	score	completion	unrouted	wirelength	vias	drc	status	description
a1b2c3d	2947.32	100.0	0	847.32	42	0	keep	baseline
b2c3d4e	2831.50	100.0	0	831.50	40	0	keep	reduce via_cost_base to 8.0
c3d4e5f	3100.00	100.0	0	900.00	44	0	discard	increase grid resolution to 0.5mm
d4e5f6g	1000000	96.0	2	0.0	0	0	discard	aggressive pres_fac caused 2 unrouted
e5f6g7h	0.0	0.0	0	0.0	0	0	crash	compile error in grid.rs
f6g7h8i	2780.10	100.0	0	780.10	40	0	keep	add diagonal movement + history_decay 0.95
```

## The Experiment Loop

The experiment runs on a dedicated branch (e.g. `autopcb-research/mar23-router`).

LOOP FOREVER:

1. **Look at the state**: read the current branch, `results.tsv`, and the code
   you've modified so far. Identify what has worked and what hasn't.

2. **Form a hypothesis**: decide what to try next. Options include:
   - **Parameter tuning**: adjust a config value (start with the most impactful
     parameters listed above)
   - **Cost function changes**: modify the A* cost function in `grid.rs` or the
     SA cost function in `simulated_annealing.rs`
   - **Algorithm changes**: modify the PathFinder loop, global routing strategy,
     escape planning, or optimization passes
   - **New heuristics**: add a new heuristic or remove one that isn't helping
   - **Simplification**: remove complexity that doesn't improve the score

3. **Implement the change**: modify the in-scope files.

4. **Build**: `cargo build -p autopcb-router --release 2>&1 | tail -20`
   - If build fails: fix the error and try again. If you can't fix it after
     2 attempts, revert and move on.

5. **Git commit**: commit the change with a descriptive message.

6. **Run the benchmark**: route the benchmark board(s).
   ```bash
   cargo run -p altium-cli --release -- routing solve \
       --target <board>.PcbDoc <board>.pcb > run.log 2>&1
   ```

7. **Extract metrics**: parse the output for completion, wirelength, vias, DRC.

8. **Compute score**: apply the scoring function.

9. **Record in results.tsv**.

10. **Decision gate**:
    - If score improved (lower) → **keep**. The branch advances.
    - If score is equal or worse → **discard**. `git reset --hard HEAD~1` to
      revert to the last good state.
    - If score is MUCH worse but the approach has promise → note it in the
      description as `discard (promising direction)` for potential revisiting.

11. **Repeat from step 1.**

## Strategy Guide

### Phase 1: Algorithm & Feature Improvements (PRIMARY FOCUS)

DO NOT SPEND TIME TWEAKING CONFIG CONSTANTS. Config tuning has been done
(see results.tsv baseline). Improvements must come from new algorithms and
features. Implement real changes to the routing engine:

**Net ordering / reordering:**
- Dynamic net reordering: route failed nets FIRST in subsequent iterations
- Bottleneck-aware ordering: nets through congested cells get priority
- Multi-seed parallel runs: try 3-5 seeds, keep best result

**Stagnation escape:**
- On stagnation: full rip-up + shuffle net ordering + halve history array
- Currently just doubles pres_fac — implement a real restart strategy

**Access point improvements:**
- Fix same-net pass-through in `compute_access_points()` (workspace.rs:709
  passes `net_id=None` — should pass the pad's net)
- Use `pin_accesses` in the detailed router when direct pad routing fails

**First-iteration zero congestion (VPR pattern):**
- Add `first_iter_pres_fac: f64` config field (default 0.0)
- Use 0.0 congestion for iteration 0 only, then switch to `initial_pres_fac`
- This lets the first iteration find natural shortest paths

**Post-route via elimination:**
- For each via: rip up the two traces meeting at it
- Re-route with same-layer-only A*
- Accept if path exists and length penalty < 2×
- Frees routing resources on 2-layer boards

**Shove/push-aside (FreeRouting-style):**
- When A* hits a cell blocked by another net's trace, try displacing it
- Depth-1 shove: push one trace aside by 1-2 cells perpendicular
- Much more effective than pure rip-up on tight 2-layer boards

**Power net handling:**
- Detect high-fanout power nets (GND, VCC) by net class
- Route them with tree topology instead of MST point-to-point
- Or exclude them from autorouter entirely (connect via copper pour)

### Phase 2: Cost Function & Heuristic Improvements

After algorithmic changes plateau:
- A* cost function: add bend cost penalty (prefer straight runs)
- Layer assignment: soft `Any` preference for 2-layer boards instead of
  hard H/V, reducing direction penalty to 1.1
- Congestion-weighted heuristic: bias A* toward less congested regions
- Per-net escalation: after N failures, relax constraints for that net

### Phase 3: Post-Route Optimization

- Via elimination pass (see above — can also be post-route)
- More aggressive rubber-banding iterations
- Redundant segment removal
- Length equalization for matched pairs

### Things That DON'T Help (proven by experiments)

- Very fine grids (0.15mm) — 5× slower, same completion
- Very high via cost (30+) — same completion, just slower
- More iterations beyond 100 — nets that fail at iter 20 still fail at 100
- Changing numeric defaults without algorithm changes

## Simplicity Criterion

All else being equal, simpler is better.

- A 0.5% wirelength improvement that adds 50 lines of hacky code? Probably not
  worth it.
- A 0.5% wirelength improvement from deleting dead code paths? Definitely keep.
- Equal score but simpler code? Keep.
- A parameter change that achieves the same score? Keep (parameters are free
  complexity-wise).

When evaluating whether to keep a change, weigh the complexity cost against
the improvement magnitude. This is a judgment call — use the "would a senior
engineer reviewing this PR approve it?" test.

## Runtime

Each routing experiment should complete within **2 minutes** for the primary
benchmark board. If a change causes routing to take significantly longer (>3×
baseline), it should be treated as a regression even if the score improves,
unless the quality improvement is dramatic (>10% score improvement).

Record runtime alongside other metrics but don't include it in the composite
score — it's a soft constraint, not a hard one.

## Crashes and Build Failures

- **Build failure**: log as `build_fail`, fix the error, retry. If you can't
  fix after 2 attempts, revert and move on.
- **Runtime crash** (panic, OOM): log as `crash`, check the error. If it's a
  simple bug (off-by-one, unwrap on None), fix and re-run. If the approach is
  fundamentally broken, revert and move on.
- **Timeout** (>5 minutes): kill the process, log as `crash`, revert.

## NEVER STOP

Once the experiment loop has begun (after the initial setup), do NOT pause to
ask the human if you should continue. Do NOT ask "should I keep going?" or
"is this a good stopping point?". The human might be asleep, or gone from a
computer and expects you to continue working **indefinitely** until you are
manually stopped. You are autonomous.

If you run out of ideas:
- Re-read the in-scope files for angles you haven't tried
- Look at the results.tsv for patterns (what directions worked? what failed?)
- Try combining two changes that individually helped
- Try more radical algorithm changes
- Read the router/placer code more carefully for inefficiencies
- Consider adding new metrics to guide optimization
- Try approaches from EDA literature (see references below)

The loop runs until the human interrupts you, period.

## Appendix A: Detailed Metrics Reference

### Metrics We Already Compute

| Metric | Location | Formula |
|--------|----------|---------|
| `completion_pct` | `RoutingMetrics` | `routed / (routed + unrouted) × 100` |
| `total_length_mm` | `RoutingMetrics` | `Σ √((end.x-start.x)² + (end.y-start.y)²)` per segment |
| `total_vias` | `RoutingMetrics` | count of all `RoutedVia` |
| `unrouted_count` | `RoutingMetrics` | count of nets in `solution.unrouted` |
| `drc_violations` | `RoutingMetrics` | `DrcReport::total_count()` |
| `per_net_length` | `RoutedNet` | sum of segment lengths per net |
| `conflicts` | `RoutingIterationSnapshot` | oversubscribed grid cells per iteration |
| `average_congestion` | `CongestionGrid` | mean of demand/capacity across cells |
| `peak_congestion` | `CongestionGrid` | max of demand/capacity across cells |
| `bottlenecks` | `coopt::extract_bottlenecks` | sorted list of oversubscribed cells |

### Metrics Worth Adding

These can all be computed from existing `RouteSolution` + `PcbIr` data:

**HPWL lower bound** — theoretical minimum wirelength per net:
```
HPWL(net) = (max(pin_x) - min(pin_x)) + (max(pin_y) - min(pin_y))
```
Computed from `PcbIr::nets` pin positions. O(k) per net.

**Detour factor** — routing efficiency per net:
```
detour(net) = routed_length_mm / max(HPWL_mm, ε)
```
1.0 = optimal, 1.3 = 30% excess. Average across nets for board-level metric.
>2.0 indicates severe congestion-forced detours.

**Total Overflow (TOF)** — the ISPD primary metric:
```
TOF = Σ max(0, demand[cell] - capacity[cell])  over all grid cells
```
TOF = 0 means globally routable. This is THE most important congestion metric.

**Maximum Overflow (MOF)** — ISPD secondary metric:
```
MOF = max(max(0, demand[cell] - capacity[cell]))  over all cells
```
Identifies single worst bottleneck.

**Layer utilization balance** — even copper distribution:
```
per_layer_length[l] = Σ segment_length for segments on layer l
CV = std_dev(per_layer_length) / mean(per_layer_length)
```
CV < 0.3 = good. CV > 0.5 = severely unbalanced. Matters for manufacturing
(copper balance affects warpage during lamination).

**Via density map** — spatial distribution of vias:
```
via_density[cell] = count of vias in cell / cell_area
```
Hotspots indicate poor layer assignment.

**DRC severity score** — weighted violations:
```
weighted_drc = Σ weight[kind] × count[kind]
```
Proposed weights: short_circuit=1000, broken_net=500, clearance=100,
width=50, via_rule=50, length_match=30, manufacturing=10.

### Placer Metrics (for placement optimization runs)

**HPWL** — primary placement quality:
```
HPWL_total = Σ over nets: (max_pin_x - min_pin_x) + (max_pin_y - min_pin_y)
```

**RUDY congestion estimate** — fast routability prediction:
```
For each net with bounding box width W, height H:
  wire_density = (W + H) / (W × H)    [for W,H > 0]
  Add wire_density to every bin overlapping the bounding box
```

**Overlap penalty** — already in SA:
```
overlap(A,B) = max(0, min(Ax_max,Bx_max) - max(Ax_min,Bx_min))
             × max(0, min(Ay_max,By_max) - max(Ay_min,By_min))
```

## Appendix B: Benchmark Tiers

Progressive difficulty for testing router/placer improvements:

| Tier | Description | Nets | Layers | Tests |
|------|-------------|------|--------|-------|
| 0 | Single net, 2 pins, no obstacles | 1 | 1 | Basic pathfinding |
| 1 | 5-20 nets, generous spacing | 5-20 | 2 | Basic completion |
| 2 | BGA fanout (144-ball, 0.8mm pitch) | 50-144 | 2-4 | Escape routing |
| 3 | Multi-component (QFP + passives) | 50-200 | 4 | Inter-component routing |
| 4 | DDR subsystem (MCU + 2 DDR chips) | 200-500 | 4-6 | Length matching, impedance |
| 5 | Full board (MCU + memory + power) | 500-2000 | 4-8 | Everything |
| 6 | Adversarial (BGA-to-BGA, near-unroutable) | 500+ | 4 | Stress test |

For the autoresearch loop, use Tier 3-5 boards as primary benchmarks.
Tier 0-2 are for unit testing (fast, known-optimal solutions).
Tier 6 is for stress testing after major algorithm changes.

## Appendix C: Academic Scoring Formulas

### ISPD 2008 Global Routing Contest

```
Primary rank: Total Overflow (TOF) — lower is better
Tiebreaker 1: Max Overflow (MOF)
Tiebreaker 2: wirelength × (1 + CPU_time_factor)
  where CPU_time_factor = 0.04 × log₂(cpu_time / median_cpu_time)
  clamped to [-0.1, 0.1]
```

### ISPD 2018 Detailed Routing Contest

```
Primary: DRC violation count (must reach zero)
Secondary: wirelength + 2 × wrong_way_wirelength + 4 × via_count
Tertiary: runtime
```

### VLSI Cost Function (general academic form)

```
C_total = β_wl × C_wirelength
        + β_ol × C_overflow
        + β_bk × C_blockage
        + β_his × C_history
```

For via weighting: normal wire δ=1, wrong-way wire δ_ww=2, via δ_via=4.

### SA Placer Cost (TimberWolf-style)

```
Cost = λ₁ × HPWL + λ₂ × Overlap + λ₃ × Timing + λ₄ × RowLength
T₀ = std_dev(ΔCost) / ln(initial_acceptance)
T_{k+1} = α × T_k    [α typically 0.85-0.95]
```

### Analytical Placer (ePlace/DREAMPlace)

```
min  WL_WA(x,y) + λ × ElectrostaticEnergy(x,y)
```
Where WL_WA is the weighted-average smooth HPWL approximation and the
electrostatic term penalizes density (components = positive charges, solved
via FFT-based Poisson equation). This is the state of the art for VLSI but
overkill for PCB component counts (<1000).

## References

### Algorithm Background

- McMurchie & Ebeling (1995) — PathFinder negotiated congestion routing
- ISPD 2007/2008 Global Routing Contest — scoring methodology and benchmarks
- ISPD 2018/2019 Detailed Routing Contest — DRC-aware scoring
- FastRoute (Pan & Chu, 2012) — efficient global routing
- CUGR (Liu et al., 2020) — CUDA-accelerated global routing
- V-GR (2024) — via minimization with congestion awareness
- FreeRouting — open-source PCB autorouter, validates breakout heuristics
- ePlace (2015) / DREAMPlace (2019) — electrostatic analytical placement
- TimberWolf — SA placement with composite cost function
- FLUTE — fast Steiner tree construction (optimal for ≤9 pins)
- tscircuit/autorouting — open-source autorouting benchmark dataset

### Key Concepts

- **PathFinder negotiation**: routes compete for shared resources; congestion
  costs rise each iteration, forcing routes to find alternatives
- **Present congestion**: penalizes currently oversubscribed nodes (grows
  exponentially per iteration)
- **History congestion**: penalizes historically congested nodes (accumulates
  across iterations, prevents cycling)
- **HPWL**: half-perimeter wirelength — lower bound on routed wirelength,
  standard placement quality metric
- **RSMT**: rectilinear Steiner minimum tree — tighter lower bound than HPWL
  for nets with >3 pins
- **Detour factor**: actual_length / HPWL — measures routing efficiency
- **Overflow**: max(usage - capacity, 0) — the primary congestion metric
- **TOF/MOF**: total/maximum overflow — ISPD primary ranking metrics
- **RUDY**: rectangular uniform wire density — fast congestion estimation
