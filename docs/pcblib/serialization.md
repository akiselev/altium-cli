# PcbLib Serialization (Round-Trip Write)

How to serialize an in-memory `PcbLib` back to a byte-identical CFB file.

Sources: Delphi classes `TPCBLibraryBinaryFileV6`, `TLibComponentSection`,
`TLibrarySection`, `TPrimitivesSection` (all in `Altium.PCB.BinaryLoader.dll` via
Ghidra project `altium26`); .NET interfaces `IPCB_LibBinaryV6Storage`,
`IPCB_LibComponentSection`, `IPCB_BinarySection`, `IPCB_LibrarySection` (all in
`AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/`); constants from
`AD26-dotnet/Altium.Edp.Interfaces/xPCBTypes/Consts.cs`.

---

## 1. Architecture: The Write Pipeline

The write pipeline is the inverse of the load pipeline:

```
PcbLib (in-memory)
  → write FileHeader stream (format identification + key token)
  → write Library/ storage:
      → Library/Header + Library/Data (board defaults, layer stack)
      → Library/EmbeddedFonts
      → Library/ComponentParamsTOC/{Header,Data}
      → Library/LayerKindMapping/{Header,Data}
      → Library/Models/{Header,Data,0,1,...}
      → Library/ModelsNoEmbed/{Header,Data}
      → Library/PadViaLibrary/{Header,Data}
      → Library/Textures/{Header,Data}
  → for each footprint (in index order):
      → write /<key>/Parameters (footprint metadata)
      → write /<key>/Header (u32 record count)
      → write /<key>/Data (pattern name block + packed binary primitives)
      → write /<key>/WideStrings (if any Text primitives)
      → write /<key>/PrimitiveGuids/{Header,Data} (if GUIDs assigned)
      → write /<key>/UniqueIDPrimitiveInformation/{Header,Data} (if UIDs assigned)
      → write /<key>/ExtendedPrimitiveInformation/{Header,Data} (if extended props)
  → write SectionKeys stream (if any footprint name > 31 chars)
  → write FileVersionInfo/{Header,Data} (version history)
  → write CFB container to disk
```

### Delphi Class Hierarchy (from Ghidra RTTI)

```
TBinaryFile
  └── TPCBBinaryFile
        └── TPCBBinaryFileV6                    (CFB container handling)
              └── TPCBLibraryBinaryFileV6       (PcbLib-specific)
                    ├── GetFileIdentifier()     → "PCB 6.0 Binary Library File"
                    ├── BinaryFile_OpenWrite()  → creates CFB + FileHeader
                    ├── Export_ToFile()          → iterates sections, calls each Export
                    └── CreateSection()         → maps names to section classes

TSection (base section)
  └── TPrimitivesSection                        (binary primitive records)
        ├── TArcsSection, TPadsSection, TViasSection, TTracksSection
        ├── TTextsSection, TFillsSection
        └── [shared with PcbDoc sections]
  └── TLibrarySection                           (Library/ global data)
  └── TLibComponentSection                      (per-footprint section)
        ├── Export_ToFile()  → Data + WideStrings + sidecars
        ├── WritePrimitive() → dispatch per type
        └── PrepareToSave()  → pre-save setup
```

### COM Interface Pipeline

```csharp
// Entry point:
IPCB_LibraryLoaderSaver.SaveToFile(library, fileName)

// Steps:
1. RegisterAllSectionsForExporting()    — register Library + per-footprint sections
2. For each section: PrepareToSave()    — collect primitives, assign IndexForSave
3. For each section: Export_ToFile()     — write Header + Data streams
4. Write global streams (FileHeader, SectionKeys, FileVersionInfo)
```

---

## 2. Binary Record Format — All Primitives

Unlike SchLib (pipe-delimited text `|KEY=VALUE|`), PcbLib uses **packed binary
structs** (little-endian) for all primitives.

### Record Framing

```
[1 byte]   u8: TObjectId type byte
[4 bytes]  u32 LE: record length (lower 24 bits = payload size, upper 8 = flags)
[N bytes]  binary payload

actual_length = raw_u32 & 0x00FFFFFF
flags         = (raw_u32 >> 24) & 0xFF
```

### Multi-Subrecord Types

Some primitive types consist of multiple subrecords. The type byte appears once;
subsequent subrecords have only `[u32 len][payload]` framing.

| Object Type     | TObjectId | Subrecord Count | Notes |
|-----------------|:---------:|:---------------:|-------|
| Arc             | 1         | 1               | Single subrecord |
| **Pad**         | **2**     | **6**           | Main body + 5 extended shape/parameter subrecords |
| Via             | 3         | 1               | Single subrecord |
| Track           | 4         | 1               | Single subrecord |
| **Text**        | **5**     | **2**           | Properties + text string subrecord |
| Fill            | 6         | 1               | Single subrecord |
| Region          | 11        | 1               | Variable length (vertex array) |
| ComponentBody   | 12        | 1               | Variable length (outline + model ref) |

Example (Pad with 6 subrecords):
```
[u8 type=2]          // Pad TObjectId
[u32 len][payload]   // Subrecord 0: main pad data (~500+ bytes)
[u32 len][payload]   // Subrecord 1: extended shapes per-layer
[u32 len][payload]   // Subrecord 2: corner radius per-layer
[u32 len][payload]   // Subrecord 3: offset from hole per-layer
[u32 len][payload]   // Subrecord 4: hole/slot/thermal data
[u32 len][payload]   // Subrecord 5: AD26+ extended fields
```

### Common Header (13 bytes)

All PCB primitives share a 13-byte common header at the start of their first
subrecord:

```
Offset  Size  Type    Field
0       1     u8      layer               // PCB layer ID
1       1     u8      _pad                // padding byte (always 0)
2       2     u16     flags               // Primitive flags bitmask
4       4     i32     net_index           // Net index (-1 = no net)
8       2     u16     polygon_index       // Polygon pour index (0 = none)
10      2     u16     component_index     // Component index (0 = none)
12      1     u8      unknown             // Unknown byte
```

**In PcbLib context**: `net_index`, `polygon_index`, and `component_index` are
always 0 or -1 (footprint primitives are not yet placed on a board).

---

## 3. Per-Primitive Binary Layouts

### Arc (TObjectId = 1)

```
Offset  Size  Type    Field
0       13    -       common header
13      4     i32     center_x
17      4     i32     center_y
21      4     i32     radius
25      8     f64     start_angle         // degrees
33      8     f64     end_angle           // degrees
41      4     i32     width               // arc line width

--- AD26 trailing fields (after byte 45) ---
45      1     u8      user_routed
46      4     i32     union_index
50      1     u8      arc_kind            // 0=normal
51      4     i32     layer_enum_index
55      4     i32     keepout_restrictions
```

Observed sizes: Legacy=45 bytes, AD26=58 bytes.

### Pad (TObjectId = 2, 6 subrecords)

**Subrecord 0: Main pad data**

```
Offset  Size  Type    Field
0       13    -       common header
13      4     i32     location_x
17      4     i32     location_y
21      4     i32     size_top_x          // Top layer pad width
25      4     i32     size_top_y          // Top layer pad height
29      4     i32     size_mid_x          // Mid layer pad width
33      4     i32     size_mid_y          // Mid layer pad height
37      4     i32     size_bot_x          // Bottom layer pad width
41      4     i32     size_bot_y          // Bottom layer pad height
45      4     i32     hole_size           // Drill hole diameter
49      1     u8      shape_top           // Top layer shape (TShape)
50      1     u8      shape_mid           // Mid layer shape
51      1     u8      shape_bot           // Bottom layer shape
52      8     f64     rotation            // Rotation in degrees
60      1     bool    is_plated           // PTH vs NPTH/SMD
61      1     u8      unknown1
62      1     u8      stack_mode          // TStackMode
63      4     i32     unknown2
67      4     i32     paste_mask_expansion
71      4     i32     solder_mask_expansion
... (more fields, per-layer arrays; ~500+ bytes total)
```

**Subrecords 1-5** carry extended per-layer shapes, corner radii, offsets,
hole/slot data, and AD26+ fields.

### Via (TObjectId = 3)

```
Offset  Size  Type    Field
0       13    -       common header
13      4     i32     location_x
17      4     i32     location_y
21      4     i32     hole_size           // Drill hole diameter
25      4     i32     diameter_top        // Via pad diameter (top)
29      4     i32     diameter_mid        // Via pad diameter (mid)
33      4     i32     diameter_bot        // Via pad diameter (bottom)
37      1     u8      from_layer          // Start layer
38      1     u8      to_layer            // End layer
... (additional fields follow)
```

### Track (TObjectId = 4)

```
Offset  Size  Type    Field
0       13    -       common header
13      4     i32     start_x
17      4     i32     start_y
21      4     i32     end_x
25      4     i32     end_y
29      4     i32     width
33      2     u16     subpoly_index

--- AD26 trailing fields (after byte 35) ---
35      1     u8      user_routed
36      4     i32     union_index
40      1     u8      track_kind
41      4     i32     layer_enum_index
45      4     i32     keepout_restrictions
```

Observed sizes: Legacy=35 bytes, AD26=49 bytes.

### Text (TObjectId = 5, 2 subrecords)

**Subrecord 0: Text properties**

```
Offset  Size  Type    Field
0       13    -       common header
13      4     i32     location_x
17      4     i32     location_y
21      4     i32     height              // Text height
25      8     f64     rotation            // Degrees
33      1     bool    is_mirrored
34      4     i32     stroke_width        // For stroke font
38      1     bool    is_comment          // Whether displays Comment
39      1     bool    is_designator       // Whether displays Designator
40      1     u8      font_kind           // TTextKind (0=Stroke, 1=TrueType, 2=BarCode)
... (additional fields: font ID, justification, etc.)
```

**Subrecord 1: Text string**

```
[4 bytes] u32 LE: subrecord length
[N bytes] Win1252 text string
```

Special tokens: `.Designator`, `.Comment`, `.Layer_Name`.
Unicode text stored in WideStrings sidecar overrides this value.

### Fill (TObjectId = 6)

```
Offset  Size  Type    Field
0       13    -       common header
13      4     i32     corner1_x
17      4     i32     corner1_y
21      4     i32     corner2_x
25      4     i32     corner2_y
29      8     f64     rotation            // Degrees
```

Observed sizes: Legacy=37 bytes, AD26=50 bytes.

### Region (TObjectId = 11)

```
Offset  Size  Type    Field
0       13    -       common header
13      1     u8      region_kind         // TRegionKind
14      4     i32     unknown
... (varies by format version)
N       4     i32     vertex_count        // Number of outline vertices
N+4     -     -       vertices[]          // Array of (x:i32, y:i32), 8 bytes each
```

Variable length due to vertex array.

### ComponentBody (TObjectId = 12)

```
Offset  Size  Type    Field
0       13    -       common header
... (body outline and properties)
N       -     GUID    model_id            // References Library/Models/Data entry
... (standoff height, rotation offsets, etc.)
```

Contains 2D outline vertices, 3D model GUID reference, and positioning data.

---

## 4. Coordinate System

PCB coordinates use a unified i32 internal unit system:

- **1 mil = 10,000 internal units** (i32 LE)
- **1 mm ≈ 393,701 internal units**
- All multi-byte integers are **little-endian**
- All floating-point values (`f64`) are **IEEE 754 little-endian**
- Colors are Win32 COLORREF: `0x00BBGGRR` (BGR, not RGB)

### String-Encoded Coordinates (in parameter streams)

The Parameters stream and Library/Data use string-encoded coords with unit
suffixes:

| Format | Example | Internal value |
|--------|---------|---------------|
| `Nmil` | `21.6535mil` | 216,535 |
| `Nmm`  | `1.0mm` | 393,701 |
| `N` (bare) | `0` | 0 |

---

## 5. FileHeader Stream

The `/FileHeader` stream is a binary format identifier (not parameter text like
SchLib's FileHeader):

```
[4 bytes] u32 LE: block length (= header text length + 1)
[1 byte]  u8: header text string length (N)
[N bytes] ASCII: "PCB 6.0 Binary Library File"
[8 bytes] f64 LE: file format version (e.g., 5.01)
[4 bytes] u32 LE: key block length
[1 byte]  u8: key string length (M)
[M bytes] ASCII: key token (e.g., "RTJRBTLE")
```

**Constant** from `Consts.cs`:
```csharp
public const string kCurrentPCBLibFormat = "PCB 6.0 Library File";
// But the binary actually writes "PCB 6.0 Binary Library File"
```

The format version float 5.01 corresponds to
`TAdvPCBFileFormatVersion.eAdvPCBFormat_Library_V6 = 11`.

---

## 6. Footprint Data Stream

### Stream: `<FootprintName>/Data`

The Data stream consists of a pattern name block followed by packed binary
primitive records.

### Pattern Name Block (always first)

```
[4 bytes] u32 LE: block_length (= string_length + 1)
[1 byte]  u8: pattern name string length (N)
[N bytes] ASCII pattern name (e.g., "CAP0402")
```

Example (CAP0402):
```hex
08 00 00 00 07 43 41 50 30 34 30 32
─────────── ── ───────────────────────
block_len=8  7  "CAP0402"
```

The pattern name must match the `PATTERN` parameter in the Parameters stream
and the CFB storage name (or SectionKeys entry).

### Binary Primitive Records (after pattern name block)

Packed consecutively with no padding:

```
[u8 type]  [u32 len][payload]                     // single-subrecord types
[u8 type]  [u32 len][payload] [u32 len][payload]  // Text (2 subrecords)
[u8 type]  [u32 len][payload] × 6                 // Pad (6 subrecords)
```

### Record Ordering

Primitives are written in their insertion order within the footprint. Unlike
SchLib (which uses `SchDataObjectComparator` for child record sorting), PcbLib
footprint primitives preserve their original order.

### No End Marker

The Data stream has no explicit end marker. Reading terminates at stream EOF or
when the expected record count (from the Header stream) is reached.

---

## 7. Footprint Header Stream

### Stream: `<FootprintName>/Header`

Always exactly 4 bytes:

```
[4 bytes] u32 LE: primitive record count
```

This is the number of primitive records in the Data stream (not counting the
pattern name block). For empty footprints: `00 00 00 00`.

---

## 8. Footprint Parameters Stream

### Stream: `<FootprintName>/Parameters`

A single length-prefixed parameter block:

```
[4 bytes] u32 LE: total block length (string_length + 1)
[1 byte]  u8: parameter string length (N)
[N bytes] Win1252 pipe-delimited parameter string (NUL-terminated)
```

### Parameter Keys

| Key | Example | Required | Description |
|-----|---------|:--------:|-------------|
| `PATTERN` | `CAP0402` | Yes | Footprint name |
| `HEIGHT` | `21.6535mil` | Yes | Component height (with unit suffix) |
| `DESCRIPTION` | `Chip Capacitor, Body 1.0x0.5mm` | Yes | Description |
| `ITEMGUID` | `{6BB694B2-...}` | No | Item GUID (may be empty) |
| `REVISIONGUID` | `{9B8FF8BD-...}` | No | Revision GUID (may have trailing NUL) |

Example:
```
|PATTERN=CAP0402|HEIGHT=21.6535mil|DESCRIPTION=Chip Capacitor, Body 1.0x0.5mm, 0402, IPC Medium Density|ITEMGUID=6BB694B2-4D0E-4A20-BCC8-3F1719C76F09|REVISIONGUID=9B8FF8BD-0664-49C8-92EE-40709DC02652\0
```

For blank footprints, `HEIGHT`, `DESCRIPTION`, `ITEMGUID`, and `REVISIONGUID`
may all be empty strings.

---

## 9. WideStrings Sidecar

### Stream: `<FootprintName>/WideStrings`

**CRITICAL**: PcbLib WideStrings use a **parameter-block format** — completely
different from PcbDoc's binary TLV `WideStrings6/Data` stream. The two formats
share NO structure and require separate implementations.

### Format

```
[4 bytes] u32 LE: block length
[1 byte]  u8: string length
[N bytes] Win1252 parameter string (NUL-terminated)
```

The parameter string contains `ENCODEDTEXT{N}` entries:

```
|ENCODEDTEXT0=46,68,101,115,105,103,110,97,116,111,114|ENCODEDTEXT1=...|
```

### ENCODEDTEXT Encoding

Each `ENCODEDTEXT{N}` value is a comma-separated sequence of decimal byte values
representing a **UTF-8 encoded string**:

```
ENCODEDTEXT0=46,68,101,115,105,103,110,97,116,111,114
             │   │   │   │   │   │   │   │   │   │   │
             .   D   e   s   i   g   n   a   t   o   r
```

**Encoding** (for write):
1. Take the text string (UTF-8)
2. Convert each byte to its decimal value
3. Join with commas
4. Write as `ENCODEDTEXT{N}=<comma_values>`

**Decoding** (for read):
1. Split value on commas to get decimal byte values
2. Convert to byte array
3. Decode as UTF-8

### Index Mapping

The index `N` in `ENCODEDTEXT{N}` corresponds to the Text primitive's sequential
position among **all** primitives in the footprint (0-based, counting from the
first primitive after the pattern name block). Text primitives that need Unicode
are collected via `AddTextsForSaveList()` and assigned sequential WideString
indices via `SetState_WideStringIndexForSave()`.

### When to Write

The WideStrings stream is present when any Text primitive in the footprint has
text that requires Unicode encoding (anything beyond Windows-1252). For empty
WideStrings, the stream contains a single NUL byte (`0x00`) in the block payload.

---

## 10. PrimitiveGuids Sidecar

### Streams: `<FootprintName>/PrimitiveGuids/Header` + `.../Data`

Binary GUID table assigning persistent GUIDs to primitives for ECO/roundtrip
tracking.

### Header

```
[4 bytes] u32 LE: count of TPrimitiveGUID entries
```

### Data

```
[4 bytes] u32 LE: block length
[N bytes] packed TPrimitiveGUID records
```

### TPrimitiveGUID Structure (24 bytes, pack=1)

Source: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TPrimitiveGUID.cs`

```csharp
[StructLayout(LayoutKind.Sequential, Pack = 1)]
public struct TPrimitiveGUID {
    public int ObjectId;       // 4 bytes: TObjectId as i32
    public int IndexForSave;   // 4 bytes: primitive index within footprint
    public Guid GUID;          // 16 bytes: standard Windows GUID
}
```

### GUID Byte Order

Windows GUIDs are stored in their native mixed-endian format:
```
Bytes 0-3:   Data1 (u32 LE)
Bytes 4-5:   Data2 (u16 LE)
Bytes 6-7:   Data3 (u16 LE)
Bytes 8-15:  Data4 (8 bytes, big-endian)
```

### Write Process

1. For each primitive with a GUID assigned:
   - Record its `ObjectId` (TObjectId as i32)
   - Record its `IndexForSave` (sequential 0-based index within the footprint)
   - Record its `GUID` (16-byte Windows GUID)
2. Write the count to Header
3. Write all packed 24-byte entries to Data

---

## 11. UniqueIDPrimitiveInformation Sidecar

### Streams: `<FootprintName>/UniqueIDPrimitiveInformation/Header` + `.../Data`

Per-primitive unique ID strings for ECO synchronization identity.

### Header

```
[4 bytes] u32 LE: count of entries
```

### Data

Parameter blocks, one per primitive with a unique ID:

```
[4 bytes] u32 LE: block length
[1 byte]  u8: string length
[N bytes] Win1252 parameter string (NUL-terminated)
```

### Parameter Keys

| Key | Example | Description |
|-----|---------|-------------|
| `PRIMITIVEINDEX` | `2` | Zero-based index within footprint primitive list |
| `PRIMITIVEOBJECTID` | `Pad` | Object type name (text, NOT numeric) |
| `UNIQUEID` | `OENFVQGU` | 8-character unique ID string |

### PRIMITIVEOBJECTID Values

From `cObjectIdStrings` in `Consts.cs`:

| TObjectId | PRIMITIVEOBJECTID string |
|:---------:|:------------------------:|
| 1         | `Arc` |
| 2         | `Pad` |
| 3         | `Via` |
| 4         | `Track` |
| 5         | `Text` |
| 6         | `Fill` |
| 11        | `PolyRegion` |
| 12        | `ComponentBody` |

**Note**: Region (TObjectId=11) maps to `"PolyRegion"`, not `"Region"`.

### Write Conditions

Not every primitive gets a unique ID. Pads almost always have unique IDs; other
types vary. Simple primitives (tracks, fills) may be skipped.

---

## 12. ExtendedPrimitiveInformation Sidecar

### Streams: `<FootprintName>/ExtendedPrimitiveInformation/Header` + `.../Data`

Same parameter-block format as UniqueIDPrimitiveInformation. Provides per-primitive
overrides for mask expansion and other properties added in later format versions.

### Parameter Keys

| Key | Example | Description |
|-----|---------|-------------|
| `PRIMITIVEINDEX` | `19` | Zero-based primitive index |
| `PRIMITIVEOBJECTID` | `PolyRegion` | Object type name |
| `TYPE` | `Mask` | Extended info type |
| `SOLDERMASKEXPANSIONMODE` | `Manual` | Solder mask expansion mode |
| `SOLDERMASKEXPANSION_MANUAL` | `1.9685mil` | Manual expansion value |
| `PASTEMASKEXPANSIONMODE` | `None` | Paste mask expansion mode |

This stream is **rare** — only 1 footprint in the LimeMicro library has it.

---

## 13. Library/ Storage

The `/Library/` storage contains library-wide global data.

### Library/Header

```
[4 bytes] u32 LE: count (typically 1)
```

### Library/Data

Pipe-delimited parameter blocks containing library-level board defaults and
layer stack configuration. The first record (no `RECORD` key) contains:

| Key | Example | Description |
|-----|---------|-------------|
| `FILENAME` | `C:\...\MyLib.PcbLib` | Original file path |
| `KIND` | `Protel_Advanced_PCB_Library` | Library type |
| `VERSION` | `3.00` | Library format version string |
| `DATE` | `2019-05-28` | Last modified date |
| `TIME` | `16:44:26` | Last modified time |
| `V9_MASTERSTACK_STYLE` | `0` | Master layer stack style |
| `V9_MASTERSTACK_ID` | `{GUID}` | Master stack GUID |
| `V9_MASTERSTACK_NAME` | `Master layer stack` | Stack name |
| `V9_STACK_LAYER{N}_*` | various | Per-layer properties |

Subsequent records have `RECORD=Board` for continuation blocks.

**For read-only parsing**: can be skipped/read opaquely. **For round-trip write**:
must be faithfully preserved.

### Library/ComponentParamsTOC/{Header,Data}

Table of contents for all footprint parameters — allows Altium to display
footprint metadata without loading individual footprints.

**Header**: `[4 bytes] u32 LE: count`

**Data**: Parameter blocks with:

| Key | Example | Description |
|-----|---------|-------------|
| `Name` | `CAP0402` | Footprint name |
| `Pad Count` | `3` | Number of pads |
| `Height` | `21.6535` | Component height (numeric, no unit suffix) |
| `Description` | `Chip Capacitor...` | Description |

### Library/Models/{Header,Data,0,1,...}

Shared 3D model pool. Multiple footprints can reference the same model by index.

**Header**: `[4 bytes] u32 LE: model count`

**Data**: Parameter blocks per model:

| Key | Example | Description |
|-----|---------|-------------|
| `EMBED` | `TRUE` | Whether model data is embedded |
| `MODELSOURCE` | `Undefined` | Model source type |
| `ID` | `{35957C61-...}` | Model GUID |
| `ROTX` / `ROTY` / `ROTZ` | `0.000` / `270.000` | Rotation offsets |
| `DZ` | `0` | Z offset |
| `CHECKSUM` | `984310846` | Model data checksum |
| `NAME` | `SOP65P640X110-24N.STEP` | Model filename |

**Numbered streams (0, 1, ...)**: zlib-compressed STEP model data (`78 9C` magic).

### Other Library/ Sub-storages

| Sub-storage | Content |
|-------------|---------|
| `EmbeddedFonts` | Embedded font data (often empty) |
| `LayerKindMapping/{H,D}` | Mechanical layer kind mapping |
| `ModelsNoEmbed/{H,D}` | External model references |
| `PadViaLibrary/{H,D}` | Pad/via template library |
| `Textures/{H,D}` | Texture images (typically empty) |

---

## 14. SectionKeys Stream

### Stream: `/SectionKeys` (optional, at CFB root)

Written only when any footprint name exceeds 31 characters.

### Format

```
[4 bytes] u32 LE: entry count

For each entry:
  [4 bytes] u32 LE: full name block length
  [1 byte]  u8: full name string length (N)
  [N bytes] ASCII full footprint display name

  [4 bytes] u32 LE: truncated key block length
  [1 byte]  u8: truncated key string length (M)
  [M bytes] ASCII truncated CFB storage key (max 31 chars)
```

### Key Generation

1. If name <= 31 characters: CFB storage key = name (no SectionKeys entry needed)
2. If name > 31 characters: truncate to 31 characters
3. Store mapping in SectionKeys

This format is identical to SchLib's SectionKeys. The same parser handles both.

---

## 15. FileVersionInfo

### Streams: `/FileVersionInfo/Header` + `/FileVersionInfo/Data`

File version history. Only present in newly-created or recently-modified libraries.

**Header**: `[4 bytes] u32 LE: count`

**Data**: Version history entries.

---

## 16. TStorageFeature Flags

Source: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TStorageFeature.cs`

These flags track which capabilities were used when saving the file:

| Value | Flag | Description |
|:-----:|------|-------------|
| 0 | `eHasImpedanceProfileCount` | Impedance profiles |
| 1 | `eHasPrintedElectronicLayers` | Printed electronics |
| 2 | `eHasMicroVias` | Micro via support |
| 3 | `eHasCustomThermalReliefsAtWriteStage` | Custom thermal reliefs |
| 4 | `eHasSystemParametersAtWriteStage` | System parameters |
| 5 | `eHasShapeBasedRegions` | Shape-based region format |
| 6 | `eHasShapeBasedCompBodies` | Shape-based component bodies |
| 7 | `eHasRF20IsUsedAtWriteStage` | RF 2.0 features |
| 8 | `eHasIPC4761ViaTypesAtWriteStage` | IPC-4761 via types |
| 9 | `eHasCustomPadShapesAtWriteStage` | Custom pad shapes |
| 10 | `eHasRotatedAnyAngleEmbeddedBoardArrayAtWriteStage` | Rotated embedded boards |
| 11 | `eHasFootprintParametersAtWriteStage` | Footprint parameters |
| 12 | `eHasCustomReliefInfosAtWriteStage` | Custom relief info |
| 13 | `eHasClearanceByLayerRuleAtWriteStage` | Per-layer clearance |
| 14 | `eHasMatrixRuleAtWriteStage` | Matrix-style rules |
| 15 | `eHasTHPadPasteInfosAtWriteStage` | TH pad paste info |
| 16 | `eHasCustomMaskInfosAtWriteStage` | Custom mask data |
| 17 | `eHasPolygonsWithNeckWidthFromRule` | Polygon neck width |
| 18 | `eHasNeckDownRuleAtWriteStage` | Neck-down rules |
| 19 | `eHasSingleLayerModeAtWriteStage` | Single-layer mode |
| 20 | `eHasCustomPadShapesDonutAtWriteStage` | Donut pad shapes |
| 21 | `eHasWirebondAtWriteStage` | Wire bonding |
| 22 | `eHasDiffpairPhaseMatching` | Diff-pair phase matching |
| 23 | `eHasExtendedGroupIndicesAreUsed` | Extended group refs |
| 24 | `eHasIncreasedSignalLayers` | > 32 signal layers |
| 25 | `eHasZAxisClearanceRuleAtWriteStage` | Z-axis clearance |

These flags affect how primitive records are interpreted and written. Key ones
for PcbLib: `eHasShapeBasedRegions` (5), `eHasShapeBasedCompBodies` (6),
`eHasCustomPadShapesAtWriteStage` (9), `eHasFootprintParametersAtWriteStage` (11).

---

## 17. CFB Container Writing

### Required Structure

```
Root
├── FileHeader                          (binary: format identification)
├── SectionKeys                         (optional: name mapping)
├── FileVersionInfo/
│   ├── Header                          (u32 count)
│   └── Data                            (version history)
├── Library/
│   ├── Header                          (u32 count)
│   ├── Data                            (board defaults + layer stack)
│   ├── EmbeddedFonts                   (font data)
│   ├── ComponentParamsTOC/
│   │   ├── Header
│   │   └── Data
│   ├── LayerKindMapping/
│   │   ├── Header
│   │   └── Data
│   ├── Models/
│   │   ├── Header
│   │   ├── Data
│   │   ├── 0, 1, ...                  (zlib-compressed STEP models)
│   ├── ModelsNoEmbed/
│   │   ├── Header
│   │   └── Data
│   ├── PadViaLibrary/
│   │   ├── Header
│   │   └── Data
│   └── Textures/
│       ├── Header
│       └── Data
├── <FootprintName>/                    (one per footprint)
│   ├── Parameters
│   ├── Header
│   ├── Data
│   ├── WideStrings                     (optional)
│   ├── PrimitiveGuids/
│   │   ├── Header
│   │   └── Data
│   ├── UniqueIDPrimitiveInformation/
│   │   ├── Header
│   │   └── Data
│   └── ExtendedPrimitiveInformation/   (rare)
│       ├── Header
│       └── Data
└── <AnotherFootprint>/
    └── ...
```

### CFB Metadata

- CFB Version: V3 (sector size 512 bytes)
- Stream/storage names are case-sensitive in CFB
- System storages to exclude when enumerating footprints: `FileVersionInfo`, `Library`
- `SectionKeys` and `FileHeader` are root-level streams (not storages)

---

## 18. Version-Dependent Record Sizes

Many primitive types have version-dependent trailing fields. The record length
field tells exactly how many bytes to read for each subrecord.

### Round-Trip Strategy

For fields we don't fully parse:

1. Read the known fields from the beginning of the record
2. If the record is longer than expected, store remaining bytes as `trailing_bytes`
3. When writing, append the stored `trailing_bytes` after known fields

This ensures round-trip fidelity for fields we don't yet understand.

### Known Size Variants

| Primitive | Legacy Size | AD26 Size | Extra Fields |
|-----------|:----------:|:---------:|-------------|
| Arc | 45 | 58 | user_routed, union_index, arc_kind, layer_enum_index, keepout_restrictions |
| Track | 35 | 49 | user_routed, union_index, track_kind, layer_enum_index, keepout_restrictions |
| Fill | 37 | 50 | Similar AD26 trailing fields |
| Pad | ~500+ | ~500+ | Highly variable per format version |

---

## 19. Key Differences from SchLib Serialization

| Aspect | SchLib | PcbLib |
|--------|--------|--------|
| Record format | Pipe-delimited text (`\|KEY=VALUE\|`) | Binary structs (LE packed) |
| Record dispatch | `RECORD=N` parameter | `u8` TObjectId byte |
| Record framing | `flags(8b) \| size(24b)` | `u8 type + u32 length + payload` |
| Pin/Pad handling | Binary pins + text records | All binary |
| Sparse saving | Two-tier Export system per field | Binary fields always written |
| Parameter ordering | Explicit per-record order in FileFormatV5.cs | N/A (binary layout) |
| Sidecar streams | 9 pin sidecars + Storage (images) | WideStrings + GUIDs + UniqueID + ExtendedPrimitiveInfo |
| Coordinate system | DXP units (i16 + frac sidecar) | i32 internal units directly |
| Library metadata | FileHeader (font table + component index) | Library/ storage (board defaults, models) |
| WideStrings | N/A (inline in text records) | Parameter-block ENCODEDTEXT format |
| 3D models | N/A | Library/Models/ (STEP files) |
| FileHeader | Full library metadata as parameters | Binary format identifier only |

---

## 20. Byte-Perfect Validation Strategy

### Round-Trip Test

```
1. Read original file → PcbLib
2. Serialize PcbLib → new CFB file
3. Compare stream contents (not CFB sector allocation)
4. On mismatch: report stream, offset, expected byte, actual byte
```

### Incremental Comparison

Compare at each layer individually:

1. **CFB structure**: Compare stream names, storage hierarchy
2. **Stream contents**: Compare raw bytes of each stream
3. **Pattern name blocks**: Compare footprint name encoding
4. **Binary records**: Compare each primitive record byte-by-byte
5. **Sidecar streams**: Compare WideStrings, PrimitiveGuids, UniqueID entries

### Known Sources of Non-Determinism

- **CFB sector allocation**: The `cfb` crate may allocate sectors differently.
  Compare at the **stream content** level instead of byte-identical CFB files.
- **Floating-point formatting**: Parameter string coords (e.g., `HEIGHT=21.6535mil`)
  must match Altium's exact formatting.
- **Record trailing bytes**: Unrecognized trailing bytes must be preserved exactly.

---

## 21. Implementation Checklist

### Layer 1: Binary Primitive Writers (one per type)

- [ ] Common header (13 bytes): `write_common_header(prim) → Vec<u8>`
- [ ] `PcbArc::to_bytes() → Vec<u8>` — single subrecord
- [ ] `PcbPad::to_bytes() → Vec<u8>` — 6 subrecords
- [ ] `PcbVia::to_bytes() → Vec<u8>` — single subrecord
- [ ] `PcbTrack::to_bytes() → Vec<u8>` — single subrecord
- [ ] `PcbText::to_bytes() → Vec<u8>` — 2 subrecords (properties + string)
- [ ] `PcbFill::to_bytes() → Vec<u8>` — single subrecord
- [ ] `PcbRegion::to_bytes() → Vec<u8>` — variable length (vertex array)
- [ ] `PcbComponentBody::to_bytes() → Vec<u8>` — variable length

### Layer 2: Stream Writers

- [ ] `write_pattern_name_block(name) → Vec<u8>`
- [ ] `write_data_stream(footprint) → Vec<u8>` — pattern name + packed primitives
- [ ] `write_header_stream(count) → Vec<u8>` — u32 LE count
- [ ] `write_parameters_stream(params) → Vec<u8>` — length-prefixed param block
- [ ] `write_wide_strings(texts) → Option<Vec<u8>>` — ENCODEDTEXT parameter blocks
- [ ] `write_primitive_guids(prims) → Option<Vec<u8>>` — 24-byte TPrimitiveGUID entries
- [ ] `write_unique_id_info(prims) → Option<Vec<u8>>` — parameter blocks
- [ ] `write_extended_prim_info(prims) → Option<Vec<u8>>` — parameter blocks (rare)

### Layer 3: Library-Level Writers

- [ ] `write_file_header() → Vec<u8>` — binary format identifier + key token
- [ ] `write_library_data() → Vec<u8>` — board defaults, layer stack params
- [ ] `write_component_params_toc(footprints) → Vec<u8>` — footprint summary index
- [ ] `write_models(models) → (Vec<u8>, Vec<Vec<u8>>)` — metadata + model blobs
- [ ] `write_section_keys(mapping) → Option<Vec<u8>>` — if long names exist
- [ ] `write_embedded_fonts() → Vec<u8>` — font data (may be empty)
- [ ] `write_layer_kind_mapping() → Vec<u8>` — layer kind map
- [ ] `write_pad_via_library() → Vec<u8>` — pad/via templates

### Layer 4: CFB Assembly

- [ ] `PcbLib::save_to_file(path) → Result<()>` — create CFB, write all streams
- [ ] `PcbLib::save_as(path) → Result<()>` — public API

### Layer 5: Validation

- [ ] `validate_round_trip(original, output) → Result<Vec<Mismatch>>` — stream comparison
- [ ] CLI command: `altium pcblib validate --original file.PcbLib --output copy.PcbLib`

---

## 22. Source References

### .NET Decompiled Sources (AD26-dotnet/)

| File | Purpose |
|------|---------|
| `Altium.Edp.Interfaces/RT_PCB/IPCB_LibBinaryStorage.cs` | PcbLib storage interface |
| `Altium.Edp.Interfaces/RT_PCB/IPCB_LibBinaryV6Storage.cs` | V6 PcbLib storage |
| `Altium.Edp.Interfaces/RT_PCB/IPCB_BinarySection.cs` | Section base interface |
| `Altium.Edp.Interfaces/RT_PCB/IPCB_LibComponentSection.cs` | Per-footprint section |
| `Altium.Edp.Interfaces/RT_PCB/IPCB_LibrarySection.cs` | Library-global section |
| `Altium.Edp.Interfaces/RT_PCB/IPCB_LibraryLoaderSaver.cs` | Save/Load entry point |
| `Altium.Edp.Interfaces/RT_PCB/IPCB_StructuredStorage.cs` | CFB document access |
| `Altium.Edp.Interfaces/RT_PCB/TPrimitiveGUID.cs` | 24-byte GUID record struct |
| `Altium.Edp.Interfaces/RT_PCB/TPrimitiveKey.cs` | 8-byte primitive key struct |
| `Altium.Edp.Interfaces/RT_PCB/TStorageFeature.cs` | Feature flags enum |
| `Altium.Edp.Interfaces/RT_PCB/TAdvPCBFileFormatVersion.cs` | Format version enum |
| `Altium.Edp.Interfaces/RT_PCB/TObjectId.cs` | Object type enum |
| `Altium.Edp.Interfaces/xPCBTypes/Consts.cs` | Constants (format strings, TObjectId→string map) |
| `Altium.Edp.Interfaces/PCBInterfaces/IPCB_Primitive_SaveLoadParameters.cs` | Primitive save params |
| `Altium.Edp.Interfaces/PCBInterfaces/IPCB_Text_SaveLoadParameters.cs` | Text WideString index |
| `Altium.Edp.Interfaces/PCBInterfaces/IPCB_LibComponent_SaveLoadParameters.cs` | Component save params |

### Delphi Binary (via Ghidra, project altium26)

| Binary | Class/Function | Address | Purpose |
|--------|---------------|---------|---------|
| `Altium.PCB.BinaryLoader.dll` | `TPCBLibraryBinaryFileV6.GetFileIdentifier` | `01b58d40` | Returns "PCB 6.0 Binary Library File" |
| `Altium.PCB.BinaryLoader.dll` | `TPCBLibraryBinaryFileV6.BinaryFile_OpenWrite` | `01b59120` | Creates CFB + FileHeader |
| `Altium.PCB.BinaryLoader.dll` | `TPCBLibraryBinaryFileV6.CreateSection` | `01b58b20` | Maps section names to classes |
| `Altium.PCB.BinaryLoader.dll` | `TLibComponentSection.Export_ToFile` | `01b4e670` | Writes Data + sidecars |
| `Altium.PCB.BinaryLoader.dll` | `TLibComponentSection.WritePrimitive` | `01b4f520` | Dispatch per primitive type |
| `Altium.PCB.BinaryLoader.dll` | `TLibComponentSection.DataWrite` | `01b4f660` | Write primitives to Data stream |
| `Altium.PCB.BinaryLoader.dll` | `TLibComponentSection.PrepareToSave` | `01b51ef0` | Pre-save preparation |
| `Altium.PCB.BinaryLoader.dll` | `TPrimitivesSection.Export_ToFile` | `018f7a00` | Generic primitive section export loop |
| `Altium.PCB.BinaryLoader.dll` | `TBinaryFile.Export_ToFile` | `01916d70` | Iterates all sections |
| `Advpcb.dll` | `PcbApi_LoadComponentFromLibrary` | `03d235e0` | Load footprint from PcbLib |
| `Advpcb.dll` | `PcbApi_CreateLibReader` | `03d58bb0` | Create library reader |

### Existing Codebase (Read Path — Reference for Inverse)

| File | Purpose |
|------|---------|
| `crates/altium-format/src/documents/pcblib.rs` | PcbLib loading pipeline |
| `crates/altium-format/src/documents/pcblib_streams.rs` | PcbLib stream codecs |
| `crates/altium-format/src/documents/pcbdoc.rs` | PcbDoc (shared primitives) |
| `crates/altium-format/src/documents/pcbdoc_streams.rs` | PcbDoc stream codecs |

### PcbLib Documentation (docs/pcblib/)

| File | Purpose |
|------|---------|
| `README.md` | Overview and navigation |
| `cfb-structure.md` | CFB storage layout |
| `fileheader.md` | FileHeader binary format |
| `loading-pipeline.md` | Complete load pipeline |
| `footprint-data-stream.md` | Data stream layout |
| `parameters-stream.md` | Parameters stream format |
| `sidecar-streams.md` | WideStrings, GUIDs, UniqueID, ExtendedPrimitiveInfo |
| `binary-primitives.md` | Per-primitive binary layouts |
| `sectionkeys.md` | SectionKeys format |
| `library-storage.md` | Library/ global data |
| `shared-with-pcbdoc.md` | Shared vs different with PcbDoc |
| `coordinate-system.md` | Internal units, colors |
| `enumerations.md` | All PCB enumerations |
