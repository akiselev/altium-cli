# DFM Research: Acid Traps, Copper Slivers, and Geometric Manufacturing Checks

Research date: 2026-03-23
Context: autopcb-router with PathFinder (McMurchie-Ebeling), A* detailed routing,
45-degree corner optimization, rubber-banding, DRC with acute angle detection (45-degree threshold).

---

## Table of Contents

1. [Acid Traps](#1-acid-traps)
2. [Copper Slivers](#2-copper-slivers)
3. [Acute Angle Avoidance in A* Cost Functions](#3-acute-angle-avoidance-in-a-cost-functions)
4. [Teardrop Insertion](#4-teardrop-insertion)
5. [Minimum Annular Ring Enforcement During Routing](#5-minimum-annular-ring-enforcement-during-routing)
6. [Integration Recommendations for autopcb-router](#6-integration-recommendations-for-autopcb-router)

---

## 1. Acid Traps

### 1.1 Definition and Physics

An acid trap is a region in the copper layout where etchant solution becomes trapped
during the wet etching process, causing over-etching that can thin or sever traces.
The classic formation mechanism: when two traces meet at an acute angle (< 90 degrees),
the inner wedge-shaped pocket between them traps etchant, which continues to dissolve
copper after the surrounding area has been properly etched.

**Key insight from Siemens (Mentor) research**: Sharp corners are merely a *proxy* for
the real cause of acid traps -- any layout geometry that creates a small, nearly
closed-off region where etchant pools. An acid trap can form even without sharp corners
if trace spacing creates narrow channels or enclosed pockets. Conversely, a single
acute-angle trace bend in open space may not trap etchant at all.

### 1.2 Geometric Conditions

Acid traps form when:

1. **Acute trace junctions**: Two traces on the same net/layer meet at an interior
   angle < 90 degrees. The wedge between them traps etchant.

2. **Trace-to-pad acute angles**: A trace enters a pad at a sharp angle, creating a
   narrow wedge between the trace edge and the pad perimeter.

3. **Narrow enclosed channels**: Closely spaced traces or copper features create
   channels where etchant cannot flow freely, even without acute angles.

4. **Polygon pour pockets**: Copper fill zones with narrow re-entrant features.

```
    Acid trap at trace junction:

         ╲  angle < 90°
          ╲ ─────────────
           ╳  ← etchant trapped in this wedge
          ╱ ─────────────
         ╱

    Acid trap at pad entry:

         ╲
          ╲_____
          |     |  ← pad
          | pad |
          |_____|
              ↑
         etchant trapped
         in acute wedge
```

### 1.3 Angle Thresholds

| Context | Threshold | Source |
|---------|-----------|--------|
| Traditional rule of thumb | < 90 degrees interior angle | Industry standard |
| Altium Designer DRC default | 45 degrees minimum angle | Altium Manufacturing Rules |
| Conservative DFM | < 60 degrees flagged as warning | Cadence DesignTrue DFM |
| Modern manufacturing | < 30 degrees truly problematic | Cadence blog (2019) |

**Altium's acute angle rule** creates a contour from all primitives in a net (on the
same layer) and analyzes this contour for any vertices that create an angle smaller than
the configured limit. It has a "Check Tracks Only" option to restrict analysis to track
objects.

### 1.4 How Commercial Tools Handle Acid Traps

**Altium Designer:**
- Manufacturing rule: "Acute Angle" rule under Manufacturing rules category
- Creates a net contour per layer, scans for vertices below the angle threshold
- Applied during Online DRC, Batch DRC, interactive routing, and autorouting
- The autorouter avoids generating acute angles by construction (45-degree routing)
- Post-route DRC catches any remaining violations

**Cadence Allegro:**
- DesignTrue DFM module provides dedicated acid trap detection
- Checks minimum line-to-pad angle and minimum line-to-shape angle
- Recommends teardrops/fillets to eliminate acute angles at pad junctions
- DFM analysis runs separately from standard DRC

**Mentor/Siemens PADS (HyperLynx DRC):**
- DFM analysis identifies enclosed regions, not just angle measurements
- Checks for "nearly closed-off areas" where etchant could pool
- Treats the problem as a fluid-flow analysis, not purely geometric

### 1.5 Detection Algorithm

The most robust acid trap detection uses two complementary approaches:

**Approach A: Vertex angle scanning (what Altium and KiCad use)**

```
for each net N on each copper layer L:
    contour = build_outline(N, L)  // union of all trace/pad/via shapes
    for each vertex V in contour:
        angle = interior_angle_at(V)
        if angle < threshold:
            report_acid_trap(V, angle)
```

**Approach B: Minimum-width channel detection (what Mentor uses)**

```
for each copper region R on layer L:
    eroded = minkowski_erosion(R, circle(min_channel_width / 2))
    if eroded has disconnected fragments vs original topology:
        narrow_channels = R - minkowski_dilation(eroded, circle(min_channel_width / 2))
        for each narrow channel C:
            if C is enclosed on 3+ sides:
                report_acid_trap(C)
```

### 1.6 Prevention During Routing vs Post-Route

**During routing (proactive):**
- Route using 45-degree (octagonal) grid or curved traces
- Penalize acute angles in the A* cost function (see Section 3)
- Ensure pad entry angles are constrained (minimum pad entry angle)
- Use teardrops at all pad/via junctions (see Section 4)

**Post-route (reactive):**
- DRC/DFM check flags violations
- Auto-fix by inserting teardrops or adjusting trace geometry
- Re-route violating segments with angle constraints

### 1.7 Modern Manufacturing Context

Modern fabrication largely mitigates acid traps through:
- **Alkaline etching** instead of acid etching (most fabs have switched)
- **Photo-activated etching** with better uniformity
- **Plasma/dry etching** for fine-pitch designs
- **Tighter process control** in equipment

However, acid trap avoidance remains best practice because:
- Not all fabs use modern processes (especially budget/prototype houses)
- Even alkaline etchants can pool in extreme cases
- Acute angles cause SI issues (impedance discontinuities) independent of etching
- Many DFM checks still flag them, causing unnecessary review cycles

---

## 2. Copper Slivers

### 2.1 Definition

A copper sliver is a thin, narrow feature of copper that is either:
1. **A thin remnant** after etching: a narrow copper "whisker" that may partially
   detach during manufacturing, potentially causing shorts
2. **A thin void/gap** in copper: a narrow etched channel that may not etch
   completely, potentially failing to isolate traces

Both are caused by geometry that creates features narrower than the fab's minimum
feature width capability.

### 2.2 Common Formation Scenarios

1. **Acute polygon vertices**: Copper pour zones that come to a sharp point
2. **Near-parallel traces**: Two traces running very close together creating a
   narrow copper strip between them (in ground plane) or narrow gap
3. **Trace-to-pad proximity**: A trace passing very close to a pad, creating a
   thin sliver of copper or thin gap in a pour
4. **Overlapping shapes**: Boolean operations on copper regions producing thin
   artifacts

### 2.3 Width Thresholds

| Fab capability | Minimum copper feature | Minimum copper gap |
|---------------|----------------------|-------------------|
| Standard (1 oz) | 4 mil (0.1 mm) | 4 mil (0.1 mm) |
| Fine pitch | 3 mil (0.075 mm) | 3 mil (0.075 mm) |
| HDI | 2 mil (0.05 mm) | 2 mil (0.05 mm) |
| Ultra-fine | 1 mil (0.025 mm) | 1 mil (0.025 mm) |

**Solder mask slivers** have a separate, typically larger threshold: 4 mil (0.1 mm)
minimum per IPC-7351 guidelines.

### 2.4 Detection Algorithm (KiCad Reference Implementation)

KiCad's `drc_test_provider_sliver_checker.cpp` implements the state-of-the-art
open-source sliver detection. The algorithm:

**Parameters:**
- `m_SliverWidthTolerance`: minimum allowed width (configurable, in mm)
- `m_SliverAngleTolerance`: maximum acute angle for sliver vertices (in degrees)

**Core algorithm using law of cosines on polygon vertices:**

```
// Precompute threshold
cos_angle_tol = 2.0 * cos(DEG2RAD(angle_tolerance))
squared_width = width_tolerance^2

for each copper zone Z on each copper layer:
    polygon = Z.filled_polygon()
    for each vertex V[i] in polygon:
        // Skip degenerate micro-segments
        if distance(V[i-1], V[i]) < min_len:
            continue

        // Compute vectors from vertex to neighbors
        v_prior = V[i-1] - V[i]   // vector to previous vertex
        v_after = V[i+1] - V[i]   // vector to next vertex

        // Quick reject: if dot product <= 0, angle >= 90 degrees, not a sliver
        if v_prior.dot(v_after) <= 0:
            continue

        // Check if vertex is "locally inside" the polygon
        // (eliminates concave vertices that open outward)
        if not is_locally_inside(i-1, i+1):
            continue

        // Law of cosines check:
        // For triangle formed by V[i-1], V[i], V[i+1]:
        arm1 = squared_length(v_prior)
        arm2 = squared_length(v_after)
        opp  = squared_distance(V[i-1], V[i+1])

        cos_ang = abs((opp - arm1 - arm2) / (sqrt(arm1) * sqrt(arm2)))

        // Sliver detected: acute angle AND opposite side long enough
        if cos_ang > cos_angle_tol AND opp > squared_width:
            report_sliver(V[i])
```

The key insight: a sliver requires BOTH an acute angle AND the opposite side
(the "width" of the sliver at the tip) exceeding a minimum length. A tiny acute
triangle is not a sliver -- it must be elongated.

### 2.5 Avoidance During Routing

Autorouters avoid generating copper slivers by:

1. **Minimum clearance enforcement**: Maintaining trace-to-trace and trace-to-pad
   clearance above the minimum feature width prevents narrow gaps.

2. **Polygon pour minimum width**: Setting the copper pour's minimum feature width
   parameter to the fab's capability ensures no thin copper remnants.

3. **Trace width constraints**: Never routing traces narrower than minimum width.

4. **Thermal relief sizing**: Ensuring thermal relief spokes are wide enough.

Our router's grid-based approach inherently prevents many sliver scenarios because
the grid resolution enforces minimum spacing. However, post-route optimization
(rubber-banding, corner chamfering) can create slivers if not constrained.

---

## 3. Acute Angle Avoidance in A* Cost Functions

### 3.1 Current State in autopcb-router

Our router currently:
- Uses a grid-based A* with 8-way movement (diagonal support)
- Has direction penalty (1.5x for off-preferred-direction moves)
- Detects acute angles post-route in DRC (geometry.rs, 45-degree threshold)
- Converts corners in post-route optimization (corners.rs)

**Gap**: No in-routing penalty for configurations that would create acid traps.
Acute angles are detected after routing, not prevented during routing.

### 3.2 Direction Change (Bend) Cost

The standard approach in grid-based routers is to add a **bend penalty** to the
A* cost function whenever the path changes direction. This naturally discourages
excessive direction changes and, with proper tuning, prevents acute angles.

**Basic bend penalty:**

```
fn neighbor_cost(
    current: GridNode,
    neighbor: GridNode,
    parent_direction: Option<(i32, i32)>,  // direction we arrived at current
) -> f64 {
    let base_cost = if is_diagonal(current, neighbor) { SQRT_2 } else { 1.0 };
    let dir_penalty = direction_penalty(dx, dy, preferred);

    // NEW: bend penalty
    let bend_penalty = if let Some((pdx, pdy)) = parent_direction {
        let (ndx, ndy) = (neighbor.x as i32 - current.x as i32,
                          neighbor.y as i32 - current.y as i32);
        bend_cost(pdx, pdy, ndx, ndy)
    } else {
        1.0  // no parent = first step, no penalty
    };

    base_cost * dir_penalty * bend_penalty
}
```

### 3.3 Acute Angle Penalty Function

The key innovation is computing the **angle between the incoming direction and the
proposed outgoing direction**, then applying a cost multiplier that increases sharply
as the angle approaches 0 degrees (hairpin) or falls below the acid trap threshold.

```
/// Compute bend cost multiplier based on the angle between incoming and
/// outgoing directions.
///
/// Returns 1.0 for straight-ahead or gentle bends (>= threshold).
/// Returns a large penalty for acute angles (< threshold).
/// Returns infinity (or very large value) for angles below hard minimum.
fn bend_cost(
    in_dx: i32, in_dy: i32,   // incoming direction
    out_dx: i32, out_dy: i32,  // proposed outgoing direction
) -> f64 {
    // Compute angle between incoming and outgoing direction vectors.
    // Note: we want the angle between the CONTINUATION of the incoming
    // direction and the outgoing direction. The continuation is (-in_dx, -in_dy).
    let cont_dx = -in_dx as f64;
    let cont_dy = -in_dy as f64;
    let out_dx = out_dx as f64;
    let out_dy = out_dy as f64;

    let dot = cont_dx * out_dx + cont_dy * out_dy;
    let len1 = (cont_dx * cont_dx + cont_dy * cont_dy).sqrt();
    let len2 = (out_dx * out_dx + out_dy * out_dy).sqrt();

    if len1 < 1e-9 || len2 < 1e-9 {
        return 1.0;  // degenerate
    }

    let cos_theta = (dot / (len1 * len2)).clamp(-1.0, 1.0);
    let angle_deg = cos_theta.acos().to_degrees();

    // angle_deg is the angle between continuation and outgoing:
    //   180 = straight ahead (no bend)
    //   135 = 45-degree bend (gentle)
    //    90 = right-angle bend
    //    45 = acute acid-trap angle
    //     0 = hairpin/U-turn

    const ACID_TRAP_THRESHOLD: f64 = 90.0;  // configurable
    const HARD_MINIMUM: f64 = 45.0;
    const GENTLE_PENALTY: f64 = 1.2;   // slight cost for any bend
    const ACUTE_PENALTY: f64 = 5.0;    // strong cost for acid-trap angles
    const REJECT_PENALTY: f64 = 100.0; // near-prohibitive for hard minimum

    if angle_deg >= 180.0 - 1e-6 {
        1.0  // straight ahead
    } else if angle_deg >= ACID_TRAP_THRESHOLD {
        // Gentle bend: small penalty proportional to bend severity
        let t = (180.0 - angle_deg) / (180.0 - ACID_TRAP_THRESHOLD);
        1.0 + (GENTLE_PENALTY - 1.0) * t
    } else if angle_deg >= HARD_MINIMUM {
        // Acid trap zone: strong exponential penalty
        let t = (ACID_TRAP_THRESHOLD - angle_deg)
              / (ACID_TRAP_THRESHOLD - HARD_MINIMUM);
        ACUTE_PENALTY * (1.0 + t * 2.0)
    } else {
        // Below hard minimum: near-prohibitive
        REJECT_PENALTY
    }
}
```

**On 8-way grids, the possible bend angles are limited:**

| Direction change | Interior angle at junction |
|-----------------|--------------------------|
| Straight ahead | 180 degrees (no bend) |
| 45-degree turn | 135 degrees |
| 90-degree turn | 90 degrees |
| 135-degree turn | 45 degrees (acid trap!) |
| U-turn (180) | 0 degrees (hairpin) |

On an 8-way grid, the only way to get an acid trap angle is a 135-degree direction
change (3 octants). The cost function should assign high penalty to this case and
prohibitive penalty to U-turns.

### 3.4 Admissibility Considerations

Adding bend penalty to the A* cost function affects admissibility:

- The bend penalty is added to **g(n)** (actual accumulated cost), not h(n)
- The heuristic h(n) remains a lower bound (Manhattan distance + via estimate)
- Since the penalty only increases g(n), the heuristic is still admissible
- However, the search becomes **non-optimal** in the sense that we may not find
  the shortest path -- we find the path with lowest *weighted* cost
- This is acceptable: we want manufacturing-friendly paths, not shortest paths

**Alternative: Weighted A* (epsilon-admissible)**

For faster convergence with DFM awareness:
```
f(n) = g(n) + w * h(n)    where w >= 1.0
```
With w = 1.1 to 1.5, the search runs faster and the bend penalty has
proportionally more influence on path selection.

### 3.5 State-of-the-Art: Direction-Aware A* with History

The most effective approach combines bend penalties with PathFinder's
negotiation-based history:

```
// Extended A* cost for a node n reached from parent p:
cost(n) = base_distance(p, n)
        * direction_penalty(dx, dy, preferred_dir)
        * bend_cost(parent_dir, (dx, dy))
        + hist_weight * (history[n] + edge_history[p->n])
        + pres_fac * max(0, usage[n] - capacity[n])
```

The `edge_history[p->n]` term is critical: it penalizes not just node congestion
but specific *directed edges* in the routing graph. This allows the history to
"remember" that a particular direction through a node was problematic (e.g.,
because it created an acid trap with a neighboring net's trace).

### 3.6 Pad-Entry Angle Constraint

A separate but related concern: the angle at which a trace enters a pad. This is
modeled as a constraint on the final A* step when reaching a pad terminal:

```
fn pad_entry_cost(
    approach_dir: (i32, i32),
    pad_center: GridNode,
    pad_preferred_entry: Option<(f64, f64)>,  // preferred approach direction
) -> f64 {
    if let Some((pref_dx, pref_dy)) = pad_preferred_entry {
        let dot = approach_dir.0 as f64 * pref_dx
                + approach_dir.1 as f64 * pref_dy;
        let cos_angle = dot / ((approach_dir.0.pow(2) + approach_dir.1.pow(2)) as f64).sqrt()
                            / (pref_dx * pref_dx + pref_dy * pref_dy).sqrt();
        let angle = cos_angle.acos().to_degrees();

        // Penalty for approaching pad from non-preferred direction
        if angle > 45.0 { 2.0 } else { 1.0 }
    } else {
        1.0
    }
}
```

### 3.7 Academic References

- McMurchie & Ebeling, "PathFinder: A Negotiation-Based Performance-Driven Router
  for FPGAs" (FPGA 1995) -- foundational negotiation-based routing with history costs
- Hart, Nilsson & Raphael, "A Formal Basis for the Heuristic Determination of
  Minimum Cost Paths" (IEEE SSC 1968) -- original A* algorithm
- "A Novel Global Routing Algorithm for PCBs Based on Triangular Grid" (Electronics
  2023, MDPI) -- uses Delaunay triangulation to avoid acute angles by construction
- "A Unified PCB Routing Algorithm With Complicated Constraints and Differential
  Pairs" (ASP-DAC 2021) -- manufacturing-constraint-aware routing
- "Performance-Driven Multi-Layer General Area Routing for PCB/MCM Designs"
  (DAC 1998) -- multi-objective cost functions including bend penalties

---

## 4. Teardrop Insertion

### 4.1 Purpose

Teardrops are filleted copper additions at trace-to-pad and trace-to-via junctions
that serve multiple purposes:

1. **Manufacturing yield**: Eliminates acute angles at junctions (acid trap prevention)
2. **Mechanical strength**: Spreads stress over larger area at the junction
3. **Drill registration tolerance**: Compensates for drill-to-pad misalignment
4. **Signal integrity**: Provides smoother impedance transition at junctions
5. **Thermal relief**: Better heat spreading at solder joints

### 4.2 Geometric Parameters

#### Altium Designer defaults:

| Junction type | Length | Width |
|--------------|--------|-------|
| Via / TH pad | 30% of pad diameter | 70% of pad diameter |
| SMD pad | 100% of trace width | 200% of trace width |
| Track-to-track transition | 100% of trace width | N/A |
| T-junction | 300% of primary track width | 100% of primary track width |

#### KiCad parameters:

| Parameter | Default | Description |
|-----------|---------|-------------|
| `m_BestLengthRatio` | 0.5 | Length as ratio of pad/via diameter |
| `m_BestWidthRatio` | 1.0 | Width as ratio of pad/via diameter |
| `m_TdMaxLen` | -1 (disabled) | Maximum absolute teardrop length |
| `m_TdMaxWidth` | -1 (disabled) | Maximum absolute teardrop width |
| `m_CurvedEdges` | false | Use Bezier curves vs straight edges |
| `m_AllowUseTwoTracks` | false | Extend teardrop across two track segments |

### 4.3 Teardrop Construction Algorithm

#### Method 1: Straight-edge teardrop (simple, fast)

```
fn create_straight_teardrop(
    pad_center: Point,
    pad_radius: f64,
    trace_start: Point,   // point on trace closest to pad
    trace_width: f64,
    length_ratio: f64,    // teardrop length / pad diameter
    width_ratio: f64,     // teardrop width / pad diameter
) -> Polygon {
    let pad_diameter = pad_radius * 2.0;
    let td_length = pad_diameter * length_ratio;
    let td_width = pad_diameter * width_ratio;

    // Direction vector from pad center to trace
    let dir = (trace_start - pad_center).normalize();
    let perp = dir.perpendicular();

    // Teardrop apex: where it meets the trace
    let apex = pad_center + dir * (pad_radius + td_length);

    // Teardrop base: where it meets the pad perimeter
    let base_left  = pad_center + perp * (td_width / 2.0);
    let base_right = pad_center - perp * (td_width / 2.0);

    // Clip base points to pad perimeter
    let base_left  = clip_to_circle(base_left, pad_center, pad_radius);
    let base_right = clip_to_circle(base_right, pad_center, pad_radius);

    // Construct polygon: pad-arc from base_left to base_right,
    // then straight edges to apex
    Polygon::new(vec![base_left, apex, base_right])
}
```

#### Method 2: Curved teardrop (Bezier, higher quality)

KiCad and Altium both support curved teardrops using cubic Bezier curves.

```
fn create_curved_teardrop(
    pad_center: Point,
    pad_radius: f64,
    trace_point: Point,   // point on trace where teardrop ends
    trace_dir: Vector,    // trace direction at that point
    trace_width: f64,
    length_ratio: f64,
    width_ratio: f64,
) -> Polygon {
    let pad_diameter = pad_radius * 2.0;
    let td_length = pad_diameter * length_ratio;
    let td_width = (pad_diameter * width_ratio).min(trace_width * 3.0);

    // Direction from pad to trace
    let to_trace = (trace_point - pad_center).normalize();
    let perp = to_trace.perpendicular();

    // Connection points on pad perimeter
    let pad_left  = pad_center + perp * pad_radius * 0.7;
    let pad_right = pad_center - perp * pad_radius * 0.7;

    // Clip to pad circle
    let pad_left  = project_to_circle(pad_left, pad_center, pad_radius);
    let pad_right = project_to_circle(pad_right, pad_center, pad_radius);

    // Trace connection points (half trace width from center)
    let trace_left  = trace_point + perp * (trace_width / 2.0);
    let trace_right = trace_point - perp * (trace_width / 2.0);

    // Cubic Bezier for each side:
    // Control points ensure tangential continuity at both ends
    //
    // At pad end: control point is tangent to pad circle
    // At trace end: control point is along trace direction
    let weight = td_length * 0.4;  // Bezier weight (experimentally tuned)

    let ctrl_pad_left = pad_left + tangent_at(pad_left, pad_center) * weight;
    let ctrl_trace_left = trace_left - trace_dir * weight;

    let ctrl_pad_right = pad_right + tangent_at(pad_right, pad_center) * weight;
    let ctrl_trace_right = trace_right - trace_dir * weight;

    // Generate points along each Bezier curve
    let left_curve = cubic_bezier(pad_left, ctrl_pad_left,
                                  ctrl_trace_left, trace_left, 8);
    let right_curve = cubic_bezier(pad_right, ctrl_pad_right,
                                   ctrl_trace_right, trace_right, 8);

    // Combine: pad arc + left curve + trace edge + right curve (reversed)
    let mut points = Vec::new();
    points.extend(arc_points(pad_left, pad_right, pad_center, pad_radius));
    points.extend(left_curve);
    points.extend(right_curve.into_iter().rev());
    Polygon::new(points)
}

fn cubic_bezier(p0: Point, p1: Point, p2: Point, p3: Point, steps: usize) -> Vec<Point> {
    (0..=steps)
        .map(|i| {
            let t = i as f64 / steps as f64;
            let u = 1.0 - t;
            // B(t) = (1-t)^3 * P0 + 3(1-t)^2*t * P1 + 3(1-t)*t^2 * P2 + t^3 * P3
            p0 * (u * u * u)
                + p1 * (3.0 * u * u * t)
                + p2 * (3.0 * u * t * t)
                + p3 * (t * t * t)
        })
        .collect()
}
```

**Critical implementation detail** (from KiCad "Melting KiCad" analysis): When a
trace approaches a pad off-center, the Bezier weights on the two sides must be
asymmetric. If the trace hits the pad at an angle, the curve on the acute side
needs a tighter weight and the curve on the obtuse side needs a looser weight to
maintain smooth tangential continuity.

### 4.4 When to Insert Teardrops

**Post-route insertion (most common, recommended for our router):**

Advantages:
- Does not complicate the A* search
- Can be tuned independently of routing
- Works with any routing algorithm
- Easy to add/remove teardrops without re-routing

```
fn insert_teardrops(solution: &mut RouteSolution, ir: &PcbIr, config: &TeardropConfig) {
    for routed_net in solution.nets.values_mut() {
        // 1. Find all via-to-trace junctions
        for via in &routed_net.vias {
            for seg in &routed_net.segments {
                if connects_to_via(seg, via) {
                    let teardrop = create_teardrop_at_via(via, seg, config);
                    if !violates_clearance(teardrop, solution, ir) {
                        routed_net.teardrops.push(teardrop);
                    }
                }
            }
        }

        // 2. Find all pad-to-trace junctions
        for pad in pads_for_net(ir, routed_net.net_id) {
            for seg in &routed_net.segments {
                if connects_to_pad(seg, pad) {
                    let teardrop = create_teardrop_at_pad(pad, seg, config);
                    if !violates_clearance(teardrop, solution, ir) {
                        routed_net.teardrops.push(teardrop);
                    }
                }
            }
        }

        // 3. Find track width transitions
        for pair in consecutive_segments(routed_net) {
            if pair.0.width_mm != pair.1.width_mm {
                let teardrop = create_teardrop_at_transition(pair, config);
                if !violates_clearance(teardrop, solution, ir) {
                    routed_net.teardrops.push(teardrop);
                }
            }
        }
    }
}
```

**During-route insertion (advanced, used by some commercial tools):**

Some routers pre-allocate space for teardrops by inflating the via/pad obstacle
size during A* search. This ensures there is always room for a teardrop without
needing to check clearance post-hoc:

```
// During obstacle map construction:
fn effective_via_radius(via: &Via, config: &TeardropConfig) -> f64 {
    let base_radius = via.drill_mm / 2.0 + via.annular_ring_mm;
    let teardrop_extent = base_radius * 2.0 * config.via_length_ratio;
    base_radius + teardrop_extent  // inflate obstacle to reserve teardrop space
}
```

### 4.5 Teardrop Size Auto-Adjustment

Altium's "Adjust teardrop size" option automatically shrinks teardrops to fit
within design rule clearances. The algorithm:

```
fn adjust_teardrop_size(
    teardrop: &mut Teardrop,
    clearance_checker: &ClearanceChecker,
    min_scale: f64,  // minimum acceptable scale (e.g., 0.3)
) -> bool {
    let mut scale = 1.0;
    while scale >= min_scale {
        teardrop.set_scale(scale);
        if !clearance_checker.violates(teardrop) {
            return true;  // teardrop fits
        }
        scale -= 0.1;
    }
    false  // teardrop cannot fit even at minimum scale
}
```

---

## 5. Minimum Annular Ring Enforcement During Routing

### 5.1 The Problem

Annular ring = (pad outer diameter - drill diameter) / 2. If the annular ring is
too small, the drill hole may break through the pad copper, causing an open circuit.

During routing, the via's total diameter (drill + 2 * annular ring) determines how
much board space it consumes. The router must ensure:

```
via_outer_diameter = drill_diameter + 2 * min_annular_ring
via_clearance_radius = via_outer_diameter / 2 + clearance
```

### 5.2 How Commercial Routers Enforce This

**Altium Designer:**
- The RoutingViaStyle rule specifies hole min/max and via diameter min/max
- The MinimumAnnularRing rule provides an independent manufacturing check
- During interactive routing, the via size is constrained by BOTH rules
- The autorouter uses the RoutingViaStyle to determine via size
- Post-route DRC checks MinimumAnnularRing independently

**Key relationship:**
```
via_diameter >= drill_diameter + 2 * min_annular_ring
```

If the routing rule specifies a via diameter of 0.5mm and drill of 0.3mm, the
annular ring is (0.5 - 0.3) / 2 = 0.1mm. The DRC then checks this against
the MinimumAnnularRing rule.

### 5.3 Enforcement During A* Routing

In our grid-based router, via placement is a layer-change in the A* graph.
Enforcement happens at two levels:

**Level 1: Via obstacle sizing (pre-routing)**

When building the obstacle map, each potential via location must have clearance
from all obstacles for the full via outer diameter:

```
fn can_place_via(
    grid: &Grid,
    node: GridNode,
    drill_mm: f64,
    annular_ring_mm: f64,
    clearance_mm: f64,
) -> bool {
    let via_radius_mm = drill_mm / 2.0 + annular_ring_mm;
    let total_exclusion_mm = via_radius_mm + clearance_mm;
    let exclusion_cells = (total_exclusion_mm / grid.resolution_mm).ceil() as u32;

    // Check all layers the via spans
    for layer in via_from_layer..=via_to_layer {
        for dx in -(exclusion_cells as i32)..=(exclusion_cells as i32) {
            for dy in -(exclusion_cells as i32)..=(exclusion_cells as i32) {
                let check = GridNode {
                    x: (node.x as i32 + dx) as u32,
                    y: (node.y as i32 + dy) as u32,
                    layer,
                };
                if grid.is_blocked(check) {
                    return false;
                }
            }
        }
    }
    true
}
```

**Level 2: Via cost includes annular ring awareness**

The via cost function should account for the annular ring constraint:

```
fn via_cost(
    node: GridNode,
    policy: &RoutingPolicy,
    net_class: Option<&str>,
) -> f64 {
    let via_bounds = policy.via_bounds_for(net_class);
    let base_cost = policy.via_cost_base;

    // Additional cost if near minimum annular ring
    // (encourages router to use locations with more clearance)
    let margin = available_clearance(node) - (via_bounds.hole_min_mm / 2.0
                                             + via_bounds.annular_ring_min_mm);
    let margin_penalty = if margin < 0.1 { 2.0 } else { 1.0 };

    base_cost * margin_penalty
}
```

### 5.4 Current State in autopcb-router

Our router already has:
- `DrcViaBounds` with `annular_ring_min_mm` (default 0.05mm)
- Via DRC checking in `drc/via.rs` that validates annular ring post-route
- Via cost model in `detailed/via_cost.rs`

What needs to be added:
- Pre-route via obstacle inflation to account for annular ring + clearance
- Via placement validation during A* that checks annular ring constraints
- Per-net-class via sizing during routing (not just DRC)

---

## 6. Integration Recommendations for autopcb-router

### 6.1 Priority Order

1. **Bend cost in A* (acid trap prevention)** -- Highest impact, moderate effort
2. **Teardrop insertion post-route** -- High impact, moderate effort
3. **Annular ring enforcement during routing** -- Already partially implemented
4. **Copper sliver detection in DRC** -- Medium impact, implement in DRC pass
5. **Acid trap detection (enclosed region analysis)** -- Lower priority, complex

### 6.2 Concrete Implementation Plan

#### Phase 1: Bend Cost in A*

Modify `detailed/astar.rs` to track parent direction in the A* state and add
bend cost to the neighbor cost computation:

```rust
// In the A* node state, add:
pub struct AStarNode {
    pub grid_node: GridNode,
    pub parent_direction: Option<(i32, i32)>,  // NEW
    pub g_cost: f64,
    pub f_cost: f64,
}

// In neighbor expansion:
let bend_penalty = compute_bend_penalty(
    node.parent_direction,
    (dx, dy),
    config.acid_trap_threshold_deg,  // NEW config field
);
let step_cost = base_cost * dir_penalty * bend_penalty;
```

**Config additions** (`config.rs`):
```rust
/// Minimum interior angle before acid trap penalty applies (degrees).
/// Default 90.0 (penalize angles below 90 degrees).
pub acid_trap_threshold_deg: f64,

/// Cost multiplier for bends that create acid trap angles.
/// Default 5.0.
pub acid_trap_penalty: f64,
```

#### Phase 2: Teardrop Insertion

Add `optimize/teardrops.rs` as a new post-route optimization pass:

```rust
pub struct TeardropConfig {
    pub enabled: bool,
    pub via_length_ratio: f64,     // default 0.3
    pub via_width_ratio: f64,      // default 0.7
    pub pad_length_ratio: f64,     // default 0.5
    pub pad_width_ratio: f64,      // default 1.0
    pub curved: bool,              // default false
    pub force: bool,               // default false (skip if DRC violation)
}

pub fn insert_teardrops(
    solution: &mut RouteSolution,
    ir: &PcbIr,
    config: &TeardropConfig,
    clearance_mm: f64,
) -> Vec<Teardrop> { ... }
```

Insert this as pass 4 in `optimize/mod.rs`, after rubber-banding.

#### Phase 3: Copper Sliver DRC

Add `drc/sliver.rs` implementing the KiCad-style polygon vertex analysis:

```rust
pub fn check_copper_slivers(
    solution: &RouteSolution,
    ir: &PcbIr,
    policy: &DrcPolicy,
) -> Vec<DrcViolation> {
    // For each copper layer:
    //   1. Build union polygon of all copper (traces, pads, pours)
    //   2. Scan polygon vertices for acute angles with long opposite sides
    //   3. Report slivers
}
```

**Policy additions:**
```rust
pub sliver_width_tolerance_mm: f64,    // default 0.1 (4 mil)
pub sliver_angle_tolerance_deg: f64,   // default 20.0
```

#### Phase 4: Enhanced Acid Trap Detection

Add `drc/acid_trap.rs` with the enclosed-region analysis:

```rust
pub fn check_acid_traps(
    solution: &RouteSolution,
    ir: &PcbIr,
    policy: &DrcPolicy,
) -> Vec<DrcViolation> {
    // For each net on each layer:
    //   1. Build contour from all primitives
    //   2. Scan for vertices with interior angle < threshold
    //   3. Also check for narrow enclosed channels (Minkowski erosion)
}
```

### 6.3 Grid Resolution Implications

Our default grid resolution of 0.25mm means:
- Minimum trace spacing is 0.25mm (10 mil) -- above standard fab minimums
- 8-way movement produces angles in multiples of 45 degrees
- Acid trap angles on-grid are limited to 45-degree interior angles (135-degree turns)
- Post-route rubber-banding/corner conversion can create off-grid angles

**Critical**: The post-route optimization passes (rubber-banding, corner conversion)
must be acid-trap-aware. Currently `rubber_band.rs` pulls vertices toward shorter
paths without checking for acute angles. It should either:
1. Reject vertex movements that create angles below the threshold, or
2. Run a post-optimization angle check and fix violations

### 6.4 Testing Strategy

1. **Unit tests for bend_cost()**: Verify penalty values for all 8-way grid angles
2. **Unit tests for teardrop geometry**: Round pad, rectangular pad, off-center approach
3. **Integration test**: Route a known board, verify no acid traps in solution
4. **Regression test**: Compare route quality metrics (total length, via count, DRC
   violations) before and after bend penalty to ensure no significant quality loss

---

## Sources

### Acid Traps
- [Cadence: Are Acid Traps Still a Problem for PCBs?](https://resources.pcb.cadence.com/blog/are-acid-traps-still-a-problem-for-pcbs-in-2019-2)
- [Cadence: Embrace Optimal Etchant Usage by Fabricators](https://resources.pcb.cadence.com/blog/2020-acid-traps-embrace-optimal-etchant-usage-by-fabricators)
- [Siemens: 4 Less Obvious PCB DFM Violations](https://blogs.sw.siemens.com/pcbflow/2020/03/16/4-less-obvious-pcb-dfm-violations/)
- [PCBSync: Acid Traps in PCB Design](https://pcbsync.com/acid-traps-pcb/)
- [NextPCB: Acid Traps](https://www.nextpcb.com/blog/acid-traps)
- [Altium: Manufacturing Rule - Acute Angle](https://www.altium.com/documentation/altium-designer/pcb-manufacturing-rule-acute-angle)

### Copper Slivers
- [KiCad: drc_test_provider_sliver_checker.cpp](https://docs.kicad.org/doxygen/drc__test__provider__sliver__checker_8cpp_source.html)
- [Cadence: How to Detect and Resolve Copper Void Slivers](https://community.cadence.com/cadence_blogs_8/b/pcb/posts/copper-void-slivers)
- [Cadence: Minimum Solder Mask Sliver](https://resources.pcb.cadence.com/blog/2023-minimum-solder-mask-sliver-and-pcb-design)
- [Numerical Innovations: Advanced DFM Checks](https://www.numericalinnovations.com/advanced-dfm-checks)
- [Altium: Manufacturing Rule Types](https://www.altium.com/documentation/altium-designer/pcb-manufacturing-rules)

### A* Cost Functions and Routing Algorithms
- [McMurchie & Ebeling: PathFinder (FPGA 1995)](https://www.semanticscholar.org/paper/PathFinder:-A-Negotiation-Based-Performance-Driven-McMurchie-Ebeling/45b0d141e847855f149b175abbc371aeb4b80cbb)
- [TinyComputers: The Mathematics of PCB Trace Routing](https://tinycomputers.io/posts/the-mathematics-of-pcb-trace-routing.html)
- [MDPI: Novel Global Routing Algorithm Based on Triangular Grid](https://www.mdpi.com/2079-9292/12/24/4942)
- [ACM: Unified PCB Routing with Complicated Constraints](https://dl.acm.org/doi/10.1145/3394885.3431568)
- [ACM: Performance-Driven Multi-Layer Routing for PCB/MCM](https://dl.acm.org/doi/10.1145/277044.277144)
- [Blog: Building a Grid-based PCB Autorouter](https://blog.autorouting.com/p/building-a-grid-based-pcb-autorouter)
- [OrthoRoute: GPU-accelerated PCB autorouter](https://bbenchoff.github.io/pages/OrthoRoute.html)

### Teardrop Insertion
- [Altium: Teardrops Documentation](https://www.altium.com/documentation/altium-designer/pcb-dlg-teardropoptionsformteardrops-ad?version=22)
- [KiCad: TEARDROP_PARAMETERS Class](https://docs.kicad.org/doxygen/classTEARDROP__PARAMETERS.html)
- [KiCad: teardrop.cpp Source](https://docs.kicad.org/doxygen/teardrop_8cpp_source.html)
- [KiCad: teardrop_utils.cpp Source](https://gitlab.com/kicad/code/kicad/-/blob/d51c7372b4f203e2fa0a6a30f6312e896f02fd30/pcbnew/teardrop/teardrop_utils.cpp)
- [Mitxela: Melting KiCad (teardrop deep dive)](https://mitxela.com/projects/melting_kicad)
- [Cadence: PCB 101 Teardrops](https://resources.pcb.cadence.com/blog/pcb101-include-teardrops-in-your-designs-to-save-your-tears-later)
- [eCADSTAR: PCB Teardrops](https://www.ecadstar.com/en/blog/pcb-teardrops/)
- [EasyEDA Pro: Teardrop](https://prodocs.easyeda.com/en/pcb/tools-teardrop/)

### Annular Ring
- [Altium: Minimum Annular Ring Rule](https://www.altium.com/documentation/altium-designer/pcb-manufacturing-rule-minimum-annular-ring)
- [Cadence: PCB Minimum Annular Ring Formula](https://resources.pcb.cadence.com/blog/2023-pcb-minimum-annular-ring-formula-and-guidelines)

### KiCad DRC Source
- [KiCad: drc_test_provider_track_angle.cpp](https://docs.kicad.org/doxygen/drc__test__provider__track__angle_8cpp_source.html)
- [KiCad: DRC_ENGINE Class](https://docs.kicad.org/doxygen/classDRC__ENGINE.html)
- [FreeRouting: GitHub Repository](https://github.com/freerouting/freerouting)

### General DFM
- [Altium: Preventing Top DFM Errors](https://resources.altium.com/p/preventing-top-dfm-errors-your-pcb-design)
- [Altium: Routing Rule Types](https://www.altium.com/documentation/altium-designer/pcb-routing-rules)
- [KiCad: PCB Editor Documentation](https://docs.kicad.org/9.0/en/pcbnew/pcbnew.html)
