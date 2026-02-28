# CFB Structure

PcbDoc files use the OLE Compound Binary (CFB / Structured Storage) format. Unlike PcbLib,
which has per-footprint sub-storages, a PcbDoc is a flat collection of section storages at
the root level -- one per primitive type, plus metadata and sidecar storages.

```
Root Storage
 |
 +-- FileHeader                          (root stream: V5 format identification, UTF-16LE)
 +-- FileHeaderSix                       (root stream: V6 extended header, Win1252)
 |
 +-- Board6/                             (storage: board-level settings and metadata)
 |    +-- Header                         (stream: u32 record count)
 |    +-- Data                           (stream: parameter blocks)
 |
 +-- Arcs6/                              (storage: arc primitives)
 |    +-- Header                         (stream: u32 record count)
 |    +-- Data                           (stream: binary primitive records)
 +-- Pads6/                              (storage: pad primitives)
 |    +-- Header / Data
 +-- Vias6/                              (storage: via primitives)
 |    +-- Header / Data
 +-- Tracks6/                            (storage: track primitives)
 |    +-- Header / Data
 +-- Texts6/                             (storage: text primitives)
 |    +-- Header / Data
 +-- Fills6/                             (storage: fill primitives)
 |    +-- Header / Data
 +-- Regions6/                           (storage: region primitives)
 |    +-- Header / Data
 +-- ShapeBasedRegions6/                 (storage: shape-based region primitives)
 |    +-- Header / Data
 +-- ComponentBodies6/                   (storage: component body primitives)
 |    +-- Header / Data
 +-- ShapeBasedComponentBodies6/         (storage: shape-based component body primitives)
 |    +-- Header / Data
 +-- BoardRegions/                       (storage: board region primitives)
 |    +-- Header / Data
 +-- Texts/                              (storage: legacy text primitives)
 |    +-- Header / Data
 |
 +-- Nets6/                              (storage: net definitions)
 |    +-- Header / Data                  (parameter blocks)
 +-- Components6/                        (storage: component instances)
 |    +-- Header / Data                  (parameter blocks)
 +-- Polygons6/                          (storage: polygon pour definitions)
 |    +-- Header / Data                  (parameter blocks)
 +-- Classes6/                           (storage: object class definitions)
 |    +-- Header / Data                  (parameter blocks)
 +-- DifferentialPairs6/                 (storage: differential pair definitions)
 |    +-- Header / Data                  (parameter blocks)
 +-- FromTos6/                           (storage: ratsnest definitions)
 |    +-- Header / Data                  (parameter blocks)
 +-- Connections6/                       (storage: connection/fromto primitives)
 |    +-- Header / Data                  (parameter blocks)
 +-- EmbeddedBoards6/                    (storage: embedded board array definitions)
 |    +-- Header / Data                  (parameter blocks)
 +-- Embeddeds6/                         (storage: embedded objects)
 |    +-- Header / Data                  (parameter blocks)
 |
 +-- Rules6/                             (storage: design rules)
 |    +-- Header / Data                  (prefixed parameter blocks: u16 + u32 len + params)
 +-- Dimensions6/                        (storage: dimension annotations)
 |    +-- Header / Data                  (prefixed parameter blocks)
 +-- Coordinates6/                       (storage: coordinate annotations)
 |    +-- Header / Data                  (prefixed parameter blocks)
 |
 +-- Models/                             (storage: 3D model data)
 |    +-- Header                         (stream: u32 count of model entries)
 |    +-- Data                           (stream: model metadata parameter blocks)
 |    +-- 0                              (stream: zlib-compressed STEP model data)
 |    +-- 1                              (stream: zlib-compressed STEP model data)
 |    +-- ...                            (one stream per embedded 3D model)
 |
 +-- WideStrings6/                       (storage: unicode string sidecar, binary TLV format)
 |    +-- Header / Data
 +-- UniqueIDPrimitiveInformation/       (storage: per-primitive unique IDs, parameter blocks)
 |    +-- Header / Data
 +-- ExtendedPrimitiveInformation/       (storage: per-primitive extended properties)
 |    +-- Header / Data
 +-- PrimitiveParameters/                (storage: primitive parameter overrides)
 |    +-- Header / Data
 |
 +-- EmbeddedFonts6/                     (storage: embedded font binary data)
 |    +-- Header / Data
 +-- FileVersionInfo/                    (storage: file version history)
 |    +-- Header / Data
 +-- LayerKindMapping/                   (storage: mechanical layer kind mapping)
 |    +-- Header / Data
 +-- Textures/                           (storage: texture image data)
 |    +-- Header / Data
 +-- ModelsNoEmbed/                      (storage: references to non-embedded models)
 |    +-- Header / Data
 +-- PadViaLibrary/                      (storage: pad/via template library)
 |    +-- Header / Data
 +-- PadViaLibraryCache/                 (storage: pad/via template cache)
 |    +-- Header / Data
 +-- PadViaLibraryLinks/                 (storage: pad/via template links)
 |    +-- Header / Data
 +-- PinPairsSection/                    (storage: pin pair definitions)
 |    +-- Header / Data
 +-- SignalClasses/                      (storage: signal class definitions)
 |    +-- Header / Data
 +-- SmartUnions/                        (storage: smart union definitions)
 |    +-- Header / Data
 +-- UnionNames/                         (storage: union name strings)
 |    +-- Header / Data
 +-- WaivedViolations/                   (storage: waived DRC violations)
 |    +-- Header / Data
 +-- Advanced Placer Options6/           (storage: auto-placer settings)
 |    +-- Header / Data
 +-- Design Rule Checker Options6/       (storage: DRC settings)
 |    +-- Header / Data
 +-- Pin Swap Options6/                  (storage: pin-swap settings)
 |    +-- Header / Data
```

Additional sections observed in the 132-file test fixture corpus but not in the LimeSDR files:

```
 +-- SplitPlaneRegions6/                 (storage: split plane region primitives)
 |    +-- Header / Data
 +-- UnionRelations/                     (storage: union relation mappings, i32 pairs)
 |    +-- Header / Data
 +-- ConstraintManager/                  (storage: constraint data, UTF-16LE base64/zlib)
 |    +-- Header / Data
 +-- Advanced Router Options6/           (storage: auto-router settings, param blocks)
 |    +-- Header / Data
 +-- NewRules6/                          (storage: extended design rules, prefixed params)
 |    +-- Header / Data
 +-- PrimitiveGuids/                     (storage: packed 24-byte TPrimitiveGUID records)
 |    +-- Header / Data
 +-- UnionFeatures/                      (storage: union feature flags, param blocks)
 |    +-- Header / Data
 +-- SharedUnion/                        (storage: union reference definitions, param blocks)
 |    +-- Header / Data
 +-- CustomShapes/                       (storage: custom pad shape definitions, param blocks)
 |    +-- Header / Data
 +-- DrillManager/                       (storage: drill configuration, 8-byte prefix + params)
 |    +-- Header / Data
 +-- TClearanceViolation/                (storage: DRC clearance violations, param blocks)
 |    +-- Header / Data
 +-- TShortCircuitViolation/             (storage: DRC short circuit violations, param blocks)
 |    +-- Header / Data
 +-- TSilkToSilkClearanceViolation/      (storage: DRC silk violations, param blocks)
 |    +-- Header / Data
```

**Note on `SharedUnion` vs `SharedUnions`**: These are different storages. `SharedUnions`
(with trailing 's') contains union definitions in a binary format parsed by
`parse_shared_union_stream()`. `SharedUnion` (no trailing 's') contains param-block
records with `|PRIMITIVEINDEX=N|OBJECTID=Pad|PRIMITIVECOUNT=N|...` fields.


## Stream inventory from real files

Verified against two PcbDoc files from the LimeSDR project:

| Storage | LimeSDR board | LimeSDR panel |
|---------|:------------:|:-------------:|
| FileHeader | 24 bytes | 24 bytes |
| FileHeaderSix | 75 bytes | 75 bytes |
| Board6/{Header,Data} | 103,358 | 101,388 |
| Arcs6/{Header,Data} | 16,900 | 975 |
| Pads6/{Header,Data} | 692,347 | 1,712 |
| Vias6/{Header,Data} | 263,541 | 0 |
| Tracks6/{Header,Data} | 686,340 | 133,326 |
| Texts6/{Header,Data} | 345,417 | 39,178 |
| Fills6/{Header,Data} | 110 | 0 |
| Regions6/{Header,Data} | 1,063,894 | 3,255 |
| ShapeBasedRegions6/{Header,Data} | 1,527,801 | 5,070 |
| ComponentBodies6/{Header,Data} | 652,351 | 0 |
| ShapeBasedComponentBodies6/{Header,Data} | 746,247 | 0 |
| BoardRegions/{Header,Data} | 966 | 390 |
| Texts/{Header,Data} | 928 | 928 |
| Nets6/{Header,Data} | 304,949 | 254,054 |
| Components6/{Header,Data} | 272,154 | 3,578 |
| Polygons6/{Header,Data} | 33,469 | 0 |
| Classes6/{Header,Data} | 25,125 | 16,284 |
| DifferentialPairs6/{Header,Data} | 2,749 | 2,797 |
| FromTos6/{Header,Data} | 0 | 0 |
| Connections6/{Header,Data} | 0 | 0 |
| EmbeddedBoards6/{Header,Data} | 0 | 1,077 |
| Embeddeds6/{Header,Data} | 0 | 0 |
| Rules6/{Header,Data} | 40,639 | 39,875 |
| Dimensions6/{Header,Data} | 5,171 | 5,164 |
| Coordinates6/{Header,Data} | 0 | 0 |
| Models/{Header,Data,0..N} | 7,565 + 44 models | 0 (no models) |
| WideStrings6/{Header,Data} | 34,824 | 11,532 |
| UniqueIDPrimitiveInformation/{Header,Data} | 114,005 | 496 |
| ExtendedPrimitiveInformation/{Header,Data} | 155 | 0 |
| PrimitiveParameters/{Header,Data} | 602,562 | 0 |
| EmbeddedFonts6/{Header,Data} | 938,699 | 938,699 |
| FileVersionInfo/{Header,Data} | 22,119 | 22,119 |
| LayerKindMapping/{Header,Data} | 20 | 20 |
| Textures/{Header,Data} | 0 | 0 |
| ModelsNoEmbed/{Header,Data} | 0 | 0 |
| PadViaLibrary/{Header,Data} | 131 | 131 |
| PadViaLibraryCache/{Header,Data} | 131 | 131 |
| PadViaLibraryLinks/{Header,Data} | 0 | 0 |
| PinPairsSection/{Header,Data} | 0 | 0 |
| SignalClasses/{Header,Data} | 220 | 220 |
| SmartUnions/{Header,Data} | 6,248 | 0 |
| UnionNames/{Header,Data} | 5,938 | 30 |
| WaivedViolations/{Header,Data} | 0 | 0 |
| Advanced Placer Options6/{Header,Data} | 208 | 208 |
| Design Rule Checker Options6/{Header,Data} | 719 | 621 |
| Pin Swap Options6/{Header,Data} | 294 | 294 |

Data sizes shown are for the Data stream only (all Header streams are exactly 4 bytes).

**Key observations:**
- Both test files contain the same 42 section storages plus 2 root streams
- `EmbeddedBoards6` has data only in the panel file (which uses embedded board arrays)
- `Models/` contains 44 numbered model streams in the board file but none in the panel
- `EmbeddedFonts6` is identical in both files (938,699 bytes) -- likely the same embedded font set
- Many sections have empty (0-byte) Data streams but are still present in the CFB container
- Neither test file contains `SplitPlaneRegions6`, `UnionRelations`, `ConstraintManager`,
  `Advanced Router Options6`, `NewRules6`, or `PrimitiveGuids`


## Root streams

PcbDoc has exactly two root-level streams (not inside any storage):

### FileHeader

The V5 format identification stream. Contains a length-prefixed UTF-16LE string:

```
[4 bytes]  u32 LE: character count (NOT byte count)
[N*2 bytes] UTF-16LE string: "PCB 5.0 Binary File"
```

In our test file: `13 00 00 00` = 19 characters, followed by UTF-16LE "PCB 5.0 Binary File"
(the stream is 24 bytes = 4-byte length + 19*2 - 14 visible bytes, matching the 24-byte
stream size).

This stream is used by `RecognizeFile()` to identify the format as
`eAdvPCBFormat_Binary_V5` or higher.

### FileHeaderSix

The V6 extended header stream. Uses Win1252 encoding:

```
[4 bytes]  u32 LE: block length
[1 byte]   u8: header text length
[N bytes]  Win1252 header text: "PCB 6.0 Binary File"
[8 bytes]  f64 LE: file format version number (e.g., 5.01)
[4 bytes]  u32 LE: key block length
[1 byte]   u8: key length
[N bytes]  Win1252 key token (GUID string, e.g., "{C1EF2D32-6634-4C5A-A58E-5AF8D518C64E}")
```

In our test file:
- Header text: `"PCB 6.0 Binary File"` (19 bytes)
- Version: 5.01 (f64 LE: `0a d7 a3 70 3d 0a 14 40`)
- Key token: `"{C1EF2D32-6634-4C5A-A58E-5AF8D518C64E}"` (38 bytes)

The `FileHeaderSix` stream is present only in V6-format files. Its presence indicates the
file uses the modern section layout with all the `*6` sections. The key token is an
opaque identifier (not a cryptographic key).

### FileHeader vs FileHeaderSix

| Property | FileHeader | FileHeaderSix |
|----------|-----------|---------------|
| Encoding | UTF-16LE | Win1252 |
| Length prefix | u32 character count | u32 byte count + u8 string length |
| Version string | "PCB 5.0 Binary File" | "PCB 6.0 Binary File" |
| Additional data | None | f64 version + GUID key token |
| Required | Yes | Yes (V6 format) |

Both streams are read during format recognition. The loader checks FileHeader first to
confirm it is a PCB binary file, then reads FileHeaderSix for the V6 version and key.


## Header/Data stream pattern

Every section storage follows the same **Header + Data** pattern:

- **Header**: Always exactly 4 bytes -- a `u32` little-endian record count
- **Data**: Variable-length payload whose format depends on the section type (see below)

This pattern is shared with PcbLib and uses the same section-reading infrastructure.


## Section classification

Each section falls into one of five categories based on its Data stream format.

### 1. Primitive binary sections

These contain packed binary primitive records. Format per record:

```
[1 byte]  u8: TObjectId type byte
[4 bytes] u32 LE: record payload length
[N bytes] binary payload (fixed-size struct + optional variable-length data)
```

| Section Name | Object ID | Primitive Type |
|-------------|-----------|----------------|
| Arcs6 | 1 (eArcObject) | Arc |
| Pads6 | 2 (ePadObject) | Pad |
| Vias6 | 3 (eViaObject) | Via |
| Tracks6 | 4 (eTrackObject) | Track |
| Texts6 | 5 (eTextObject) | Text |
| Fills6 | 6 (eFillObject) | Fill |
| Regions6 | 11 (eRegionObject) | Region |
| ShapeBasedRegions6 | 11 (eRegionObject) | Region (shape-based variant) |
| ComponentBodies6 | 12 (eComponentBodyObject) | Component body |
| ShapeBasedComponentBodies6 | 12 (eComponentBodyObject) | Component body (shape-based) |
| BoardRegions | 11 (eRegionObject) | Board region (legacy) |
| Texts | 5 (eTextObject) | Text (legacy) |

**Variant sections:** `ShapeBasedRegions6` vs `Regions6` and `ShapeBasedComponentBodies6` vs
`ComponentBodies6` share the same object ID but use different binary layouts. The
`TStorageFeature` flags `eHasShapeBasedRegions` and `eHasShapeBasedCompBodies` indicate which
pair is active. In modern files, both the legacy and shape-based sections are present; the
shape-based version contains the authoritative data.

### 2. Parameter sections

These contain concatenated parameter blocks:

```
[4 bytes] u32 LE: length of parameter string (including NUL terminator)
[N bytes] Win1252, NUL-terminated, pipe-delimited: |KEY1=VALUE1|KEY2=VALUE2|
```

| Section Name | Content |
|-------------|---------|
| Board6 | Board-level settings and metadata |
| Nets6 | Net definitions |
| Components6 | Component instances |
| Polygons6 | Polygon pour definitions |
| Classes6 | Object class definitions |
| DifferentialPairs6 | Differential pair definitions |
| FromTos6 | FromTo/ratsnest definitions |
| Connections6 | Connection primitives |
| EmbeddedBoards6 | Embedded board array definitions |
| Embeddeds6 | Embedded objects |
| UniqueIDPrimitiveInformation | Per-primitive unique IDs (sidecar) |
| ExtendedPrimitiveInformation | Per-primitive extended properties (sidecar) |
| PrimitiveParameters | Primitive parameter overrides |
| PadViaLibrary | Pad/via template library |
| PadViaLibraryCache | Pad/via template cache |
| PadViaLibraryLinks | Pad/via template links |
| PinPairsSection | Pin pair definitions |
| SignalClasses | Signal class definitions |
| SmartUnions | Smart union definitions |
| WaivedViolations | Waived DRC violations |
| Advanced Placer Options6 | Auto-placer settings |
| Design Rule Checker Options6 | DRC settings |
| Pin Swap Options6 | Pin-swap settings |

### 3. Prefixed parameter sections

These contain parameter blocks with a 2-byte prefix before each record:

```
[2 bytes] u16 LE: prefix word (section-specific interpretation)
[4 bytes] u32 LE: length of parameter string
[N bytes] Win1252, NUL-terminated, pipe-delimited parameter string
```

| Section Name | Content |
|-------------|---------|
| Rules6 | Design rules |
| Dimensions6 | Dimension annotations |
| Coordinates6 | Coordinate annotations |

(The `NewRules6` section also uses this format but was not present in test files.)

### 4. Raw binary sections

These contain a Data stream whose format is section-specific (not the standard
parameter-block or binary-primitive framing):

| Section Name | Content |
|-------------|---------|
| WideStrings6 | Unicode string table (indexed binary TLV format) |
| EmbeddedFonts6 | Embedded font binary data |
| FileVersionInfo | File version history parameter blocks |
| LayerKindMapping | Mechanical layer kind mapping |
| Textures | Texture image data |
| ModelsNoEmbed | References to non-embedded models |
| UnionNames | Union name strings |

### 5. Models section (special)

The `Models/` storage has a unique structure with numbered sub-streams for each
embedded 3D model blob:

| Stream | Content |
|--------|---------|
| Models/Header | u32 count of model metadata records |
| Models/Data | Model metadata as parameter blocks (u32 len + pipe-delimited params) |
| Models/0 | First model binary blob (typically zlib-compressed STEP data) |
| Models/1 | Second model binary blob |
| Models/N | Nth model binary blob |

The model metadata in `Models/Data` uses the same parameter-block format as other parameter
sections. Each block contains keys like `EMBED=TRUE`, `MODELSOURCE=Undefined`,
`ID={GUID}`, `ROTX`, `ROTY`, `ROTZ`, `DZ`, `CHECKSUM`, `NAME`.


## Differences from PcbLib structure

| Feature | PcbDoc | PcbLib |
|---------|--------|--------|
| Organization | Flat: all sections at root | Hierarchical: Library/ + per-footprint storages |
| FileHeader encoding | UTF-16LE, u32 char count | Win1252, u8 string length |
| FileHeader text | "PCB 5.0 Binary File" | "PCB 6.0 Binary Library File" |
| FileHeaderSix | Root stream | Not present (library header is in FileHeader) |
| Section naming | Sections at root (e.g., `/Arcs6/`) | Under Library/ or per-footprint (e.g., `/<name>/Data`) |
| Board6 section | Board settings + layer stack | Under `Library/{Header,Data}` |
| Models location | `/Models/` at root | `/Library/Models/` |
| WideStrings format | Binary TLV (`WideStrings6`) | Parameter-block (`WideStrings` per footprint) |
| WideStrings scope | Board-wide, one shared table | Per-footprint, separate per component |
| Primitives | One section per type, board-wide | Per-footprint Data stream with all types packed |
| ComponentParamsTOC | Not present | Under `Library/ComponentParamsTOC/` |
| SectionKeys | Not present (no name-length issues) | Optional (when footprint names > 31 chars) |
| EmbeddedFonts | Own section (`EmbeddedFonts6/`) | Under `Library/EmbeddedFonts` (no Header/Data) |
| Sidecar streams | Board-wide sections at root | Per-footprint sub-storages |
| Parameters stream | Not present | Per-footprint `Parameters` stream |


## Optional vs required sections

All 42 section storages observed in our test files are always present, even when their
Data stream is empty (0 bytes). This suggests Altium always creates the full set of
sections when saving a PcbDoc file. The empty sections (like `FromTos6`, `Connections6`,
`Coordinates6`, `Embeddeds6`, etc.) have a Header of `00 00 00 00` (count = 0) and an
empty Data stream.

Sections NOT present in either test file but documented in the .NET code:

| Section | Likely conditions |
|---------|-------------------|
| SplitPlaneRegions6 | Files with split power/ground planes |
| UnionRelations | Files with union relation mappings |
| ConstraintManager | Files with constraint manager data |
| Advanced Router Options6 | Files with auto-router settings |
| NewRules6 | Files with extended design rules |
| PrimitiveGuids | Older files (mutually exclusive with UniqueIDPrimitiveInformation in some cases) |

The loader discovers sections dynamically by enumerating CFB directory entries and matching
storage names. Unknown sections are handled by `IsSectionToIgnore()`.
