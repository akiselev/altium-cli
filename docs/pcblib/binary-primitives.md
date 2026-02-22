# Binary Primitives

All PCB primitives in PcbLib are stored as packed binary structs. This document covers the
binary layout of each primitive type found in PcbLib footprints.

Coordinates are `i32` little-endian values: 10,000 internal units = 1 mil. See
[coordinate-system.md](coordinate-system.md).

## Common header

All PCB primitives share a 13-byte common header at the start of their first subrecord:

```
Offset  Size  Type    Field
0       1     u8      layer           // PCB layer (see enumerations.md)
2       2     u16     flags           // Primitive flags bitmask
4       4     i32     net_index       // Net index (-1 = no net)
8       2     u16     polygon_index   // Polygon pour index (0 = none)
10      2     u16     component_index // Component index (0 = none)
12      1     u8      unknown         // Unknown byte
```

**Note**: In PcbLib context, `net_index`, `polygon_index`, and `component_index` are
typically 0 or -1 since footprint primitives are not yet placed on a board. These fields
become meaningful in PcbDoc.

## Arc (TObjectId = 1)

A circular arc on a copper or mechanical layer.

```
Offset  Size  Type    Field
0       13    -       common header
13      4     i32     center_x
17      4     i32     center_y
21      4     i32     radius
25      8     f64     start_angle     // degrees
33      8     f64     end_angle       // degrees
41      4     i32     width           // arc line width
```

Observed sizes:
- Legacy: 45 bytes (subrecord payload)
- AD26: 58 bytes (additional trailing fields)

AD26 trailing fields (after byte 45):
```
45      1     u8      user_routed
46      4     i32     union_index
50      1     u8      arc_kind        // 0=normal, etc.
51      4     i32     layer_enum_index
55      4     i32     keepout_restrictions
```

## Pad (TObjectId = 2)

The most complex primitive. Uses **6 subrecords**.

### Subrecord 0: Main pad data

```
Offset  Size  Type    Field
0       13    -       common header
13      4     i32     location_x
17      4     i32     location_y
21      4     i32     size_top_x      // Top layer pad width
25      4     i32     size_top_y      // Top layer pad height
29      4     i32     size_mid_x      // Mid layer pad width
33      4     i32     size_mid_y      // Mid layer pad height
37      4     i32     size_bot_x      // Bottom layer pad width
41      4     i32     size_bot_y      // Bottom layer pad height
45      4     i32     hole_size       // Drill hole diameter
49      1     u8      shape_top       // Top layer shape
50      1     u8      shape_mid       // Mid layer shape
51      1     u8      shape_bot       // Bottom layer shape
52      8     f64     rotation        // Rotation in degrees
60      1     bool    is_plated       // PTH vs NPTH/SMD
61      1     u8      unknown1
62      1     u8      stack_mode      // 0=Simple, 1=TopMiddleBottom, 2=FullStack
63      4     i32     unknown2
67      4     i32     paste_mask_expansion
71      4     i32     solder_mask_expansion
... (more fields follow, including per-layer arrays)
```

The full pad record is approximately 500+ bytes for the first subrecord and varies
significantly based on format version (legacy vs AD26).

### Subrecords 1-5: Additional pad data

Each of the 5 additional subrecords carries supplementary pad information:
- Extended shapes for the full 32-layer stack
- Corner radius percentages per layer
- Offsets from hole center per layer
- Hole shape, slot dimensions, thermal relief settings
- AD26+ extended fields

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
| 0 | Simple — one size/shape for all layers (top layer values used) |
| 1 | TopMiddleBottom — three sizes: top, mid, bottom |
| 2 | FullStack — independent per each of 32 layers |

## Via (TObjectId = 3)

A plated through-hole connecting copper layers.

```
Offset  Size  Type    Field
0       13    -       common header
13      4     i32     location_x
17      4     i32     location_y
21      4     i32     hole_size       // Drill hole diameter
25      4     i32     diameter_top    // Via pad diameter (top layer)
29      4     i32     diameter_mid    // Via pad diameter (mid layers)
33      4     i32     diameter_bot    // Via pad diameter (bottom layer)
37      1     u8      from_layer      // Start layer
38      1     u8      to_layer        // End layer
... (additional fields follow)
```

Via types are determined by from_layer/to_layer:
- Through: Top(0) → Bottom(31)
- Blind: Top(0) or Bottom(31) → internal layer
- Buried: internal → internal

## Track (TObjectId = 4)

A single line segment.

```
Offset  Size  Type    Field
0       13    -       common header
13      4     i32     start_x
17      4     i32     start_y
21      4     i32     end_x
25      4     i32     end_y
29      4     i32     width
33      2     u16     subpoly_index
```

Observed sizes:
- Legacy: 35 bytes
- AD26: 49 bytes

AD26 trailing fields (after byte 35):
```
35      1     u8      user_routed
36      4     i32     union_index
40      1     u8      track_kind
41      4     i32     layer_enum_index
45      4     i32     keepout_restrictions
```

## Text (TObjectId = 5)

Text strings. Uses **2 subrecords**.

### Subrecord 0: Text properties

```
Offset  Size  Type    Field
0       13    -       common header
13      4     i32     location_x
17      4     i32     location_y
21      4     i32     height          // Text height
25      8     f64     rotation        // Degrees
33      1     bool    is_mirrored
34      4     i32     stroke_width    // For stroke font
38      1     bool    is_comment      // Whether this displays the Comment
39      1     bool    is_designator   // Whether this displays the Designator
40      1     u8      font_kind       // 0=Stroke, 1=TrueType, 2=BarCode
... (additional fields including font ID, justification, etc.)
```

### Subrecord 1: Text string

```
[4 bytes] u32 LE: subrecord length
[N bytes] Win1252 text string
```

The text string may be a literal value or a special token:
- `.Designator` — replaced with the component designator at placement
- `.Comment` — replaced with the component comment at placement
- `.Layer_Name` — replaced with the layer name

Unicode text is stored in the WideStrings sidecar stream and overrides this value.

## Fill (TObjectId = 6)

A solid rectangular fill.

```
Offset  Size  Type    Field
0       13    -       common header
13      4     i32     corner1_x
17      4     i32     corner1_y
21      4     i32     corner2_x
25      4     i32     corner2_y
29      8     f64     rotation        // Degrees
```

Observed sizes:
- Legacy: 37 bytes
- AD26: 50 bytes (additional trailing fields like Track/Arc)

## Region (TObjectId = 11)

A closed region (board outline, keepout, courtyard, copper shape).

```
Offset  Size  Type    Field
0       13    -       common header
13      1     u8      region_kind     // Region type (see enumerations)
14      4     i32     unknown
... (varies by format version)
N       4     i32     vertex_count    // Number of outline vertices
N+4     -     -       vertices[]      // Array of CoordPoint (x:i32, y:i32)
```

Regions have variable length due to the vertex array. Each vertex is 8 bytes (x:i32, y:i32).

The exact offset of the vertex count field depends on the format version and requires
careful analysis of the binary data for each specific region record.

## ComponentBody (TObjectId = 12)

3D component body. Contains a reference to a 3D model in the Library/Models storage.

```
Offset  Size  Type    Field
0       13    -       common header
... (body outline and properties)
N       -     GUID    model_id        // References a model in Library/Models/Data
... (standoff height, rotation offsets, etc.)
```

The ComponentBody record is large and complex, containing:
- 2D outline (vertex list for the body footprint)
- 3D model reference (GUID matching `ID` in Library/Models/Data)
- Standoff height
- Rotation offsets (X, Y, Z)
- Body projection settings

## Version-dependent record sizes

Many primitive types have version-dependent trailing fields. The record length field tells
you exactly how many bytes to read for each subrecord, so unknown trailing bytes can be
safely read and preserved even if not fully understood.

The general strategy for handling version-dependent sizes:
1. Read the known fields from the beginning of the record
2. If the record is longer than expected, store the remaining bytes as `trailing_bytes`
3. When writing, append the stored `trailing_bytes` after the known fields

This ensures round-trip fidelity even for fields we don't yet parse.

## Byte order

All multi-byte integer fields are **little-endian**. Floating-point values (`f64`) are
IEEE 754 little-endian (same byte order as x86/x64 native).
