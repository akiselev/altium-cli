# AutoPCB Router: First-Pass Full-Stack Implementation Plan

## Overview

This plan defines the first real autorouter milestone set for the codebase.
It is intentionally **not** a toy single-net MVP. The target is a first pass of
the full routing stack described in the research notes:

- preprocessing and routing data extraction
- global routing
- detailed routing
- PathFinder rip-up/reroute
- trace optimization
- differential pairs and buses
- route-aware DRC
- PcbDoc write-back
- CLI and viewer integration
- placement-router feedback hooks

The current codebase already has:

- `autopcb-ir` extraction and viewer support for existing copper
- placement solver plumbing
- routing rule parsing/serialization groundwork in `altium-format`
- shell/viewer affordances for future routing

What is missing is the actual routing engine.


## Architecture Decision

### Canonical IR: `autopcb-ir`

The router should **not** introduce a second canonical board IR.

`autopcb-ir` remains the source-of-truth semantic board model for:

- board geometry
- components and pads
- nets
- layer stack
- design rules
- existing/free copper
- polygons and keepouts

Router work should extend `autopcb-ir` where the shared domain model is missing
data needed by routing, DRC, or visualization.

### Router Working State: derived, transient, optimization-oriented

The router **does** need its own runtime/search data structures, but these are
derived working sets, not a replacement IR:

- `RoutingWorkspace`
- `RoutingGrid`
- `GlobalRoutingGraph`
- `ObstacleMap`
- `PathFinderState`
- `RouteSolution`
- `SpatialIndex`
- congestion/history/present-cost arrays
- shape-routing expansion rooms / visibility graph

These live in `autopcb-router` and are rebuilt from `PcbIr` plus router config.

### Why this split

| Decision | Reasoning |
|----------|-----------|
| Extend `autopcb-ir`, do not fork it | Components, pads, nets, rules, and board geometry are shared concerns for placement, routing, DRC, CLI inspect, and viewer → duplicating them creates drift and duplicated extraction logic |
| Router gets derived runtime state | A* and PathFinder want cache-friendly grids, occupancy maps, bitmaps, and hot-path arrays → these are optimization structures, not domain truth |
| Keep extraction read-only | Matches existing IR design: `PcbDoc`/`PcbDocBoard` → `PcbIr` → derived solver/router states |
| Keep write-back outside `autopcb-ir` | `autopcb-ir` is a semantic view; route emission back to `.PcbDoc` belongs in bridge/writeback code |


## Planning Context

### Research Basis

This plan is based on the full routing corpus, especially:

- `docs/future/solverang/autorouter.md`
- `docs/future/solverang/implementation-plan.md`
- `docs/future/solverang/routability-metrics-spec.md`
- `docs/future/solverang/ir.md`
- `docs/future/solverang/design-rules-mapping.md`
- `docs/future/solverang/spec-grammar.md`
- `docs/future/solverang/placement-algorithms.md`
- `docs/routing/routing-serialization.md`
- `docs/routing/routing-data-model.md`
- `docs/routing/active-router.md`
- `docs/routing/push-pull-router.md`

### Scope Boundary

Included in this first pass:

- batch autorouting on existing placed boards
- single-net and full-board route flows
- grid routing and first shape-based escape/fanout backend
- route-critical design rule enforcement
- write-back to board primitives
- viewer playback and congestion overlays
- placement feedback hooks

Explicitly deferred:

- GPU execution as a delivery requirement
- exact cloning of Altium's interactive Delphi router
- deep learning / RL / diffusion routing
- full interactive push-and-shove parity


## Crate Layout

### New crate

- `crates/autopcb-router`

### Existing crates touched

- `crates/autopcb-ir`
- `crates/altium-format`
- `crates/altium-cli`
- `crates/autopcb-viewer`
- `crates/autopcb-shell`
- `crates/autopcb-placement`

### Proposed module layout in `autopcb-router`

```text
autopcb-router/
  src/
    lib.rs
    config.rs
    workspace.rs
    routing_ir.rs
    rules.rs
    spatial.rs
    obstacles.rs
    global/
      mod.rs
      steiner.rs
      congestion.rs
      layer_assignment.rs
      ordering.rs
    detailed/
      mod.rs
      grid.rs
      astar.rs
      via_cost.rs
      shape.rs
      fanout.rs
    pathfinder/
      mod.rs
      history.rs
      ripup.rs
      hot_set.rs
    optimize/
      mod.rs
      staircase.rs
      corners.rs
      rubber_band.rs
      serpentine.rs
    high_speed/
      mod.rs
      diff_pair.rs
      bus.rs
    drc.rs
    solution.rs
    writeback.rs
    pipeline.rs
    coopt.rs
```


## Parallel Delivery Model

Approach: **Parallel Waves** with disjoint ownership where possible. Agent work
should be split by write scope, not by vague feature theme.

### Wave A: shared foundations

Strict dependency wave:

1. `autopcb-ir` routing extensions
2. router crate scaffold
3. rule/options bridge

### Wave B: independent routing engines

Can proceed in parallel after Wave A:

- global routing
- detailed/grid routing
- spatial index and obstacle maps
- CLI command skeletons

### Wave C: negotiation and optimization

Depends on Wave B outputs:

- PathFinder
- trace optimization
- diff-pair / bus routing

### Wave D: integration and feedback

Depends on Wave C:

- DRC
- write-back
- viewer integration
- placement feedback hooks


## Milestones

### Milestone 0: Architecture Lock + Crate Scaffold

**Files**:

- `Cargo.toml`
- `crates/autopcb-router/Cargo.toml`
- `crates/autopcb-router/src/lib.rs`
- `docs/plans/autopcb-router.md`

**Requirements**:

- Add `autopcb-router` workspace crate
- Declare dependency set: `autopcb-ir`, `altium-format`, `petgraph`,
  `pathfinding`, `rstar`, `geo`, `bitvec`, `rayon`, `good_lp`
- Define public top-level APIs:
  - `build_workspace(ir, config) -> RoutingWorkspace`
  - `route_single_net(...)`
  - `route_board(...)`
  - `optimize_solution(...)`
  - `verify_solution(...)`
  - `write_solution_to_board(...)`
- Freeze the architectural split:
  - canonical IR in `autopcb-ir`
  - transient router state in `autopcb-router`

**Acceptance Criteria**:

- `cargo check -p autopcb-router` passes
- no duplicate board IR types are introduced in `autopcb-router`
- crate docs state derived-state-only policy explicitly

**Ownership**:

- One agent, because this defines public interfaces others will consume


### Milestone 1: `autopcb-ir` Routing Extensions

**Files**:

- `crates/autopcb-ir/src/extract.rs`
- `crates/autopcb-ir/src/copper.rs`
- `crates/autopcb-ir/src/component.rs`
- `crates/autopcb-ir/src/rule.rs`
- `crates/autopcb-ir/src/layer_stack.rs`
- new files as needed for pad detail / routing metadata

**Requirements**:

- Extend `PcbIr` to carry the route-critical data assumed by the research:
  - pre-routed / locked / user-routed flags on copper primitives
  - pad layer detail and per-layer pad existence
  - via layer span (`from_layer`, `to_layer`) in board IR, not just library IR
  - drill pair / layer-pair information
  - net classes and diff-pair memberships where available
  - routing-priority / topology / layer-permission / via-style rule extraction
  - layer preferred-direction metadata
  - board obstacles needed for routing, not just for viewing
- Add `IrPadDetail` or equivalent extension path for DRC/router geometry
- Preserve existing simple viewer/inspect use cases

**Acceptance Criteria**:

- `PcbIr::extract()` exposes enough information to build:
  - an obstacle map
  - per-net route policies
  - legal via transitions
- unit tests cover:
  - locked/pre-route/user-routed primitive extraction
  - routing layer rules
  - via-style rule extraction
  - diff-pair routing rule extraction

**Tests**:

- `cargo test -p autopcb-ir`
- extraction tests from synthetic boards and at least one real fixture

**Ownership**:

- Agent 1 owns `autopcb-ir` changes exclusively


### Milestone 2: Routing Rules + Options Bridge

**Files**:

- `crates/autopcb-router/src/rules.rs`
- `crates/autopcb-router/src/config.rs`
- `crates/altium-format/...` only if missing parser support is required

**Requirements**:

- Convert `autopcb-ir` design rules plus board router options into router-native policy:
  - clearance
  - width min/max/preferred
  - routing layers
  - topology
  - priority
  - via style / templates
  - corner style
  - neck-down
  - diff-pair width/gap/skew
  - matched lengths
  - fanout control
- Support per-net-class and per-layer resolution
- Define precedence rules when multiple Altium rules match
- Make unsupported rule kinds explicit, not silent

**Acceptance Criteria**:

- Router policy can be queried as:
  - `policy.for_net(net_id)`
  - `policy.allowed_layers(net_id)`
  - `policy.via_candidates(net_id, from, to)`
  - `policy.trace_width(net_id, layer)`
- unresolved / ambiguous rule cases fail loudly

**Tests**:

- synthetic rule-resolution tests
- regression tests for width/layer/via/diff-pair precedence

**Ownership**:

- Agent 2 owns `autopcb-router/src/rules.rs`
- must not edit `autopcb-ir`


### Milestone 3: Routing Workspace + Spatial/Obstacle Foundations

**Files**:

- `crates/autopcb-router/src/workspace.rs`
- `crates/autopcb-router/src/spatial.rs`
- `crates/autopcb-router/src/obstacles.rs`
- `crates/autopcb-router/src/routing_ir.rs`

**Requirements**:

- Build derived runtime state from `PcbIr`:
  - R-tree over fixed obstacles
  - per-layer obstacle bitmaps
  - clearance-inflated occupancy queries
  - route pin access points
  - pre-routed segment reservation
  - legal via stack transitions
- Distinguish:
  - fixed obstacles
  - pushable future category
  - already-routed solution occupancy
  - pre-route / must-preserve geometry
- Support both coarse and fine grids

**Acceptance Criteria**:

- Workspace can answer:
  - `is_blocked(layer, x, y, net)`
  - `clearance_query(segment)`
  - `legal_via_transitions(net, layer)`
  - `pin_accesses(pin_id)`
- obstacle maps are reproducible and deterministic

**Tests**:

- obstacle inflation tests
- keepout / board-edge clipping tests
- pre-routed trace reservation tests

**Ownership**:

- Agent 3 owns `workspace.rs`, `spatial.rs`, `obstacles.rs`


### Milestone 4: Net Decomposition + Global Routing

**Files**:

- `crates/autopcb-router/src/global/steiner.rs`
- `crates/autopcb-router/src/global/congestion.rs`
- `crates/autopcb-router/src/global/layer_assignment.rs`
- `crates/autopcb-router/src/global/ordering.rs`
- `crates/autopcb-router/src/global/mod.rs`

**Requirements**:

- Implement net decomposition abstraction with:
  - MST backend immediately
  - FLUTE backend integrated behind same trait/interface
- Implement coarse routing grid with demand/capacity cells
- Implement congestion-aware global routing:
  - region paths for 2-pin subnets
  - seeded ordering heuristic
  - present congestion penalty
- Implement layer assignment:
  - heuristic path first
  - `good_lp` backend for constrained cases
- Produce region guidance for detailed router

**Acceptance Criteria**:

- Global router produces stable subnet plans for full boards
- congestion reports identify oversubscribed cells
- net ordering prioritizes critical nets, short nets, and defers high-fanout nets

**Tests**:

- MST and FLUTE decomposition tests
- congestion capacity tests
- deterministic ordering tests
- global-routing snapshot tests on small synthetic boards

**Ownership**:

- Agent 4 owns `src/global/**`


### Milestone 5: Detailed Routing Backend

**Files**:

- `crates/autopcb-router/src/detailed/grid.rs`
- `crates/autopcb-router/src/detailed/astar.rs`
- `crates/autopcb-router/src/detailed/via_cost.rs`
- `crates/autopcb-router/src/detailed/shape.rs`
- `crates/autopcb-router/src/detailed/fanout.rs`

**Requirements**:

- Implement 3D A* detailed router:
  - `(x, y, layer)` node space
  - 4-way and 8-way movement as configured
  - via transitions with net-class-sensitive costs
  - layer-direction bias
  - admissible heuristic
  - region-guided routing from global stage
- Implement first shape-based backend for:
  - surface escape
  - BGA / fine-pitch fanout
  - tight channels where regular grid is too lossy
- Implement fanout routing hooks from `FanoutControl`

**Acceptance Criteria**:

- `route_single_net` produces valid segments/vias for representative cases:
  - same-layer route
  - multi-layer route with via
  - route around keepout
  - preserve pre-route
- shape backend can route at least simple fanout/escape cases that grid backend struggles with

**Tests**:

- unit tests for heuristic admissibility
- same-layer and multi-layer path tests
- via-cost class tests
- fanout tests

**Ownership**:

- Agent 5 owns `src/detailed/**`


### Milestone 6: PathFinder Negotiation + Rip-Up/Reroute

**Files**:

- `crates/autopcb-router/src/pathfinder/mod.rs`
- `crates/autopcb-router/src/pathfinder/history.rs`
- `crates/autopcb-router/src/pathfinder/ripup.rs`
- `crates/autopcb-router/src/pathfinder/hot_set.rs`
- `crates/autopcb-router/src/solution.rs`

**Requirements**:

- Implement full-board routing loop with:
  - history congestion
  - present congestion factor
  - full rip-up
  - hot-set partial rip-up
  - convergence detection
  - unrouted/failed-net reporting
- Support deterministic replay and iteration snapshots
- Define persistent or rollback-capable state boundary for:
  - iteration checkpoints
  - speculative alternative orderings

**Acceptance Criteria**:

- `route_board` completes on small/medium synthetic boards with fewer conflicts each iteration
- PathFinder loop converges or exits with explicit bottleneck data
- iteration outputs are consumable by viewer/CLI

**Tests**:

- PathFinder cost-update tests
- hot-set selection tests
- convergence tests
- regression tests for no oscillation on known scenarios

**Ownership**:

- Agent 6 owns `src/pathfinder/**` and `solution.rs`


### Milestone 7: Trace Optimization + High-Speed Routing

**Files**:

- `crates/autopcb-router/src/optimize/staircase.rs`
- `crates/autopcb-router/src/optimize/corners.rs`
- `crates/autopcb-router/src/optimize/rubber_band.rs`
- `crates/autopcb-router/src/optimize/serpentine.rs`
- `crates/autopcb-router/src/high_speed/diff_pair.rs`
- `crates/autopcb-router/src/high_speed/bus.rs`

**Requirements**:

- Implement post-route cleanup:
  - staircase elimination
  - 45-degree conversion
  - corner-style aware cleanup
  - Solverang rubber-banding constrained by clearance queries
- Implement differential-pair routing first pass:
  - coupled or semi-coupled routing mode
  - width/gap enforcement
  - uncoupled length/skew checks
- Implement bus routing first pass:
  - member ordering
  - channel routing
  - spacing preservation
- Implement matched-length correction:
  - serpentine/accordion insertion
  - per-group target selection

**Acceptance Criteria**:

- optimized solution reduces bend count or total jog count without introducing violations
- diff-pair routes satisfy gap/skew policy on synthetic tests
- bus routes maintain member order through a constrained channel

**Tests**:

- staircase-to-diagonal replacement tests
- rubber-band no-clearance-violation tests
- diff-pair skew tests
- bus ordering tests
- serpentine target-length tests

**Ownership**:

- Agent 7 owns `src/optimize/**`
- Agent 8 owns `src/high_speed/**`


### Milestone 8: DRC + Write-Back + CLI

**Files**:

- `crates/autopcb-router/src/drc.rs`
- `crates/autopcb-router/src/writeback.rs`
- `crates/autopcb-router/src/pipeline.rs`
- `crates/altium-cli/src/main.rs`
- new `crates/altium-cli/src/commands/route.rs` if desired

**Requirements**:

- Implement route-critical DRC for the routed solution:
  - clearance
  - width
  - via style / hole bounds
  - layer restrictions
  - corner style
  - length / matched length
  - diff-pair routing
- Emit routed solution back to `.PcbDoc`:
  - tracks
  - vias
  - route flags (`UserRouted`, pre-route preservation where appropriate)
- Add CLI surfaces:
  - `altium route <pcbdoc> --net <name>`
  - `altium route <pcbdoc>`
  - `altium route <pcbdoc> --drc`

**Acceptance Criteria**:

- routed board can be written and reopened through existing parser
- CLI prints completion stats, unrouted nets, via count, total length, DRC summary
- route write-back does not clobber unrelated board state

**Tests**:

- roundtrip write-back tests
- DRC rule tests against routed solutions
- CLI smoke tests

**Ownership**:

- Agent 9 owns `drc.rs` and `writeback.rs`
- Agent 10 owns CLI wiring


### Milestone 9: Viewer + Shell Integration

**Files**:

- `crates/autopcb-viewer/src/app.rs`
- `crates/autopcb-viewer/src/renderer.rs`
- `crates/autopcb-viewer/src/view3d.rs`
- `crates/autopcb-shell/src/...` as needed

**Requirements**:

- viewer support for:
  - route solution overlays
  - via markers
  - global congestion heatmap
  - PathFinder iteration playback
  - diff-pair / bus overlays
  - before/after optimization toggle
- shell command plumbing for route execution jobs
- job reporting and playback loading

**Acceptance Criteria**:

- viewer can load a route result or iteration stream and display path evolution
- shell route commands run through job system surfaces rather than ad hoc mutation

**Tests**:

- focused renderer tests where possible
- shell command intent/registry tests

**Ownership**:

- Agent 11 owns `autopcb-viewer`
- Agent 12 owns `autopcb-shell`


### Milestone 10: Placement Feedback Hooks

**Files**:

- `crates/autopcb-router/src/coopt.rs`
- `crates/autopcb-placement/src/...` as needed

**Requirements**:

- Implement forward hook:
  - congestion oracle from router/global planner into placement SA cost
- Implement backward hook:
  - bottleneck extraction from failed PathFinder runs
  - placement nudge request generation
- Expose a stable interface even if the full outer loop ships behind a flag

**Acceptance Criteria**:

- placement code can request a congestion estimate without invoking full routing
- router can emit actionable bottleneck data tied back to blocking components

**Tests**:

- congestion oracle deterministic tests
- bottleneck-to-component attribution tests

**Ownership**:

- Agent 13 owns `coopt.rs`
- any placement edits must be coordinated explicitly with placement owners


## Testing Gates

Minimum gating per merge wave:

- `cargo test -p autopcb-ir`
- `cargo test -p autopcb-router`
- `cargo test -p altium-cli`
- targeted viewer/shell tests where routing plumbing changed

Additional acceptance artifacts:

- synthetic routing fixtures for:
  - same-layer single net
  - forced via
  - keepout detour
  - dense channel congestion
  - diff-pair escape
  - simple bus channel
- at least one end-to-end board fixture producing a writable routed result


## Rejected Alternatives

| Alternative | Why Rejected |
|-------------|-------------|
| Router-owned canonical board IR | Duplicates `autopcb-ir`, creates extraction drift, and splits truth across crates |
| Pure single-net MVP | Contradicts the research stack; would throw away core design work around global routing, PathFinder, optimization, and high-speed routing |
| Grid-only first pass | Research explicitly calls out shape-based advantages for fanout/surface routing; first pass should include at least a bounded escape backend |
| DRC afterthought | Width/clearance/layer/via constraints drive route legality during search; verification cannot be bolted on at the end |
| Interactive-router parity target | The actual Altium algorithms live in Delphi; cloning UI behavior is a distraction from delivering a real batch router |
| GPU in critical path | CPU-first is required by the research itself; GPU should be an acceleration layer, not a blocker |


## Immediate Next Step

Start with Milestones 0-3 in sequence:

1. scaffold `autopcb-router`
2. extend `autopcb-ir`
3. build rule/options bridge
4. build routing workspace and obstacle maps

That yields the minimum stable substrate for parallel agent work on global,
detailed, and PathFinder routing without rework.
