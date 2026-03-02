# SchDoc Placement & Wire Routing

Design notes for automatic schematic placement and wire routing in SchDoc files.
This is a fundamentally different problem from PCB placement.

## PCB vs Schematic Placement

| Property | PcbDoc (PCB) | SchDoc (Schematic) |
|----------|-------------|-------------------|
| **Primary goal** | Manufacturability | Readability |
| **Optimization target** | Minimize wire length (HPWL) | Minimize wire crossings |
| **Signal flow** | Not directional | Left→right convention |
| **Wiring** | Any-angle (layer dependent) | Orthogonal only (H/V) |
| **Components** | Physical footprints with clearance | Symbols with pins on edges |
| **Design rules** | Clearance, width, annular ring | Visual spacing, alignment |
| **Power** | Power planes, traces | Distributed VCC/GND symbols |
| **Connectivity** | Physical copper | Wires + net labels + ports |
| **Hierarchy** | Single board | Multi-sheet with ports |
| **Grid** | Continuous coordinates | Snap grid (10mil typical) |


## SchDoc Record Types for Placement

From `docs/dxp/schematic-records.md` and the codebase:

### Placeable Objects

| RECORD | Type | Placement Fields | Notes |
|--------|------|-----------------|-------|
| 1 | **SchComponent** | `LOCATION.X/Y`, `ORIENTATION` (0-3), `CURRENTPARTID`, `ISMIRRORED` | Placed symbol instance |
| 27 | **SchWire** | `X1/Y1...XN/YN` (vertex list), `LOCATIONCOUNT` | Electrical connection |
| 26 | **SchBus** | Same as Wire | Multi-signal bus |
| 25 | **SchNetLabel** | `LOCATION.X/Y`, `TEXT`, `ORIENTATION` | Net name annotation |
| 17 | **SchPowerObject** | `LOCATION.X/Y`, `TEXT`, `STYLE`, `ORIENTATION` | VCC/GND/power symbols |
| 29 | **SchJunction** | `LOCATION.X/Y` | Wire junction dot |
| 37 | **SchBusEntry** | `LOCATION.X/Y`, `CORNER.X/Y` | Bus tap |
| 18 | **SchPort** | `LOCATION.X/Y`, `NAME`, `IO_TYPE`, `STYLE` | Sheet-to-sheet connector |

### Coordinate System

- DXP fractional encoding: `integer * 100,000 + fractional`
- 10,000 internal units = 1 mil
- Typical grid: 10 mil snap (= 100,000 internal units)
- Sheet sizes: A4 (1150×760 DXP), A3 (1550×1150 DXP), custom
- Origin: bottom-left

### Component Orientation

```
ORIENTATION=0  →  0° (default, pins as drawn)
ORIENTATION=1  →  90° CCW
ORIENTATION=2  →  180°
ORIENTATION=3  →  270° CCW (= 90° CW)
ISMIRRORED=T   →  horizontal flip (X-axis mirror)
```

### Wire Vertices

Wires are polylines with 2+ vertices, always orthogonal:
```
RECORD=27|LOCATIONCOUNT=4|X1=300|Y1=200|X2=400|Y2=200|X3=400|Y3=100|X4=500|Y4=100
```
This traces: right → down → right (two bends).


## The Schematic Layout Problem

### Input
- **Netlist**: components + nets (which pins connect to which nets)
- **Component symbols**: bounding boxes + pin positions
- **User constraints**: signal flow, grouping, alignment preferences
- **Sheet size**: available drawing area

### Output
- Component positions `(x, y, orientation)` on grid
- Wire routes: orthogonal polylines connecting pins
- Net labels / power symbols placed for clarity
- Junctions at wire crossing points

### Quality Metrics (in priority order)
1. **Wire crossing count** — fewer crossings = more readable
2. **Signal flow direction** — inputs left, outputs right
3. **Component grouping** — related components visually clustered
4. **Wire bend count** — fewer bends = cleaner routing
5. **Total wire length** — shorter = less visual noise
6. **Alignment** — components on grid, aligned in rows/columns
7. **Symmetry** — differential pairs, push-pull stages drawn symmetrically


## Algorithms for Schematic Placement

### 1. Sugiyama Layered Graph Drawing (Primary)

The classic algorithm for DAG visualization. Natural fit for schematic signal
flow (left→right = layer assignment).

**Four phases**:

```
Phase 1: Cycle Removal
    Break feedback loops by reversing minimum edges
    (greedy feedback arc set, O(V+E))
    → Produces a DAG

Phase 2: Layer Assignment (= column assignment)
    Topological sort → assign each component to a column
    Components with no inputs → column 0 (leftmost)
    Components fed by column 0 → column 1
    ...and so on
    → Components assigned to vertical columns

Phase 3: Crossing Minimization
    Within each column, order components to minimize
    edge crossings with adjacent columns
    Methods: barycenter heuristic, median heuristic
    (iterate left→right, right→left until stable)
    → Components ordered within columns

Phase 4: Coordinate Assignment
    Assign exact (x, y) positions respecting grid
    Minimize total edge length while maintaining ordering
    → Final component positions
```

**Rust implementation**: `rust-sugiyama` crate (uses petgraph, implements
all four phases with barycenter and median heuristics).

**Mapping to SchDoc**:
- Nodes = components (U1, U2, R1, C1, ...)
- Edges = nets (directed by signal flow: output pin → input pin)
- Layers = columns (left-to-right)
- Y-ordering within column = vertical position

**Signal flow direction**: Determined by pin electrical types:
- Output/Bidirectional pins → source
- Input/Passive pins → sink
- Power pins → separate (VCC/GND symbols, not in signal flow graph)

### 2. Force-Directed Placement (Complement)

For components that don't have clear signal flow (e.g., passive networks,
analog circuits), use force-directed placement as a fallback:

- **Spring forces**: Connected components attract (net springs)
- **Repulsion forces**: All components repel (prevent overlap)
- **Alignment forces**: Pull components toward grid lines
- **Group forces**: Pull grouped components together

This maps directly to solverang constraints:
- Spring = HPWL residual (soft)
- Repulsion = ComponentClearance (hard)
- Alignment = grid-snap equality constraints
- Group = NearConstraint

### 3. Barycenter/Median Ordering (Within Columns)

After Sugiyama assigns columns, order components within each column:

**Barycenter**: Position = average of connected neighbors' positions
```
y_order(v) = (1/|N(v)|) × Σ_{u ∈ N(v)} y(u)
```

**Median**: Position = median of connected neighbors' positions
(more robust to outliers than barycenter)

Iterate: fix odd columns → reorder even columns → fix even → reorder odd.
Repeat until stable or max iterations.


## Algorithms for Wire Routing

After components are placed, connect pins with orthogonal wires.

### 1. A* Orthogonal Pathfinding

For each net, find the shortest orthogonal path between pins:

```
Grid = snap grid of the schematic
Obstacles = component bounding boxes + already-routed wires
For each net (sorted by: 2-pin first, short distance first):
    source = output pin location
    sink = input pin location
    path = A_star(source, sink, grid, obstacles)
    Add path as SchWire record
    Add path to obstacle set
```

**Cost function for A***:
```
g(node) = distance so far + bend_penalty × bend_count
h(node) = manhattan_distance(node, target)
```

The `bend_penalty` encourages straight routes (fewer bends = cleaner schematic).

### 2. Channel Routing

For bus-like parallel connections, use channel routing:

```
    ┌─────────┐              ┌─────────┐
    │   U1    │              │   U2    │
    │    pin1 ├──────────────┤ pin1    │
    │    pin2 ├──────────────┤ pin2    │
    │    pin3 ├──────────────┤ pin3    │
    └─────────┘              └─────────┘
```

When multiple pins on the same side connect to pins on another component's
same side, route them as parallel horizontal wires (no crossings needed).

### 3. Net Label Insertion

For nets that span long distances or cross functional groups, insert net
labels instead of routing long wires:

```
Rule: If manhattan_distance(pin_A, pin_B) > threshold (e.g., 500 mil)
      AND the wire would cross more than 2 other wires
      THEN: place NetLabel at pin_A, place NetLabel at pin_B
            (same TEXT = same net, no wire needed)
```

### 4. Power Symbol Placement

Power nets (VCC, GND, 3V3, 5V) use SchPowerObject symbols placed at each
power pin, rather than routing wires to a central point:

```
For each power net:
    For each pin on that net:
        Place SchPowerObject at pin location
        Style: VCC → bar on top, GND → triangle pointing down
        Orientation: auto (based on pin direction)
```

### 5. libavoid / Adaptagrams (Reference)

The Adaptagrams project provides `libavoid`, a C++ library for orthogonal
obstacle-avoiding connector routing. It solves exactly our wire routing
problem. Key papers:
- "Orthogonal Connector Routing" (Wybrow, Marriott, Stuckey)
- "Incremental Connector Routing" (same authors)

While libavoid is C++, the algorithmic approach (visibility graph + Dijkstra
on orthogonal grid) can be implemented in Rust.


## How Solverang Fits for SchDoc

Solverang's role is **different** from PcbDoc:

### What Solverang Does

1. **Position refinement after Sugiyama**: Sugiyama gives layer/order assignments,
   solverang computes exact (x, y) coordinates that satisfy:
   - Grid alignment (snap to 10-mil grid)
   - Minimum spacing between components
   - Group clustering (related components together)
   - Symmetry constraints (diff pairs drawn symmetrically)
   - Custom alignment (user says "align U1, U2, U3 vertically")

2. **Constraint satisfaction for user specs**: The LLM agent specifies:
   - "U1 and U2 should be vertically aligned"
   - "The power section goes in the top-left corner"
   - "R1-R4 should be in a row with 200mil spacing"

3. **DRC-like verification**: Check that the schematic satisfies visual
   quality rules (no overlapping symbols, minimum pin-to-pin wire clearance,
   consistent spacing).

### What Solverang Does NOT Do

- **Layer assignment** (Sugiyama Phase 2) — discrete, not continuous
- **Crossing minimization** (Sugiyama Phase 3) — combinatorial optimization
- **Wire routing** (A*, channel routing) — pathfinding, not optimization
- **Net label insertion** — heuristic decision, not constraint problem


## SchDoc Constraint Types for Solverang

### Entities

```rust
pub struct SchComponent {
    id: EntityId,
    x: ParamId,            // center X (solvable)
    y: ParamId,            // center Y (solvable)
    column: usize,          // assigned by Sugiyama (fixed)
    order_in_column: usize, // assigned by crossing minimization (fixed)
    half_width: f64,
    half_height: f64,
    designator: String,
    params: [ParamId; 2],
}
```

### Constraints

| Constraint | Type | Residual | Purpose |
|-----------|------|----------|---------|
| **GridSnap** | Equality | `x mod grid_size = 0, y mod grid_size = 0` | Snap to grid |
| **ColumnAlignment** | Equality | `x = column_x` | Fix X to column |
| **VerticalSpacing** | Inequality | `y_above - y_below - combined_hh ≥ gap` | Min vertical gap |
| **HorizontalSpacing** | Inequality | `x_right - x_left - combined_hw ≥ gap` | Min horizontal gap |
| **VerticalAlign** | Equality | `x_A = x_B` | Align two components |
| **HorizontalAlign** | Equality | `y_A = y_B` | Align two components |
| **GroupCluster** | Inequality | `dist(comp, group_center) ≤ max_radius` | Keep group together |
| **SymmetryX** | Equality | `(y_A + y_B) / 2 = axis_y` | Mirror about X axis |
| **SymmetryY** | Equality | `(x_A + x_B) / 2 = axis_x` | Mirror about Y axis |
| **WireLengthMin** | Soft | `weight × manhattan_dist(pin_A, pin_B)` | Minimize wire length |

**Grid snap constraint**:
Grid snap is tricky for continuous solvers. Use a smooth penalty:
```
r = sin(2π × x / grid_size)    // zero at every grid point
```
This has the same flavor as `sin(2θ)` for rotation — zero at all valid values.
The solver naturally converges to the nearest grid point.


## Multi-Stage Pipeline for SchDoc

```
╔══════════════════════════════════════════════════════════════════╗
║  PHASE 0: Netlist Analysis                                       ║
║  • Parse SchDoc → component list + netlist                      ║
║  • Identify signal flow direction per net (output→input pins)   ║
║  • Identify power nets (VCC, GND, etc.)                         ║
║  • Identify functional blocks from connectivity                  ║
║  • Build directed signal flow graph (DAG after cycle removal)   ║
║  Method: Graph analysis + heuristics                             ║
╠══════════════════════════════════════════════════════════════════╣
║  PHASE 1: Layer Assignment (Sugiyama)                            ║
║  • Break cycles (greedy feedback arc set)                       ║
║  • Assign components to columns (topological sort)              ║
║  • Respect user constraints ("U1 goes in column 2")             ║
║  Method: Sugiyama Phase 1-2 (rust-sugiyama or custom)           ║
╠══════════════════════════════════════════════════════════════════╣
║  PHASE 2: Crossing Minimization                                  ║
║  • Order components within each column                           ║
║  • Minimize wire crossings between adjacent columns             ║
║  • Barycenter or median heuristic, iterated                     ║
║  Method: Sugiyama Phase 3                                        ║
╠══════════════════════════════════════════════════════════════════╣
║  PHASE 3: Position Assignment (Solverang)                        ║
║  • Assign exact (x, y) on grid                                  ║
║  • Column X = fixed from layer assignment                        ║
║  • Y positions = solvable with spacing + alignment constraints  ║
║  • Group clustering, symmetry, custom alignment                  ║
║  Method: Solverang ConstraintSystem (AutoSolver→LM) with       ║
║          GridSnap + spacing constraints                         ║
╠══════════════════════════════════════════════════════════════════╣
║  PHASE 4: Wire Routing                                           ║
║  • Route orthogonal wires between connected pins                ║
║  • Insert net labels for long-distance connections               ║
║  • Insert junctions where wires cross on same net               ║
║  • Place power symbols at power pins                             ║
║  Method: A* orthogonal pathfinding + channel routing            ║
╠══════════════════════════════════════════════════════════════════╣
║  PHASE 5: Beautification                                         ║
║  • Adjust wire paths to minimize bends                           ║
║  • Align parallel wires to equal spacing                         ║
║  • Center labels on wires                                        ║
║  • Adjust component spacing for visual balance                   ║
║  Method: Solverang ConstraintSystem refinement + heuristics      ║
╚══════════════════════════════════════════════════════════════════╝
```


## Spec Language for SchDoc Placement

### Grammar Extension

```
// my-design.schdoc-spec

schematic {
    target: "my-design.SchDoc"

    // ── Signal flow ─────────────────────────────────
    flow: left_to_right              // or right_to_left, top_to_bottom

    // ── Sheet configuration ─────────────────────────
    sheet: A3                        // or A4, custom(1500, 1000)
    grid: 10mil

    // ── Functional groups ───────────────────────────
    group power_input {
        components: [J1, F1, D1, U1]
        style: vertical              // stack vertically within group
        label: "Power Input"         // optional group label
    }

    group mcu {
        components: [U2, Y1, C1, C2, C3, C4, R1, R2]
        style: radial                // center on U2, passives around it
        center: $U2
    }

    group output {
        components: [J2, J3, R3, R4, D2, D3]
        style: vertical
    }

    // ── Signal flow sequence ────────────────────────
    // Left-to-right order of groups
    sequence: [$power_input, $mcu, $output]

    // ── Component-level constraints ─────────────────
    align $R1, $R2, $R3, $R4 {
        axis: horizontal             // all at same Y
        gap: 200mil                  // 200mil between each
    }

    below $Y1, $U2 { gap: 300mil }  // crystal below MCU

    // ── Symmetry ────────────────────────────────────
    symmetric $R1, $R2 { axis: horizontal, center: $U2 }

    // ── Routing preferences ─────────────────────────
    routing {
        style: orthogonal
        max_wire_length: 2000mil     // beyond this, use net labels
        power_style: symbols         // VCC/GND as symbols, not wires
        bus_routing: grouped         // parallel bus wires
    }

    // ── Spacing ─────────────────────────────────────
    spacing {
        component: 200mil            // minimum between component BBs
        wire: 50mil                  // minimum between parallel wires
        pin_to_wire: 100mil          // minimum pin-end to crossing wire
    }
}
```

### New AST Nodes

```rust
pub enum SpecItem {
    // existing
    Import(ImportDecl),
    LetBinding(LetBinding),
    Component(ComponentDecl),
    Footprint(FootprintDecl),
    Project(ProjectDecl),
    Placement(PlacementDecl),
    Rule(RuleDecl),
    // new
    Schematic(SchematicDecl),
}

pub struct SchematicDecl {
    pub body: Vec<Spanned<SchematicItem>>,
}

pub enum SchematicItem {
    Property(Property),           // target, flow, sheet, grid
    LetBinding(LetBinding),
    Group(GroupDecl),              // shared with PcbDoc
    Sequence(Vec<Spanned<Expr>>), // signal flow order
    Align(AlignDecl),
    Symmetric(SymmetricDecl),
    Below(DirectionalDecl),       // shared directional constraints
    Above(DirectionalDecl),
    LeftOf(DirectionalDecl),
    RightOf(DirectionalDecl),
    Routing(Object),
    Spacing(Object),
}

pub struct AlignDecl {
    pub components: Vec<Spanned<Expr>>,
    pub props: Object,             // axis, gap
}

pub struct SymmetricDecl {
    pub a: Spanned<Expr>,
    pub b: Spanned<Expr>,
    pub props: Object,             // axis, center
}
```

### New Model Types

```rust
pub struct SchDocSpec {
    pub schematic: Option<SchematicSpec>,
}

pub struct SchematicSpec {
    pub target: Option<String>,
    pub flow: SignalFlow,
    pub sheet: SheetSize,
    pub grid: Coord,
    pub groups: Vec<GroupSpec>,
    pub sequence: Vec<String>,     // group name ordering
    pub alignments: Vec<AlignSpec>,
    pub symmetries: Vec<SymmetrySpec>,
    pub directional: Vec<DirectionalSpec>,
    pub routing: RoutingSpec,
    pub spacing: SpacingSpec,
}

pub enum SignalFlow {
    LeftToRight,    // default
    RightToLeft,
    TopToBottom,
    BottomToTop,
}

pub enum SheetSize {
    A4, A3, A2, A1, A0,
    Letter, Legal, Tabloid,
    Custom { width: Coord, height: Coord },
}

pub struct RoutingSpec {
    pub style: WireStyle,           // Orthogonal (only option for now)
    pub max_wire_length: Option<Coord>,
    pub power_style: PowerStyle,    // Symbols, Wires, Mixed
    pub bus_routing: BusStyle,      // Grouped, Individual
}

pub struct SpacingSpec {
    pub component: Coord,           // min component-to-component
    pub wire: Coord,                // min parallel wire spacing
    pub pin_to_wire: Coord,         // min pin-end to crossing wire
}
```


## Integration with Existing `place` Op

The existing `place` op in `ops-lang-spec.md` already supports anchor-based
placement:

```
place $pin1 { on: $rect.top, at: start }
place $pin2 { on: $rect.top, after: $pin1, gap: 20mil }
```

This can be extended for schematic layout:

```
// Place component relative to another
place $R1 { on: $U2.right, after: $U2.pin[3], gap: 200mil }

// Place wire connecting two pins
wire { from: $U1.pin[1], to: $U2.pin[5] }

// Place net label
net_label "SPI_CLK" { at: $U1.pin[12], orientation: right }

// Place power symbol
power_port "VCC" { at: $U1.pin[1], style: bar }
```


## LLM Agent for Schematic Layout

An LLM agent generating schematic placement specs would reason differently
from PCB placement:

```markdown
## Schematic Analysis

I see this is an STM32F407 development board schematic:

1. **Signal flow**: Power input → voltage regulation → MCU → peripherals → connectors
2. **Functional blocks**:
   - Power: barrel jack J1 → reverse protection D1 → LDO U1 → decoupling
   - MCU core: STM32 U2 + crystal Y1 + bypass caps
   - USB: connector J2 → ESD U3 → MCU USB pins
   - HDMI: connector J3 → level shifter U4 → MCU HDMI pins
   - Debug: SWD header J4 → MCU debug pins

3. **Layout strategy**:
   - Left column: power input (J1, D1, U1)
   - Center column: MCU (U2) with crystal below, caps around
   - Right column: output connectors (J2, J3, J4) with their interface ICs
   - Power symbols throughout (no long VCC/GND wires)

4. **Sheet**: A3 landscape (enough room for ~50 components)
```


## Comparison: PcbDoc vs SchDoc Solverang Usage

| Aspect | PcbDoc | SchDoc |
|--------|--------|--------|
| **Phases using solverang** | Global placement + refinement | Position assignment + beautification |
| **Variables** | (x, y, θ) per component | (x, y) per component (θ from Sugiyama) |
| **Hard constraints** | Board containment, clearance, edge | Column alignment, grid snap, spacing |
| **Soft objectives** | HPWL (wire length) | Wire crossing proxy, wire length |
| **Rotation** | sin(2θ) continuous + discrete snap | Fixed by signal flow direction |
| **SA phase** | Yes (detailed placement) | Maybe (crossing minimization) |
| **Wire routing** | Not applicable (router is separate) | Integral part of schematic layout |
| **Density** | Pairwise clearance | Not needed (Sugiyama spreads) |


## Implementation Priority

### Milestone 1: Sugiyama Layout Engine
- Integrate `rust-sugiyama` or implement custom Sugiyama
- Build netlist → directed graph conversion (pin electrical types → edge direction)
- Layer assignment + crossing minimization
- Basic coordinate assignment (uniform grid)
- **Output**: Components placed in columns with minimal crossings

### Milestone 2: Solverang Position Refinement
- Grid snap constraints
- Vertical/horizontal spacing
- Group clustering
- Custom alignment from spec

### Milestone 3: Wire Routing
- A* orthogonal pathfinding on grid
- Net label insertion for long wires
- Power symbol placement
- Junction insertion at wire crossings

### Milestone 4: Spec Language + LLM Integration
- `.schdoc-spec` parser extension
- `schematic { ... }` block with groups, sequence, alignment
- LLM prompt template for schematic analysis
- Interactive iteration (solve → review → adjust)
