# Binary Primitives

All PCB primitives in PcbDoc are stored as packed binary structs within section Data
streams (e.g., `/Arcs6/Data`, `/Tracks6/Data`). This document covers the binary layout
of each primitive type found in PcbDoc files.

Coordinates are `i32` little-endian values: 10,000 internal units = 1 mil. See
[../dxp/coordinates.md](../dxp/coordinates.md).

## Record framing

Each section Data stream contains a sequence of records with the following framing:

```
[u8   object_id]          // TObjectId: Arc=1, Pad=2, Via=3, Track=4, etc.
[u32  record_length]      // LE, high byte may contain flags; mask with 0x00FFFFFF
[N    payload]            // record_length bytes of packed binary data
```

The corresponding Header stream contains a single `u32 LE` record count.

**PcbDoc vs PcbLib framing difference**: PcbDoc uses global per-type section streams
(`/Arcs6/Data`, `/Tracks6/Data`, etc.) where all records of the same type are packed
sequentially. PcbLib stores records per-component in `/<component>/Data` streams,
prefixed with a pascal string (u8 length + name bytes) containing the footprint pattern
name.

## Common header

All PCB primitives share a 13-byte common header at the start of their payload:

```
Offset  Size  Type    Field
0       1     u8      layer               // PCB layer (V6 layer number)
1       2     u16     flags               // Primitive flags bitmask (LE)
3       2     i16     net_index           // Net index (-1 = no net)
5       2     i16     unknown1            // Always -1 in observed data
7       2     i16     component_index     // Component index (-1 = none)
9       2     i16     polygon_index       // Polygon pour index (-1 = none)
11      2     i16     unknown2            // Always -1 in observed data
```

**Note on field sizes**: In PcbDoc, `net_index`, `component_index`, and
`polygon_index` are `i16` (2 bytes each), not `i32` as in some older format versions.
The value `-1` (`0xFFFF`) indicates "not associated."

**Note on `unknown1` and `unknown2`**: These 2-byte fields at offsets 5 and 11 are
always `0xFFFF` (i16 = -1) across all observed primitive types. Their purpose is not
yet determined. They may be reserved for coordinate or dimension association indices.

### Flags bitmask

Observed flag values and their correlations:

| Value  | Binary             | Context |
|--------|--------------------|---------|
| 0x000C | `0000000000001100` | All tracks, arcs, fills |
| 0x002C | `0000000000101100` | Vias (tenting bottom) |
| 0x004C | `0000000001001100` | Vias (tenting top) |
| 0x006C | `0000000001101100` | Vias (tenting both) |

Based on the `IPCB_Primitive` interface, the flags encode properties such as:
- Bit 2: Unknown (always set in observed data)
- Bit 3: Unknown (always set in observed data)
- Bit 5: `IsTenting_Bottom`
- Bit 6: `IsTenting_Top`
- Other bits: `IsKeepout`, `UserRouted`, `TearDrop`, `AllowGlobalEdit`, etc.

## Arc (TObjectId = 1)

A circular arc on a copper or mechanical layer.

Stored in: `/Arcs6/Data`

```
Offset  Size  Type    Field
0       13    -       common header
13      4     i32     center_x
17      4     i32     center_y
21      4     i32     radius
25      8     f64     start_angle         // degrees (0.0 = 3 o'clock)
33      8     f64     end_angle           // degrees (360.0 = full circle)
41      4     i32     width               // arc line width
```

Observed size: **60 bytes** (AD26 format).

AD26 trailing fields (after byte 45):
```
45      2     u16     subpoly_index       // sub-polygon index (0 = none)
47      1     u8      user_routed         // 0 = auto-routed, 1 = manual
48      4     i32     union_index         // smart union index (0 = none)
52      4     i32     layer_enum_index    // V7 layer identifier
56      4     i32     keepout_restrictions // keepout restriction flags (0 = none)
```

**Note**: Unlike the PcbLib format, PcbDoc arcs do NOT have an `arc_kind` byte in the
trailing fields. The `subpoly_index` field is placed after the legacy arc data and
before the other AD26 trailing fields.

### Example: Full circle arc

```
start_angle = 0.0, end_angle = 360.0 -> full circle
layer = 33 (Bottom overlay) -> silkscreen marking
net_index = -1 -> no net association
component_index = 6 -> belongs to component 6
```

## Pad (TObjectId = 2)

The most complex primitive. Uses **multiple subrecords** within a single record entry.

Stored in: `/Pads6/Data`

Pad records in PcbDoc contain embedded parameter strings (pipe-delimited `|KEY=VALUE|`
format) interleaved with binary data. The first 4 bytes after the record header appear
to be a pascal-style string length for a pad name/designator, followed by parameter
data.

Observed sizes: **Variable** (typically 150-200+ bytes per pad record, with multiple
subrecords contributing to a total of ~400-700 bytes per pad).

### Subrecord structure

The Pads6/Data stream contains multiple blocks. The first block appears to contain
the main pad data. Each pad consists of multiple size-prefixed sub-blocks:

1. **Subrecord 0: Main pad data** - Location, sizes, shapes, drill hole, rotation
2. **Subrecords 1-5: Extended data** - Per-layer arrays, corner radii, offsets,
   thermal relief settings

Due to the complexity and variable-length nature of pad records (per-layer arrays for
32 layers of shape, size, corner radius, and offset data), the full pad binary layout
requires extensive analysis. See [../pcblib/binary-primitives.md](../pcblib/binary-primitives.md)
for the PcbLib pad layout, which shares the same field structure.

### Pad shapes (u8)

| Value | Shape |
|-------|-------|
| 0 | NoShape |
| 1 | Round |
| 2 | Rectangular |
| 3 | Octagonal |
| 4 | RoundRect |
| 5 | RotatedRect |

### Stack modes

| Value | Mode |
|-------|------|
| 0 | Simple -- one size/shape for all layers (top layer values used) |
| 1 | TopMiddleBottom -- three sizes: top, mid, bottom |
| 2 | FullStack -- independent per each of 32 layers |

## Via (TObjectId = 3)

A plated through-hole connecting copper layers.

Stored in: `/Vias6/Data`

```
Offset  Size  Type    Field
0       13    -       common header
13      4     i32     location_x
17      4     i32     location_y
21      4     i32     hole_size           // Drill hole diameter
25      ...   -       (diameter data, per-layer arrays, thermal relief, etc.)
```

Observed size: **316 bytes** (AD26 format, all records uniform).

Via records are large because they contain per-layer diameter arrays (32 layers x 4
bytes = 128 bytes) and additional thermal relief, solder mask, and stack mode settings.
The layer field in the common header is typically `0x4A` (74), which represents the
multi-layer/via layer.

The `from_layer` and `to_layer` fields determine via type:
- **Through-hole**: Top (0) to Bottom (31)
- **Blind**: Top (0) or Bottom (31) to an internal layer
- **Buried**: Internal layer to internal layer

Via flags typically include tenting bits:
- `0x006C`: Tenting on both top and bottom
- `0x002C`: Tenting on bottom only
- `0x004C`: Tenting on top only

## Track (TObjectId = 4)

A single routed line segment.

Stored in: `/Tracks6/Data`

```
Offset  Size  Type    Field
0       13    -       common header
13      4     i32     start_x
17      4     i32     start_y
21      4     i32     end_x
25      4     i32     end_y
29      4     i32     width
33      2     u16     subpoly_index       // sub-polygon index (0 = none)
```

Observed size: **49 bytes** (AD26 format).

AD26 trailing fields (after byte 35):
```
35      1     u8      user_routed         // 0 = auto-routed, 1 = manual
36      4     i32     union_index         // smart union index (0 = none)
40      1     u8      track_kind          // 0 = normal
41      4     i32     layer_enum_index    // V7 layer identifier
45      4     i32     keepout_restrictions // keepout restriction flags (0 = none)
```

### Example: Copper trace

```
layer = 1 (Top copper)
flags = 0x000C
net_index = 88 -> connected to net 88
component_index = -1 -> free-standing routed trace
width = 39370 -> 39370 / 10000 = 3.937 mils = 0.1mm
```

### Example: Board outline track

```
layer = 57 (Mechanical 2)
flags = 0x000C
net_index = -1 -> no net
component_index = 143 -> belongs to component 143
layer_enum_index = 0x01020001 -> V7: Mechanical category, index 1
```

## Text (TObjectId = 5)

Text strings. Uses **multiple subrecords**.

Stored in: `/Texts6/Data`

Text records in PcbDoc are complex and may use a format where the text string subrecord
is embedded within the main record. The Texts6/Data stream in the test file contains
very few records at the binary level (3 records), though the Header indicates 1339 text
primitives. This suggests that text records may be packed differently -- potentially with
all text primitives concatenated within a single large binary record, or using the
block-level structure for grouping.

### Subrecord 0: Text properties

```
Offset  Size  Type    Field
0       13    -       common header
13      4     i32     location_x
17      4     i32     location_y
21      4     i32     height              // Text height
25      8     f64     rotation            // Degrees
33      1     bool    is_mirrored
34      4     i32     stroke_width        // For stroke font
38      1     bool    is_comment          // Whether this displays the Comment
39      1     bool    is_designator       // Whether this displays the Designator
40      1     u8      font_kind           // 0=Stroke, 1=TrueType, 2=BarCode
... (additional fields including font ID, justification, etc.)
```

### Subrecord 1: Text string

```
[4 bytes] u32 LE: subrecord length
[N bytes] Win1252 text string
```

Unicode text is stored in the `/WideStrings6/Data` sidecar stream and overrides the
Win1252 value.

## Fill (TObjectId = 6)

A solid rectangular fill.

Stored in: `/Fills6/Data`

```
Offset  Size  Type    Field
0       13    -       common header
13      4     i32     corner1_x
17      4     i32     corner1_y
21      4     i32     corner2_x
25      4     i32     corner2_y
29      8     f64     rotation            // Degrees
```

Observed size: **50 bytes** (AD26 format).

AD26 trailing fields (after byte 37):
```
37      1     u8      user_routed         // 0 = auto-routed, 1 = manual
38      4     i32     union_index         // smart union index (0 = none)
42      4     i32     layer_enum_index    // V7 layer identifier
46      4     i32     keepout_restrictions // keepout restriction flags (0 = none)
```

**Note**: Fill trailing fields do NOT include a `fill_kind` or `subpoly_index` byte.
The trailing is simply `user_routed(1) + union_index(4) + layer_enum_index(4) + keepout_restrictions(4) = 13 bytes`.

### Example: Copper fill

```
layer = 1 (Top copper)
net_index = 88 -> connected to net 88
component_index = 143 -> belongs to component 143
corner1 = (55793713, 44150009)
corner2 = (58549619, 46905915)
rotation = 180.0 degrees
layer_enum_index = 0x01000001 -> V7: Signal category, layer 1
```

## Region (TObjectId = 11)

Closed polygonal regions (board outlines, keepout areas, copper shapes).

Stored in: `/Regions6/Data` (legacy) and `/ShapeBasedRegions6/Data` (AD26+)

Regions have variable-length records due to vertex arrays.

```
Offset  Size  Type    Field
0       13    -       common header
13      1     u8      region_kind         // Region type
14      ...   -       (varies by format version)
N       4     i32     vertex_count        // Number of outline vertices
N+4     -     -       vertices[]          // Array of CoordPoint (x:i32, y:i32)
```

Each vertex is 8 bytes (x: i32, y: i32). The total record size varies based on the
number of vertices.

**ShapeBasedRegions6 vs Regions6**: In AD26 files, both sections exist and contain the
same record count. `ShapeBasedRegions6` is the authoritative modern format;
`Regions6` may contain a legacy-compatible copy.

## ComponentBody (TObjectId = 12)

3D component body. Contains a reference to a 3D model in `/Models/Data`.

Stored in: `/ComponentBodies6/Data` (legacy) and `/ShapeBasedComponentBodies6/Data` (AD26+)

```
Offset  Size  Type    Field
0       13    -       common header
... (body outline and properties)
N       -     GUID    model_id            // References a model in /Models/Data
... (standoff height, rotation offsets, etc.)
```

ComponentBody records contain:
- 2D outline (vertex list for the body footprint)
- 3D model reference (GUID matching `ID` in `/Models/Data`)
- Standoff height
- Rotation offsets (X, Y, Z)
- Body projection settings

**ShapeBasedComponentBodies6 vs ComponentBodies6**: Same relationship as the region
variants. Both exist with identical record counts in AD26 files.

## Polygon (TObjectId = 10)

Copper pour polygon with thermal relief, clearance, and connectivity rules.

Stored in: `/Polygons6/Data`

Polygons contain a list of vertex points defining the polygon boundary, along with
pour settings (connect style, air gap, conductor width, etc.). Used for ground planes
and power pours.

## Dimension (TObjectId = 13)

Measurement annotations on the PCB.

Stored in: `/Dimensions6/Data`

Includes the dimension kind (linear, angular, radial, leader, etc.), reference points,
and text formatting.

## AD26 trailing fields

Many primitive types include trailing fields added in the AD26 format version. The
general pattern for AD26 trailing fields is:

```
[u8   user_routed]          // 0 = auto-routed, 1 = manually routed
[i32  union_index]          // Smart union membership (0 = none)
[u8   kind]                 // Type-specific kind (track_kind, etc.) -- NOT always present
[i32  layer_enum_index]     // V7 layer identifier (replaces V6 layer byte)
[i32  keepout_restrictions] // Keepout restriction flags (0 = none)
```

**Variations by primitive type**:

| Primitive | subpoly_index | kind byte | Total trailing |
|-----------|---------------|-----------|----------------|
| Arc       | Yes (u16, before user_routed) | No | 15 bytes |
| Track     | In legacy section (u16 at offset 33) | Yes (track_kind) | 14 bytes |
| Fill      | No | No | 13 bytes |
| Via       | Complex (embedded in larger record) | - | - |

### V7 layer identifiers (layer_enum_index)

The `layer_enum_index` field uses the AD26 V7 layer encoding format:

```
0x01CCNNNN
  CC = category (00=Signal, 02=Mechanical, 03=Special)
  NNNN = index within category
```

Observed mappings:

| layer_enum_index | V6 Layer | Description |
|------------------|----------|-------------|
| 0x01000001 | 1 | Top copper (signal 1) |
| 0x01000004 | 4 | Mid3 (signal 4) |
| 0x01000005 | 5 | Mid4 (signal 5) |
| 0x0100000A | 10 | Mid9 (signal 10) |
| 0x0100FFFF | 73 | Multi-layer |
| 0x01020001 | 57 | Mechanical 1 (index into mechanical) |
| 0x0102000D | 69 | Mechanical 13 |
| 0x0102000E | 70 | Mechanical 14 |
| 0x0102000F | 71 | Mechanical 15 |
| 0x01030006 | 33 | Bottom overlay (silkscreen) |
| 0x01030007 | 34 | Top paste |
| 0x0103000A | 37 | Bottom solder mask |

## Differences from PcbLib format

The PcbDoc binary primitive format is identical in field layout to PcbLib with these
key differences:

1. **Stream organization**: PcbDoc uses global per-type section streams (`/Arcs6/Data`,
   `/Tracks6/Data`, etc.). PcbLib stores all primitive types together per-component in
   `/<component>/Data` streams.

2. **Common header field sizes**: In PcbDoc, `net_index`, `component_index`, and
   `polygon_index` are `i16` (2 bytes). In PcbLib, these fields may use different widths.
   Additionally, PcbDoc has 2 unknown `i16` fields at offsets 5 and 11 that are always -1.

3. **Association semantics**: In PcbDoc, `net_index`, `component_index`, and
   `polygon_index` carry real associations (tracks belong to nets, primitives belong to
   components). In PcbLib, these are typically -1 since footprint primitives are not yet
   placed on a board.

4. **ShapeBased sections**: PcbDoc contains both legacy (`Regions6`, `ComponentBodies6`)
   and modern (`ShapeBasedRegions6`, `ShapeBasedComponentBodies6`) variants with identical
   record counts. PcbLib typically only has the legacy format.

## Byte order

All multi-byte integer fields are **little-endian**. Floating-point values (`f64`) are
IEEE 754 little-endian (same byte order as x86/x64 native).
