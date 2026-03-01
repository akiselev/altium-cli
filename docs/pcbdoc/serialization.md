# PcbDoc Serialization (Round-Trip Write)

How to serialize an in-memory `PcbDoc` back to a byte-identical CFB file.

Sources: Delphi classes `TPCBBinaryFileV6`, `TPrimitivesSection`, `TBoardSection` (in
`Altium.PCB.BinaryLoader.dll` via Ghidra project `altium26`); .NET interfaces
`IPCB_StructuredStorage`, `IPCB_BinarySection`, `IPCB_Board_SaveLoadParameters`,
`IPCB_IndexForSaveIndexer` (in `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/`); constants from
`AD26-dotnet/Altium.Edp.Interfaces/xPCBTypes/Consts.cs`.

---

## 1. Architecture: The Write Pipeline

The write pipeline reverses the load pipeline (see [loading-pipeline.md](loading-pipeline.md)):

```
PcbDoc (in-memory)
  → Phase 1: Pre-save preparation
      → PrepareToSave() on each section
      → Assign IndexForSave values
      → Collect extra primitives
      → Collect text primitives for WideStrings
  → Phase 2: Compute ownership indices
      → SetIndexes() on each primitive (net, polygon, component, pad, coordinate, dimension)
      → Build TReferenceToGroup entries if extended indices enabled
  → Phase 3: Write CFB container
      → Write /FileHeader (UTF-16LE legacy header)
      → Write /FileHeaderSix (V6 pascal-block header)
      → Set TStorageFeature flags
  → Phase 4: Export sections
      → For each section: Export_ToFile()
      → Write sidecar streams (WideStrings6, UniqueID, ExtendedPrimitiveInfo, PrimitiveGuids)
      → Write Models section
  → Phase 5: Post-save cleanup
      → PostLoadOrSave(true)
      → Clear dirty flag
      → Close CFB container
```

### Delphi Class Hierarchy (from Ghidra RTTI)

```
TBinaryFile
  └── TPCBBinaryFile (RTTI at 0x0190ae58)
        └── TPCBBinaryFileV6 (RTTI at 0x01b42ef8)
              ├── GetFileIdentifier()             → "PCB 6.0 Binary File"
              ├── BinaryFile_OpenWrite()           → creates CFB + FileHeader + FileVersionInfo
              ├── Export_ToFile_PreProcess()        → Board6 + feature flags
              ├── Export_ToFile(silent)             → iterates sections, calls each Export
              └── CreateSection(name)              → maps names to section classes

TSection (base section)
  └── TPrimitivesSection                          (binary primitive records)
        ├── Export_ToFile()                       → Header/Data stream writing loop
        ├── WritePrimitive(prim)                  → per-record binary serialization
        ├── Section_ExportSetup()                 → create Header/Data streams
        └── Section_ExportFinalize()              → close streams
  └── TBoardSection                               (Board6 parameter export)
```

### COM Interface Pipeline

```csharp
// Entry point (from .NET side):
IPCB_StructuredStorage.Export_ToFile(bool argSilent)

// Steps:
1. BinaryFile_OpenWrite()             — create CFB, write FileHeader + FileVersionInfo
2. Export_ToFile_PreProcess()          — export Board6, detect features, set flags
3. For each section: PrepareToSave()   — assign IndexForSave, collect extras
4. For each section: Export_ToFile()    — write Header + Data streams
5. Write sidecar streams               — WideStrings6, UniqueID, etc.
6. Write global streams                — FileHeader, FileHeaderSix
```

---

## 2. Phase 1: Pre-Save Preparation

### IndexForSave Assignment

Every primitive must be assigned a sequential zero-based index within its section
before saving. This is managed by `IPCB_IndexForSaveIndexer`:

```csharp
public interface IPCB_IndexForSaveIndexer {
    void Clear();                                      // Reset all counters
    int GetIndex(TObjectId argObjectId);               // Current counter for object type
    void SetIndex(TObjectId argObjectId, int argValue); // Set counter
}
```

Each section iterates its primitives and calls:

```csharp
primitive.SetState_IndexForSave(indexer.GetIndex(objectId));
indexer.SetIndex(objectId, indexer.GetIndex(objectId) + 1);
```

### Extra Primitives

Some primitives (e.g. oversized binary fields > 32,000 bytes) are split into "extra"
records. Each section collects these:

```csharp
public interface IPCB_BinarySection {
    void CollectExtraPrimitives();
    void IndexExtraPrimitives(IPCB_IndexForSaveIndexer argIndexer);
    void AddExtraPrimitive(IPCB_Primitive argPrimitive);
}
```

From the Delphi decompilation of `HandleExtraPrimitives` (at `0x01882360`), extra
primitives are created when any of 5 sub-object types on a primitive exceed 32,000 bytes.
Each extra is assigned a type code (8, 9, 10, 0xD, 0xE).

### WideStrings Collection

Text primitives that require Unicode representation are collected before save:

```csharp
storage.AddTextsForSaveList(primitive);       // Add text to save list
int count = storage.TextsForSaveListCount();   // Total texts
primitive.SetState_WideStringIndexForSave(idx); // Assign WS index
```

The primitives that participate in WideStrings are defined by `WideStringObjects` in
`Consts.cs` (line 70): `eTextObject` (5), `ePadObject` (2), `eComponentObject` (9),
`eDimensionObject` (13).

---

## 3. Phase 2: Ownership Index Computation

Each primitive has 6 cross-reference indices stored in its binary common header (see
[binary-primitives.md](binary-primitives.md)):

```csharp
section.SetIndexes(primitive,
    vNet,          // index into Nets6 (0xFFFF = no net)
    vPolygon,      // index into Polygons6 (0xFFFF = none)
    vComponent,    // index into Components6 (0xFFFF = none)
    vPadOwner,     // index into parent pad (0xFFFF = none)
    vCoordinate,   // index into Coordinates6 (0xFFFF = none)
    vDimension     // index into Dimensions6 (0xFFFF = none)
);
```

These indices are written into the 13-byte common header at offsets 3-12. They establish
the PCB ownership graph — a fundamentally different model from schematic OWNERINDEX:

- A pad can simultaneously belong to a net, a component, and have no polygon association
- Multiple primitives can point to the same net/component/polygon
- The indices are per-section sequential numbers, not global IDs

### Extended Group Indices

When `TStorageFeature.eHasExtendedGroupIndicesAreUsed` is set, an additional mechanism
is used:

```csharp
struct TReferenceToGroup {  // 16 bytes, pack=8
    TPrimitiveKey Prim;       // (i32 ObjectId, i32 IndexForSave) — 8 bytes
    TPrimitiveKey PrimGroup;  // (i32 ObjectId, i32 IndexForSave) — 8 bytes
}
```

These are stored per-section and applied via `ApplyExtendedIndices()`.

---

## 4. Phase 3: Write CFB Container

### FileHeader Stream

The legacy `/FileHeader` stream uses UTF-16LE encoding:

```
[4 bytes]    u32 LE: character count (NOT byte count)
[N*2 bytes]  UTF-16LE string: "PCB 5.0 Binary File"
```

The stream is always 24 bytes: 4-byte length (19) + 19*2 = 38 UTF-16LE bytes... but
observed as 24 bytes total, meaning it's 4 + 20 bytes (19 chars + NUL in UTF-16LE = 40,
fitting in 24 via the character count as storage length).

From the Delphi decompilation of `BinaryFile_OpenWrite` (at `0x01b455d0`):

```c
stream = CreateStream(cfb, "FileHeader", 0);
WriteToStream(stream, &charCount, 4);      // u32 LE: 19
WriteToStream(stream, utf16_string, len);   // UTF-16LE "PCB 5.0 Binary File"
```

### FileHeaderSix Stream

The `/FileHeaderSix` stream uses Win1252 pascal-block format:

```
[4 bytes]  u32 LE: block length (string_length + 1)
[1 byte]   u8: header text length (N)
[N bytes]  Win1252: "PCB 6.0 Binary File"
[8 bytes]  f64 LE: format version number (e.g. 5.01)
[4 bytes]  u32 LE: key block length
[1 byte]   u8: key length (M)
[M bytes]  Win1252: document GUID key (e.g. "{C1EF2D32-6634-4C5A-A58E-5AF8D518C64E}")
```

See [fileheader.md](fileheader.md) for the full layout and constants.

### FileVersionInfo Stream

Written by `BinaryFile_OpenWrite`. Contains a single string block:

```
[4 bytes]  u32 LE: length (0x13 = 19)
[20 bytes] "AdvancedPCBVersion6\0"
```

### Storage Feature Flags

Set via `SetState_Feature()` on the storage. These 26 flags (see
[enumerations.md](enumerations.md)) record which capabilities were active at save time:

```csharp
public interface IPCB_StructuredStorage {
    void SetState_Feature(TStorageFeature argFeature, bool argValue);
    bool GetState_Feature(TStorageFeature argFeature);
}
```

Key flags that affect serialization format:

| Flag | Effect on Save |
|------|----------------|
| `eHasShapeBasedRegions` (5) | Write ShapeBasedRegions6 instead of Regions6 as authoritative |
| `eHasShapeBasedCompBodies` (6) | Write ShapeBasedComponentBodies6 as authoritative |
| `eHasCustomPadShapesAtWriteStage` (9) | Include custom pad shape subrecords |
| `eHasFootprintParametersAtWriteStage` (11) | Include footprint-level parameters |
| `eHasExtendedGroupIndicesAreUsed` (23) | Use TReferenceToGroup extended indices |
| `eHasIncreasedSignalLayers` (24) | Support > 32 signal layers |

---

## 5. Phase 4: Section Export

### Section Registration

The Delphi function `RegisterAllSectionsForExporting` (at `0x01918020`) registers sections
in a specific order. The complete section name table from the binary (at `0x01bb4a80`)
reveals 50+ known section names:

```
Section_Board, Section_Arcs, Section_Pads, Section_Vias, Section_Tracks,
Section_Texts, Section_Fills, Section_Connections, Section_Regions,
Section_ComponentBody, Section_Nets, Section_Components, Section_Polygons,
Section_Rules, Section_Dimensions, Section_Coordinates, Section_Classes,
Section_DifferentialPairs, Section_FromTos, Section_EmbeddedBoards,
Section_Embeddeds, Section_Models, Section_Textures, Section_PinSwap,
Section_DesignRuleChecker, Section_AdvancedPlacer, Section_PadViaLibrary,
Section_EmbeddedFonts, Section_FileVersionInfo, Section_LayerKindMapping,
Section_SignalClasses, Section_SmartUnions, Section_UnionNames,
Section_UnionRelations, Section_UnionFeatures, Section_PinPairs,
Section_WaivedViolations, Section_Violations, Section_ConstraintManager,
Section_PrimitiveGuids, Section_xNetClasses, Section_HoleSizeInfo,
Section_RuleAdditionalData, Section_LetterGeometry, Section_ZAxisClearanceCache,
Section_SimberianCache, Section_ViaInstance, Section_ExtendedPrimitiveIndices,
Section_MechanicalPrimitives, Section_Testpoint, WideStrings
```

### Export Loop (TPrimitivesSection)

From the Delphi decompilation of `TPrimitivesSection.Export_ToFile` (at `0x018825f0`):

```
1. Section_ExportSetup()         — create CFB storage + Header/Data streams
2. recordIndex = 0
3. For each primitive in main list:
     WritePrimitive(primitive)
4. For each primitive in extras list:
     WritePrimitive(primitive)
5. Section_ExportFinalize()      — close streams
```

### Section Stream Setup

From the Delphi decompilation of `Section_ExportSetup` (at `0x018a0320`):

```
1. Compute record count
2. Create CFB storage for section (e.g. "/Arcs6")
3. Create "Header" sub-stream → write u32 LE record count
4. Create "Data" sub-stream → set as active write target
```

### WritePrimitive (per-record)

From the Delphi decompilation of `WritePrimitive` (at `0x01882500`):

```
1. Check ShouldExportPrimitive(primitive)
2. BeginRecord()
3. WriteRecordHeader(primitive)
4. Build record descriptor:
     byte[0] = GetObjectId(primitive) & 0xFF    // TObjectId type byte
     int[1]  = recordIndex                       // sequential index
     payload = primitive.Export_ToBinary()        // binary serialization
5. WriteRecordToStream(descriptor)               // type + u32 length + payload
6. HandleExtraPrimitives(primitive, recordIndex)  // check for >32KB sub-objects
7. recordIndex++
```

---

## 6. Section Serialization by Category

Each section's Data stream format depends on its category.
See [cfb-structure.md](cfb-structure.md) for the complete section classification.

### 6.1 Primitive Binary Sections

**Sections**: Arcs6, Pads6, Vias6, Tracks6, Texts6, Fills6, Regions6,
ShapeBasedRegions6, ComponentBodies6, ShapeBasedComponentBodies6, BoardRegions,
Texts (legacy), Connections6, SplitPlaneRegions6

**Header stream**: `u32 LE` record count

**Data stream**: packed binary records:

```
For each primitive:
  [1 byte]   u8 TObjectId type byte
  [4 bytes]  u32 LE payload length
  [N bytes]  binary payload (starts with 13-byte common header)
```

Multi-subrecord types (Pad = 6 subrecords, Text = 2 subrecords) pack all subrecords
sequentially — only the first subrecord has the type byte prefix:

```
Pad:  [u8 type=2] [u32 len][payload₀] [u32 len][payload₁] ... [u32 len][payload₅]
Text: [u8 type=5] [u32 len][payload₀] [u32 len][payload₁]
```

**Variant sections**: When `eHasShapeBasedRegions` is set, `ShapeBasedRegions6` contains
the authoritative data and `Regions6` is the legacy fallback (both are written). Same
for `ShapeBasedComponentBodies6` / `ComponentBodies6`.

### 6.2 Standard Parameter Sections

**Sections**: Board6, Nets6, Components6, Polygons6, Classes6, DifferentialPairs6,
FromTos6, Connections6 (param variant), EmbeddedBoards6, Embeddeds6,
UniqueIDPrimitiveInformation, ExtendedPrimitiveInformation, PadViaLibrary,
PadViaLibraryCache, PadViaLibraryLinks, PinPairsSection, SignalClasses,
SmartUnions, UnionRelations, WaivedViolations, PrimitiveParameters,
Advanced Placer Options6, Advanced Router Options6,
Design Rule Checker Options6, Pin Swap Options6, UnionNames

**Header stream**: `u32 LE` record count

**Data stream**: concatenated parameter blocks:

```
For each record:
  [4 bytes]  u32 LE string length (including NUL terminator)
  [N bytes]  Win1252 NUL-terminated pipe-delimited: |KEY1=VALUE1|KEY2=VALUE2|...\0
```

See [parameter-sections.md](parameter-sections.md) for per-section parameter keys.

### 6.3 Prefixed Parameter Sections

**Sections**: Rules6, NewRules6, Dimensions6, Coordinates6

**Header stream**: `u32 LE` record count

**Data stream**: prefixed parameter blocks:

```
For each record:
  [2 bytes]  u16 LE prefix (section-specific interpretation)
  [4 bytes]  u32 LE string length (including NUL terminator)
  [N bytes]  Win1252 NUL-terminated pipe-delimited parameter string
```

**Prefix meanings**:

| Section | Prefix | Meaning |
|---------|--------|---------|
| Rules6 / NewRules6 | u16 | `TRuleKind` enum value |
| Dimensions6 | u16 | `TDimensionKind` enum value |
| Coordinates6 | u16 | Likely analogous to Dimensions6 |

### 6.4 WideStrings6 (Binary TLV)

**Streams**: `WideStrings6/Header` + `WideStrings6/Data`

**Header**: `u32 LE` entry count

**Data**: flat binary entries (NOT block-framed):

```
For each text primitive:
  [4 bytes]  u32 LE: primitive index (sequential 0, 1, 2, ...)
  [4 bytes]  u32 LE: byte_length (UTF-16LE byte count, including NUL terminator)
  [N bytes]  UTF-16LE encoded string (NUL-terminated)
```

**Empty string sentinel**: When `byte_length == 2`, the entry represents an empty string.
The 2 bytes are just the UTF-16LE NUL terminator.

**CRITICAL**: This format is completely different from PcbLib's parameter-block WideStrings
(`ENCODEDTEXT0=...`). They share NO structure.

See [sidecar-streams.md](sidecar-streams.md) for full format details and hex examples.

### 6.5 Models Section (Special)

**Structure**:

```
/Models/
  Header     u32 LE: model count
  Data       model metadata parameter blocks (u32 len + params per entry)
  0          zlib-compressed STEP model blob
  1          zlib-compressed STEP model blob
  ...
```

The numbered sub-streams correspond 1:1 with metadata entries. Model metadata parameters
include: `EMBED`, `MODELSOURCE`, `ID` (GUID), `ROTX/ROTY/ROTZ`, `DZ`, `CHECKSUM`, `NAME`.

### 6.6 EmbeddedFonts6 (Binary)

Custom binary format:

```
For each font entry:
  [u32 LE byte_len] [UTF-16LE full_name]
  [u32 LE byte_len] [UTF-16LE face_name]
  [u32 LE byte_len] [UTF-16LE style_name]
  IF style_name is NOT empty:
    [u8 bold]
    [u8 italic]
  [u8 charset]
  [u32 LE blob_size]
  [blob_size bytes] zlib-compressed font data
```

**Edge case**: When `style_name` is empty (byte_len == 2, just UTF-16LE NUL), the
bold and italic bytes are **omitted**.

### 6.7 ConstraintManager (Special)

**Streams**: `ConstraintManager/Header` + `ConstraintManager/Data`

The ConstraintManager section uses serialized UTF-16LE strings that may contain
base64-encoded, zlib-compressed constraint data:

```csharp
public interface IPCB_BoardConstraintManager {
    int GetState_ConstraintManagerExtraDataCount();
    string GetState_ConstraintManagerExtraData(int argIndex);
    void AddConstraintManagerExtraData(string argSerializedData);
    void ClearConstraintManagerExtraDatas();
}
```

Multiple entries are stored, indexed sequentially. The exact internal format of the
serialized strings requires further investigation.

### 6.8 PrimitiveParameters (Hierarchical)

**Streams**: `PrimitiveParameters/Header` + `PrimitiveParameters/Data`

**Header**: `u32 LE` component count (NOT total block count)

**Data**: grouped parameter blocks:

```
For each component:
  [component header block]  |PRIMITIVEID=<id>|VARIANTGUID=<guid>|COUNT=<N>|
  [parameter block 1]       |NAME=<name>|VALUE=<value>|ISIMPORTED=TRUE|
  [parameter block 2]       ...
  ...
  [parameter block N]       ...
```

Each block uses standard `[u32 LE length][NUL-terminated params]` format.
The `COUNT` field in the header block determines how many parameter blocks follow.

### 6.9 Other Binary Sections

| Section | Format | Notes |
|---------|--------|-------|
| LayerKindMapping | Version string + layer kind entries | Same parser as PcbLib |
| UnionNames | u32 format_version + parameter blocks | Union name strings |
| PrimitiveGuids | Packed 24-byte `TPrimitiveGUID` records | See [sidecar-streams.md](sidecar-streams.md) |
| Textures | Custom | Typically empty |
| ModelsNoEmbed | Parameter blocks | External model references |

---

## 7. Board6 Section Export

Board6 is special — it's always exported first (in `Export_ToFile_PreProcess`) and
contains the board object as a single massive parameter block (~100KB).

From the Delphi decompilation of `Export_ToFile_PreProcess` (at `0x01b45d40`):

```
1. Create TBoardSection
2. Set origin (default 1000mil, 1000mil = 10,000,000 internal units)
3. Set max record size limits (40,000 → 80,000 bytes)
4. Export board primitive in multiple passes (standard + continuation blocks)
5. Detect storage features from board content
6. Write feature flags to container
```

The Board6 Data stream contains one or more parameter blocks. The first block contains
~2,700 keys covering board metadata, layer stack, grids, outline geometry, and
editor state. See [board-section.md](board-section.md) for the complete parameter reference.

---

## 8. Sidecar Stream Export

After all sections are exported, sidecar streams are built from the primitives:

### WideStrings6 Build Process

```
1. During PrepareToSave: AddTextsForSaveList(primitive) for each text-bearing primitive
2. Assign sequential WideStringIndexForSave to each collected primitive
3. Build binary table: [u32 index][u32 byte_len][UTF-16LE string] per entry
4. Write to WideStrings6/{Header,Data}
```

### UniqueIDPrimitiveInformation Build Process

```
1. For each primitive with a UniqueID string:
   a. Build parameter string: |PRIMITIVEINDEX=N|PRIMITIVEOBJECTID=Type|UNIQUEID=XXXXXXXX|
   b. Write as [u32 len][NUL-terminated params]
2. Write count to Header, blocks to Data
```

### PrimitiveGuids Build Process

```
1. For each primitive with a GUID:
   a. Build TPrimitiveGUID: { ObjectId, IndexForSave, 16-byte GUID }
   b. Append 24-byte packed record to Data
2. Write count to Header
```

---

## 9. Section Presence Rules

From the 96-file test corpus analysis:

### Always present (even when empty)

All 42+ sections observed in test files are always created in the CFB container,
even when their Data stream is empty (0 bytes). The Header contains `00 00 00 00`
(count=0) and the Data stream is zero-length.

This includes: Board6, Arcs6, Pads6, Vias6, Tracks6, Texts6, Fills6, Regions6,
ShapeBasedRegions6, ComponentBodies6, ShapeBasedComponentBodies6, BoardRegions,
Texts (legacy), Nets6, Components6, Polygons6, Classes6, DifferentialPairs6,
FromTos6, Connections6, EmbeddedBoards6, Embeddeds6, Rules6, Dimensions6,
Coordinates6, Models, WideStrings6, UniqueIDPrimitiveInformation,
ExtendedPrimitiveInformation, PrimitiveParameters, EmbeddedFonts6,
FileVersionInfo, LayerKindMapping, Textures, ModelsNoEmbed, PadViaLibrary,
PadViaLibraryCache, PadViaLibraryLinks, PinPairsSection, SignalClasses,
SmartUnions, UnionNames, WaivedViolations, Advanced Placer Options6,
Design Rule Checker Options6, Pin Swap Options6.

### Conditionally present

These sections appear only when the file uses the corresponding feature:

| Section | Condition |
|---------|-----------|
| SplitPlaneRegions6 | Files with split power/ground planes |
| UnionRelations | Files with union relation mappings |
| ConstraintManager | Files with constraint manager data |
| Advanced Router Options6 | Files with auto-router settings |
| NewRules6 | Files with extended design rules |
| PrimitiveGuids | Files with persistent primitive GUIDs |
| UnionFeatures | Files with union feature flags |
| SharedUnion | Files with shared union param groups |
| CustomShapes | Files with custom pad shapes |
| DrillManager | Files with drill manager configuration |

### DRC Violation Sections

DRC violation results are stored as parameter sections. Known types:

| Section | DRC Rule |
|---------|----------|
| TClearanceViolation | Clearance rule violations |
| TShortCircuitViolation | Short circuit violations |
| TSilkToSilkClearanceViolation | Silk-to-silk clearance |
| TRoutingViaStyleViolation | Routing via style |
| TMinSolderMaskSliverViolation | Min solder mask sliver |
| TModifiedPolygonViolation | Modified polygon |
| TNetAntennaeViolation | Net antennae |
| TDiffPairsViolation | Differential pairs |
| TBoardOutlineClearanceViolation | Board outline clearance |
| TUnconnectedPinViolation | Unconnected pin |
| TSilkToSolderMaskClearanceViolation | Silk-to-soldermask clearance |
| TMinimumAnnularRingViolation | Minimum annular ring |
| TComponentClearanceViolation | Component clearance |
| TMaxMinLengthViolation | Max/min length |
| TDisconnectedSubnetsViolation | Disconnected subnets |
| TMatchedNetLengthsViolation | Matched net lengths |
| THoleToHoleViolation | Hole-to-hole clearance |
| TMaxMinComponentHeightViolation | Component height |
| TMaxMinViaHoleSizeViolation | Via hole size |
| TMaxMinPadSlotWidthViolation | Pad slot width |

All use standard parameter block format. These are dynamic — their presence depends on
whether a DRC run has been performed and violations exist.

---

## 10. Key Differences from PcbLib Serialization

| Aspect | PcbDoc | PcbLib |
|--------|--------|--------|
| **Organization** | Flat: one section per type at root | Hierarchical: per-footprint storages |
| **FileHeader** | UTF-16LE `"PCB 5.0 Binary File"` + FileHeaderSix | Pascal-block `"PCB 6.0 Binary Library File"` |
| **Board data** | `/Board6/Data` (~100KB, first section exported) | `/Library/Data` (smaller, library defaults) |
| **Primitive grouping** | Type-per-section (all arcs in Arcs6, etc.) | All types mixed in single Data stream per footprint |
| **Ownership** | 6 cross-reference indices in common header | Implicit (footprint storage = owner) |
| **WideStrings** | Binary TLV `[index][len][UTF-16LE]` | Parameter blocks `ENCODEDTEXT0=byte,byte,...` |
| **Models location** | `/Models/` at root | `/Library/Models/` |
| **Sidecar scope** | Board-wide (global) | Per-footprint |
| **Pattern name block** | N/A | First block in each footprint Data stream |
| **SectionKeys** | Not used | Maps long footprint names to 31-char keys |
| **Section count** | 42+ always-present + conditionals | 5-7 per footprint + library-global |
| **Nets, rules, classes** | Full sections | Not present |
| **ComponentParamsTOC** | Not present | `/Library/ComponentParamsTOC/` |

See [shared-with-pcblib.md](shared-with-pcblib.md) for the complete overlap analysis.

---

## 11. Implementation Status (altium-cli)

### Currently implemented

- **Parse**: Full PcbDoc loading pipeline — 92/96 test files pass validation
- **Primitive serialization (partial)**: `write_primitive_section()`, `serialize_common_header()`,
  and `serialize_primitive_payload()` exist for Track only
- **Validation**: `validate_invariants()` and `validate_pcbdoc_primitive_coords()`
- **Save stub**: `PcbDoc::save()` returns a hard error — intentionally disabled

### Not yet implemented

- Full primitive serialization for all 8 types (only Track has a serializer)
- Parameter section export (Nets6, Components6, etc.)
- Prefixed parameter section export (Rules6, Dimensions6)
- WideStrings6 binary TLV export
- Sidecar stream export (UniqueID, ExtendedPrimitiveInfo, PrimitiveGuids)
- Board6 section export
- FileHeader / FileHeaderSix write
- Models section export
- Ownership index computation
- Feature flag management

### Blocked items

- 2 test files fail with missing `/FileHeaderSix` (pre-V6 legacy format)
- 2 test files fail with unimplemented DRC violation types (`TMaxMinViaHoleSizeViolation`,
  `TMaxMinPadSlotWidthViolation`)

---

## 12. Implementation Checklist

### Layer 1: Primitive Serializers (extend existing)

- [x] Common header (13 bytes): `serialize_common_header()`
- [x] Track serializer
- [ ] Arc serializer
- [ ] Pad serializer (6 subrecords)
- [ ] Via serializer
- [ ] Text serializer (2 subrecords)
- [ ] Fill serializer
- [ ] Region serializer (variable: vertex array)
- [ ] ComponentBody serializer (variable: outline + model ref)

### Layer 2: Section Writers

- [ ] Standard parameter section writer (generic: Nets6, Components6, etc.)
- [ ] Prefixed parameter section writer (Rules6, Dimensions6, Coordinates6)
- [ ] Board6 section export (special: multi-pass, large parameter block)
- [ ] WideStrings6 binary TLV builder
- [ ] UnionNames section writer
- [ ] PrimitiveParameters hierarchical writer
- [ ] SharedUnions binary writer

### Layer 3: Sidecar Streams

- [ ] UniqueIDPrimitiveInformation parameter block builder
- [ ] ExtendedPrimitiveInformation parameter block builder
- [ ] PrimitiveGuids 24-byte record packer
- [ ] PrimitiveParameters hierarchical group builder

### Layer 4: Global Streams

- [ ] FileHeader (UTF-16LE legacy)
- [ ] FileHeaderSix (pascal-block V6)
- [ ] FileVersionInfo
- [ ] Feature flag computation and storage
- [ ] Models section (metadata + blob streams)
- [ ] EmbeddedFonts6

### Layer 5: CFB Assembly

- [ ] `PcbDoc::save(path) -> Result<()>` — create CFB, write all sections in order
- [ ] Ownership index computation (6 cross-references per primitive)
- [ ] IndexForSave assignment per section
- [ ] Empty section creation (all 42+ always-present sections)

### Layer 6: Testing

- [ ] Per-primitive roundtrip tests (parse → serialize → compare)
- [ ] Per-section roundtrip tests
- [ ] Full document `assert_cfb_files_semantic_eq` roundtrip
- [ ] Property tests for primitive serializers

---

## 13. Source References

### .NET Decompiled Sources (AD26-dotnet/)

| File | Purpose |
|------|---------|
| `Altium.Edp.Interfaces/RT_PCB/IPCB_StructuredStorage.cs` | Top-level save interface |
| `Altium.Edp.Interfaces/RT_PCB/IPCB_BinarySection.cs` | Section base: PrepareToSave, Export_ToFile |
| `Altium.Edp.Interfaces/RT_PCB/IPCB_BoardBinarySection.cs` | Board6 section |
| `Altium.Edp.Interfaces/RT_PCB/IPCB_IndexForSaveIndexer.cs` | Index assignment interface |
| `Altium.Edp.Interfaces/RT_PCB/IPCB_BoardConstraintManager.cs` | ConstraintManager serialization |
| `Altium.Edp.Interfaces/PCBInterfaces/IPCB_Board_SaveLoadParameters.cs` | Board save/load |
| `Altium.Edp.Interfaces/PCBInterfaces/IPCB_Primitive_SaveLoadParameters.cs` | IndexForSave |
| `Altium.Edp.Interfaces/PCBInterfaces/IPCB_Text_SaveLoadParameters.cs` | WideString index |
| `Altium.Edp.Interfaces/RT_PCB/TPrimitiveGUID.cs` | 24-byte GUID struct |
| `Altium.Edp.Interfaces/RT_PCB/TPrimitiveKey.cs` | 8-byte primitive key |
| `Altium.Edp.Interfaces/RT_PCB/TReferenceToGroup.cs` | 16-byte extended index |
| `Altium.Edp.Interfaces/RT_PCB/TStorageFeature.cs` | Feature flags enum |
| `Altium.Edp.Interfaces/RT_PCB/TAdvPCBFileFormatVersion.cs` | Format version enum |
| `Altium.Edp.Interfaces/xPCBTypes/Consts.cs` | Constants, WideStringObjects |

### Delphi Binary (via Ghidra, project altium26)

| Binary | Address | Purpose |
|--------|---------|---------|
| `Altium.PCB.BinaryLoader.dll` | `0x01b455d0` | `BinaryFile_OpenWrite` — create CFB + FileHeader |
| `Altium.PCB.BinaryLoader.dll` | `0x01b45d40` | `Export_ToFile_PreProcess` — Board6 + features |
| `Altium.PCB.BinaryLoader.dll` | `0x018825f0` | `TPrimitivesSection.Export_ToFile` — main export loop |
| `Altium.PCB.BinaryLoader.dll` | `0x01882500` | `WritePrimitive` — per-record binary write |
| `Altium.PCB.BinaryLoader.dll` | `0x01882360` | `HandleExtraPrimitives` — >32KB sub-object split |
| `Altium.PCB.BinaryLoader.dll` | `0x018a0320` | `Section_ExportSetup` — create Header/Data streams |
| `Altium.PCB.BinaryLoader.dll` | `0x018a0c40` | `Section_ExportFinalize` — close streams |
| `Altium.PCB.BinaryLoader.dll` | `0x01918020` | `RegisterAllSectionsForExporting` — section list |

### Existing Codebase (Reference)

| File | Purpose |
|------|---------|
| `crates/altium-format/src/pcbdoc/mod.rs` | PcbDoc loading + save stub + partial serialization |
| `crates/altium-format/src/pcbdoc/records.rs` | Section kind enums, record types |
| `crates/altium-format/src/pcbdoc/primitives.rs` | Primitive type definitions |
| `crates/altium-format/src/pcblib/mod.rs` | PcbLib save (working reference implementation) |

### Other PcbDoc Documentation

| File | Contents |
|------|----------|
| [cfb-structure.md](cfb-structure.md) | Complete CFB layout and section classification |
| [fileheader.md](fileheader.md) | FileHeader and FileHeaderSix format |
| [board-section.md](board-section.md) | Board6 parameter keys (~2700 keys) |
| [binary-primitives.md](binary-primitives.md) | Binary layouts for all primitive types |
| [parameter-sections.md](parameter-sections.md) | Parameter keys per section |
| [sidecar-streams.md](sidecar-streams.md) | WideStrings6, UniqueID, PrimitiveGuids formats |
| [loading-pipeline.md](loading-pipeline.md) | Load pipeline (inverse of this document) |
| [shared-with-pcblib.md](shared-with-pcblib.md) | PcbDoc ↔ PcbLib overlap analysis |
| [enumerations.md](enumerations.md) | All PCB enumerations |
