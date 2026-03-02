# altium-format-ir: Intermediate Representation for Downstream Consumers

Design notes for an `altium-format-ir` crate that provides a domain-semantic
intermediate representation of Altium files, purpose-built for downstream
consumers like the solverang autoplacer/autorouter, DRC engine, schematic
layout, and the spec language compiler.


## 1. Why an IR Crate?

### The Gap Today

`altium-format` exposes a high-level API (`api/` module) with types like
`Footprint`, `Pad`, `Component`, `SchDocSheet`, etc. These are clean but still
tightly coupled to the **file format's structure**:

- **Index-based cross-references**: Nets, components, polygons are linked by
  `u16` indices into flat arrays. Consumers must resolve these manually.
- **Coordinate soup**: Raw `Coord` values with no spatial indexing, no
  bounding boxes precomputed, no polygon operations.
- **No graph structure**: Netlists are implicit (scattered across pad headers
  via `net_index`). There's no `Net` object with a list of connected pins.
- **Format artifacts**: Sidecar streams, owner indices, record dispatch —
  meaningful for serialization but noise for placement algorithms.
- **Privacy boundary**: `altium-format` deliberately keeps implementation
  details private. The IR provides a stable, public, **semantic** interface
  that doesn't leak format internals.

### What Downstream Consumers Actually Need

| Consumer | Needs |
|----------|-------|
| **Solverang placement** | Component bounding boxes, pad positions in local frame, netlist as graph, board outline polygon, design rules as typed constraints |
| **DRC engine** | Copper geometry (tracks, pads, vias, fills, regions) with spatial index, design rules with scope expressions, layer stack |
| **Schematic layout** | Component symbols with pin positions, netlist as directed graph (signal flow), sheet dimensions, ownership hierarchy |
| **Spec compiler** | Designator→component lookup, net name→net lookup, footprint→pad mapping, rule name→rule lookup |
| **LLM agent** | Human-readable component list, net topology summary, board dimensions, current placement positions |

None of these need to know about CFB streams, block encoding, sidecar merging,
or parameter string parsing. They need **domain objects with relationships**.


## 2. Design Principles

### 2.1 Extraction, Not Wrapping

The IR is **extracted** from `altium-format` documents, not a thin wrapper.
Extraction resolves indices, precomputes geometry, and builds graph structures.
This is a one-way transformation:

```
PcbDoc (file format) ──extract──→ PcbIr (domain model) ──consume──→ solverang
                                                        ──consume──→ DRC
                                                        ──consume──→ spec compiler
```

The IR does NOT support writing back to Altium files. That path goes through
`altium-format` directly. The IR is read-only by design.

### 2.2 Owned, Self-Contained Data

IR types own all their data (no lifetimes, no references back to the source
document). This means:
- The source `PcbDoc`/`SchDoc` can be dropped after extraction
- IR can be serialized, cached, sent across threads
- No lifetime gymnastics for downstream consumers

### 2.3 Graph-First, Not Array-First

Where `altium-format` uses flat arrays with index cross-references (matching
the file format), the IR uses **typed handles and adjacency structures**:

```rust
// altium-format style (file-format-shaped):
let net_index = pad.common.net_index;  // u16, what net is this?
let net = &doc.nets6[net_index];       // manual lookup

// IR style (domain-shaped):
let net = &ir.net(pad.net);            // typed handle → direct access
for pin in net.pins() { ... }         // iterate connected pins
```

### 2.4 Coordinates in Millimeters (f64)

The IR converts from Altium internal units (`Coord`, 10,000 per mil) to
**millimeters as f64**. Rationale:

- Solverang operates in f64 (least-squares solver)
- Millimeters are the standard PCB unit internationally
- Eliminates integer overflow concerns for arithmetic
- Coordinate conversion is a source of bugs — do it once at extraction

Internal `Coord` values are preserved as metadata for roundtrip scenarios
where exact Altium coordinates matter.

### 2.5 Fail-Fast Extraction

Extraction inherits `altium-format`'s fail-fast philosophy. If the IR
extractor encounters data it can't represent (unknown layer, missing net,
corrupt geometry), it returns an error — never silently drops data.


## 3. Crate Position in the Dependency Graph

```
altium-format-types (enums, constants)
     ↓
altium-format-derive (proc macros)
     ↓
altium-format (parsing, serialization)
     ↓
altium-format-ir  ←── NEW ──── depends on altium-format + types
     ↓
altium-format-spec (spec compiler uses IR for lookups)
     ↓
altium-cli (CLI uses IR for inspect/placement/DRC commands)

External:
solverang ← depends on altium-format-ir (for PcbIr → ConstraintSystem input)
  Note: solverang's geometry feature is removed; PCB geometry lives in
  constraint residual functions in solverang-pcb, not in solverang core.
```

The IR crate depends on `altium-format` (for extraction) and
`altium-format-types` (for shared enums like `V6Layer`, `PadShape`, etc.).
It does NOT depend on `solverang` — the bridge from IR to solver types
lives in `solverang-pcb` or `altium-format-spec`.

### Why Not Just Extend the `api/` Module?

The `api/` module in `altium-format` is a presentation layer — it exposes
individual records in a clean way but doesn't build cross-cutting structures
(netlist graphs, spatial indices, resolved relationships). The IR is a
**materialized view** that precomputes everything consumers need. It also
lives in a separate crate so that:

1. `altium-format` stays focused on parsing/serialization (single responsibility)
2. The IR's dependencies (e.g., `petgraph` for graphs, spatial index crates)
   don't bloat the core parsing library
3. Downstream crates can depend on the IR without pulling in the full parser


## 4. PCB Intermediate Representation

### 4.1 Top-Level Structure

```rust
/// Complete PCB board representation extracted from a PcbDoc.
pub struct PcbIr {
    /// Source file metadata.
    pub metadata: PcbMetadata,

    /// Board outline and keepout geometry.
    pub board: BoardGeometry,

    /// Layer stack configuration.
    pub layer_stack: LayerStack,

    /// All placed components, indexed by ComponentId.
    pub components: IdMap<ComponentId, IrComponent>,

    /// All nets (electrical connectivity), indexed by NetId.
    pub nets: IdMap<NetId, IrNet>,

    /// All design rules, indexed by RuleId.
    pub rules: IdMap<RuleId, IrDesignRule>,

    /// Copper geometry not owned by components (free tracks, vias, fills).
    pub free_copper: FreeCopperGeometry,

    /// Polygon pours.
    pub polygons: IdMap<PolygonId, IrPolygon>,
}
```

### 4.2 Typed Handles

All cross-references use typed newtype handles instead of raw indices:

```rust
/// Typed handle for cross-referencing within the IR.
/// Cheap to copy, impossible to mix up ComponentId with NetId.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ComponentId(u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NetId(u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PadId(u32);   // global pad ID (unique across all components)

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RuleId(u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PolygonId(u32);

/// Efficient indexed storage with typed handles.
pub struct IdMap<K, V> { /* Vec<V> internally, K indexes into it */ }
```

### 4.3 Board Geometry

```rust
pub struct BoardGeometry {
    /// Board outline as a closed polygon (mm, CCW winding).
    pub outline: Polygon,

    /// Board cutouts (holes in the board), each a closed polygon.
    pub cutouts: Vec<Polygon>,

    /// Axis-aligned bounding box of the outline (mm).
    pub bounds: BoundingBoxMm,

    /// Keepout zones (component placement forbidden).
    pub keepouts: Vec<KeepoutZone>,

    /// Mounting holes and other fixed obstacles.
    pub fixed_obstacles: Vec<FixedObstacle>,
}

pub struct Polygon {
    /// Vertices in mm. Closed (first == last) or implicitly closed.
    pub vertices: Vec<PointMm>,
    /// Arc segments (if any vertex-to-vertex edge is an arc).
    pub arcs: Vec<ArcSegment>,
}

pub struct BoundingBoxMm {
    pub min: PointMm,
    pub max: PointMm,
}

pub struct PointMm {
    pub x: f64,
    pub y: f64,
}

pub struct KeepoutZone {
    pub outline: Polygon,
    pub layers: LayerSet,
    pub restrictions: KeepoutRestrictions,
}

pub struct FixedObstacle {
    pub location: PointMm,
    pub kind: ObstacleKind,  // MountingHole, Fiducial, etc.
    pub exclusion_radius: f64,  // mm
}
```

### 4.4 Components

```rust
pub struct IrComponent {
    pub id: ComponentId,

    /// Designator ("U1", "R3", "J2").
    pub designator: String,

    /// Footprint pattern name ("QFP-100", "0402", "USB-C").
    pub pattern: String,

    /// Component value/comment ("100nF", "STM32F407VGT6").
    pub value: String,

    /// Description text.
    pub description: String,

    /// Current placement position (component origin, mm).
    pub position: PointMm,

    /// Current rotation in degrees (0, 90, 180, 270 typically).
    pub rotation: f64,

    /// Board side (top or bottom).
    pub side: BoardSide,

    /// Whether the component is locked (position fixed by user).
    pub locked: bool,

    /// Bounding box in LOCAL coordinates (before rotation/placement).
    /// Computed from the union of all child primitive bounding boxes.
    pub local_bounds: BoundingBoxMm,

    /// Bounding box in WORLD coordinates (after rotation + translation).
    pub world_bounds: BoundingBoxMm,

    /// Component height (3D, for height-limit DRC).
    pub height: f64,  // mm

    /// Pads belonging to this component, in local coordinates.
    pub pads: Vec<IrComponentPad>,

    /// Non-pad primitives (silkscreen, courtyard, etc.) — for DRC.
    pub graphics: Vec<IrComponentGraphic>,

    /// Component kind (standard, mechanical, graphical, etc.).
    pub kind: ComponentKind,
}

pub enum BoardSide { Top, Bottom }
```

### 4.5 Pads (Component-Local)

```rust
pub struct IrComponentPad {
    /// Global pad ID (unique across the entire board).
    pub id: PadId,

    /// Pad name/designator ("1", "2", "A1", "GND").
    pub name: String,

    /// Position in component-local coordinates (mm, before rotation).
    pub local_position: PointMm,

    /// Position in world coordinates (mm, after component placement).
    /// Recomputed when component moves — provided as convenience.
    pub world_position: PointMm,

    /// Net this pad is connected to (None = unconnected).
    pub net: Option<NetId>,

    /// Pad shape and size (top layer — simplified for placement).
    pub shape: PadShapeInfo,

    /// Whether this is a through-hole pad.
    pub is_through_hole: bool,

    /// Hole size (mm, 0.0 for SMD pads).
    pub hole_size: f64,

    /// Layer(s) this pad exists on.
    pub layers: LayerSet,
}

pub struct PadShapeInfo {
    pub kind: PadShapeKind,
    pub size_x: f64,  // mm
    pub size_y: f64,  // mm
    pub rotation: f64, // degrees, pad-local rotation
}

pub enum PadShapeKind {
    Round,
    Rectangular,
    RoundRect { corner_radius_ratio: f64 },
    Octagonal,
    Custom,  // complex shape, use bounding box approximation
}
```

### 4.6 Nets (Connectivity Graph)

```rust
pub struct IrNet {
    pub id: NetId,

    /// Net name ("GND", "VCC3P3", "SPI_CLK", "Net_C3_1").
    pub name: String,

    /// All pads (pins) connected to this net.
    /// Each entry identifies both the pad and its parent component.
    pub pins: Vec<NetPin>,

    /// Net class memberships (e.g., "Power", "Signal", "HighSpeed").
    pub classes: Vec<String>,

    /// Number of connected components (useful for topology analysis).
    pub component_count: usize,
}

pub struct NetPin {
    pub pad: PadId,
    pub component: ComponentId,
    /// Pad world position (denormalized for fast access).
    pub position: PointMm,
}
```

### 4.7 Design Rules

```rust
pub struct IrDesignRule {
    pub id: RuleId,

    /// Rule name ("Clearance_Default", "Width_Signal").
    pub name: String,

    /// Rule kind (maps to Altium TRuleKind).
    pub kind: RuleKind,

    /// Rule priority (lower = higher priority).
    pub priority: i32,

    /// Whether the rule is enabled.
    pub enabled: bool,

    /// Rule-specific parameters.
    pub params: RuleParams,

    /// Scope filters (which objects this rule applies to).
    pub scope1: ScopeExpr,
    pub scope2: ScopeExpr,
    pub net_scope: NetScope,
    pub layer_scope: LayerScope,
}

/// Rule parameters by category.
pub enum RuleParams {
    Clearance { gap: f64 },
    ComponentClearance { gap: f64 },
    BoardOutlineClearance { gap: f64 },
    Width { min: f64, max: f64, preferred: f64 },
    HoleSize { min: f64, max: f64 },
    HoleToHoleClearance { gap: f64 },
    AnnularRing { min: f64 },
    SolderMaskExpansion { expansion: f64 },
    PasteMaskExpansion { expansion: f64 },
    Height { min: f64, max: f64 },
    DiffPairs { gap: f64, max_gap: f64, max_skew: f64 },
    MatchedLengths { tolerance: f64 },
    // ... other geometric rule types
    Confinement { region: ConfinementRegion },
    Rotations { allowed: Vec<i32> },
    // Logical rules (not geometric — evaluated as predicates)
    Logical(LogicalRuleData),
}

pub enum NetScope { Any, SameNet, DifferentNets }
pub enum LayerScope { Any, SameLayer, AdjacentLayers }
```

### 4.8 Free Copper (Not Component-Owned)

```rust
pub struct FreeCopperGeometry {
    /// Standalone tracks (routing between components).
    pub tracks: Vec<IrTrack>,

    /// Standalone vias.
    pub vias: Vec<IrVia>,

    /// Standalone fills.
    pub fills: Vec<IrFill>,

    /// Copper regions (not polygon pours).
    pub regions: Vec<IrRegion>,
}

pub struct IrTrack {
    pub start: PointMm,
    pub end: PointMm,
    pub width: f64,  // mm
    pub layer: V6Layer,
    pub net: Option<NetId>,
}

pub struct IrVia {
    pub position: PointMm,
    pub diameter: f64,  // mm
    pub hole_size: f64,  // mm
    pub from_layer: V6Layer,
    pub to_layer: V6Layer,
    pub net: Option<NetId>,
}
```

### 4.9 Layer Stack

```rust
pub struct LayerStack {
    /// Ordered list of copper layers (top to bottom).
    pub copper_layers: Vec<CopperLayerInfo>,

    /// Total board thickness (mm).
    pub total_thickness: f64,

    /// Layer pair definitions (for via drilling).
    pub drill_pairs: Vec<DrillPair>,
}

pub struct CopperLayerInfo {
    pub layer: V6Layer,
    pub name: String,
    pub copper_thickness: f64,  // mm
    pub dielectric_thickness: f64,  // mm (to next layer)
}
```


## 5. Schematic Intermediate Representation

### 5.1 Top-Level Structure

```rust
/// Complete schematic sheet representation extracted from a SchDoc.
pub struct SchIr {
    pub metadata: SchMetadata,

    /// Sheet dimensions and display settings.
    pub sheet: SheetInfo,

    /// Font table (referenced by font_id in text objects).
    pub fonts: Vec<IrFont>,

    /// Placed component instances.
    pub components: IdMap<SchComponentId, IrSchComponent>,

    /// Nets (derived from wire connectivity + net labels).
    pub nets: IdMap<SchNetId, IrSchNet>,

    /// Wires (electrical connections).
    pub wires: Vec<IrSchWire>,

    /// Buses.
    pub buses: Vec<IrSchBus>,

    /// Power symbols.
    pub power_objects: Vec<IrSchPowerObject>,

    /// Net labels.
    pub net_labels: Vec<IrSchNetLabel>,

    /// Ports (cross-sheet connections).
    pub ports: Vec<IrSchPort>,

    /// Sheet symbols (hierarchical references).
    pub sheet_symbols: Vec<IrSchSheetSymbol>,

    /// Junctions.
    pub junctions: Vec<IrSchJunction>,
}
```

### 5.2 Schematic Components

```rust
pub struct IrSchComponent {
    pub id: SchComponentId,
    pub designator: String,
    pub lib_reference: String,
    pub value: String,
    pub description: String,

    /// Placement position (mm).
    pub position: PointMm,

    /// Orientation (0, 90, 180, 270).
    pub orientation: RotationBy90,

    /// Mirror state.
    pub is_mirrored: bool,

    /// Bounding box in world coordinates (mm).
    pub world_bounds: BoundingBoxMm,

    /// Pins with electrical types and positions.
    pub pins: Vec<IrSchPin>,

    /// Component parameters (Comment, Value, custom).
    pub parameters: Vec<IrSchParameter>,
}

pub struct IrSchPin {
    pub name: String,
    pub designator: String,
    pub electrical_type: PinElectricalType,

    /// Pin tip position in world coordinates (mm).
    /// This is where wires connect to.
    pub world_position: PointMm,

    /// Pin length (mm).
    pub length: f64,

    /// Net this pin is connected to (resolved from wire tracing).
    pub net: Option<SchNetId>,

    /// Orientation of the pin symbol.
    pub orientation: RotationBy90,
}
```

### 5.3 Schematic Nets (Resolved Connectivity)

This is a key value-add of the IR: `altium-format` stores wires and net labels
as independent records; the IR resolves connectivity by tracing wire paths and
matching net label names to build an explicit net graph.

```rust
pub struct IrSchNet {
    pub id: SchNetId,
    pub name: String,

    /// All pins connected to this net.
    pub pins: Vec<SchNetPin>,

    /// Is this a power net (connected via power objects)?
    pub is_power: bool,
}

pub struct SchNetPin {
    pub component: SchComponentId,
    pub pin_index: usize,  // index into component's pins Vec
    pub position: PointMm,
}
```

### 5.4 Signal Flow Graph (for Sugiyama)

The IR can optionally build a directed signal flow graph for schematic
auto-layout. Edge direction is determined by pin electrical types:

```rust
pub struct SignalFlowGraph {
    /// Nodes = components.
    pub nodes: Vec<SchComponentId>,

    /// Directed edges = nets, from output pins to input pins.
    /// Each edge carries the net ID and connected pin indices.
    pub edges: Vec<SignalFlowEdge>,
}

pub struct SignalFlowEdge {
    pub from_component: SchComponentId,
    pub to_component: SchComponentId,
    pub net: SchNetId,
    pub from_pin: usize,  // output/bidirectional pin index
    pub to_pin: usize,    // input/passive pin index
}
```


## 6. Extraction API

### 6.1 PCB Extraction

```rust
impl PcbIr {
    /// Extract IR from a parsed PcbDoc.
    /// Resolves all indices, computes bounding boxes, builds netlist graph.
    pub fn extract(doc: &PcbDoc) -> Result<Self, IrExtractionError>;

    /// Extract IR from a PcbLib footprint (single component, no netlist).
    pub fn extract_footprint(fp: &Footprint) -> Result<IrFootprint, IrExtractionError>;
}
```

### 6.2 Schematic Extraction

```rust
impl SchIr {
    /// Extract IR from a parsed SchDoc.
    /// Resolves ownership, traces wire connectivity, builds net graph.
    pub fn extract(doc: &SchDoc) -> Result<Self, IrExtractionError>;

    /// Build signal flow graph from the extracted IR.
    pub fn signal_flow_graph(&self) -> Result<SignalFlowGraph, IrExtractionError>;
}
```

### 6.3 Error Types

```rust
pub enum IrExtractionError {
    /// A net index in a primitive references a non-existent net.
    InvalidNetReference { primitive: String, net_index: u16 },
    /// A component index references a non-existent component.
    InvalidComponentReference { primitive: String, component_index: u16 },
    /// Board outline could not be determined.
    NoBoardOutline,
    /// Geometry error during bounding box computation.
    GeometryError(String),
    /// Underlying format error.
    FormatError(AltiumFormatError),
}
```


## 7. Consumer Bridges

The IR is designed to be consumed by multiple downstream systems. Each consumer
has a thin bridge layer that maps IR types to its own domain.

### 7.1 Solverang Bridge (in `solverang-pcb` or `altium-format-spec`)

```rust
/// Convert PcbIr into solverang ConstraintSystem with entities + constraints.
pub fn ir_to_solver_input(
    ir: &PcbIr,
    spec: &PlacementSpec,
) -> Result<ConstraintSystem, SolverBuildError> {
    let mut system = ConstraintSystem::new();

    // 1. Create a PcbComponent entity per IR component
    // Each entity's params are allocated via system.alloc_param(value, entity_id)
    // using generational IDs for safety
    for comp in ir.components.values() {
        let entity_id = system.alloc_entity_id();
        let x = system.alloc_param(comp.position.x, entity_id);
        let y = system.alloc_param(comp.position.y, entity_id);
        let entity = PcbComponent::new(
            entity_id, x, y,
            comp.local_bounds.half_width(),
            comp.local_bounds.half_height(),
            comp.rotation,
            comp.designator.clone(),
        );
        system.add_entity(Box::new(entity));
    }

    // 2. Board containment from ir.board.bounds
    // 3. Pairwise clearance from ir.rules (ComponentClearance)
    // 4. HPWL objectives from ir.nets (pin world positions) — is_soft() = true
    // 5. User constraints from spec
    // 6. system.solve() returns SystemResult with per-cluster results

    Ok(system)
}
```

Key mapping:
| IR Type | Solverang Type |
|---------|---------------|
| `IrComponent.local_bounds` | `PcbComponent.half_width/half_height` |
| `IrComponentPad.local_position` | `PinInfo.offset_x/offset_y` |
| `IrNet.pins` | `SmoothHPWL.pin_xs/pin_ys` |
| `BoardGeometry.bounds` | `BoardContainment` constraint params |
| `IrDesignRule::ComponentClearance` | `ComponentClearance` constraint |

### 7.2 DRC Bridge

```rust
/// Evaluate all design rules against the current board state.
pub fn ir_to_drc_report(ir: &PcbIr) -> DrcReport {
    let mut violations = Vec::new();

    for rule in ir.rules.values() {
        if !rule.enabled { continue; }
        match &rule.params {
            RuleParams::Clearance { gap } => {
                // Check all copper object pairs matching scope
                check_copper_clearance(ir, rule, *gap, &mut violations);
            }
            RuleParams::ComponentClearance { gap } => {
                // Check component bounding box distances
                check_component_clearance(ir, *gap, &mut violations);
            }
            // ...
        }
    }

    DrcReport { violations, rules_checked: ir.rules.len() }
}
```

### 7.3 LLM Agent Bridge

```rust
/// Generate a human-readable board summary for LLM consumption.
pub fn ir_to_board_summary(ir: &PcbIr) -> String {
    // Component list with designators, footprints, values
    // Net topology summary
    // Board dimensions
    // Current placement positions
    // Design rule summary
}
```

### 7.4 Schematic Layout Bridge (Sugiyama)

```rust
/// Convert SchIr signal flow graph to petgraph for Sugiyama layout.
pub fn ir_to_signal_flow_graph(ir: &SchIr) -> petgraph::DiGraph<SchComponentId, SchNetId> {
    let flow = ir.signal_flow_graph().unwrap();
    let mut graph = DiGraph::new();
    // ... map nodes and edges
    graph
}
```


## 8. What the IR Intentionally Omits

The IR is a **lossy extraction** — it discards information that downstream
consumers don't need:

| Omitted | Why |
|---------|-----|
| CFB container details | Irrelevant to domain logic |
| Block encoding / stream layout | Serialization detail |
| Sidecar stream contents | Merged at parse time by altium-format |
| Parameter string ordering | Invariant of file format, not domain |
| Raw `Coord` values | Converted to mm at extraction |
| Record indices / dispatch codes | Replaced by typed handles |
| Unique IDs / GUIDs | Preserved only where needed (e.g., model refs) |
| Display settings (colors, fonts) | Relevant for rendering, not solving |
| Embedded 3D models | Not needed for 2D placement/routing |
| Simulation records (probes, stimuli) | Out of scope |

For PCB specifically, the IR omits fine-grained pad stack data (per-layer
shapes, thermal relief details, mask expansions) in the base representation.
These are available via an extended `IrPadDetail` if DRC needs them:

```rust
impl PcbIr {
    /// Get detailed pad information (full pad stack, thermal relief, etc.)
    /// Only needed by DRC, not by placement.
    pub fn pad_detail(&self, pad: PadId) -> Option<&IrPadDetail>;
}
```


## 9. Performance Considerations

### 9.1 Extraction Cost

Extraction traverses all records once and builds the IR in O(N) where N is
the total record count. For a typical 200-component PcbDoc:

| Step | Records | Time Est. |
|------|---------|-----------|
| Resolve net indices | ~2000 primitives | <1ms |
| Compute bounding boxes | ~200 components | <1ms |
| Build netlist graph | ~300 nets | <1ms |
| Parse design rules | ~20 rules | <1ms |
| **Total** | | **<5ms** |

The IR should be extracted once and reused across multiple operations
(placement, DRC, inspection) within a session.

### 9.2 Memory Layout

IR types use `Vec` and owned `String`s — no arena allocation needed at
PCB scale. A 500-component board with 1000 nets and 5000 primitives
should fit in ~1–2 MB.

### 9.3 Optional Spatial Index

For DRC (which needs pairwise distance checks), the IR can lazily build
a spatial index:

```rust
impl PcbIr {
    /// Build an R-tree spatial index over all copper geometry.
    /// Lazy — only built on first call, then cached.
    pub fn spatial_index(&self) -> &SpatialIndex;
}
```

This is NOT needed for placement (which uses pairwise component clearance
at O(N^2) scale, manageable for N<500).


## 10. Versioning Strategy

The IR types are versioned independently of `altium-format`. When
`altium-format` adds support for new record types or fields:

1. New fields are added to IR types (non-breaking if `Option<T>`)
2. Extraction logic is updated to populate new fields
3. Consumers that don't use the new fields are unaffected

The IR version tracks the minimum `altium-format` version it requires:

```toml
[dependencies]
altium-format = { path = "../altium-format", version = "0.1" }
altium-format-types = { path = "../altium-format-types", version = "0.1" }
```


## 11. Relationship to Existing Architecture Docs

This IR formalizes the data structures that appear informally across the
solverang integration docs:

| Existing Doc | Informal Type | IR Equivalent |
|-------------|--------------|---------------|
| architecture.md | `PlacementData` | `PcbIr` (full extraction) |
| architecture.md | `ComponentData` | `IrComponent` |
| architecture.md | `extract_netlist()` | `PcbIr::extract()` → `ir.nets` |
| constraint-types.md | `PcbComponent` entity | Built from `IrComponent` |
| constraint-types.md | `PcbNet` | Built from `IrNet` |
| constraint-types.md | `PcbBoardOutline` | Built from `BoardGeometry` |
| spec-grammar.md | `PlaceSpec.designators` | Resolved via `ir.components` |
| schdoc-placement.md | `SchComponent` entity | Built from `IrSchComponent` |
| llm-constraint-generation.md | `altium inspect` output | `ir_to_board_summary()` |


## 12. Implementation Priority

### Phase 1: PCB IR Core (MVP for Placement)

- `PcbIr` with components, nets, board outline, basic rules
- `IrComponent` with bounding boxes and pad positions
- `IrNet` with pin connectivity
- `BoardGeometry` with outline polygon and bounds
- `extract()` from `PcbDoc`
- Unit tests with fixture PcbDoc files

### Phase 2: Schematic IR Core (MVP for Layout)

- `SchIr` with components, nets, wires
- `IrSchComponent` with pin positions and electrical types
- `IrSchNet` with resolved connectivity
- `SignalFlowGraph` for Sugiyama input
- `extract()` from `SchDoc`

### Phase 3: DRC Extensions

- `FreeCopperGeometry` (tracks, vias, fills, regions)
- `IrPadDetail` (full pad stack data)
- `SpatialIndex` for efficient pairwise checks
- `IrDesignRule` with scope expression evaluation
- `LayerStack` with copper/dielectric thicknesses

### Phase 4: PcbLib / SchLib IR

- `IrFootprint` extracted from PcbLib (single-component IR)
- `IrSchSymbol` extracted from SchLib
- Used by the spec compiler for footprint-level operations


## 13. Open Questions

### Q1: Should the IR support incremental updates?

When solverang produces new component positions, should the IR be updated
in-place, or should we extract a fresh IR after applying changes to the
underlying `PcbDoc`?

**Recommendation**: Fresh extraction. The IR is cheap to build (<5ms) and
incremental updates add complexity. The flow is:
```
PcbDoc → extract → PcbIr → solve → PlacementSolution → apply to PcbDoc → re-extract
```

### Q2: Should the IR include rendering data?

Colors, line styles, font choices — needed for visualization but not for
solving. Two options:
- (a) Omit entirely (consumers that render use `altium-format` directly)
- (b) Include as `Option<RenderingHints>` on IR types

**Recommendation**: (a) for now. Rendering is a separate concern. If CLI
visualization needs IR data, add an `IrRenderExt` trait in the CLI crate.

### Q3: Polygon representation

Board outlines and regions can have arc segments. Should the IR:
- (a) Tessellate arcs into line segments (simpler, slight loss of precision)
- (b) Preserve arc segments as first-class geometry

**Recommendation**: (b) with a `tessellate(resolution)` method for consumers
that need polyline-only input (e.g., solverang's AABB containment checks).

### Q4: Should extraction be configurable?

Some consumers need minimal data (placement: just components + nets + outline)
while others need everything (DRC: full copper geometry + rules). Options:
- (a) Single `extract()` that builds everything
- (b) Builder pattern: `PcbIr::builder().components().nets().rules().build(doc)`

**Recommendation**: (a) for simplicity. At PCB scale the cost of extracting
everything is negligible. Profiling can justify lazy/optional extraction later.
