# Active Routing and Push-Pull on GPU: Reusing Batch Router Infrastructure for Interactive Routing

## Overview

This document analyzes how our GPU-accelerated autorouter infrastructure (wgpu
Bellman-Ford SSSP, GAMER H/V sweep, PathFinder negotiation, X-Check DRC) can be
reused to implement Altium-style interactive routing modes: Active Router obstacle
avoidance, push/pull trace displacement, gloss smoothing, diff-pair interactive
routing, bus routing, length tuning, and trace sliding.

The central thesis is that the GPU batch router and the interactive router share
the same core primitives (obstacle maps, shortest-path solvers, DRC checkers) but
differ in their outer control loops and latency requirements. By factoring these
primitives into a shared `GpuRoutingEngine`, both batch and interactive routing can
be served from the same GPU infrastructure with different configuration profiles.

**Source material**: Altium reverse engineering from `docs/routing/active-router.md`,
`docs/routing/push-pull-router.md`, `docs/routing/delphi-routing-engine.md`,
`docs/routing/delphi-routing-engine-deep.md`, and `docs/routing/routing-data-model.md`.
GPU infrastructure from `docs/plans/router-gpu/01-04` and
`docs/notes/autorouter-gpu/00-06`.

---

## 1. Active Routing Modes and GPU Mapping

Altium's `TAdvancedRouteMode` enum defines seven conflict resolution modes for
interactive routing. Each mode has a direct mapping to our GPU infrastructure:

### 1.1 `eARIgnoreObstacle` (mode 0) -- Route Through Everything

**Altium behavior**: Routes through all obstacles, creating DRC violations. No
obstacle avoidance at all.

**GPU mapping**: Trivially supported. Run Bellman-Ford (or GAMER sweep) with the
obstacle bitmap cleared (all cells passable). The result is a shortest-path route
that ignores all existing copper. DRC violations are expected and reported.

**Shared infrastructure**: Distance/predecessor buffers, cost encoding, path
reconstruction. The obstacle buffer is simply zeroed.

**Latency**: Single BF/GAMER pass, no iterative negotiation. Sub-5ms for typical
interactive routing subgraphs (< 100x100 cells). Well within interactive budget.

### 1.2 `eARWalkAroundObstacle` (mode 1) -- Navigate Around Obstacles

**Altium behavior**: Routes around fixed obstacles without disturbing them. This
is Altium's core walk-around algorithm, implemented in Delphi as a visibility
graph Dijkstra (see `delphi-routing-engine-deep.md`, `TPolygonPathFinder`).

**GPU mapping**: This IS our Bellman-Ford / GAMER sweep SSSP. The obstacle bitmap
marks all existing copper (pads, tracks, vias, keepouts) as blocked. The GPU
solver finds the shortest legal path around obstacles. This is the primary use
case for the batch autorouter and directly reusable for interactive routing.

**Shared infrastructure**: Full obstacle map, distance/predecessor arrays, BF/GAMER
shaders, path reconstruction. History costs are NOT needed (single-pass, no
negotiation).

**Comparison with Altium's approach**: Altium uses a visibility graph with Dijkstra
(O(V log V) where V = contour vertices). Our grid-based GPU BF operates on a
regular grid (O(W*H) cells but massively parallel). For interactive routing on
a local subgraph (Corolla-style bounding box extraction), the GPU approach is
competitive: a 200x200 subgraph (40K cells) converges in 2-5 BF iterations on
GPU, each taking < 0.5ms, for a total of < 2.5ms. Altium's Dijkstra on a
visibility graph with ~100-500 contour vertices runs in < 1ms on CPU. The GPU
approach has higher constant overhead but scales better for dense obstacle fields.

**Latency**: 1-5ms per route computation. Acceptable for interactive routing
(target: < 50ms per mouse-move update; Altium's `UpdateObject` runs on every
mouse move).

### 1.3 `eARPushObstacle` (mode 2) -- Push Other Traces

**Altium behavior**: Displaces existing tracks to make room for the new route.
The Delphi `Pusher` module uses an 8-direction push system with constraint
analysis (`TIntervalRec`, `TConstrain`). Key predicate: `IsPushablePrimitive`.
Push operates by: (1) identifying conflicting primitives, (2) computing valid
displacement directions, (3) generating new track segments for displaced traces,
(4) DRC-validating the result.

**GPU mapping**: Push routing can be modeled as a **mini-PathFinder iteration**
on a subset of nets:

1. Route the new trace with obstacles (walk-around), ignoring pushable traces
   (remove them from obstacle map).
2. Identify the set of conflicting nets whose existing traces overlap with the
   new route's clearance envelope.
3. Rip up conflicting segments and reroute them with the new trace treated as
   a fixed obstacle.
4. Repeat until no conflicts remain or a maximum depth is reached.

This is exactly our PathFinder rip-up/reroute loop, but limited to a small set
of affected nets (typically 2-10) and a local subgraph (the bounding box of the
new trace plus its immediate neighbors).

**GPU dispatch sequence** for push routing:
```
1. Clear obstacle map for pushable traces (GPU: bitwise AND with pushable mask)
2. Route new trace via GPU BF/GAMER (single net, local subgraph)
3. Upload new trace as obstacle
4. For each conflicting net (CPU determines which):
   a. Rip up conflicting segments (update obstacle map)
   b. Route conflict net via GPU BF/GAMER (local subgraph)
   c. Upload rerouted trace as obstacle
5. GPU DRC pass on local region to validate
```

**Key difference from batch PathFinder**: No history costs, no convergence
negotiation across all nets. Push routing is a greedy local operation: displace
the minimum number of traces to accommodate the new route. PathFinder's history
mechanism is unnecessary because we are not balancing global congestion.

**Latency**: Each BF pass is 1-3ms. With 2-5 conflicting nets, total push
resolution takes 5-20ms. Within interactive budget if conflicting net count is
bounded. Altium's push algorithm (Delphi `FPusher`) typically resolves in < 10ms
on CPU for moderate-density boards.

**Shared infrastructure**: Obstacle maps, BF/GAMER shaders, DRC shaders. The
`IsPushablePrimitive` predicate maps to a per-cell bitmask in the obstacle buffer
(bit 0 = blocked, bit 1 = pushable, etc.).

### 1.4 `eARHugAndPushObstacle` (mode 3) -- Hug Tightly Then Push

**Altium behavior**: First attempts walk-around with tight hugging
(`THuggingStyle`), then falls back to push if the walk-around path exceeds a
cost threshold. The Delphi engine temporarily switches to walk-around mode for
the gloss pass, then to push if needed (see `delphi-routing-engine-deep.md`,
`CheckGlossMode`).

**GPU mapping**: Composition of modes 1 and 2:

1. Run walk-around (mode 1) with a tightness bias. On GPU, this means adding
   a **proximity cost** to cells near obstacles: cells adjacent to obstacles get
   a small bonus cost reduction (encouraging the path to hug obstacle contours).
   This is implemented by modifying the cost function in the BF shader:
   ```wgsl
   let proximity_bonus = select(0u, HUGGING_BONUS, is_adjacent_to_obstacle(x, y, layer));
   let edge_cost = base_cost - proximity_bonus + history_cost;
   ```
2. If the walk-around path exceeds a length threshold (relative to the direct
   distance), fall back to push (mode 2).

**Shared infrastructure**: Same as modes 1 + 2, plus a proximity cost texture
(precomputed on GPU via distance transform from obstacle bitmap).

**Latency**: 2-25ms (walk-around attempt + optional push fallback).

### 1.5 `eARStopAtFirstObstacle` (mode 4) -- Stop at Collision

**Altium behavior**: Routes until hitting an obstacle, then stops. The route
terminates at the collision point. Used when the user wants to manually handle
obstacle conflicts.

**GPU mapping**: Bellman-Ford with early termination. After each BF iteration,
check if the wavefront has reached any obstacle cell. If so, terminate and
return the path from source to the last reachable cell before the obstacle.

Implementation: After each BF dispatch, read the distance at the target cell.
If the target is unreachable (distance = INFINITY) but the wavefront has
propagated to cells adjacent to the obstacle, reconstruct the path to the
nearest-to-target reachable cell.

Alternatively, run BF to completion but with obstacles present. If the target
is unreachable, trace back from the cell nearest to the target (in Euclidean
distance) that has a finite distance value.

**Shared infrastructure**: Standard BF/GAMER with obstacle map. Path
reconstruction modified to handle partial paths.

**Latency**: 1-3ms (same as walk-around, possibly faster due to smaller
wavefront).

### 1.6 `eARAutoRouteCurrentLayer` (mode 5) -- Single-Layer Autoroute

**Altium behavior**: Hands off the current segment to the autorouter, restricted
to the current layer. No via insertion.

**GPU mapping**: Standard GPU BF/GAMER on a single-layer subgraph. The via
transition shader is simply not dispatched, or the via cost is set to INFINITY.
History costs CAN be used here (this is a mini batch-route), but for interactive
responsiveness, a single-pass BF without history is preferred.

**Shared infrastructure**: Everything from the batch router, but constrained to
one layer.

**Latency**: 2-10ms depending on subgraph size. Acceptable.

### 1.7 `eARAutoRouteMultiLayer` (mode 6) -- Multi-Layer Autoroute

**Altium behavior**: Full autoroute with layer changes (vias). This is
essentially a single-net batch route with all layers available.

**GPU mapping**: Standard GPU BF/GAMER with via transitions enabled. This is
identical to `route_single_net()` in our existing router. The full subgraph
extraction (Corolla bounding box) and multi-layer BF apply directly.

**Shared infrastructure**: Complete batch router infrastructure.

**Latency**: 5-30ms depending on board complexity and number of layers. At the
upper end of interactive budget but acceptable for an "auto complete" operation
(user clicks and waits for the route to resolve).

### 1.8 Summary Table

| Mode | GPU Backend | Obstacle Map | History | BF Iterations | Expected Latency |
|------|------------|-------------|---------|---------------|-----------------|
| IgnoreObstacle | BF/GAMER | Cleared | None | 1-3 | < 2ms |
| WalkAround | BF/GAMER | Full | None | 3-10 | 2-5ms |
| Push | BF/GAMER x N | Modified per conflict | None | 3-10 x N | 5-20ms |
| HugAndPush | BF/GAMER + push fallback | Full + proximity | None | 3-10 + push | 2-25ms |
| StopAtFirst | BF/GAMER | Full | None | 1-5 | 1-3ms |
| AutoRouteCurrent | BF/GAMER (1 layer) | Full | Optional | 3-10 | 2-10ms |
| AutoRouteMulti | BF/GAMER (all layers) | Full | Optional | 5-20 | 5-30ms |

---

## 2. Push Routing on GPU

### 2.1 Algorithm

Push routing is a constrained local re-routing problem. When a new trace
conflicts with existing traces, the conflicting traces are displaced ("pushed")
to make room. The algorithm:

```
PushRoute(new_trace, workspace):
  1. Route new_trace ignoring pushable obstacles
     (set pushable cells to passable in obstacle map)
  2. Identify conflicting traces:
     For each cell occupied by new_trace's clearance envelope:
       If cell is occupied by a different-net trace T:
         Add T to conflict_set
  3. For each trace T in conflict_set:
     a. Remove T's cells from obstacle map (rip up T)
     b. Add new_trace's cells to obstacle map
     c. Reroute T via GPU BF around the new obstacle configuration
     d. If reroute succeeds:
        Update T's geometry
     e. If reroute fails:
        Restore T, report failure
  4. Run local DRC to validate no remaining violations
  5. Optionally: gloss (smooth) pushed traces
```

### 2.2 Altium's Push Direction System

Altium's Delphi `Pusher` module uses an 8-direction push system with priority
ordering (see `delphi-routing-engine-deep.md`). This is a heuristic for fast
CPU-based displacement: try to push the conflicting trace in the direction
opposite to the incoming route, falling back to adjacent directions.

Our GPU approach does not need this heuristic. Instead of computing a push
direction and generating displaced track geometry directly, we simply rip up
the conflicting trace and reroute it with the new obstacle configuration. The
GPU BF finds the optimal displaced path without needing to enumerate push
directions. This is more expensive per-trace (a full BF solve vs. a geometric
displacement) but produces better results (globally optimal path) and is
simpler to implement.

### 2.3 Real-Time Push on GPU: Can We Hit < 50ms?

Interactive push routing requires resolving all conflicts within a single
mouse-move update cycle (< 50ms target). Budget analysis:

| Step | Time | Notes |
|------|------|-------|
| Identify conflicts (CPU) | < 1ms | R-tree query on new trace bbox |
| Update obstacle map (GPU) | < 0.5ms | Bitwise ops on bitmap buffer |
| Route new trace (GPU BF) | 1-3ms | Local subgraph |
| Per-conflict reroute (GPU BF) | 1-3ms each | Local subgraph per trace |
| DRC validation (GPU) | 1-2ms | Local region |
| Path reconstruction (CPU) | < 0.5ms | |
| **Total (3 conflicts)** | **~10-15ms** | **Within budget** |
| **Total (10 conflicts)** | **~20-35ms** | **Within budget** |
| **Total (20+ conflicts)** | **~40-70ms** | **Marginal** |

For typical interactive routing (2-5 conflicting traces), GPU push routing
is well within the 50ms budget. For dense areas with many conflicts (> 15
traces), latency may exceed the budget. Mitigation strategies:

1. **Bounded push depth**: Limit the number of push iterations (Altium allows
   1-2 levels of recursive push). Set `max_push_depth = 2`.
2. **Incremental push**: Only reroute traces that are actually violated by the
   new route, not all traces in the bounding box. The conflict set is typically
   small.
3. **Persistent obstacle map**: Keep the obstacle map on GPU between mouse
   moves. Only update the cells that changed (incremental bitmap update).
4. **Async push**: Route the new trace synchronously (< 5ms). Queue
   conflicting trace reroutes asynchronously. Display the new trace immediately;
   pushed traces appear in the next frame.

### 2.4 GPU Buffer Extensions for Push

The push algorithm requires tracking which obstacle cells are "pushable"
(belong to traces that can be displaced) vs. "fixed" (pads, keepouts, board
edges). Extend the obstacle buffer:

```wgsl
// Current: single u32 per 2D cell, one bit per layer
//   bit i = blocked on layer i
// Extended: two u32 per 2D cell
//   obstacle_fixed[cell]: bit i = fixed obstacle on layer i (never pushed)
//   obstacle_trace[cell]: bit i = pushable trace on layer i
// Combined obstacle: obstacle_fixed | obstacle_trace (both block routing)
// Push mode: obstacle_fixed only (pushable traces removed from obstacle map)
```

This doubles the obstacle buffer size (from 4 MB to 8 MB for a 2000x2000
grid) but is negligible relative to the distance/predecessor arrays.

---

## 3. Interactive Routing Latency Requirements

### 3.1 Target Latency Breakdown

Altium's `UpdateObject` method (the core interactive routing update) is called
on every mouse move during routing. The Delphi code at `0x03901C60` calls the
core routing dispatch `FUN_039009F0`, which computes the entire route from the
last committed point to the current mouse position.

For interactive feel, the route must update within one display frame:
- **60 FPS**: 16.7ms per frame
- **30 FPS**: 33.3ms per frame
- **20 FPS** (acceptable minimum): 50ms per frame

Our target: **< 16ms** for walk-around mode, **< 33ms** for push mode.

### 3.2 GPU BF Latency for Single-Net Interactive Routing

For a single net routed on a local subgraph:

| Grid subgraph size | BF iterations | GPU time | Notes |
|-------------------|---------------|----------|-------|
| 50 x 50 (2,500 cells) | 3-5 | 0.5-1ms | Very short net |
| 100 x 100 (10K cells) | 5-10 | 1-2ms | Typical short net |
| 200 x 200 (40K cells) | 8-15 | 2-5ms | Typical medium net |
| 500 x 500 (250K cells) | 15-30 | 5-15ms | Long net, sparse obstacles |
| 1000 x 1000 (1M cells) | 20-50 | 10-30ms | Full-board route |

The Corolla subgraph extraction keeps the active grid small. For interactive
routing, the subgraph is the bounding box of the source point (last committed
segment end) and the current mouse position, expanded by a margin. Typical
interactive subgraphs are 100x100 to 300x300 cells.

### 3.3 GPU Dispatch Overhead

Each GPU dispatch (command encoding + submission + fence wait) has fixed
overhead:

| Operation | Typical time |
|-----------|-------------|
| Command encoding (CPU) | 0.05-0.1ms |
| Queue submission | 0.01ms |
| Pipeline bind | 0.02ms per pass |
| Staging buffer map/unmap | 0.1-0.5ms |

For a BF solve with 10 iterations and convergence check:
- 10 BF dispatches: 10 x 0.13ms overhead = 1.3ms
- 1-2 convergence readbacks: 0.2-1.0ms
- Total overhead: ~2ms

**Optimization**: Batch multiple BF iterations into a single command buffer
(already planned in `01-corolla-bellman-ford.md`, `bf_batch_size` config param).
With batch size 8, overhead drops to ~0.5ms for 10 iterations.

### 3.4 Persistent GPU State Between Mouse Moves

Interactive routing benefits from keeping GPU state persistent between mouse
movements:

**Always persistent** (uploaded once at route-start, freed at route-end):
- Obstacle bitmap (changes only when traces are pushed)
- DRC clearance matrix
- Grid parameters (dimensions, layer count)
- Compiled pipelines and bind group layouts

**Reset per mouse move**:
- Distance array (reset to INFINITY)
- Predecessor array (reset to NONE)
- Source cell (updated to current route start)
- Target cell (updated to mouse position)

The reset operation is a single GPU dispatch (< 0.3ms for a 200x200 subgraph).
Obstacle map updates for push mode are incremental (update only changed cells).

### 3.5 Incremental Obstacle Map Updates

When the user is actively routing (placing segments), the obstacle map must be
updated incrementally:

1. **User commits a segment** (left-click): Mark the committed segment's cells
   as obstacles. This is a small bitmap update (typically 10-100 cells).
2. **User pushes a trace** (push mode): Remove pushed trace's cells, add new
   trace's cells. Bitmap OR/AND operations on affected cells.
3. **User undoes a segment** (backspace): Remove the last segment's cells from
   the obstacle map.

These incremental updates are cheap on GPU: a small dispatch that writes to
specific cells in the obstacle buffer. No full obstacle map rebuild needed.

---

## 4. Gloss / Post-Route Optimization

### 4.1 Altium's Gloss System

Altium applies "gloss" (smoothing) after each segment commit. The gloss engine
(`TGlossEffort`: None/Weak/Strong) uses the visibility graph to find smoother
paths through the routed trace:

- **None**: No smoothing.
- **Weak**: Remove unnecessary bends, straighten when possible.
- **Strong**: Aggressive corner optimization using Dijkstra on the visibility
  graph contours.

Neighbor gloss (`NeighborGlossEffort`) applies the same smoothing to adjacent
pre-existing traces that were displaced by push operations.

### 4.2 GPU-Accelerated Gloss

Real-time gloss on GPU can be implemented as a post-processing pass after the
BF path is computed:

**Corner optimization**: For each corner in the BF path, check if a shortcut
exists (diagonal or arc that bypasses the corner while maintaining clearance).
This is a local operation per corner vertex.

```
For each corner vertex V in path:
  Let A = predecessor of V, B = successor of V
  Try direct A->B connection:
    If no obstacles in the swept area of A->B:
      Replace A->V->B with A->B (corner elimination)
  Try 45-degree chamfer A->M->B:
    If no obstacles in chamfer corridor:
      Replace A->V->B with A->M->B (45-degree conversion)
```

**GPU implementation**: One thread per corner vertex. Each thread checks obstacle
bitmap in the shortcut corridor. This is embarrassingly parallel and completes in
< 0.5ms for typical paths (10-50 corners).

**Rubber-banding**: Pull trace vertices toward shorter paths using clearance
queries. Already planned in `crates/autopcb-router/src/optimize/rubber_band.rs`.
For interactive use, run 1-2 iterations of rubber-banding instead of the full
convergence loop used in batch optimization.

### 4.3 Latency for Interactive Gloss

| Gloss level | Operations | GPU time | Total with routing |
|-------------|-----------|----------|-------------------|
| None | Skip | 0ms | BF only |
| Weak | Corner elimination | 0.2-0.5ms | BF + 0.5ms |
| Strong | Corner + rubber-band (2 iters) | 0.5-2ms | BF + 2ms |
| Neighbor | Reroute affected neighbors | 2-10ms | BF + push + 10ms |

All within interactive budget when combined with walk-around routing.

---

## 5. Length Tuning / Accordion on GPU

### 5.1 Altium's Length Tuning System

Altium's `IPCB_AccordionMakerSettings` supports four meander styles
(`TAccordionMode`):

- **Accordion**: Standard back-and-forth meander
- **Trombone**: Single-direction U-turn extensions
- **Sawtooth**: Diagonal teeth pattern
- **Root**: Internal container mode

With three corner styles (`TAccordionStyle`):
- 45-degree mitered lines
- Arc-mitered corners
- Fully rounded corners

The user interactively adjusts amplitude, gap, and orientation while the tool
shows real-time length feedback from `GetState_EstimateLength()`.

### 5.2 GPU-Accelerated Length Calculation

During interactive length tuning, the most performance-critical operation is
**real-time net length calculation**. The user adjusts meander parameters and
immediately sees the updated total net length.

**GPU parallel length calculation**:
```wgsl
// One thread per trace segment
@compute @workgroup_size(64)
fn compute_segment_lengths(@builtin(global_invocation_id) gid: vec3<u32>) {
    let seg_idx = gid.x;
    if (seg_idx >= params.num_segments) { return; }

    let seg = segments[seg_idx];
    let dx = f32(seg.x2 - seg.x1);
    let dy = f32(seg.y2 - seg.y1);
    let length = sqrt(dx * dx + dy * dy);

    // Parallel reduction to compute total
    atomicAdd(&total_length, u32(length * SCALE));
}
```

For a net with 100-500 segments, this completes in < 0.1ms. The total net
length is available on GPU; only a single u32 readback is needed for display.

### 5.3 Serpentine Geometry Generation on GPU

Serpentine (meander) geometry generation is fundamentally a sequential operation:
each meander segment depends on the previous one's end point and the available
space. However, given fixed meander parameters (amplitude, gap, count), the
geometry is deterministic and can be computed in parallel:

**Parallel meander generation**:
```
Given: base_path, amplitude, gap, style, count
For each meander i in 0..count (parallel):
  offset_along_path = start_offset + i * (2 * amplitude + gap)
  perpendicular_offset = amplitude * direction(i)
  generate_corner_segments(offset, perpendicular_offset, style)
```

Each meander is independent once the count and spacing are known. GPU threads
can generate meander geometry in parallel, then concatenate results.

**Practical consideration**: Meander counts are typically 5-50. This is not
enough parallelism to justify GPU dispatch overhead. Meander geometry generation
is better handled on CPU (< 0.1ms for 50 meanders). The GPU's value is in the
length calculation and DRC validation of the generated pattern, not in the
geometry generation itself.

### 5.4 DRC Validation of Meanders

After generating meander geometry, validate that the pattern does not violate
clearance rules. This reuses the X-Check GPU DRC pipeline
(`03-xcheck-gpu-drc.md`):

1. Upload meander segments to the GPU segment buffer
2. Run sweepline DRC pass (existing shader pipeline)
3. Read back violation count

If violations exist, reduce amplitude or gap and regenerate. This
generate-validate loop runs 2-3 iterations in < 5ms total.

---

## 6. Diff-Pair Interactive Routing

### 6.1 Altium's Diff-Pair Interactive Process

`IPCB_InteractiveDiffPairRoutingProcess` routes both pair members simultaneously
with gap mode selection (`TRoutingDiffPairGapMode`: Min/Preferred/Max). The router
maintains the configured gap from `IPCB_DifferentialPairsRoutingRule` and reports
uncoupled length and intra-pair skew.

### 6.2 Coupled Bellman-Ford for Interactive Diff-Pair

Our GPU diff-pair routing plan (`docs/notes/autorouter-gpu/04-highspeed-routing.md`)
describes coupled Bellman-Ford, where the search state space encodes both trace
positions simultaneously:

```
Coupled state:   (x, y, layer, direction) -- N offset implicit from gap
Uncoupled state: (xP, yP, xN, yN, layer) -- both positions explicit
```

For **interactive** diff-pair routing, the same algorithm applies with these
adaptations:

1. **Local subgraph**: Extract subgraph around the diff-pair source-to-mouse
   bounding box (expanded by gap + margin). Since both traces are within this
   bbox, the subgraph is at most 2x wider than a single-net subgraph.

2. **Coupled-only mode for interactive**: In interactive mode, force coupled
   routing (suppress uncoupled state exploration) for responsiveness. Allow
   uncoupling only at obstacles where coupling is geometrically impossible.
   This dramatically reduces the search space.

3. **Gap enforcement in cost function**: Add a penalty for deviating from the
   preferred gap:
   ```wgsl
   let gap_deviation = abs(actual_gap - preferred_gap);
   let gap_penalty = gap_deviation * GAP_WEIGHT;
   let edge_cost = base_cost + gap_penalty;
   ```

### 6.3 Latency for Interactive Diff-Pair

The coupled BF search space is 2-4x larger than single-net (due to the direction
dimension and occasional uncoupled exploration). Expected latency:

| Subgraph | Single-net BF | Coupled diff-pair BF |
|----------|--------------|---------------------|
| 100x100 | 1-2ms | 3-6ms |
| 200x200 | 2-5ms | 5-12ms |
| 300x300 | 5-10ms | 10-25ms |

For typical interactive routing (100x100 to 200x200 subgraphs), diff-pair
interactive routing runs in 3-12ms -- within the 16ms frame budget for 60 FPS.

### 6.4 Push Mode for Diff-Pair

When the diff-pair encounters pushable obstacles, push routing applies to both
traces simultaneously. The push algorithm from Section 2 is extended: the "new
trace" is actually two traces (P and N), and conflicting traces must be rerouted
to avoid both.

---

## 7. Bus/Multi-Net Interactive Routing

### 7.1 Altium's Multi-Route Process

`IPCB_InteractiveMultiRoutingProcess` routes multiple nets simultaneously with
uniform `BusSpacing`. The user selects multiple ratsnest connections and routes
them as a parallel group.

### 7.2 GPU Batch Routing for Interactive Bus

This maps directly to our multi-net batching strategy from
`docs/notes/autorouter-gpu/00-overview.md` Section 4 (Multi-Net Parallelism).
The key insight: bus nets are by definition parallel and non-conflicting (they
route side-by-side with fixed spacing), so they can be routed simultaneously
on GPU.

**Parallel BF for bus routing**:
```
For each bus member net (parallel on GPU):
  source = bus_source + member_index * bus_spacing * perpendicular_direction
  target = bus_target + member_index * bus_spacing * perpendicular_direction
  Run BF on local subgraph with offset source/target
```

Each bus member routes on a slightly offset subgraph. All N members route
simultaneously in a single GPU dispatch by using N source cells in the
distance array initialization.

**Alternative**: Route bus members sequentially, each treating previously routed
members as fixed obstacles. This produces tighter results (each member hugs its
predecessor) but is N times slower. For interactive use, parallel routing is
preferred.

### 7.3 Latency for Interactive Bus Routing

| Bus width (nets) | Parallel BF | Sequential BF |
|-----------------|------------|---------------|
| 4 | 3-5ms | 8-16ms |
| 8 | 3-5ms | 16-32ms |
| 16 | 5-8ms | 32-64ms |
| 32 | 8-15ms | > 100ms |

Parallel bus routing scales well on GPU. Even 32-wide buses route within
interactive budget. Sequential routing exceeds budget beyond 8-wide buses.

**Shared infrastructure**: Standard BF/GAMER infrastructure. Multiple source
cells set in the distance array. InstantGR-style spatial partitioning for
independence verification.

---

## 8. Shared Infrastructure Between Batch and Interactive Routing

### 8.1 What Is Shared (Identical Code)

| Component | Batch Router | Interactive Router | Shared? |
|-----------|-------------|-------------------|---------|
| Obstacle bitmaps | `obstacle_buf` | Same buffer | Yes |
| BF/GAMER shaders | `bellman_ford.wgsl`, `horizontal_sweep.wgsl`, etc. | Same shaders | Yes |
| Via transition shader | `via_transition.wgsl` | Same shader | Yes |
| Distance/predecessor buffers | `distance_buf`, `predecessor_buf` | Same buffers | Yes |
| Cost encoding (fixed-point u32) | Scale 1024 | Same encoding | Yes |
| Grid linearization | `layer * (W*H) + y * W + x` | Same formula | Yes |
| Path reconstruction (CPU) | `trace_back()` | Same function | Yes |
| Subgraph extraction (Corolla) | Bounding box + expansion | Same algorithm | Yes |
| DRC (X-Check sweepline) | `sweepline_check.wgsl` | Same shaders | Yes |
| Pipeline/bind-group management | `GpuRouter` struct | Same struct | Yes |
| Obstacle packing | `pack_obstacle_bitmaps()` | Same function | Yes |

### 8.2 What Is Different

| Aspect | Batch Router | Interactive Router |
|--------|-------------|-------------------|
| Outer loop | PathFinder (rip-up/reroute, 10-100 iterations) | Single-pass or bounded push (1-5 iterations) |
| History costs | Accumulated across iterations, drives convergence | Not used (no global negotiation) |
| Convergence criterion | All nets routed, zero DRC violations | Single net routed, local DRC clean |
| Net ordering | Priority-based, seeded RNG | User-driven (route what the user clicks) |
| Latency requirement | Minutes acceptable | < 16-50ms required |
| Subgraph scope | Per-net bbox (may be large) | Source-to-mouse bbox (always local) |
| Obstacle map updates | Full rebuild per iteration | Incremental per mouse move |
| Solution output | `RouteSolution` file | Live board update (track primitives) |
| Undo support | Not needed (batch output) | Required (backspace = undo last segment) |
| Gloss/smoothing | Post-route pass (`optimize_solution`) | Real-time per segment |

### 8.3 What Needs New Implementation

1. **Incremental obstacle map update** (`update_obstacle_cells()`) -- add/remove
   specific cells from the obstacle bitmap without full rebuild.
2. **Push conflict detection** -- R-tree query to identify traces whose clearance
   envelopes overlap the new route.
3. **Partial path support** -- route from a mid-trace point (the last committed
   segment end) rather than from a pad.
4. **Undo stack** -- maintain a stack of obstacle map snapshots for backspace
   (or more efficiently, a log of obstacle map deltas).
5. **Mouse-position source/target update** -- fast uniform buffer update to
   change source/target cells without rebuilding the entire parameter struct.
6. **Proximity cost texture** -- distance transform from obstacle bitmap for
   hug-and-push mode.
7. **Interactive solution output** -- convert GPU path to PCB track primitives
   for live board rendering.

---

## 9. Architecture Proposal

### 9.1 `GpuRoutingEngine` -- Shared Core

Factor all shared GPU infrastructure into a single `GpuRoutingEngine` struct:

```rust
/// Core GPU routing infrastructure shared between batch and interactive routing.
///
/// Owns the wgpu device, compiled pipelines, and persistent GPU buffers.
/// Both `BatchRouter` (PathFinder loop) and `InteractiveRouter` (single-pass)
/// operate through this engine.
pub struct GpuRoutingEngine {
    // Device and queue (owned or shared with viewer)
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,

    // Compiled compute pipelines (created once at startup)
    pipelines: RoutingPipelines,

    // Persistent GPU buffers
    obstacle_fixed_buf: wgpu::Buffer,   // fixed obstacles (pads, keepouts)
    obstacle_trace_buf: wgpu::Buffer,   // pushable trace obstacles
    distance_buf: wgpu::Buffer,         // per-solve, reset before each use
    predecessor_buf: wgpu::Buffer,      // per-solve, reset before each use
    history_buf: wgpu::Buffer,          // PathFinder history (batch only)
    occupancy_buf: wgpu::Buffer,        // per-cell net count
    params_buf: wgpu::Buffer,           // uniform: grid dims, costs, source/target

    // Staging buffers for readback
    predecessor_staging: wgpu::Buffer,
    change_flag_staging: wgpu::Buffer,

    // Grid dimensions (fixed at engine creation)
    grid_width: u32,
    grid_height: u32,
    layer_count: u32,

    // Grid config for coordinate conversion
    grid_config: GridConfig,
}

struct RoutingPipelines {
    reset: wgpu::ComputePipeline,
    set_sources: wgpu::ComputePipeline,
    bellman_ford: wgpu::ComputePipeline,
    h_sweep: wgpu::ComputePipeline,
    v_sweep: wgpu::ComputePipeline,
    via_transition: wgpu::ComputePipeline,
    convergence: wgpu::ComputePipeline,
    history_update: wgpu::ComputePipeline,
    corner_optimize: wgpu::ComputePipeline,
    drc_sweepline: wgpu::ComputePipeline,
    drc_history_update: wgpu::ComputePipeline,
}
```

### 9.2 `BatchRouter` -- PathFinder Loop

Uses `GpuRoutingEngine` for the batch autorouter:

```rust
pub struct BatchRouter<'a> {
    engine: &'a GpuRoutingEngine,
    pathfinder_state: PathFinderState,
    config: RoutingConfig,
}

impl<'a> BatchRouter<'a> {
    pub fn route_board(
        &mut self,
        workspace: &RoutingWorkspace,
        ir: &PcbIr,
    ) -> Result<RouteSolution, RoutingError> {
        // PathFinder loop: rip-up -> order -> route all nets -> update history -> converge
        for iteration in 0..self.config.max_iterations {
            self.rip_up_all(workspace);
            let ordered_nets = self.order_nets(workspace, ir);
            for net_id in ordered_nets {
                self.route_net(workspace, net_id)?;
            }
            self.engine.dispatch_history_update(&self.pathfinder_state);
            self.engine.dispatch_drc(workspace);
            if self.check_convergence() { break; }
        }
        self.build_solution()
    }
}
```

### 9.3 `InteractiveRouter` -- Single-Pass

Uses the same `GpuRoutingEngine` for interactive routing:

```rust
pub struct InteractiveRouter<'a> {
    engine: &'a GpuRoutingEngine,
    mode: TAdvancedRouteMode,
    committed_path: Vec<PathSegment>,    // segments committed by user clicks
    undo_stack: Vec<ObstacleMapDelta>,   // for backspace
    current_layer: LayerId,
    routing_width: f64,
    gloss_effort: GlossEffort,
}

impl<'a> InteractiveRouter<'a> {
    /// Called on every mouse move. Returns the proposed route from the last
    /// committed point to the mouse position.
    pub fn update(
        &mut self,
        mouse_pos: PointMm,
    ) -> Result<Vec<PathSegment>, RoutingError> {
        let source = self.last_committed_point();
        let target = self.engine.grid_config.to_grid(mouse_pos);

        match self.mode {
            IgnoreObstacle => self.route_ignore(source, target),
            WalkAround => self.route_walkaround(source, target),
            Push => self.route_push(source, target),
            HugAndPush => self.route_hug_and_push(source, target),
            StopAtFirst => self.route_stop_at_first(source, target),
            AutoRouteCurrent => self.route_auto_single_layer(source, target),
            AutoRouteMulti => self.route_auto_multi_layer(source, target),
        }
    }

    /// User commits current route segment (left-click).
    pub fn commit(&mut self) -> Result<(), RoutingError> {
        // Add committed segment cells to obstacle map
        // Push undo delta
        // Update committed_path
    }

    /// User undoes last segment (backspace).
    pub fn undo(&mut self) -> Result<(), RoutingError> {
        // Pop undo delta, restore obstacle map
        // Remove last segment from committed_path
    }

    fn route_walkaround(
        &self,
        source: GridCell,
        target: GridCell,
    ) -> Result<Vec<PathSegment>, RoutingError> {
        self.engine.dispatch_reset();
        self.engine.dispatch_set_source(source);
        self.engine.dispatch_bf_until_converge(target)?;
        let predecessors = self.engine.download_predecessors();
        let path = self.engine.trace_back(&predecessors, source, target);
        if self.gloss_effort != GlossEffort::None {
            self.engine.dispatch_corner_optimize(&path);
        }
        Ok(path)
    }
}
```

### 9.4 `PushRouter` -- Mini-PathFinder on Affected Nets

A specialized component used by `InteractiveRouter` in push mode:

```rust
pub struct PushRouter<'a> {
    engine: &'a GpuRoutingEngine,
    max_push_depth: u32,
}

impl<'a> PushRouter<'a> {
    pub fn push_route(
        &self,
        new_trace: &[PathSegment],
        workspace: &RoutingWorkspace,
    ) -> Result<PushResult, RoutingError> {
        // 1. Identify conflicting traces (CPU, R-tree query)
        let conflicts = workspace.find_conflicts(new_trace);

        // 2. For each conflict, reroute via GPU BF
        let mut pushed_traces = Vec::new();
        for conflict in &conflicts {
            self.engine.update_obstacle_cells(conflict.cells(), false); // remove
            self.engine.update_obstacle_cells(new_trace_cells, true);  // add

            let rerouted = self.engine.route_net_bf(
                conflict.source, conflict.target, conflict.net_id
            )?;
            pushed_traces.push(rerouted);
        }

        // 3. Validate via local DRC
        self.engine.dispatch_local_drc(new_trace, &pushed_traces)?;

        Ok(PushResult { new_trace, pushed_traces })
    }
}
```

### 9.5 Configuration Differences

| Parameter | Batch (`BatchRouter`) | Interactive (`InteractiveRouter`) |
|-----------|----------------------|----------------------------------|
| `bf_batch_size` | 8 (amortize overhead) | 1-2 (minimize latency) |
| `subgraph_expansion` | 0.021 * sqrt(grid) | Fixed 50-cell margin |
| `history_enabled` | true | false |
| `drc_mode` | Full (sweepline) | Local (bounding-box check) |
| `gloss_enabled` | Post-route only | Real-time per segment |
| `max_bf_iterations` | 200 (full convergence) | 30 (bounded latency) |
| `convergence_check_freq` | Every 8 BF iters | Every 1-2 BF iters |

---

## 10. PcbIr / Spec Extensions for Interactive Routing

### 10.1 Additional Data for Interactive Routing

The interactive router requires data not needed by the batch autorouter:

| Data | Purpose | Source |
|------|---------|--------|
| Mouse position | Route target | UI event |
| Partial path | Already-committed segments | Router state |
| Segment lock flags | Preserve user-placed segments | `IrTrack::locked` |
| Undo history | Backspace support | Router state |
| Route mode | Conflict resolution strategy | User selection |
| Corner style | 90/45/any angle | `TRoutingCornerStyle` |
| Width mode | Default/Min/Preferred/Max | `TRoutingWidthMode` |
| Gloss effort | None/Weak/Strong | `TGlossEffort` |
| Hugging style | Mixed/Rounded/Degrees | `THuggingStyle` |
| Via combination | Available via stacks | `IPCB_ViaCombinationManagerInterface` |

Most of this is session state (not persisted in PcbIr). The interactive router
holds this state in its own struct.

### 10.2 New Types in `autopcb-ir`

```rust
/// Interactive routing session options (not persisted in PcbIr).
pub struct InteractiveRoutingOptions {
    pub route_mode: AdvancedRouteMode,
    pub corner_style: RoutingCornerStyle,
    pub width_mode: RoutingWidthMode,
    pub gloss_effort: GlossEffort,
    pub neighbor_gloss_effort: GlossEffort,
    pub hugging_style: HuggingStyle,
    pub allow_via_pushing: bool,
    pub auto_remove_loops: bool,
    pub auto_remove_antennas: bool,
    pub follow_mouse_trail: bool,
    pub restrict_to_90_45: bool,
}

/// Mirrors TAdvancedRouteMode from Altium.
pub enum AdvancedRouteMode {
    IgnoreObstacle,
    WalkAroundObstacle,
    PushObstacle,
    HugAndPushObstacle,
    StopAtFirstObstacle,
    AutoRouteCurrentLayer,
    AutoRouteMultiLayer,
}

/// Mirrors TGlossEffort from Altium.
pub enum GlossEffort {
    None,
    Weak,
    Strong,
}

/// Mirrors THuggingStyle from Altium.
pub enum HuggingStyle {
    Mixed,
    Rounded,
    Degrees,
}

/// Mirrors TRoutingCornerStyle from Altium.
pub enum RoutingCornerStyle {
    Style90,
    Style45,
    Any,
}
```

### 10.3 Spec Language Extensions

The spec language does not need interactive routing directives (interactive
routing is a UI operation, not a batch operation). However, the spec can
declare routing preferences that inform the interactive router:

```
routing_options {
    default_mode: walkaround
    gloss: strong
    corner_style: 45
    auto_remove_loops: true
}
```

These preferences are loaded by the viewer/editor and passed to the
`InteractiveRouter` at session start.

---

## 11. Sliding / Trace Dragging on GPU

### 11.1 Altium's Sliding Process

`IPCB_SlidingRoutingProcess` slides existing trace segments while maintaining
connections at both ends. Uses `TVertexAction` to control vertex handling:
`eDeform` (move segment, adjust neighbors), `eScale` (proportional scaling),
`eSmooth` (smooth transition curves).

### 11.2 GPU-Accelerated Sliding

Sliding is a constrained optimization: move a trace segment to a new position
while keeping both endpoints connected to their neighbors. The constraint is
that the connecting segments (from the fixed neighbors to the new segment
position) must be legal (no DRC violations).

**GPU approach**: For each candidate slide position:
1. Compute connecting segment geometry (CPU, simple geometry)
2. DRC check connecting segments against obstacles (GPU DRC pass)
3. If legal, output the new segment position

Since the user drags interactively, the slide position changes every mouse
move. The DRC check is the performance-critical operation. Using the GPU
DRC pipeline, checking a 3-segment slide configuration against the full
obstacle map takes < 1ms.

**Push during sliding**: When `TAdvancedRouteMode` is set for the sliding
process, conflicting traces can be pushed during the slide. This uses the
same push algorithm from Section 2, applied to the sliding segment.

---

## 12. Implementation Roadmap

### Phase 1: GpuRoutingEngine Extraction (refactor existing code)

Factor `GpuGridRouter` (from `01-corolla-bellman-ford.md`) into a shared
`GpuRoutingEngine` + `BatchRouter` wrapper. No new functionality, just
restructuring:

**Files**:
- `crates/autopcb-router/src/gpu/engine.rs` -- `GpuRoutingEngine`
- `crates/autopcb-router/src/gpu/batch.rs` -- `BatchRouter`
- `crates/autopcb-router/src/gpu/mod.rs` -- exports

### Phase 2: Interactive Router Scaffold

Implement `InteractiveRouter` with walk-around mode only:

**Files**:
- `crates/autopcb-router/src/gpu/interactive.rs` -- `InteractiveRouter`
- `crates/autopcb-router/src/gpu/incremental.rs` -- incremental obstacle updates

**Acceptance criteria**:
- Walk-around route computed in < 10ms on a 200x200 subgraph
- Route updates on mouse position change
- Commit and undo operations work correctly

### Phase 3: Push Routing

Add push mode to `InteractiveRouter`:

**Files**:
- `crates/autopcb-router/src/gpu/push.rs` -- `PushRouter`

**Acceptance criteria**:
- Push routing resolves 5 conflicting traces in < 30ms
- Pushed traces are DRC-clean
- Undo correctly restores pre-push state

### Phase 4: Advanced Modes

Add remaining interactive modes:

- Hug-and-push (proximity cost texture + push fallback)
- Stop-at-first (partial path reconstruction)
- Auto-route current/multi layer (single-pass BF with layer constraints)
- Interactive gloss (corner optimization dispatch)

### Phase 5: Diff-Pair and Bus Interactive Routing

- Coupled BF for interactive diff-pair
- Parallel BF for bus routing
- Gap enforcement and skew reporting

### Phase 6: Length Tuning and Sliding

- Real-time length calculation on GPU
- Meander DRC validation
- Trace sliding with GPU DRC

---

## 13. Open Questions

1. **GPU device sharing between viewer and router**: When interactive routing
   runs inside the viewer, the router needs to share the GPU device. The
   `Arc<wgpu::Device>` approach works but requires careful synchronization
   between rendering and compute workloads. wgpu does not support concurrent
   compute + render on the same queue (tracked in wgpu #5576). Workaround:
   time-slice between render frames and routing dispatches.

2. **Obstacle map granularity for push**: The push algorithm needs per-net
   obstacle tracking (to know which net owns each obstacle cell). The current
   single-bitmask-per-layer approach only records "blocked/unblocked". Extending
   to per-cell net ID storage requires 16 bits per cell per layer (supporting
   up to 65535 nets), quadrupling obstacle buffer size. Alternative: maintain
   a CPU-side net-to-cell mapping and use the GPU obstacle map for routing only.

3. **Undo granularity**: Should undo restore obstacle maps at the GPU buffer
   level (fast, requires storing full obstacle snapshots) or at the semantic
   level (reconstruct obstacle map from the updated trace list)? Snapshot
   approach is O(obstacle_buffer_size) per undo step (~4-8 MB for large boards).
   Semantic approach is cheaper in storage but requires CPU-GPU round-trip for
   obstacle rebuild.

4. **Async push resolution**: For dense areas where push resolution exceeds the
   frame budget, should we display an "in-progress" state (new trace rendered,
   pushed traces still resolving) or block until push completes? Altium blocks
   (the UI freezes briefly during complex push operations). An async approach
   provides better UX but requires careful state management.

5. **Corner style enforcement in BF**: The BF grid router produces Manhattan
   or 8-way paths. Converting to 45-degree or arc corners is a post-processing
   step (gloss). Altium's router natively produces the correct corner style
   during routing. Our approach (route on grid, then smooth) may produce
   suboptimal paths that cannot be smoothed to the target corner style. Need
   to evaluate whether grid resolution is sufficient for 45-degree routing or
   whether the GAMER sweep approach (which only supports Manhattan) needs a
   diagonal sweep extension.

6. **Warm-start for mouse-move updates**: When the mouse moves slightly, the
   new route is very similar to the previous one. Can we warm-start the BF
   solve using the previous distance array (without resetting to INFINITY)?
   This would reduce BF iterations from 5-10 to 1-2 for small mouse movements.
   Risk: stale distances from the previous solve may cause incorrect paths if
   obstacles changed. Safe warm-start requires versioning distance values.

---

## References

### Altium Reverse Engineering

- `docs/routing/active-router.md` -- C# interface analysis
- `docs/routing/push-pull-router.md` -- Conflict resolution modes
- `docs/routing/delphi-routing-engine.md` -- Delphi binary analysis
- `docs/routing/delphi-routing-engine-deep.md` -- Push algorithm, visibility graph, gloss engine
- `docs/routing/routing-data-model.md` -- PCB track/rule interfaces

### GPU Router Plans

- `docs/plans/router-gpu/01-corolla-bellman-ford.md` -- GPU BF implementation
- `docs/plans/router-gpu/02-gamer-sweep-routing.md` -- GAMER H/V sweep
- `docs/plans/router-gpu/03-xcheck-gpu-drc.md` -- GPU DRC pipeline
- `docs/plans/router-gpu/04-cypress-congestion-feedback.md` -- GPU congestion estimation

### GPU Research Notes

- `docs/notes/autorouter-gpu/00-overview.md` -- GPU acceleration overview
- `docs/notes/autorouter-gpu/04-highspeed-routing.md` -- Diff-pair, bus, length matching
- `docs/notes/autorouter-gpu/05-wgpu-implementation.md` -- wgpu patterns

### Router Plan

- `docs/plans/router/README.md` -- Full autorouter implementation plan

### Papers

- McMurchie & Ebeling, "PathFinder," FPGA 1995
- Corolla: GPU-Accelerated FPGA Routing, FPGA 2017
- GAMER: GPU-Accelerated Maze Routing, IEEE TCAD 2023
- X-Check: Parallel DRC, ICCAD 2022

---

## 14. Enhanced ActiveRoute: Multi-Layer Guided Routing with Via Support

### 14.1 Altium's ActiveRoute Limitation

Altium has three entirely separate routing systems:

1. **Interactive Routing** (`TAdvancedRoute` Delphi class) -- manual, one trace at a
   time, with walk-around/push/hug modes controlled by `TAdvancedRouteMode`. This is
   what Sections 1-4 of this document cover.
2. **ActiveRoute** -- a semi-automated guided routing mode. The user selects multiple
   nets and draws a "Route Guide" polyline corridor. The system uses **river routing**
   to distribute traces along the guide. **ActiveRoute cannot place vias.** Routing
   is restricted to a single layer.
3. **Autorouter** (`IPCB_SpecctraRouterOptions`) -- fully automatic batch routing
   using a Specctra-derived engine. Handles all layers, vias, and full conflict
   resolution.

ActiveRoute fills a gap between manual interactive routing (one net at a time, tedious
for buses) and the full autorouter (black-box, less user control). But its river-routing
approach is fundamentally limited: it treats the route guide as a 1D corridor and
distributes traces evenly across it, with no ability to transition between layers.

### 14.2 Our Enhanced ActiveRoute

Our implementation uses the same GPU Bellman-Ford infrastructure as the batch
autorouter (Corolla BF from `01-corolla-bellman-ford.md`) and the InstantGR batching
strategy from `05-instantgr-net-batching.md`. This means our ActiveRoute inherits
capabilities that Altium's river-routing approach cannot provide:

**Full via support**: Layer transitions are a natural part of the BF cost function.
When a routing corridor spans regions where a single layer cannot accommodate all
traces, the BF solver places vias and routes across layers. Via costs come from the
same `ViaCostModel` used by the batch autorouter -- via placement quality does not
degrade relative to batch mode.

**PathFinder negotiation for conflict resolution**: When multiple selected nets
conflict within the corridor, our ActiveRoute runs a bounded PathFinder negotiation
(limited iterations, not full convergence). Altium's ActiveRoute simply fails when
traces conflict. Ours resolves conflicts through history-cost rip-up/reroute, bounded
to a small iteration count for latency.

**Multi-layer routing**: The BF search naturally spans all allowed layers. Altium's
ActiveRoute is single-layer only.

**GPU-accelerated multi-net parallelism**: With InstantGR batching, independent nets
within the selected group are routed simultaneously on the GPU. For 100 nets with
typical independence ratios, this produces 5-15 batches of 7-20 nets each, all
resolved in under 1 second.

### 14.3 Route Guide to Routing Corridor Mapping

Altium's Route Guide is a polyline drawn interactively by the user. Our equivalent
has two input paths:

1. **Spec-declared corridors**: The `routing_corridor` directive in a `.pcb` spec
   declares a corridor path and width. The LLM can generate these from schematic
   analysis:
   ```
   routing_corridor "ddr_bus" {
       path: [(10mm, 20mm), (10mm, 80mm), (50mm, 80mm)]
       width: 5mm
       nets: [DDR_D0, DDR_D1, ..., DDR_D15]
   }
   ```

2. **Interactive polyline**: In the viewer, the user draws a polyline (identical
   UX to Altium's Route Guide). The polyline is converted to a corridor constraint.

**Corridor geometry**: The route guide polyline is inflated into a bounding region
that constrains the BF search space. The inflation width is either:

- User-specified (explicit corridor width)
- Auto-computed: `width = (net_count * trace_pitch) + 2 * margin` where
  `trace_pitch = preferred_width + clearance` and `margin = 2 * trace_pitch`

**BF subgraph extraction**: The corridor polygon clips the BF subgraph. Cells outside
the corridor are marked as blocked in the obstacle bitmap (or more efficiently, the
subgraph extraction from Corolla only extracts cells within the corridor bounding box,
and an additional corridor mask buffer marks out-of-corridor cells as impassable).

```
Corridor subgraph extraction:
  1. Polyline → inflated polygon (offset by half corridor width)
  2. Polygon bounding box → Corolla subgraph extraction
  3. Per-cell corridor mask: cell in polygon → passable, else blocked
  4. Upload corridor mask to GPU as additional obstacle layer
```

This approach is more powerful than river routing because the BF solver finds optimal
paths through the corridor rather than distributing traces at uniform offsets. If the
corridor has obstacles (pads, keepouts), the BF routes around them. If the corridor
is too narrow for all traces on one layer, vias are placed automatically.

### 14.4 ActiveRoute Workflow

**Batch/spec-driven workflow**:
```
.pcb spec routing_corridor declaration
    │
    ├── Spec compiler extracts: net list, corridor polyline, width
    │
    ▼
ActiveRouter::route_corridor(nets, corridor, config)
    │
    ├── 1. Extract corridor subgraph (clip BF grid to corridor polygon)
    ├── 2. Partition nets via InstantGR batching (05)
    ├── 3. For each batch:
    │       GPU BF on corridor subgraph (Corolla, 01)
    ├── 4. PathFinder negotiation (bounded: 5 iterations max)
    │       Rip up conflicting nets, reroute with history costs
    ├── 5. Corner optimization (corners-only, no rubber-band)
    │
    ▼
RouteSolution (subset: only corridor nets)
```

**Interactive/viewer workflow**:
```
User selects nets in viewer (click, lasso, or net-class filter)
    │
    ├── Optionally: user draws route guide polyline
    │
    ▼
User clicks "Route" button
    │
    ├── Viewer calls ActiveRouter::route_corridor()
    ├── Iteration snapshots stream to viewer for real-time feedback
    │
    ▼
Results appear on board (accept / reject / undo)
```

### 14.5 Feature Comparison: Our ActiveRoute vs Altium

| Feature | Altium ActiveRoute | Our ActiveRoute |
|---------|-------------------|-----------------|
| Via placement | **No** (single layer only) | **Yes** (GPU BF with via costs) |
| Multi-layer routing | No | Yes (full layer stack) |
| Conflict resolution | None (fails on conflicts) | PathFinder negotiation (bounded) |
| Routing algorithm | River routing (uniform distribution) | Bellman-Ford SSSP (optimal paths) |
| Obstacle avoidance | Limited (corridor must be clear) | Full (BF routes around obstacles) |
| Speed (100 nets) | ~1 second | < 1 second (GPU parallel) |
| Route Guide input | Interactive drawing only | Spec-declared corridors + interactive |
| Diff-pair support | Yes (paired river routing) | Yes (coupled BF, `04-highspeed-routing.md`) |
| Length matching | No | Yes (post-route serpentine insertion) |
| DRC validation | Post-route only | Integrated X-Check GPU DRC per iteration |
| Undo granularity | All-or-nothing | Per-iteration snapshots |

---

## 15. Routing Profiles: Quality vs Latency Configuration

All routing modes (batch, ActiveRoute, interactive, push) use the same
`GpuRoutingEngine` and the same BF/GAMER shaders. They differ only in their
configuration profiles, which control the quality-vs-latency tradeoff.

### 15.1 `RoutingProfile` Enum

```rust
/// Controls the quality-vs-latency tradeoff for GPU routing.
///
/// Each profile adjusts PathFinder iterations, BF convergence thresholds,
/// optimization passes, and subgraph scope. All profiles use the same
/// GPU shaders and buffer infrastructure.
pub enum RoutingProfile {
    /// Full quality, no time limit. For batch autorouting via `route_board()`.
    /// PathFinder runs to convergence or max_iterations.
    BatchFull,

    /// Reduced PathFinder iterations, good quality. For ActiveRoute
    /// (guided multi-net corridor routing).
    /// Target: < 1 second for 100 nets.
    ActiveRoute {
        /// Maximum PathFinder negotiation iterations.
        /// Default: 5 (vs 50 for BatchFull).
        max_iterations: u32,
        /// Hard wall-clock time limit in milliseconds.
        /// Default: 1000. PathFinder exits early if exceeded.
        max_time_ms: u32,
        /// Skip post-route optimization (rubber-band, staircase elimination).
        /// Default: true. Corner optimization still runs (cheap).
        skip_optimization: bool,
    },

    /// Minimal iterations, fastest response. For interactive single-net
    /// routing (mouse-move updates).
    /// Target: < 16ms (60 FPS frame budget).
    Interactive {
        /// Maximum BF relaxation iterations (not PathFinder iterations).
        /// Default: 50. For a 200x200 subgraph, BF converges in 10-20
        /// iterations; 50 provides headroom for complex obstacles.
        max_bf_iterations: u32,
        /// Subgraph expansion factor relative to source-target bbox.
        /// Default: 0.5 (smaller than batch's 0.021 * sqrt(grid) because
        /// interactive subgraphs are already small).
        subgraph_expansion: f64,
    },

    /// Push mode: reroute conflicting nets after placing a new trace.
    /// Target: < 33ms (30 FPS frame budget).
    Push {
        /// Maximum number of conflicting nets to reroute.
        /// Default: 10. Beyond this, the remaining conflicts are deferred
        /// to the next mouse-move update (async push).
        max_conflicts: u32,
        /// Maximum push depth (recursive conflict resolution).
        /// Default: 3. Deeper push chains are truncated.
        max_reroute_iterations: u32,
    },
}
```

### 15.2 Profile Effect on Routing Parameters

| Parameter | BatchFull | ActiveRoute | Interactive | Push |
|-----------|-----------|-------------|-------------|------|
| PathFinder iterations | 50 (configurable) | 5 | 1 (no negotiation) | 1-3 (mini-PathFinder) |
| BF convergence | Strict (run to stable) | Relaxed (early exit) | First-valid path | First-valid path |
| BF max iterations | 200 | 100 | 50 | 50 |
| Optimization passes | Full (staircase + rubber-band + corners) | Corners only | None | None |
| Subgraph scope | Per-net bbox (Corolla) | Corridor polygon | Source-to-mouse bbox | New trace bbox + neighbors |
| History costs | Yes (accumulated) | Yes (bounded) | No | No |
| DRC mode | Full X-Check sweepline | X-Check per iteration | Local bbox check | Local bbox check |
| InstantGR batching | Yes (full partitioning) | Yes (corridor nets only) | No (single net) | No (affected nets only) |
| Via cost | Standard | Standard | Standard | Standard |
| Gloss/smoothing | Post-route (full) | Post-route (corners only) | Per-segment (if enabled) | None |

**Via cost is invariant across all profiles.** Via placement quality should never
degrade for interactive responsiveness. A poorly-placed via is worse than a
slightly slower route computation.

### 15.3 Latency Budgets

| Profile | Target Latency | Typical Net Count | GPU Time per Net |
|---------|---------------|-------------------|-----------------|
| BatchFull | Minutes (no limit) | 1000-50000 | 5-30ms |
| ActiveRoute | < 1 second total | 10-500 | 2-10ms (batched) |
| Interactive | < 16ms total | 1 | 2-5ms |
| Push | < 33ms total | 1 new + 2-10 pushed | 1-3ms each |

### 15.4 Profile Selection

Profile selection is automatic based on the calling context:

```rust
impl GpuRoutingEngine {
    /// Batch autorouter entry point.
    pub fn route_board(&self, workspace: &RoutingWorkspace) -> Result<RouteSolution> {
        self.route_with_profile(workspace, RoutingProfile::BatchFull)
    }

    /// ActiveRoute entry point (spec corridor or interactive selection).
    pub fn route_corridor(
        &self,
        workspace: &RoutingWorkspace,
        corridor: &RoutingCorridor,
        nets: &[NetId],
    ) -> Result<RouteSolution> {
        self.route_with_profile(
            workspace,
            RoutingProfile::ActiveRoute {
                max_iterations: 5,
                max_time_ms: 1000,
                skip_optimization: true,
            },
        )
    }

    /// Interactive single-net entry point (mouse-move update).
    pub fn route_interactive(
        &self,
        source: GridCell,
        target: GridCell,
        mode: AdvancedRouteMode,
    ) -> Result<Vec<PathSegment>> {
        // Uses RoutingProfile::Interactive internally
    }
}
```

---

## 16. Unified Routing Engine Architecture

All routing modes share the same `GpuRoutingEngine`. The differentiation happens
at the orchestration layer, not the GPU layer.

### 16.1 Architecture Diagram

```
GpuRoutingEngine (shared GPU state: device, queue, pipelines, buffers)
    │
    ├── BatchRouter (RoutingProfile::BatchFull)
    │     Outer loop: PathFinder negotiation (50 iterations)
    │     Inner loop: InstantGR batching → Corolla/GAMER BF per batch
    │     Post-route: X-Check DRC → optimization (staircase, rubber-band, corners)
    │     Output: RouteSolution with iteration snapshots
    │
    ├── ActiveRouter (RoutingProfile::ActiveRoute)
    │     Outer loop: PathFinder negotiation (5 iterations, time-bounded)
    │     Inner loop: InstantGR batching on corridor nets → Corolla BF
    │     Subgraph: Corridor polygon clip (not full-board bbox)
    │     Post-route: Corner optimization only
    │     NEW vs Altium: Full via support, multi-layer, conflict resolution
    │     Output: RouteSolution (corridor nets subset)
    │
    ├── InteractiveRouter (RoutingProfile::Interactive)
    │     No outer loop (single-pass BF, no PathFinder negotiation)
    │     Modes: walk-around, push, hug, stop, auto (TAdvancedRouteMode)
    │     Subgraph: Source-to-mouse bbox (small, local)
    │     No InstantGR (single net, no batching needed)
    │     Real-time gloss per committed segment
    │     Output: Vec<PathSegment> (live board update)
    │
    └── PushRouter (helper, used by InteractiveRouter in push mode)
          Mini-PathFinder on affected nets (1-3 iterations)
          Identifies conflicting traces via R-tree query
          Rip-up + reroute each conflict via Corolla BF on local subgraph
          Output: PushResult { new_trace, pushed_traces }
```

### 16.2 Shared vs Mode-Specific Components

**Shared across all modes** (identical code, same GPU pipelines):

| Component | Description |
|-----------|------------|
| BF/GAMER shaders | `bellman_ford.wgsl`, `horizontal_sweep.wgsl`, `vertical_sweep.wgsl` |
| Via transition shader | `via_transition.wgsl` — layer change cost/connectivity |
| Obstacle bitmaps | `obstacle_fixed_buf`, `obstacle_trace_buf` |
| Distance/predecessor arrays | `distance_buf`, `predecessor_buf` |
| Cost encoding | Fixed-point `u32`, scale factor 1024 |
| Grid linearization | `layer * (W*H) + y * W + x` |
| Path reconstruction | `trace_back()` on CPU from predecessor array |
| Subgraph extraction | Corolla bounding-box extraction |
| DRC shaders | `sweepline_check.wgsl` (X-Check) |
| Pipeline/bind-group management | `GpuRoutingEngine` struct |

**Mode-specific components**:

| Component | BatchRouter | ActiveRouter | InteractiveRouter |
|-----------|-------------|-------------|-------------------|
| Outer loop | PathFinder (full convergence) | PathFinder (bounded) | None |
| Net batching | InstantGR (all nets) | InstantGR (corridor nets) | N/A (single net) |
| History costs | Accumulated, drives convergence | Accumulated (bounded) | Disabled |
| Subgraph scope | Per-net Corolla bbox | Corridor polygon | Source-to-mouse bbox |
| Obstacle map updates | Full rebuild per iteration | Full rebuild per iteration | Incremental per mouse move |
| Optimization | Full post-route pipeline | Corners only | Real-time per segment |
| Solution output | `RouteSolution` (file) | `RouteSolution` (file or live) | `Vec<PathSegment>` (live) |
| Undo | N/A | Per-iteration | Per-segment (backspace) |

### 16.3 `ActiveRouter` Implementation

```rust
/// Guided multi-net router with corridor constraints.
///
/// Routes a set of user-selected (or spec-declared) nets within a
/// routing corridor. Uses GPU BF with InstantGR batching and bounded
/// PathFinder negotiation.
///
/// Unlike Altium's ActiveRoute, supports vias and multi-layer routing.
pub struct ActiveRouter<'a> {
    engine: &'a GpuRoutingEngine,
    profile: RoutingProfile,
}

impl<'a> ActiveRouter<'a> {
    pub fn route_corridor(
        &self,
        workspace: &RoutingWorkspace,
        corridor: &RoutingCorridor,
        nets: &[NetId],
    ) -> Result<RouteSolution, RoutingError> {
        // 1. Extract corridor subgraph
        let corridor_mask = self.build_corridor_mask(corridor, workspace);
        self.engine.upload_corridor_mask(&corridor_mask);

        // 2. Decompose multi-pin nets into 2-pin subnets
        let subnets = self.decompose_nets(workspace, nets);

        // 3. InstantGR batching on corridor nets
        let batches = self.engine.batch_nets_instantgr(&subnets, &corridor_mask);

        // 4. Bounded PathFinder negotiation
        let max_iters = match self.profile {
            RoutingProfile::ActiveRoute { max_iterations, .. } => max_iterations,
            _ => 5,
        };
        let start_time = Instant::now();

        let mut pathfinder = PathFinderState::new(workspace.grid_size());
        let mut solution = RouteSolutionBuilder::new();

        for iteration in 0..max_iters {
            // Time check
            if let RoutingProfile::ActiveRoute { max_time_ms, .. } = self.profile {
                if start_time.elapsed().as_millis() > max_time_ms as u128 {
                    break;
                }
            }

            // Rip up all corridor nets
            pathfinder.rip_up_nets(nets);

            // Route each batch via GPU BF
            for batch in &batches {
                self.engine.dispatch_batch_bf(batch, &pathfinder)?;
            }

            // Update history costs
            self.engine.dispatch_history_update(&pathfinder);

            // Check convergence (no oversubscribed cells within corridor)
            if pathfinder.corridor_converged(&corridor_mask) {
                break;
            }

            solution.capture_snapshot(iteration, &pathfinder);
        }

        // 5. Corner optimization (cheap, always applied)
        self.engine.dispatch_corner_optimize(&solution);

        solution.build()
    }

    fn build_corridor_mask(
        &self,
        corridor: &RoutingCorridor,
        workspace: &RoutingWorkspace,
    ) -> CorridorMask {
        // Inflate polyline by half-width to get corridor polygon
        // Rasterize polygon onto grid → per-cell bool mask
        // Cells outside corridor are treated as blocked by BF
        let polygon = corridor.polyline.inflate(corridor.width / 2.0);
        let mut mask = CorridorMask::new(workspace.grid_width(), workspace.grid_height());
        for cell in workspace.grid_cells_in_polygon(&polygon) {
            mask.set(cell, true);
        }
        mask
    }
}

/// Corridor constraint for guided routing.
pub struct RoutingCorridor {
    /// Polyline defining the corridor center path.
    pub polyline: Vec<PointMm>,
    /// Total corridor width. Traces are distributed within this width.
    /// If None, auto-computed from net count and trace pitch.
    pub width: Option<f64>,
    /// Allowed layers within the corridor. If empty, all layers allowed.
    pub allowed_layers: Vec<LayerId>,
}
```

---

## 17. Spec Language Extensions for ActiveRoute

### 17.1 `routing_corridor` Directive

The spec language already supports `routing { ... }` for batch autorouter config
(see `docs/plans/router/README.md`, Milestone 9). ActiveRoute adds the
`routing_corridor` directive for guided multi-net routing:

```
routing_corridor "ddr_data_bus" {
    path: [(10mm, 20mm), (10mm, 80mm), (50mm, 80mm)]
    width: 5mm
    nets: [DDR_D0, DDR_D1, DDR_D2, DDR_D3,
           DDR_D4, DDR_D5, DDR_D6, DDR_D7,
           DDR_D8, DDR_D9, DDR_D10, DDR_D11,
           DDR_D12, DDR_D13, DDR_D14, DDR_D15]
    layers: [Top, Inner1, Inner2]
    profile: active_route           // optional, default: active_route
    max_iterations: 5               // optional, override profile default
    max_time_ms: 2000               // optional, override profile default
}
```

Multiple corridors can be declared. They are routed independently (corridor nets
are disjoint -- the spec compiler validates this).

### 17.2 `routing_profile` Directive

For batch routing, the profile is implicitly `BatchFull`. For corridors, the
profile is implicitly `ActiveRoute`. The spec can override profile parameters:

```
routing {
    grid_resolution: 0.1mm
    max_iterations: 50
    profile: batch_full             // explicit (same as default)
}

routing_corridor "power_bus" {
    path: [(5mm, 5mm), (95mm, 5mm)]
    width: 10mm
    nets: [VCC_3V3, GND, VCC_5V]
    profile: active_route {
        max_iterations: 10
        max_time_ms: 5000
        skip_optimization: false    // enable full optimization for power
    }
}
```

---

## 18. Viewer Integration for ActiveRoute

### 18.1 Interactive ActiveRoute Workflow

The viewer provides a GUI workflow analogous to Altium's ActiveRoute UX, but
with the enhanced capabilities described in Section 14:

```
Step 1: Select Nets
    User clicks individual ratsnest lines, lasso-selects a group,
    or filters by net class (e.g., "DDR_DATA").
    Selected nets are highlighted in the viewer.

Step 2: Draw Route Guide (optional)
    User draws a polyline on the board.
    The polyline defines the corridor center path.
    Corridor width is auto-computed or user-adjustable via slider.
    Corridor region is shown as a translucent overlay.

Step 3: Route
    User clicks "Route" (or presses Enter).
    ActiveRouter::route_corridor() runs on GPU.
    Iteration snapshots stream back to the viewer:
      - After each PathFinder iteration, the current routing state
        is rendered (traces appear/disappear as rip-up/reroute runs).
      - Progress bar shows iteration count and conflict count.
    Typical time: < 1 second for 100 nets.

Step 4: Review
    Routed traces are displayed on the board.
    DRC violations (if any) are highlighted.
    User can:
      - Accept (traces committed to board state)
      - Reject (all traces removed, board restored)
      - Undo per-iteration (step back through PathFinder iterations)
      - Adjust corridor width and re-route
```

### 18.2 Real-Time Feedback During Routing

Because GPU routing is fast enough, the viewer can display intermediate results:

| Event | Display Update | Latency |
|-------|---------------|---------|
| Routing starts | Corridor overlay appears | Immediate |
| PathFinder iteration N completes | Traces for iteration N rendered | < 200ms per iteration |
| Conflict detected | Conflicting traces highlighted in red | < 200ms |
| Routing completes | Final traces rendered, DRC overlay | < 1 second total |

The iteration snapshot mechanism from `RouteSolution` (see `docs/plans/router/README.md`,
Milestone 7) provides the per-iteration state. The viewer subscribes to a channel
that emits snapshots as the PathFinder loop runs:

```rust
/// Channel for streaming iteration snapshots to the viewer.
pub struct ActiveRouteProgress {
    pub iteration: u32,
    pub total_nets: u32,
    pub routed_count: u32,
    pub conflict_count: u32,
    pub paths: Vec<(NetId, Vec<PathSegment>)>,
}
```

### 18.3 Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `A` | Enter ActiveRoute mode (select nets + draw guide) |
| `Enter` | Route selected nets |
| `Escape` | Cancel / reject results |
| `Ctrl+Z` | Undo last iteration (or reject all) |
| `Scroll` | Adjust corridor width (while drawing guide) |

---

## 19. Implementation Roadmap Additions

The ActiveRoute and routing profile work extends the existing implementation
roadmap (Section 12) with two additional phases.

### Phase 7: Routing Profiles + ActiveRouter

**Depends on**: Phase 1 (GpuRoutingEngine), Phase 2 (InteractiveRouter scaffold)

**Files**:
- `crates/autopcb-router/src/gpu/profile.rs` -- `RoutingProfile` enum
- `crates/autopcb-router/src/gpu/active.rs` -- `ActiveRouter`
- `crates/autopcb-router/src/gpu/corridor.rs` -- `RoutingCorridor`, corridor mask

**Acceptance criteria**:
- ActiveRouter routes 100 nets through a corridor in < 1 second on GPU
- Via placement works within corridors (multi-layer routing)
- PathFinder negotiation resolves conflicts within 5 iterations
- Corridor mask correctly constrains BF search space
- All routing profiles produce valid solutions (DRC-clean)

### Phase 8: Spec + Viewer Integration for ActiveRoute

**Depends on**: Phase 7, Milestone 9 from `docs/plans/router/README.md`

**Files**:
- `crates/autopcb-spec/src/model.rs` -- `RoutingCorridorDecl` type
- `crates/autopcb-spec/src/compiler.rs` -- `compile_routing_corridor_decl()`
- `crates/autopcb-viewer/src/active_route.rs` -- viewer ActiveRoute UI
- `crates/autopcb-viewer/src/corridor_overlay.rs` -- corridor visualization

**Acceptance criteria**:
- `routing_corridor { ... }` parses and compiles in spec language
- Viewer allows net selection + route guide drawing + one-click routing
- Iteration snapshots display in real-time during routing
- Accept/reject/undo workflow works correctly
