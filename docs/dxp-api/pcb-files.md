# PcbDoc / PcbLib Binary File Format Guide

Reverse-engineered from Altium Designer 26 via decompiled .NET code
(`AD26-dotnet/`) and Ghidra analysis of Delphi DLLs (`altium26` project).

---

## 1. Architecture Overview

Both PcbDoc (board design) and PcbLib (footprint library) files are **OLE
Compound Binary Files** (CFB/IStorage). The Delphi DLL `Advpcb.dll` handles
all PCB binary I/O through a structured storage system built on Microsoft's
COM Structured Storage API.

### Key .NET Interfaces

The .NET side exposes the storage hierarchy through COM interop:

| Interface | File | Purpose |
|-----------|------|---------|
| `IPCB_StructuredStorage` | `RT_PCB/IPCB_StructuredStorage.cs` | Base CFB document access |
| `IPCB_LibBinaryStorage` | `RT_PCB/IPCB_LibBinaryStorage.cs` | PcbLib-specific storage |
| `IPCB_LibBinaryV6Storage` | `RT_PCB/IPCB_LibBinaryV6Storage.cs` | V6 format PcbLib storage |
| `IPCB_BinarySection` | `RT_PCB/IPCB_BinarySection.cs` | One section (stream pair) |
| `IPCB_BoardBinarySection` | `RT_PCB/IPCB_BoardBinarySection.cs` | Board-level section |
| `IPCB_LibrarySection` | `RT_PCB/IPCB_LibrarySection.cs` | Library-global section |
| `IPCB_LibComponentSection` | `RT_PCB/IPCB_LibComponentSection.cs` | Per-footprint section |

### Inheritance Hierarchy

```
IPCB_StructuredStorage
  +-- IPCB_LibBinaryStorage          (PcbLib base)
       +-- IPCB_LibBinaryV6Storage   (PcbLib V6 format)

IPCB_BinarySection
  +-- IPCB_BoardBinarySection        (Board-level sections)
  +-- IPCB_RequiredBinarySection     (Sections required to exist)
  +-- IPCB_PolygonsBinarySection     (Polygons section)
  +-- IPCB_DimensionsSection         (Dimensions section)
  +-- IPCB_ModelsSection             (3D models)
  |    +-- IPCB_ModelsNoEmbedSection (Models without embedded data)
  +-- IPCB_TextureSection            (Textures)
  +-- IPCB_LayerKindMappingSection   (Layer kind mapping)
  +-- IPCB_ViolationSection          (DRC violations)
  +-- IPCB_BoardRegionsSection       (Board region data)
  +-- IPCB_WirebondTemplateSection   (Wirebond templates)
  +-- IPCB_LibrarySection            (Library-global section)
  +-- IPCB_LibComponentSection       (Per-footprint section)
```

Source: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/`

---

## 2. Object Type System (TObjectId)

Every PCB primitive has a type byte (`TObjectId`). Two variants exist in the
.NET source:

### Pcbtypes.TObjectId (canonical, byte-based)

Source: `AD26-dotnet/Altium.Edp.Interfaces/Pcbtypes/TObjectId.cs`

| Value | Name | Description |
|-------|------|-------------|
| 0 | `eIgnoreObject` | Null/sentinel |
| 1 | `eArcObject` | Arc primitive |
| 2 | `ePadObject` | Pad primitive |
| 3 | `eViaObject` | Via primitive |
| 4 | `eTrackObject` | Track (line) primitive |
| 5 | `eTextObject` | Text string primitive |
| 6 | `eFillObject` | Solid fill primitive |
| 7 | `eFromToObject` | FromTo (ratsnest endpoint) |
| 8 | `eNetObject` | Net grouping object |
| 9 | `eComponentObject` | Component (footprint instance) |
| 10 | `ePolygonObject` | Copper pour polygon |
| 11 | `eRegionObject` | Region (copper, cutout, cavity) |
| 12 | `eComponentBodyObject` | 3D body attached to component |
| 13 | `eDimensionObject` | Dimension annotation |
| 14 | `eCoordinateObject` | Coordinate annotation |
| 15 | `eClassObject` | Object class definition |
| 16 | `eRuleObject` | Design rule definition |
| 17 | `eManualFromToObject` | Manual FromTo definition |
| 18 | `eDifferentialPairObject` | Differential pair definition |
| 19 | `eViolationObject` | DRC violation marker |
| 20 | `eEmbeddedObject` | Embedded object (generic) |
| 21 | `eEmbeddedBoardObject` | Embedded board panel |
| 22 | `eSplitPlaneObject` | Split plane region |
| 23 | `eTraceObject` | Trace (routed path group) |
| 24 | `eSpareViaObject` | Spare via |
| 25 | `eBoardObject` | Board document root |
| 26 | `eBoardOutlineObject` | Board outline shape |

### RT_PCB.TObjectId (runtime, byte-based)

Source: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TObjectId.cs`

Identical values but `eFromToObject` at index 7 is named `eConnectionObject` and
`eManualFromToObject` at 17 is `eFromToObject`. The RT_PCB namespace is used by
the runtime code; Pcbtypes is the serialization-canonical one.

---

## 3. CFB Document Structure

### 3.1 PcbDoc Structure

A PcbDoc is a CFB document with the following structure:

```
/
+-- FileHeader              (stream: format identification)
+-- FileHeaderSix            (stream: optional, V6 extended header)
+-- Board6/                  (storage)
|   +-- Header               (stream: u32 record count)
|   +-- Data                 (stream: parameter blocks)
+-- Arcs6/
|   +-- Header               (stream: u32 record count)
|   +-- Data                 (stream: binary primitive records)
+-- Pads6/
|   +-- Header
|   +-- Data
+-- Vias6/
|   +-- Header
|   +-- Data
+-- Tracks6/
|   +-- Header
|   +-- Data
+-- Texts6/
|   +-- Header
|   +-- Data
+-- Fills6/
|   +-- Header
|   +-- Data
+-- Connections6/
|   +-- Header
|   +-- Data
+-- Nets6/
|   +-- Header
|   +-- Data
+-- Components6/
|   +-- Header
|   +-- Data
+-- Polygons6/
|   +-- Header
|   +-- Data
+-- Regions6/
|   +-- Header
|   +-- Data
+-- ComponentBodies6/
|   +-- Header
|   +-- Data
+-- Dimensions6/
|   +-- Header
|   +-- Data
+-- Coordinates6/
|   +-- Header
|   +-- Data
+-- Classes6/
|   +-- Header
|   +-- Data
+-- Rules6/
|   +-- Header
|   +-- Data
+-- DifferentialPairs6/
|   +-- Header
|   +-- Data
+-- FromTos6/
|   +-- Header
|   +-- Data
+-- EmbeddedBoards6/
|   +-- Header
|   +-- Data
+-- Embeddeds6/
|   +-- Header
|   +-- Data
+-- Models/
|   +-- Header
|   +-- Data
|   +-- 0                    (stream: model binary blob)
|   +-- 1                    (stream: model binary blob)
|   +-- ...
+-- WideStrings6/
|   +-- Header
|   +-- Data
+-- UniqueIDPrimitiveInformation/
|   +-- Header
|   +-- Data
+-- ExtendedPrimitiveInformation/
|   +-- Header
|   +-- Data
+-- PrimitiveGuids/
|   +-- Header
|   +-- Data
+-- FileVersionInfo/
|   +-- Header
|   +-- Data
+-- LayerKindMapping/
|   +-- Header
|   +-- Data
+-- EmbeddedFonts6/
|   +-- Header
|   +-- Data
+-- Textures/
|   +-- Header
|   +-- Data
+-- ModelsNoEmbed/
|   +-- Header
|   +-- Data
+-- PadViaLibrary/
|   +-- Header
|   +-- Data
+-- PadViaLibraryCache/
|   +-- Header
|   +-- Data
+-- PadViaLibraryLinks/
|   +-- Header
|   +-- Data
+-- PinPairsSection/
|   +-- Header
|   +-- Data
+-- SignalClasses/
|   +-- Header
|   +-- Data
+-- SmartUnions/
|   +-- Header
|   +-- Data
+-- UnionRelations/
|   +-- Header
|   +-- Data
+-- UnionNames/
|   +-- Header
|   +-- Data
+-- WaivedViolations/
|   +-- Header
|   +-- Data
+-- PrimitiveParameters/
|   +-- Header
|   +-- Data
+-- ConstraintManager/
|   +-- Header
|   +-- Data
+-- ShapeBasedRegions6/
|   +-- Header
|   +-- Data
+-- ShapeBasedComponentBodies6/
|   +-- Header
|   +-- Data
+-- SplitPlaneRegions6/
|   +-- Header
|   +-- Data
+-- BoardRegions/
|   +-- Header
|   +-- Data
+-- Texts/
|   +-- Header
|   +-- Data
+-- Advanced Placer Options6/
|   +-- Header
|   +-- Data
+-- Advanced Router Options6/
|   +-- Header
|   +-- Data
+-- Design Rule Checker Options6/
|   +-- Header
|   +-- Data
+-- Pin Swap Options6/
|   +-- Header
|   +-- Data
+-- NewRules6/
|   +-- Header
|   +-- Data
```

### 3.2 PcbLib Structure

A PcbLib has global storages plus one storage per footprint:

```
/
+-- FileHeader               (stream: library format identification)
+-- SectionKeys              (stream: optional, footprint name-to-key mapping)
+-- FileVersionInfo/
|   +-- Header
|   +-- Data
+-- Library/
|   +-- Header               (stream: u32 count)
|   +-- Data                 (stream: library-wide parameters)
|   +-- EmbeddedFonts        (stream: font data)
|   +-- ComponentParamsTOC/
|   |   +-- Header
|   |   +-- Data
|   +-- LayerKindMapping/
|   |   +-- Header
|   |   +-- Data
|   +-- Models/
|   |   +-- Header
|   |   +-- Data
|   |   +-- 0                (stream: model binary blob)
|   |   +-- 1                (stream: model binary blob)
|   |   +-- ...
|   +-- ModelsNoEmbed/
|   |   +-- Header
|   |   +-- Data
|   +-- PadViaLibrary/
|   |   +-- Header
|   |   +-- Data
|   +-- Textures/
|       +-- Header
|       +-- Data
+-- <FootprintName>/          (one storage per footprint)
|   +-- Parameters            (stream: pipe-delimited metadata)
|   +-- Header                (stream: count + version info)
|   +-- Data                  (stream: pattern name block + binary primitives)
|   +-- WideStrings           (stream: optional, parameter-block format)
|   +-- PrimitiveGuids/
|   |   +-- Header
|   |   +-- Data
|   +-- UniqueIDPrimitiveInformation/
|   |   +-- Header
|   |   +-- Data
|   +-- ExtendedPrimitiveInformation/
|       +-- Header
|       +-- Data
+-- <AnotherFootprint>/
    +-- ...
```

---

## 4. Complete Section Registry

Each section in a PcbDoc falls into one of five categories based on its data format.

### 4.1 Primitive Sections (Binary Records)

These contain packed binary primitive records. Format: `u8 type_byte + u32 length + binary_payload`.

| Section Name | Object ID | Primitive Type |
|-------------|-----------|----------------|
| `Arcs6` | 1 | Arc |
| `Pads6` | 2 | Pad |
| `Vias6` | 3 | Via |
| `Tracks6` | 4 | Track |
| `Texts6` | 5 | Text |
| `Fills6` | 6 | Fill |
| `Connections6` | 7 | Connection/FromTo |
| `Regions6` | 11 | Region |
| `ShapeBasedRegions6` | 11 | Region (shape-based variant) |
| `SplitPlaneRegions6` | 11 | Region (split plane variant) |
| `ComponentBodies6` | 12 | ComponentBody |
| `ShapeBasedComponentBodies6` | 12 | ComponentBody (shape-based variant) |
| `BoardRegions` | 11 | Region (legacy) |
| `Texts` | 5 | Text (legacy) |

### 4.2 Parameter Sections (Key-Value Blocks)

These contain `u32 length + NUL-terminated parameter string` blocks.
Parameters are pipe-delimited: `|KEY1=VALUE1|KEY2=VALUE2|`.

| Section Name | Content |
|-------------|---------|
| `Board6` | Board-level settings and metadata |
| `Nets6` | Net definitions |
| `Components6` | Component instances |
| `Polygons6` | Polygon pour definitions |
| `Classes6` | Object class definitions |
| `DifferentialPairs6` | Differential pair definitions |
| `FromTos6` | FromTo/ratsnest definitions |
| `EmbeddedBoards6` | Embedded board array definitions |
| `Embeddeds6` | Embedded objects |
| `UniqueIDPrimitiveInformation` | Per-primitive unique IDs (sidecar) |
| `ExtendedPrimitiveInformation` | Per-primitive extended properties (sidecar) |
| `PadViaLibrary` | Pad/via template library |
| `PadViaLibraryCache` | Pad/via template cache |
| `PadViaLibraryLinks` | Pad/via template links |
| `PinPairsSection` | Pin pair definitions |
| `SignalClasses` | Signal class definitions |
| `SmartUnions` | Smart union definitions |
| `UnionRelations` | Union relation mappings |
| `WaivedViolations` | Waived DRC violations |
| `PrimitiveParameters` | Primitive parameter overrides |
| `Advanced Placer Options6` | Auto-placer settings |
| `Advanced Router Options6` | Auto-router settings |
| `Design Rule Checker Options6` | DRC settings |
| `Pin Swap Options6` | Pin-swap settings |

### 4.3 Prefixed Parameter Sections

These contain `u16 prefix + u32 length + NUL-terminated parameter string` blocks.
The 2-byte prefix precedes each parameter block.

| Section Name | Content |
|-------------|---------|
| `Rules6` | Design rules |
| `NewRules6` | Extended design rules |
| `Dimensions6` | Dimension annotations |
| `Coordinates6` | Coordinate annotations |

### 4.4 Raw Binary Sections

These contain a `Header` stream (u32 count) and a `Data` stream (raw binary
payload). The format of the data is section-specific.

| Section Name | Content |
|-------------|---------|
| `WideStrings6` | Unicode string table (binary TLV format) |
| `EmbeddedFonts6` | Embedded font data |
| `FileVersionInfo` | File version history |
| `LayerKindMapping` | Mechanical layer kind map |
| `ModelsNoEmbed` | Model references without embedded data |
| `PrimitiveGuids` | Primitive GUID assignments |
| `Textures` | Texture image data |
| `UnionNames` | Union name strings |
| `ConstraintManager` | Constraint manager data |

### 4.5 Models Section (Special)

The `Models` section has a unique structure:

| Stream | Content |
|--------|---------|
| `Models/Header` | u32 count of model entries |
| `Models/Data` | Model metadata records |
| `Models/0` | First model binary blob (STEP/other 3D format) |
| `Models/1` | Second model binary blob |
| `Models/N` | Nth model binary blob |

---

## 5. PcbDoc Loading Pipeline

The loading pipeline is primarily in Delphi (`Advpcb.dll`). The .NET side
acts as a COM wrapper. The key pipeline steps:

### Step 1: Open CFB Document

The `IPCB_StructuredStorage` implementation opens the CFB file using
Microsoft's IStorage API. `RecognizeFile()` validates the format by checking
the `FileHeader` stream.

Source: `IPCB_StructuredStorage.RecognizeFile()` in
`AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_StructuredStorage.cs`

### Step 2: Read FileHeader

The root `FileHeader` stream contains a version identifier string:
- PcbDoc: `"PCB 5.0 Binary File"` (UTF-16LE encoded with u32 length prefix)
- PcbLib: `"PCB 6.0 Binary Library File"` (Win1252, with length prefix + version float + key token)

The `PCBFileFormatVersion()` method returns one of the `TAdvPCBFileFormatVersion` values:

```
ePCBFileFormatNone           = 0
eAdvPCBFormat_Binary_V3      = 1
eAdvPCBFormat_Library_V3     = 2
eAdvPCBFormat_ASCII_V3       = 3
eAdvPCBFormat_Binary_V4      = 4
eAdvPCBFormat_Library_V4     = 5
eAdvPCBFormat_ASCII_V4       = 6
eAdvPCBFormat_Binary_V5      = 7
eAdvPCBFormat_Library_V5     = 8
eAdvPCBFormat_ASCII_V5       = 9
eAdvPCBFormat_Binary_V6     = 10
eAdvPCBFormat_Library_V6    = 11
eAdvPCBFormat_ASCII_V6      = 12
eAdvPCBFormat_Binary_V6_CS  = 13
eAdvPCBFormat_Binary_V6_CM  = 14
eAdvPCBFormat_Binary_V6_PCBWorks = 15
eAdvPCBFormat_PadViaLibrary_V6   = 16
```

Source: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TAdvPCBFileFormatVersion.cs`

### Step 3: Register Sections

Each section storage in the CFB is discovered and an `IPCB_BinarySection`
implementation is created for it via `CreateSection()`. The storage calls
`RegisterWithBoard()` on each section to associate it with the board object.

The full section list is enumerated from the CFB directory entries. The loader
identifies each section's type by its storage name (e.g., `"Arcs6"`, `"Board6"`)
and creates the appropriate specialized section object.

### Step 4: Import Sections from File

Each section's `Import_FromFile()` method reads the `Header` and `Data` streams:

1. Read `Header` stream: always a 4-byte little-endian u32 representing the record count
2. Read `Data` stream: format depends on section type (see Section 6)
3. Parse records from the data stream into primitive objects
4. Call `RegisterWithBoard()` to add primitives to the board's internal collections

The import options are controlled by `TStructuredStorageFileSectionImportOptions`,
which currently only has one option: `ioSkipModels`.

Source: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TStructuredStorageFileSectionImportOption.cs`

### Step 5: Apply Board Section

The `Board6` section is parsed as parameter blocks. Board-level state is set via
`IPCB_Board_SaveLoadParameters`:
- `SetState_BoardVersion(version)` -- set the board format version
- `SetState_BoardOutline(outline)` -- set the board outline
- `UpdateLayerStackTables()` -- rebuild layer stack from saved data
- `AssignLayerStackToLayerPairs()` -- assign drill layer pairs
- `CreateDefaultRules()` -- create default DRC rules if missing

Source: `AD26-dotnet/Altium.Edp.Interfaces/PCBInterfaces/IPCB_Board_SaveLoadParameters.cs`

### Step 6: Parse Primitive Records

For primitive sections (Arcs6, Pads6, etc.), each record in the Data stream is:
```
[1 byte]  Object ID (TObjectId type byte)
[4 bytes] Record length (u32 LE, NOT including type byte)
[N bytes] Record payload (binary fields specific to object type)
```

The object ID byte must match the expected type for the section. Arcs6 should
contain only type=1 records, Pads6 only type=2, etc.

### Step 7: Build Ownership Graph

After all sections are loaded, the loader builds the ownership graph using
cross-reference indices stored in each section. The `SetIndexes` / `GetIndexes`
methods on `IPCB_BinarySection` handle this:

```csharp
void SetIndexes(IPCB_Primitive prim,
    int vNet,        // index into Nets6
    int vPolygon,    // index into Polygons6
    int vComponent,  // index into Components6
    int vPadOwner,   // index into parent pad (for shape-based features)
    int vCoordinate, // index into Coordinates6
    int vDimension   // index into Dimensions6
);
```

This tells the loader which net, polygon, component, etc. owns each primitive.
The index values are 0-based positions in the corresponding section's record list.
A value of -1 or 0 typically means "no owner."

Source: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_BinarySection.cs` lines 75-77

### Step 8: Merge Sidecar Streams

After the core records are loaded, sidecar data is merged in:

1. **WideStrings6**: The binary TLV string table is loaded. Each text primitive
   references its Unicode string by index.

2. **UniqueIDPrimitiveInformation**: Per-primitive GUIDs/unique IDs loaded from
   parameter blocks, keyed by `(ObjectId, IndexForSave)`.

3. **ExtendedPrimitiveInformation**: Extended property overrides loaded from
   parameter blocks, merged into primitives by index.

4. **PrimitiveGuids**: Binary GUID table loaded, `TPrimitiveGUID` entries
   (ObjectId + IndexForSave + 16-byte GUID) applied to primitives.

See Section 7 for detailed sidecar format information.

### Step 9: Post-Load Rebuild

After all data is loaded, the board is rebuilt:
- `RebuildAfterLoad()` -- triggers internal recalculation
- `RebuildConnectivityGraph()` -- rebuild ratsnest
- `ValidateBoardOutlineRegions()` -- validate board outline
- `AnalyzeAllNets()` -- recompute net connectivity
- `InitializeScopeTester()` -- initialize DRC scope engine
- `PostLoadOrSave(false)` -- final post-load housekeeping

Source: `AD26-dotnet/Altium.Edp.Interfaces/PCBInterfaces/IPCB_Board_SaveLoadParameters.cs`

---

## 6. Binary Record Formats

### 6.1 Primitive Binary Records (PcbDoc sections)

All primitive sections share the same framing:

```
+--------+----------+-----------+
| TypeID | Length   | Payload   |
| 1 byte | 4 bytes  | N bytes   |
+--------+----------+-----------+
```

- **TypeID**: `TObjectId` byte (1=Arc, 2=Pad, 3=Via, 4=Track, etc.)
- **Length**: u32 LE, length of payload in bytes (does NOT include the type byte or length field)
- **Payload**: Fixed-size struct specific to the object type, possibly followed by variable-length data

The `Header` stream for each section is always exactly 4 bytes: the u32 LE
record count.

### 6.2 Parameter Block Records (PcbDoc sections)

Parameter sections contain concatenated parameter blocks:

```
+----------+----------------------------------+
| Length   | Parameter String                  |
| 4 bytes  | N bytes (Win1252, NUL-terminated) |
+----------+----------------------------------+
```

- **Length**: u32 LE, length of the parameter string (including NUL terminator)
- **Parameter String**: Pipe-delimited key-value pairs: `|KEY1=VALUE1|KEY2=VALUE2|`

The string encoding is Windows-1252 (Western European), not UTF-8. Unicode
text is stored separately in the WideStrings sidecar stream.

### 6.3 Prefixed Parameter Block Records

Rules6, Dimensions6, Coordinates6, and NewRules6 use a prefixed variant:

```
+--------+----------+----------------------------------+
| Prefix | Length   | Parameter String                  |
| 2 bytes| 4 bytes  | N bytes (Win1252, NUL-terminated) |
+--------+----------+----------------------------------+
```

- **Prefix**: u16 LE prefix word (interpretation is section-specific)
- **Length**: u32 LE, length of the parameter string
- **Parameter String**: Same pipe-delimited format

### 6.4 PcbLib Primitive Records (per-footprint Data stream)

PcbLib footprint Data streams begin with a length-prefixed pattern name block,
then contain packed binary primitive records:

```
[Pattern Name Block]
  4 bytes: u32 LE block length
  N bytes: length-prefixed string (1-byte len + ASCII name)

[Primitive Records]
  1 byte: TObjectId type byte
  4 bytes: u32 LE record length (high byte may contain flags)
  N bytes: record payload
  ...repeating...
```

The high byte of the u32 length field in PcbLib records may contain flags
(the `SIZE_FLAG_MASK` constant masks off flags to get the actual length).

### 6.5 PcbLib Parameters Stream

Each footprint has a `Parameters` stream containing a single parameter block:

```
4 bytes: u32 LE total block length
1 byte: string length
N bytes: Win1252 parameter string
```

The parameter string contains footprint metadata like `|PATTERN=name|HEIGHT=value|`.

---

## 7. Sidecar Stream Merging

Sidecar streams extend primitive data without modifying the core binary records.
They exist as a backwards-compatibility mechanism: new fields were added via new
streams so older readers could skip them.

### 7.1 WideStrings (PcbDoc board-level)

**Stream**: `WideStrings6/Header` + `WideStrings6/Data`

The Data stream uses a binary TLV encoding for each string entry:

| Type byte | Length field | Data encoding | Notes |
|-----------|-------------|---------------|-------|
| `0x06` | 1 byte (u8) | ASCII | Short ASCII strings (len <= 255) |
| `0x0C` | 4 bytes (u32 LE) | ASCII | Long ASCII strings |
| `0x12` | 4 bytes (u32 LE, in chars) | UTF-16LE | Unicode, length is chars not bytes |
| `0x14` | 4 bytes (u32 LE) | UTF-8 | Unicode, length is bytes |

Primitives reference their wide string by zero-based index into this table.

### 7.2 WideStrings (PcbLib footprint-level)

**Stream**: `<FootprintName>/WideStrings`

PcbLib footprint-level WideStrings use a **completely different format** -- parameter
blocks rather than binary TLV:

```
[4-byte block header: u32 LE length]
[NUL-terminated parameter string]
  e.g. |ENCODEDTEXT0=65,66,67|ENCODEDTEXT1=...|
```

This is a sequence of parameter blocks, one per WideStrings entry. The
`ENCODEDTEXT{N}` values are comma-separated integer sequences representing
encoded text codepoints.

### 7.3 UniqueIDPrimitiveInformation

**PcbDoc streams**: `UniqueIDPrimitiveInformation/Header` + `UniqueIDPrimitiveInformation/Data`
**PcbLib streams**: `<FootprintName>/UniqueIDPrimitiveInformation/Header` + `.../Data`

Format: `Header` contains u32 count. `Data` contains that many parameter blocks
(`u32 len + NUL-terminated parameter string`).

Each parameter block contains:
- `PRIMITIVEINDEX=<N>` -- index of the primitive this applies to
- `PRIMITIVEOBJECTID=<type>` -- TObjectId of the primitive
- `UNIQUEID=<guid>` -- the unique identifier string

### 7.4 ExtendedPrimitiveInformation

**PcbDoc streams**: `ExtendedPrimitiveInformation/Header` + `ExtendedPrimitiveInformation/Data`
**PcbLib streams**: `<FootprintName>/ExtendedPrimitiveInformation/Header` + `.../Data`

Same parameter-block format as UniqueIDPrimitiveInformation. Contains extended
properties that were added in later format versions (e.g., extended pad shapes,
impedance profiles, etc.).

### 7.5 PrimitiveGuids

**PcbDoc streams**: `PrimitiveGuids/Header` + `PrimitiveGuids/Data`
**PcbLib streams**: `<FootprintName>/PrimitiveGuids/Header` + `.../Data`

Binary format, fixed-size records of 24 bytes each:

```
struct TPrimitiveGUID {  // 24 bytes, pack=1
    int ObjectId;        // 4 bytes: TObjectId
    int IndexForSave;    // 4 bytes: primitive index within section
    GUID guid;           // 16 bytes: standard Windows GUID
}
```

The Header u32 count indicates the number of 24-byte entries in Data.

Source: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TPrimitiveGUID.cs`

### 7.6 Merging Process

During load:
1. Core primitive records are parsed from each section's Data stream
2. Primitives are assigned their `IndexForSave` based on their sequential position
3. Sidecar streams are read and entries matched to primitives by `(ObjectId, IndexForSave)` pair
4. Matched sidecar data is merged into the primitive object's in-memory representation
5. At runtime, there is no distinction between core and sidecar data

During save:
1. Primitives are assigned sequential `IndexForSave` values per section
2. Core binary records are written to each section's Data stream
3. Sidecar data is split out from each primitive and written to the appropriate sidecar streams
4. The `AddTextsForSaveList()` method collects text primitives that need WideStrings entries

Source: `IPCB_StructuredStorage.AddTextsForSaveList()` and
`IPCB_StructuredStorage.AddWSForLoadList()` in
`AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_StructuredStorage.cs`

---

## 8. PcbLib Loading Pipeline

The PcbLib loading pipeline differs from PcbDoc in several ways.

### Step 1: Open CFB and Identify Format

The `FileHeader` stream is read to identify the library format version:

```
[4 bytes] u32 LE: block length (= header text length)
[1 byte]  u8: header text length
[N bytes] ASCII header text (e.g., "PCB 6.0 Binary Library File")
[8 bytes] f64 LE: file format version number (e.g., 5.01)
[4 bytes] u32 LE: key block length
[1 byte]  u8: key length
[N bytes] ASCII key token (e.g., "AAAAAAAA")
```

Source: `pcblib_streams.rs` function `parse_file_header_stream()`

### Step 2: Read SectionKeys (Optional)

The optional `/SectionKeys` stream maps display footprint names to obfuscated
storage names. Format:

```
[4 bytes] u32 LE: entry count
For each entry:
  [4 bytes] u32 LE: name block length
  [1 byte]  u8: string length
  [N bytes] ASCII footprint display name
  [4 bytes] u32 LE: key block length
  [1 byte]  u8: string length
  [N bytes] ASCII storage key (8-char obfuscated name)
```

When SectionKeys exists, the actual CFB storage names are the keys (e.g., `"ABCDEFGH"`),
and the mapping provides the original footprint names.

### Step 3: Enumerate Footprint Storages

Top-level CFB storages are enumerated. A storage is identified as a footprint if:
1. It is not one of the system storages (`SectionKeys`, `FileHeader`, `Library`, `FileVersionInfo`)
2. It contains a `Data` sub-stream

Source: `pcblib.rs` line 180-207

### Step 4: Read Library-Global Streams

The `/Library/` storage contains shared data:

| Stream | Content |
|--------|---------|
| `Library/Header` | u32 count |
| `Library/Data` | Library-wide parameter data |
| `Library/EmbeddedFonts` | Embedded font binary data |
| `Library/ComponentParamsTOC/{Header,Data}` | Component parameter table of contents |
| `Library/LayerKindMapping/{Header,Data}` | Mechanical layer kind mapping |
| `Library/Models/{Header,Data,0,1,...}` | 3D model storage (see Section 9) |
| `Library/ModelsNoEmbed/{Header,Data}` | Model references without embedded blobs |
| `Library/PadViaLibrary/{Header,Data}` | Pad/via template library |
| `Library/Textures/{Header,Data}` | Texture data |

The `IPCB_LibrarySection` interface handles the library-global data.
`ImportModel_FromFile()` loads individual 3D models by GUID and checksum.

Source: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_LibrarySection.cs`

### Step 5: Load Each Footprint

For each footprint storage:

#### 5a. Read Parameters Stream
The `Parameters` stream contains a single parameter block with footprint metadata:
`|PATTERN=name|HEIGHT=value|DESCRIPTION=text|...`

#### 5b. Read Header Stream
The `Header` stream contains count and version information.

#### 5c. Read Data Stream
The `Data` stream begins with a length-prefixed pattern name block:
```
[4 bytes] u32 LE: pattern name block length
[1 byte]  u8: pattern name string length
[N bytes] ASCII pattern name
```

Following the pattern name block are packed binary primitive records:
```
[1 byte]  TObjectId type byte
[4 bytes] u32 LE record length (with optional flag bits in high byte)
[N bytes] record payload
```

#### 5d. Read Sidecar Streams
Within each footprint storage, optional sidecar streams are read:

| Stream | Format |
|--------|--------|
| `<Footprint>/WideStrings` | Parameter-block format (NOT binary TLV!) |
| `<Footprint>/PrimitiveGuids/{Header,Data}` | Binary GUID table (24 bytes/entry) |
| `<Footprint>/UniqueIDPrimitiveInformation/{Header,Data}` | Parameter-block table |
| `<Footprint>/ExtendedPrimitiveInformation/{Header,Data}` | Parameter-block table |

**Critical difference from PcbDoc**: PcbLib footprint WideStrings use parameter-block
format, not the binary TLV format used by board-level WideStrings6.

#### 5e. Build Footprint Object
The `IPCB_LibComponentSection` loads the parsed primitives and:
- Creates the `IPCB_LibComponent` object
- Sets the component's owner library via `SetState_OwnerLib()`
- Loads 3D models if `GetState_LoadModels()` is true
- Tracks missing models via `MissingComponentBodyModelsCount()` / `MissingComponentBodyModelsName()`

Source: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_LibComponentSection.cs`

### Step 6: V6 Format Extras

For V6 format libraries (`IPCB_LibBinaryV6Storage`), additional operations are
available:
- `ReadComponentParamsTOC()` -- read the component parameter table of contents
- `ReadLayerKindMapping()` -- read the mechanical layer kind mapping
- `LoadComponentFromLibrary6File()` -- load a single component with layer mapping

Source: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_LibBinaryV6Storage.cs`

### Step 7: Post-Load Synchronization

After loading, the library state is synchronized:
- `SaveStateLayersAndPairs()` -- save layer state
- `SynchronizeLayerKinds()` -- sync layer kinds with mapping
- `UpdateOwnerBoard()` -- update the internal board object

Source: `AD26-dotnet/Altium.Edp.Interfaces/PCBInterfaces/IPCB_Library_SaveLoadParameters.cs`

---

## 9. 3D Model Handling

### 9.1 Model Types

```
enum T3DModelType : byte {
    e3DModelType_Extruded = 0,   // Extruded 2D outline
    e3DModelType_Generic  = 1,   // Generic STEP/STP file
    e3DModelType_Cylinder = 2,   // Parametric cylinder
    e3DModelType_Sphere   = 3    // Parametric sphere
}
```

Source: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/T3DModelType.cs`

### 9.2 Model Source

Models can come from:
- Local file system (file path reference)
- Embedded in the document (binary blob in Models stream)
- Vault/workspace (GUID reference to managed content)

The `IPCB_Model` interface provides:
- `GetFileName()` -- local file path
- `GetEmbed()` / `SetEmbed()` -- whether model data is embedded
- `GetModelSource()` -- enum indicating source type
- `GetVaultGUID()` / `GetItemGUID()` / `GetItemRevisionGUID()` -- workspace references

### 9.3 ComponentBody and Model Relationship

A `ComponentBody` (`eComponentBodyObject`, type=12) is a region primitive with
3D model data attached. Key interfaces:

- `IPCB_ComponentBody.GetModel()` -- returns the `IPCB_Model` instance
- `IPCB_ComponentBody.SetModel()` -- assigns a model
- `ModelFactory_FromFilename()` -- create model from STEP file
- `ModelFactory_CreateCylinder()` -- create parametric cylinder
- `ModelFactory_CreateSphere()` -- create parametric sphere
- `ModelFactory_CreateExtruded()` -- create extruded body

Source: `AD26-dotnet/Altium.SDK.Interfaces/PCB/IPCB_ComponentBody.cs`

### 9.4 Model Storage Format

#### PcbDoc Models Section

The `Models/` storage in PcbDoc contains:
- `Header`: u32 count of model records
- `Data`: Model metadata (parameter blocks describing each model)
- `0`, `1`, ..., `N`: Raw binary blobs containing the actual 3D model data
  (typically STEP format)

The `IPCB_ModelsSection` interface handles import/export:
- `ImportModel_FromFile(GUID modelID, uint checkSum)` -- load a model blob by ID

The `IPCB_ModelsNoEmbedSection` tracks models that are referenced but not
embedded, with `InvalidChecksumModelsCount()` reporting models with checksum
mismatches.

#### PcbLib Models Storage

In PcbLib, models are stored under `/Library/Models/`:
- `Library/Models/Header`: u32 count
- `Library/Models/Data`: Model metadata
- `Library/Models/0`, `Library/Models/1`, ...: Model binary blobs

This is a shared model pool -- multiple footprints can reference the same model
by its index into this pool.

### 9.5 Model Checksum

Each model has a checksum (`GetChecksum()`) used to detect stale or modified
models. The `TStorageFeature` flag system includes checks like
`eHasCustomThermalReliefsAtWriteStage` that indicate which feature sets are
present in the saved file.

---

## 10. File Version Handling

### 10.1 Format Version Enum

The `TAdvPCBFileFormatVersion` enum covers all known format versions:

| Value | Name | Description |
|-------|------|-------------|
| 0 | `ePCBFileFormatNone` | Unknown/invalid format |
| 1 | `eAdvPCBFormat_Binary_V3` | Protel 99 SE binary board |
| 2 | `eAdvPCBFormat_Library_V3` | Protel 99 SE library |
| 3 | `eAdvPCBFormat_ASCII_V3` | Protel 99 SE ASCII |
| 4 | `eAdvPCBFormat_Binary_V4` | DXP binary board |
| 5 | `eAdvPCBFormat_Library_V4` | DXP library |
| 6 | `eAdvPCBFormat_ASCII_V4` | DXP ASCII |
| 7 | `eAdvPCBFormat_Binary_V5` | Altium Designer binary board |
| 8 | `eAdvPCBFormat_Library_V5` | Altium Designer library |
| 9 | `eAdvPCBFormat_ASCII_V5` | Altium Designer ASCII |
| 10 | `eAdvPCBFormat_Binary_V6` | Modern AD binary board |
| 11 | `eAdvPCBFormat_Library_V6` | Modern AD library |
| 12 | `eAdvPCBFormat_ASCII_V6` | Modern AD ASCII |
| 13 | `eAdvPCBFormat_Binary_V6_CS` | CircuitStudio variant |
| 14 | `eAdvPCBFormat_Binary_V6_CM` | CircuitMaker variant |
| 15 | `eAdvPCBFormat_Binary_V6_PCBWorks` | PCBWorks variant |
| 16 | `eAdvPCBFormat_PadViaLibrary_V6` | Pad/via library format |

Source: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TAdvPCBFileFormatVersion.cs`

### 10.2 Storage Feature Flags

The `TStorageFeature` enum provides fine-grained feature flags that indicate
which capabilities were used when saving the file:

| Value | Flag | Description |
|-------|------|-------------|
| 0 | `eHasImpedanceProfileCount` | Impedance profiles in layer stack |
| 1 | `eHasPrintedElectronicLayers` | Printed electronics layer support |
| 2 | `eHasMicroVias` | Micro via support |
| 3 | `eHasCustomThermalReliefsAtWriteStage` | Custom thermal relief shapes |
| 4 | `eHasSystemParametersAtWriteStage` | System-level parameters |
| 5 | `eHasShapeBasedRegions` | Shape-based region format (vs legacy) |
| 6 | `eHasShapeBasedCompBodies` | Shape-based component bodies |
| 7 | `eHasRF20IsUsedAtWriteStage` | RF 2.0 features |
| 8 | `eHasIPC4761ViaTypesAtWriteStage` | IPC-4761 via type classification |
| 9 | `eHasCustomPadShapesAtWriteStage` | Custom pad shapes |
| 10 | `eHasRotatedAnyAngleEmbeddedBoardArrayAtWriteStage` | Rotated embedded boards |
| 11 | `eHasFootprintParametersAtWriteStage` | Footprint parameter storage |
| 12 | `eHasCustomReliefInfosAtWriteStage` | Custom thermal relief info |
| 13 | `eHasClearanceByLayerRuleAtWriteStage` | Per-layer clearance rules |
| 14 | `eHasMatrixRuleAtWriteStage` | Matrix-style clearance rules |
| 15 | `eHasTHPadPasteInfosAtWriteStage` | Through-hole pad paste info |
| 16 | `eHasCustomMaskInfosAtWriteStage` | Custom mask expansion data |
| 17 | `eHasPolygonsWithNeckWidthFromRule` | Polygons with neck width from rule |
| 18 | `eHasNeckDownRuleAtWriteStage` | Neck-down routing rules |
| 19 | `eHasSingleLayerModeAtWriteStage` | Single-layer editing mode |
| 20 | `eHasCustomPadShapesDonutAtWriteStage` | Donut-shaped custom pads |
| 21 | `eHasWirebondAtWriteStage` | Wire bonding features |
| 22 | `eHasDiffpairPhaseMatching` | Differential pair phase matching |
| 23 | `eHasExtendedGroupIndicesAreUsed` | Extended group reference format |
| 24 | `eHasIncreasedSignalLayers` | Support for > 32 signal layers |
| 25 | `eHasZAxisClearanceRuleAtWriteStage` | Z-axis (3D) clearance rules |

Source: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TStorageFeature.cs`

These flags are stored via `IPCB_StructuredStorage.GetState_Feature()` /
`SetState_Feature()` and affect how the loader interprets certain record fields.

### 10.3 Board Version Number

The board version is a floating-point number stored in the `Board6` section
parameters (accessed via `IPCB_Board.GetState_BoardVersion()`). This is separate
from the file format version and tracks incremental format changes within the
V6 format family.

### 10.4 FileVersionInfo Section

The `FileVersionInfo/{Header,Data}` section stores a history of software versions
that have modified the file. The `IPCB_FileVersionInfoList` interface provides
access to this list.

Source: `AD26-dotnet/Altium.Edp.Interfaces/PCBInterfaces/IPCB_FileVersionInfoList.cs`

---

## 11. Extended Group Indices

When `eHasExtendedGroupIndicesAreUsed` is set, the file uses extended group
references stored via:

```csharp
struct TPrimitiveKey {  // 8 bytes, pack=1
    int ObjectId;
    int IndexForSave;
}

struct TReferenceToGroup {  // 16 bytes, pack=8
    TPrimitiveKey Prim;       // The primitive
    TPrimitiveKey PrimGroup;  // The group it belongs to
}
```

The `IPCB_BinarySection` interface handles these:
- `ExtendedIndexCount()` -- number of extended group references
- `GetExtendedIndex(i)` / `AddExtendedIndex(ref)` -- access entries
- `ApplyExtendedIndices()` -- apply after loading

Source:
- `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TPrimitiveKey.cs`
- `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TReferenceToGroup.cs`

---

## 12. Summary of Key Source Files

### .NET Interfaces (AD26-dotnet/)

| File | Purpose |
|------|---------|
| `Altium.Edp.Interfaces/RT_PCB/IPCB_StructuredStorage.cs` | CFB document access |
| `Altium.Edp.Interfaces/RT_PCB/IPCB_LibBinaryStorage.cs` | PcbLib storage |
| `Altium.Edp.Interfaces/RT_PCB/IPCB_LibBinaryV6Storage.cs` | PcbLib V6 storage |
| `Altium.Edp.Interfaces/RT_PCB/IPCB_BinarySection.cs` | Section base |
| `Altium.Edp.Interfaces/RT_PCB/IPCB_BoardBinarySection.cs` | Board section |
| `Altium.Edp.Interfaces/RT_PCB/IPCB_LibComponentSection.cs` | Footprint section |
| `Altium.Edp.Interfaces/RT_PCB/IPCB_LibrarySection.cs` | Library section |
| `Altium.Edp.Interfaces/RT_PCB/IPCB_ModelsSection.cs` | Models section |
| `Altium.Edp.Interfaces/RT_PCB/IPCB_ModelsNoEmbedSection.cs` | Non-embedded models |
| `Altium.Edp.Interfaces/RT_PCB/IPCB_PolygonsBinarySection.cs` | Polygons section |
| `Altium.Edp.Interfaces/RT_PCB/IPCB_DimensionsSection.cs` | Dimensions section |
| `Altium.Edp.Interfaces/RT_PCB/IPCB_TextureSection.cs` | Textures section |
| `Altium.Edp.Interfaces/RT_PCB/IPCB_LayerKindMappingSection.cs` | Layer mapping |
| `Altium.Edp.Interfaces/RT_PCB/TAdvPCBFileFormatVersion.cs` | Format version enum |
| `Altium.Edp.Interfaces/RT_PCB/TStorageFeature.cs` | Feature flags |
| `Altium.Edp.Interfaces/RT_PCB/TObjectId.cs` | Object type enum |
| `Altium.Edp.Interfaces/RT_PCB/TPrimitiveGUID.cs` | GUID record struct |
| `Altium.Edp.Interfaces/RT_PCB/TPrimitiveKey.cs` | Primitive key struct |
| `Altium.Edp.Interfaces/RT_PCB/TReferenceToGroup.cs` | Group reference struct |
| `Altium.Edp.Interfaces/RT_PCB/T3DModelType.cs` | 3D model type enum |
| `Altium.Edp.Interfaces/RT_PCB/Consts.cs` | Constants and mappings |
| `Altium.Edp.Interfaces/PCBInterfaces/IPCB_Board_SaveLoadParameters.cs` | Board load parameters |
| `Altium.Edp.Interfaces/PCBInterfaces/IPCB_Library_SaveLoadParameters.cs` | Library load parameters |
| `Altium.Edp.Interfaces/PCBInterfaces/IPCB_LibComponent_SaveLoadParameters.cs` | Component load params |
| `Altium.Edp.Interfaces/PCBInterfaces/IPCB_Primitive_SaveLoadParameters.cs` | Primitive load params |
| `Altium.SDK.Interfaces/PCB/IPCB_Board.cs` | Board COM interface |
| `Altium.SDK.Interfaces/PCB/IPCB_Library.cs` | Library COM interface |
| `Altium.SDK.Interfaces/PCB/IPCB_ComponentBody.cs` | ComponentBody COM interface |
| `Altium.SDK.Interfaces/PCB/IPCB_Model.cs` | Model COM interface |
| `Altium.SDK.Interfaces/PCB/IPCB_WideStrings.cs` | WideStrings COM interface |
| `Altium.SDK.Interfaces/PCB/IPCB_LibraryLoaderSaver.cs` | Library loader/saver |

### Rust Implementation

| File | Purpose |
|------|---------|
| `crates/altium-format/src/documents/pcbdoc.rs` | PcbDoc document module |
| `crates/altium-format/src/documents/pcbdoc_streams.rs` | PcbDoc stream codecs |
| `crates/altium-format/src/documents/pcblib.rs` | PcbLib document module |
| `crates/altium-format/src/documents/pcblib_streams.rs` | PcbLib stream codecs |

### Delphi (via Ghidra)

The actual binary I/O implementation lives in `Advpcb.dll` (Delphi native code).
All `PcbApi_*` functions are exported from this DLL. See `pcb-api-functions.md`
for the complete function reference.
