# Sidecar Streams Deep Dive

Reverse-engineered from Altium Designer 26 via decompiled .NET code
(`AD26-dotnet/`) and Ghidra analysis of Delphi DLLs (`altium26` project).

---

## 1. Architectural Overview

Altium stores supplementary primitive data in **sidecar streams** -- separate
OLE/CFB streams that extend the main record data. In Altium's runtime model
these streams are purely a **serialization concern**: on load the sidecar data
is merged into the primitive objects; on save it is split back out. The split
exists for backwards compatibility -- new fields were added via new streams so
older readers could ignore them without choking.

### Key principle

> There is **no distinction** in Altium's runtime model between "core record
> data" and "sidecar data." The primitive object has all fields directly. The
> sidecar streams are format-evolution artifacts.

### Architectural pattern across all document types

1. **Main data stream first** (`FileHeader` / `Data`) -- core records that
   older Altium versions understand.
2. **Global sidecars second** -- streams that apply across all primitives
   (WideStrings, UniqueIDs, ExtendedPrimitiveInfo).
3. **Per-record sidecars last** -- streams scoped to a specific component
   or footprint (pin sidecars in SchLib, PrimitiveGuids in PcbLib).

Sidecars always **merge into** the runtime objects. They never stand alone.

---

## 2. Complete Stream Load Orders

### 2.1 SchLib Load Order

From `SchDataImporterLibraryV5.Run()` (lines 49-62). The SchLib importer does
**not** inherit from `SchDataImporterBaseV5` -- it has its own `Run()` method.

| #  | Stream Path | Purpose |
|----|---|---|
| 1  | `/FileHeader` | Library header (Weight, MinorVersion, UniqueID), library-level objects |
| 2  | `/SectionKeys` | Component display name to CFB section key mapping |
| 3  | `/<key>/Data` (x N) | Per-component records (FindFirstStream loop over all sections) |
| 4  | `/Storage` | Embedded binary blobs (images, icons -- `SchDataEmbeddedObject` format) |
| 5  | `/<key>/PinFrac` (x N) | Pin fractional coordinates |
| 6  | `/<key>/PinDesc` (x N) | Pin long descriptions (ASCII overflow) |
| 7  | `/<key>/PinMiscData` (x N) | Pin swap ID pairs |
| 8  | `/<key>/PinTextData` (x N) | Pin custom name/designator text display |
| 9  | `/<key>/PinWideText` (x N) | Pin Unicode text (authoritative replacement) |
| 10 | `/<key>/PinSymbolLineWidth` (x N) | Pin symbol line width |
| 11 | `/<key>/PinPackageLength` (x N) | Pin package length |
| 12 | `/<key>/PinPropagationDelay` (x N) | Pin signal propagation delay |
| 13 | `/<key>/PinFunctionData` (x N) | Pin alternate functions |
| 14 | `/LibAdditional` | Library-level additional warehouse header |
| 15 | `/<key>/Additional` (x N) | Per-component additional objects |

Steps 5-13 are the **pin sidecar streams**, all read inside
`ReadAndProcessPinsExtendedData()` (lines 455-511). For each component,
`ReadPinsExtendedData()` (lines 513-540) checks `StreamExists(key, streamName)`,
opens the stream, reads the embedded object header (RECORD, HEADER, Weight),
then reads `Weight` blobs (instruction byte `0xD0`). Streams that don't exist
are silently skipped.

**Method chain:**
```
Run()
  -> ImportBaseWarehouse()
       -> ReadBaseWarehouse()          -- steps 1-3
       -> ProcessImportedBaseWarehouse()
  -> ImportExtendedWarehouse()
       -> ReadExtendedWarehouse()      -- step 4
       -> ProcessImportedExtendedWarehouse()
       -> ReadAndProcessPinsExtendedData()  -- steps 5-13
  -> ImportAdditionalWarehouse()
       -> ReadAdditionalWareHouse()    -- steps 14-15
  -> UpdateDocumentAfterImport()       -- fires SchDataAfterImportDocumentEvent
```

### 2.2 SchDoc Load Order

From `SchDataImporterSheetV5` (extends `SchDataImporterDocumentV5` extends
`SchDataImporterBaseV5`). Pin sidecar streams do **not** exist in SchDoc --
all pin data is stored inline in `FileHeader` records.

| # | Stream | Purpose |
|---|---|---|
| 1 | `FileHeader` | All sheet objects: components, wires, labels, pins, everything inline |
| 2 | `Storage` | Embedded binary blobs (images -- `SchDataEmbeddedObject` format) |
| 3 | `ReuseBlocks` | Design reuse block info V1 (vault GUIDs, part ID mappings) |
| 4 | `ReuseBlocksV2` | Extends V1 with PCB snippet vault/item/revision references |
| 5 | `HarnessConnectionPointConnector` | Harness connector-to-pin mappings |
| 6 | `Additional` | Additional objects (overflow) |
| 7 | `ObjectDefinitions` | Object definition records |
| 8 | `ReuseBlockInfos` | Dissolved reuse block implementation info |

**Method chain (SchDataImporterBaseV5.Run, lines 33-54):**
```
Run()
  -> ImportBaseWarehouse()          -- step 1
       -> ReadBaseWarehouse()
       -> ProcessImportedBaseWarehouse()
       -> PostProcessImportedBaseWarehouse()
  -> ImportExtendedWarehouse()      -- steps 2-5
       -> ReadExtendedWarehouse()                                    -- step 2
       -> ProcessImportedExtendedWarehouse()
       -> ReadAndProcessReuseBlockInfoList()                         -- steps 3-4
       -> ReadAndProcessHarnessLayoutConnectionPointConnectorsData() -- step 5
  -> ImportAdditionalWarehouse()    -- step 6
  -> ImportDefinitionWarehouse()    -- step 7
  -> ImportReuseBlockInfo()         -- step 8
  -> ImportFilesWarehouse()         -- no-op for SchDoc
  -> UpdateAfterImport()
  -> FinalizeForLoading()           -- MoveSpecialObjectsToTop()
```

### 2.3 PcbDoc Load Order

From `TPCBBinaryFile::RegisterAllSectionsForExporting` (Ghidra,
`BinaryLoader.dll` at `0x01918020`). 23 primary sections are registered,
each with `Header` + `Data` sub-streams, with a `6` suffix appended at
runtime for version-6 format files.

**Primary sections (in registration order):**

| #  | Display Name | CFB Storage Name | Delphi Class |
|----|---|---|---|
| 1  | PCB 4.0 Binary File | (file header) | TSection |
| 2  | ECO Options | ECO Options6 | TBoardSection |
| 3  | Output Options | Output Options6 | TOutputSection |
| 4  | Printer Options | Printer Options6 | TPrinterSection |
| 5  | Gerber Options | Gerber Options6 | TGerberSection |
| 6  | Advanced Placer Options | Advanced Placer Options6 | TAdvancedPlacerSection |
| 7  | Design Rule Checker Options | DRC Options6 | TDesignRuleCheckerSection |
| 8  | Classes | Classes6 | TClassesSection |
| 9  | Nets | Nets6 | TNetsSection |
| 10 | Components | Components6 | TComponentsSection |
| 11 | Polygons | Polygons6 | TPolygonsSection |
| 12 | Dimensions | Dimensions6 | TDimensionsSection |
| 13 | Coordinates | Coordinates6 | TCoordinatesSection |
| 14 | Connections | Connections6 | TConnectionsSection |
| 15 | Rules | Rules6 | TRulesSection |
| 16 | FromTos | FromTos6 | TRulesSection |
| 17 | Embeddeds | Embeddeds6 | TEmbeddedsSection |
| 18 | Arcs | Arcs6 | TArcsSection |
| 19 | Pads | Pads6 | TPadsSection |
| 20 | Vias | Vias6 | TViasSection |
| 21 | Tracks | Tracks6 | TTracksSection |
| 22 | Texts | Texts6 | TTextsSection |
| 23 | Fills | Fills6 | TFillsSection |

Sections 1-7 are option/config sections. Sections 8-23 are data sections.
Primitive sections (18-23) use the `FUN_0190cd70` constructor variant;
parameter sections (8-16) use the `FUN_0190b250` constructor variant.

**Global sidecar streams (loaded after all primitives via separate dispatch):**

| Stream | Delphi Class | Purpose |
|---|---|---|
| `WideStrings6` | `TWideStringsSection` | Binary TLV Unicode string table |
| `UniqueIDPrimitiveInformation` | `TUniqueIDPrimitiveInformationSection` | Per-primitive identity strings for ECO sync |
| `ExtendedPrimitiveInformation` | `TExtendedPrimitiveInformationSection` | Per-primitive mask expansion overrides |
| `ExtendedPrimitiveIndices` | (part of above) | Fast lookup index into ExtendedPrimitiveInformation |

Sidecar streams are loaded via `ApplyGUIDs` and `ApplyExtendedIndices` COM
dispatch calls on `IPCB_BinarySection` -- after all primitive records are loaded.

### 2.4 PcbLib Load Order

From `TPCBLibraryBinaryFile::RegisterAllSectionsForExporting` (Ghidra,
`BinaryLoader.dll` at `0x01919170`).

| # | Stream Path | Purpose |
|---|---|---|
| 1 | `/` (file header) | Library header (`PCB 4.0 Binary Library File`) |
| 2..N | `/<PatternName>/Data` | Per-footprint packed primitive records |

**Per-footprint sidecar streams (loaded as part of footprint data):**

| Stream | Format | Purpose |
|---|---|---|
| `/<name>/WideStrings` | Parameter blocks | Unicode text (different format than PcbDoc!) |
| `/<name>/PrimitiveGuids/{Header,Data}` | Binary 24 bytes/entry | Persistent GUIDs per primitive |
| `/<name>/UniqueIDPrimitiveInformation/{Header,Data}` | Parameter-block table | Per-primitive unique IDs |
| `/<name>/ExtendedPrimitiveInformation/{Header,Data}` | Parameter-block table | Per-primitive mask expansion overrides |

Loading is via `LoadComponentFromLibraryFile` at `0x01919210`, which iterates
sections, matches by pattern name, then calls `TLibComponentSectionK::Import_FromFile`.

---

## 3. Complete PCB Stream Name Table (88 entries)

Decoded from `Advpcb.dll` addresses `0x03840700`-`0x038415f8` (UTF-16LE Delphi
UnicodeString constants). The same table is replicated in `BinaryLoader.dll` at
the `0x0186a8c0` region.

| Index | Stream Name | Category |
|-------|---|---|
| 0 | `Board` | Board setup |
| 1 | `Advanced Placer Options` | Options |
| 2 | `Advanced Router Options` | Options |
| 3 | `Design Rule Checker Options` | Options |
| 4 | `Pin Swap Options` | Options |
| 5 | `Classes` | Design data |
| 6 | `Nets` | Design data |
| 7 | `Components` | Placement |
| 8 | `Polygons` | Primitives |
| 9 | `Dimensions` | Annotations |
| 10 | `Coordinates` | Annotations |
| 11 | `EmbeddedBoards` | Embedded |
| 12 | `Connections` | Connectivity |
| 13 | `Rules` | Design rules |
| 14 | `NewRules` | Design rules |
| 15 | `FromTos` | Connectivity |
| 16 | `DifferentialPairs` | Connectivity |
| 17 | `Embeddeds` | Embedded |
| 18 | `Arcs` | Primitives |
| 19 | `Pads` | Primitives |
| 20 | `Vias` | Primitives |
| 21 | `Tracks` | Primitives |
| 22 | `Texts` | Primitives |
| 23 | `Fills` | Primitives |
| 24 | `ShapeBasedRegions` | Primitives |
| 25 | `Regions` | Primitives |
| 26 | `ShapeBasedComponentBodies` | Primitives |
| 27 | `ComponentBodies` | Primitives |
| 28 | `Library` | Library |
| 29 | `WideStrings` | **Sidecar** |
| 30 | `EmbeddedFonts` | Resources |
| 31 | `FileVersionInfo` | Metadata |
| 32 | `Models` | 3D models |
| 33 | `ModelsNoEmbed` | 3D models |
| 34 | `SplitPlaneRegions` | Primitives |
| 35 | `Textures` | Resources |
| 36 | `Testpoint Options` | Options |
| 37 | `ExtendedPrimitiveInformation` | **Sidecar** |
| 38 | `ExtendedPrimitiveIndices` | **Sidecar** |
| 39 | `UnionNames` | Unions |
| 40 | `UnionRelations` | Unions |
| 41 | `SmartUnions` | Unions |
| 42 | `BoardRegions` | Regions |
| 43 | `UniqueIDPrimitiveInformation` | **Sidecar** |
| 44 | `ComponentParamsTOC` | Metadata |
| 45 | `LayerStackSection` | Layer stack |
| 46 | `PinPairsSection` | Connectivity |
| 47 | `SignalClasses` | Design data |
| 48 | `PadViaLibrary` | Pad/via defs |
| 49 | `PadViaLibraryCache` | Cache |
| 50 | `PadViaLibraryLinks` | Pad/via defs |
| 51 | `PadViaCacheLibraryLinksSection` | Cache |
| 52 | `ConnectivityGraphCache` | Cache |
| 53 | `ComponentCache` | Cache |
| 54 | `GeometryZeroCache` | Cache |
| 55 | `PrimitiveParameters` | Metadata |
| 56 | `WaivedViolations` | DRC |
| 57 | `LayerKindMapping` | Layer stack |
| 58 | `ConstraintManager` | Design rules |
| 59 | `3DRoutingData` | 3D routing |
| 60 | `3DRoutingXYZData` | 3D routing |
| 61 | `3DRoutingSurfaceData` | 3D routing |
| 62 | `3DRoutingSketchesData` | 3D routing |
| 63 | `MechanicalPrimitives` | Primitives |
| 64 | `CounterHolesSection` | Manufacturing |
| 65 | `CounterHolesPresetsSection` | Manufacturing |
| 66 | `ViaStructureManager` | Via defs |
| 67 | `ViaStructures` | Via defs |
| 68 | `UnionFeatures` | Unions |
| 69 | `CustomShapes` | Pad shapes |
| 70 | `LayerToLayerMapping` | Layer stack |
| 71 | `CustomReliefs` | Thermal relief |
| 72 | `PrimitiveGuids` | **Sidecar** |
| 73 | `LettersGeometry` | Text rendering |
| 74 | `SharedUnion` | Unions |
| 75 | `CustomMaskShapes` | Mask shapes |
| 76 | `RuleAdditionalData` | Design rules |
| 77 | `CornerRadiusChamfer` | Primitives |
| 78 | `DrillManager` | Manufacturing |
| 79 | `xNetClassesSection` | Design data |
| 80 | `Wirebonds` | Wirebond |
| 81 | `WirebondTemplates` | Wirebond |
| 82 | `WirebondBodies` | Wirebond |
| 83 | `DiePadsInfo` | Wirebond |
| 84 | `RegionHoles` | Primitives |
| 85 | `ViaInstancing` | Via defs |
| 86 | `SimbeorCacheSection` | Simulation |
| 87 | `ZAxisClearanceCache` | Cache |

Version-6 variants (`WideStrings6`, `ComponentBodies6`, etc.) also exist in
`BinaryLoader.dll` for format version >= 6 files.

### PCB Storage Manager Architecture

**Entry point:** `GetStorageManager` export (`BinaryLoader.dll` at `0x01b774f0`)
returns a singleton `TStorageManager2`.

`TStorageManager2` (RTTI at `0x01b6e682`) is a **facade** wrapping
version-specific managers:

```
TStorageManager2 fields:
  FStorageManager_ASCII
  FStorageManager_Ver3
  FStorageManager_Ver4
  FStorageManager_Ver5
  FStorageManager_Ver6        -- current format
  FStorageManager_Lib_Ver5
  FStorageManager_Lib_Ver6    -- current library format
  FStorageManager_OldLibrary
  FStorageManager_ParameterManager
```

Each stream has a dedicated `TSection` subclass (e.g. `TWideStringsSection`).
The `TSection.Create` base constructor (at `0x0189fcf0`):
1. Takes a stream name (Delphi UnicodeString from the constant table)
2. Opens the corresponding OLE stream in the CFB compound file
3. Reads a header (count or version value)
4. Allocates a 128KB buffer for streaming reads
5. Subclass overrides virtual methods for stream-specific read logic

### TStorageFeature Flags

From `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TStorageFeature.cs`. These
feature flags control which sidecar data is present in a PCB file, queried
via `IPCB_StructuredStorage.GetState_Feature()`:

| Flag | Purpose |
|---|---|
| `eHasShapeBasedRegions` | Shape-based board regions present |
| `eHasShapeBasedCompBodies` | Shape-based component bodies present |
| `eHasCustomPadShapesAtWriteStage` | Custom pad shapes present |
| `eHasCustomReliefInfosAtWriteStage` | Custom thermal relief info |
| `eHasCustomMaskInfosAtWriteStage` | Custom mask shapes present |
| `eHasTHPadPasteInfosAtWriteStage` | Through-hole pad paste info |
| `eHasWirebondAtWriteStage` | Wirebond support present |
| `eHasExtendedGroupIndicesAreUsed` | Extended group indices (TReferenceToGroup) |
| `eHasIncreasedSignalLayers` | Increased signal layer count |
| `eHasIPC4761ViaTypesAtWriteStage` | IPC-4761 via types |
| `eHasMicroVias` | Micro vias support |
| (16 more flags) | Other format features |

---

## 4. PCB Sidecar Streams Detail

### 4.1 WideStrings -- Unicode Text for Primitives

**Source:** `Advpcb.dll` function at `0x548920` (Ghidra decompilation confirmed).

#### Binary TLV Encoding (PcbDoc board-level)

Each entry in the `WideStrings6` stream uses a Type-Length-Value encoding:

| Type byte | Length field | Data encoding | Notes |
|---|---|---|---|
| `0x06` | 1 byte (u8) | ASCII bytes | Short ASCII strings (len <= 255) |
| `0x0C` | 4 bytes (u32 LE) | ASCII bytes | Long ASCII strings |
| `0x12` | 4 bytes (u32 LE, chars) | UTF-16LE bytes | Unicode, length is in chars not bytes |
| `0x14` | 4 bytes (u32 LE) | UTF-8 bytes | Unicode, length is in bytes |

**Decompiled read logic (pseudocode):**

```
fn read_wide_string(stream) -> String:
    type_byte = stream.read_byte()
    match type_byte:
        0x06 => len = stream.read_u8()
                data = stream.read_bytes(len)
                return decode_ascii(data)
        0x0C => len = stream.read_u32_le()
                data = stream.read_bytes(len)
                return decode_ascii(data)
        0x12 => char_count = stream.read_u32_le()
                data = stream.read_bytes(char_count * 2)
                return decode_utf16le(data)
        0x14 => byte_len = stream.read_u32_le()
                data = stream.read_bytes(byte_len)
                return decode_utf8(data)
        _    => error("unknown string type")
```

**Write optimization:** The writer prefers ASCII (`0x06`/`0x0C`) when possible,
falls back to UTF-8 (`0x14`) or UTF-16LE (`0x12`) depending on which is shorter.

Each primitive that has a text field (e.g. `PcbTextRecord`) references its wide
string by **zero-based index** into this table.

#### PcbLib Footprint-Level WideStrings

PcbLib footprint-level `WideStrings` streams use a **completely different format**:
parameter blocks (`ENCODEDTEXT0=...`) rather than the binary TLV format. The
`ENCODEDTEXT{N}` values are comma-separated integer sequences representing
encoded text.

Format:
```
[4-byte block header: flags(1) | length(3)]
[NUL-terminated parameter string]
  e.g. |ENCODEDTEXT0=65,66,67|ENCODEDTEXT1=...|
```

This is a sequence of parameter blocks, one per WideStrings entry.

### 4.2 UniqueIDPrimitiveInformation -- Primitive Identity

**Format:** Parameter-block table with `Header` (u32 count) + `Data` (parameter blocks).

Each entry contains:

| Key | Type | Purpose |
|---|---|---|
| `PRIMITIVEINDEX` | u32 | Zero-based index within the type stream |
| `UNIQUEID` | string | Unique identifier string |

**Load lifecycle:**
1. BinaryLoader reads `Header` stream to get entry count
2. BinaryLoader reads `Data` stream, parsing `count` parameter blocks
3. For each entry, looks up the primitive at `PRIMITIVEINDEX` in its type stream
4. Calls `primitive.SetState_UniqueID(value)` to set the ID

**Save lifecycle:**
1. Iterate all primitives that have a UniqueID set
2. For each, emit a parameter block with `PRIMITIVEINDEX` + `UNIQUEID`
3. Write `Header` with count, `Data` with serialized blocks

**Cross-document purpose:** UniqueIDs are used for schematic-to-PCB linking.
When ECO (Engineering Change Order) operations synchronize between SchDoc and
PcbDoc, the UniqueID is the identity key that maps schematic pins/components
to their PCB counterparts.

The `IPCB_Primitive` interface exposes:
- `GetState_UniqueId() -> string`
- `SetState_UniqueID(string)`

And `IPCB_Board2` exposes `GenerateUniqueID() -> string` for creating new IDs.

### 4.3 ExtendedPrimitiveInformation -- Property Overrides

**Format:** Parameter-block table with `Header` (u32 count) + `Data` (parameter blocks).
Works with `ExtendedPrimitiveIndices` for fast lookup.

Each entry contains per-primitive property overrides:

| Key | Type | Purpose |
|---|---|---|
| `PRIMITIVEINDEX` | u32 | Primitive index |
| `PASTEMASKEXPANSIONMODE` | `TMaskExpansionMode` | Paste mask mode |
| `PASTEMASKEXPANSION_MANUAL` | coord (i32) | Manual paste expansion |
| `SOLDERMASKEXPANSIONMODE` | `TMaskExpansionMode` | Solder mask mode |
| `SOLDERMASKEXPANSION_MANUAL` | coord (i32) | Manual solder expansion |

#### TMaskExpansionMode Enum

From `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TMaskExpansionMode.cs`:

```csharp
enum TMaskExpansionMode : byte {
    eMaskExpansionMode_NoMask  = 0,  // No mask expansion
    eMaskExpansionMode_Rule    = 1,  // Use design rule
    eMaskExpansionMode_Manual  = 2,  // Manual override value
}
```

Serialized as the string `"None"` / `"Rule"` / `"Manual"` in parameter blocks
(confirmed by `ListPair_Rules.cs` which searches for `"SOLDERMASKEXPANSIONMODE=None|PASTEMASKEXPANSIONMODE=None"`).

#### Property Resolution Chain

From `IPCB_Primitive2`, the mask expansion value a consumer sees depends on mode:

1. **`NoMask` (0)**: No expansion applied.
2. **`Rule` (1)**: Query matching `IPCB_SolderMaskExpansionRule` or
   `IPCB_PasteMaskExpansionRule` from the board design rules.
3. **`Manual` (2)**: Use the primitive's local expansion value directly.

The `MaskExpansion(TV7_Layer)` method computes the actual value for a given
layer, accounting for the mode and per-layer overrides.

#### Additional Paste Mask Properties (IPCB_Primitive2)

```csharp
bool   GetState_PasteMaskEnabled()
bool   GetState_PasteMaskUsePercent()
double GetState_PasteMaskPercent()
double GetState_PasteMaskManualPercent()
bool   GetState_PasteMaskManualEnabled()
Guid   GetGUID()
```

#### ExtendedPrimitiveIndices

The `ExtendedPrimitiveIndices` stream provides a lookup table for fast
random access into `ExtendedPrimitiveInformation`. This avoids linear
scanning when looking up a specific primitive's extended properties.

**Note:** The indices stream is an optimization. The `Data` stream itself
contains `PRIMITIVEINDEX` in each parameter block, so the information is
self-contained.

### 4.4 PrimitiveGuids (PcbLib only)

PcbLib footprints also have a `PrimitiveGuids/{Header,Data}` stream pair.

**Binary format per entry (24 bytes):**
```
[4 bytes] objectId   (u32 LE) -- TObjectId enum value (primitive type)
[4 bytes] indexForSave (u32 LE) -- primitive index within its section
[16 bytes] guid       (raw GUID bytes)
```

From `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TPrimitiveGUID.cs`:
```csharp
[StructLayout(LayoutKind.Sequential, Pack = 1)]
public struct TPrimitiveGUID
{
    public int ObjectId;       // 4 bytes -- TObjectId enum
    public int IndexForSave;   // 4 bytes -- primitive index
    public Guid GUID;          // 16 bytes -- persistent GUID
}
```

This stream assigns a persistent GUID to each primitive within a footprint,
separate from the UniqueID string mechanism. The GUIDs survive across
library updates and are used for `IPCB_Primitive2.GetGUID() / SetGUID()`.

### 4.5 Primitive Attribute Enum (TPrimitiveAttribute)

From `AD26-dotnet/Altium.Edp.Interfaces/xPCBTypes/TPrimitiveAttribute.cs`.

Altium enumerates all primitive properties via a `TPrimitiveAttribute` enum
(533 values total). Mask-related indices:

| Index | Attribute |
|---|---|
| 61 | `SolderMaskOverride` |
| 62 | `UseSeparateSolderMaskExpansion` |
| 63 | `SolderMaskExpansion` |
| 64 | `SolderMaskExpansionTop` |
| 65 | `SolderMaskExpansionBottom` |
| 66 | `SolderMaskExpansionMode` |
| 67 | `SolderMaskExpansionFromHoleEdge` |
| 70 | `PasteMaskOverride` |
| 71 | `PasteMaskEnabled` |
| 72 | `TopPasteMaskEnabled` |
| 73 | `BottomPasteMaskEnabled` |
| 74 | `PasteMaskExpansion` |
| 75 | `PasteMaskUsePercent` |
| 76 | `PasteMaskPercent` |
| 77 | `PasteMaskExpansionMode` |

This confirms that mask expansion data is an integral part of the primitive's
property set -- it just happens to be serialized into a separate stream.

---

## 5. SchLib Pin Sidecar Streams Detail

### 5.1 File Scope

**Critical finding:** Pin sidecar streams exist ONLY in **SchLib** files, NOT
in SchDoc files. SchDoc files store all pin data inline in the main `Data`
stream. This was confirmed by examining both `SchDataImporterDocumentV5` (SchDoc)
and `SchDataImporterLibraryV5` (SchLib) -- only the library importer calls
`ReadPinsExtendedData()`.

### 5.2 Stream Names

Defined as constants in
`AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.FileFormats/FileFormatConsts.cs`:

| Stream Name | Purpose | Format |
|---|---|---|
| `PinFrac` | Fractional coordinate parts for pins | 12 bytes binary |
| `PinDesc` | Long descriptions (>254 chars, ASCII) | length-prefixed ASCII |
| `PinMiscData` | Misc data (SwapID pairs) | length-prefixed UTF-16LE params |
| `PinTextData` | Custom text display settings per pin | 2-22 bytes binary |
| `PinWideText` | Wide (Unicode) text for all pin string fields | length-prefixed UTF-16LE params |
| `PinSymbolLineWidth` | Symbol line width per pin | length-prefixed UTF-16LE params |
| `PinPackageLength` | Package-specific pin length | length-prefixed UTF-16LE params |
| `PinPropagationDelay` | Signal propagation delay | length-prefixed UTF-16LE params |
| `PinFunctionData` | Pin function metadata | length-prefixed UTF-16LE params |
| `Redirection` | Alias resolution (component aliases) | parameter block |

### 5.3 Embedded Object Container Format

All pin sidecar streams use the same envelope (`SchDataEmbeddedObject`):

```
[header block] -- parameter string: RECORD=0|HEADER=<StreamType>|Weight=<count>|
[entry 0]      -- compressed tag 0xD0 + id (pin index as string) + binary data
[entry 1]      -- ...
[entry N]      -- ...
```

**Outer block structure (per block):**
```
[4 bytes] header: flags(1 byte, bits 31-24) | length(3 bytes, bits 23-0)
[N bytes] payload
```

- Header block: flags = `0x00`, payload = NUL-terminated parameter string
- Entry blocks: flags = `0x01`, payload = compressed object

**Compressed object structure (within entry payload):**
```
[1 byte]  0xD0 tag (CFB_COMPRESSED_TAG / BinaryFileCode.CEmbeddedStream = 208)
[1 byte]  id_length
[N bytes] id (pin index as Win-1252 string, e.g. "0", "1", "2")
[4 bytes] inner header: flags(1) | length(3)
[N bytes] inner data (stream-specific binary data)
```

The `id` field is the pin index as a string ("0", "1", "2", ...).
Ordering must match the pin ordering within the component.

### 5.4 Import Order Dependency

The import order of pin sidecar streams **matters**:

- **PinDesc** (step 6) *appends* to the description field
- **PinWideText** (step 9) *fully replaces* the description field

PinWideText must be processed AFTER PinDesc because it replaces the
description entirely. PinWideText is the **authoritative source** for text
when present.

### 5.5 PinFrac -- Fractional Coordinates

**Binary data per entry: 12 bytes**

```
[4 bytes] locationX_frac (i32 LE)
[4 bytes] locationY_frac (i32 LE)
[4 bytes] length_frac    (i32 LE)
```

**Import merge:**
```csharp
pin.Location.X += locationX_frac;
pin.Location.Y += locationY_frac;
pin.PinLength  += length_frac;
```

**Export condition:** Only written when pin coordinates don't fit on DXP2004SP1
unit grid. The `PinFitsOnUnit_DXP2004SP1()` method calculates the fractional
remainder; if non-zero, it emits the sidecar entry.

**Purpose:** Backwards compatibility with DXP2004SP1 which only stored
whole-unit coordinates. The fractional parts were added later as a sidecar
stream to avoid breaking older readers.

### 5.6 PinDesc -- Long Descriptions

**Binary data format:**
```
[4 bytes] text_length (u32 LE)
[N bytes] ASCII text (the overflow portion beyond 254 chars)
```

**Import merge:** Appends to existing description:
```csharp
pin.SetDescription(pin.GetDescription() + value);
```

**Export condition:** Only written when `description.Length > 254`.
The exported data contains `description.Substring(254)` (everything
after the first 254 chars).

### 5.7 PinMiscData -- Swap ID Pairs

**Binary data format:**
```
[4 bytes] text_length (u32 LE)
[N bytes] UTF-16LE parameter string (e.g. "PairSwapID=value")
```

**Import merge:**
```csharp
if (GetParameterValue(value, "PairSwapID", out value2))
    pin.SetSwapIdPair(value2);
```

**Export condition:** Only written when `pin.GetSwapIdPair()` is non-empty.

### 5.8 PinTextData -- Custom Text Display

**Binary data: two consecutive text-data structs (Name, then Designator)**

Each struct:
```
Byte 1: Flags
  Bit 0: PositionMode    (1=Custom, 0=Default)
  Bit 1: RotationAnchor  (1=raComponent, 0=raPin)
  Bits 2-3: RotationRelative (enum 0-3, TRotationBy90)
  Bit 4: FontMode         (1=Custom, 0=Default)

If PositionMode == Custom (bit 0 set):
  [4 bytes] customMargin (i32 LE)

If FontMode == Custom (bit 4 set):
  [2 bytes] customFontID (i16 LE)
  [4 bytes] customColor  (u32 LE)
```

**Variable-length:** Each struct is 1 byte minimum (both defaults), up to
11 bytes (both custom: 1 + 4 + 2 + 4). Two structs concatenated means
2-22 bytes total.

**Fields merged into `SchDataPin`:**

For Name:
- `pin.namePositionMode`
- `pin.nameCustomPositionMargin`
- `pin.nameCustomRotationRelative` (TRotationBy90: eRotate0..eRotate270)
- `pin.nameCustomRotationAnchor` (TPinTextRotationAnchor: raPin/raComponent)
- `pin.nameFontMode`
- `pin.nameCustomFontID`
- `pin.nameCustomColor`

For Designator: same set with `designator` prefix.

**Export condition:** Only written when at least one of the four modes
(namePosition, nameFont, designatorPosition, designatorFont) is Custom.

### 5.9 PinWideText -- Unicode Text

**Binary data format:**
```
[4 bytes] text_length (u32 LE)
[N bytes] UTF-16LE parameter string
```

**Parameter keys and target fields:**

| Key | Target Field | Override Behavior |
|---|---|---|
| `Desc` | `pin.description` | Full replacement |
| `Name` | `pin.name` | Full replacement |
| `Desig` | `pin.designator` | Full replacement |
| `SwapId` | `pin.swapIdPin` | Full replacement |
| `SwapIDPart` | `pin.swapIdPartAndPartPin` | Full replacement |
| `DefValue` | `pin.defaultValue` | Full replacement |

**Import merge:** Unlike PinDesc (which appends), PinWideText **fully replaces**
the target field. This is because PinWideText contains the complete text
(including non-ASCII characters or text exceeding 254 chars).

**Export condition:** Only written when at least one field is non-empty.

**Import order matters:** PinWideText is imported AFTER PinDesc, so its full
replacement overwrites the append done by PinDesc. PinWideText is the
authoritative source for text when present.

### 5.10 PinSymbolLineWidth

**Binary data format:**
```
[4 bytes] text_length (u32 LE)
[N bytes] UTF-16LE parameter string (e.g. "SymBol_LineWidth=3")
```

**Import merge:**
```csharp
pin.SetSymbolLineWidth((TSize)int.Parse(value));
```

**Export condition:** Only written when `pin.GetSymbolLineWidth() != TSize.eZeroSize`.

### 5.11 PinPackageLength

**Binary data format:**
```
[4 bytes] text_length (u32 LE)
[N bytes] UTF-16LE parameter string (e.g. "PinPackageLength=200000")
```

**Import merge:**
```csharp
pin.SetPinPackageLength(int.Parse(value));
```

**Export condition:** Only written when `pin.GetPinPackageLength() != 0`.

### 5.12 PinPropagationDelay

**Binary data format:**
```
[4 bytes] text_length (u32 LE)
[N bytes] UTF-16LE parameter string (e.g. "PinPropagationDelay=1.5E-9")
```

**Import merge:**
```csharp
pin.SetPropagationDelay(StrUtils.TryParseExponent(value));
```

**Export condition:** Only written when `pin.GetPropagationDelay() != 0.0`.
Uses `StrUtils.DelayToString()` for scientific notation serialization.

### 5.13 PinFunctionData -- Pin Alternate Functions

**Binary data format:**
```
[4 bytes] text_length (u32 LE)
[N bytes] UTF-16LE parameter string
```

**Parameter keys:**

| Key | Purpose |
|---|---|
| `PinSelectedFunctionsCount` | Number of selected functions |
| `PinSelectedFunction1..N` | Selected function names (1-based) |
| `PinDefinedFunctionsCount` | Number of defined functions |
| `PinDefinedFunction1..N` | Defined function names (1-based) |

**Import merge:**
```csharp
for (int i = 1; i <= count; i++)
    pinFunctions.AddFunction(GetParameterValue("PinSelectedFunction" + i));
```

**Note:** Function indices are **1-based**, unlike most other Altium indices.

**Export condition:** Only written when at least one function list has entries.

### 5.14 Redirection -- Component Aliases

**Format:** Single parameter block in a separate stream.

**Location:** `/<alias_section_key>/Redirection`

**Content:** `SECTIONNAME=<canonical_component_name>`

**Import logic (from `SchDataImporterLibraryV5.GetLibraryReferenceByAliasName`):**
1. Check if `/<section>/Redirection` stream exists
2. If yes, read `SECTIONNAME` parameter -> resolve to canonical component
3. If no Redirection stream but `/<section>/Data` exists, use the section name directly
4. Otherwise, fall back to searching `FileHeader` for alias mappings

**Purpose:** For each component alias, Altium creates a CFB section `/<alias>/`
with a `Redirection` stream pointing to the canonical component. The canonical
component has an `AliasList` (`SchDataAliasList`) tracking all its aliases.

---

## 6. SchDoc-Specific Streams Detail

### 6.1 ReuseBlocks (V1)

**Envelope:** `WriteBinaryBlocksData` format (instruction `0xD0` embedded object).

**Binary data:**
```
[4 bytes] version (i32 LE) -- must be <= 2
[4 bytes] count (i32 LE)
For each entry:
    [string] id                          -- ReadString(reader, encoding, version)
    [string] blockVaultGuid
    [string] blockItemGuid
    [string] blockItemRevisionGuid
    [string] schSnippetVaultGuid
    [string] schSnippetItemGuid
    [string] schSnippetItemRevisionGuid
    [4 bytes] partInfoCount (i32 LE)
    For each partInfo:
        [string] uniqueId
        [string] uniqueIdInReuseBlock
```

String encoding: version 1 = 1-byte length prefix + ASCII;
version 2 = 4-byte length prefix (i32 LE) + ASCII.

### 6.2 ReuseBlocksV2

**Envelope:** Same `WriteBinaryBlocksData` format.

**Binary data:**
```
[4 bytes] version (i32 LE) -- must be <= 2
[4 bytes] count (i32 LE)
For each entry:
    [string] id                           -- must match V1 entry
    [string] pcbSnippetVaultGuid
    [string] pcbSnippetItemGuid
    [string] pcbSnippetItemRevisionGuid
```

Extends V1 with PCB snippet references. Matched to V1 entries by `id`.
Fallback: if version=2 encoding fails, retries with version=1 encoding.

### 6.3 HarnessConnectionPointConnector

**Envelope:** `WriteBinaryBlocksData` format.

**Binary data:**
```
[4 bytes] version (i32 LE) -- currently 1, import checks <= 1
[4 bytes] connectionPointCount (i32 LE)
For each connection point:
    [length-prefixed string] uniqueId      -- .NET BinaryWriter format (7-bit encoded length + UTF-8)
    [4 bytes] connectorCount (i32 LE)
    For each connector:
        [length-prefixed string] connectorUniqueId
        [4 bytes] pinCount (i32 LE)
        For each pin:
            [length-prefixed string] pinId
```

### 6.4 ObjectDefinitions

**Format:** Custom warehouse -- same header+record format as the base warehouse.
Contains object definition records (BinaryFileCode 129 = `CObjectDefinition`).

### 6.5 ReuseBlockInfos

**Format:** Custom warehouse. Contains dissolved reuse block implementation info
(BinaryFileCode 138 = `CReuseBlockImplementationInfo`).

### 6.6 WriteBinaryBlocksData Envelope Format

Used by ReuseBlocks, ReuseBlocksV2, and HarnessConnectionPointConnector:

```
[stream start: StartStream("", streamName)]
[header record: RECORD=0 | HEADER=<streamName>]
[binary embedded object: instruction=208 (0xD0) + SchDataEmbeddedObject data]
[footer record: RECORD=0]
[stream end: EndStream()]
```

---

## 7. Harness-Specific Streams

### 7.1 Files Stream (Harness Layout Drawing only)

**Format:**
```
[header record: RECORD=0 | HEADER=... | Weight=<count>]
For each file:
    [1 byte] instruction tag = 227 (0xE3 = BinaryFileCode.CFileStream)
    [SchDataFileObject]:
        Guid (System.Guid, 16 bytes)
        Data (compressed content bytes)
        Hash (string)
```

Stores embedded image parameter model files (referenced by
`ISchDataImageParameter.ModelFileId` GUID). Only present in harness layout
drawing documents.

### 7.2 Reserved Stream Names (Defined but Unused)

From `FileFormatConsts.cs`:
- `HarnessComponentCrimps` (version 1) -- no import/export code exists
- `HarnessAssociatedParts` (version 1) -- serialized inline via base warehouse instead

---

## 8. Binary File Codes for Sidecar Envelopes

From `AD26-dotnet/Altium.Sch.DataModel/Altium.Sch.DataModel.FileFormats/BinaryFileCode.cs`:

| Code | Constant | Purpose |
|---|---|---|
| 208 (0xD0) | `CEmbeddedStream` | Tag for embedded binary data objects (pin sidecars, reuse blocks) |
| 227 (0xE3) | `CFileStream` | Tag for file stream objects (harness `Files` stream) |
| 254 (0xFE) | `CExtraObjectIndex` | Extended instruction code marker (for codes > 255) |
| 255 (0xFF) | `CEndInstruction` | End-of-stream marker |

---

## 9. Source References

### .NET Decompiled Sources

| File | Purpose |
|---|---|
| `AD26-dotnet/Altium.Sch.DataModel/.../FileFormatConsts.cs` | Stream name constants |
| `AD26-dotnet/Altium.Sch.DataModel/.../SchDataImporterLibraryV5.cs` | SchLib import (lines 49-62: Run, 455-511: pin sidecars) |
| `AD26-dotnet/Altium.Sch.DataModel/.../SchDataExporterLibraryV5.cs` | SchLib export with all 9 sidecar writes |
| `AD26-dotnet/Altium.Sch.DataModel/.../SchDataImporterDocumentV5.cs` | SchDoc import (ReuseBlocks, HarnessConnector, ObjectDefinitions) |
| `AD26-dotnet/Altium.Sch.DataModel/.../SchDataImporterSheetV5.cs` | SchDoc sheet import (ReuseBlocks V1/V2) |
| `AD26-dotnet/Altium.Sch.DataModel/.../SchDataImporterBaseV5.cs` | Base import flow (lines 33-54: Run method chain) |
| `AD26-dotnet/Altium.Sch.DataModel/.../SchDataExporterBaseV5.cs` | Base export flow (write ordering) |
| `AD26-dotnet/Altium.Sch.DataModel/.../SchDataEmbeddedObject.cs` | Embedded object container |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_Primitive.cs` | UniqueId get/set on primitives |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_Primitive2.cs` | Mask expansion, paste mask, GUID |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_StructuredStorage.cs` | PCB storage interface |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_BinarySection.cs` | Section interface (ApplyGUIDs, etc.) |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TPrimitiveGUID.cs` | 24-byte GUID struct layout |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TStorageFeature.cs` | 26 feature flags |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TMaskExpansionMode.cs` | Mask expansion mode enum |
| `AD26-dotnet/Altium.Edp.Interfaces/xPCBTypes/TPrimitiveAttribute.cs` | Full primitive attribute enum (533 values) |

### Ghidra Decompiled (Delphi DLLs)

| DLL | Address | Identifier | Purpose |
|---|---|---|---|
| `Advpcb.dll` | `0x548920` | `FUN_00548920` | WideStrings binary TLV read |
| `Advpcb.dll` | `0x03840700` | (data) | Stream name table (88 entries) |
| `Advpcb.dll` | `0x038418e0` | `FUN_038418e0` | Primitive type dispatch (83 types) |
| `Advpcb.dll` | `0x03d20660` | `PcbApi_LoadBoardByFullFileName` | Board loading entry point |
| `BinaryLoader.dll` | `0x01b774f0` | `GetStorageManager` | Singleton storage manager factory |
| `BinaryLoader.dll` | `0x01b6fd90` | `TStorageManager2.Create` | Storage manager constructor |
| `BinaryLoader.dll` | `0x01923210` | `TStorageManager_Ver6.Create` | Ver6 manager constructor |
| `BinaryLoader.dll` | `0x0189fcf0` | `TSection.Create` | Base section constructor |
| `BinaryLoader.dll` | `0x019679e0` | `TWideStringsSection.Create` | WideStrings section constructor |
| `BinaryLoader.dll` | `0x01918020` | `TPCBBinaryFile::RegisterAllSectionsForExporting` | PcbDoc 23-section registration |
| `BinaryLoader.dll` | `0x01919170` | `TPCBLibraryBinaryFile::RegisterAllSectionsForExporting` | PcbLib section registration |
| `BinaryLoader.dll` | `0x01919210` | `LoadComponentFromLibraryFile` | PcbLib footprint loading |
| `BinaryLoader.dll` | `0x0186a8c0` | (data) | Stream name table (88 entries, replicated) |

### Existing Codebase

| File | Purpose |
|---|---|
| `crates/altium-format/src/documents/schlib_streams.rs` | SchLib sidecar stream codecs |
| `crates/altium-format/src/documents/pcblib_streams.rs` | PcbLib sidecar stream codecs |
