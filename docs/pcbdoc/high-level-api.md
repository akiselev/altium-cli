# PcbDoc High-Level API Design

## Overview

The PcbDoc high-level API provides a public, domain-typed interface for reading and
writing PCB board designs. It follows the established patterns from SchLib, PcbLib, and
SchDoc APIs while handling PcbDoc's unique cross-reference model.

PcbDoc is the most complex Altium document type. Unlike SchLib/PcbLib (keyed collections
of named entities) or SchDoc (a tree linked via OWNERINDEX), PcbDoc uses a **flat,
section-based storage model** where every primitive independently references up to 5
different parent collections via indices. A single track can simultaneously belong to a
net, a component, and a polygon.

## Root API Surface

```rust
impl PcbDoc {
    // Existing
    pub fn open(path: impl AsRef<Path>) -> Result<Self>;
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()>;
    pub fn validate_invariants(&self) -> Result<()>;
    pub fn version_header(&self) -> &str;
    pub fn minor_version(&self) -> f64;

    // New: high-level API
    pub fn board(&self) -> Result<PcbDocBoard>;
    pub fn update_board(&mut self, board: &PcbDocBoard) -> Result<()>;
}
```

The `board()` / `update_board()` pair follows the SchDoc `sheet()` / `update_sheet()`
pattern: extract a fully-resolved public type from internal storage, modify it, write
it back with format-internal field preservation.

## PcbDocBoard — The Root Type

```rust
pub struct PcbDocBoard {
    // Board-level metadata (from Board6)
    pub settings: BoardSettings,

    // Named collections (parameter sections)
    pub nets: Vec<Net>,
    pub components: Vec<PcbDocComponent>,
    pub polygons: Vec<Polygon>,
    pub classes: Vec<NetClass>,
    pub rules: Vec<DesignRule>,
    pub differential_pairs: Vec<DifferentialPair>,

    // Primitives (from binary sections, cross-referenced)
    pub tracks: Vec<Track>,
    pub arcs: Vec<Arc>,
    pub vias: Vec<Via>,
    pub pads: Vec<Pad>,
    pub fills: Vec<Fill>,
    pub texts: Vec<Text>,
    pub regions: Vec<Region>,
    pub component_bodies: Vec<ComponentBody>,

    // Dimensions and coordinates (prefixed param sections)
    pub dimensions: Vec<Dimension>,

    // Models (3D)
    pub models: Vec<Model3D>,
}
```

### Design Decision: Typed Vectors vs. Enum

Unlike SchDoc which uses a `SheetObject` enum, PcbDoc stores primitives in
**type-specific vectors**. This matches the file format (separate sections per type)
and makes type-safe queries natural:

```rust
// PcbDoc: type-specific vectors (matches file format)
board.tracks.iter().filter(|t| t.net == Some("VCC"))

// vs. SchDoc: enum variant filtering
sheet.objects.iter().filter_map(|o| match o { SheetObject::Wire(w) => Some(w), _ => None })
```

The typed-vector approach is better for PcbDoc because:
1. Primitives in PcbDoc are stored in separate sections (Arcs6, Tracks6, etc.)
2. Each type has vastly different fields — a union enum would be unwieldy
3. Query/render consumers always filter by type anyway
4. Index-based cross-references (net_index, component_index) are type-specific

## Stable Human-Readable IDs

### Problem

PcbDoc primitives need stable identity for:
1. **Spec language**: declarative specs must reference specific primitives
2. **Query engine**: selectors need addressable entities
3. **Reconciler**: diff/plan/apply needs to match spec objects to existing objects
4. **Rendering**: object identity for interactive selection

Altium files provide two identity mechanisms:
- `UniqueID` strings (8-char alpha, e.g., "LVUUGVHQ") from UniqueIDPrimitiveInformation
- 128-bit GUIDs from PrimitiveGuids sidecar

Neither is human-readable or stable across spec compilation.

### Solution: The `id` Field

Every API type gets an `id: String` field as its primary stable identity:

```rust
pub struct Track {
    pub id: String,          // Stable identity
    pub start: CoordPoint,
    pub end: CoordPoint,
    pub width: Coord,
    pub layer: LayerRef,
    pub net: Option<String>,         // Resolved net name
    pub component: Option<String>,   // Resolved component designator
    // ...
}
```

### Spec Syntax: Block-Level Names

The ID is provided as an **optional name at the block level**, following the same
pattern as existing entity declarations (`component R_0603 { ... }`,
`net_label VCC { ... }`, `pad A1 { ... }`):

```
track main_bus {
    start: (0, 0)
    end: (100mil, 0)
    width: 10mil
    layer: top
    net: VCC
}

via { ... }            // anonymous — gets auto-generated ID

track power_rail {
    start: $main_bus.end    // reference via $ prefix
    end: (200mil, 0)
    width: 10mil
}
```

The syntax is: `TYPE [NAME] { ... }` where NAME follows the existing `entity_name`
grammar — unquoted identifiers, integers, or quoted strings:

```
entity_name = STRING | IDENT | INTEGER ;
```

Examples:
```
track main_bus { ... }             // unquoted identifier
track "USB D+" { ... }            // quoted string (special chars)
via 1 { ... }                     // integer name
via { ... }                       // anonymous (auto-generated ID)
```

Named objects can be referenced elsewhere in the spec using `$name`. If the name
is a valid identifier, use `$main_bus` directly. If the name is a quoted string
that isn't a valid identifier (e.g., `"USB D+"`), use a `let` binding for the
`$` reference:

```
let usb_dp = track "USB D+" { ... }
track "USB D-" {
    start: $usb_dp.end             // reference the let-binding name
    ...
}
```

`let` bindings also work as an alternative way to name objects. The binding name
provides the `$` reference, while the block-level name (if present) provides the ID:

```
let t = track main_bus { ... }    // id = "main_bus", reference as $t
let t = track { ... }             // id from position (track_N), reference as $t
track main_bus { ... }            // id = "main_bus", reference as $main_bus
```

### ID Sources and Generation

IDs come from three sources depending on context:

#### 1. Block-level name (spec-provided)

When the user names an object at the block level:
```
track main_bus { ... }     // id = "main_bus"
via bypass_cap { ... }     // id = "bypass_cap"
```

#### 2. Compiler-generated defaults (positional)

When no name is provided, the compiler generates a stable ID based on the
object's **positional index within its type** in the spec file:

```
track { ... }                // id = "track_0" (first track in spec)
track { ... }                // id = "track_1" (second track)
track main_bus { ... }       // id = "main_bus" (explicit)
track { ... }                // id = "track_3" (counter includes ALL tracks)
```

The positional counter advances for ALL objects of a type, including named ones.
This ensures that adding/removing a name from an object doesn't shift the
auto-generated IDs of other anonymous objects.

#### 3. File-derived (reading existing PcbDoc)

When reading from an existing PcbDoc file (no spec involved), IDs are synthesized
from the object's type and section index:
```
track_0, track_1, ...     // Tracks6[0], Tracks6[1], ...
pad_0, pad_1, ...         // Pads6[0], Pads6[1], ...
via_0, via_1, ...         // Vias6[0], Vias6[1], ...
```

### ID Format

```
{type}_{index}              — file-derived or compiler default
{name}                      — block-level name or let-binding
```

The `type` prefix uses lowercase singular form matching the API type name:
`track`, `arc`, `via`, `pad`, `fill`, `text`, `region`, `body`.

### ID Rename and the Reconciler

**Problem**: If the compiler auto-generates `track_0` and the user later adds a
name (`track main_bus { ... }`), the reconciler would see `track_0` disappear and
`main_bus` appear — treating it as delete + create, losing Altium's internal
UniqueID/GUID and breaking schematic-to-PCB linking.

**Solution: Positional-index fallback matching**

The compiler tracks each object's **positional index** (its order within its type
in the spec file) alongside the final ID. The reconciler uses a two-pass match:

1. **First pass — match by ID**: Exact ID match between spec and existing file.
   Handles the common case (IDs haven't changed).

2. **Second pass — match by positional index**: For unmatched items, fall back
   to matching the spec's Nth track to the existing file's Nth track. This
   handles the rename case: `track_0` (position 0) becomes `main_bus` (still
   position 0) — recognized as a rename, not delete+create.

3. **Remaining unmatched**: Spec-only items are creates. File-only items are
   deletes (or preserved if the spec is additive-only).

This means:
- **Adding `id: "main_bus"` to an existing track**: Recognized as rename via
  positional match. Altium UniqueID/GUID preserved.
- **Reordering tracks in the spec**: Position changes cause re-matching. If
  objects have explicit IDs, they match by ID regardless of position.
- **Recommendation**: Use explicit `id:` fields for any object you plan to
  reference by name. Let the auto-generated IDs handle "bulk" objects.

### ID Stability Guarantees

- **Within a spec**: IDs are deterministic — same spec always produces same IDs
- **File-derived**: IDs are stable across open/save cycles (section order preserved)
- **Reconciler matching**: ID first, positional-index fallback, then natural keys
  (pad_name for pads, net_name for nets, designator for components)

### Relationship to Altium UniqueID/GUID

The `id` field is our API concept — human-readable, stable, spec-friendly.
Altium's native identity (UniqueID 8-char string, 128-bit GUID) is preserved as
an internal detail during roundtrip but not exposed in the public API as the
primary identity. During `update_board()`, existing UniqueID/GUID values are
preserved for objects that match by `id` (or positional fallback).

## Named Collection Types

### Net

```rust
pub struct Net {
    pub id: String,          // Usually same as name: "GND", "VCC3P3"
    pub name: String,        // Net name (from NAME parameter)
    pub color: Color,
    pub visible: bool,
}
```

Net IDs default to the net name since names are unique within a board.

### PcbDocComponent

```rust
pub struct PcbDocComponent {
    pub id: String,          // Usually same as designator: "U1", "R47"
    pub designator: String,  // Reference designator (SOURCEDESIGNATOR)
    pub pattern: String,     // Footprint pattern name
    pub comment: String,
    pub location: CoordPoint,
    pub rotation: f64,
    pub layer: LayerRef,     // Top or Bottom
    pub source_library: String,
    pub source_lib_reference: String,
}
```

Component IDs default to the designator since designators are unique within a board.

### Polygon

```rust
pub struct Polygon {
    pub id: String,
    pub name: String,
    pub net: Option<String>,         // Net name for copper pour
    pub layer: LayerRef,
    pub connect_style: PlaneConnectionStyle,
    pub pour_order: i32,
    pub vertices: Vec<CoordPoint>,
    // Thermal relief settings
    pub relief_conductor_width: Coord,
    pub relief_entries: i32,
    pub relief_air_gap: Coord,
}
```

### NetClass

```rust
pub struct NetClass {
    pub id: String,          // Same as name
    pub name: String,
    pub kind: ClassKind,     // Net, Component, etc.
    pub members: Vec<String>,
}
```

### DesignRule

```rust
pub struct DesignRule {
    pub id: String,          // Same as name
    pub name: String,
    pub kind: RuleKind,
    pub enabled: bool,
    pub priority: i32,
    pub scope: String,       // Scope expression
    pub comment: String,
    // Rule-specific data handled by the RuleKind enum
}
```

## Primitive Types

All primitive types share this pattern:
- `id: String` — stable identity
- `layer: LayerRef` — PCB layer
- `net: Option<String>` — resolved net name (None = unconnected)
- `component: Option<String>` — resolved component designator (None = free-standing)
- Type-specific geometry fields

### Track

```rust
pub struct Track {
    pub id: String,
    pub layer: LayerRef,
    pub net: Option<String>,
    pub component: Option<String>,
    pub start: CoordPoint,
    pub end: CoordPoint,
    pub width: Coord,
}
```

### Arc

```rust
pub struct Arc {
    pub id: String,
    pub layer: LayerRef,
    pub net: Option<String>,
    pub component: Option<String>,
    pub center: CoordPoint,
    pub radius: Coord,
    pub start_angle: f64,
    pub end_angle: f64,
    pub width: Coord,
}
```

### Via

```rust
pub struct Via {
    pub id: String,
    pub net: Option<String>,
    pub component: Option<String>,
    pub location: CoordPoint,
    pub diameter: Coord,
    pub hole_size: Coord,
    pub from_layer: LayerRef,
    pub to_layer: LayerRef,
    pub solder_mask_expansion: Option<Coord>,
}
```

### Pad

```rust
pub struct Pad {
    pub id: String,
    pub pad_name: String,        // "1", "2", "A1" — natural key within component
    pub layer: LayerRef,
    pub net: Option<String>,
    pub component: Option<String>,
    pub location: CoordPoint,
    pub shape: PadShape,
    pub x_size: Coord,
    pub y_size: Coord,
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
}
```

Pad IDs default to `pad_{index}` from file, or `spec:{context}:{pad_name}` from spec.
The `pad_name` field serves as the natural key within a component for reconciliation.

### Fill

```rust
pub struct Fill {
    pub id: String,
    pub layer: LayerRef,
    pub net: Option<String>,
    pub component: Option<String>,
    pub corner1: CoordPoint,
    pub corner2: CoordPoint,
    pub rotation: f64,
}
```

### Text

```rust
pub struct Text {
    pub id: String,
    pub layer: LayerRef,
    pub component: Option<String>,
    pub location: CoordPoint,
    pub text: String,
    pub height: Coord,
    pub width: Coord,
    pub rotation: f64,
    pub font_name: String,
    pub is_mirrored: bool,
    pub is_comment: bool,
    pub is_designator: bool,
}
```

### Region

```rust
pub struct Region {
    pub id: String,
    pub layer: LayerRef,
    pub net: Option<String>,
    pub component: Option<String>,
    pub kind: RegionKind,
    pub outline: Vec<CoordPoint>,
    pub holes: Vec<Vec<CoordPoint>>,
    pub is_board_cutout: bool,
    pub is_keepout: bool,
}
```

### ComponentBody

```rust
pub struct ComponentBody {
    pub id: String,
    pub layer: LayerRef,
    pub component: Option<String>,
    pub standoff_height: Coord,
    pub overall_height: Coord,
    pub body_color_3d: Color,
    pub body_opacity_3d: f64,
    pub model_name: String,
    pub outline: Vec<CoordPoint>,
}
```

## Query Helpers on PcbDocBoard

```rust
impl PcbDocBoard {
    // Net queries
    pub fn net(&self, name: &str) -> Option<&Net>;
    pub fn tracks_for_net(&self, net_name: &str) -> Vec<&Track>;
    pub fn pads_for_net(&self, net_name: &str) -> Vec<&Pad>;
    pub fn vias_for_net(&self, net_name: &str) -> Vec<&Via>;

    // Component queries
    pub fn component(&self, designator: &str) -> Option<&PcbDocComponent>;
    pub fn pads_for_component(&self, designator: &str) -> Vec<&Pad>;
    pub fn tracks_for_component(&self, designator: &str) -> Vec<&Track>;
    pub fn bodies_for_component(&self, designator: &str) -> Vec<&ComponentBody>;

    // Layer queries
    pub fn primitives_on_layer(&self, layer: LayerRef) -> BoardLayerView<'_>;

    // Rule queries
    pub fn rule(&self, name: &str) -> Option<&DesignRule>;
    pub fn rules_for_kind(&self, kind: RuleKind) -> Vec<&DesignRule>;
}
```

## Cross-Reference Resolution

### Read Path (internal -> public)

During `board()`, the read path resolves indices to names:

```
net_index: 5       -> net: Some("VCC")       (lookup in Nets6)
component_index: 2 -> component: Some("U1")  (lookup in Components6)
polygon_index: 0   -> (polygon membership tracked internally)
```

Index 0xFFFF (65535 for u16, -1 for i16) means "none" and maps to `None`.

### Write Path (public -> internal)

During `update_board()`, the write path resolves names back to indices:

```
net: Some("VCC")  -> net_index: 5       (lookup net position in board.nets)
component: Some("U1") -> component_index: 2  (lookup in board.components)
```

### Sidecar Merging

All sidecar data (WideStrings6, UniqueIDPrimitiveInformation, PrimitiveGuids,
ExtendedPrimitiveInformation) is merged into the API types at read time and
regenerated at write time. The public API is sidecar-agnostic.

## BoardSettings

The Board6 section contains extensive configuration. We expose a curated subset:

```rust
pub struct BoardSettings {
    // Identity
    pub document_name: String,

    // Layer stack (summary — full editing is complex)
    pub signal_layer_count: i32,

    // Board outline
    pub board_outline: Option<Vec<CoordPoint>>,

    // Grid
    pub snap_grid_size: Coord,
    pub visible_grid_size: Coord,

    // Units
    pub display_unit: DisplayUnit,
}
```

Full layer stack editing (V6/V8/V9 stacks, dielectric properties, impedance
profiles) is deferred — the internal data is preserved during roundtrip but not
exposed for mutation in the initial API.

## File Layout

```
crates/altium-format/src/api/
    pcbdoc_types.rs    — Public API types defined above
    pcbdoc_read.rs     — Internal -> public conversion (board() implementation)
    pcbdoc_write.rs    — Public -> internal conversion (update_board() implementation)
```

Re-exported from `api/mod.rs`:
```rust
pub use pcbdoc_types::{
    PcbDocBoard, BoardSettings,
    Net, PcbDocComponent, Polygon, NetClass, DesignRule,
    Track, Arc, Via, Pad, Fill, Text, Region, ComponentBody,
    Dimension, Model3D,
};
```

## Relationship to PcbLib API Types

PcbDoc and PcbLib share the same underlying primitive parsers (PcbPad, PcbVia, etc.)
but their **API types are separate**:

- `pcblib_types::Pad` — footprint-level pad (no net, no component context)
- `pcbdoc_types::Pad` — board-level pad (has net, component, board-level ID)

The internal-to-public conversion paths (`pcblib_read.rs` vs `pcbdoc_read.rs`) handle
the different contexts. Common conversion logic for shared primitive fields can be
extracted to helper functions.

## Extended API (v2)

See [high-level-api-v2.md](high-level-api-v2.md) for the v2 extensions that add:

- **LayerStack** — physical layer ordering, copper/dielectric thicknesses
- **BoardGeometry** — arc-preserving outlines, cutouts, keepouts, bounding box
- **PadStack** — per-layer pad shapes for multi-layer Gerber aperture generation
- **RuleParams** — typed design rule parameter values (clearance, width, expansion)
- **BoardConnectivity** — pre-built net-to-pin connectivity graph

These extensions are needed by downstream consumers: Gerber export, DRC engine,
placement solver, and the spec language.

## What's Deferred

The initial API is **read-only with write support for core collections**. Deferred:

1. **Full Board6 layer stack editing** — preserved on roundtrip but not exposed
2. **DRC violation read/write** — violations preserved internally, not in public API
3. **Constraint manager XML** — preserved on roundtrip
4. **Union management** — union names, relations, features preserved internally
5. **Embedded boards/objects** — preserved on roundtrip
6. **DrillManager/LettersGeometry** — cache data, preserved on roundtrip
7. **Advanced router/placer/pin swap options** — preserved on roundtrip
