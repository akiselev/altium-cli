# DXP File Format Documentation Synthesis

Comprehensive synthesis of all reverse-engineered documentation in `docs/dxp/`.
Source material: 20 documents covering container format, schematic/PCB records,
serialization, coordinates, sidecar streams, .NET/Delphi internals, API functions,
and pad field analysis.

---

## 1. Container Format (OLE/CFB)

All Altium Designer files are **OLE Compound Document Binary (CFB) Version 3**
files -- the same container used by legacy Microsoft Office (.doc, .xls). The
Rust `cfb` crate provides low-level access; `altium-format` wraps it for
Altium-specific stream layouts.

### 1.1 Block Encoding

Data within each CFB stream is organized as **size-prefixed blocks**:

```
i32 header:
  bits [0..23]  = payload size in bytes (header & 0x00FFFFFF)
  bits [24..31] = flags (header >> 24)
                  0x00 = parameter/ASCII record
                  0x01 = binary record

Followed by: payload (size bytes)
```

Some blocks use **zlib compression** (detected by a `0xD0` tag byte). The reader
skips the first 2 bytes (zlib header) and decompresses the remainder.

### 1.2 Stream Layouts by File Type

#### SchLib (Schematic Library)

```
CompoundFile
+-- /Storage                    Icon storage header block
+-- /FileHeader                 Component index + metadata
|   Block 0: ParameterCollection
|     HEADER = "Protel for Windows - Schematic Library..."
|     WEIGHT = <total primitives + aliases>
|     COMPCOUNT = <number of components>
|     LIBREF0, LIBREF1, ...         (component names)
|     PARTCOUNT0, PARTCOUNT1, ...   (parts per component)
|     COMPDESCR0, COMPDESCR1, ...   (descriptions)
|     ALIASCOUNT0, COMP0ALIAS0, ... (alias mappings)
+-- /SectionKeys                (optional) Maps long names > 31 chars
|   KEYCOUNT, LIBREF0/SECTIONKEY0, ...
+-- /{ComponentName}/           One OLE storage per component
    +-- Data                    Stream of SchRecord blocks
        Block 0: SchComponent     (RECORD=1)
        Block 1: SchPin           (RECORD=2)
        Block 2: SchSymbol        (RECORD=3)
        ...more primitives...
        Block N: (EOF)
```

- Component names exceeding the 31-char OLE storage limit use `/SectionKeys`
  to map a short key to the full `LIBREF`.
- Aliases are stored as redirect streams: the alias storage contains a single
  block with `|SECTIONNAME=<real_component_name>\0`.

#### SchDoc (Schematic Document)

```
CompoundFile
+-- /Storage                    Icon storage header
+-- /FileHeader                 All schematic primitives (flat list)
|   Block 0: ParameterCollection (HEADER, WEIGHT)
|   Block 1: SchSheetHeader     (RECORD=31)
|   Block 2: SchComponent       (RECORD=1)
|   Block 3: SchPin             (RECORD=2)
|   ...more primitives...
|   Block N: (EOF)
+-- /Additional                 (optional) Extra parameters
```

All primitives live in a single flat stream. Parent-child relationships are
encoded via `OWNERINDEX`.

#### PcbDoc (PCB Document)

```
CompoundFile
+-- /Board6/Data                Board-level parameters
+-- /Components6/Data           Component metadata (designator, pattern, comment)
+-- /Primitives6/Data           Board-level PCB primitives (binary)
|   Block 0: u32 record count
|   Block 1..N: binary primitive records
+-- /Nets6/Data                 Net names (string blocks)
+-- /Rules6/Data                Design rules (DRC)
+-- /Classes6/Data              Net/component classes
+-- ... (30+ total sections, see Section 6 for sidecar streams)
```

Each primitive section follows the pattern: `/{SectionName}/Header` (parameter
block with record count) + `/{SectionName}/Data` (binary records).

#### PcbLib (PCB Library)

Similar to PcbDoc but organized per-footprint, with each footprint in its own
OLE storage containing binary primitive blocks. Each footprint has its own
set of sections (Arcs6, Tracks6, Pads6, etc.) nested under a footprint storage.

#### PrjPcb (PCB Project)

Parameter-based format listing document paths, build configurations, and
variant definitions. Not binary.

#### IntLib (Integrated Library)

Bundles a SchLib and PcbLib into a single CFB container. The library extracts
embedded sub-files and delegates to the appropriate reader.

### 1.3 Parameter String Encoding

Schematic records use **pipe-delimited key=value strings** in Windows-1252:

```
|RECORD=2|OWNERINDEX=0|LOCATION.X=100|LOCATION.Y=200|NAME=VCC|DESIGNATOR=1|
```

Rules:
- **Nesting**: Level 0 uses `|`; level 1 uses backtick.
- **Unicode**: Values outside Windows-1252 use `%UTF8%` prefix in the key.
- **Booleans**: `T`/`F` (short) or `TRUE`/`FALSE` (long), context-dependent.
- **Order**: Preserved using `IndexMap` for round-trip fidelity.

---

## 2. Coordinate System

### 2.1 Internal Representation

Fixed-point integer system:

- **Resolution**: 10,000 internal units = 1 mil (0.001 inch)
- **Types**: `Coord(i32)`, `CoordPoint { x: Coord, y: Coord }`,
  `CoordRect { location1: CoordPoint, location2: CoordPoint }`

| Value | Internal units | Mils | mm |
|-------|---------------|------|----|
| 1 internal unit | 1 | 0.0001 | 0.0000254 |
| 1 mil | 10,000 | 1 | 0.0254 |
| 1 inch | 10,000,000 | 1,000 | 25.4 |
| 1 mm | ~393,701 | ~39.37 | 1 |

### 2.2 DXP Fractional Encoding (Schematic)

Coordinates are split into **two parameters**: integer + fractional.

```
LOCATION.X=100, LOCATION.X_FRAC=5000
raw = 100 * 100000 + 5000 = 10,005,000 (= 1000.5 mils)
```

Source: `Rt_Schematic.Consts.cBaseUnit = 100000` — each DXP unit = 10 mils.
Encoding: `integer = raw / 100000`, `frac = raw % 100000` (always 0..99999).
When frac is 0, the `_FRAC` parameter is typically omitted.
Non-canonical values are accepted on read and normalized on write.

### 2.3 PCB Binary Coordinates

Stored directly as **i32 little-endian** with the same 10,000 units/mil
resolution. No fractional split needed.

### 2.4 Color Encoding

Win32 COLORREF format: `0x00BBGGRR` (blue high byte, red low byte).
Stored as `i32` in parameter format.

---

## 3. Serialization System

### 3.1 Derive Macros

| Macro | Generates | Used on |
|-------|-----------|---------|
| `AltiumRecord` | `FromParams`/`ToParams` and/or `FromBinary`/`ToBinary` | Record structs |
| `AltiumBase` | Composition trait (`HasXxxBase`) + `FromParams`/`ToParams` | Base types |
| `AltiumEnum` | Integer-to-enum conversion + `FromParamValue`/`ToParamValue` | Enums |

### 3.2 Traits

**Parameter-based (Schematic)**:
- `FromParams` / `ToParams` -- serialize to/from `ParameterCollection`
- `from_params_preserving()` returns `(Self, UnknownFields)` for round-trip

**Binary-based (PCB)**:
- `FromBinary` / `ToBinary` -- serialize to/from binary streams
- `read_from_preserving()` returns `(Self, Vec<u8>)` for unknown trailing bytes

**Value conversion**:
- `FromParamValue` / `ToParamValue` -- single parameter string values
- Implemented for: `i32`, `f64`, `bool`, `String`, `Coord`, `Color`, all `AltiumEnum` types

**Polymorphic traits**:
- `SchPrimitive`: `RECORD_ID`, `owner_index()`, `set_owner_index()`, `calculate_bounds()`, `record_type_name()`
- `PcbPrimitive`: `OBJECT_ID`, `layer()`, `calculate_bounds()`

### 3.3 Field Attributes -- Parameter Format

| Attribute | Example | Purpose |
|-----------|---------|---------|
| `param = "KEY"` | `#[altium(param = "LIBREFERENCE")]` | Map field to parameter key |
| `default` | `#[altium(param = "X", default)]` | Use Default::default() if missing |
| `optional` | `#[altium(param = "X", optional)]` | Wrap in Option<T> |
| `skip_default` | `#[altium(param = "X", skip_default)]` | Omit on write if default |
| `frac = "KEY_FRAC"` | `#[altium(param = "X", frac = "X_FRAC")]` | DXP fractional coordinate |
| `indexed_coords` | see below | Variable-length vertex arrays |
| `flatten` | `#[altium(flatten)]` | Compose base struct fields |
| `color` | `#[altium(color)]` | Win32 COLORREF value |
| `list` | `#[altium(list)]` | Comma-separated values |
| `unknown` | `#[altium(unknown)]` | Capture unrecognized params |
| `skip` | `#[altium(skip)]` | Ignore during serialization |

Indexed coordinates:
```rust
#[altium(indexed_coords, prefix_x = "X", prefix_y = "Y", count = "LOCATIONCOUNT")]
pub vertices: Vec<(i32, i32)>,
```

### 3.4 Field Attributes -- Binary Format

| Attribute | Example | Purpose |
|-----------|---------|---------|
| `binary, ty = "i32le"` | `#[altium(binary, ty = "i32le")]` | Basic binary type |
| `coord_point` | `#[altium(coord_point)]` | Two consecutive i32le as (x,y) |
| `coord` | `#[altium(coord)]` | Single i32le coordinate |
| `string_block` | `#[altium(string_block)]` | i32 length + UTF-8 bytes |
| `pascal_string` | `#[altium(pascal_string)]` | u8 length + bytes |
| `array = N` | `#[altium(array = 32)]` | Fixed-size array |
| `skip_bytes = N` | `#[altium(skip_bytes = 10)]` | Read/discard N bytes |
| `unknown_binary` | `#[altium(unknown_binary)]` | Capture remaining bytes |

Supported binary types: `i8`, `u8`, `i16le`, `u16le`, `i32le`, `u32le`,
`i64le`, `u64le`, `f32le`, `f64le`, `bool`.

### 3.5 AltiumBase Derive

Generates a composition trait for base types with accessors for each field.
The `extends` attribute chains trait bounds (supertrait relationships).

### 3.6 AltiumEnum Derive

Generates bidirectional integer-to-enum conversion with a default fallback
variant for unknown values.

---

## 4. Schematic Records

### 4.1 Record Type Table

| RECORD | Rust Type | Purpose |
|--------|-----------|---------|
| 1 | `SchComponent` | Component instance (container for children) |
| 2 | `SchPin` | Electrical connection pin |
| 3 | `SchSymbol` | Graphical shape primitive |
| 4 | `SchLabel` | Text annotation |
| 5 | `SchBezier` | Bezier curve |
| 6 | `SchPolyline` | Multi-segment line |
| 7 | `SchPolygon` | Closed filled polygon |
| 8 | `SchEllipse` | Ellipse |
| 9 | `SchPie` | Pie/wedge shape |
| 11 | `SchEllipticalArc` | Elliptical arc |
| 12 | `SchArc` | Circular arc |
| 13 | `SchLine` | Single line segment |
| 14 | `SchRectangle` | Rectangle |
| 17 | `SchPowerObject` | Power/ground symbol |
| 18 | `SchPort` | Sheet port connector |
| 22 | `SchNoErc` | "No ERC" marker |
| 25 | `SchNetLabel` | Net name label |
| 26 | `SchBus` | Bus (multi-wire) connection |
| 27 | `SchWire` | Electrical wire |
| 28 | `SchTextFrame` | Rich text box |
| 29 | `SchJunction` | Wire junction dot |
| 30 | `SchImage` | Embedded image |
| 31 | `SchSheetHeader` | Sheet properties (page size, grid, fonts) |
| 34 | `SchDesignator` | Component reference designator text |
| 37 | `SchBusEntry` | Bus tap point |
| 41 | `SchParameter` | Named parameter (user attribute) |
| 43 | `SchWarningSign` | Warning annotation |
| 44 | `SchImplementationList` | Container for footprint assignments |
| 45 | `SchImplementation` | Single footprint assignment |
| 46 | `SchMapDefinerList` | Container for pin mappings |
| 47 | `SchMapDefiner` | Pin-to-pad mapping entry |
| 48 | `SchImplementationParameters` | Footprint parameters |
| 209 | `SchTextFrameVariant` | Variant-aware text frame |

Unknown record IDs are captured as `SchRecord::Unknown { record_id, params }`.

### 4.2 Base Types (Composition)

**SchPrimitiveBase** -- common to all schematic primitives:
- `owner_index: i32` -- OWNERINDEX, parent record index (-1 = root)
- `is_not_accessible: bool` -- ISNOTACCESIBLE
- `owner_part_id: Option<i32>` -- OWNERPARTID, multi-part part number
- `owner_part_display_mode: Option<i32>` -- OWNERPARTDISPLAYMODE
- `graphically_locked: bool` -- GRAPHICALLYLOCKED

**SchGraphicalBase** -- extends SchPrimitiveBase with position/color:
- `base: SchPrimitiveBase` (flattened)
- `location_x: i32` -- LOCATION.X + LOCATION.X_FRAC (DXP fractional)
- `location_y: i32` -- LOCATION.Y + LOCATION.Y_FRAC
- `color: i32` -- COLOR (Win32 COLORREF)
- `area_color: i32` -- AREACOLOR (fill color)

Most record types embed `SchGraphicalBase` via `#[altium(flatten)]`.

### 4.3 Ownership Model

Records form parent-child trees via `OWNERINDEX`:

```
SchComponent (OWNERINDEX=-1, index 0 in primitives list)
+-- SchPin        (OWNERINDEX=0)
+-- SchPin        (OWNERINDEX=0)
+-- SchRectangle  (OWNERINDEX=0)
+-- SchDesignator (OWNERINDEX=0)
+-- SchParameter  (OWNERINDEX=0)
+-- SchImplementationList (OWNERINDEX=0)
    +-- SchImplementation (OWNERINDEX=6)
        +-- SchMapDefiner (OWNERINDEX=7)
```

- `OWNERINDEX = -1` or absent: top-level record
- `OWNERINDEX = N`: owned by record at position N

### 4.4 Multi-Part Symbols and Display Modes

- `part_count` on SchComponent: total number of parts
- `owner_part_id` on children: which part (1-based; 0 = common to all)
- `display_mode_count` on SchComponent: number of display modes
- `owner_part_display_mode` on children: which mode (0 = common to all)

### 4.5 Key Record Detail: SchComponent (Record 1)

| Field | Parameter | Notes |
|-------|-----------|-------|
| `lib_reference` | LIBREFERENCE | Component name in library |
| `component_description` | COMPONENTDESCRIPTION | |
| `unique_id` | UNIQUEID | |
| `current_part_id` | CURRENTPARTID | |
| `part_count` | PARTCOUNT | Stored as value+1 in file |
| `display_mode_count` | DISPLAYMODECOUNT | |
| `display_mode` | DISPLAYMODE | |
| `show_hidden_pins` | SHOWHIDDENPINS | |
| `library_path` | LIBRARYPATH | Default: "*" |
| `source_library_name` | SOURCELIBRARYNAME | |
| `sheet_part_filename` | SHEETPARTFILENAME | |
| `target_filename` | TARGETFILENAME | |
| `override_colors` | OVERRIDECOLORS | |
| `designator_locked` | DESIGNATORLOCKED | |
| `part_id_locked` | PARTIDLOCKED | |
| `component_kind` | COMPONENTKIND | |
| `alias_list` | ALIASLIST | |
| `orientation` | ORIENTATION | Bitmask: ROTATED, FLIPPED |

### 4.6 Key Record Detail: SchPin (Record 2)

| Field | Parameter | Notes |
|-------|-----------|-------|
| `symbol_inner_edge` | SYMBOL_INNEREDGE | PinSymbol enum |
| `symbol_outer_edge` | SYMBOL_OUTEREDGE | PinSymbol enum |
| `symbol_inside` | SYMBOL_INSIDE | PinSymbol enum |
| `symbol_outside` | SYMBOL_OUTSIDE | PinSymbol enum |
| `symbol_line_width` | SYMBOL_LINEWIDTH | LineWidth enum |
| `description` | DESCRIPTION | |
| `formal_type` | FORMALTYPE | |
| `electrical` | ELECTRICAL | PinElectricalType (0-7) |
| `pin_conglomerate` | PINCONGLOMERATE | Bitmask flags |
| `pin_length` | PINLENGTH + PINLENGTH_FRAC | DXP fractional |
| `name` | NAME | Pin function name (e.g., "VCC") |
| `designator` | DESIGNATOR | Pin number (e.g., "1") |
| `swap_id_group` | SWAPIDGROUP | |
| `swap_id_part` | SWAPIDPART | |
| `swap_id_sequence` | SWAPIDSEQUENCE | |
| `hidden_net_name` | HIDDENNETNAME | |
| `default_value` | DEFAULTVALUE | |
| `pin_propagation_delay` | PINPROPAGATIONDELAY | f64 |
| `unique_id` | UNIQUEID | |

**Pin electrical types**: Input(0), InputOutput(1), Output(2), OpenCollector(3),
Passive(4, default), HiZ(5), OpenEmitter(6), Power(7).

**Pin conglomerate flags**: HIDE, DISPLAY_NAME_VISIBLE, DESIGNATOR_VISIBLE,
ROTATED, FLIPPED.

### 4.7 Key Record Detail: SchWire (Record 27)

| Field | Parameter | Notes |
|-------|-----------|-------|
| `line_width` | LINEWIDTH | LineWidth enum |
| `line_style` | LINESTYLE | LineStyle enum |
| `vertices` | X1/Y1, X2/Y2, ... | Indexed coordinates, count = LOCATIONCOUNT |

### 4.8 Implementation Records (44-48)

Footprint assignment hierarchy:
```
SchComponent
+-- SchImplementationList (RECORD=44, OWNERINDEX -> component)
    +-- SchImplementation (RECORD=45, OWNERINDEX -> impl list)
        +-- SchMapDefiner (RECORD=47, OWNERINDEX -> implementation)
        +-- SchImplementationParameters (RECORD=48, OWNERINDEX -> implementation)
```

---

## 5. PCB Records

### 5.1 Binary Record Format

PCB primitives use binary format:
```
u8   object_id    -- dispatches to type
[payload bytes]   -- type-specific binary fields
```

Within a section, each record is wrapped in a block:
```
u8   object_id
u32  payload_length (little-endian)
[payload_length bytes of data]
```

### 5.2 Record Type Table (TObjectId)

| ID | Rust Type | Purpose |
|----|-----------|---------|
| 1 | `PcbArc` | Circular arc |
| 2 | `PcbPad` | Through-hole or SMD pad |
| 3 | `PcbVia` | Plated through-hole via |
| 4 | `PcbTrack` | Copper trace segment |
| 5 | `PcbText` | Text string |
| 6 | `PcbFill` | Solid copper fill rectangle |
| 7 | (Connection) | Ratsnest connection (transient) |
| 8 | `PcbPolygon` | Copper pour polygon |
| 9 | `PcbDimension` | Dimension annotation |
| 11 | `PcbComponent` | Component footprint container |
| 12 | `PcbRegion` | Arbitrary copper region |
| 13 | `PcbComponentBody` | 3D model reference |
| 14 | (Embedded) | Embedded object |

Full TObjectId from Delphi has 26 types:
eArcObject(1), ePadObject(2), eViaObject(3), eTrackObject(4),
eTextObject(5), eFillObject(6), eConnectionObject(7),
eNetObject(8 -- also used for polygon in file format context),
eComponentObject(9 -- runtime ID, maps to 11 in file),
ePolyObject(10), eDimensionObject(11 -- runtime),
eCoordinateObject(12), eClassObject(13), eRuleObject(14),
eFromToObject(15), eEmbeddedBoardObject(16), eEmbeddedObject(17),
eRegionObject(18), eComponentBodyObject(19), eBoardOutlineObject(26).

### 5.3 Common Header (PcbPrimitiveCommon)

All PCB primitives share a common header:
- `layer: Layer` -- u8 layer value (V6 layer ID)
- `flags: PcbFlags` -- u16 bitmask
- `unique_id: Option<String>` -- populated from sidecar stream

**PcbFlags bitmask**:
- Bit 0: selected
- Bit 4: locked
- Bit 7: union member
- Other bits: type-specific

### 5.4 Layer System

**V6 Layer IDs (TV6_Layer)** -- byte-based, 82 layers:
- 1-32: Signal layers (TopLayer=1, MidLayer1-30=2-31, BottomLayer=32)
- 33-38: Mechanical layers (original 6)
- 39-46: Internal plane layers
- 47-52: Overlay/solder/paste mask pairs
- 53-54: Drill layers
- 55-56: Multi-layer, Connections
- 57-82: Additional mechanical layers

**V7 Layer System** -- 32-bit structured:
- `IV7_Layer` interface with genus/family/species/flags
- Supports unlimited user-defined layers
- Backwards-compatible: V6 IDs embedded in low byte

### 5.5 Key Record Detail: PcbPad (Object ID 2)

The most complex PCB primitive. Binary format with multiple sub-records.

**Stack modes**: Simple, TopMiddleBottom, FullStack
**Pad shapes**: Round, Rectangular, Octagonal, RoundedRectangle

Key fields:
| Field | Type | Notes |
|-------|------|-------|
| net | i16le | Net index |
| component | i16le | Component index |
| position | CoordPoint | Center position |
| top_size / mid_size / bot_size | CoordPoint | Pad dimensions per layer |
| top_shape / mid_shape / bot_shape | u8 | Shape enum per layer |
| rotation | f64le | Rotation angle |
| plated | bool | Through-hole plated |
| pad_mode | u8 | Stack mode |
| paste_mask_expansion | i32le | |
| solder_mask_expansion | i32le | |
| hole_size | Coord | Drill hole diameter |
| direction | f64le | |
| per_layer_sizes | [CoordPoint; 32] | Full-stack sizes |
| per_layer_shapes | [u8; 32] | Full-stack shapes |

20 unknown fields documented with hypotheses (see pad analysis section below).

### 5.6 Key Record Detail: PcbTrack (Object ID 4)

| Field | Type | Notes |
|-------|------|-------|
| start | CoordPoint | Start position |
| end | CoordPoint | End position |
| width | Coord | Track width |
| unknown | Vec<u8> | 16 trailing unknown bytes |

### 5.7 Key Record Detail: PcbVia (Object ID 3)

| Field | Type | Notes |
|-------|------|-------|
| position | CoordPoint | Center |
| diameter | Coord | Via diameter |
| hole_size | Coord | Drill hole size |
| from_layer | Layer | Start layer |
| to_layer | Layer | End layer |
| net | i16le | Net index |
| via_type | u8 | Through/blind/buried |

### 5.8 Flat Ownership Model (PCB)

Unlike schematic's OWNERINDEX tree, PCB uses flat ownership:
- Each primitive has a `component` index field (-1 = board-level)
- Components are in a separate `/Components6/Data` section
- Cross-references via indices into section arrays

---

## 6. Sidecar Streams

Supplementary data stored in separate CFB streams alongside main records.
These are format-evolution artifacts that merge into runtime objects on load.

### 6.1 SchLib Sidecar Load Order (15 steps)

1. Load base warehouse (`/{Component}/Data`)
2. Load extended warehouse (binary blobs: images, embedded objects)
3. Load additional warehouse (supplementary parameter objects)
4. **Pin sidecars** (9 streams, loaded for each component):
   - `PinFrac` -- 12 bytes binary per pin: fractional parts for location/length
   - `PinDesc` -- ASCII overflow: full DESCRIPTION for pins whose desc was
     truncated in the base record (255-char limit)
   - `PinMiscData` -- UTF-16LE parameter blocks: PINPROPAGATIONDELAY,
     PINPACKAGELENGTH, PINFUNCTION, SWAPIDPART, SWAPIDSEQUENCE, SWAPIDGROUP,
     DEFAULTVALUE
   - `PinTextData` -- 1-22 bytes variable binary per pin: hidden net name as
     length-prefixed string
   - `PinWideText` -- UTF-16LE full replacement for NAME and DESIGNATOR
     (handles Unicode pin names)
   - `PinSymbolLineWidth` -- 4 bytes binary per pin: symbol line width as i32le
   - `PinPackageLength` -- 12 bytes binary per pin: package length + frac + unknown
   - `PinPropagationDelay` -- 12 bytes binary per pin: propagation delay + frac + unknown
   - `PinFunctionData` -- 4 bytes binary per pin: formal type as i32le

### 6.2 SchDoc Sidecar Load Order (8 steps)

1. Load base warehouse (`/FileHeader`)
2. Load extended warehouse
3. Load additional warehouse (`/Additional`)
4. WideStrings sidecar
5. UniqueIDs sidecar
6. ExtendedPrimitiveInfo sidecar
7. PinFrac (for any inline components)
8. PinWideText

### 6.3 PcbDoc Sidecar Load Order

23 primary sections loaded in order, plus global sidecars:

Primary sections: Arcs6, Pads6, Vias6, Tracks6, Texts6, Fills6,
Connections6, Polygons6, Dimensions6, Components6, Regions6,
ComponentBodies6, Nets6, Classes6, Rules6, Models (from Models/Data),
Board6, ShapeBasedRegions6, ShapeBasedComponentBodies6, etc.

**Global sidecar streams**:
- `WideStrings6/Data` -- binary TLV format: `[u32 prim_index][u32 length][UTF-16LE data]`
  Replaces ASCII name fields with full Unicode text.
- `UniqueIDPrimitiveInformation` -- parameter blocks, one per primitive:
  `PRIMITIVEINDEX`, `UNIQUEID`, `PRIMITIVEKIND`, etc.
- `ExtendedPrimitiveInformation` -- parameter blocks with mask expansion mode
  overrides: `PASTEMASKEXPANSIONMODE`, `SOLDERMASKEXPANSIONMODE`, `TENTINGMODE`
- `PrimitiveGuids` -- 24-byte binary records per primitive (checksum/GUID data)

**88 total PCB stream names** documented in the sidecar deep-dive.

### 6.4 PcbLib Sidecar Streams

Per-footprint WideStrings use **parameter-block format** (not binary TLV):
```
|DESIGNATOR=U1|NAME=...|COMMENT=...|DESCRIPTION=...|
```

---

## 7. SchDoc/SchLib Loading Pipeline

Reverse-engineered from `Altium.Sch.DataModel.dll` (.NET 8).

### 7.1 Three-Warehouse Architecture

| Warehouse | Stream | Contents |
|-----------|--------|----------|
| Base | `/{Component}/Data` or `/FileHeader` | Main objects (parameter records) |
| Extended | `/{Component}/Data` (binary blocks) | Binary blobs (images, embedded objects) |
| Additional | `/{Component}/Data` or `/Additional` | Supplementary objects |

### 7.2 Class Hierarchy

Key classes from the .NET decompilation:
- `SchDataSerializerParam` -- production serializer (parameter format)
- `SchStorageFileImporter` / `SchStorageFileExporter` -- file I/O pipeline
- `TSchGraphicalObject` -- base for all graphical primitives
- `TSchComponent` -- component container
- `TSchPin` -- pin primitive
- `SchLibrary` / `SchDocument` -- top-level document classes

### 7.3 Font Table Format

Fonts stored in the FileHeader block as 1-based indexed parameters:
```
FONTIDCOUNT=3
SIZE1=10|FONTNAME1=Times New Roman|ROTATION1=0|...
SIZE2=12|FONTNAME2=Arial|ROTATION2=0|...
```

### 7.4 Binary-Code-to-TObjectId Mapping

Complete table of ~80 RECORD values to TObjectId mappings. Key entries:

| RECORD | TObjectId | Name |
|--------|-----------|------|
| 1 | eComponent | Component |
| 2 | ePin | Pin |
| 3 | eSymbol | IEEE symbol |
| 4 | eLabel | Text label |
| 6 | ePolyline | Polyline |
| 7 | ePolygon | Polygon |
| 14 | eRectangle | Rectangle |
| 17 | ePowerObject | Power port |
| 25 | eNetLabel | Net label |
| 27 | eWire | Wire |
| 31 | eSheet | Sheet |
| 34 | eDesignator | Designator |
| 41 | eParameter | Parameter |
| 44 | eImplementationList | Implementation list |
| 45 | eImplementation | Implementation |
| 46 | eMapDefinerList | Map definer list |
| 47 | eMapDefiner | Map definer |
| 48 | eImplementationChild | Implementation parameters |

---

## 8. PcbDoc/PcbLib Loading Pipeline

Reverse-engineered from .NET and Ghidra analysis of Delphi DLLs.

### 8.1 Architecture

PCB loading remains **Delphi native** (unlike SCH which is fully .NET 8).
The .NET interface definitions (`IPCB_StructuredStorage`) provide the API
contract, but actual loading is in `Advpcb.dll`.

### 8.2 Section Types

Each primitive type has its own section in the CFB container:
- `/{SectionName}/Header` -- parameter block with metadata (RECORD_COUNT, etc.)
- `/{SectionName}/Data` -- binary records

Section loading uses `TAdvPCBFileFormatVersion` to handle format evolution:
- V3: Protel 99 era
- V4: DXP era
- V5: AD early versions
- V6: Modern AD (current default)

### 8.3 File Version and Feature Flags

`TStorageFeature` flags control which sections/features are present:
- `sfNets`, `sfClasses`, `sfRules`, `sfPolygons`, `sfDimensions`,
  `sfConnections`, `sfComponents`, `sf3DModels`, `sfRegions`,
  `sfComponentBodies`, `sfBoardOutline`, etc.

### 8.4 Per-Primitive Binary Format

For each section, records follow:
```
u8  object_type_id
u32 record_length (little-endian)
[record_length bytes: common_header + type-specific fields]
```

Common header sizes vary by format version:
- V3: 14 bytes
- V4+: 19 bytes (adds unique_id field)

### 8.5 3D Model Handling

Models stored in `/Models/Data` section. Each model record contains:
- Model ID, filename, rotation, standoff height
- Stored as parameter blocks (not binary)
- Referenced by `PcbComponentBody` records via model index

---

## 9. .NET/Delphi Interop Architecture

### 9.1 Technology Stack

- **Protel era (1985-2003)**: Pure Delphi/Object Pascal
- **DXP era (2003-2020)**: Delphi + .NET Framework (COM interop)
- **AD26 (current)**: Delphi + .NET 8 R2R (Runtime Ready-to-Run)

### 9.2 Interop Model

.NET does NOT use P/Invoke. All interop uses **COM interfaces**:
- .NET defines interfaces (e.g., `IPCB_Primitive`, `ISch_Pin`)
- Delphi implements them (via COM vtables)
- Both sides can call through the interface

### 9.3 Binary Inventory

| Category | Technology | Examples |
|----------|-----------|----------|
| Schematic | .NET 8 R2R | Altium.Sch.DataModel.dll, Altium.Sch.DesignEditor.dll |
| PCB | Native Delphi | Advpcb.dll, AdvPcbDsgMgr.dll |
| Core | Delphi | X2.EXE, DXP.dll |
| Scripting | .NET | Altium.ScriptInterfaces.dll |

### 9.4 Two Loading Paths

- **Schematic**: Fully .NET 8. `SchStorageFileImporter` reads CFB, creates
  .NET objects directly.
- **PCB**: Fully Delphi native. `TPCB_V6BinaryFileReader` in Advpcb.dll reads
  CFB, creates Delphi objects. .NET wraps them via COM for the UI layer.

---

## 10. API Functions

### 10.1 Schematic API (AdvSch.dll -- Delphi)

135 `SchAPI_*` exports organized by category:

**Document/window management** (15 functions):
SchAPI_CreateNewDocument, SchAPI_OpenDocument, SchAPI_CloseDocument,
SchAPI_SaveDocument, SchAPI_GetActiveDocument, etc.

**Object creation/destruction** (8 functions):
SchAPI_CreateObject, SchAPI_DestroyObject, SchAPI_CopyObject,
SchAPI_MoveObject, etc.

**Iterators** (standard, spatial, group):
SchAPI_CreateIterator, SchAPI_CreateSpatialIterator,
SchAPI_CreateGroupIterator, SchAPI_IteratorNext, etc.

**Property getters/setters** (~80 functions):
SchAPI_GetOwnerIndex, SchAPI_SetOwnerIndex,
SchAPI_GetLocation, SchAPI_SetLocation,
SchAPI_GetPinName, SchAPI_SetPinName, etc.

**Library operations**:
SchAPI_GetLibComponentCount, SchAPI_GetLibComponent,
SchAPI_AddLibComponent, SchAPI_RemoveLibComponent, etc.

### 10.2 PCB API (Advpcb.dll -- Delphi)

~290 `PcbApi_*` exports:

**Iterator/traversal** (~20 functions):
PcbApi_CreateIterator, PcbApi_IteratorNext, PcbApi_DestroyIterator,
PcbApi_SetIteratorFilter, etc.

**Object factory** (~10 functions):
PcbApi_CreateObject, PcbApi_DestroyObject, PcbApi_CopyObject,
PcbApi_PlaceObject, etc.

**Property query -- pad-specific** (37+ params, 20+ query functions):
PcbApi_GetPadXSize, PcbApi_GetPadYSize, PcbApi_GetPadShape,
PcbApi_GetPadHoleSize, PcbApi_GetPadPlated, PcbApi_GetPadStackMode,
PcbApi_GetPadSolderMaskExpansion, PcbApi_GetPadPasteMaskExpansion, etc.

**Container/board management**:
PcbApi_GetBoardOutline, PcbApi_GetLayerCount, PcbApi_GetLayerName,
PcbApi_GetNetCount, PcbApi_GetNetName, etc.

---

## 11. .NET Data Model Interfaces

### 11.1 Schematic Object Model

Two generations of interfaces:
- **Legacy COM**: `SCHInterfaces` namespace (older, still used for Delphi interop)
- **Modern .NET**: `Altium.Sch.Interfaces.Objects` namespace

**TObjectId enumeration** (120+ values):
eFirstObjectID(0) through extended objects at 120+. Key entries:
eComponent(15), ePin(16), eWire(38), eNetLabel(37), ePowerObject(28),
eRectangle(20), eLabel(5), eDesignator(43), eParameter(55), eSheet(41).

**Interface hierarchy**:
```
ISch_BasicObject
+-- ISch_GraphicalObject (position, bounds, color)
    +-- ISch_Component (lib_ref, designator, part_count)
    +-- ISch_Pin (name, designator, electrical_type)
    +-- ISch_Wire (vertices)
    +-- ISch_NetLabel (text, font)
    +-- ISch_PowerObject (style, text)
    +-- ISch_Rectangle (corner, is_solid)
    ...
```

### 11.2 PCB Object Model

**TObjectId enumeration** (26 types): eArcObject(1) through eBoardOutlineObject(26).

**V6 Layer IDs (TV6_Layer)** -- 82-value enum:
eTopLayer(1), eMidLayer1-30(2-31), eBottomLayer(32), eTopOverlay(33),
eBottomOverlay(34), eTopPaste(35), eBottomPaste(36), eTopSolder(37),
eBottomSolder(38), eInternalPlane1-16(39-54), eDrillGuide(55),
eKeepOutLayer(56), eMechanical1-16(57-72), eMultiLayer(74), eConnectLayer(75).

**V7 Layer System**:
```
IV7_Layer
  genus: LayerGenus (signal, plane, mechanical, mask, silkscreen, etc.)
  family: LayerFamily (copper, dielectric, etc.)
  species: u32 (layer-specific index)
  flags: u32 (enabled, visible, etc.)
```

**Interface hierarchy**:
```
IPCB_Primitive (layer, net, component, flags, bounds)
+-- IPCB_Arc (center, radius, start_angle, end_angle)
+-- IPCB_Pad (x_size, y_size, shape, hole_size, stack_mode)
+-- IPCB_Via (diameter, hole_size, from_layer, to_layer)
+-- IPCB_Track (start, end, width)
+-- IPCB_Text (text, font, height, rotation)
+-- IPCB_Fill (location1, location2, rotation)
+-- IPCB_Polygon (pour_mode, vertices)
+-- IPCB_Region (vertices, kind)
+-- IPCB_Component (name, pattern, designator, rotation)
+-- IPCB_ComponentBody (model_id, standoff)
+-- IPCB_Dimension (text_height, arrows, units)
```

---

## 12. Pad Field Analysis

### 12.1 Known Binary Layout (Subrecord 5)

The PcbPad binary record contains ~110-202+ bytes. Key known offsets:

| Offset | Size | Field |
|--------|------|-------|
| 0 | 1 | layer |
| 1 | 2 | flags |
| 3 | 2 | net index |
| 7 | 2 | component index |
| 13 | 4+4 | position (x, y) |
| 21 | 4+4 | top_size (x_size, y_size) |
| 29 | 4+4 | mid_size |
| 37 | 4+4 | bot_size |
| 45 | 1+1+1 | top_shape, mid_shape, bot_shape |
| 48 | 8 | rotation (f64le) |
| 56 | 1 | plated |
| 58 | 1 | pad_mode (stack mode) |
| 73 | 4 | paste_mask_expansion |
| 77 | 4 | solder_mask_expansion |
| 81 | 4 | hole_size |

### 12.2 Unknown Field Hypotheses (20 fields)

Priority 1 (single-byte, high confidence 85-90%):
- Offset 61: `hole_type` (round=0, square=1, slot=2)
- Offset 96: `assembly_testpoint_top` (bool)
- Offset 97: `assembly_testpoint_bottom` (bool)
- Offset 57: `paste_mask_enable` (bool)
- Offset 59: `solder_mask_enable` (bool)

Priority 2 (four-byte, moderate confidence):
- Offset 85: `thermal_relief_airgap` (i32le, coordinate)
- Offset 89: `thermal_relief_conductor_width` (i32le, coordinate)
- Offset 93: `union_index` (i32le)
- Offset 99: `pad_x_offset` / `pad_y_offset` (i32le each)

---

## 13. Impedance Mismatch Analysis

### 13.1 Core Gap

"We model serialization; Altium models the design domain."

The current Rust implementation focuses on file format round-tripping. Altium's
API provides a full design domain model with iterators, layer management, design
rules, net/class management, undo, and spatial queries.

### 13.2 Implementation Status (26 PCB Object Types)

| Type | Status | Priority |
|------|--------|----------|
| Arc | Implemented | - |
| Pad | Implemented (20 unknown fields) | High |
| Via | Implemented | - |
| Track | Implemented | - |
| Text | Implemented | - |
| Fill | Implemented | - |
| Connection | Not implemented (transient) | Low |
| Polygon | Implemented | - |
| Dimension | Partial | Medium |
| Component | Implemented | - |
| Region | Partial | Medium |
| ComponentBody | Partial | Medium |
| Net | Metadata only | Medium |
| Class | Not implemented | Medium |
| Rule | Not implemented | Low |
| Embedded | Not implemented | Low |
| BoardOutline | Not implemented | Medium |

### 13.3 Priority Additions (4 Phases)

1. **Phase 1**: Sidecar merge (WideStrings, UniqueIDs, ExtendedPrimitiveInfo),
   container mutation API, typed iterators
2. **Phase 2**: Layer abstraction (V6+V7), unique ID generation, net/class
   read support
3. **Phase 3**: Design rule read support, dimension completion, region
   completion
4. **Phase 4**: Full pad field coverage, component body completion, board
   outline support

---

## 14. Cross-Domain Overlap

### 14.1 Shared Concepts

| Concept | Schematic | PCB |
|---------|-----------|-----|
| Container | CFB/OLE | CFB/OLE |
| Coordinates | Fixed-point (10k/mil) | Fixed-point (10k/mil) |
| Ownership | OWNERINDEX tree | Flat (component index) |
| Serialization | Parameter strings | Binary structs |
| Unknown preservation | UnknownFields (params) | Vec<u8> (trailing bytes) |
| Unique IDs | UNIQUEID parameter | Sidecar stream |
| Layer system | N/A | V6 byte / V7 structured |
| Color | Win32 COLORREF (i32) | Win32 COLORREF (i32) |
| Derive macros | AltiumRecord (params) | AltiumRecord (binary) |
| Sidecar streams | 9 pin sidecars + 3 global | 4+ global + per-section |

### 14.2 Key Differences

- **Record dispatch**: Schematic uses `RECORD` parameter value; PCB uses binary
  `object_id` byte.
- **Stream organization**: SchDoc = single flat stream; PcbDoc = one section per
  primitive type.
- **Loading technology**: SCH = .NET 8; PCB = Delphi native.
- **Field encoding**: SCH = pipe-delimited ASCII; PCB = packed little-endian binary.
- **Format evolution**: SCH evolved via sidecar streams (backwards-compatible
  additions); PCB evolved via format version flags and section additions.

---

## 15. File Type Summary

| Extension | Type | Container | Serialization | Key Sections |
|-----------|------|-----------|---------------|--------------|
| .SchLib | Schematic Library | CFB | Parameters | FileHeader, SectionKeys, per-component storages |
| .SchDoc | Schematic Document | CFB | Parameters | FileHeader, Additional |
| .PcbDoc | PCB Document | CFB | Binary | Board6, Components6, Primitives6, Nets6, Rules6, Classes6, 30+ sections |
| .PcbLib | PCB Library | CFB | Binary | Per-footprint storages with primitive sections |
| .PrjPcb | PCB Project | CFB | Parameters | Project settings, document list |
| .IntLib | Integrated Library | CFB | Both | Embedded SchLib + PcbLib |
