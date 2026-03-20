# Spec Constraints for GPU-Accelerated Autorouting

How an LLM-authored pcbdoc-spec can provide rich constraint information to guide
a GPU-accelerated PathFinder router (negotiation-based, McMurchie-Ebeling with
Bellman-Ford on GPU).

## Design Philosophy

Traditional EDA constraint systems are designed for humans: a handful of rules
applied to broad scopes (net classes, all nets, board-wide). An LLM authoring
the spec has no such ergonomic limitation. It can analyze the schematic, identify
every net's function, and emit per-net constraints that would take a human
engineer hours to specify manually. This shifts constraint specification from
"rules the designer remembers to add" to "exhaustive constraints derived from
circuit analysis."

The key insight: **every constraint the LLM can pre-compute is information the
router does not need to discover by trial and error.** On a GPU, reducing the
search space per-net is multiplicatively beneficial because Bellman-Ford visits
every reachable node -- fewer reachable nodes means fewer GPU threads doing
useful work per dispatch.

---

## 1. Routing Topology Constraints

### Background

Altium already defines `NetTopology` (from `TNetTopology` in the C# codebase)
with these variants:

| Value | Name               | Description |
|-------|--------------------|-------------|
| 0     | Shortest           | MST -- minimum total wire length |
| 1     | Horizontal         | Horizontal chain (left-to-right) |
| 2     | Vertical           | Vertical chain (top-to-bottom) |
| 3     | DaisyChainSimple   | Linear chain through all pins |
| 4     | DaisyChainMidDriven | Chain from middle pin outward |
| 5     | DaisyChainBalanced | Balanced binary tree from source |
| 6     | Starburst          | Star from source to every sink |

The existing IR already has `IrRuleParams::RoutingTopology { topology: NetTopology }`.
The router currently uses `MstDecomposer` (Shortest) for all nets regardless of
this rule. The topology constraint directly controls Steiner tree decomposition.

### How Each Topology Constrains the Router

**Shortest (MST/RSMT)**: Default. `MstDecomposer` produces n-1 subnets from
Prim's MST. No ordering constraint on pin visitation. The GPU router has
maximum freedom -- each subnet is independent. Best for general signals where
wire length is the only concern.

**Star (Starburst)**: One source pin fans out to every sink independently.
Produces n-1 subnets, all sharing a common source point. Critically important
for:
- Clock distribution (FPGA CLKIN -> all flip-flop clock pins)
- Reset networks
- Any signal where simultaneous arrival matters

Router impact: All subnets share the source, creating congestion at the source
pad. The router should reserve a fan-out pattern at the source before routing
individual branches. On GPU, all branches can be routed in parallel since they
only share the source node (mark source as multi-owner).

**Daisy Chain (Simple)**: Linear chain visiting pins in order. Produces n-1
subnets forming a path graph: pin[0]->pin[1]->pin[2]->...->pin[n-1]. Critical
for:
- SPI chip select chains
- I2C bus connections
- JTAG chains (TDI/TDO)
- DDR address/command signals

Router impact: Subnets must be routed in sequence because each subnet's target
is the next subnet's source. The pin ordering is fixed by the LLM based on
physical placement (nearest-neighbor heuristic or schematic-driven order).
This eliminates the Steiner tree decomposition entirely -- the LLM provides
the exact chain order.

**Daisy Chain Mid-Driven**: Chain starts from a middle pin and extends outward
in both directions. For DDR command/address signals where the memory controller
is in the center of the DIMM slot row. Produces two sub-chains, each routable
independently.

**Daisy Chain Balanced**: Binary tree from source. Each level splits the
remaining sinks in half. For clock trees where skew must be minimized. The
tree structure gives the router explicit length-matching groups at each level.

**Horizontal/Vertical**: Constrained chain ordering by physical position.
Useful for bus signals where the LLM knows the optimal routing direction from
component placement.

### LLM-Specific Enhancement: Pin Ordering in Chains

The existing Altium topology types specify the *shape* of the tree but not
the pin visitation order (Altium auto-computes ordering from placement). Since
the LLM has access to both the schematic and placement, it can specify the
exact pin ordering, removing ambiguity from the router.

### Proposed Spec Syntax

```
net "CLK_100M" {
    topology star;
    source_pin U1.CLK_OUT;  // LLM identifies the driver
}

net "SPI_CS" {
    topology daisy_chain;
    chain_order [U1.CS, U3.CS, U5.CS, U7.CS];  // LLM-computed physical order
}

net "DDR_A0" {
    topology daisy_chain_mid_driven;
    source_pin U1.A0;  // memory controller
    // LLM computes that U1 is physically centered among DIMMs
}

net "DDR_CLK" {
    topology balanced_tree;
    source_pin U1.CK;
    // Router builds binary tree with matched-length branches at each level
}
```

### Router Impact on GPU

For star topology, the router can dispatch all n-1 Bellman-Ford invocations in
parallel (one per branch). Each uses the same source but different targets.
The source node is marked as shared-owner so branches don't conflict with each
other at the source.

For daisy chain, subnets are routed sequentially (each depends on the previous),
but within each subnet the GPU Bellman-Ford still accelerates pathfinding.

For balanced tree, the router processes level by level. Within each level, all
branches are independent and can be dispatched in parallel. This is a natural
fit for GPU wavefront parallelism.

---

## 2. Signal Integrity Constraints

### 2.1 Impedance Targets

The LLM can pre-compute target impedance for every net from schematic analysis:
examining termination resistors, driver/receiver impedance specifications from
datasheets, and transmission line requirements.

**How this helps the router**: Impedance determines trace width (for a given
stackup). Instead of using the default width rule, the router knows the exact
width per net, which changes the effective clearance requirements and determines
which channels a trace can fit through.

```
net_class "DDR4_DQ" {
    impedance single_ended 50 ohm;
    // Router derives: for 4-layer 1.6mm FR4, this means ~0.15mm trace width
    // on inner layers, ~0.22mm on outer layers
    trace_width layer_specific {
        "Top Layer"    = 0.22mm;
        "Inner Layer 1" = 0.15mm;
        "Inner Layer 2" = 0.15mm;
        "Bottom Layer" = 0.22mm;
    }
}

net_class "DDR4_CLK" {
    impedance differential 100 ohm;
    diff_pair_gap = 0.15mm;
    // Router derives trace width from impedance + gap + stackup
    trace_width = 0.10mm;
}

net_class "USB3_TX" {
    impedance differential 90 ohm;
    diff_pair_gap = 0.127mm;
    trace_width = 0.127mm;
}
```

**Router impact on GPU**: Layer-specific trace widths change the obstacle
inflation per net. The GPU obstacle bitmap can store per-width-class inflation
or the router can maintain multiple obstacle maps (one per distinct trace width
class). Since typical boards have 3-5 distinct impedance classes, this is a
small number of additional bitmaps.

### 2.2 Maximum Stub Length

Stubs are unterminated branches that cause signal reflections. The LLM can
identify stubs from the topology and compute maximum allowable stub lengths
from signal rise time:

```
max_stub = signal_rise_time / (6 * propagation_delay_per_unit_length)
```

For DDR4 at 2400 MT/s (rise time ~120ps, propagation ~170ps/inch):
`max_stub = 120ps / (6 * 170ps/inch) = 0.118 inch = 3.0mm`

```
net_class "DDR4_DQ" {
    max_stub_length = 3.0mm;
}

net "DDR4_A0" {
    topology daisy_chain;
    max_stub_length = 2.0mm;  // tighter for address lines (full bus rate)
}
```

**Router impact**: During Bellman-Ford, when a candidate path creates a stub
(branch point to pad distance), the router adds a penalty or hard constraint.
For daisy-chain topology, the stub is the short segment from the main chain to
each pad -- the router must keep these under the limit. This is enforceable as
a path-length constraint during detailed routing.

### 2.3 Via Count Limits

Each via introduces ~0.5nH inductance and ~0.3pF capacitance, degrading signal
integrity. The LLM can set per-net via budgets:

```
net_class "DDR4_DQ" {
    max_vias = 2;  // source pad -> one layer transition -> target pad
}

net_class "DDR4_CLK" {
    max_vias = 0;  // clock must stay on one layer (no vias)
}

net "ANALOG_VREF" {
    max_vias = 1;  // precision reference, minimize discontinuities
}

net_class "GPIO" {
    max_vias = 6;  // general purpose, no SI concern
}
```

**Router impact on GPU**: The via budget is a hard constraint during Bellman-Ford.
The search state must track via count: `GridNode { x, y, layer, via_count }`.
This expands the state space by a factor of `max_vias+1`, but for typical
budgets (0-4) this is manageable. When `via_count >= max_vias`, the router
disables layer transitions for that net.

On GPU, this means the distance array becomes 4D:
`dist[x][y][layer][via_count]` with size `W * H * L * V` where V = max via
budget across all nets. For a net with max_vias=2 on a 4-layer board, this is
3x the base state space -- still well within GPU memory for typical boards.

### 2.4 Return Path Continuity (Reference Plane Awareness)

When a signal transitions between layers, the return current must also
transition. If the reference planes on the two layers have different nets
(e.g., transitioning from a GND-referenced layer to a VCC-referenced layer),
the return current has no path, causing EMI and signal integrity issues.

The LLM can analyze the layer stackup and identify which layer transitions
maintain return path continuity:

```
layer_stack {
    "Top Layer"      reference="GND" (Inner Layer 1);
    "Inner Layer 1"  net="GND";
    "Inner Layer 2"  net="VCC";
    "Bottom Layer"   reference="VCC" (Inner Layer 2);
}

net_class "SIGNAL" {
    // LLM computes: Top <-> Inner1 is safe (both reference GND)
    // Top <-> Inner2 breaks return path (GND -> VCC boundary)
    // Top <-> Bottom breaks return path
    // Inner1 <-> Bottom is safe (both reference adjacent planes)
    allowed_via_transitions = [
        (Top, Inner1),       // same reference plane
        (Inner2, Bottom),    // same reference plane
    ];
    // Forbidden: (Top, Inner2), (Top, Bottom), (Inner1, Inner2), (Inner1, Bottom)
}
```

**Router impact on GPU**: The via transition constraint reduces the
`successors()` function's branching factor. Instead of allowing transitions
between any two routing layers, only the allowed pairs generate valid edges.
This directly reduces the number of nodes the GPU Bellman-Ford must visit per
iteration, improving both runtime and signal integrity.

Implementation: encode allowed transitions as a bitmap per net class.
`allowed_transitions[net_class]` is a `L*L` bit matrix where L = layer count.
The GPU shader checks this bitmap before generating via-transition edges.

### 2.5 Crosstalk Spacing Rules

Crosstalk between parallel traces is a function of coupling length and spacing.
The LLM can identify aggressor/victim pairs and specify spacing requirements:

```
// 3W rule: space between trace centers >= 3x trace width
// Reduces crosstalk to < 3% for most geometries
net_class "DDR4_DQ" {
    crosstalk_spacing = 3W;  // 3x trace width center-to-center
}

// Explicit spacing for critical nets
net "ANALOG_IN" {
    isolation_spacing = 1.0mm;  // absolute minimum distance from any digital net
    isolation_from_classes = ["DDR4_DQ", "DDR4_CLK", "USB3_TX"];
}

// Parallel length limit: even at 3W, long parallel runs cause crosstalk
net_class "DDR4_DQ" {
    max_parallel_length = 15mm;  // at 3W spacing
    max_parallel_length_at_1W = 5mm;  // at minimum spacing
}
```

**Router impact on GPU**: Crosstalk spacing inflates the clearance requirement
between specific net class pairs. The GPU obstacle map must account for this:
when routing a DDR4_DQ net, traces belonging to other DDR4_DQ nets appear as
wider obstacles (inflated by 3W instead of standard clearance). This is
implementable as a per-net-class clearance matrix looked up during Bellman-Ford
edge cost computation.

The parallel length limit is harder to enforce during routing (requires tracking
cumulative parallel exposure). This is better handled as a post-route DRC check
with a rip-up-and-reroute penalty in PathFinder: if two traces violate the
parallel length limit, the history cost for the shared corridor is increased.

---

## 3. Timing and Length Constraints

### 3.1 Matched-Length Groups

The LLM can identify matched-length groups from schematic analysis of bus
structures. This goes far beyond Altium's simple "MatchedLengths" rule by
specifying the exact group membership, tolerance, and target reference.

```
matched_group "DDR4_BYTE0" {
    nets = [DQ0, DQ1, DQ2, DQ3, DQ4, DQ5, DQ6, DQ7, DQS0_P, DQS0_N];
    reference_net = DQS0_P;  // match to strobe length
    tolerance = 0.5mm;       // 2.5ps at ~170ps/inch
    // LLM pre-computes: DQS0 route length ~42mm, so all DQ must be 41.5-42.5mm
}

matched_group "DDR4_BYTE1" {
    nets = [DQ8, DQ9, DQ10, DQ11, DQ12, DQ13, DQ14, DQ15, DQS1_P, DQS1_N];
    reference_net = DQS1_P;
    tolerance = 0.5mm;
}

matched_group "DDR4_ADDR" {
    nets = [A0, A1, A2, A3, A4, A5, A6, A7, A8, A9, A10, A11, A12, A13];
    reference_net = CK_P;  // match to clock
    tolerance = 2.0mm;     // address has wider timing margin
}

// Inter-group constraint: byte lane strobes must match clock
matched_group "DDR4_CLK_TO_DQS" {
    nets = [DQS0_P, DQS1_P, DQS2_P, DQS3_P, CK_P];
    reference_net = CK_P;
    tolerance = 5.0mm;  // write leveling compensates, but keep reasonable
}
```

**Router impact on GPU**: Length matching is enforced in two phases:

1. **During routing**: The router tracks cumulative path length per net. When a
   net in a matched group approaches its target length, the router biases the
   Bellman-Ford cost function to avoid further detours. This is implemented as
   a "length penalty" term: when `current_length > target_length - margin`,
   the cost of non-direct-path edges increases exponentially.

2. **Post-route serpentine insertion**: The existing `insert_serpentine()`
   function adds meander to short nets. The LLM pre-computes the target length
   from the reference net's estimated length (based on Manhattan distance plus
   routing overhead factor).

GPU optimization: Length tracking during Bellman-Ford requires per-net state
that is updated atomically. Since Bellman-Ford already tracks distance (which
is path length in grid units), the length constraint can be encoded as a
distance cap: `if dist[node] > max_allowed_distance, don't relax further`.

### 3.2 Maximum Propagation Delay

For nets that are not in matched groups but still have timing requirements:

```
net "RESET_N" {
    max_length = 100mm;     // ~600ps, within reset timing budget
}

net "SPI_CLK" {
    max_length = 50mm;
    min_length = 10mm;      // minimum for proper stub termination
}

net_class "POWER_GOOD" {
    max_length = 200mm;     // slow signals, generous budget
}
```

**Router impact**: `max_length` translates directly to a Bellman-Ford distance
cap. The GPU shader simply skips relaxation when `dist[node] > max_cost`. This
prunes the search space and reduces GPU work. `min_length` is enforced
post-route via serpentine insertion.

### 3.3 Skew Budgets Within Groups

Beyond simple length matching, the LLM can specify asymmetric skew budgets
that reflect the actual timing margins of the interface:

```
matched_group "DDR4_BYTE0" {
    nets = [DQ0, DQ1, DQ2, DQ3, DQ4, DQ5, DQ6, DQ7, DQS0_P, DQS0_N];

    // Per-net skew relative to reference (DQS0_P)
    // Positive = longer than reference, negative = shorter
    skew_budget {
        DQ0 = -0.5mm .. +0.5mm;  // tight: byte lane
        DQ1 = -0.5mm .. +0.5mm;
        // ...
        DQS0_N = -0.1mm .. +0.1mm;  // very tight: diff pair partner
    }
}

// Cross-domain skew
timing_constraint "READ_VALID_WINDOW" {
    from_group = "DDR4_CLK_TO_DQS";
    to_group = "DDR4_BYTE0";
    max_skew = 10mm;  // ~60ps, within read valid window
}
```

**Router impact**: The per-net skew budget gives the router a precise target
range for each net's length. During PathFinder iterations, nets that are outside
their skew budget get higher reroute priority (moved earlier in the net ordering
for the next iteration). The GPU Bellman-Ford uses the skew budget to set both
a lower and upper bound on path length.

### 3.4 GPU Enforcement Strategy for Length Constraints

Length constraints in a GPU Bellman-Ford context are enforced through cost
function modification rather than hard pruning:

```
// Pseudocode for length-aware edge cost
fn edge_cost(node, neighbor, net_config) -> u32 {
    let base = base_edge_cost(node, neighbor);
    let history = history_cost[neighbor];
    let present = present_factor;

    // Standard PathFinder cost
    let pathfinder_cost = (base + history) * present;

    // Length penalty: soft constraint that increases cost as path
    // approaches length limit
    let length_at_neighbor = dist[node] + base;
    let length_penalty = if length_at_neighbor > net_config.max_length {
        INFINITY  // hard cap
    } else if length_at_neighbor > net_config.max_length * 0.9 {
        // Exponential ramp in last 10% of budget
        let overshoot = (length_at_neighbor - net_config.max_length * 0.9)
                       / (net_config.max_length * 0.1);
        (overshoot * overshoot * LENGTH_PENALTY_SCALE) as u32
    } else {
        0
    };

    pathfinder_cost + length_penalty
}
```

This approach works well on GPU because:
- No branching divergence (all threads compute the same penalty formula)
- The penalty is a pure function of local state (distance, config)
- Hard cap at max_length prevents wasted work on already-too-long paths

---

## 4. Physical Constraints

### 4.1 Keepout Regions Per Net Class

Beyond board-level keepouts, the LLM can define signal-class-specific keepout
zones for analog isolation, EMC compliance, or thermal management:

```
keepout "ANALOG_ISLAND" {
    polygon = [(10, 10), (30, 10), (30, 40), (10, 40)];
    applies_to = all_except ["ANALOG_VREF", "ANALOG_IN", "ANALOG_OUT", "AGND"];
    // Digital signals cannot enter this region
    layers = all;
}

keepout "RF_CLEARANCE" {
    polygon = [(50, 20), (70, 20), (70, 35), (50, 35)];
    applies_to = all_except ["RF_IN", "RF_OUT", "RF_GND"];
    layers = ["Top Layer", "Bottom Layer"];
    // Only RF signals route in this zone; digital kept out
}

keepout "POWER_STAGE" {
    polygon = [(0, 0), (15, 0), (15, 15), (0, 15)];
    applies_to = all_except ["VIN", "VOUT", "SW_NODE", "PGND", "BST"];
    layers = all;
    reason = "high-current switching area, keep sensitive signals away";
}
```

**Router impact on GPU**: Per-net-class keepouts are encoded as additional
obstacle layers in the GPU bitmap. The shader indexes into the correct bitmap
based on the net's class. With a small number of distinct keepout sets (typically
3-5), this means 3-5 additional bitmap layers, each `W * H` bits.

Implementation: `obstacle_bitmap[class_id][layer][y][x]` where `class_id`
maps to the net's keepout class. The Bellman-Ford shader receives the
`class_id` as a uniform and indexes into the correct obstacle layer.

### 4.2 Component Courtyard Routing Restrictions

The LLM can analyze component datasheets to determine which components allow
routing under their courtyard and which don't:

```
component U1 {
    // BGA: must route under package for escape
    courtyard_routing = allowed;
    escape_strategy = "dog_bone";  // via-in-pad with dog-bone fanout
}

component L1 {
    // Inductor: no routing underneath (magnetic coupling)
    courtyard_routing = forbidden;
    courtyard_clearance = 1.0mm;  // extra clearance around courtyard
}

component Q1 {
    // MOSFET: no signal routing under thermal pad area
    courtyard_routing = restricted;
    restricted_zone = thermal_pad;  // only power nets allowed in thermal pad zone
    signal_routing = courtyard_edge_only;
}

component J1 {
    // High-speed connector: controlled-impedance routing zone
    courtyard_routing = impedance_controlled_only;
    // Only nets with impedance constraints may route here
}
```

**Router impact**: Component courtyard restrictions are rasterized into the
obstacle bitmap during workspace build. Components with `courtyard_routing =
forbidden` create full-blockage zones. Components with `restricted` create
net-class-specific zones (similar to per-class keepouts above).

### 4.3 Test Point Access Requirements

The LLM can identify nets that need test points and specify access requirements:

```
net_class "DDR4_DQ" {
    test_point = not_required;  // too fast for boundary scan
}

net "VCC_3V3" {
    test_point = required;
    test_point_side = top;
    test_point_min_pad = 0.8mm;
    // Router must leave space for a test point pad on this net
}

net "I2C_SDA" {
    test_point = required;
    test_point_side = either;  // accessible from top or bottom
    test_point_location_preference = near_connector;
}
```

**Router impact**: When a net requires a test point, the router must ensure the
trace passes through a region large enough to accommodate the test point pad.
This is modeled as a "must-visit waypoint" constraint: the Bellman-Ford
pathfinding includes intermediate waypoint targets. The router finds the
shortest path source -> waypoint -> target rather than source -> target directly.

### 4.4 Manufacturing Constraints

The LLM can specify DFM constraints that affect routing:

```
manufacturing {
    // Acid trap prevention: no acute angles < 90 degrees
    min_trace_angle = 90deg;  // Router uses 45-degree or rounded corners

    // Copper balance: routing should roughly equalize copper density per layer
    copper_balance_target = 40%;  // aim for 40% copper fill per layer
    copper_balance_tolerance = 15%;  // 25% to 55% acceptable range

    // Teardrop requirements: all pad-to-trace junctions need teardrops
    teardrops = required;
    teardrop_length = 0.3mm;

    // Minimum feature size affects via and trace selection
    min_annular_ring = 0.1mm;
    min_drill = 0.2mm;
    min_trace_width = 0.1mm;
    min_trace_spacing = 0.1mm;
}
```

**Router impact on GPU**: These constraints are mostly post-route DRC checks,
but some affect routing directly:

- `min_trace_angle`: Enforced by the `CornerStyle` in routing config. Already
  implemented.
- `copper_balance_target`: Can be encoded as a per-layer congestion bias. If
  a layer has less than target copper, its base routing cost is reduced; if
  more, increased. This steers the router toward balanced copper fill.
- `teardrops`: Affects clearance calculation near pads (teardrop geometry
  requires additional space). The obstacle inflation around pads includes
  teardrop envelope.

---

## 5. Power Distribution Constraints

### 5.1 Current Capacity Requirements

The LLM can compute required trace widths from current analysis:

```
net "VCC_5V" {
    max_current = 3.0A;
    // LLM computes: for 1oz copper, 10C rise, outer layer:
    //   width = I / (k * dT^0.44 * A^0.725)  (IPC-2152)
    //   ~1.5mm trace width required
    trace_width_override = 1.5mm;

    // Prefer wide traces on outer layers (better thermal dissipation)
    preferred_layers = ["Top Layer", "Bottom Layer"];
}

net "VCC_1V0_CORE" {
    max_current = 8.0A;
    // Too much current for traces -- use copper pour
    routing_style = polygon_pour;
    pour_min_width = 3.0mm;
}

net "GND" {
    routing_style = plane;  // full copper plane, don't route as traces
    plane_layers = ["Inner Layer 1"];
}
```

**Router impact**: Power nets with `trace_width_override` are routed with wider
traces, which require more clearance and can only fit through wider channels.
The router's obstacle inflation is per-net, using the net's trace width. Nets
with `routing_style = polygon_pour` or `plane` are excluded from the trace
router entirely -- they are handled by the polygon fill engine separately.

For GPU routing, power nets with wide traces reduce the effective routing
channel width for signal nets. The obstacle bitmap for power-net traces is
wider, leaving less room for signal routing. The router should route power nets
first (higher priority), then signal nets route around them.

### 5.2 Via Current Limits

```
net "VCC_3V3" {
    max_current = 2.0A;
    // Standard via (0.3mm drill, 1oz copper): ~1A capacity
    // Need minimum 2 vias for layer transition
    min_vias_per_transition = 2;
    via_size = 0.4mm;  // larger via for better current handling
}

net "VCC_1V0" {
    max_current = 5.0A;
    min_vias_per_transition = 6;  // via array for high current
    via_array_pattern = "2x3";   // 2 columns x 3 rows of vias
    via_pitch = 0.8mm;           // minimum center-to-center spacing
}
```

**Router impact**: Multi-via transitions require more board area at via
locations. The router treats a via array as a single "super-via" obstacle that
occupies a larger footprint than a single via. The via cost model in the GPU
shader accounts for the array size: `via_cost = base_cost * num_vias +
area_penalty * array_footprint`.

### 5.3 Thermal Relief Patterns

```
net "GND" {
    thermal_relief {
        style = "four_spoke";    // 4 connections at 90-degree intervals
        spoke_width = 0.3mm;
        air_gap = 0.3mm;
        // Applies to all pad connections to GND plane
    }
}

net "VCC_3V3" {
    thermal_relief {
        style = "direct_connect";  // no thermal relief, maximum current
        // For high-current pads: bypass capacitors, voltage regulator output
        applies_to_components = [C1, C2, C3, U2];
    }
}
```

**Router impact**: Thermal relief patterns affect pad connectivity. A pad with
thermal relief has limited connection points (only at the spoke locations),
which constrains the routing approach to that pad. The router's pin access
computation (`pin_accesses()` in `obstacles.rs`) must account for thermal relief
geometry: only spoke-aligned directions are valid access points.

### 5.4 Split Plane Awareness

The LLM can identify power plane splits and their impact on signal routing:

```
plane_split "INNER1_SPLIT" {
    layer = "Inner Layer 1";
    regions = [
        { net = "GND",       polygon = [(0,0), (100,0), (100,50), (0,50)] },
        { net = "GND_ANALOG", polygon = [(0,50), (100,50), (100,100), (0,100)] },
    ];
    gap = 0.5mm;  // gap between split regions
}

// Signals that cross the split boundary need special handling
constraint "NO_CROSS_SPLIT" {
    // No signal traces should cross the GND/GND_ANALOG boundary
    // on layers that reference Inner Layer 1
    affected_layers = ["Top Layer"];  // Top references Inner Layer 1
    forbidden_crossings = [
        { from_region = "GND", to_region = "GND_ANALOG" }
    ];
    // Signal traces on Top Layer must not cross y=50mm
}

// Exception: signals that intentionally bridge domains
net "ADC_IN" {
    may_cross_splits = true;
    split_crossing_strategy = "stitching_caps";
    // Router adds stitching capacitor placement near crossing point
}
```

**Router impact on GPU**: Split plane boundaries are encoded as linear keepout
barriers in the obstacle bitmap for affected signal layers. The GPU shader
treats the split boundary as a wall that signal traces cannot cross. This is a
simple line rasterization into the obstacle bitmap.

For nets that may cross splits (`may_cross_splits = true`), the barrier is
removed from the obstacle map for that specific net. This requires per-net
obstacle map variants, which is handled the same way as per-class keepouts
(indexed by net class).

---

## 6. LLM-Computed Routing Hints

Beyond formal constraints, the LLM can provide "soft" hints that improve
routing quality without being hard requirements:

### 6.1 Net Priority and Ordering

```
// LLM analyzes schematic and assigns routing priorities
net_priority {
    // Route in this order (highest priority first)
    critical = ["DDR4_CLK_P", "DDR4_CLK_N"];   // clock first
    high     = ["DDR4_DQS*", "USB3_TX*"];       // strobes and high-speed
    medium   = ["DDR4_DQ*", "DDR4_A*"];          // data and address
    low      = ["SPI_*", "I2C_*", "GPIO_*"];     // low-speed interfaces
    power    = ["VCC_*", "GND"];                  // power last (wide traces)
}
```

**Router impact**: Net ordering directly affects PathFinder convergence. Routing
critical nets first in each iteration ensures they get the best paths. The LLM's
priority assignment replaces the router's default heuristic (by fanout, HPWL,
etc.) with domain-specific knowledge.

### 6.2 Layer Assignment Hints

```
net "DDR4_CLK_P" {
    preferred_layers = ["Inner Layer 1"];  // stripline for better SI
    avoid_layers = ["Top Layer", "Bottom Layer"];  // avoid microstrip
}

net_class "DDR4_DQ" {
    preferred_layers = ["Top Layer", "Inner Layer 2"];
    // LLM knows: byte lanes route on specific layers to avoid crossings
    layer_assignment_hint = {
        byte0 = "Top Layer";
        byte1 = "Inner Layer 2";
    }
}
```

**Router impact on GPU**: Layer preferences are encoded as layer-specific cost
biases in the Bellman-Ford edge cost function. Preferred layers have lower base
cost; avoided layers have higher cost. This is a simple uniform parameter that
the GPU shader uses to scale edge weights:

```
cost_multiplier = layer_preference[net_class][layer];
// preferred: 1.0, neutral: 1.5, avoid: 5.0, forbidden: INFINITY
```

### 6.3 Routing Channel Hints

The LLM can identify routing channels from component placement analysis:

```
routing_channel "NORTH_BUS" {
    entry = (20, 45);   // channel entry point
    exit = (80, 45);    // channel exit point
    width = 5mm;        // available channel width
    nets = ["DDR4_A0" .. "DDR4_A13", "DDR4_BA0" .. "DDR4_BA2"];
    // LLM computed: these 16 nets must pass through a 5mm gap between U1 and U2
    // At 0.15mm trace + 0.15mm space = 0.3mm pitch, can fit 16 nets in 4.8mm
}
```

**Router impact**: Channel hints pre-compute what the global router would
discover through congestion analysis. The router can use channel hints to:
1. Pre-assign nets to specific channels before global routing
2. Set channel-aware congestion limits (capacity = channel_width / pitch)
3. Order nets within a channel to minimize crossings (bus ordering)

This reduces PathFinder iterations needed for convergence because the initial
routing is already informed by channel capacity.

### 6.4 Fanout Strategy

```
component U1 {
    package = "BGA-256";
    fanout_strategy {
        outer_ring = direct_escape;     // outer pins route directly
        inner_ring = dog_bone;          // inner pins use dog-bone vias
        center_pins = via_in_pad;       // center pins use via-in-pad
        // LLM computes escape routing order from BGA geometry
        escape_layer_map = {
            ring_1 = "Top Layer";       // outermost ring escapes on top
            ring_2 = "Top Layer";       // second ring also escapes on top
            ring_3 = "Inner Layer 1";   // third ring escapes to inner layer
            ring_4 = "Inner Layer 2";   // innermost escapes to deeper layer
        }
    }
}
```

**Router impact**: Fanout strategy pre-determines the layer transition pattern
for BGA components. Instead of the router discovering the escape pattern through
trial and error, the LLM specifies it. This converts the NP-hard BGA escape
routing problem into a pre-solved constraint that the detailed router simply
follows.

---

## 7. Summary: Constraint Impact on GPU Router Search Space

| Constraint Type | Search Space Reduction | GPU Implementation |
|----------------|----------------------|-------------------|
| Topology (star, chain, tree) | Eliminates Steiner tree discovery; fixes subnet decomposition | Per-net subnet list pre-computed on CPU |
| Impedance / trace width | Reduces viable channels per net class | Per-class obstacle bitmaps (3-5 variants) |
| Max stub length | Limits detour budget for daisy chain branches | Distance cap in Bellman-Ford |
| Via count limit | Prunes layer transitions | 4D distance array (adds via_count dimension) |
| Return path / via transitions | Reduces layer transition options | Bitmap mask on via edges |
| Crosstalk spacing | Inflates inter-net clearance for specific pairs | Per-class clearance in edge cost |
| Matched length groups | Bounds path length per net | Distance cap + length penalty in cost |
| Max/min propagation delay | Hard bounds on path length | Distance cap in Bellman-Ford |
| Per-class keepouts | Removes routing resources for specific nets | Per-class obstacle bitmaps |
| Component courtyard | Blocks routing through component areas | Rasterized into obstacle bitmap |
| Current capacity / width | Forces wider traces, reducing channels | Per-net trace width in cost computation |
| Split plane barriers | Creates impassable walls for signal nets | Line rasterization in obstacle bitmap |
| Layer preferences | Biases layer assignment | Cost multiplier per layer per net class |
| Channel hints | Pre-solves global routing | Congestion pre-seeding |
| Fanout strategy | Pre-solves BGA escape | Fixed layer assignments at component pads |
| Net priority | Controls PathFinder iteration order | CPU-side net ordering |

### Quantitative Estimate

For a typical DDR4 board (500 nets, 4 layers, 1000x1000 grid):

- **Without LLM constraints**: 500 nets x 4M grid cells x ~6 iterations = ~12B
  Bellman-Ford relaxations total across all PathFinder iterations.
- **With LLM constraints**: Via limits reduce reachable cells by ~50% (2
  layers instead of 4 for most nets). Layer preferences reduce another ~25%.
  Keepouts reduce ~10%. Net total: ~3B relaxations, a **4x reduction** in GPU
  work.

The reduction is even more significant for convergence: with pre-computed
topologies and channel hints, PathFinder typically converges in 10-15 iterations
instead of 30-50, giving an additional **2-3x improvement** in total runtime.

---

## 8. Implementation Roadmap

### Phase 1: Core Constraints (Extend Existing IR)
- Topology constraint consumption (use existing `IrRuleParams::RoutingTopology`)
- Via count limits (new `IrRuleParams` variant or spec-only constraint)
- Max/min length (extend `IrRuleParams::MatchedLengths` with group membership)
- Net priority (extend `IrRuleParams::RoutingPriority` with LLM-computed values)

### Phase 2: SI-Aware Constraints (New IR Extensions)
- Per-net impedance targets and layer-specific trace widths
- Via transition restrictions (return path awareness)
- Crosstalk spacing rules (per-class clearance matrix)
- Stub length limits

### Phase 3: Physical Constraints (Spec Language Extensions)
- Per-class keepout zones
- Component courtyard routing restrictions
- Split plane awareness
- Channel hints and fanout strategies

### Phase 4: GPU Integration
- Encode Phase 1-3 constraints into GPU-friendly data structures
- Per-class obstacle bitmaps (indexed by net class ID)
- Via transition bitmaps (L x L per net class)
- Length caps in Bellman-Ford distance initialization
- Layer cost multipliers in uniform buffer

Each phase builds on the previous. Phase 1 requires only extensions to existing
`IrRuleParams` and `RoutingPolicy`. Phase 2 requires new spec language constructs
and IR types. Phase 3 requires spec language extensions for physical constraints.
Phase 4 translates all constraints into GPU buffer layouts.
