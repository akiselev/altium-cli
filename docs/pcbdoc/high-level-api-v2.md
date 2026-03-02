# PcbDoc High-Level API v2: Extended Types

Extends the existing `PcbDocBoard` API with five new capabilities needed by
downstream consumers (Gerber export, DRC, placement solvers, spec language).

These additions are **non-breaking**: existing fields and methods remain unchanged.
New fields are added to existing types, and new types are added alongside them.

## Overview of Changes

| Area | What Changes | Why |
|------|-------------|-----|
| **LayerStack** | New `BoardSettings.layer_stack` field | Gerber X2 attributes, layer ordering, copper/dielectric data |
| **BoardGeometry** | New `BoardSettings.geometry` field | Board outline with arcs, cutouts, keepouts, bounds |
| **Pad Stack** | New `Pad.stack` field | Per-layer pad shapes for multi-layer Gerber apertures |
| **Design Rule Params** | New `DesignRule.params` field | Typed rule values (clearance, mask expansion, width) |
| **Net Connectivity** | New `PcbDocBoard.connectivity()` method | Pin-level connectivity graph for X2 attributes and DRC |

---

## 1. LayerStack

### Problem

The current `BoardSettings` has only `signal_layer_count: i32`. Consumers need:
- Physical layer ordering (Gerber X2: `TF.FileFunction,Copper,L{n},Top/Bot/Inr`)
- Copper thickness (impedance calculations, DRC)
- Dielectric properties (stackup visualization)
- Layer type classification (signal vs plane)
- Drill pair definitions (Gerber drill file separation)

### Internal Data Available

`PcbBoardConfig` already parses the full V7/V8/V9 layer stacks from Board6:
- `v9_stack_layers: Vec<PcbStackLayerEntry>` with `cop_thick`, `diel_type`, `diel_height`,
  `diel_material`, `diel_const`, `component_placement`, `layer_id`
- `v9_master_stack` / `v8_master_stack` with `style`, `is_flex`
- Legacy `LAYER{n}` entries with same fields

### Public API Design

```rust
/// Layer stack configuration extracted from Board6.
///
/// Represents the physical stackup of the PCB from top to bottom.
/// All thicknesses are in the original Altium string format (e.g. "1.350000mil")
/// to avoid lossy conversion — downstream consumers parse as needed.
#[derive(Debug, Clone)]
pub struct LayerStack {
    /// Stack style (layer pairs, internal planes, etc.).
    pub style: LayerStackStyle,

    /// Whether this is a flex PCB stackup.
    pub is_flex: bool,

    /// Ordered copper layers from top to bottom.
    /// The first entry is always the top copper layer, the last is bottom.
    pub layers: Vec<StackLayer>,

    /// Total number of copper layers (convenience, == layers.len()).
    pub copper_layer_count: usize,
}

/// A single layer in the physical stackup.
#[derive(Debug, Clone)]
pub struct StackLayer {
    /// Layer reference (for matching to primitives).
    pub layer: LayerRef,

    /// Human-readable layer name ("Top Layer", "GND", "Signal 3").
    pub name: String,

    /// Physical position in stack (1-based, top = 1).
    pub physical_order: usize,

    /// Whether this is an internal plane layer.
    pub is_plane: bool,

    /// Copper thickness (e.g. "1.350000mil").
    pub copper_thickness: String,

    /// Dielectric type to the NEXT layer below.
    pub dielectric_type: DielectricType,

    /// Dielectric constant to the next layer below.
    pub dielectric_constant: String,

    /// Dielectric thickness to the next layer below (e.g. "11.800000mil").
    pub dielectric_height: String,

    /// Dielectric material name (e.g. "FR-4").
    pub dielectric_material: String,

    /// Component placement side (for top/bottom identification).
    pub component_placement: Option<ComponentPlacementType>,
}

impl LayerStack {
    /// Get the top copper layer.
    pub fn top(&self) -> Option<&StackLayer> { self.layers.first() }

    /// Get the bottom copper layer.
    pub fn bottom(&self) -> Option<&StackLayer> { self.layers.last() }

    /// Get a layer by its LayerRef.
    pub fn layer(&self, layer: &LayerRef) -> Option<&StackLayer> {
        self.layers.iter().find(|l| l.layer == *layer)
    }

    /// Physical order number (1-based) for a given layer.
    /// Used for Gerber X2: TF.FileFunction,Copper,L{n},Top/Bot/Inr
    pub fn physical_order(&self, layer: &LayerRef) -> Option<usize> {
        self.layer(layer).map(|l| l.physical_order)
    }

    /// Inner layers only (excluding top and bottom).
    pub fn inner_layers(&self) -> &[StackLayer] {
        if self.layers.len() <= 2 { &[] }
        else { &self.layers[1..self.layers.len() - 1] }
    }
}
```

### Changes to BoardSettings

```rust
pub struct BoardSettings {
    // Existing fields unchanged
    pub document_name: String,
    pub signal_layer_count: i32,
    pub board_outline: Option<Vec<CoordPoint>>,  // kept for backward compat
    pub snap_grid_size: Coord,
    pub visible_grid_size: Coord,
    pub display_unit: Unit,

    // NEW
    /// Full layer stack configuration.
    pub layer_stack: LayerStack,

    /// Board geometry with arc-preserving outlines, cutouts, and bounds.
    pub geometry: BoardGeometry,
}
```

### Spec Language

```
board "MyPCB" {
    signal_layer_count: 4
    display_unit: "metric"

    layer_stack {
        style: "layer_pairs"
        is_flex: false

        layer "Top Layer" {
            copper_thickness: 1.35mil
            dielectric_type: "core"
            dielectric_height: 11.8mil
            dielectric_material: "FR-4"
            dielectric_constant: 4.5
        }
        layer "GND" {
            is_plane: true
            copper_thickness: 1.35mil
            dielectric_type: "prepreg"
            dielectric_height: 7.5mil
            dielectric_material: "FR-4"
        }
        layer "Power" {
            is_plane: true
            copper_thickness: 1.35mil
            dielectric_type: "core"
            dielectric_height: 11.8mil
        }
        layer "Bottom Layer" {
            copper_thickness: 1.35mil
        }
    }
}
```

### Dump Output

```
board "MyPCB" {
    signal_layer_count: 4
    display_unit: "metric"

    layer_stack {
        style: "layer_pairs"

        layer "Top Layer"    { copper_thickness: 1.35mil, dielectric: "core" 11.8mil "FR-4" }
        layer "GND"          { is_plane: true, copper_thickness: 1.35mil, dielectric: "prepreg" 7.5mil "FR-4" }
        layer "Power"        { is_plane: true, copper_thickness: 1.35mil, dielectric: "core" 11.8mil }
        layer "Bottom Layer" { copper_thickness: 1.35mil }
    }
}
```

---

## 2. BoardGeometry

### Problem

The current `BoardSettings.board_outline: Option<Vec<CoordPoint>>` has three issues:
1. **Loses arc segments** — `contour_to_coord_points()` flattens `PolySegment` to vertices
2. **No cutouts** — only returns the first board outline region
3. **No keepouts** — keepout regions exist in the flat `regions` vec but aren't grouped
4. **No bounding box** — consumers must compute it themselves

### Internal Data Available

- `PcbRegion.outline: Contour` with `Contour::ShapeBased(Vec<PolySegment>)` preserving
  arc center/radius/angles
- `PcbRegion.is_board_cutout: bool` and `PcbRegion.keepout: bool` flags
- `PcbRegion.holes: Vec<Contour>` for cutout holes

### Public API Design

```rust
/// Board-level geometry: outline, cutouts, keepouts, bounding box.
#[derive(Debug, Clone)]
pub struct BoardGeometry {
    /// Primary board outline (closed contour, may contain arc segments).
    pub outline: BoardContour,

    /// Internal cutouts (holes in the board, e.g., mounting slots).
    /// Each cutout is from a region with is_board_cutout == true that isn't
    /// the primary outline.
    pub cutouts: Vec<BoardContour>,

    /// Board keepout zones (areas where placement/routing is restricted).
    pub keepouts: Vec<KeepoutZone>,

    /// Axis-aligned bounding box of the primary outline.
    pub bounds: BoundingBox,
}

/// A closed contour that may contain both line and arc segments.
///
/// Preserves the exact geometry from the Altium file, including arc segments
/// with center/radius/angles. This is critical for Gerber profile output
/// and accurate board outline representation.
#[derive(Debug, Clone)]
pub struct BoardContour {
    /// Ordered segments forming a closed contour.
    /// The contour is implicitly closed (last vertex connects to first).
    pub segments: Vec<ContourSegment>,
}

/// A segment in a board contour — either a line or an arc.
#[derive(Debug, Clone)]
pub enum ContourSegment {
    /// Straight line to the endpoint.
    Line {
        endpoint: CoordPoint,
    },
    /// Circular arc to the endpoint.
    Arc {
        endpoint: CoordPoint,
        center: CoordPoint,
        radius: Coord,
        start_angle: f64,
        end_angle: f64,
    },
}

/// A keepout zone restricting placement/routing.
#[derive(Debug, Clone)]
pub struct KeepoutZone {
    /// Zone outline.
    pub outline: BoardContour,

    /// Which layer(s) the keepout applies to.
    pub layer: LayerRef,

    /// Keepout restriction flags (from KEEPOUTRESTRICTIONS parameter).
    /// Bit field: tracks, vias, pads, copper, etc.
    pub restrictions: u32,
}

/// Axis-aligned bounding box in Altium coordinates.
#[derive(Debug, Clone)]
pub struct BoundingBox {
    pub min: CoordPoint,
    pub max: CoordPoint,
}

impl BoundingBox {
    pub fn width(&self) -> Coord { self.max.x - self.min.x }
    pub fn height(&self) -> Coord { self.max.y - self.min.y }
    pub fn center(&self) -> CoordPoint {
        CoordPoint::new(
            Coord::from_internal((self.min.x.to_internal() + self.max.x.to_internal()) / 2),
            Coord::from_internal((self.min.y.to_internal() + self.max.y.to_internal()) / 2),
        )
    }
}

impl BoardContour {
    /// Flatten to simple vertices (losing arc info).
    /// Useful for quick bounding box computation or simple consumers.
    pub fn to_points(&self) -> Vec<CoordPoint> {
        self.segments.iter().map(|s| match s {
            ContourSegment::Line { endpoint } => *endpoint,
            ContourSegment::Arc { endpoint, .. } => *endpoint,
        }).collect()
    }

    /// Tessellate arc segments into line segments with the given chord tolerance.
    /// Returns a pure-line contour suitable for polygon operations.
    pub fn tessellate(&self, chord_tolerance: Coord) -> BoardContour {
        // ... arc → polyline conversion
        todo!()
    }
}
```

### Migration

`BoardSettings.board_outline` is kept as-is for backward compatibility. New code
should use `BoardSettings.geometry.outline` which preserves arc segments.

### Spec Language

Board geometry is **read-only in the spec language** — you don't define the board
outline in a `.pcbdoc-spec` (it's a physical property set in the PCB editor). The
dump shows it for information:

```
board "MyPCB" {
    // ...

    geometry {
        outline: [
            line (0, 0),
            line (100mm, 0),
            arc (100mm, 10mm) { center: (100mm, 5mm), radius: 5mm },
            line (100mm, 50mm),
            line (0, 50mm),
        ]
        bounds: (0, 0) to (100mm, 50mm)

        cutout [
            line (40mm, 20mm),
            line (60mm, 20mm),
            line (60mm, 30mm),
            line (40mm, 30mm),
        ]

        keepout { layer: "TopLayer", restrictions: 0x3F, outline: [...] }
    }
}
```

---

## 3. Pad Stack (Per-Layer Shapes)

### Problem

The current `Pad` type exposes only top-layer shape (`shape`, `x_size`, `y_size`).
Gerber export needs different apertures per layer — a pad can have:
- Round on top copper, rectangular on inner layers, octagonal on bottom
- Different sizes on solder mask (expanded) and paste mask (contracted)
- Different hole offsets per layer

### Internal Data Available

`PcbPad` stores:
- `shape_top`, `shape_mid`, `shape_bot` + `size_top`, `size_mid`, `size_bot`
- `PcbPadStackData` with `inner_shape[29]`, `inner_size_x[29]`, `inner_size_y[29]`
- `alt_shape[32]`, `corner_radius_pct[32]`, `hole_offset_x[32]`, `hole_offset_y[32]`

### Public API Design

```rust
/// Per-layer pad shape/size for pad stack configurations.
///
/// When `pad_mode` is `Simple`, only `top` is meaningful (mid/bot inherit from top).
/// When `LocalStack` or `ExternalStack`, each layer can have independent shapes.
#[derive(Debug, Clone)]
pub struct PadStack {
    /// Top copper layer shape and size.
    pub top: PadLayerShape,

    /// Mid (inner) layers default shape and size.
    /// Used for all inner layers unless overridden in `inner_layers`.
    pub mid: PadLayerShape,

    /// Bottom copper layer shape and size.
    pub bot: PadLayerShape,

    /// Per-inner-layer overrides (only populated for LocalStack/ExternalStack mode).
    /// Key is the inner layer index (0-28, where 0 = InternalPlane1).
    /// Layers not present in this map use `mid` as their shape.
    pub inner_layers: Vec<PadInnerLayerOverride>,

    /// Hole shape (round, square, or slot).
    pub hole_shape: PadShape,

    /// Slot dimensions (only meaningful for slot holes).
    pub slot_size: Coord,
    pub slot_rotation: f64,
}

/// Shape and size for a pad on a specific layer.
#[derive(Debug, Clone)]
pub struct PadLayerShape {
    pub shape: PadShape,
    pub x_size: Coord,
    pub y_size: Coord,
    /// Corner radius percentage (0-100) for rounded rectangle shapes.
    pub corner_radius_pct: u8,
}

/// Override for a specific inner layer's pad shape.
#[derive(Debug, Clone)]
pub struct PadInnerLayerOverride {
    /// Inner layer index (0 = first inner layer).
    pub inner_layer_index: usize,
    pub shape: PadLayerShape,
}
```

### Changes to Pad

```rust
pub struct Pad {
    // Existing fields unchanged
    pub id: String,
    pub pad_name: String,
    pub layer: LayerRef,
    pub net: Option<String>,
    pub component: Option<String>,
    pub location: CoordPoint,
    pub shape: PadShape,       // top layer shape (kept for convenience/compat)
    pub x_size: Coord,         // top layer x_size (kept for convenience/compat)
    pub y_size: Coord,         // top layer y_size (kept for convenience/compat)
    pub rotation: f64,
    pub hole_size: Coord,
    pub is_plated: bool,
    pub pad_mode: PadStackMode,
    pub solder_mask_expansion: Coord,
    pub paste_mask_expansion: Coord,
    pub plane_connection: PlaneConnectionStyle,
    pub relief_conductor_width: Coord,
    pub relief_entries: i32,
    pub relief_air_gap: Coord,

    // NEW
    /// Full pad stack with per-layer shapes.
    /// For Simple mode, top/mid/bot all have the same shape as `self.shape`.
    pub stack: PadStack,
}
```

The existing `shape`/`x_size`/`y_size` fields remain as convenience accessors for
the top layer (the common case). `stack` provides the full picture.

### Spec Language

```
pad 1 {
    at: (0, 0)
    layer: multi_layer
    shape: round
    x_size: 1.6mm
    y_size: 1.6mm
    hole_size: 0.8mm
    is_plated: true
    pad_mode: local_stack

    stack {
        top { shape: round, x_size: 1.6mm, y_size: 1.6mm }
        mid { shape: round, x_size: 1.5mm, y_size: 1.5mm }
        bot { shape: round, x_size: 1.6mm, y_size: 1.6mm }
        hole_shape: round

        inner 2 { shape: rectangular, x_size: 1.4mm, y_size: 1.4mm }
    }
}
```

For simple pads (the majority), `stack` is omitted and inferred from the top-level
shape/size:

```
pad 1 {
    at: (0, 0)
    shape: round
    x_size: 1.6mm
    y_size: 1.6mm
    // stack is auto-populated: top = mid = bot = { round, 1.6mm, 1.6mm }
}
```

### Dump Output

Simple pads dump without `stack {}` (no noise). Only pads with `pad_mode != Simple`
that have differing per-layer shapes dump the stack block:

```
pad 1 { at: (0, 0), shape: round, x_size: 1.6mm, y_size: 1.6mm, hole_size: 0.8mm }
pad 2 {
    at: (2.54mm, 0), shape: rectangular, x_size: 1.2mm, y_size: 2.0mm
    pad_mode: local_stack
    stack {
        top { shape: rectangular, x_size: 1.2mm, y_size: 2.0mm }
        mid { shape: round, x_size: 1.0mm, y_size: 1.0mm }
        bot { shape: rectangular, x_size: 1.2mm, y_size: 2.0mm }
    }
}
```

---

## 4. Design Rule Parameters

### Problem

The current `DesignRule` exposes only metadata (`name`, `kind`, `enabled`, `priority`,
`scope`, `comment`) but none of the rule-specific parameter values. Internally, all
55+ rule variants are fully parsed into typed structs in `PcbRuleKindData`.

Consumers need:
- **Gerber**: Solder mask expansion, paste mask expansion values
- **DRC**: Clearance values, width constraints, hole sizes
- **Placement**: Component clearance, board outline clearance
- **Spec language**: Ability to declare and modify rule values

### Design Approach

Expose rule parameters via a `RuleParams` enum that mirrors the internal
`PcbRuleKindData` but uses public types. We don't expose all 55+ variants
immediately — start with the rules that downstream consumers actually need,
with an `Other` fallback for the rest.

### Public API Design

```rust
/// Design rule parameter values, specific to the rule kind.
///
/// This exposes the most commonly-needed rule parameters. Rule kinds not yet
/// covered by a dedicated variant use `Other { kind }` which exposes the kind
/// but not the parameters (they're preserved on roundtrip internally).
#[derive(Debug, Clone)]
pub enum RuleParams {
    /// Copper clearance between objects.
    Clearance {
        gap: Coord,
    },

    /// Width constraints for routing.
    Width {
        min: Coord,
        max: Coord,
        preferred: Coord,
    },

    /// Component-to-component clearance.
    ComponentClearance {
        gap: Coord,
    },

    /// Board outline clearance.
    BoardOutlineClearance {
        gap: Coord,
    },

    /// Solder mask expansion from pad edges.
    SolderMaskExpansion {
        expansion: Coord,
        /// Tent vias on top side (no mask opening).
        is_tenting_top: bool,
        /// Tent vias on bottom side.
        is_tenting_bottom: bool,
    },

    /// Paste mask expansion (usually negative = contraction).
    PasteMaskExpansion {
        expansion: Coord,
    },

    /// Hole size constraints.
    HoleSize {
        min: Coord,
        max: Coord,
    },

    /// Hole-to-hole clearance.
    HoleToHoleClearance {
        gap: Coord,
    },

    /// Minimum annular ring width.
    MinimumAnnularRing {
        min: Coord,
    },

    /// Short circuit allowance.
    ShortCircuit {
        allowed: bool,
    },

    /// Minimum solder mask sliver.
    MinimumSolderMaskSliver {
        min_width: Coord,
    },

    /// Silkscreen to solder mask clearance.
    SilkToSolderMaskClearance {
        gap: Coord,
    },

    /// Silkscreen to silkscreen clearance.
    SilkToSilkClearance {
        gap: Coord,
    },

    /// Power plane connect style (thermal relief parameters).
    PowerPlaneConnectStyle {
        connect_style: PlaneConnectionStyle,
        relief_conductor_width: Coord,
        relief_entries: i32,
        relief_air_gap: Coord,
    },

    /// Polygon connect style.
    PolygonConnectStyle {
        connect_style: PlaneConnectionStyle,
        relief_conductor_width: Coord,
        relief_entries: i32,
        relief_air_gap: Coord,
    },

    /// Power plane clearance.
    PowerPlaneClearance {
        clearance: Coord,
    },

    /// Matched net lengths.
    MatchedLengths {
        tolerance: Coord,
    },

    /// Differential pairs routing.
    DifferentialPairsRouting {
        gap: Coord,
        max_uncoupled_length: Coord,
    },

    /// Max/min height constraint.
    Height {
        min: Coord,
        max: Coord,
        preferred: Coord,
    },

    /// Routing topology.
    RoutingTopology {
        topology: String,  // "shortest", "horizontal", "vertical", "star", "daisy"
    },

    /// Any rule kind not yet covered by a dedicated variant.
    /// The internal data is preserved on roundtrip but not exposed for reading.
    Other {
        kind: RuleKind,
    },
}
```

### Changes to DesignRule

```rust
pub struct DesignRule {
    // Existing fields unchanged
    pub id: String,
    pub name: String,
    pub kind: RuleKind,
    pub enabled: bool,
    pub priority: i32,
    pub scope: String,
    pub comment: String,

    // NEW
    /// Rule-specific parameter values.
    pub params: RuleParams,

    /// Secondary scope expression (for binary rules like clearance).
    pub scope2: String,

    /// Net scope (same net, different nets, any).
    pub net_scope: String,

    /// Layer scope (same layer, adjacent, any).
    pub layer_scope: String,
}
```

### Spec Language

```
rule "Clearance_Default" {
    kind: "clearance"
    enabled: true
    priority: 1
    scope: "All"
    scope2: "All"
    gap: 6mil
}

rule "Width_Signal" {
    kind: "width"
    enabled: true
    priority: 1
    scope: "InNetClass('Signal')"
    min: 4mil
    max: 50mil
    preferred: 10mil
}

rule "SMExpansion" {
    kind: "solder_mask_expansion"
    enabled: true
    priority: 1
    scope: "All"
    expansion: 4mil
    tenting_top: true
    tenting_bottom: true
}
```

### Dump Output

```
rule "Clearance_Default" { kind: "clearance", enabled: true, priority: 1, scope: "All", gap: 6mil }
rule "Width_Signal" { kind: "width", enabled: true, priority: 1, scope: "InNetClass('Signal')", min: 4mil, max: 50mil, preferred: 10mil }
rule "SMExpansion" { kind: "solder_mask_expansion", enabled: true, priority: 1, expansion: 4mil, tenting_top: true, tenting_bottom: true }
```

---

## 5. Net Connectivity

### Problem

The current API has `Net` objects and primitives with `net: Option<String>`, but
no pre-built connectivity graph. Consumers need:
- **Gerber X2**: `TO.N,{net}*`, `TO.P,{refdes},{pin}*` for each pad
- **DRC**: Pin-to-pin connectivity for broken nets checking
- **Placement**: Net topology for HPWL wirelength estimation
- **Spec language**: Net-level queries ("all pins on GND")

### Design

Rather than changing existing types, add a **computed view** via a method on
`PcbDocBoard`. The connectivity data is built from existing fields (iterate
pads, group by net).

```rust
/// Pre-built connectivity information for net-level queries.
#[derive(Debug, Clone)]
pub struct BoardConnectivity {
    /// Per-net pin lists, keyed by net name.
    pub net_pins: Vec<NetPinList>,
}

/// All pins connected to a single net.
#[derive(Debug, Clone)]
pub struct NetPinList {
    /// Net name.
    pub net_name: String,

    /// All pins (pads) on this net with their component context.
    pub pins: Vec<NetPin>,

    /// Number of distinct components this net touches.
    pub component_count: usize,
}

/// A single pin (pad) in the connectivity graph.
#[derive(Debug, Clone)]
pub struct NetPin {
    /// Component designator (None for free-standing pads).
    pub component: Option<String>,

    /// Pad name within the component ("1", "A3", "GND").
    pub pad_name: String,

    /// Pad world position (for wirelength estimation).
    pub location: CoordPoint,
}

impl PcbDocBoard {
    /// Build the connectivity graph from pads and nets.
    ///
    /// Groups all pads by their net, producing per-net pin lists with
    /// component context. This is computed on demand (not cached) since
    /// the board data may change between calls.
    pub fn connectivity(&self) -> BoardConnectivity {
        let mut by_net: IndexMap<String, Vec<NetPin>> = IndexMap::new();

        for pad in &self.pads {
            if let Some(net_name) = &pad.net {
                by_net.entry(net_name.clone()).or_default().push(NetPin {
                    component: pad.component.clone(),
                    pad_name: pad.pad_name.clone(),
                    location: pad.location,
                });
            }
        }

        let net_pins = by_net.into_iter().map(|(net_name, pins)| {
            let component_count = pins.iter()
                .filter_map(|p| p.component.as_deref())
                .collect::<std::collections::HashSet<_>>()
                .len();
            NetPinList { net_name, pins, component_count }
        }).collect();

        BoardConnectivity { net_pins }
    }
}
```

### Spec Language

Connectivity is derived from pad/net assignments — no dedicated syntax needed.
The dump can optionally show a connectivity summary:

```
// Net summary (informational, not compilable)
// net "GND" { pins: U1.3, U1.7, C1.2, C2.2, R1.1 }
// net "VCC" { pins: U1.1, C1.1, C2.1 }
```

---

## 6. Query Helpers (New Methods)

```rust
impl PcbDocBoard {
    // Existing methods unchanged...

    // NEW: Layer queries
    /// All primitives on a given layer (tracks, arcs, pads, fills, regions, texts).
    pub fn primitives_on_layer(&self, layer: &LayerRef) -> LayerPrimitives<'_>;

    /// All tracks on a given layer.
    pub fn tracks_on_layer(&self, layer: &LayerRef) -> Vec<&Track>;

    /// All pads on a given layer (includes multi-layer pads).
    pub fn pads_on_layer(&self, layer: &LayerRef) -> Vec<&Pad>;

    // NEW: Polygon queries
    /// All regions that belong to a given polygon (by polygon name).
    pub fn regions_for_polygon(&self, polygon_name: &str) -> Vec<&Region>;

    // NEW: Drill queries
    /// All vias grouped by their layer pair (from_layer, to_layer).
    pub fn vias_by_drill_pair(&self) -> Vec<DrillPairGroup<'_>>;

    /// All plated through-hole pads.
    pub fn plated_through_hole_pads(&self) -> Vec<&Pad>;

    /// All non-plated through-hole pads.
    pub fn non_plated_through_hole_pads(&self) -> Vec<&Pad>;
}

/// Vias grouped by their layer pair for drill file generation.
#[derive(Debug)]
pub struct DrillPairGroup<'a> {
    pub from_layer: LayerRef,
    pub to_layer: LayerRef,
    pub vias: Vec<&'a Via>,
}
```

---

## Implementation Priority

### Phase 1: Gerber-critical (implement first)

1. **LayerStack** — extract from `PcbBoardConfig.v9_stack_layers` (or v8/v7/legacy
   fallback) into the new public type. Wire into `BoardSettings`.
2. **PadStack** — expose `PcbPad.size_mid`/`size_bot`/`stack_data` through new
   `Pad.stack` field. Update `pad_from_internal()` in pcbdoc_read.rs.
3. **RuleParams** — map internal `PcbRuleKindData` variants to public `RuleParams`.
   Start with Clearance, Width, SolderMaskExpansion, PasteMaskExpansion,
   ComponentClearance, BoardOutlineClearance.

### Phase 2: DRC/Placement-critical

4. **BoardGeometry** — preserve arc segments during `contour_to_coord_points()`,
   extract cutouts and keepouts, compute bounding box.
5. **BoardConnectivity** — implement `connectivity()` method.
6. **Query helpers** — `tracks_on_layer()`, `pads_on_layer()`, `vias_by_drill_pair()`.

### Phase 3: Spec language

7. **Dump updates** — extend `dump_pcbdoc()` to emit layer_stack, pad stack,
   rule params, geometry.
8. **Compiler/executor** — handle new fields in the PcbDoc spec compilation path.

---

## Relationship to IR Crate

This design deliberately extends the **existing `PcbDocBoard` API** rather than
creating a separate IR crate. Rationale:

- Gerber export is a **rendering** task, not a **solving** task — it needs
  format-faithful data in Altium coordinates, not mm/f64 domain abstractions
- The PcbDocBoard API is already the public interface; splitting into IR adds
  a layer with no benefit for Gerber
- The solverang IR (from `docs/future/solverang/ir.md`) remains a future separate
  crate that transforms `PcbDocBoard` into solver-optimized types (mm coordinates,
  typed handles, precomputed bounding boxes)

Data flow:
```
PcbDoc file
    ↓ open() + board()
PcbDocBoard (THIS API — format-faithful, Coord units)
    ↓                        ↓
Gerber writer               PcbIr extractor (future)
    ↓                        ↓
.GTL/.GBL/.DRL files        solverang solver
```
