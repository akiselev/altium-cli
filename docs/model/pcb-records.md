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

| ID | Rust Type | Purpose |
|----|-----------|---------|
| 1 | `PcbArc` | Circular arc on a layer |
| 2 | `PcbPad` | Component connection pad (multi-layer, multi-shape) |
| 3 | `PcbVia` | Plated through-hole connecting layers |
| 4 | `PcbTrack` | Routed copper line segment |
| 5 | `PcbText` | Text string (silkscreen, copper, etc.) |
| 6 | `PcbFill` | Solid rectangular fill |
| 10 | `PcbPolygon` | Copper pour polygon |
| 11 | `PcbRegion` | Keepout/board outline region |
| 12 | `PcbComponentBody` | 3D component body definition |
| 13 | `PcbDimension` | Measurement annotation |
| 14 | `PcbCoordinate` | Position marker |

Unknown object IDs are captured as `PcbRecord::Unknown { object_id, raw_data }`.

## Dispatch Enum

```rust
// crates/altium-format/src/records/pcb/primitive.rs
pub enum PcbRecord {
    Arc(PcbArc),
    Pad(Box<PcbPad>),        // Boxed — PcbPad is large (per-layer arrays)
    Via(PcbVia),
    Track(PcbTrack),
    Text(PcbText),
    Fill(PcbFill),
    Region(PcbRegion),
    ComponentBody(Box<PcbComponentBody>),
    Polygon(PcbPolygon),
    Dimension(Box<PcbDimension>),
    Coordinate(PcbCoordinate),
    Unknown { object_id: PcbObjectId, raw_data: Vec<u8> },
}
```

`PcbPad`, `PcbComponentBody`, and `PcbDimension` are boxed because they contain
large fixed-size arrays or many fields.

## Base Type

### PcbPrimitiveCommon

All PCB primitives share a common header:

```rust
// crates/altium-format/src/records/pcb/primitive.rs
pub struct PcbPrimitiveCommon {
    pub layer: Layer,              // PCB layer (u8)
    pub flags: PcbFlags,           // Bitmask (u16)
    pub unique_id: Option<String>, // Optional UUID
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
// crates/altium-format/src/records/pcb/track.rs
#[derive(AltiumRecord)]
#[altium(format = "binary")]
pub struct PcbTrack {
    #[altium(flatten)]
    pub common: PcbPrimitiveCommon,
    #[altium(coord_point)]
    pub start: CoordPoint,          // Start coordinate
    #[altium(coord_point)]
    pub end: CoordPoint,            // End coordinate
    #[altium(coord)]
    pub width: Coord,               // Track width
    #[altium(unknown_binary)]
    pub unknown: Vec<u8>,           // 16 bytes: net ID, rule refs, etc.
}
```

Binary layout:
```
[PcbPrimitiveCommon]    layer(u8) + flags(u16) + unique_id
[i32 start_x]
[i32 start_y]
[i32 end_x]
[i32 end_y]
[i32 width]
[16 bytes unknown]      net ID, rule references
```

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
`width` (trace width of the arc).

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
rotation angle.

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
└── (component primitives     Pads/tracks belonging to components are
     are associated by         linked by component index, not owner_index)
     component index)
```

There is no `OWNERINDEX` linking. Component ownership is determined by which
stream the primitive appears in or by a component index field.
