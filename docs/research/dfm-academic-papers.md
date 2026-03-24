# DFM-Aware PCB Autorouting: Academic Research Survey

Research survey covering recent academic papers and algorithms for Design-for-Manufacturability
(DFM) aware autorouting, focused on how they could apply to our PathFinder/A*-based
grid router architecture.

Last updated: 2026-03-23

---

## Table of Contents

1. [ISPD Routing Contests and DFM Metrics](#1-ispd-routing-contests-and-dfm-metrics)
2. [DFM-Aware Detailed Routing Algorithms](#2-dfm-aware-detailed-routing-algorithms)
3. [Via Minimization and Optimization](#3-via-minimization-and-optimization)
4. [Antenna Effect Avoidance](#4-antenna-effect-avoidance)
5. [CMP-Aware Routing](#5-cmp-aware-routing)
6. [Electromigration-Aware Routing](#6-electromigration-aware-routing)
7. [Design-for-Test Routing](#7-design-for-test-routing)
8. [ML/AI Approaches to DFM Routing](#8-mlai-approaches-to-dfm-routing)
9. [Multi-Physics Aware Routing](#9-multi-physics-aware-routing)
10. [Applicability to Our Router](#10-applicability-to-our-router)

---

## 1. ISPD Routing Contests and DFM Metrics

### Contest Evolution (2018-2025)

The ISPD routing contests have progressively incorporated more DFM-relevant metrics and
design rules over time, providing the best available benchmark for what the academic
community considers important in routing quality.

#### ISPD 2018: First Detailed Routing Contest
- **Organizers**: Wei-Hao Liu, Sheng Mantik, et al.
- **Paper**: "ISPD 2018 Initial Detailed Routing Contest and Benchmarks" (ISPD '18)
- **Scoring metrics**: Wirelength + via count + DRC violations + routing guide adherence
- **Design rules enforced**: Spacing table, cut spacing, end-of-line (EOL) spacing, min-area
- **Key DFM element**: Benchmarks synthesized from industrial tools at advanced technology nodes
  with real design rules
- **Winner**: TritonRoute (UCSD) achieved best known solutions on all 2018 benchmarks

#### ISPD 2019: Advanced Routing Rules
- **Paper**: "ISPD 2019 Initial Detailed Routing Contest and Benchmark with Advanced Routing Rules"
  (ISPD '19)
- **New DFM metrics added**:
  - Double-cut via encouragement (yield improvement through redundant vias)
  - Parallel-run spacing with width-dependent rules
  - End-of-line (EOL) spacing with parallel-edge detection
  - Corner-to-corner spacing (Euclidean distance measurement)
  - Cut (via) spacing rules including cutWithin restrictions
  - Same-net spacing violations
- **Scoring**: Short violations penalized twice (500 pts per count + 500 pts per area unit),
  plus wire length, guide obedience, and track obedience penalties
- **Winner**: Dr. CU (CUHK), followed by NTUidRoute (NTU), Kim & Lee (POSTECH)

#### ISPD 2024: GPU/ML-Enhanced Global Routing
- **Organizers**: NVIDIA Research (Rongjian Liang et al.)
- **Paper**: "GPU/ML-Enhanced Large Scale Global Routing Contest" (ISPD '24)
- **Scoring formula**: `Score = w1 * TotalWL + w2 * ViaCount + OverflowScore`
  where OverflowScore uses 50x multiplier per overflow unit
- **Scale**: Benchmarks up to 50 million cells
- **Hardware**: 4x NVIDIA A100 GPUs, 8 CPU cores, 200GB RAM
- **Shift**: Moved focus to scalability and GPU acceleration; no explicit DFM scoring

#### ISPD 2025: Performance-Driven Global Routing
- **Paper**: "ISPD 2025 Performance-Driven Large Scale Global Routing Contest" (ISPD '25)
- **New additions**: Timing and power metrics integrated into scoring
- **Integration**: Industry-standard input files + OpenROAD for accurate performance assessment
- **Trend**: Moving toward multi-objective optimization (timing + power + routability)

### Contest-Winning Router Architectures

#### TritonRoute (UCSD, Andrew B. Kahng et al.)
- **Paper**: "TritonRoute: The Open Source Detailed Router", IEEE TCAD 2020
- **Architecture**: Five-stage pipeline:
  1. **Pin access analysis** (PAO - Pin Access Oracle): Finds on-track locations to access
     I/O pins without DRC violations
  2. **Track assignment**: Allocates routing resources across metal layers
  3. **Initial detailed routing**: Pattern routing honoring global routing guides
  4. **Search and repair**: Iterative rip-up and reroute
  5. **DRC engine**: In-memory design rule checking throughout
- **Key innovation**: Correct-by-construction approach to complex design rules (EOL, min-area)
- **Performance**: Improves wirelength by ~0.4%, via count by ~9.3%, DRCs by ~92% avg
- **Open source**: Part of OpenROAD project (https://github.com/The-OpenROAD-Project/TritonRoute)

#### Dr. CU 2.0 (CUHK, Evangeline F.Y. Young et al.)
- **Paper**: "Dr. CU 2.0: A Scalable Detailed Routing Framework with Correct-by-Construction
  Design Rule Satisfaction" (ICCAD '19)
- **Key algorithms**:
  - Two-level sparse data structures for enormous 3D grid graphs
  - Optimal correct-by-construction path search for min-area constraint
  - Bulk synchronous parallel (BSP) scheme for runtime reduction
- **Performance**: Up to 65% routing quality improvement over ISPD 2018 1st place, 80-93%
  memory reduction, 2.5-15x speedup

#### CUGR (CUHK, Evangeline F.Y. Young et al.)
- **Paper**: "CUGR: Detailed-Routability-Driven 3D Global Routing with Probabilistic
  Resource Model" (DAC '20)
- **Key innovation**: Quality measured solely by final detailed routing results (not just
  wirelength/overflow). Uses:
  - 3D pattern routing combining pattern routing + layer assignment
  - Multi-level 3D maze routing with different cost functions per level
  - Patching technique for adding useful route guides
- **Contest**: Won ICCAD '19 Global Routing Contest

### Relevance to our router

Our PathFinder/A* router could incorporate contest-style DFM metrics as secondary
cost terms in the A* evaluation function. The key insight from contest evolution is that
DFM awareness has shifted from post-route checking to in-route optimization.

---

## 2. DFM-Aware Detailed Routing Algorithms

### Core DFM-Aware Routing Papers

#### "DFM-aware Routing for Yield Enhancement" (IEEE, 2007)
- **Authors**: David Z. Pan et al. (UT Austin)
- **Key contribution**: Unified framework addressing three DFM concerns simultaneously:
  1. **Critical area reduction** (open/short defects)
  2. **Redundant via insertion** (yield improvement)
  3. **CMP density uniformity** (copper thickness variation)
- **Algorithm**: Detailed routing with searching process in expanding tree
- **Results**: Wire spreading achieves ~53.7% of wire length at 3x min spacing (reduces
  short defects), ~26.4% widened to 1.2x min width (reduces open defects)
- **DFM-driven routing**: 7.55% fewer vias, 18.79% more manufacturing-robust vias via doubling

#### "ECP- and CMP-Aware Detailed Routing Algorithm for DFM" (IEEE TCAD, 2009)
- **Authors**: Huang-Yu Chen, Sao-Jie Chen et al. (NTU)
- **Key contribution**: Minimizes copper thickness range after damascene process
- **Algorithm**: W-shape multilevel full-chip routing with DFS + branch-and-bound in maze
  backtracking. ECP and CMP model predictors inserted directly into maze routing cost
- **Results**: 12% improvement in metal density standard deviation, 7% reduction in dummy fill
  vs. standard maze routing

#### "CMP-aware Maze Routing Algorithm for Yield Enhancement"
- **Key idea**: Modified maze routing cost function that penalizes paths through regions
  where wire density would deviate from target uniformity
- **Integration point**: Additional cost term in A* evaluation, similar to how we add
  corridor penalties

#### Wire Spreading/Widening/Filling (SFF Methodology)
- **Paper**: "Concurrent Wire Spreading, Widening, and Filling" (DAC '07)
- **Authors**: Rizzo, Melzner et al.
- **Three-phase post-route optimization**:
  1. **Spread**: Move wires apart to increase spacing (reduces short defects)
  2. **Fatten**: Widen wires where space permits (reduces open defects)
  3. **Fill**: Insert dummy metal in remaining gaps (CMP uniformity)
- **Key model**: Critical area analysis with defect size distribution
- **Paper**: "Post-route optimization for improved yield using a rubber-band wiring model"
  (ICCAD '97)
- **Algorithm**: Rubber-band model moves vias and wires toward less dense areas while
  preserving wiring paths

#### YOR: Yield-Optimizing Routing (IEEE TCAD, 1993)
- **Author**: S.-Y. Kuo
- **Algorithm**: Channel routing with systematic critical area elimination via:
  - Floating, burying, and bumping net segments
  - Shifting vias away from vulnerable positions
- **Result**: Large reduction in critical areas, significant yield improvement

### Relevance to our router

Our post-route optimization pipeline (staircase -> corners -> rubber-band) is the natural
place to add wire spreading and widening. The rubber-band step already moves trace
segments; extending it with critical-area-aware cost functions is straightforward.
For in-route DFM, we can add density tracking to the grid and penalize paths through
over/under-dense regions in the A* cost function.

---

## 3. Via Minimization and Optimization

### Layer Assignment for Via Minimization

#### Extended Conflict-Continuation (ECC) Graph (DAC '97)
- **Paper**: "An Efficient Approach to Multilayer Layer Assignment with Application to Via
  Minimization"
- **Algorithm**: ECC graph formulation for multilayer gridless layouts. When graph is a tree,
  the problem is solved optimally in O(n) time; for non-tree graphs, constructs maximal
  induced subtrees and applies optimal algorithm to each
- **Applicability**: Post-routing layer assignment to reduce via count

#### Conjugate Conflict Continuation Graphs (Information Sciences, 2007)
- **Problem**: Multi-layer constrained via minimization
- **Algorithms**: Both ILP formulation and simulated annealing on conjugate conflict
  continuation graph model
- **Results**: 6.4% via reduction on average under practical constraints

#### V-GR: 3D Global Routing with Via Minimization (ASP-DAC '24)
- **Authors**: CUHK research team (Bei Yu et al.)
- **Key contributions**:
  - Modified via-aware routing cost sensitive to wire density impact on via placement
  - Novel multi-strategy rip-up and rerouting framework:
    1. 3D monotonic routing (controls via count)
    2. 3D 3-via-stack routing (reduces overflow)
    3. RSMT-aware expanded source 3D maze routing (shorter wire length)
- **Results**: 4.7% fewer vias, 8.7% fewer DRVs

#### MLV-CBS: Minimal Layer Via Routing for PCB (2025)
- **Paper**: "Multi-agent based minimal-layer via routing algorithm for PCB design"
  (Integration, the VLSI Journal, 2025)
- **Key innovation**: First application of Conflict-Based Search (CBS) from Multi-Agent
  Path Finding (MAPF) to PCB routing
- **Extends**: Point-to-point pathfinding to line-to-line routing with PCB-specific constraints
- **Strategies**:
  - Adaptive heatmap partitioning (AHP) for solution time reduction
  - Congestion negotiated routing order (CNRO) for efficiency
- **Results**: High routing success rate, minimal via count

### Redundant Via Insertion

#### "Redundant-Via Enhanced Maze Routing for Yield Improvement" (ASP-DAC '05)
- **Authors**: UT Austin (David Z. Pan group)
- **Algorithm**: First routing algorithm considering redundant via insertion feasibility
  during detailed routing. Formulated as multiple-constraint shortest path problem,
  solved by Lagrangian relaxation
- **Key idea**: During maze routing, the cost function considers whether a via location
  permits a redundant via adjacent to it

#### "Fast and Optimal Redundant Via Insertion" (IEEE TCAD, 2008)
- **Algorithm**: 0-1 ILP formulation of double-cut via insertion (DVI) problem
- **Performance**: Up to 74x speedup over heuristic algorithms while finding optimal solutions
- **Post-routing**: Maximum independent set (MIS) formulation for inserting redundant vias
  without DRC violations

### Relevance to our router

Our via cost model (`ViaCostConfig`) already penalizes vias in A*. We could extend this to:
1. **Layer assignment post-processing**: After PathFinder converges, run ECC-graph-based
   layer reassignment to reduce via count while preserving connectivity
2. **Redundant via insertion**: Post-route pass that attempts to add second cut to every via
   where DRC permits (simple geometric check)
3. **Via-aware cost in A***: Like V-GR, modify the via transition cost to account for local
   wire density (dense regions get higher via penalty to preserve routing resources)

---

## 4. Antenna Effect Avoidance

### Background

The antenna effect (plasma-induced gate oxide damage) occurs during plasma etching of metal
interconnects. Charge accumulates on long metal conductors connected to transistor gates,
and excessive charge discharges through thin gate oxide, causing permanent damage.

**Antenna rule**: Expressed as maximum allowable ratio of metal area to gate area per
interconnect layer. Violation means too much metal connected to a gate without
source/drain discharge path.

**Primarily a VLSI/IC concern**, less relevant for PCB routing where there are no gate
oxides. However, understanding the techniques is valuable for completeness and for any
future IC routing work.

### Key Algorithms

#### Jumper Insertion for Antenna Avoidance
- **Paper**: "An Exact Jumper Insertion Algorithm for Antenna Effect Avoidance/Fixing"
  (DAC '05)
- **Technique**: Break long metal wire by forcing layer change (metal N -> metal N+1 -> metal N),
  reducing antenna ratio on the original layer
- **Algorithm**: Polynomial-time exact algorithm

#### Simultaneous Diode/Jumper Insertion
- **Paper**: "An Optimal Simultaneous Diode/Jumper Insertion Algorithm for Antenna Fixing"
  (ICCAD '06, Y.-W. Chang et al., NTU)
- **Algorithm**: Minimum-cost network-flow formulation for polynomial-time optimal solution
- **Technique**: Combines diode insertion (n+ in p-substrate provides discharge path) with
  jumper insertion, choosing the minimum-cost combination

#### Multilevel Routing with Antenna Avoidance
- **Paper**: "Multilevel routing with jumper insertion for antenna avoidance"
  (Integration, the VLSI Journal)
- **Integration**: Antenna checking built into multilevel routing framework;
  layer assignment considers antenna ratios during routing, not just as post-processing

### Relevance to our router

Not directly applicable to PCB routing (no gate oxides), but the jumper insertion technique
is conceptually similar to our via escape planning. The network-flow formulation for optimal
jumper placement could be adapted for via optimization problems.

---

## 5. CMP-Aware Routing

### Background

Chemical-Mechanical Polishing (CMP) is used in semiconductor manufacturing to planarize
copper interconnect layers. Non-uniform copper density causes:
- **Over-polishing** in low-density regions (copper thinning, increased resistance)
- **Under-polishing** in high-density regions (shorts, planarity issues)
- **Dishing**: Copper recessed below oxide surface in wide features
- **Erosion**: Oxide removal in dense copper areas

The goal is uniform copper density across the die/board, typically within each layer.

### Key Papers

#### Wire Density-Driven Routing for CMP Control
- **Paper**: "A Novel Wire-Density-Driven Full-Chip Routing System for CMP Variation Control"
  (IBM Research, IEEE TCAD)
- **Algorithm**: Global routing with wire density as unified metric for both CMP variation
  and timing optimization
- **Key insight**: Wire density driven global routing on a layer-by-layer basis using
  quadratic congestion optimization

#### Full-Chip CMP+ECP Routing System
- **Paper**: "Full-chip routing system for reducing Cu CMP & ECP variation" (SBCCI '08)
- **Innovation**: First work to consider both Cu CMP and electroplating (ECP) variation
  throughout the entire routing procedure
- **Three stages**: CMP-aware global routing -> CMP-aware layer assignment ->
  ECP-aware detailed routing

#### Dummy Fill Insertion Algorithms
- **Paper**: "Provably Good and Practically Efficient Algorithms for CMP Dummy Fill"
  (UCSD, Andrew Kahng et al.)
- **Algorithm**: O(n log n) effective model-based dummy insertion (EMDI) vs. previous
  O(n^3) LP approaches
- **Paper**: "A Novel and Unified Full-Chip CMP Model Aware Dummy Fill Insertion Framework"
  (Peking University, 2021)
- **Innovation**: Full-chip CMP simulator-aware dummy fill with unified optimization framework

#### GAN-Dummy Fill (GLSVLSI '22)
- **Authors**: Myong Kong, Daeyeon Kim, Minhyuk Kweon, Seokhyeong Kang
- **Innovation**: GAN trained with density + parasitic capacitance loss function to generate
  dummy fill patterns
- **Results**: Reduces negative timing slack from parasitic capacitance by up to 45% vs.
  commercial tools

#### RL-Fill (ICCAD '24)
- **Paper**: "RL-Fill: Timing-Aware Fill Insertion using Reinforcement Learning"
- **Algorithm**: Two-phase training:
  1. Imitation learning from expert data
  2. Online RL optimization
- **Innovation**: LayoutMix data augmentation for data-efficient training
- **Approach**: Generate fills everywhere, then RL policy removes timing-critical fills

### Relevance to our router

For PCB routing, CMP is less relevant than for IC fabrication, but copper density
uniformity still matters for:
- **Etch uniformity**: Non-uniform copper distribution affects etching quality
- **Warpage/bow**: Uneven copper distribution causes board warpage during lamination
- **Impedance control**: Local copper density affects dielectric thickness after pressing

Our router could track per-layer copper density in a grid overlay and add a density
deviation penalty to the A* cost function, biasing routes toward under-filled regions.
Post-route copper fill (dummy pads/traces) would handle remaining imbalance.

---

## 6. Electromigration-Aware Routing

### Background

Electromigration (EM) is the gradual displacement of metal atoms in a conductor due to
momentum transfer from conduction electrons. At high current densities, this causes:
- **Voids**: Material depletion creating opens
- **Hillocks**: Material accumulation creating shorts

For PCBs, EM manifests as trace/via degradation under sustained high-current loads,
especially at elevated temperatures.

### Key Current Density Formula (IPC Standards)

#### IPC-2221 (Legacy, 1950s-era data)
```
I = K * dT^0.44 * A^0.725
```
Where: I = current (A), dT = temperature rise (C), A = cross-sectional area (mil^2),
K = 0.048 (outer layers) or 0.024 (inner layers)

#### IPC-2152 (Modern, 2009)
Chart-based methodology accounting for:
- Board thickness and thermal conductivity
- Dielectric material properties
- Proximity to copper planes
- Via thermal effects
- Ambient temperature

IPC-2152 is significantly less conservative than IPC-2221, especially when copper pour
and planes are present. No single formula; uses lookup charts from empirical testing.

### Academic EM-Aware Routing

#### "Electromigration Design Rule Aware Global and Detailed Routing Algorithm" (GLSVLSI '18)
- **Authors**: Xiaotao Jia, Jing Wang, Yici Cai, Qiang Zhou
- **Algorithm**:
  - Global routing: EM-aware maze routing with physics-based EM model as design rule
  - Detailed routing: Concurrent EM-aware router based on multi-commodity flow method
- **Results**: 92% reduction in EM risk with slight increase in wire length and via count
- **Key insight**: EM modeled as physical design rule, not just post-route check

#### "Electromigration-aware Routing for 3D ICs with Stress-aware EM Modeling" (UT Austin)
- **Authors**: UTDA group (David Z. Pan et al.)
- **Innovation**: Thermal-stress-aware EM model for 3D IC interconnects

#### "On Potential Design Impacts of Electromigration Awareness" (UCSD)
- **Authors**: Andrew Kahng et al.
- **Approach**: NDR (non-default rules) in ECO routing to widen wires and spacings,
  fanout reduction, driver downsizing

### Practical EM-Aware Routing for PCB

The typical PCB approach is simpler than IC:
1. **Width constraints per net class**: Power nets get wider traces based on expected current
2. **Via current limits**: Multiple vias in parallel for high-current nets
3. **Thermal relief**: Connection style to copper pours for thermal management
4. **Current density rules**: `J_max = I / (w * t)` where w = trace width, t = copper thickness

### Relevance to our router

Our `NetRoutingConfig::width_override` already supports per-net-class trace width overrides.
To make this EM-aware:
1. **Current-aware width selection**: Given expected current per net (from spec), automatically
   compute minimum trace width using IPC-2152 charts
2. **Variable-width routing**: Route at minimum width in unconstrained areas, widen near
   high-current branches (requires grid cells to track width, or post-route widening pass)
3. **Via current budgeting**: When a net carries high current, the via cost model should
   prefer multiple parallel vias over single vias
4. **Thermal coupling**: High-current traces should avoid tight parallel runs with other
   high-current traces (add proximity penalty)

---

## 7. Design-for-Test Routing

### Test Point Accessibility

#### The Problem
PCB testing requires physical or electrical access to circuit nodes for verification.
In-circuit test (ICT) and flying-probe testers need test points (pads, vias, or dedicated
probe pads) accessible from one or both sides of the board. Typically only ~70% of nodes
are accessible without DFT planning.

#### DFT Routing Considerations
- **Test pad placement**: Each net should have at least one accessible test point
- **Probe spacing**: Test points must respect minimum probe pitch (typically 50-100 mil grid)
- **Via accessibility**: Through-hole vias on grid locations serve as natural test points
- **Boundary scan (JTAG)**: IEEE 1149.1 test access port requires dedicated routing for
  TCK, TMS, TDI, TDO signals in daisy-chain topology

### Security-Oriented PCB Routing with DRL (2025)
- **Paper**: "Security-oriented printed-circuit-board routing with deep reinforcement learning"
  (Integration, the VLSI Journal, 2025)
- **Authors**: Katherine Shu-Min Li, Fang-Chi Wu, Ching-Han Lai, Sying-Jyan Wang
- **Framework**: ARTPI (three phases):
  1. Routing preprocessing with net priority assignment
  2. A*-based multilayer net routing with custom heuristic
  3. Reinforcement learning-based test point insertion
- **Results**: 100% routing success, full test point coverage across all evaluated designs
- **Key insight**: Test point insertion can be integrated into routing, not just post-processing

### Relevance to our router

Test point accessibility could be modeled as additional constraints in our routing:
1. **Via placement on grid**: Prefer via locations that align with standard test probe grids
   (add grid-alignment bonus to via cost)
2. **Test point coverage**: Track which nets have accessible test points; after routing,
   identify nets without and attempt via insertion at accessible locations
3. **JTAG chain routing**: Treat as special net class with point-to-point ordering constraint

---

## 8. ML/AI Approaches to DFM Routing

### PCB-Specific ML Routing

#### FanoutNet (AAAI '23)
- **Authors**: Chinese research team
- **Problem**: BGA/QFP fanout automation (directly relevant to our escape planning)
- **Architecture**: CNN + attention-based networks trained with PPO (Proximal Policy Optimization)
- **Approach**: Policy model learns PCB layout representations to make fanout decisions,
  value model evaluates quality
- **Results**: 100% routability on all industrial cases, 6.8% wirelength improvement
- **Relevance**: Our 3-tier breakout system (stub/perimeter/via escape) could potentially
  be enhanced with learned policies for escape direction selection

#### Unet-Astar: Deep Learning-Based Fast PCB Routing (IEEE Access, 2023)
- **Algorithm**: Deeper U-Net predicts feasible routing regions, then A* searches within
  predicted regions only
- **Key innovation**: ML model captures remote contextual information from PCB layout to
  predict where routes should go, dramatically reducing A* search space
- **Results**: ~70% runtime improvement compared to vanilla A* router
- **Open source**: https://github.com/Firesuiry/Unet-Astar-For-PCB-Routing
- **Relevance**: Could be adapted to predict congestion-free corridors for our A* detailed
  router, similar to how our coarse global routing already constrains the search

#### DeepPCB (InstaDeep, Commercial, 2023-present)
- **Technology**: Reinforcement learning for both placement and routing
- **Capabilities**: DRC-clean layouts, via minimization, differential pair support,
  dynamic wire widths, multi-plane routing
- **Approach**: RL agent learns from experience, unlike classical solvers that start
  from scratch per instance
- **Relevance**: Demonstrates commercial viability of RL-based PCB routing

#### Dueling Double Deep Q-Network for PCB Routing (2024)
- **Paper**: "Reinforcement Learning Based PCB Routing Using Dueling Double Deep Q Network"
- **Algorithm**: Multi-agent RL with local observation using D3QN
- **Relevance**: Multi-agent formulation maps naturally to our per-net PathFinder decomposition

#### PCB Routing on Unstructured Meshes with CBS (J. Supercomputing, 2025)
- **Key innovation**: Applies conflict-based search from MAPF domain to PCB routing on
  unstructured (non-grid) meshes
- **Relevance**: Demonstrates that negotiation-based routing (like our PathFinder) can be
  formulated in the CBS framework

### VLSI ML/AI Routing (Applicable Techniques)

#### MEDUSA: ML Congestion Estimation (ACM TODAES, 2023)
- **Algorithm**: Multi-resolution CNN with sliding-window approach
- **Three parts**: Feature extraction + hyper-image encoding, fixed-resolution CNN prediction,
  sliding-window for full-chip coverage
- **Results**: 22-54% lower initial overflow, up to 3x less runtime vs. other estimators
- **Relevance**: Congestion prediction could replace or augment our coarse global routing
  for initial corridor guidance

#### RouteNet: Routability Prediction (ICCAD '18)
- **Algorithm**: CNN-based routability prediction for mixed-size designs
- **Relevance**: Could predict which regions of our board will be hard to route, informing
  net ordering and escape planning strategy

#### ML-Enhanced Global Routing (ISPD '24 Contest)
- **Trend**: GPU-accelerated routing with ML guidance achieving 50M+ cell scalability
- **Relevance**: While our PCB scale is smaller, GPU acceleration of A* search and
  ML-guided net ordering are directly applicable

### Relevance to our router

The most immediately applicable ML techniques are:
1. **Congestion prediction** (Unet/MEDUSA style): Train a small CNN on our grid to predict
   congestion before routing, using this as corridor guidance for A*
2. **Learned escape planning** (FanoutNet style): Replace heuristic escape direction
   selection with a learned policy
3. **Net ordering** (RL): Learn optimal net ordering for PathFinder iterations rather than
   using MST-weight heuristic

---

## 9. Multi-Physics Aware Routing

### Thermal-Aware Routing

#### Thermal Via Optimization
- **Paper**: "Thermal Modeling and Design Optimization of PCB Vias and Pads"
  (IEEE Trans. Components, Packaging and Manufacturing Technology, 2019)
- **Authors**: Shen, Wang et al.
- **Key findings**:
  - Optimal via diameter for thermal conductivity: 0.30mm
  - Optimal via-to-via spacing: 0.80mm
  - Plugged thermal vias provide best heat conduction paths
- **Optimization**: Uses analytical thermal resistance models for vias and pads

#### PCB Thermal Layout Optimization
- **Algorithm**: MFEP + genetic algorithm for power device thermal layout
- **Results**: ~19C reduction in maximum junction temperature vs. initial layout

### Electrical-Thermal-Mechanical Co-optimization

#### Cypress: VLSI-Inspired PCB Placement with GPU Acceleration (ISPD '25 Best Paper)
- **Authors**: Niansong Zhang et al. (Cornell/NVIDIA)
- **Key contributions**:
  - First GPU-accelerated PCB placement method adapted from VLSI techniques
  - Defines "net crossing" metric for routing resource estimation
  - Macro halo technique for spacing constraint enforcement
  - Tailored cost functions for PCB-specific constraints (thermal, mechanical, electrical)
- **Results**: Up to 492x speedup in runtime
- **Open source**: https://github.com/NVlabs/Cypress
- **Relevance**: Placement quality directly impacts routing difficulty; better placement
  reduces DFM challenges during routing

#### Ansys Electrothermal-Mechanical Design Flow
- **Commercial approach**: Coupled electrical -> thermal -> mechanical simulation loop
- **Sequence**: Power loss analysis -> temperature distribution -> thermal stress/strain
- **Relevance**: Defines the multi-physics metrics that routing should optimize for

### Relevance to our router

Multi-physics awareness in our router would be implemented as:
1. **Thermal via insertion**: Post-route pass that adds thermal vias under hot components
   (power regulators, FPGAs) based on thermal budget from spec
2. **Current-aware routing**: Wider traces for high-current nets (already partially supported
   via `width_override`), with automated width selection based on IPC-2152
3. **Impedance-controlled routing**: Trace width/spacing tied to stackup parameters for
   controlled impedance (partially handled by our DRC policy)

---

## 10. Applicability to Our Router

### Current Architecture Recap

Our router uses:
- Grid-based routing with configurable resolution (default 0.25mm)
- PathFinder negotiated congestion (McMurchie-Ebeling '95)
- A* detailed routing with multi-objective cost function
- MST-based net decomposition
- Coarse global routing for corridor guidance
- 3-tier breakout (stub/perimeter/via escape) for BGA/dense pads
- Post-route optimization (staircase -> corners -> rubber-band)
- DRC engine with clearance checking

### DFM Features: Priority-Ranked Implementation Roadmap

#### Tier 1: Low-Hanging Fruit (Cost function modifications + post-processing)

| Feature | Effort | Impact | How |
|---------|--------|--------|-----|
| **Redundant via insertion** | Low | High yield | Post-route: for each via, attempt double-cut placement if DRC allows |
| **Wire spreading** | Medium | High yield | Extend rubber-band optimization to push traces apart when space exists |
| **Wire widening** | Medium | High yield | Post-route: widen traces where clearance > minimum, respecting critical area model |
| **Via grid alignment** | Low | DFT | Prefer via placement on standard test probe grid (add alignment bonus to via cost) |
| **Copper density tracking** | Medium | Etch quality | Add per-layer density overlay; penalize routes through over-dense regions |

#### Tier 2: Cost Function Enhancements (A* modifications)

| Feature | Effort | Impact | How |
|---------|--------|--------|-----|
| **EM-aware width selection** | Medium | Reliability | Auto-compute trace width from net current + IPC-2152; enforce as per-net constraint |
| **Density-driven routing** | Medium | CMP/etch | Add copper density term to A* cost: `density_penalty = abs(local_density - target) * weight` |
| **Via-aware routing cost** | Low | Via count | Like V-GR: increase via penalty in already-dense via regions to spread via load |
| **Critical area cost term** | High | Yield | Model open/short defect probability based on local wire geometry; add to A* cost |

#### Tier 3: Advanced Features (New pipeline stages)

| Feature | Effort | Impact | How |
|---------|--------|--------|-----|
| **Layer assignment post-pass** | High | Via count | ECC-graph or ILP-based layer reassignment after initial routing to minimize vias |
| **ML congestion prediction** | High | Routability | U-Net trained on routing results to predict congestion, replace coarse global routing |
| **Learned escape planning** | High | BGA routing | FanoutNet-style PPO policy for escape direction selection |
| **Thermal via insertion** | Medium | Thermal | Post-route: identify thermal hotspots from component power specs, insert thermal via arrays |
| **Test point coverage** | Medium | DFT | Post-route audit: identify nets without accessible test points, insert via at probe-grid location |

### Key Cost Function Extension

Our current A* cost is:
```
C(n) = base * dir_penalty * corridor_penalty + hist_weight * history[n] + pres_fac * max(0, usage[n] - 1)
```

A DFM-extended version could be:
```
C(n) = base * dir_penalty * corridor_penalty
     + hist_weight * history[n]
     + pres_fac * max(0, usage[n] - 1)
     + density_weight * abs(density[layer][region] - target_density)     // CMP/etch uniformity
     + em_weight * max(0, current_density[n] - j_max)                   // Electromigration
     + critical_area_weight * defect_susceptibility(n, neighbors)        // Yield
```

Where new terms are only activated when DFM mode is enabled in `RoutingConfig`.

### Key Academic Groups to Watch

| Group | Institution | Strengths | Key Tools |
|-------|------------|-----------|-----------|
| Andrew B. Kahng | UC San Diego | Detailed routing, DFM, contests | TritonRoute, TritonRoute-WXL |
| Evangeline F.Y. Young | CUHK | Global/detailed routing, contests | CUGR, Dr. CU, V-GR |
| David Z. Pan | UT Austin | DFM-aware routing, EM, yield | UTDA publications |
| Yao-Wen Chang | NTU | Antenna, multilevel routing | NTUidRoute |
| Zhiru Zhang | Cornell/NVIDIA | GPU-accelerated EDA, PCB | Cypress |
| InstaDeep | Industry | RL-based PCB routing | DeepPCB |

---

## References

### ISPD Contests
- W.-H. Liu et al., "ISPD 2018 Initial Detailed Routing Contest and Benchmarks", ISPD '18
- W.-H. Liu et al., "ISPD 2019 Initial Detailed Routing Contest and Benchmark with Advanced Routing Rules", ISPD '19
- R. Liang et al., "GPU/ML-Enhanced Large Scale Global Routing Contest", ISPD '24
- R. Liang et al., "ISPD 2025 Performance-Driven Large Scale Global Routing Contest", ISPD '25

### Contest Winners / Open Source Routers
- A.B. Kahng et al., "TritonRoute: The Open Source Detailed Router", IEEE TCAD, 2020
- G. Li et al., "Dr. CU 2.0: A Scalable Detailed Routing Framework with Correct-by-Construction Design Rule Satisfaction", ICCAD '19
- J. Liu et al., "CUGR: Detailed-Routability-Driven 3D Global Routing with Probabilistic Resource Model", DAC '20
- L. He et al., "SPRoute 2.0: A Detailed-Routability-Driven Deterministic Parallel Global Router with Soft Capacity", ASP-DAC '22

### Via Minimization
- V-GR authors, "V-GR: 3D Global Routing with Via Minimization and Multi-Strategy Rip-up and Rerouting", ASP-DAC '24
- MLV-CBS authors, "Multi-agent based minimal-layer via routing algorithm for PCB design", Integration, 2025
- C.-L. Lee, "Redundant-Via Enhanced Maze Routing for Yield Improvement", ASP-DAC '05
- Y.-J. Lee, "Fast and Optimal Redundant Via Insertion", IEEE TCAD, 2008

### DFM-Aware Routing
- D.Z. Pan et al., "DFM-aware Routing for Yield Enhancement", IEEE, 2007
- H.-Y. Chen et al., "ECP- and CMP-Aware Detailed Routing Algorithm for DFM", IEEE TCAD, 2009
- IBM Research, "Wire Density Driven Global Routing for CMP Variation Control", IEEE TCAD
- Rizzo et al., "Concurrent Wire Spreading, Widening, and Filling", DAC '07
- S.-Y. Kuo, "YOR: A Yield-Optimizing Routing Algorithm by Minimizing Critical Areas and Vias", IEEE TCAD, 1993

### Antenna Effect
- DAC '05, "An Exact Jumper Insertion Algorithm for Antenna Effect Avoidance/Fixing"
- Y.-W. Chang et al., "An Optimal Simultaneous Diode/Jumper Insertion Algorithm for Antenna Fixing", ICCAD '06

### CMP and Copper Density
- A.B. Kahng et al., "Provably Good and Practically Efficient Algorithms for CMP Dummy Fill"
- PKU, "A Novel and Unified Full-Chip CMP Model Aware Dummy Fill Insertion Framework", 2021
- M. Kong et al., "GAN-Dummy Fill: Timing-aware Dummy Fill Method using GAN", GLSVLSI '22
- "RL-Fill: Timing-Aware Fill Insertion using Reinforcement Learning", ICCAD '24

### Electromigration
- X. Jia et al., "Electromigration Design Rule Aware Global and Detailed Routing Algorithm", GLSVLSI '18
- UT Austin, "Electromigration-aware Routing for 3D ICs with Stress-aware EM Modeling"
- A.B. Kahng et al., "On Potential Design Impacts of Electromigration Awareness", UCSD

### ML/AI for Routing
- FanoutNet authors, "FanoutNet: A Neuralized PCB Fanout Automation Method Using Deep Reinforcement Learning", AAAI '23
- Unet-Astar authors, "Unet-Astar: A Deep Learning-Based Fast Routing Algorithm for Unified PCB Routing", IEEE Access, 2023
- MEDUSA authors, "MEDUSA: A Multi-Resolution Machine Learning Congestion Estimation Method for 2D and 3D Global Routing", ACM TODAES, 2023
- K.S.-M. Li et al., "Security-oriented printed-circuit-board routing with deep reinforcement learning", Integration, 2025

### Multi-Physics and Thermal
- N. Zhang et al., "Cypress: VLSI-Inspired PCB Placement with GPU Acceleration", ISPD '25 (Best Paper)
- Shen & Wang et al., "Thermal Modeling and Design Optimization of PCB Vias and Pads", IEEE TCPMT, 2019

### PathFinder / Negotiation-Based Routing
- L. McMurchie & C. Ebeling, "PathFinder: A Negotiation-Based Performance-Driven Router for FPGAs", FPGA '95
- L. He et al., "SPRoute: A Scalable Parallel Negotiation-based Global Router", ICCAD '19
