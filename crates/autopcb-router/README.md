# autopcb-router

Pure-algorithm PCB autorouter. Receives `PcbIr` (produced by the spec compiler) and
`RoutingConfig`, and returns a `RouteSolution` that is serialized to a `.routes` file
and imported back into the spec pipeline.

## Architecture

```
pcbdoc-spec (source of truth)
  │
  ├── spec compiler ──→ PcbIr (board model in mm)
  │                       │
  │                       ▼
  │                 autopcb-router (pure algorithm)
  │                   RoutingConfig ──→ RouteSolution
  │                                        │
  │                                        ▼
  │                               autopcb-routes (format crate)
  │                                 write ──→ board.routes (binary/JSON)
  │                                        │
  ├── import "board.routes" ◄──────────────┘
  │
  ▼
spec compiler apply ──→ PcbDoc output (tracks, vias from route solution)
```

**PcbDoc is never a direct input.** Everything flows through
`pcbdoc-spec` → `PcbIr` → `autopcb-router`.

## Derived-State-Only Policy

This crate defines no board IR types. All geometry, component, net, pad, and rule
types come from `autopcb-ir`. The types defined here are transient optimization
structures (grids, obstacle maps, congestion arrays, pathfinder state) that are built
fresh from `PcbIr` + `RoutingConfig` for each invocation and never persisted.

## Data Flow

```
pcbdoc-spec ──parse──→ PcbDocSpec
                         │
                         ├── RoutingSpec (grid resolution, max iterations, via costs, seed)
                         └── target PcbDoc path
                                │
                          load + extract
                                │
                                ▼
                             PcbIr (with placement applied)
                                │
                    ┌───────────┴───────────┐
                    ▼                       ▼
            RoutingConfig              Board geometry,
          (from RoutingSpec)           nets, pads, rules,
                    │                  obstacles, layers
                    └───────────┬───────────┘
                                ▼
                    RoutingWorkspace (derived)
                    ├── ObstacleMap (R-tree + bitmaps)
                    ├── RoutingGrid (3D: x, y, layer)
                    ├── GlobalRoutingGraph (coarse grid)
                    └── PathFinderState (history, congestion)
                                │
                         route_board()
                                │
                                ▼
                         RouteSolution
                         ├── routed nets (segments + vias)
                         ├── unrouted nets
                         ├── metrics (HPWL, via count, etc.)
                         └── iteration snapshots (for viewer)
                                │
                           serialize
                                │
                    ┌───────────┴───────────┐
                    ▼                       ▼
            board.routes (bincode)   board.routes.json
```

## Public API

| Function | Signature | Purpose |
|----------|-----------|---------|
| `build_workspace` | `(&PcbIr, &RoutingConfig) -> Result<RoutingWorkspace>` | Build all derived state from the board IR |
| `route_board` | `(&RoutingWorkspace, &PcbIr, &RoutingConfig) -> Result<RouteSolution>` | Route the full board via PathFinder negotiation |
| `route_single_net` | `(&RoutingWorkspace, &PcbIr, &RoutingConfig, NetId) -> Result<RoutedNet>` | Route one net without modifying other nets |
| `optimize_solution` | `(&RoutingWorkspace, &mut RouteSolution) -> Result<()>` | Apply post-route optimization passes |

Re-exported types: `RoutingConfig`, `RoutingWorkspace`, `GridConfig`, `RoutingPolicy`,
`RouteSolutionBuilder`, `CongestionGrid`, `Bottleneck`, `congestion_oracle`,
`extract_bottlenecks`, `build_policy`, `WidthConstraint`, `ViaTemplate`, `DiffPairConfig`,
`AccessPoint`.

## RoutingConfig

All fields have `#[serde(default)]` — an empty `{}` is valid. Designed for
deserialization from the `routing { ... }` block in `pcbdoc-spec`.

| Field | Type | Default | Notes |
|-------|------|---------|-------|
| `grid_resolution_mm` | `f64` | `0.1` | Cell size. Finer grids find tighter channels but scale quadratically in memory |
| `max_iterations` | `u32` | `50` | PathFinder iteration cap before reporting failure |
| `via_cost_base` | `f64` | `10.0` | Base A* penalty for placing a via |
| `pres_fac_multiplier` | `f64` | `1.15` | Present congestion factor growth per iteration (McMurchie & Ebeling 1995 §3.2) |
| `pres_fac_cap` | `f64` | `100.0` | Upper cap on present congestion factor |
| `history_increment` | `f64` | `1.0` | History cost accumulated per oversubscribed-node-iteration |
| `corner_style` | `CornerStyle` | `FortyFiveDegree` | `forty_five_degree` or `rounded_corner` |
| `allowed_layers` | `Vec<LayerId>` | `[]` | Empty = all copper layers allowed |
| `net_configs` | `BTreeMap<String, NetRoutingConfig>` | `{}` | Per-net-class overrides (via cost, width, layers) |
| `seed` | `u64` | `0` | RNG seed for net ordering. Sole source of non-determinism |
| `movement` | `MovementStyle` | `FourWay` | `four_way` or `eight_way` |

## Pipeline Stages

### 1. Global Routing (`global/`)

Decomposes multi-pin nets into 2-pin subnets via MST (`petgraph`). Runs negotiation-based
A* on a coarse grid (5–10× trace pitch) to plan routing regions and detect congestion.
Orders nets: critical first, short nets early, high-fanout last, `ChaCha8Rng`-seeded
tiebreaker. Optionally assigns layers via a heuristic (preferred direction) or ILP
(`good_lp` + minilp backend).

### 2. Detailed Routing (`detailed/`)

3D A* on `(x, y, layer)` nodes using the `pathfinding` crate. Movement is 4-way or
8-way (configurable). Via transitions carry net-class-sensitive costs from `ViaCostModel`.
Layer-direction bias penalty steers horizontal routes to H-preferred layers and vertical
routes to V-preferred layers. Heuristic: Manhattan distance + minimum via transitions
(admissible). Region guidance from global routing constrains the search space.

### 3. PathFinder Negotiation (`pathfinder/`)

Full-board rip-up/reroute loop implementing McMurchie & Ebeling 1995:

- Cost function: `C(n) = (b_n + h_n) × p_n` (base + history × present congestion)
- History congestion: linear accumulation per oversubscribed-node-iteration
- Present congestion factor: exponential growth (`pres_fac *= 1.15`), capped at `pres_fac_cap`
- Convergence: no oversubscribed routing resources
- Hot-set partial rip-up (100 worst nets) is available as an optimization; full rip-up is the default

History array cells are linearized as `x * (grid_height * layer_count) + y * layer_count + layer.raw() as usize`.
Grid dimensions are fixed at workspace build time.

### 4. Optimization (`optimize/`)

Post-route cleanup passes applied in order:
1. Staircase elimination — consecutive H-V bends → diagonal 45°
2. Corner conversion — apply `CornerStyle` policy
3. Rubber-banding — pull traces tight using clearance queries from the spatial index

Serpentine insertion for matched-length constraints is available via
`optimize::serpentine::insert_serpentine` and must be triggered explicitly with
per-net target-length parameters.

### 5. High-Speed Routing (`high_speed/`)

Differential-pair routing (coupled/semi-coupled), bus routing (parallel groups, crossing
minimization), and matched-length correction via serpentine/accordion insertion.

## Module Layout

| Module | Contents |
|--------|----------|
| `config` | `RoutingConfig`, `ViaCostConfig`, `NetRoutingConfig`, `CornerStyle`, `MovementStyle` |
| `workspace` | `RoutingWorkspace`, `GridConfig`, `build_workspace()` |
| `spatial` | `SpatialIndex` over `rstar::RTree<ObstacleEntry>` |
| `obstacles` | `ObstacleMap` (per-layer `BitVec`), `ObstacleEntry`, `AccessPoint` |
| `rules` | `RoutingPolicy`, `build_policy()`, `WidthConstraint`, `ViaTemplate`, `DiffPairConfig` |
| `global/` | Net decomposition (MST), coarse congestion grid, layer assignment, net ordering |
| `detailed/` | `GridRouter`, `GridNode`, `ViaCostModel`, `ShapeRouter` stub, fanout |
| `pathfinder/` | `PathFinderState`, rip-up/reroute loop, hot-set, convergence |
| `optimize/` | Staircase, corners, rubber-band, serpentine |
| `high_speed/` | Diff-pair, bus routing |
| `drc` | Post-route DRC checks |
| `solution` | `RouteSolutionBuilder` |
| `pipeline` | End-to-end orchestration |
| `coopt` | Placement-router co-optimization hooks |

## Co-Optimization Hooks (`coopt`)

Two interfaces connect the router with the placement simulated annealing:

**Forward hook — `congestion_oracle(ir, config) -> Result<CongestionGrid>`**

Estimates routing congestion from net bounding boxes without running the router.
Complexity: O(nets × bbox_cells), typically < 10 ms. Deterministic (no RNG).
The placement SA can call this during its cost function to penalize congested layouts.

**Backward hook — `extract_bottlenecks(solution, ir, config) -> Result<Vec<Bottleneck>>`**

Post-processes a `RouteSolution` to identify persistently oversubscribed coarse grid
cells and maps each back to the `ComponentId`s of nearby components. Output is sorted
by severity (highest first). The placement engine uses this to decide which components
to move after a failed routing pass.

## Invariants

- Router never modifies `PcbIr` — it is a read-only input
- `RouteSolution` is self-contained — the viewer loads it without `PcbIr`
- All coordinates in `RouteSolution` are mm (`f64`), matching `PcbIr` convention
- `RouteSolution` uses `BTreeMap` (not `HashMap`) — same input produces byte-identical binary output
- Net ordering uses `ChaCha8Rng` (platform-stable, version-stable) seeded from `RoutingConfig.seed`; changing the algorithm is a breaking change
- `LayerId(u16)` conversion from `autopcb-ir`'s `LayerId(u32)` is `id.raw() as u16` with a `debug_assert!(id.raw() <= u16::MAX as u32)` guard at the `route_board()` boundary
- `NetId(u32)` conversion from `autopcb-ir`'s `NetId` is `routes::NetId(ir_net_id.raw())` at the same boundary
- Unsupported design rule kinds return `RoutingError::UnsupportedRule { kind }` — never silently skipped

## Tradeoffs

| Decision | Tradeoff |
|----------|----------|
| Grid resolution 0.1mm default | Finer grids (0.05mm) find tighter channels but use quadratically more memory |
| Full rip-up default | Simpler and correct; hot-set partial rip-up (100 worst nets) available as optimization |
| PathFinder over greedy sequential | Greedy produces order-dependent results; PathFinder negotiation allows all nets to compete at the cost of multiple iterations |
| Separate `.routes` file | Route geometry is too complex for the spec language; binary file is compact and fast; JSON variant enables debugging; cost is one extra file |
| MST for net decomposition | Simple; optimal for 2-pin and 3-pin nets (vast majority); `NetDecomposer` trait interface allows FLUTE substitution for high-fanout |
| `good_lp` + minilp backend | Pure-Rust LP solver, no native C dependency, builds in CI without headers |
| `ChaCha8Rng` | `SmallRng`/`StdRng` are not stable across versions or platforms; `ChaCha8Rng` is both |

## Dependencies

| Crate | Role |
|-------|------|
| `autopcb-ir` | Board IR types (geometry, nets, pads, rules) — read-only input |
| `autopcb-routes` | Route solution types — output format |
| `petgraph` | MST for net decomposition |
| `pathfinding` | Callback-based A* for detailed routing |
| `rstar` | R-tree spatial index for clearance queries |
| `geo` | Geometric predicates (polygon intersection, containment) for keepout checks |
| `bitvec` | Per-layer obstacle bitmaps |
| `rayon` | Parallel net routing where applicable |
| `good_lp` | ILP layer assignment (minilp pure-Rust backend) |
| `rand` + `rand_chacha` | Seeded net ordering via `ChaCha8Rng` |
| `ordered-float` | `OrderedFloat` for priority queue ordering |
| `tracing` | Structured logging of routing progress |
