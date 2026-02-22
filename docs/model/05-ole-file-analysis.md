# OLE File Analysis - Real Altium Files

Comprehensive analysis of real OLE/CFB files from the `data/` directory, validated using `scripts/ole-inspect.py`.

## 1. File Types Overview

| Extension | OLE? | Description |
|-----------|------|-------------|
| `.SchDoc` | Yes  | Schematic document (single sheet) |
| `.SchLib` | Yes  | Schematic symbol library |
| `.PcbDoc` | Yes  | PCB board layout document |
| `.PcbLib` | Yes  | PCB footprint library |
| `.PrjPcb` | No   | Project file (plain-text INI format) |

All OLE files use Microsoft Compound Binary File (CFB) format. Project files are **not** OLE -- they are plain-text Windows INI-style files with BOM (`\xEF\xBB\xBF`).

---

## 2. SchDoc Files (Schematic Documents)

**Files examined:** `01_BlockDiagram.SchDoc`, `07_FPGA.SchDoc`

### 2.1 Stream Structure

SchDoc files are flat (no OLE storages). They contain exactly **3 streams**:

| Stream | Kind | Description |
|--------|------|-------------|
| `FileHeader` | text | File metadata header + all schematic records (one block per record) |
| `Additional` | text | Secondary header with format version string |
| `Storage` | text | Embedded binary resources (images, templates) with compressed blocks |

### 2.2 File Headers

Both `FileHeader` and `Additional` start with a header block (block index 0) containing:
```
|HEADER=Protel for Windows - Schematic Capture Binary File Version 5.0|Weight=65|MinorVersion=2|UniqueID=LVUUGVHQ
```

Key header fields:
- `HEADER`: Format identification string (always `Protel for Windows - Schematic Capture Binary File Version 5.0`)
- `Weight`: Total record count indicator
- `MinorVersion`: Sub-version (values 2 and 9 observed)
- `UniqueID`: 8-character uppercase alphanumeric identifier

### 2.3 Record Format

All records are **size-prefixed blocks** containing **pipe-delimited key=value text**.

Block framing: `[u32 header]` where:
- Bits 0-23: payload size (little-endian)
- Bits 24-31: flags byte (0x00 for text records, 0x01 for compressed blocks)

Text payload format:
```
|RECORD=31|FontIdCount=5|Size1=10|FontName1=Times New Roman|...
```

- Leading `|` before first key
- `RECORD=N` identifies the record type
- Null byte (`\x00`) at end of payload

### 2.4 Record 31 (Sheet Properties)

Always the first RECORD in `FileHeader` (block 1). Contains:
- Font definitions (`FontIdCount`, `SizeN`, `FontNameN`, `BoldN`, `ItalicN`)
- Grid settings (`SnapGridOn`, `SnapGridSize`, `VisibleGridOn`, `VisibleGridSize`)
- Sheet size (`CustomX`, `CustomY`)
- Template reference (`TemplateFileName`)
- Display units (`Display_Unit`: 0=DXP default, 1=imperial/mils)

### 2.5 Record 39 (Template Reference)

Contains `FileName` pointing to the `.SchDot` template file.

### 2.6 Storage Stream (Embedded Resources)

Block 0 is a text header: `|HEADER=Icon storage|Weight=N`

Subsequent blocks have `flags=0x01` and contain **compressed payloads**:
```
[0xD0][u8 id_len][id_bytes][u32 block_header][zlib_compressed_data]
```

- `0xD0` magic byte identifies compressed storage
- `id_bytes`: Original file path (e.g., `D:\Saniok\...\LimeMicroLogoPCB.bmp`)
- Compressed data is zlib-compressed (sometimes with 2-byte skip before raw deflate)

Example from `01_BlockDiagram.SchDoc`:
- Block 1: BMP image (6,343 bytes compressed -> 99,478 bytes)
- Block 2: PNG image (181,071 bytes compressed -> 2,780,210 bytes)

### 2.7 Record ID Distribution (FPGA.SchDoc -- 5,970 records)

| RECORD | Count | Description |
|--------|-------|-------------|
| 1 | 63 | Component |
| 2 | 1,486 | Pin |
| 4 | 111 | Label |
| 6 | 217 | Polyline |
| 12 | 3 | Arc |
| 14 | 166 | Rectangle |
| 17 | 42 | Power Port |
| 22 | 5 | No Connect |
| 25 | 146 | Net Label |
| 27 | 227 | Wire |
| 28 | 6 | Bus |
| 29 | 96 | Bus Entry |
| 30 | 1 | Image |
| 31 | 1 | Sheet Properties |
| 34 | 63 | Designator |
| 39 | 1 | Template Reference |
| 41 | 3,061 | Parameter (attribute) |
| 43 | 6 | Warning Sign |
| 44 | 63 | Implementation Parent |
| 45 | 63 | Implementation Child |
| 46 | 63 | Implementation Pin Map |
| 48 | 63 | Implementation Parameters |
| 209 | 8 | Parameter Set |
| 225 | 5 | Compile Mask |

---

## 3. SchLib Files (Schematic Libraries)

**Files examined:** `LimeMicroAltiumLib_schLib.SchLib`, `Synthiam.SchLib`, `BlankSchlibComponent.SchLib`

### 3.1 Stream Structure

SchLib files use **OLE storages** to organize components. Each component is a storage (directory) with child streams.

Top-level structure:
```
FileHeader          -- Library-wide metadata (text, single block)
Storage             -- Icon storage (same as SchDoc)
<ComponentName>/
    Data            -- Component records (size-prefixed text blocks)
    PinFrac         -- Pin fractional coordinates (AD16+, compressed blocks)
    PinPackageLength -- Pin package length data (compressed blocks)
    PinSymbolLineWidth -- Pin symbol line widths (compressed blocks)
    Redirection     -- Alias redirect (rare, for component aliases)
```

**Observed counts:**
- `LimeMicroAltiumLib_schLib.SchLib`: 200 storages, 784 streams
- `Synthiam.SchLib`: 174 storages, 176 streams (no PinFrac/PinPackageLength/PinSymbolLineWidth sidecar streams)
- `BlankSchlibComponent.SchLib`: 1 storage, 3 streams

### 3.2 FileHeader

The FileHeader in SchLib contains:
```
HEADER=Protel for Windows - Schematic Library Editor Binary File Version 5.0
```

Key differences from SchDoc:
- Contains `CompCount=N` with a **component index table**
- `LibRefN=<name>`: Component reference name at index N
- `CompDescrN=<desc>`: Component description at index N
- `PartCountN=<count>`: Number of parts (multi-part components)
- `AliasCountN=<count>` and `CompNAliasM=<alias>`: Component aliases

Example from Synthiam.SchLib (173 components):
```
CompCount=173|LibRef0=PIC16F1704 - Ultrasonic|PartCount0=2|LibRef1=100-pin GT connector|PartCount1=2|...
```

### 3.3 Component Data Stream

Each `<ComponentName>/Data` stream contains the full component definition as size-prefixed blocks:

**Block 0 -- RECORD=1 (Component Definition):**
```
RECORD=1|LibReference=2N3904|ComponentDescription=NPN General Purpose Amplifier|PartCount=2|
DisplayModeCount=1|IndexInSheet=-1|OwnerPartId=-1|CurrentPartId=1|SourceLibraryName=*|
TargetFileName=*|UniqueID=XXOEJKEY|AreaColor=11599871|Color=128|PartIDLocked=F|AllPinCount=3
```

**Subsequent blocks -- child records (owned by component):**
- `RECORD=41` (Parameter): Attributes like Published, LatestRevisionDate, ComponentLink URLs
- `RECORD=2` (Pin): Not in text form in some files -- pins may be in binary blocks (flags != 0x00)
- `RECORD=13` (Line): Drawing primitives
- `RECORD=7` (Polygon): Filled shapes
- `RECORD=34` (Designator): The `Q?`, `U?`, etc. designator
- `RECORD=44` (Implementation Parent): Links to footprint models
- `RECORD=45` (Implementation Child): Model definition (`ModelName`, `ModelType=PCBLIB/SIM/SI`)
- `RECORD=46` (Implementation Pin Map)
- `RECORD=47` (Implementation Pin Map Entry): `DesIntf`, `DesImpCount`, `DesImp0`
- `RECORD=48` (Implementation Parameters)

### 3.4 Pin Records in SchLib

Pins in older SchLib files appear as binary blocks (flags=0x00, but non-text payload). In the text form they show raw binary with fields like pin name and number embedded.

The LimeMicro SchLib (AD16/MinorVersion=2) has **sidecar streams** per component:
- `PinFrac`: Compressed blocks, one per pin, containing fractional coordinate data (12 bytes uncompressed per pin)
- `PinPackageLength`: Compressed blocks (42 bytes uncompressed per pin)
- `PinSymbolLineWidth`: Compressed blocks (42 bytes uncompressed per pin)

These sidecar streams are **absent** in the Synthiam SchLib (MinorVersion=9), suggesting these extended pin properties may be stored inline in newer format versions.

Each sidecar stream starts with a text header block:
```
|HEADER=PinFrac|Weight=N
```
Where `Weight=N` matches the pin count.

### 3.5 Redirection Streams

For component aliases, a `Redirection` stream maps one name to another:
```
SectionName=PIC10F220T-I/OT
```
This means `PIC10F220` is an alias that redirects to the `PIC10F220T-I/OT` storage.

### 3.6 Storage Stream

Same format as SchDoc: `|HEADER=Icon storage|Weight=N` with optional compressed bitmap blocks.

---

## 4. PcbDoc Files (PCB Board Documents)

**File examined:** `LimeSDR_Mini_1v3_Rounded.PcbDoc`

### 4.1 Stream Structure

PcbDoc files are the most complex. They use **section-based storages** where each section type is an OLE storage containing `Data` and `Header` sub-streams.

**46 storages, 138 streams** in the examined file.

#### Primitive Sections (Binary Object Data)

| Section Storage | Object ID | Count | Description |
|-----------------|-----------|-------|-------------|
| `Arcs6/` | 1 | 260 | Arc primitives |
| `Pads6/` | 2 | 1,771 | Pad primitives |
| `Vias6/` | 3 | 821 | Via primitives |
| `Tracks6/` | 4 | 12,710 | Track (trace) primitives |
| `Texts6/` | 5 | 1,339 | Text objects |
| `Fills6/` | 6 | 2 | Filled rectangles |
| `Regions6/` | 11 | ~581 | Region/polygon primitives |
| `ShapeBasedRegions6/` | 11 | ~581 | Shape-based regions |
| `ComponentBodies6/` | 12 | ~776 | 3D component bodies |
| `ShapeBasedComponentBodies6/` | 12 | ~776 | Shape-based 3D bodies |
| `BoardRegions/` | 11 | 1 | Board outline region |
| `Texts/` | 5 | 3 | Legacy text objects |

#### Binary Data Format

Each primitive section's `Data` stream contains binary objects. The format is:
```
[u8 type_id][u32 subrecord_len][subrecord_bytes]...
```

Subrecord count depends on object type:
- Pad (type 2): **6 subrecords**
- Text (type 5): **2 subrecords**
- All others: **1 subrecord**

The `Header` stream contains a 4-byte value interpreted as `[u32 count]` (the number of objects in the Data stream).

#### Text/Parameter Sections

| Section Storage | Description |
|-----------------|-------------|
| `Board6/` | Board-level properties (layer stack, grid, origin) |
| `Components6/` | Component placement records |
| `Nets6/` | Net definitions |
| `Rules6/` | Design rules |
| `Classes6/` | Net classes |
| `DifferentialPairs6/` | Differential pair definitions |
| `Polygons6/` | Polygon pour definitions |
| `Models/` | 3D model references + embedded model data |
| `FileVersionInfo/` | Version compatibility information |
| `PrimitiveParameters/` | Per-primitive extended parameters |
| `UniqueIDPrimitiveInformation/` | Unique ID mapping for primitives |
| `WideStrings6/` | Wide (Unicode) string data for text objects |

### 4.2 FileHeader

PcbDoc FileHeader is a short text block:
```
PCB 6.0 Binary File
```
Followed by binary data (not the same format as SchDoc headers).

The actual file version string is embedded differently -- first block has a Pascal-style length-prefixed string.

### 4.3 Components6 Records

Each component is one size-prefixed text block:
```
SELECTION=FALSE|LAYER=TOP|LOCKED=FALSE|X=4686.0219mil|Y=4313.5838mil|
PATTERN=RES0402|NAMEON=TRUE|COMMENTON=FALSE|ROTATION=1.80000000000000E+0002|
HEIGHT=15.748mil|SOURCEDESIGNATOR=R96|SOURCEFOOTPRINTLIBRARY=LimeMicroAltiumLib_pcbLib.PcbLib|
SOURCECOMPONENTLIBRARY=LimeFPGAboard_schLib.SchLib|SOURCELIBREFERENCE=Res_0402|
FOOTPRINTDESCRIPTION=Chip Resistor, Body 1.0x0.5mm, EIA 0402, IPC Medium Density|
UNIQUEID=LFDGOEQV
```

Note: PcbDoc text records do **not** use `RECORD=N` -- they use a flat key=value format without a RECORD identifier. The record type is determined by which section the stream belongs to.

### 4.4 Board6 Properties

Large single-block text record with comprehensive board settings:
- Layer stack configuration
- Grid settings (`SNAPGRIDSIZE`, `VISIBLEGRIDSIZE`)
- Design origin (`ORIGINX`, `ORIGINY`)
- Polygon pour settings
- Board outline definition

### 4.5 Nets6 Records

One block per net:
```
NAME=NetR95_2|VISIBLE=TRUE|COLOR=7709086|LOOPREMOVAL=TRUE|
TOPLAYER_MRWIDTH=5.9055mil|...|BOTTOMLAYER_MRWIDTH=5.9055mil
```

Contains per-layer minimum route widths for up to 30 mid-layers.

### 4.6 Rules6 Records

Design rules with:
```
RULEKIND=<rule_type>|NETSCOPE=<scope>|NAME=<name>|ENABLED=TRUE|
PRIORITY=<n>|SCOPE1EXPRESSION=<expr>|SCOPE2EXPRESSION=<expr>
```

Rule kinds include clearance, width, polygon connect style, etc.

### 4.7 FileVersionInfo

Version compatibility table stored as comma-separated ASCII ordinals:
```
COUNT=33|VER0=54,46,51|FWDMSG0=<ASCII ordinals>|BKMSG0=<ASCII ordinals>
```

Each version entry has forward/backward migration messages encoded as ASCII ordinal sequences.

### 4.8 Models Section

`Models/Data` contains text records for 3D model references.
`Models/0`, `Models/1`, ..., `Models/N` contain embedded 3D model data (STEP files, etc.) -- these are large binary blobs that don't follow normal block framing.

### 4.9 Coordinate System

PcbDoc uses **mil** (thousandths of an inch) as the primary unit, stored as string values with the "mil" suffix (e.g., `X=4686.0219mil`). Rotation is stored in scientific notation (e.g., `1.80000000000000E+0002` for 180 degrees).

---

## 5. PcbLib Files (PCB Footprint Libraries)

**Files examined:** `LimeMicroAltiumLib_pcbLib.PcbLib`, `Synthiam.PcbLib`, `BlankPcbLibComponent.PcbLib`

### 5.1 Stream Structure

PcbLib files organize footprints similarly to how SchLib organizes components, but with significant differences.

**Top-level system streams/storages:**

| Stream/Storage | Description |
|----------------|-------------|
| `FileHeader` | Binary header: `PCB 6.0 Binary Library File` (length-prefixed string) |
| `FileVersionInfo/Data` | Version compatibility (same format as PcbDoc) |
| `SectionKeys` | Binary stream mapping section names (not text) |
| `Library/` | Library-wide settings storage |

**Library/ sub-streams:**

| Stream | Description |
|--------|-------------|
| `Library/Data` | Board-level properties (like PcbDoc Board6, very large) |
| `Library/Header` | Object count header |
| `Library/EmbeddedFonts` | Embedded font data |
| `Library/LayerKindMapping/Data` | Layer kind mapping (binary, UTF-16LE "1.0" version) |
| `Library/Models/Data` | 3D model reference table |
| `Library/ModelsNoEmbed/Data` | Non-embedded model references |
| `Library/PadViaLibrary/Data` | Pad/via library definitions |
| `Library/Textures/Data` | Texture data |
| `Library/ComponentParamsTOC/Data` | Component parameter table of contents |

**ComponentParamsTOC format** (one block per footprint):
```
Name=PCBComponent_1|Pad Count=0|Height=0|Description=
```

**Per-footprint storage:**
```
<FootprintName>/
    Data                -- Binary primitive objects (same format as PcbDoc sections)
    Header              -- Object count (4 bytes, same as PcbDoc headers)
    Parameters          -- Text: PATTERN, HEIGHT, DESCRIPTION, ITEMGUID, REVISIONGUID
    WideStrings         -- Wide string table (usually single null byte for empty)
    UniqueIDPrimitiveInformation/
        Data            -- Unique ID mapping for primitives
        Header          -- Count
    PrimitiveGuids/     -- (Synthiam PcbLib only, newer format)
        Data            -- GUID assignment per primitive
        Header          -- Count
```

### 5.2 Per-Footprint Data Stream

The `Data` stream format is:
```
[u32 pattern_name_len][pattern_name_bytes][u8 type_id][subrecords...]...
```

The first element is a **Pascal-style length-prefixed string** containing the footprint pattern name. Then PCB primitives follow in the same binary format as PcbDoc primitive sections.

**Example primitive type distributions per footprint:**

| Footprint | Primitives |
|-----------|------------|
| `Fiducial` | 1 arc, 1 pad |
| `RES0402` | 2 pads, 16 tracks, 1 text, 1 body |
| `SOT23-3` | 3 arcs, 3 pads, ~16 tracks, 1 text, 1 body |
| `10M16SAU169C8G` | 2 arcs, 169 pads, many tracks, texts, bodies |
| `WurthElectronic_RF_shield` | 5 component bodies only |

### 5.3 Parameters Stream

One text block per footprint:
```
|PATTERN=RES0402|HEIGHT=15.748mil|DESCRIPTION=Chip Resistor...|ITEMGUID=<guid>|REVISIONGUID=<guid>
```

### 5.4 Differences: LimeMicro vs Synthiam PcbLib

| Feature | LimeMicro | Synthiam |
|---------|-----------|----------|
| Storages | 566 | 1,448 |
| Streams | 1,817 | 3,863 |
| Per-footprint streams | Data, Header, Parameters, UniqueID, WideStrings | Data, Header, Parameters, UniqueID, WideStrings, **PrimitiveGuids** |
| PrimitiveGuids | Not present | Present (binary GUID data) |
| Models | 120+ embedded 3D models in Library/Models/ | Embedded models in Library/Models/ |

The `PrimitiveGuids` storage appears to be a newer AD feature for tracking individual primitives by GUID.

### 5.5 BlankPcbLibComponent.PcbLib (Minimal File)

Only 1 footprint (`PCBComponent_1`) with zero primitives:
- `PCBComponent_1/Data`: 19 bytes (just the pattern name string, length=14, "PCBComponent_1", plus 1 byte)
- `PCBComponent_1/Parameters`: `|PATTERN=PCBComponent_1|HEIGHT=0mil|DESCRIPTION=|ITEMGUID=|REVISIONGUID=`
- `Library/Data`: 94,588 bytes of board configuration (default layer stack, etc.)

---

## 6. PrjPcb Files (Project Files)

**File examined:** `BlankProject.PrjPcb`

### 6.1 Format

**Not an OLE file.** Plain text INI format with UTF-8 BOM.

```ini
[Design]
Version=1.0
HierarchyMode=0
ChannelRoomNamingStyle=0
...

[Preferences]
PrefsVaultGUID=
PrefsRevisionGUID=

[Document1]
DocumentPath=BlankSchlibComponent.SchLib
AnnotationEnabled=1
DocumentUniqueId=IIEGGIJT

[Document2]
DocumentPath=BlankPcbLibComponent.PcbLib
DocumentUniqueId=RTJRBTLE

[Configuration1]
Name=Sources
ContentTypeGUID=CB6F2064-E317-11DF-B822-12313F0024A2
ConfigurationType=Source

[OutputGroup1]
Name=Netlist Outputs
...
```

Sections include:
- `[Design]`: Global project settings
- `[Preferences]`: User preferences
- `[DocumentN]`: Project documents (paths, annotation settings)
- `[ConfigurationN]`: Build configurations
- `[OutputGroupN]`: Output job definitions

---

## 7. Block Framing Summary

All OLE-based Altium files use a common **size-prefixed block** framing:

```
[u32 header][payload bytes]
```

Header encoding:
- **Bits 0-23** (3 bytes): Payload size in bytes (little-endian)
- **Bits 24-31** (1 byte): Flags

| Flags | Meaning |
|-------|---------|
| `0x00` | Normal text or binary record |
| `0x01` | Compressed payload (zlib) |
| other | Some binary sub-record types use non-zero flags |

### 7.1 Compressed Block Payload

When flags=0x01, the payload contains:
```
[0xD0][u8 id_len][id_string][u32 inner_header][compressed_data]
```

- `0xD0` magic byte
- Length-prefixed identifier string (file path or index like "0", "1")
- Inner block header (same 24-bit size / 8-bit flags format)
- Zlib compressed data (sometimes with 2-byte skip before raw deflate)

---

## 8. Text Record Format Summary

### 8.1 Pipe-Delimited Key=Value

```
|KEY1=VALUE1|KEY2=VALUE2|...\0
```

Rules:
- Leading `|` before first key
- `=` separates key from value
- `|` separates key=value pairs
- Trailing null byte (`\x00`) common but not universal
- Keys are case-insensitive for matching but preserved as-is
- Values may contain special characters, paths, scientific notation
- `%UTF8%` prefix on keys indicates UTF-8 encoded values re-encoded through Windows-1252

### 8.2 Schematic Records

Identified by `RECORD=N` key. Common keys:
- `OwnerIndex`: Index of parent record (0-based)
- `OwnerPartId`: Part number for multi-part components (-1 = all parts)
- `IndexInSheet`: Position in sheet record list
- `UniqueID`: 8-char alphanumeric identifier
- `Location.X`, `Location.Y`: Coordinates (integer, in Altium internal units)
- `Location.X_Frac`, `Location.Y_Frac`: Fractional coordinate extensions
- `Color`: 24-bit RGB color as integer
- `FontID`: Reference to font table in RECORD=31

### 8.3 PCB Records

No `RECORD=N` key. Section determines record type. Common keys:
- `SELECTION`, `LAYER`, `LOCKED`, `KEEPOUT`: Boolean flags
- `X`, `Y`: Coordinates with unit suffix (e.g., `4686.0219mil`)
- `ROTATION`: Scientific notation angle
- `PATTERN`: Footprint reference name
- `NAME`: Net name, rule name, etc.
- `UNIQUEID`: 8-char alphanumeric identifier
- `UNIONINDEX`: Component grouping index

---

## 9. Key Differences: Library vs Document Files

| Aspect | Document (SchDoc/PcbDoc) | Library (SchLib/PcbLib) |
|--------|--------------------------|------------------------|
| OLE structure | Flat (no storages) or section-based storages | Component-per-storage hierarchy |
| Component data | All records in one stream (FileHeader for SchDoc) | Each component in its own storage |
| Indexing | Record index in `FileHeader` block sequence | `FileHeader` contains component index table |
| PCB primitives | Section-based (`Arcs6/Data`, `Pads6/Data`, etc.) | Per-footprint `Data` stream with pattern name prefix |
| Sidecar streams | PcbDoc: UniqueIDPrimitiveInformation, WideStrings6, PrimitiveParameters | PcbLib: UniqueIDPrimitiveInformation, WideStrings, PrimitiveGuids per footprint |
| Board config | `Board6/Data` | `Library/Data` |
| Models | `Models/Data` + `Models/N` | `Library/Models/Data` + `Library/Models/N` |

---

## 10. Aggregate Coverage Analysis

Scanning all 18 files in `data/`:

### Schematic Record IDs (all observed, all implemented):
```
1, 2, 4, 5, 6, 7, 8, 9, 11, 12, 13, 14, 17, 22, 25, 27, 28, 29, 30, 31,
34, 39, 41, 43, 44, 45, 46, 47, 48, 209, 225
```

Total: 31 distinct RECORD IDs. **Zero coverage gaps** vs implemented IDs.

### PCB Object IDs (all observed, all implemented):
```
1 (Arc), 2 (Pad), 3 (Via), 4 (Track), 5 (Text), 6 (Fill),
11 (Region), 12 (ComponentBody)
```

Total: 8 distinct object IDs. **Zero coverage gaps** vs observed data.

PCB IDs in docs/model but not yet observed in test data: 10, 13, 14

### Record Counts Across All Files

| Schematic | Count | PCB | Count |
|-----------|-------|-----|-------|
| RECORD=41 (Parameter) | 17,713 | ID=4 (Track) | 21,819 |
| RECORD=2 (Pin) | 3,777 | ID=2 (Pad) | 16,011 |
| RECORD=6 (Polyline) | 2,652 | ID=12 (Body) | 2,480 |
| RECORD=14 (Rectangle) | 949 | ID=5 (Text) | 1,966 |
| RECORD=45 (Impl Child) | 911 | ID=11 (Region) | 1,668 |
| RECORD=46 (Pin Map) | 911 | ID=3 (Via) | 1,262 |
| RECORD=48 (Impl Params) | 911 | ID=1 (Arc) | 830 |
| RECORD=27 (Wire) | 891 | ID=6 (Fill) | 134 |
| RECORD=1 (Component) | 815 | | |
| RECORD=34 (Designator) | 815 | | |

---

## 11. Encoding Notes

- **Text encoding**: Windows-1252 (Latin-1 superset) is the primary encoding
- **UTF-8 values**: Prefixed with `%UTF8%` on the key name; value bytes are Windows-1252 re-encoding of UTF-8
- **Unicode strings**: PcbDoc uses `WideStrings6/Data` for wide (UTF-16) text; PcbLib uses per-footprint `WideStrings`
- **Coordinates**: SchDoc uses integer internal units with optional `_Frac` suffix for sub-unit precision; PcbDoc uses string values with `mil` suffix
- **Colors**: Stored as 24-bit integers in BGR format (e.g., `8388608` = 0x800000 = dark red)
- **Booleans**: `T`/`F` in schematic records; `TRUE`/`FALSE` in PCB records
- **UniqueIDs**: 8-character strings from alphabet `ABCDEFGHIJKLMNOPQRSTUVWXYZ` (uppercase only)
