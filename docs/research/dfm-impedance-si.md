# Impedance-Controlled Routing and Signal Integrity for Manufacturing

Research notes for integrating impedance control, signal integrity awareness, and
DFM (Design for Manufacturing) considerations into the autopcb-router.

**Context**: We have a grid-based autorouter with PathFinder (McMurchie & Ebeling 1995)
negotiated congestion, A* detailed routing, differential pair support (gap/skew checking),
and layer-preferred-direction bias. This document surveys the state of the art and
proposes concrete integration strategies.

---

## Table of Contents

1. [Impedance-Controlled Routing in Autorouters](#1-impedance-controlled-routing-in-autorouters)
2. [Reference Plane Awareness](#2-reference-plane-awareness)
3. [Return Path Optimization](#3-return-path-optimization)
4. [Crosstalk Minimization in Routing Cost Functions](#4-crosstalk-minimization-in-routing-cost-functions)
5. [Length-Matched Routing for Manufacturing](#5-length-matched-routing-for-manufacturing)
6. [Via Stub Management](#6-via-stub-management)
7. [Integration Strategy for autopcb-router](#7-integration-strategy-for-autopcb-router)
8. [References](#8-references)

---

## 1. Impedance-Controlled Routing in Autorouters

### 1.1 Fundamentals

Controlled impedance routing ensures that PCB traces meet a target characteristic
impedance (typically 50 ohm single-ended, 90-100 ohm differential) by coordinating
trace geometry with the PCB layer stackup. The impedance of a transmission line on a
PCB is determined by:

- **Trace width** (w)
- **Dielectric height** to reference plane (h)
- **Dielectric constant** (Er / Dk) of the substrate material
- **Trace thickness** (t)
- **Copper roughness** and etch factor

Different layer positions produce different transmission line structures:

| Structure | Description | Typical Use |
|-----------|-------------|-------------|
| **Microstrip** | Trace on outer layer, one reference plane below | Top/bottom signal layers |
| **Embedded microstrip** | Outer trace covered by soldermask or prepreg | Soldermask-covered outer traces |
| **Symmetric stripline** | Inner trace equidistant between two reference planes | Inner signal layers |
| **Asymmetric stripline** | Inner trace at different distances from upper/lower planes | Inner layers with unequal dielectrics |

### 1.2 Impedance Calculation: Wadell's Equations

The industry-standard analytical formulas come from Brian C. Wadell's *Transmission
Line Design Handbook* (1991). For microstrip:

```
Z0 = (87 / sqrt(Er + 1.41)) * ln(5.98 * h / (0.8 * w + t))
```

Where:
- Z0 = characteristic impedance (ohms)
- Er = relative dielectric constant of substrate
- h = dielectric height (distance from trace to reference plane)
- w = trace width
- t = trace thickness

This simplified formula has < 1% error for typical PCB geometries. The more accurate
Hammerstad & Jensen formulation uses effective dielectric constant and effective width
corrections:

```
Z0 = (eta0 / (2*pi*sqrt(2)*sqrt(Er+1))) * ln(1 + 4*(h/w_eff) * (X1 + X2))
```

For **differential pairs** (edge-coupled microstrip), the formula requires 67
interrelated equations including elliptical integrals (Wadell). A practical
approximation for edge-coupled microstrip differential impedance:

```
Zdiff = 2 * Z0 * (1 - 0.48 * exp(-0.96 * s / h))
```

Where s = edge-to-edge spacing between the two traces.

Modern EDA tools (Altium, Cadence) use 2D field solvers (Method of Moments, e.g.,
Simberian SFS) rather than analytical formulas for production accuracy. These solvers
mesh the conductor and dielectric boundaries, solving for frequency-dependent RLGC
matrices, and provide impedance, delay, inductance, and capacitance per unit length.

### 1.3 How Commercial Autorouters Handle Impedance

**The core mechanism**: Impedance profiles are defined in the layer stackup manager.
For each signal layer, a field solver computes the trace width that achieves the target
impedance given that layer's dielectric height, Er, and reference plane arrangement.
The result is a **per-layer width table** that maps each impedance profile to a
specific trace width on each routable layer.

**Altium Designer's approach** (representative of the industry):

1. User defines impedance profiles in the Layer Stack Manager's Impedance tab:
   - Type: Single / Differential / Single-Coplanar / Differential-Coplanar
   - Target impedance and tolerance (e.g., 50 ohm +/- 10%)
   - Reference layers (top and bottom plane for each signal layer)
2. The Simbeor SFS field solver computes required trace width per signal layer
3. A Routing Width design rule with "Use Impedance Profile" enabled locks the
   Preferred Width to the calculated value per layer
4. **During routing**, when the user (or autorouter) changes layers, the trace width
   **automatically adjusts** to the width needed for that layer's impedance profile
5. Min Width and Max Width can be separately constrained for manufacturing tolerance

**Key insight for our router**: The impedance calculation is a **pre-routing step**
that produces a per-layer width lookup table. The router itself does not solve
electromagnetic equations during pathfinding. It simply applies the correct width
for the current layer.

### 1.4 Implementation Strategy for Our Router

Our router needs:

1. **Impedance profile data structure** in `RoutingConfig` or `PcbIr`:
   ```
   ImpedanceProfile {
       name: String,
       target_impedance_ohm: f64,
       tolerance_percent: f64,
       profile_type: Single | Differential | SingleCoplanar | DifferentialCoplanar,
       layer_widths: BTreeMap<LayerId, f64>,  // pre-computed width per layer (mm)
   }
   ```

2. **Net-to-profile mapping**: Each net or net class maps to an impedance profile.
   Nets without an impedance profile use the default trace width.

3. **Width-aware grid routing**: When the A* router expands a node on a given layer,
   it must use the width from the impedance profile for that net on that layer. This
   affects:
   - **Clearance inflation**: Wider traces need more obstacle clearance cells
   - **Grid occupancy**: A wider trace may occupy multiple adjacent grid cells
   - **Via transitions**: Changing layers changes the trace width, which may require
     tapered transitions (neckdown/neckup) at the via pad

4. **Layer preference for impedance**: Some impedance targets may only be achievable
   on certain layers (e.g., 50 ohm may require a specific h/w ratio only available
   on inner layers). The cost function should prefer layers where the target impedance
   is achievable within manufacturing tolerance.

### 1.5 Width Variation Across Layers

A critical manufacturing insight: **the same trace on different layers will have
different widths** to maintain the same impedance. For example, on a 6-layer board:

| Layer | Type | Dielectric h | Er | Width for 50 ohm |
|-------|------|-------------|-----|-------------------|
| L1 (Top) | Microstrip | 4 mil | 4.2 | ~7 mil |
| L2 (Inner) | Stripline | 5 mil | 4.2 | ~5 mil |
| L5 (Inner) | Stripline | 5 mil | 4.2 | ~5 mil |
| L6 (Bottom) | Microstrip | 4 mil | 4.2 | ~7 mil |

Stripline traces are typically narrower than microstrip for the same impedance because
they are surrounded by dielectric on both sides (higher effective Er).

---

## 2. Reference Plane Awareness

### 2.1 Why Reference Planes Matter

Every signal trace forms a transmission line with its reference plane (the nearest
copper plane, usually ground or power). The return current flows on the surface of
the reference plane directly beneath the signal trace. A continuous, unbroken
reference plane ensures:

- **Controlled impedance**: Impedance depends on geometry to the reference plane
- **Low loop inductance**: Tight signal-return coupling minimizes radiated EMI
- **Predictable crosstalk**: Consistent plane height reduces coupling variations

### 2.2 Plane Splits and Voids: The Problem

When a signal trace routes over a gap, split, or void in its reference plane:

1. **Impedance spike**: The trace impedance increases sharply over the gap (visible
   as a spike in TDR measurements), causing signal reflections
2. **Return path disruption**: The return current must detour around the gap,
   creating a large current loop that radiates EMI
3. **Increased crosstalk**: Multiple signals sharing the same detoured return path
   couple through common impedance
4. **Ground bounce**: Higher inductance return path causes ground noise

### 2.3 Detection Algorithms in EDA Tools

Modern EDA tools implement "nets crossing gaps" DRC rules:

1. **Plane gap geometry extraction**: For each copper plane layer, compute the
   polygon geometry of the plane, including all antipads, thermal relief gaps,
   routing clearances, and explicit splits
2. **Trace projection**: For each signal trace segment on an adjacent signal layer,
   project it onto the reference plane layer
3. **Gap intersection test**: Check if the projected trace path crosses any gap or
   void in the reference plane polygon
4. **Reporting**: Flag violations with the trace net name, the gap location, and
   the affected reference plane

**Algorithmic approach** for a grid-based router:

```
For each signal layer L:
  ref_plane = adjacent_plane_layer(L)  // the ground/power plane
  For each grid cell (x, y) on layer L:
    project (x, y) onto ref_plane
    if ref_plane has no copper at (x, y):
      mark cell as "broken_reference" with high SI penalty
```

This can be precomputed into a per-layer **reference quality bitmap** during
workspace construction.

### 2.4 Plane-Aware Cost Function

The router's A* cost function should include a reference plane continuity term:

```
C(n) = base_cost * dir_penalty * corridor_penalty
     + ref_penalty(n)           // NEW: penalty for broken reference
     + hist_weight * history[n]
     + pres_fac * max(0, usage[n] - 1)
```

Where `ref_penalty(n)` returns:
- **0.0** if the grid cell has continuous reference plane copper beneath it
- **HIGH_PENALTY** (e.g., 50.0-100.0) if the cell is over a plane void/split
- Only applied to nets marked as impedance-controlled or SI-sensitive

This discourages but does not absolutely prohibit routing over plane gaps (the DRC
will catch any violations post-route).

### 2.5 Layer Transition and Reference Plane Changes

When a signal transitions layers via a via, the reference plane may change:

| From Layer | To Layer | Reference Change | Action Required |
|------------|----------|-----------------|-----------------|
| Top (ref: GND L2) | Inner (ref: GND L2) | Same plane | No action |
| Top (ref: GND L2) | Inner (ref: PWR L3) | Different plane | Stitching via needed |
| Inner (ref: GND L2) | Inner (ref: GND L5) | Same net, different layer | Stitching via recommended |

When the reference plane changes, the return current must also change planes. Without
a nearby stitching via or capacitor, the return current has no local path and must
travel a long distance to find a connection between the planes.

**Algorithm for detecting reference plane changes**:
```
on_via_placement(signal_layer_from, signal_layer_to):
    ref_from = reference_plane(signal_layer_from)
    ref_to = reference_plane(signal_layer_to)
    if ref_from.net != ref_to.net:
        flag_reference_change(via_location, ref_from, ref_to)
        // Add cost penalty or require stitching via
```

---

## 3. Return Path Optimization

### 3.1 Return Current Physics

At frequencies above ~1 MHz, return current follows the path of least impedance
(not least resistance), which means it flows directly beneath the signal trace on
the reference plane. This tight coupling minimizes loop area and thus minimizes:

- Radiated EMI (proportional to loop area * frequency^2)
- Susceptibility to external interference
- Crosstalk via common impedance coupling

### 3.2 Via Transitions: The Return Path Problem

When a signal via transitions between layers with different reference planes, the
return current has no direct path. The solutions are:

**Ground stitching vias** (preferred):
- Place a ground via within **0.5-2 mm** of every signal via that changes reference
  planes
- For differential pairs, place ground vias **symmetrically** near the signal via pair
- Via spacing rule of thumb: stitching via distance < lambda/20 of the highest
  signal frequency
- For a 5 GHz signal: lambda/20 = 3 mm (in FR4 with Er=4)

**Stitching capacitors** (when crossing power/ground plane boundaries):
- Place 0.1 uF to 10 nF capacitors at or near the layer transition point
- These provide an AC return path between planes of different DC potential
- Effective reduction: up to 10 dB crosstalk reduction with proper placement
- Multiple capacitors recommended for critical signals

### 3.3 Router Integration: Automated Return Via Placement

**Post-routing approach** (simpler, recommended for initial implementation):

1. After all signal routing is complete, scan all vias
2. For each signal via, determine if it causes a reference plane change
3. If yes, check if a ground via exists within the required proximity
4. If no nearby ground via exists, attempt to place one:
   - Search nearby grid cells for a legal placement (not blocked, on ground net)
   - Prefer placement adjacent to the signal via (within 2-4 grid cells)
   - Respect clearance rules to all other nets

**In-router approach** (more sophisticated):

1. When the A* router places a via that changes reference planes, add a **return
   via cost** to the via penalty:
   ```
   via_cost = base_via_cost + ref_change_penalty + return_via_search_cost
   ```
2. The `ref_change_penalty` is high if no ground via can be placed nearby
3. This naturally discourages layer transitions that lack return path support

### 3.4 Differential Pair Via Transitions

For differential pairs, the via transition is especially critical:

- Place **two** ground vias, one on each side of the differential pair vias
- The ground vias should be **symmetric** about the pair centerline
- Maintain the differential pair spacing through the via transition
- The anti-pad size on the reference plane must not create a void that disrupts
  the return path for adjacent traces

### 3.5 Quantitative Guidelines

| Signal Frequency | Max Return Via Distance | Stitching Via Spacing (lambda/10) |
|-----------------|------------------------|----------------------------------|
| 100 MHz | 15 mm | 150 mm |
| 1 GHz | 1.5 mm | 15 mm |
| 5 GHz | 0.6 mm | 3 mm |
| 10 GHz | 0.3 mm | 1.5 mm |
| 28 GHz | 0.1 mm | 0.5 mm |

---

## 4. Crosstalk Minimization in Routing Cost Functions

### 4.1 Crosstalk Physics

Crosstalk between parallel traces is caused by two electromagnetic coupling mechanisms:

1. **Capacitive coupling** (mutual capacitance Cm): Electric field coupling between
   adjacent conductors. Produces displacement current on the victim.
2. **Inductive coupling** (mutual inductance Lm): Magnetic field coupling between
   adjacent current loops. Produces induced EMF on the victim.

These produce two types of crosstalk:

**Near-End Crosstalk (NEXT)** - backward crosstalk:
```
Kb = NEXT = (1/4) * (Cm/C0 + Lm/L0)
```
Where C0 and L0 are the victim's self-capacitance and self-inductance per unit length.

NEXT saturates at a **critical coupling length**:
```
L_sat = (t_rise * c) / (2 * sqrt(Dk_eff))
```
For t_rise = 0.1 ns in FR4 stripline: L_sat ~ 295 mil (7.5 mm).
Beyond L_sat, NEXT does not increase with additional parallel length.

**Far-End Crosstalk (FEXT)** - forward crosstalk:
```
Kf = FEXT = (L_coupled / t_rise) * (1 / (2*v)) * (Cm/C0 - Lm/L0)
```
FEXT is **proportional to coupled length** -- it does NOT saturate.

**Critical property**: In **stripline** (inner layers), FEXT is approximately zero
because Cm/C0 = Lm/L0 when the dielectric is homogeneous on both sides of the trace.
In **microstrip** (outer layers), FEXT is non-zero because the dielectric is
asymmetric (air above, substrate below).

### 4.2 Spacing Rules: 3W and Beyond

The **3W rule**: Center-to-center spacing between traces should be at least 3x the
trace width. This provides ~70% reduction in electromagnetic field coupling.

| Spacing (center-to-center) | Crosstalk Reduction | Use Case |
|---------------------------|--------------------:|----------|
| 2W | ~50% | Minimum for non-critical signals |
| 3W | ~70% | Standard for most digital signals |
| 5W | ~90% | High-speed or sensitive analog signals |
| 10W | ~98% | Ultra-sensitive or isolation-critical |

The 3W rule is a practical approximation. The actual crosstalk depends on the **s/h
ratio** (spacing / height above reference plane). The coupling decreases rapidly as
s/h increases.

For practical extraction of coupling coefficients, a 2D field solver is needed.
However, for a router cost function, we can use simplified models.

### 4.3 Academic Approaches to Crosstalk-Aware Routing

**Ho, Chang, Chen, Lee (ICCAD 2003)**: "A Fast Crosstalk- and Performance-Driven
Multilevel Routing System"
- Incorporated an intermediate stage of layer/track assignment into a multilevel
  routing framework
- Used a minimum-radius minimum-cost spanning-tree (MRMCST) heuristic
- **Results**: 30% reduction in maximum crosstalk (coupling length), 24% average
  reduction, 15% max delay reduction, 6.7x runtime speedup vs prior art

**Zhou & Wong (IEEE TCAD 1999)**: "Global Routing with Crosstalk Constraints"
- Formulated crosstalk as a constraint on the global routing problem
- Budgeted crosstalk across routing regions

**Chaudhary et al. (ACM FPGA 2001)**: "A Crosstalk-Aware Timing-Driven Router for
FPGAs"
- Extended PathFinder-style negotiated congestion routing with crosstalk awareness
- Added crosstalk delay to the timing model
- **Result**: 7.1% average routing delay reduction vs crosstalk-unaware router

**Key insight from the literature**: Crosstalk in routers is typically modeled as
**coupling length** -- the total parallel run length between two adjacent traces at
close spacing. Minimizing total coupling length is the primary optimization objective.

### 4.4 Crosstalk Cost Function for Grid-Based Router

For our A* grid router, the crosstalk cost can be modeled as an **adjacency penalty**:

```
crosstalk_cost(node_n, direction) =
    for each adjacent grid cell perpendicular to direction:
        if cell is occupied by a different net:
            coupling_penalty * (1.0 / distance_cells)
```

More sophisticated model accounting for coupling length:

```
// During A* expansion, when moving from node p to node n:
crosstalk_cost(p, n) =
    for each neighbor cell of n (perpendicular to travel direction):
        if neighbor is occupied by net != current_net:
            // Check if the same net was also adjacent at node p
            // (continuing parallel run)
            if same_aggressor_at_p:
                parallel_run_length += 1
                penalty = coupling_weight * parallel_run_length * (1/distance)
            else:
                parallel_run_length = 1
                penalty = coupling_weight * (1/distance)
    return penalty
```

This models the physical reality that FEXT increases with coupled length.

**Practical simplification for our grid router**:

Since our grid cells are uniform, the adjacency penalty can be precomputed:

```
// In workspace construction:
for each grid cell (x, y, layer):
    adjacency_score[x][y][layer] = count of occupied neighbor cells by other nets

// In A* cost:
crosstalk_term = si_weight * adjacency_score[x][y][layer]
```

This is updated dynamically as nets are routed/ripped up in the PathFinder loop.

### 4.5 Orthogonal Layer Routing for Crosstalk

The standard practice of routing adjacent signal layers in **orthogonal preferred
directions** (horizontal on one layer, vertical on the next) inherently minimizes
parallel coupling length. Our existing `dir_penalty` already encourages this.

For same-layer crosstalk, the 3W spacing can be enforced through:
1. **Clearance inflation**: Inflate obstacle blocking by the 3W spacing requirement
   (net-class dependent) rather than just the minimum manufacturing clearance
2. **SI clearance zones**: For SI-sensitive net classes, use a wider clearance
   inflation that corresponds to 3W or 5W spacing

---

## 5. Length-Matched Routing for Manufacturing

### 5.1 Why Length Matching Matters

Length matching ensures that signals in a bus or differential pair arrive at the
receiver within a timing skew budget. This is critical for:

| Interface | Typical Skew Budget | Length Match Tolerance |
|-----------|--------------------|-----------------------|
| DDR3 | 25 ps | ~3.7 mil |
| DDR4 | 5-10 ps | ~0.75-1.5 mil |
| PCIe Gen3 | 20 ps | ~3 mil |
| USB 3.0 | 10 ps | ~1.5 mil |
| HDMI 2.0 | 10 ps | ~1.5 mil |
| 100 MHz bus | ~100 ps | ~15 mil |

General skew tolerance categories:

| Signal Speed | Frequency | Allowed Skew |
|-------------|-----------|--------------|
| Low-speed | < 100 MHz | +/- 100 mil (2.54 mm) |
| Moderate-speed | 100 MHz - 1 GHz | +/- 25-50 mil |
| High-speed | > 1 GHz | +/- 5-10 mil |
| Ultra-high-speed | Multi-GHz | +/- 2-5 mil |

### 5.2 Serpentine/Meander Pattern Types

Three main delay-matching structures:

**Sawtooth (serpentine)**:
- Diagonal extensions at 45 degrees
- Follows "S-2S" spacing rule and "3W" upper length limit
- Best for differential pairs and parallel buses
- Minimizes impedance discontinuities through angular transitions
- Recommended for most applications

**Accordion**:
- Orthogonal (90-degree) extensions of varying lengths
- More compact than sawtooth for the same added length
- Best for differential pairs and parallel buses with common-mode tolerance
- Preferred for DDR interfaces where synchronization matters
- Creates slight impedance mismatch at entry/exit points

**Trombone**:
- Multiple 90/180-degree turns creating a U-shape extension
- Best for parallel buses only at lower speeds
- **Should NOT be used on differential pairs** -- causes mode conversion
  (signal switches between common mode and differential mode)
- Generates significantly more NEXT than alternatives

### 5.3 Minimum Spacing Between Serpentine Segments

**Critical DFM/SI constraint**: Serpentine segments that are too close together
create self-coupling (crosstalk between adjacent segments of the same trace), which
causes:

1. **Impedance discontinuity**: Each bend creates a small reflection
2. **Self-coupling**: Adjacent parallel segments couple capacitively and inductively,
   effectively shortening the electrical delay added by the serpentine
3. **Manufacturing difficulty**: Very tight serpentine patterns may violate
   manufacturing clearance or acid trap rules

**Minimum spacing guidelines**:

| Application | Min Spacing | Rationale |
|-------------|------------|-----------|
| General digital | >= 2x trace width | Minimum to reduce self-coupling |
| Standard high-speed | >= 3x trace width | Industry standard (3W rule) |
| High-speed > 5 GHz | >= 4-5x trace width | Minimize insertion loss degradation |
| 25+ Gbps SerDes | >= 5x trace width or guard traces | Validated by simulation |

Real-world example: A 25 Gbps SerDes design with 6 mil spacing between meanders
suffered 12 dB insertion loss degradation. Increasing to 18 mil spacing (3x trace
width for 6 mil traces) restored signal integrity.

### 5.4 Differential Pair Length Matching with Impedance Constraints

When length-tuning a differential pair, the serpentine pattern is applied to the
**shorter** trace of the pair. This creates a region where the pair spacing increases,
which affects differential impedance:

**The problem**: As intra-pair gap enlarges for serpentining, the differential mode
impedance rises (because coupling decreases), creating an impedance discontinuity.
This also worsens differential-to-common-mode conversion.

**Mitigation strategies**:

1. **Minimize added length**: Place length tuning at the source of skew (near
   connectors, via transitions) rather than adding unnecessary serpentine
2. **Maintain coupling**: Use the smallest serpentine amplitude that achieves the
   required delay, keeping the pair spacing increase minimal
3. **Symmetric patterns**: Use accordion or sawtooth (not trombone) to maintain
   symmetric differential excitation
4. **Post-tuning coupling region**: After the serpentine section, route the pair
   tightly coupled for a distance to allow the differential mode to re-establish

### 5.5 Length Matching Algorithm (State of the Art)

**Obstacle-aware length matching** (Xu et al., 2024, arxiv:2407.19195):

1. Assign a routable area for each trace that needs length tuning
2. Within the routable area, use dynamic programming to insert serpentine patterns:
   ```
   dp[i][dir] = max(dp[i][dir], dp[i-w][+/-dir] + h)
   ```
   Where h = pattern height constrained by available space
3. Handle obstacles via "UnReachable Area" (URA) concept -- rectangular buffers
   at half the gap distance from existing segments
4. Iteratively shrink pattern heights when obstacles intersect
5. Works with any-angle traces (not just Manhattan routing)

**Key DFM integration point**: The algorithm respects:
- Minimum gap distance between serpentine segments (configurable per net class)
- Minimum segment length (avoids acid traps in manufacturing)
- Miter angles (45-degree preferred for high-speed)
- Obstacle clearances to other nets

### 5.6 Fiber Weave Effects on Length Matching

**Manufacturing concern**: FR4 PCB substrates have a glass fiber weave with a
non-uniform dielectric constant. Traces aligned with the weave direction can
experience different propagation delays than traces at an angle. This creates
unpredictable skew that can defeat length matching.

**Mitigation**:
- Rotate PCB image 10-35 degrees relative to fiber weave direction
- Use zig-zag routing patterns that average out the dielectric variation
- Specify tighter weave PCB materials (e.g., NE-glass, spread glass) for critical
  high-speed designs

---

## 6. Via Stub Management

### 6.1 The Via Stub Problem

When a through-hole via connects only two inner layers, the unused barrel portions
act as open-circuited stubs. These stubs create a resonance at the quarter-wave
frequency that causes a deep null in the signal's frequency response.

**Quarter-wave resonant frequency**:
```
f_res = c / (4 * L_stub * sqrt(Dk_eff))
```

Where:
- c = speed of light (11.8 in/ns or 30 cm/ns)
- L_stub = stub length
- Dk_eff = effective dielectric constant (typically 15-25% higher than bulk Dk
  for FR4 due to anisotropy and via pad capacitance)

**Simplified rule of thumb** for FR4 (Dk_eff ~ 6.0-6.4):

```
f_res [GHz] = 1.5 / L_stub [inches]
f_res [GHz] = 3.8 / L_stub [cm]
```

Higher-order resonances occur at odd harmonics: 3*f_res, 5*f_res, etc.

### 6.2 Maximum Stub Length by Data Rate

The design guideline is that the first resonant null should occur at or above the
7th harmonic of the Nyquist frequency:

```
L_stub_max = c / (4 * sqrt(Dk_eff) * 7 * f_nyquist)
```

| Data Rate | f_nyquist | Max Stub (FR4) | Back-drill Required? |
|-----------|-----------|----------------|---------------------|
| 1 Gbps | 0.5 GHz | 300 mil (7.6 mm) | No (typical boards) |
| 5 Gbps | 2.5 GHz | 60 mil (1.5 mm) | Usually |
| 10 Gbps | 5 GHz | 33 mil (0.84 mm) | Yes |
| 25 Gbps | 12.5 GHz | 12 mil (0.3 mm) | Yes |
| 28 Gbps (PAM4) | 14 GHz | 10 mil (0.25 mm) | Yes |
| 56 Gbps (PAM4) | 28 GHz | 5 mil (0.13 mm) | Yes, tight tolerance |

### 6.3 Stub Mitigation Strategies

1. **Back-drilling**: Mechanically remove the unused via barrel from the side where
   the stub exists. Manufacturing tolerance: minimum residual stub of 5-10 mil.
   Cost-effective for moderate volumes. The back-drill depth is controlled per
   via, per side of board, with reference to the layer stackup.

2. **Blind/buried vias**: Only connect the layers needed. More expensive due to
   sequential lamination but eliminates stubs entirely. Common in HDI designs.

3. **Via-in-pad with through-hole**: Acceptable when the stub is short enough for
   the data rate. Cheapest option when the board is thin enough.

4. **Layer assignment optimization**: Route high-speed signals on layers that
   minimize stub length when using through-hole vias.

### 6.4 Layer Assignment Strategy for Stub Minimization

**For through-hole vias** (most common, cheapest):

The stub length depends on which layers the via actually connects. For a signal
transitioning from Top to an inner layer:

```
stub_length = board_thickness - depth_to_deepest_connected_layer
```

For a signal transitioning from Bottom to an inner layer:

```
stub_length = depth_to_shallowest_connected_layer
```

**Optimal strategy**: Route high-speed signals on layers that are **near the surface**
of the board, so the stub (unused barrel on the opposite side) is short.

| Board Layers | High-Speed Signal Layers | Via Stub Impact |
|-------------|-------------------------|-----------------|
| 4-layer | Top, Bottom | Minimal (thin board) |
| 6-layer | L1, L2, L5, L6 | Short stubs from surface |
| 8-layer | L1, L2 (top), L7, L8 (bottom) | Short stubs |
| 10+ layer | Outer layers preferred | Long stubs if using inner layers |

**Cost function integration**:
```
via_stub_penalty(from_layer, to_layer, board) =
    stub_length = compute_stub_length(from_layer, to_layer, board.stackup)
    if stub_length > max_stub_for_net_class:
        return HIGH_PENALTY  // Discourage this layer transition
    else:
        return stub_weight * stub_length / max_stub  // Proportional penalty
```

### 6.5 Back-Drilling in the Autorouter

The autorouter should be **back-drill aware** but does not perform the actual
back-drilling. Instead:

1. **During routing**: Apply via stub penalties to the cost function so that
   layer assignments naturally minimize stub lengths for high-speed nets
2. **Post-routing DRC**: Flag all vias where the stub length exceeds the maximum
   for the net class (using the Altium-style Max Via Stub Length rule)
3. **Back-drill annotation**: Generate back-drill specifications for flagged vias,
   specifying drill side, depth, and drill diameter
4. **Via type selection**: When blind/buried vias are available in the stackup,
   prefer them for high-speed nets (with appropriate cost weighting)

---

## 7. Integration Strategy for autopcb-router

### 7.1 Phase 1: Data Model Extensions

Add to `PcbIr` or `RoutingConfig`:

```
struct ImpedanceProfile {
    name: String,
    target_ohm: f64,
    tolerance_pct: f64,
    kind: ImpedanceKind,  // Single, Differential, Coplanar
    layer_widths: BTreeMap<LayerId, f64>,  // mm, pre-computed
}

struct StackupInfo {
    layers: Vec<StackupLayer>,        // ordered top to bottom
    reference_planes: BTreeMap<LayerId, LayerId>,  // signal layer -> ref plane
}

struct SiNetClass {
    name: String,
    impedance_profile: Option<String>,  // reference to ImpedanceProfile
    max_via_stub_mm: Option<f64>,
    max_parallel_coupling_mm: Option<f64>,
    crosstalk_spacing_rule: SpacingRule,  // 3W, 5W, or custom
    require_return_via: bool,
    max_skew_ps: Option<f64>,
}
```

### 7.2 Phase 2: Reference Plane Bitmap

During workspace construction:

1. For each signal layer, identify the adjacent reference plane layer(s)
2. Compute a **reference quality bitmap** where each grid cell is marked:
   - `SOLID`: continuous copper in reference plane
   - `VOID`: gap, split, or antipad in reference plane
   - `EDGE`: near the edge of a plane pour (within 3W of edge)
3. Use this bitmap in the A* cost function as `ref_penalty(n)`

### 7.3 Phase 3: SI-Aware Cost Function

Extend the A* cost function:

```
C(n) = base_cost * dir_penalty * corridor_penalty
     + ref_penalty(n)                    // Phase 2: reference plane quality
     + crosstalk_penalty(n, direction)   // Phase 3: adjacent trace coupling
     + via_stub_penalty(from, to)        // Phase 3: via stub length
     + return_path_penalty(via)          // Phase 3: reference change at via
     + hist_weight * history[n]
     + pres_fac * max(0, usage[n] - 1)
```

### 7.4 Phase 4: Width-Aware Routing

1. Look up trace width from impedance profile for current net + current layer
2. Compute clearance inflation based on wider of (manufacturing clearance, SI
   spacing requirement for net class)
3. At via transitions, handle width changes between layers

### 7.5 Phase 5: Post-Route SI Optimization

After the PathFinder loop converges:

1. **Return via insertion**: Scan all signal vias for reference plane changes,
   insert ground stitching vias where needed
2. **Via stub DRC**: Flag all vias exceeding max stub length for their net class
3. **Crosstalk DRC**: Measure parallel coupling lengths between adjacent nets,
   flag violations of spacing rules
4. **Length matching**: Apply serpentine patterns to nets that need delay matching,
   using the DP-based obstacle-aware algorithm

### 7.6 Implementation Priority

| Priority | Feature | Impact | Complexity |
|----------|---------|--------|------------|
| **P0** | Per-layer width from impedance profile | High | Low |
| **P0** | SI spacing clearance (3W/5W) per net class | High | Low |
| **P1** | Reference plane bitmap + cost penalty | High | Medium |
| **P1** | Via stub penalty in cost function | Medium | Low |
| **P2** | Crosstalk adjacency penalty in A* | Medium | Medium |
| **P2** | Return via placement post-route | Medium | Medium |
| **P3** | Serpentine length matching | High | High |
| **P3** | Automated via type selection (blind/buried) | Low | Medium |

---

## 8. References

### Textbooks and Foundational Work

- Wadell, B.C. *Transmission Line Design Handbook*. Artech House, 1991.
  (Standard reference for impedance equations, including the 67-equation
  differential pair formulation)

- Wheeler, H.A. "Transmission-line properties of a strip on a dielectric sheet
  on a plane." IEEE Trans. Microwave Theory and Techniques, vol. MTT-25,
  pp. 631-647, Aug. 1977. (Foundation for Wadell's microstrip equations)

- McMurchie, L. and Ebeling, C. "PathFinder: A Negotiation-Based Performance-Driven
  Router for FPGAs." ACM/SIGDA FPGA, 1995.
  (Foundation of our negotiated congestion routing approach)

### Crosstalk-Aware Routing

- Ho, T.Y., Chang, Y.W., Chen, S.J., Lee, D.T. "A Fast Crosstalk- and Performance-
  Driven Multilevel Routing System." ICCAD 2003.
  (30% max crosstalk reduction, 24% average, via multilevel routing with layer/track
  assignment. Published also in IEEE TCAD 2005.)
  https://ieeexplore.ieee.org/document/1257806/

- Chaudhary, K. et al. "A Crosstalk-Aware Timing-Driven Router for FPGAs."
  ACM/SIGDA FPGA 2001.
  (Extended PathFinder with crosstalk; 7.1% delay reduction)
  https://dl.acm.org/doi/10.1145/360276.360292

- Zhou, H. and Wong, D.F. "Global Routing with Crosstalk Constraints."
  IEEE TCAD, 1999.
  (Crosstalk budgeting in global routing)
  http://www.eecs.northwestern.edu/~haizhou/publications/zhou99tcad.pdf

- Chen, H.Y. and Chang, Y.W. "A Coupling and Crosstalk Considered Timing-Driven
  Global Routing Algorithm." ASP-DAC 2004.
  https://ieeexplore.ieee.org/document/1337678/

### Length Matching

- Xu, J. et al. "Obstacle-Aware Length-Matching Routing for Any-Direction Traces
  in Printed Circuit Board." arXiv:2407.19195, 2024.
  (DP-based serpentine insertion with URA obstacle avoidance)
  https://arxiv.org/html/2407.19195v1

### Signal Integrity and Via Stubs

- Simonovich, B. "Via Stubs Demystified." Lamsim Enterprises Design Notes, 2017.
  (Quarter-wave resonance, Dk_eff estimation, maximum stub length formulas)
  https://blog.lamsimenterprises.com/2017/03/08/via-stubs-demystified/

- Simonovich, B. "Coupled Transmission Lines and Crosstalk." Signal Integrity
  Journal, 2022.
  (NEXT/FEXT formulas, saturation length, coupling coefficients)
  https://www.signalintegrityjournal.com/articles/2722-coupled-transmission-lines-and-crosstalk

- Dannan, A. "Signal Integrity Characterization of Via Stubs on High Speed DDR4
  Channels." DesignCon 2020.
  https://www.signalintegrityjournal.com/ext/resources/article-images-2020/

### Industry Tool Documentation

- Altium Designer. "Controlled Impedance Routing."
  https://www.altium.com/documentation/altium-designer/pcb/high-speed-design/interactively-routing-controlled-impedance

- Altium Designer. "Max Via Stub Length (Back Drilling) Design Rule."
  https://www.altium.com/documentation/altium-designer/pcb-high-speed-rule-max-via-stub-length-back-drilling

- Cadence. "Grounding and Return Paths: Advanced Techniques."
  https://resources.pcb.cadence.com/blog/er-grounding-and-return-paths-advanced-techniques

- Cadence. "Differential Pair Length Matching Guidelines." 2025.
  https://resources.pcb.cadence.com/blog/2025-differential-pair-length-matching-guidelines

### Modern AI-Driven Approaches

- Quilter AI. "PCB Autorouting in 2026: A Review of Traditional Tools vs. Quilter's
  AI Approach."
  https://www.quilter.ai/blog/pcb-autorouting-in-2026-a-review-of-traditional-tools-vs-quilters-ai-approach
  (Physics-driven AI routing with impedance-net identification, return path
  optimization, and constraint-aware layout generation using reinforcement learning)

### Multi-Agent and ILP-Based Routing

- ScienceDirect, 2025. "Multi-agent based minimal-layer via routing algorithm for
  PCB design."
  https://www.sciencedirect.com/science/article/abs/pii/S0167926025001907
  (Minimal Layer Via (MLV) routing using enhanced Luby algorithm + MIS preprocessing
  for CBS; minimizes vias while maintaining routing quality)
