# DFM: Copper Balance, Thermal Relief, and Thermal Management in PCB Autorouting

Research document for integrating manufacturing-aware copper distribution and thermal
management into the autopcb-router. Covers state of the art in commercial and academic
EDA, algorithmic approaches, and concrete implementation recommendations.

---

## Table of Contents

1. [Copper Balance Across Layers](#1-copper-balance-across-layers)
2. [Thermal Relief Patterns](#2-thermal-relief-patterns)
3. [Current-Carrying Capacity Aware Routing](#3-current-carrying-capacity-aware-routing)
4. [Via Stitching for Thermal Management](#4-via-stitching-for-thermal-management)
5. [Copper Pour Integration](#5-copper-pour-integration)
6. [Implementation Recommendations](#6-implementation-recommendations-for-autopcb-router)

---

## 1. Copper Balance Across Layers

### 1.1 Why Copper Balance Matters

Copper balance refers to achieving uniform copper density across all layers of a PCB
stackup and across the area of each individual layer. Imbalanced copper distribution
causes three categories of manufacturing defect:

**Bow and Twist (Warpage)**

When copper is unevenly distributed between the top and bottom halves of a stackup,
differential thermal contraction during lamination and reflow causes mechanical warpage.
IPC-6012 defines acceptable limits:

- Boards with surface-mount components: max 0.75% bow/twist
- Other applications: max 1.5% bow/twist
- Boards thinner than 1.0 mm face significantly increased warpage risk

Bow is calculated as: `(board_length_or_width * percentage) / 100`
Twist is calculated as: `(2 * board_diagonal * percentage) / 100`

The factor of 2 in twist accounts for the board being constrained at one corner with
deformation in two directions. Beyond ~0.7% warpage of diagonal length, the board is
generally considered to have failed.

**Etching Uniformity**

The etchant removes copper at a rate dependent on the local copper density. In sparse
copper areas (low density), the etchant over-etches, producing narrower traces than
designed. In dense copper areas, under-etching leaves copper remnants ("nubs") that can
cause short circuits. The result is trace width variation across the board. This is
analogous to the CMP (Chemical Mechanical Polishing) uniformity problem in VLSI, where
metal density variation causes polishing depth variation and thickness non-uniformity.

**Plating Uniformity**

During electroplating, current density is higher in sparse copper regions and lower in
dense regions. This produces:
- Excess copper deposition in sparse areas (mushroom profiles, thicker-than-intended plating)
- Insufficient copper deposition in dense areas (thin plating, weak PTH barrels)
- Vias are especially liable for failure with unequal copper distribution

### 1.2 Quantitative Targets

The industry consensus is that each layer should have copper coverage in the 30-70%
range, with the deviation between the most-dense and least-dense layers minimized. A
symmetrical stackup (mirroring the top and bottom halves) is essential: Layer 1 should
approximate the copper density of Layer N, Layer 2 should approximate Layer N-1, and so
on.

### 1.3 How Commercial Tools Handle Copper Balance

**Current State: No autorouter does copper balancing during routing.**

Copper balance is universally handled as a post-routing or fabrication step, not during
trace routing:

1. **Altium Designer**: No built-in copper balance feature. Provides `Tools > Calc. Copper
   Area` to generate per-layer copper area reports. Copper thieving is achieved through a
   workaround: use the via stitching feature to populate non-functional pads in low-density
   areas. Polygon pours (solid or cross-hatched) are the primary mechanism for density
   equalization, placed after routing is complete.

2. **Cadence Allegro**: Provides copper area reporting and layer cost controls in the
   autorouter. Designers can set per-layer routing costs to bias routing toward under-utilized
   layers, but this is manual — no automatic density-driven layer assignment exists. Copper
   balance is handled in the CAM/DFM stage.

3. **Mentor/Siemens Xpedition**: Includes "self-healing copper pour" that automatically
   adjusts pour boundaries, but copper balance is still a post-routing DFM check.

4. **Fabricator CAM Stage**: In practice, copper balance is primarily addressed by the PCB
   fabricator's CAM operators, who add copper thieving patterns (non-functional copper
   dots, squares, or crosshatch) to equalize plating current distribution. The fabricator
   pushes these modifications back to the designer for approval.

### 1.4 Copper Thieving Patterns

Copper thieving adds non-functional copper shapes to low-density areas. The fabricator
(or a post-processing tool) analyzes copper density per region and fills sparse areas.

**Pattern types:**
- **Dot patterns**: Small circular copper shapes, 0.5-2.0 mm diameter. Most versatile,
  suitable for small empty areas.
- **Grid/Square patterns**: Square copper shapes in a grid formation, 0.2-0.5 mm spacing
  between shapes.
- **Crosshatch patterns**: Connected copper mesh. Better for large areas, provides some
  RF shielding benefit but can affect impedance.

**Spacing rules:**
- Minimum 2.5 mm (100 mil) from any functional copper feature on outer layers
- Minimum 2.5 mm from traces on the first buried signal layer beneath
- Minimum 0.2 mm from active circuit areas to prevent shorts or signal interference

**Density targets:**
- Target region should reach 30-70% copper fill after thieving
- Larger thieving dots in low-density regions, smaller dots in moderate-density regions

### 1.5 VLSI Analogues: Density-Driven Fill Insertion

The VLSI industry solved an analogous problem for CMP uniformity using dummy metal fill.
The algorithms are directly applicable to PCB copper balance:

**Sliding Window Density Model (Kahng et al., UCSD VLSI CAD Lab):**

The layout is divided into overlapping windows (e.g., 200um x 200um stepped by 50um).
For each window, the metal density (copper area / window area) is computed. The
optimization objective is to minimize the density variation across all windows while
satisfying per-window density bounds [rho_min, rho_max].

Adapted for PCB: divide each layer into a grid of tiles (e.g., 5mm x 5mm). Compute
copper coverage percentage per tile. The optimization problem is:

```
minimize  max(density[i]) - min(density[i])  across all tiles i on each layer
subject to  density[i] >= rho_min  for all tiles i
```

**Three-Phase Fill Algorithm:**
1. **Layout analysis**: Divide the layer into uniform windows/tiles, compute density per tile
2. **Fill synthesis**: Determine how much additional copper each tile needs (LP relaxation
   or greedy assignment)
3. **Fill insertion**: Place thieving shapes to achieve the target density per tile

**Greedy Min-Variation Fill:**

Process tiles in order of lowest density first. For each tile, insert fill shapes
(dots/squares) until the tile reaches the target density or no more space is available
(respecting clearance to functional copper). This is simple and effective.

### 1.6 Layer-Balancing During Routing (Novel Approach)

While no commercial tool does this today, a PathFinder-based router can incorporate
copper density into the layer assignment cost function:

**Per-layer density tracking:**

Maintain a per-layer copper density counter that updates as routes are committed. When
choosing which layer to route on (during layer transitions/via decisions), add a cost
term that penalizes routing on the already-densest layer:

```
layer_balance_cost(layer) = alpha * (density[layer] - avg_density) / avg_density
```

Where `alpha` is a configurable weight. This biases the router to distribute copper more
evenly across layers. The density can be computed per-tile (local balance) or per-layer
(global balance).

**Challenges:**
- Conflicts with preferred-direction layer assignment
- Conflicts with shortest-path optimality
- Requires careful weight tuning (too aggressive = poor routability, too mild = no effect)
- Density changes dynamically as routes are added and ripped up

**Recommendation:** Implement as an optional post-routing layer reassignment pass rather
than an in-loop cost function modification. After routing converges, identify the
densest/sparsest layers and attempt to reassign some routes from dense to sparse layers
via rip-up and reroute with adjusted layer preferences.

---

## 2. Thermal Relief Patterns

### 2.1 What Thermal Reliefs Are

A thermal relief is a patterned connection between a pad (or via) and a surrounding
copper pour. Instead of a solid (direct) connection, the pad connects through narrow
copper "spokes" with air gaps between them. This limits heat conduction from the pad to
the pour, making the pad easier to solder (the soldering iron or reflow oven can heat
the pad without the heat being immediately conducted away by the copper plane).

### 2.2 Connection Styles

PCB design tools support three connection styles for pad-to-pour connections:

**Direct Connect:**
- Solid copper connection between pad and pour, no air gap
- Lowest electrical resistance and thermal resistance
- Required for high-current connections (power pads carrying > 1A)
- Difficult to solder by hand (heat dissipates into plane)
- Appropriate for reflow-only SMD power pads, heatsink pads

**Relief Connect (Thermal Relief):**
- Spoke connections with air gaps
- Higher thermal resistance = easier to solder
- Slightly higher electrical resistance (usually negligible)
- Standard default for most through-hole and SMD connections to pours
- The dominant connection style in production PCBs

**No Connect:**
- Pad/via is completely isolated from the pour with a clearance gap
- Used for pads on different nets, or deliberately isolated pads
- Creates an "antipad" around the pad in the pour

### 2.3 Thermal Relief Parameters

Based on Altium Designer's Polygon Connect Style rule (representative of industry
standard parameters):

| Parameter | Description | Typical Values |
|-----------|-------------|----------------|
| Conductor Count | Number of spokes | 2, 4, or Auto |
| Spoke Angle | Rotation angle of spokes | 45 deg or 90 deg |
| Conductor Width | Width of each spoke | 0.2-0.5 mm (8-20 mil) |
| Air Gap Width | Gap between pad edge and pour | 0.25-0.5 mm (10-20 mil) |
| Expansion | Radial width of copper ring around the hole | Varies |

**Auto mode** (Altium): generates one spoke from the center of each separate edge of the
pad/via shape, radiating outward at 90 degrees to that edge. For rounded shapes, one
conductor per 90 degrees of arc. A "Min Distance" option removes adjacent spokes if
spacing is too tight.

**Advanced mode**: Separate configuration for through-hole pads, SMD pads, and vias.
This is important because:
- Through-hole pads: always use thermal relief (hand soldering)
- SMD pads: thermal relief or direct connect depending on current
- Vias: typically direct connect (no soldering concerns)

### 2.4 Thermal Relief Generation Algorithm

Thermal reliefs are generated during the copper pour fill operation, not during routing.
The algorithm (based on KiCad's zone_filler.cpp and Altium's behavior):

1. **Identify connected pads/vias**: For each pad or via within the pour boundary that
   belongs to the same net, look up the applicable Polygon Connect Style rule.

2. **Generate spoke geometry**: Based on the spoke count, angle, and width, create spoke
   polygons radiating from the pad center.

3. **Generate antipad geometry**: Create the air gap ring around the pad (pad shape
   inflated by air_gap_width). This is subtracted from the pour.

4. **Boolean operations**: The pour polygon is computed as:
   ```
   pour = boundary_polygon
          - union(all_antipads_for_different_nets)
          - union(all_thermal_relief_gaps)
          + union(all_spoke_geometries)
          - union(all_clearance_violations)
   ```

5. **Island detection**: After the boolean operations, check for disconnected copper
   islands. Remove or flag them based on configuration.

### 2.5 How Autorouters Interact with Thermal Reliefs

**Critical finding: Autorouters do NOT generate thermal reliefs.**

Thermal reliefs are exclusively a copper pour feature. The routing phase and the pour
phase are separate:

1. Router places traces and vias between pads
2. After routing, copper pours are filled (or re-filled)
3. During pour fill, thermal reliefs are generated at pad-pour intersections
4. DRC validates the final result

The autorouter must be *aware* of copper pours to avoid routing through pour-reserved
areas and to account for the fact that poured ground/power planes provide connectivity
(so the router doesn't need to route explicit traces for nets that are connected through
pours).

### 2.6 Thermal Relief and Current Capacity

The spoke connections in a thermal relief have a limited current capacity. For a 4-spoke
relief with 0.25 mm (10 mil) wide spokes on 1 oz copper:

- Cross-sectional area per spoke: 0.25 mm * 0.035 mm = 0.00875 mm^2
- Per IPC-2221 external layer: ~0.5A per spoke at 10 degC rise
- Total for 4 spokes: ~2.0A

For high-current pads, the designer must either:
- Use direct connect (no thermal relief)
- Increase spoke width
- Increase spoke count
- Use wider spokes with larger air gaps

The autorouter's per-net current specification should flag pads where thermal relief
current capacity is insufficient.

---

## 3. Current-Carrying Capacity Aware Routing

### 3.1 IPC Standards for Trace Current

**IPC-2221 (Original, 1998):**

The foundational formula relates trace cross-sectional area to current capacity:

```
I = k * dT^0.44 * A^0.725
```

Where:
- I = maximum current (Amps)
- k = 0.048 (external layers) or 0.024 (internal layers)
- dT = temperature rise above ambient (degC)
- A = cross-sectional area (square mils)

Solving for area: `A = (I / (k * dT^0.44))^(1/0.725)`

Trace width (mils): `W = A / (thickness_oz * 1.378)`

For common scenarios (1 oz copper, 10 degC rise):
- 1A external: ~10 mil (0.25 mm) trace width
- 1A internal: ~25 mil (0.635 mm) trace width
- 3A external: ~50 mil (1.27 mm) trace width
- 5A external: ~110 mil (2.8 mm) trace width

Internal layers require approximately 2-3x wider traces than external layers for the
same current because they have no convective cooling and rely entirely on conduction
through FR4 substrate.

**IPC-2152 (Successor, 2009):**

More accurate standard based on extensive empirical testing. Key differences from
IPC-2221:
- Accounts for board thickness, copper weight, thermal conductivity of substrate
- Provides derating modifiers for proximity to planes, board edges, other traces
- Charts assume traces spaced > 1 inch apart (conservative)
- Recommends 20-30% current derating for high-reliability applications
- Plane proximity modifier: traces over solid ground planes dissipate heat better
- Board thickness modifier: thicker boards dissipate more heat

### 3.2 How Commercial Autorouters Handle Current

**No commercial autorouter automatically widens traces based on current requirements.**

The workflow is:

1. Designer defines net classes with minimum trace widths (e.g., "Power" class = 20 mil)
2. Autorouter routes using the specified minimum width as the actual width
3. Designer (or DFM tool) performs post-routing power integrity analysis
4. IR drop simulation identifies inadequate traces
5. Designer manually widens critical traces

Altium's Situs autorouter includes a dedicated "power and ground router" engine, but it
routes power nets using pre-defined trace widths from net class rules, not calculated
from current requirements.

**Why autorouters don't auto-widen:**
- Current requirements are not always specified in the netlist
- Trace-level current depends on the complete circuit (branch currents, not net current)
- Widening traces has cascading effects on neighboring routes (clearance violations)
- Power distribution often uses copper pours or planes, not traces

### 3.3 Cost Function Modifications for Current-Aware Routing

For a PathFinder router, current awareness can be integrated through the net class
configuration:

**Approach 1: Pre-computed width assignment (recommended)**

Before routing, compute minimum trace width per net based on expected current:

```rust
fn min_width_for_current(current_a: f64, temp_rise_c: f64, copper_oz: f64, external: bool) -> f64 {
    let k = if external { 0.048 } else { 0.024 };
    let area_sq_mils = (current_a / (k * temp_rise_c.powf(0.44))).powf(1.0 / 0.725);
    let width_mils = area_sq_mils / (copper_oz * 1.378);
    width_mils * 0.0254 // convert to mm
}
```

This width becomes the net's minimum width constraint in the routing config. The grid
resolution must accommodate the widest net (grid cell size <= min_width / 2 for adequate
clearance representation).

**Approach 2: Width-dependent edge cost**

Wider traces occupy more grid cells. In the A* cost function, wider nets should have
higher base cost to account for the additional blockage they create:

```
base_cost(net) = 1.0 + width_penalty * (trace_width / grid_resolution - 1.0)
```

This discourages unnecessarily long routes for wide traces.

**Approach 3: Power net priority ordering**

Route power/high-current nets first (before signal nets) since they are less flexible
and need wider traces. This is already common practice. In the PathFinder net ordering,
assign higher priority to nets with larger width requirements.

### 3.4 Voltage Drop (IR Drop) Considerations

Beyond trace width, voltage drop along power traces is critical. The resistance of a
copper trace is:

```
R = rho * L / (W * T)
```

Where rho = 1.7e-8 ohm*m (copper resistivity), L = trace length, W = width, T = thickness.

Voltage drop: `V_drop = I * R`

For 1A through a 10 mil (0.254 mm) trace, 1 oz copper (35 um), 100 mm long:
- R = 1.7e-8 * 0.1 / (0.000254 * 0.000035) = 0.191 ohms
- V_drop = 0.191V (significant for 3.3V or 1.8V power rails)

An IR-drop-aware router would minimize total power trace length, which is already
achieved by shortest-path routing. The key insight is that for power nets, the router
should strongly prefer wider traces and shorter paths, even at the cost of more vias
or non-preferred-direction routing.

---

## 4. Via Stitching for Thermal Management

### 4.1 Types of Via Stitching

**Ground Stitching Vias:**
Connect ground planes across layers to reduce ground impedance and suppress resonant
cavities. Spacing is frequency-dependent:
- Low frequency: lambda/20 spacing
- High frequency: lambda/10 spacing
- At 2.4 GHz: 6-15 mm spacing
- General rule: less than lambda/8 to lambda/20 of highest frequency

**Thermal Vias:**
Placed under high-power components (ICs, power regulators, LED drivers) to transfer
heat from the component's thermal pad through the board to a heatsink or ambient air
on the opposite side.

**Signal Return Path Stitching:**
Placed adjacent to signal vias (within 0.5-1.0 mm) to provide a low-inductance return
current path when signals transition between layers.

### 4.2 Thermal Via Design Parameters

**Single via thermal resistance:**

```
R_theta = L / (k * A)
```

Where:
- L = PCB thickness (board thickness, typically 1.6 mm)
- k = thermal conductivity of copper (385 W/m*K for plated, 398 W/m*K for solid)
- A = cross-sectional area of copper in the via barrel

For a plated (hollow) via:
```
A = pi * (D * t - t^2)
```
Where D = via diameter, t = plating thickness.

Example: 0.25 mm diameter, 25 um plating, 1.6 mm board:
- A = pi * (0.00025 * 0.000025 - 0.000025^2) = 1.767e-8 m^2
- R_theta = 0.0016 / (385 * 1.767e-8) = 235 degC/W per via

**Via array: R_theta_array = R_theta_single / N_vias**

Typical thermal via parameters:
- Finished hole size: 0.25-0.30 mm
- Via pad size: 0.5-0.6 mm
- Plating thickness: 25-35 um (1 oz)
- Via-to-via spacing: 0.8-1.5 mm center-to-center
- Minimum 1.0-1.2 mm spacing to prevent solder wicking

**Filled vias** (VIPPO or epoxy-filled) have much lower thermal resistance because the
entire cross-section conducts heat. For a 0.3 mm filled via:
- A = pi * (0.00015)^2 = 7.07e-8 m^2
- R_theta = 0.0016 / (385 * 7.07e-8) = 58.7 degC/W

### 4.3 IPC-7093 Thermal Pad Guidelines (QFN/BTC Packages)

IPC-7093 "Design and Assembly Process Implementation for Bottom Termination Components"
provides thermal via guidelines for exposed-pad packages:

- Use as many vias as can practically fit within the exposed pad (EPAD)
- Minimum 8 vias for 36-pin QFN, 16+ vias for 6x6 mm EPADs
- Via voiding post-reflow must not exceed 25% per IPC-A-610
- Solder paste thickness on thermal pad: 100-125 um (4-5 mil)
- Finished standoff dimension: 50-75 um after reflow

### 4.4 Via Stitching Algorithms

**Grid-Based Placement (Most Common):**

1. Define the stitching region (board area, ground pour boundary, or component EPAD)
2. Create a regular grid within the region at the specified pitch
3. At each grid point, check DRC clearances against all existing objects
4. Place a via if clearance is satisfied
5. Connect the via to the target net (typically GND)

**Density-Aware Placement:**

Rather than a uniform grid, place more vias in high-heat areas:

1. Compute a thermal map (from component power dissipation data)
2. In high-heat areas, use tighter via pitch
3. In low-heat areas, use standard pitch or skip vias
4. Verify all vias satisfy DRC

**Integration with Routing:**

Via stitching is a POST-routing operation. It must not block existing routes. The
algorithm:

1. After routing is complete, identify stitching regions
2. For each candidate via position, check clearance against all existing traces, pads,
   and other vias on all layers the via penetrates
3. Only place vias that satisfy all clearances
4. Re-fill copper pours after stitching via placement

### 4.5 Thermal Via Placement Under Components

For components with thermal pads (QFN, DFN, power ICs), the library footprint should
include thermal via patterns in the pad definition. This is a component library concern,
not an autorouter concern. However, the router must:

- Recognize thermal vias as obstacles (they block routing channels)
- Route signal pins around the thermal via array
- Ensure the thermal via array connects to the correct net (usually GND or a thermal
  plane)

---

## 5. Copper Pour Integration

### 5.1 The Pour/Route Sequencing Problem

The fundamental tension in copper pour integration is: **pours and routes are
interdependent but computed at different stages.**

- Routes determine which areas are available for pour filling
- Pours determine which nets are already connected (no routing needed)
- Pours create obstacles that constrain routing
- Routes create obstacles that constrain pour shape

**Industry-standard sequencing:**

1. Place components
2. Define pour boundaries and net assignments (but don't fill yet)
3. Route all traces (router knows pour net assignments for connectivity)
4. Fill/re-fill all pours (respecting trace clearances)
5. Run DRC on the combined result
6. Iterate (add via stitching, adjust pours, re-route if needed)

Some tools (Altium, KiCad) allow pre-routing pour fill for specific purposes (critical
ground regions, unusual power routing shapes), but the general recommendation is to
route first, pour second.

### 5.2 KiCad Zone Fill Algorithm (Reference Implementation)

KiCad's zone_filler.cpp provides the most accessible reference for copper pour
algorithms (open-source, well-documented):

**Multi-stage pipeline:**

1. **Dependency DAG**: Build a directed acyclic graph of zone fill dependencies. Higher-
   priority zones must fill before lower-priority zones (for correct knockout behavior).

2. **Wave-based parallel fill**: Zones with no dependencies fill in parallel. As each
   completes, its successors become eligible.

3. **Per-zone fill**:
   a. Start with the zone boundary polygon
   b. Compute clearance polygons for all obstacles (pads, traces, vias, other zones
      on different nets)
   c. Inflate obstacle polygons by clearance distance
   d. Subtract all clearance polygons from the boundary (Clipper boolean operations)
   e. For same-net pads: generate thermal relief or direct connect geometry
   f. Add spoke geometry back into the pour
   g. Subtract thermal relief air gaps

4. **Island detection**: After fill, identify disconnected polygon fragments.
   Configurable modes: always remove, never remove, or remove if below area threshold.

5. **Iterative re-fill**: When islands are removed from high-priority zones, lower-priority
   zones may expand. The system re-fills affected zones using cached pre-knockout state.

6. **Via re-evaluation**: Final pass checks whether vias should be connected to the filled
   zone based on the final pour geometry.

**Polygon library**: KiCad uses the Clipper library (Angus Johnson) for boolean
polygon operations (union, intersection, difference, XOR). This is a critical
dependency for any pour implementation.

### 5.3 Autorouter Interaction with Pours

**Freerouting (Open Source) Limitation:**

Freerouting's architecture is fundamentally incompatible with copper pours. It represents
all routing as point-to-point traces. Copper pours are imported as fixed `ConductionArea`
objects with shapes frozen at import time. The router cannot dynamically expand or
contract pour regions. This is a structural limitation, not a missing feature.

**Altium Situs Autorouter:**

Can route boards with existing polygon pours, but the pours are treated as fixed
obstacles. A large hatched polygon pour introduces thousands of track/arc objects,
dramatically increasing routing complexity. Altium recommends placing pours AFTER
routing.

**Commercial Router Best Practice:**

Autorouters treat pours in one of two ways:
1. **Pour-as-obstacle**: The pour boundary and clearances are treated as blocked regions.
   Routes go around pours.
2. **Pour-as-connectivity**: The pour's net is recognized. If two pads are on the same
   net as the pour and both are within pour boundary, the router marks that connection
   as "already routed" (via the pour) and skips it.

Neither approach involves the router modifying the pour geometry.

### 5.4 Orphaned Copper Islands

Copper islands are disconnected fragments of copper pour that are not connected to the
intended net. They can:
- Act as unintended antennas, causing EMI
- Accumulate static charge during manufacturing
- Create unexpected impedance discontinuities
- Violate DRC if they're too small to plate reliably

**Detection algorithm:**

After pour fill, perform a flood-fill connectivity check:
1. Start from each pad/via connected to the pour's net
2. Flood-fill through the filled copper polygon
3. Any filled region not reachable from a connected pad/via is an island
4. Apply the configured island policy (keep, remove, remove-if-small)

**Prevention strategies:**
- Via stitching: ensures all pour regions are connected to ground through vias
- Pour priority ordering: higher-priority pours knockout lower-priority pours,
  preventing fragmentation
- Minimum area threshold: automatically remove islands below a specified area (e.g.,
  below 1 mm^2)

### 5.5 Pour-to-Trace Clearance Management

The clearance between a copper pour and traces on different nets is governed by design
rules. Different clearance values may apply to:
- Pour-to-trace (typically the standard clearance)
- Pour-to-pad (may be larger for creepage compliance)
- Pour-to-pour (for pours on different nets)
- Pour-to-board-edge (typically larger, 0.25-0.5 mm)

The pour fill algorithm inflates each obstacle by the applicable clearance before
subtracting it from the pour polygon. The autorouter must respect the same clearances
when routing near pour boundaries.

### 5.6 Ground Plane Integrity and Return Paths

For signal integrity, the most critical pour interaction is maintaining continuous ground
planes under signal layers. Split ground planes cause:
- Return current discontinuities (current must detour around the split)
- Increased loop area and inductance
- Higher crosstalk between signals sharing the split's gap
- EMI radiation from the enlarged loop

**Key rules for autorouter integration:**
- Never route signal traces across ground plane splits
- Ensure every layer transition (via) has an adjacent ground stitching via
- Prefer routing on layers with adjacent solid ground planes
- Track which ground plane regions are continuous (connectivity analysis)

---

## 6. Implementation Recommendations for autopcb-router

### 6.1 Phased Implementation Plan

**Phase 1: Copper Density Tracking (Foundation)**

Add per-layer, per-tile copper density tracking to the workspace:

```rust
struct CopperDensityMap {
    /// Tile size in grid cells (e.g., 20x20 = 5mm x 5mm at 0.25mm grid)
    tile_size: u32,
    /// Per-layer copper area counters, indexed by [layer][tile_y][tile_x]
    density: Vec<Vec<Vec<f32>>>,  // copper_area / tile_area ratio
}
```

Update density as routes are committed/ripped up. This is the prerequisite for all
copper-balance-aware features. Expose density reports in the routing output.

**Phase 2: Current-Aware Net Width Assignment**

Add current specification to net class config:

```rust
struct NetRoutingConfig {
    // ... existing fields ...
    /// Expected current in Amps. If set, minimum trace width is computed from
    /// IPC-2221 formula.
    pub current_a: Option<f64>,
    /// Allowed temperature rise in degC. Default 10.
    pub temp_rise_c: Option<f64>,
}
```

Before routing, compute minimum width per net from current and temperature rise. Use
this as the effective `width_override`. Route power nets first (priority ordering).

**Phase 3: Layer Balance Cost Term**

Add an optional layer balance bias to the A* cost function:

```
layer_cost(l) = 1.0 + layer_balance_weight * max(0, density[l] - target_density) / target_density
```

This gently biases routing away from over-utilized layers. Weight should be small
(0.1-0.3) to avoid degrading routability.

**Phase 4: Copper Pour Framework**

Implement copper pour filling as a post-routing step:

1. Define pour regions in the PcbIr (boundary polygon, net, priority, fill style)
2. After routing converges, fill pours using boolean polygon operations
   (Clipper2 library for Rust: `clipper2` crate or `geo` crate with boolean ops)
3. Generate thermal reliefs for same-net pads (configurable per pad type)
4. Detect and remove orphaned islands
5. Re-run DRC on combined routes + pours

**Phase 5: Via Stitching**

Post-routing via stitching for ground planes and thermal management:

1. Identify ground pour regions
2. Generate candidate via positions on a grid within pour regions
3. Check DRC clearance against all routes, pads, and existing vias
4. Place vias that pass clearance check
5. Re-fill affected pours

**Phase 6: Copper Thieving / Dummy Fill**

Post-routing, post-pour copper balance optimization:

1. Compute per-layer, per-tile density map
2. For each tile below the target density:
   a. Compute how much additional copper area is needed
   b. Place thieving shapes (dots on a grid) in the tile's free space
   c. Respect clearances to all functional copper
3. Iterate until density is within target range

### 6.2 Cost Function Integration Summary

The extended cost function for the A* router:

```
C(n) = base_cost
     * direction_penalty(layer, move_dir)
     * corridor_penalty(global_route)
     * width_penalty(net)          // NEW: wider nets cost more per cell
     + hist_weight * history[n]
     + pres_fac * max(0, usage[n] - 1)
     + layer_balance_weight * layer_imbalance(layer)  // NEW: density balance
```

Where:
- `width_penalty(net)` = `1.0 + k * (trace_width / grid_resolution - 1.0)` for
  multi-cell-wide traces
- `layer_imbalance(layer)` = `max(0, density[layer] - target) / target`

### 6.3 Data Flow

```
PcbIr (from spec)
  |
  v
RoutingWorkspace
  + CopperDensityMap (per-layer tile density)
  + NetCurrentSpec (from net class config)
  + PourDefinitions (boundary, net, priority, style)
  |
  v
PathFinder Loop (with density-aware cost)
  |
  v
Route Solution (traces + vias)
  |
  v
Post-Routing Pipeline:
  1. Copper Pour Fill (boolean polygon ops, thermal reliefs, island removal)
  2. Via Stitching (ground stitching + thermal vias)
  3. Copper Thieving (density equalization)
  4. DRC Validation (full design rule check)
  |
  v
Final Output (traces + vias + pours + stitching + thieving)
```

### 6.4 Key Dependencies

| Feature | External Dependency | Notes |
|---------|-------------------|-------|
| Polygon boolean ops | `geo` crate + `geo-clipper` or `clipper2` | Required for pour fill |
| Thermal relief geometry | Custom (spoke generation from pad geometry) | No external dep |
| Copper density analysis | Custom (grid tile counting) | No external dep |
| IPC-2221 calculation | Custom (single formula) | No external dep |
| Island detection | Flood fill on polygon connectivity | No external dep |
| Thieving pattern generation | Custom (grid placement with clearance check) | No external dep |

### 6.5 What NOT to Do

1. **Do not try to balance copper during the PathFinder negotiation loop.** Copper
   balance is a global property that conflicts with the local, greedy nature of
   negotiation-based routing. Post-routing optimization is the right approach.

2. **Do not auto-widen traces during routing.** Trace width is a constraint, not a
   variable. Compute the minimum width before routing and use it as a fixed parameter.

3. **Do not fill pours during routing.** Pour filling is expensive (polygon boolean
   operations) and the pour shape changes every time a route changes. Fill once after
   routing converges.

4. **Do not place thermal vias during routing.** Thermal via placement is a post-routing
   operation that depends on the final route geometry.

5. **Do not try to optimize thermal relief parameters automatically.** Thermal relief
   configuration is a design rule, specified by the designer based on manufacturing
   process and soldering method. The router should apply the specified rules, not invent
   new ones.

---

## References

### Standards

- IPC-2221B: "Generic Standard on Printed Board Design" (trace current charts, original)
- IPC-2152: "Standard for Determining Current-Carrying Capacity in Printed Board Design" (2009)
- IPC-6012: "Qualification and Performance Specification for Rigid Printed Boards" (bow/twist limits)
- IPC-7093: "Design and Assembly Process Implementation for Bottom Termination Components" (thermal pad/via guidelines)
- IPC-A-610: "Acceptability of Electronic Assemblies" (thermal pad voiding limits)

### Academic

- McMurchie & Ebeling, "PathFinder: A Negotiation-Based Performance-Driven Router for
  FPGAs," FPGA 1995. (Core negotiation algorithm)
- Kahng et al., "Filling Algorithms and Analyses for Layout Density Control,"
  IEEE TCAD, 2002. (VLSI dummy fill density optimization, sliding window model)
- "A Novel and Unified Full-Chip CMP Model Aware Dummy Fill Insertion Framework,"
  IEEE TCAD, 2020. (Modern density-driven fill with SQP optimization)
- "Manufacturability Aware Routing in Nanometer VLSI," Foundations and Trends in EDA,
  2014. (Comprehensive survey of DFM-aware routing)
- "A Unified Printed Circuit Board Routing Algorithm with Complicated Constraints and
  Differential Pairs," ASP-DAC 2021. (Modern PCB routing with constraints)
- "Multi-agent based minimal-layer via routing algorithm for PCB design," 2025.
  (CBS algorithm adaptation for PCB, layer assignment preprocessing)

### Commercial Tool Documentation

- Altium Designer: Polygon Connect Style rules, Situs Autorouter, ActiveRoute
  (https://www.altium.com/documentation)
- Cadence Allegro: Copper area reporting, layer cost controls
  (https://resources.pcb.cadence.com)
- KiCad: zone_filler.cpp source code
  (https://github.com/KiCad/kicad-source-mirror/blob/master/pcbnew/zone_filler.cpp)
- Freerouting: Open-source autorouter, architectural limitations with copper pours
  (https://github.com/freerouting/freerouting)

### Manufacturing Guidelines

- Sierra Circuits: Balanced copper distribution guide
  (https://www.protoexpress.com/blog/balanced-copper-distribution-and-copper-weight-in-pcbs/)
- Cadence: Copper thieving guide
  (https://resources.pcb.cadence.com/blog/2023-copper-thieving-improves-etch-and-plate-results)
- Eurocircuits: Copper distribution influence on PCB quality
  (https://www.eurocircuits.com/newsletter/the-influence-of-copper-distribution-on-pcb-quality/)
- Altium: Thermal via management
  (https://resources.altium.com/p/management-of-thermal-vias)
- Altium: Thermal relief design guide
  (https://resources.altium.com/p/thermal-relief-design)

### Thermal Analysis

- Thermal conductivity of copper: 385-398 W/m*K
- Thermal conductivity of FR4: 0.3 W/m*K
- Via thermal resistance formula: R = L / (k * A)
- Trace resistance: R = rho * L / (W * T), where rho = 1.7e-8 ohm*m

### Key Formulas

**IPC-2221 trace current capacity:**
```
I = k * dT^0.44 * A^0.725
k = 0.048 (external), 0.024 (internal)
A in sq mils, dT in degC, I in Amps
```

**Trace width from current:**
```
A = (I / (k * dT^0.44))^(1/0.725)
W_mils = A / (thickness_oz * 1.378)
W_mm = W_mils * 0.0254
```

**Single via thermal resistance:**
```
R_theta = L / (k * pi * (D*t - t^2))
L = board thickness (m), k = 385 W/m*K
D = via diameter (m), t = plating thickness (m)
```

**Via array thermal resistance:**
```
R_array = R_single / N_vias
```

**Copper density per tile:**
```
density = copper_area_in_tile / tile_area
Target: 0.30 <= density <= 0.70 per tile per layer
```
