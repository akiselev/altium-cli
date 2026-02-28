# PCB Records

PCB files (`.PcbDoc`, `.PcbLib`) store data as **binary records** — little-endian
structs inside size-prefixed blocks within CFB streams.

## Record Format

Each record block starts with a `u8` object ID byte, followed by the struct
fields in a fixed binary layout:

```
┌─────────────────────────────────────────────────┐
│ [u8 object_id]                                  │
│ [struct fields — little-endian, packed]          │
│   layer: u8                                     │
│   flags: u16                                    │
│   ... type-specific fields ...                  │
└─────────────────────────────────────────────────┘
```

Coordinates are stored as `i32` little-endian values (10,000 units = 1 mil).
See [Coordinate System](coordinates.md) for details.

## Object ID Table

| ID | Rust Type | Purpose | Context |
|----|-----------|---------|---------|
| 1 | `PcbArc` | Circular arc on a layer | PcbLib + PcbDoc |
| 2 | `PcbPad` | Component connection pad (multi-layer, multi-shape) | PcbLib + PcbDoc |
| 3 | `PcbVia` | Plated through-hole connecting layers | PcbLib + PcbDoc |
| 4 | `PcbTrack` | Routed copper line segment | PcbLib + PcbDoc |
| 5 | `PcbText` | Text string (silkscreen, copper, etc.) | PcbLib + PcbDoc |
| 6 | `PcbFill` | Solid rectangular fill | PcbLib + PcbDoc |
| 10 | `PcbPolygon` | Copper pour polygon | PcbDoc only |
| 11 | `PcbRegion` | Keepout/board outline region | PcbLib + PcbDoc |
| 12 | `PcbComponentBody` | 3D component body definition | PcbLib + PcbDoc |
| 13 | `PcbDimension` | Measurement annotation | PcbDoc only |
| 14 | `PcbCoordinate` | Position marker | PcbDoc only |

Unknown object IDs produce a hard parse error — there is no `Unknown` catch-all variant.

## Dispatch Enum

```rust
// crates/altium-format/src/pcblib/mod.rs
pub(crate) enum PcbPrimitive {
    Arc(PcbArc),
    Pad(PcbPad),
    Via(PcbVia),
    Track(PcbTrack),
    Text(PcbText),
    Fill(PcbFill),
    Region(PcbRegion),
    ComponentBody(PcbComponentBody),
}
```

This enum covers the 8 primitive types present in PcbLib footprints. `Polygon`,
`Dimension`, and `Coordinate` (IDs 10, 13, 14) are PcbDoc-only and not part of
PcbLib footprints.

## Base Type

### PcbPrimitiveCommon

All PCB primitives share a common header:

```rust
// crates/altium-format/src/pcblib/mod.rs
pub struct PcbPrimitiveCommon {
    pub layer: V6Layer,           // PCB layer (u8)
    pub flags: PcbFlags,          // Bitmask (u16 LE)
    pub net_index: u16,           // Net index (0xFFFF = no net)
    pub polygon_index: u16,       // Polygon pour index (0xFFFF = none)
    pub component_index: u16,     // Component index (0xFFFF = none)
    pub coordinate_index: u16,    // Coordinate index (0xFFFF = none)
    pub dimension_index: u16,     // Dimension index (0xFFFF = none)
}
```

#### PcbFlags

| Flag | Meaning |
|------|---------|
| UNLOCKED | Primitive can be moved/edited |
| TENTING_TOP | Solder mask tenting on top |
| TENTING_BOTTOM | Solder mask tenting on bottom |
| FABRICATION_TOP | Fabrication output on top |
| FABRICATION_BOTTOM | Fabrication output on bottom |
| KEEPOUT | Keepout region marker |

#### Layer

The `Layer` type wraps a `u8` value representing the PCB layer:

| Value | Layer |
|-------|-------|
| 0 | Top signal (copper) |
| 1-30 | Mid signal layers |
| 31 | Bottom signal (copper) |
| 32 | Top overlay (silkscreen) |
| 33 | Bottom overlay |
| 34 | Top paste |
| 35 | Bottom paste |
| 36 | Top solder mask |
| 37 | Bottom solder mask |
| 38-53 | Internal planes 1-16 |
| 54 | Drill guide |
| 55 | Keepout |
| 56-71 | Mechanical layers 1-16 |
| 72 | Drill drawing |
| 73 | Multi-layer |

---

## Key Record Types

### PcbPad (Object ID 2)

The most complex PCB primitive. Pads support different shapes and sizes on each
of the 32 layers, making this a large struct.

```rust
// crates/altium-format/src/records/pcb/pad.rs
pub struct PcbPad {
    pub common: PcbPrimitiveCommon,
    pub designator: String,                        // Pin name (e.g., "1", "GND")
    pub location: CoordPoint,                      // Center position
    pub rotation: f64,                             // Rotation in degrees
    pub is_plated: bool,                           // PTH (true) vs NPTH/SMD (false)
    pub jumper_id: i16,                            // Jumper connection ID
    pub stack_mode: PcbStackMode,                  // Simple / TopMiddleBottom / FullStack
    pub hole_size: Coord,                          // Drill hole diameter
    pub hole_shape: PcbPadHoleShape,               // Round / Square / Slot
    pub hole_rotation: f64,                        // Hole rotation (for slots)
    pub hole_slot_length: Coord,                   // Slot length (if hole_shape = Slot)
    pub paste_mask_expansion: MaskExpansion,        // Auto or Manual(Coord)
    pub solder_mask_expansion: MaskExpansion,       // Auto or Manual(Coord)
    pub size_layers: [CoordPoint; 32],             // Pad size (width, height) per layer
    pub shape_layers: [PcbPadShape; 32],           // Pad shape per layer
    pub corner_radius_percentage: [u8; 32],        // Corner radius % per layer
    pub offsets_from_hole_center: [CoordPoint; 32], // Pad offset from hole per layer
}
```

#### Stack Modes

The `stack_mode` field controls how per-layer pad data is interpreted:

| Mode | Meaning |
|------|---------|
| `Simple` | One size/shape used for all layers (index 0) |
| `TopMiddleBottom` | Three sizes: top (index 0), middle (index 1), bottom (index 31) |
| `FullStack` | Independent size/shape for each of the 32 layers |

#### Pad Shapes

| Variant | Visual |
|---------|--------|
| `NoShape` | No pad (placeholder) |
| `Round` | Circular pad |
| `Rectangular` | Sharp-cornered rectangle |
| `Octagonal` | Octagon |
| `RoundRect` | Rectangle with rounded corners |
| `RotatedRect` | Rotated rectangle |

#### Mask Expansion

```rust
pub enum MaskExpansion {
    Auto,            // Altium calculates from design rules
    Manual(Coord),   // User-specified expansion value
}
```

#### Example: SMD Pad

A typical surface-mount pad might be:
- `is_plated = false`, `hole_size = 0`
- `stack_mode = Simple`
- `shape_layers[0] = RoundRect`
- `size_layers[0] = CoordPoint { x: 600_000, y: 1_000_000 }` (60 mil x 100 mil)
- `corner_radius_percentage[0] = 25`

#### Example: Through-Hole Pad

A typical through-hole pad:
- `is_plated = true`, `hole_size = 400_000` (40 mil drill)
- `stack_mode = Simple`
- `shape_layers[0] = Round`
- `size_layers[0] = CoordPoint { x: 700_000, y: 700_000 }` (70 mil diameter)

### PcbTrack (Object ID 4)

A single line segment of routed copper.

```rust
// crates/altium-format/src/v2/records/pcb_track.rs
pub struct PcbTrackRecord {
    pub header: PcbCommonHeader,     // 13-byte common header
    pub start_x: PcbCoord,
    pub start_y: PcbCoord,
    pub end_x: PcbCoord,
    pub end_y: PcbCoord,
    pub width: PcbCoord,
    pub subpoly_index: u16,
    pub user_routed: bool,           // AD26+
    pub union_index: i32,            // AD26+
    pub track_kind: u8,              // AD26+
    pub layer_enum_index: i32,       // AD26+
    pub keepout_restrictions: i32,   // AD26+
}
```

Binary layout:
```
[13-byte common header]
[i32 start_x]
[i32 start_y]
[i32 end_x]
[i32 end_y]
[i32 width]
[u16 subpoly_index]
[u8 user_routed]           AD26+
[i32 union_index]          AD26+
[u8 track_kind]            AD26+
[i32 layer_enum_index]     AD26+
[i32 keepout_restrictions] AD26+
```

Observed sizes:
- Legacy: 35 bytes
- AD26: 49 bytes

### PcbVia (Object ID 3)

A plated hole connecting two or more copper layers.

```rust
// crates/altium-format/src/records/pcb/via.rs
pub struct PcbVia {
    pub common: PcbPrimitiveCommon,
    pub location: CoordPoint,                      // Center position
    pub hole_size: Coord,                          // Drill hole diameter
    pub from_layer: Layer,                         // Start layer
    pub to_layer: Layer,                           // End layer
    pub thermal_relief_air_gap_width: Coord,
    pub thermal_relief_conductors: u8,
    pub thermal_relief_conductors_width: Coord,
    pub solder_mask_expansion: MaskExpansion,
    pub diameter_stack_mode: PcbStackMode,         // Simple / TopMiddleBottom / FullStack
    pub diameters: [Coord; 32],                    // Via diameter per layer
    pub unknown: Vec<u8>,
}
```

Via types are determined by `from_layer` and `to_layer`:
- **Through-hole**: Top (0) to Bottom (31)
- **Blind**: Top or Bottom to an internal layer
- **Buried**: Internal layer to internal layer

### PcbArc (Object ID 1)

A circular arc on a copper or mechanical layer.

Fields include: `location` (center), `radius`, `start_angle`, `end_angle`,
`width` (trace width of the arc), plus AD26 trailing fields
(`user_routed`, `union_index`, `layer_enum_index`, `keepout_restrictions`).

Observed sizes:
- Legacy: 47 bytes
- AD26: 60 bytes

### PcbText (Object ID 5)

Text strings for silkscreen, copper layers, or mechanical layers.

Key fields:
- `text: String` — the text content
- `location: CoordPoint` — position
- `height: Coord` — text height
- `rotation: f64` — rotation angle
- `font_kind: PcbTextKind` — `Stroke`, `TrueType`, or `BarCode`
- `justification: PcbTextJustification` — 9-position grid (BottomLeft to
  TopRight)
- `is_mirrored: bool`

### PcbFill (Object ID 6)

A solid rectangular copper fill. Defined by two corner coordinates and a
rotation angle, plus AD26 trailing fields (`user_routed`, `union_index`,
`layer_enum_index`, `keepout_restrictions`).

Observed sizes:
- Legacy: 37 bytes
- AD26: 50 bytes

### PcbPolygon (Object ID 10)

A copper pour polygon with thermal relief, clearance, and connectivity rules.
Contains a list of vertex points defining the polygon boundary. Used for ground
planes and power pours.

### PcbRegion (Object ID 11)

A closed region used for board outlines, keepout areas, or other non-copper
boundaries. Similar to polygon but typically on mechanical or keepout layers.

### PcbComponentBody (Object ID 12)

3D component body information including step model references, bounding box,
and standoff height.

### PcbDimension (Object ID 13)

Measurement annotations showing distances, angles, or radii on the PCB.
Includes the dimension kind (linear, angular, radial, leader, etc.),
reference points, and text formatting.

## PCB Hierarchy

Unlike schematics, PCB files use a **flat ownership model**:

```
PcbDoc
├── /Components6/Data         Component metadata (designator, pattern, comment)
│   ├── Component 0           Parameters for each placed component
│   ├── Component 1
│   └── …
├── /Primitives6/Data         Board-level primitives (not component-owned)
│   ├── PcbTrack              Free-standing traces
│   ├── PcbVia                Board-level vias
│   └── …
└── (Component primitives are associated by component index, not owner_index)
```

There is no `OWNERINDEX` linking. Component ownership is determined by which
stream the primitive appears in or by a component index field.
