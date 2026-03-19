look at the @docs/future/solverang/ notes on the future autoplacer and lets do the research to expand to an autorouter too (not necessarily using solverang for the algorithms though)

#Notes

## Extending to 3D (2.5D Layering and Z-Height)

True 3D PCB placement usually means handling component height against enclosures, and handling Top/Bottom layer assignments.

* **Layer Assignment via Simulated Annealing (2.5D):** Because layer assignment is a discrete binary choice (Top vs. Bottom), it cannot be solved smoothly via Solverang's continuous Levenberg-Marquardt optimizer. Instead, you would inject a "Mirror" move into the Phase 3 Simulated Annealing (SA) pipeline, allowing the algorithm to randomly flip components to the other side of the board.
* **Layer-Aware Clearance Constraints:** Solverang's $O(N^2)$ pairwise `ComponentClearance` constraints must be updated to check a component's layer property. If `CompA.layer != CompB.layer`, the clearance residual evaluates to exactly 0 (they don't collide), allowing dense stacking of passives on the bottom underneath large ICs on the top.
* **Z-Height Mechanical Envelopes:** The LLM Phase 0 can parse enclosure CAD or mechanical drawings to extract height limits (e.g., "Under LCD, under heatsink"). This translates to a new 3D Solverang constraint: a spatial grid where `Z_max` varies. The residual would be: `max(0, component_height - Z_max_at(x, y))`. If a tall electrolytic capacitor is placed in a low-profile zone, the solver faces a massive hard-constraint penalty and pushes it out. The design notes explicitly list "3D clearance (component height)" as a Milestone 4 advanced feature.

## Preparing for a State-of-the-Art Autorouter

A SOTA autorouter (like DeepPCB, Freerouting, or Altium's ActiveRoute) will fail completely if the placement doesn't optimize for routing flow. The proposed system must optimize for the following metrics:

* **Minimizing Net Crossings (The Routability Proxy):**
HPWL (Half-Perimeter Wirelength) alone is not enough, because minimizing HPWL can still result in a tangled "rat's nest" of crossing wires. In a 2-layer or 4-layer PCB, crossing nets force the use of vias, which eat up routing channels. The notes reference the *Cypress* placement engine, suggesting the use of a "Net crossing metric" during the SA phase. The cost function must heavily penalize crossing net segments: `Cost = w₁·HPWL + w₂·net_crossings`.
* **LLM-Driven Bus & Differential Pair Alignment:**
SOTA autorouters excel when buses (e.g., DDR4 data lines) or differential pairs (e.g., HDMI TMDS, USB D+/D-) are unentangled. The LLM can analyze the netlist to detect buses and generate alignment constraints. For example, instead of just placing a connector on the edge, the LLM can output constraints like `align: center` and `near: $IC` to minimize stub lengths and impedance mismatches for differential pairs.
* **Pin-Level Resolution (Bound2Bound):**
To prep for an autorouter, the solver shouldn't just calculate wirelength from the *center* of Component A to the *center* of Component B. It needs to calculate the distance between specific pad coordinates. The notes mention the "Bound2Bound net model," which models multi-pin nets as 2-pin connections between extreme pins, giving a much more accurate approximation of how the autorouter will actually lay the copper.
* **Routing Congestion Estimates:**
Autorouters fail when trace density in a specific X-Y bin exceeds the physical capacity of the copper layers. During the SA phase (Phase 3), the cost function should include a `congestion_estimate`. This involves projecting a grid over the board, counting how many nets conceptually pass through each grid cell, and penalizing moves that push a cell past its routing capacity limit.



## Core Principles for Any Extension

When adding new types or features to the IR, you must adhere to these established design rules:

* **Keep it Read-Only (Extraction, Not Wrapping):** The IR is a one-way transformation from the raw file format (`PcbDoc`/`SchDoc`). It should never support writing back to the Altium files; it should remain a materialized view. Do not implement incremental updates; if the board changes, extract a fresh IR.
* **Use Typed Handles:** Never use raw `u16` indices (like the base format does). If you add a new entity, create a strongly typed handle (e.g., `ComponentId`, `NetId`) and store it in an `IdMap` to represent relationships.
* **Standardize on Millimeters (f64):** Convert all internal Altium coordinates (`Coord`) into `f64` millimeters during extraction to prevent integer overflow and prepare the data for the least-squares solver.
* **Omit Rendering and Format Artifacts:** Do not include display settings (colors, fonts), container details, or sidecar streams unless absolutely necessary for solving or DRC.

## Schematic Layout Extensions

To support schematic auto-layout, the IR needs to understand signal flow:

* Extend `SchIr` to extract components, nets, and wires.
* Add `IrSchComponent` to track pin positions and electrical types.
* Implement a `SignalFlowGraph` builder that creates a directed graph of nets flowing from output pins to input pins, which is required for Sugiyama layout algorithms.

## Design Rule Check (DRC) Extensions

While the base MVP focuses on placement, a DRC engine requires much more detailed copper and spatial awareness:

* **Free Copper Geometry:** Add support for extracting standalone tracks, vias, fills, and copper regions that are not owned by components.
* **Spatial Indexing:** Implement a lazy-loaded `SpatialIndex` (like an R-tree) over all copper geometry to allow the DRC engine to perform efficient pairwise distance checks.
* **Detailed Pad Stacks:** Add an `IrPadDetail` extension to extract fine-grained pad stack data (per-layer shapes, mask expansions, thermal reliefs) that placement doesn't need but DRC requires.
* **Rule Scope Evaluation:** Extend `IrDesignRule` to actually evaluate scope expressions and layer stacks (copper/dielectric thicknesses).

## Library Extentions (`PcbLib` / `SchLib`)

The spec compiler will need footprint-level operations:

* Create an `IrFootprint` type extracted from `PcbLib` (acting as a single-component IR without a netlist).
* Create an `IrSchSymbol` type extracted from `SchLib`.

## Geometry Handling (Polygons and Arcs)

When dealing with board outlines and regions that have arc segments, you should preserve the arc segments as first-class geometry within the IR. However, to support consumers like solverang that might only need polylines for AABB (Axis-Aligned Bounding Box) containment checks, you should implement a `.tessellate(resolution)` method.


---


Building a state-of-the-art (SOTA) PCB autorouter is a fundamentally different class of problem than placement. While placement is largely a continuous, floating-point optimization problem (which is why Solverang is perfect for it), routing is a **discrete, graph-search, and computational geometry problem**.

To build an autorouter on top of the `altium-format-ir` architecture you've proposed, you will need a completely different set of algorithms and crates. Modern autorouting is typically broken down into three distinct phases: **Global Routing**, **Detailed Routing**, and **Rip-up & Reroute**.

Here are the algorithms you need to implement, mapped to the Rust crates that can power them.


### 1. Global Routing (The "Planning" Phase)

Before laying down exact copper traces, the router divides the board into a coarse grid (or routing channels) and decides the general topological path each net will take to avoid congesting any single area.

* **Multi-Terminal Net Decomposition (Steiner Trees):**
You cannot just route from pin to pin in a chain. A net with 4 pins needs to be routed as a Minimum Spanning Tree (MST) or a Rectilinear Steiner Minimum Tree (RSMT) to minimize copper usage.
* *Algorithm:* **FLUTE** (Fast Lookup Table Based Rectilinear Steiner Minimal Tree) or Kruskal's/Prim's for MST.


* **Congestion-Aware Pathing:**
Assigning nets to global routing cells without exceeding the capacity of the copper layer.
* *Algorithm:* **Multicommodity Flow** or Negotiation-based A*.



**Rust Crates for Global Routing:**

* `petgraph`: Your `ir.md` notes already specify using `petgraph` for the schematic signal flow. It is also the perfect crate to represent the global routing grid graph to compute minimum spanning trees and shortest paths.
* `good_lp`: An excellent linear programming crate. Advanced global routers often formulate channel capacity limits as an Integer Linear Programming (ILP) problem.

### 2. Detailed Routing (The "Pathfinding" Phase)

This is where the actual copper tracks and vias are generated. It takes the general plan from the Global Router and finds exact, design-rule-compliant coordinates.

* **Grid-Based Maze Routing:**
The classic approach where the board is divided into a fine grid (e.g., 0.1mm) and the algorithm finds a path around obstacles.
* *Algorithm:* **A* (A-Star)** or **Lee's Algorithm** (Breadth-First Search with multiple layers and via penalties).


* **Topological / Shape-Based Routing:**
Grid-based routing uses too much memory for large, modern boards. SOTA routers (like Freerouting) use "Shape-based" routing. They treat the board as a continuous space with geometric obstacles (pads, keepouts) and find paths through the gaps.
* *Algorithm:* **Visibility Graphs** or **Delaunay Triangulation** to find safe channels between pads.



**Rust Crates for Detailed Routing:**

* `pathfinding`: This is the gold-standard Rust crate for pathfinding. It includes highly optimized implementations of A*, Dijkstra, BFS, and DFS. You will use this heavily for maze routing.
* `spade`: A phenomenal crate for Delaunay Triangulation and Voronoi diagrams. If you want to build a modern topological shape-based router, `spade` will help you find the maximal routing channels between components.

### 3. Rip-Up and Reroute (RRR)

No router succeeds on the first pass. Nets will inevitably block each other. The router must intentionally rip up (delete) conflicting traces and route them in a different order.

* **Negotiated Routing:**
Instead of permanently blocking a path, the router allows nets to overlap temporarily, but assigns a massive "penalty cost" to the overlapping grid cells. In each iteration, the cost of sharing a cell increases. Nets that have alternative routes will naturally seek cheaper, longer paths, leaving the contested bottleneck to the net that truly needs it.
* *Algorithm:* **PathFinder** (The foundational algorithm used in FPGA routing and modern PCB routers).



**Rust Crates for RRR:**

* `pathfinding`: You will use custom cost functions inside your A* search to implement the dynamic penalties of the PathFinder algorithm.

### 4. Trace Optimization & Smoothing

Detailed routing often results in ugly, jagged traces with unnecessary stair-stepping or 90-degree angles that create acid traps during manufacturing.

* **Trace Smoothing / Rubber-Banding:**
Treating the trace as a rubber band under tension. You pull the trace tight around obstacles to convert jagged 90-degree paths into smooth 45-degree or any-angle paths.

**Rust Crates for Optimization & Geometry:**

* `rstar`: As mentioned in your `ir.md` file, an R-tree spatial index is crucial for DRC clearance checks. During smoothing, you need to query an R-tree to ask, "If I pull this trace tight, does it violate clearance with any nearby pads?"
* `geo` and `geo-booleanop`: Standard Rust crates for continuous geometry. You will need these to compute polygon intersections (e.g., "does this trace cross this keepout region?") and compute offsets (to inflate a pad by the DRC clearance rule to create an obstacle boundary).
* `solverang`: Your existing least-squares solver could actually be repurposed here. You can feed a routed track into Solverang, set its endpoints as `Fixed`, set the surrounding obstacles as `Clearance` constraints, and ask Solverang to minimize the track length (HPWL). It will literally pull the track tight like a rubber band!

## Persistent / Immutable Data Structures (The im crate)

If you want to branch the entire board state to try 4 different component rotations concurrently, cloning standard Rust Vecs and HashMaps is too slow.
Instead, use the im crate (Immutable Data Structures). It uses structural sharing (like Git or functional languages).

When you clone an im::HashMap, it takes O(1) time and allocates almost no memory.

When you mutate the clone, it only allocates memory for the specific tree nodes that changed.

Result: You can branch your PcbIr state 1,000 times to try different routing paths, and simply drop the branches that fail (instant rollback).



# Architecture

Tightly integrating them is exactly how the most advanced, state-of-the-art (SOTA) EDA tools work. In academia and high-end VLSI/PCB design, this is called **Placement and Routing Co-Optimization**.

If the placer and router are strictly sequential (Placer -> Router), the router will inevitably fail on dense boards because the placer optimized for "shortest wires" (HPWL) without understanding where the actual copper needs to physically flow.

Here is how you should architect the integration between the Solverang placer and the graph-search autorouter, using feedback loops in both directions.

### 1. Forward Integration: The Placer uses Router Metrics

Instead of relying solely on Half-Perimeter Wirelength (HPWL) or basic net-crossing counts, the Simulated Annealing (SA) placement phase should invoke a "Fast Router" to grade its moves.

* **The Global Routing Oracle:** You don't run the full A* detailed router during placement (it's too slow). Instead, you run a **Global Router** (like FLUTE for Steiner Trees).
* **Congestion Maps as a Cost Function:** When the SA phase proposes swapping two components, it quickly computes the Steiner tree for the affected nets and updates a coarse 2D "Congestion Map" of the board.
* **The New SA Cost Equation:** `Cost = w₁·HPWL + w₂·net_crossings + w₃·max_cell_congestion`
If a placement move causes 15 nets to pass through a 1mm channel between two BGAs, the `max_cell_congestion` penalty explodes, and the SA phase rejects the move *before* the detailed router ever has to deal with it.

### 2. Backward Integration: The Router Adjusts Placement

When the Detailed Router (Rip-up and Reroute) gets completely stuck, it needs a mechanism to tell the placer, *"I cannot route this bus because Component U4 is exactly 1mm too far to the left."*

* **Congestion-Driven Nudging:** If the PathFinder/A* algorithm fails to resolve a congested grid cell after $X$ iterations, it identifies the physical components bordering that cell.
* **Injecting Temporary Constraints:** The router generates a temporary Solverang constraint or an SA penalty (e.g., `Repel(U4, U5, distance=3mm)`).
* **Micro-Resolves (The Rollback Loop):** Because your `altium-format-ir` can be versioned using persistent data structures (like the `im` crate discussed previously), you can pause the router, fork the board state, and hand it back to the Placer.
The Placer runs a very brief "Micro-SA" or a quick Solverang Levenberg-Marquardt pass to shift U4 slightly, satisfying the new repel constraint while keeping everything else mostly intact. The router then resumes on this new branched state.

### The Co-Optimized Architecture Pipeline

To implement this practically without turning the system into a monolithic "God object," you structure it as a pipeline with defined **Feedback Loops**:

```text
┌─────────────────────────────────────────────────────────────┐
│ 1. LLM Phase (Intent & Datasheet Constraints)               │
└──────────────────────────┬──────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────┐
│ 2. Solverang Global Placement (Continuous Math)             │
│    Optimizes HPWL subject to hard mechanical constraints.   │
└──────────────────────────┬──────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────┐
│ 3. Simulated Annealing (Discrete Tweaks)                    │
│    Uses Fast Global Routing (Steiner Trees) to avoid        │
│    creating unroutable congestion bottlenecks.              │
│    ←───── (Loopback A: Congestion Map Updates) ─────────    │
└──────────────────────────┬──────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────┐
│ 4. Detailed Autorouting (A* / Shape-based RRR)              │
│    Lays down actual copper.                                 │
│                                                             │
│    IF DEADLOCK:                                             │
│    1. Identify the physical bottleneck (e.g., clearance)    │
│    2. Generate a spatial penalty/constraint                 │
│    3. Trigger Loopback B                                    │
│    ←───── (Loopback B: Micro-Placement Adjust) ─────────    │
└──────────────────────────┬──────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────┐
│ 5. Trace Smoothing & DRC verification                       │
│    (Using Solverang to pull traces tight like rubber bands) │
└─────────────────────────────────────────────────────────────┘

```

### Why this architecture wins

By separating the engines (Solverang for continuous math, SA for discrete placement, A* for discrete routing) but allowing them to communicate via **Cost Maps** (Forward) and **Dynamic Constraints** (Backward), you get the best of both worlds.

The router isn't burdened with moving components, and the placer isn't burdened with checking design rules on 10,000 via permutations. They simply pass penalty scores and spatial constraints back and forth until the board is fully solved.


---


Integrating GPU acceleration into Electronic Design Automation (EDA) tools is the frontier of modern research (like the DREAMPlace and Cypress papers mentioned in your design notes). However, it requires a significant architectural shift: GPUs are terrible at sequential logic and branching (like A* search), but unparalleled at massive, dense math operations.

Here is how you can architect GPU optimization into both the Placer and Router, followed by the best Rust crates for a cross-vendor (NVIDIA, AMD, Intel, Apple Silicon) and cross-OS stack.


### 1. GPU-Optimizing the Placer

Your current design uses Levenberg-Marquardt (Solverang) and Simulated Annealing (SA). To use a GPU, you must shift toward **Tensor-Based Analytical Placement**.

* **The Analytical Phase (Continuous Math):** Instead of passing individual constraints to a CPU solver, you formulate the entire board as a massive set of matrices.
* **Wirelength (HPWL):** You compute the Log-Sum-Exp (LSE) approximation of HPWL for all nets simultaneously using tensor multiplication.
* **Density/Overlap:** Instead of $O(N^2)$ pairwise clearance checks, you project component bounding boxes onto a 2D grid (density map) and use the GPU to compute an electrostatic penalty (Fast Fourier Transform or 2D Convolution).
* The GPU calculates the gradients of the wirelength and density penalties, and updates all X/Y coordinates in a single parallel step.


* **The SA Phase (Parallel Markov Chains):**
Simulated Annealing is strictly sequential (Move -> Evaluate -> Accept/Reject). To parallelize it on a GPU, you use **Batched Markov Chains**. You clone your board state 1,000 times on the GPU. The GPU generates 1,000 random component swaps in parallel, evaluates the cost $\Delta C$ for all of them simultaneously, and keeps the best branch.

### 2. GPU-Optimizing the Autorouter

Routing is fundamentally a graph-search problem (A* or Lee's algorithm). Putting a standard A* priority queue on a GPU is notoriously slow because thread divergence destroys performance. Instead, you offload the **environment** to the GPU while keeping the **search** on the CPU.

* **Parallel Congestion & Distance Fields (The Cost Map):**
Instead of the CPU calculating whether a trace violates clearance against 500 components, the GPU maintains a high-resolution 2D grid of the board. Every frame, the GPU renders the "Clearance Distance Field" and "Congestion Cost" into this grid. The CPU's A* algorithm simply does an $O(1)$ memory lookup into this GPU-generated grid to check if a grid cell is safe to route through.
* **Independent Net Routing (Negotiated Routing):**
During the Rip-Up and Reroute (PathFinder) phase, if you have 200 nets to route, the GPU can run 200 independent Breadth-First Searches (BFS) simultaneously in a massive flood-fill operation. They will inevitably collide, but the GPU simply adds a penalty to the contested grid cells, resets, and runs the 200 flood-fills again.

---

### 3. The Best Rust Crates for Cross-Platform GPU Compute

If you write CUDA, your tool will only work on NVIDIA GPUs. To support Mac (Metal), Windows (DirectX 12), Linux (Vulkan), and AMD/Intel GPUs seamlessly, you must use modern abstraction layers.

####  `wgpu` (The Gold Standard for Compute Shaders)

* **What it is:** The core of Firefox's WebGPU implementation, designed as a safe, cross-platform graphics and compute API.
* **How you use it:** You write your layout density map calculations and routing flood-fills in **WGSL** (WebGPU Shading Language). `wgpu` compiles this at runtime to Vulkan (Linux), Metal (Mac), or DX12 (Windows).
* **Why it's perfect:** It requires zero vendor-specific drivers to be installed by the user. If the OS can draw a window, `wgpu` can run compute shaders on it.

#### `burn` (For Tensor-based Analytical Placement)

* **What it is:** A state-of-the-art Deep Learning framework in Rust.
* **How you use it:** Remember how DREAMPlace uses PyTorch to do placement? You can use `burn` to do the same in Rust. You define your placement positions as a `burn::Tensor`, define your LSE wirelength and density equations as operations on that tensor, and `burn` uses **Automatic Differentiation (Autograd)** to calculate the gradients and move the components.
* **Why it's perfect:** `burn` uses `wgpu` as one of its backend engines. You get the math and gradient capabilities of PyTorch, compiled natively in Rust, running on any GPU without CUDA dependencies.

### Architectural Recommendation

For the fastest path to a SOTA GPU placer/router: Use **`burn`** with its `wgpu` backend for Phase 1 (Analytical Global Placement) to push components apart using tensor math and gradients. Then, drop down to raw **`wgpu`** compute shaders to generate 2D congestion/distance grids for your CPU-bound A* router to consume.