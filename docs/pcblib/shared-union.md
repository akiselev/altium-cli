# SharedUnion Sidecar Stream (PcbLib)

## Overview

The `SharedUnion` stream is a **PcbLib footprint-level sidecar** that stores union grouping
information for primitives. It associates an "owner" primitive (typically a Pad) with child
primitives (typically paste/mask Regions) that share properties across layers.

In PcbDoc, the equivalent functionality is provided by the board-level `SmartUnions` section
(standard block-encoded parameter section) along with `UnionNames` and `UnionRelations`.
The PcbLib `SharedUnion` stream uses a different, simpler format.

**CFB path**: `/<FootprintName>/SharedUnion`

**Presence**: Optional. Only footprints with union-grouped primitives have this stream.
Most footprints do not have it.

**Related Parameters stream keys** (in `/<FootprintName>/Parameters`):
- `SMARTUNIONSSTORAGE` — optional metadata string
- `SMARTUNION_*` — optional prefixed parameters

These Parameters keys are metadata; the actual union data is in the SharedUnion stream.

## Binary Format

The stream does **NOT** use standard Altium block encoding. It uses a custom
count + length-prefixed parameter string format.

### Stream layout

```
u32 LE: entry_count         // Number of SharedUnion entries

For each entry:
  u32 LE: header_len        // Byte length of header parameter string (including NUL)
  [u8; header_len]:          // Header params (NUL-terminated, pipe-delimited)
                             //   Starts with leading '|'
                             //   Contains: PRIMITIVEINDEX, OBJECTID, and child info

  // If header contains HIDDENPRIMITIVESCOUNT=N (N > 0):
  For each hidden primitive (N times):
    u32 LE: prim_len         // Byte length of primitive parameter string (including NUL)
    [u8; prim_len]:           // Primitive params (NUL-terminated, pipe-delimited)
                              //   NO leading '|' (starts directly with key name)
                              //   Full inline primitive description
```

### Header parameter keys

| Key | Example | Description |
|-----|---------|-------------|
| `PRIMITIVEINDEX` | `3` | 0-based index of the owner primitive in the footprint |
| `OBJECTID` | `Pad` | Object type name of the owner primitive |
| `HIDDENPRIMITIVESCOUNT` | `1` | Number of inline hidden primitives following (mutually exclusive with PRIMITIVESCOUNT) |
| `PRIMITIVESCOUNT` | `1` | Number of referenced child primitives (mutually exclusive with HIDDENPRIMITIVESCOUNT) |
| `REF{i}INDEX` | `15` | 0-based primitive index of the i-th referenced child |
| `REF{i}OBJID` | `Region` | Object type name of the i-th referenced child |

### Two child-reference modes

1. **Hidden primitives** (`HIDDENPRIMITIVESCOUNT > 0`): Child primitives are stored
   inline in the stream as full parameter strings. These primitives do NOT exist in
   the footprint's main `Data` stream — they are "hidden" and only accessible via
   the SharedUnion sidecar.

2. **Referenced primitives** (`PRIMITIVESCOUNT > 0`): Child primitives already exist
   in the footprint's main `Data` stream. The header references them by index
   (`REF{i}INDEX`) and object type (`REF{i}OBJID`).

### Hidden primitive parameter keys

Hidden primitives are full primitive descriptions. The key set depends on `PRIMITIVEOBJECTID`.
For Region (type 11), commonly observed keys:

| Key | Example | Description |
|-----|---------|-------------|
| `PRIMITIVEOBJECTID` | `11` | Numeric object type ID (NOT text name like header uses) |
| `SELECTION` | `FALSE` | Selection state |
| `LAYER` | `TOPPASTE` | Layer name |
| `LOCKED` | `FALSE` | Lock state |
| `POLYGONOUTLINE` | `FALSE` | Polygon outline flag |
| `USERROUTED` | `TRUE` | User-routed flag |
| `KEEPOUT` | `FALSE` | Keepout flag |
| `UNIONINDEX` | `0` | Union index |
| `SOLDERMASKEXPANSIONMODE` | `None` | Mask expansion mode |
| `PASTEMASKEXPANSIONMODE` | `None` | Paste mask expansion mode |
| `NAME` | ` ` | Name (often blank) |
| `KIND` | `0` | Region kind |
| `SUBPOLYINDEX` | `-1` | Sub-polygon index |
| `ARCRESOLUTION` | `0.1mil` | Arc resolution |
| `ISSHAPEBASED` | `TRUE` | Shape-based region flag |
| `MAINCONTOURVERTEXCOUNT` | `5` | Number of contour vertices |
| `KIND{i}` | `0` | Vertex i kind (0=line, 1=arc) |
| `VX{i}` | `-15.748mil` | Vertex i X coordinate |
| `VY{i}` | `10.8268mil` | Vertex i Y coordinate |
| `CX{i}` | `0mil` | Vertex i arc center X |
| `CY{i}` | `0mil` | Vertex i arc center Y |
| `SA{i}` | ` 0.00000000000000E+0000` | Vertex i start angle |
| `EA{i}` | ` 0.00000000000000E+0000` | Vertex i end angle |
| `R{i}` | `0mil` | Vertex i arc radius |
| `HOLECOUNT` | `0` | Number of holes in region |

## Worked Examples

### Example 1: TO263-5L (87 bytes, referenced primitives)

```
Stream: /TO263-5L/SharedUnion (87 bytes)

00000000: 01 00 00 00                  count = 1
00000004: 4f 00 00 00                  header_len = 79
00000008: |PRIMITIVEINDEX=7|OBJECTID=Pad|PRIMITIVESCOUNT=1|REF0INDEX=15|REF0OBJID=Region\0
```

Pad at index 7 references Region at index 15 (already in the Data stream).

### Example 2: WSON-6 (959 bytes, hidden primitives)

```
Stream: /WSON-6/SharedUnion (959 bytes)

00000000: 01 00 00 00                  count = 1
00000004: 37 00 00 00                  header_len = 55
00000008: |PRIMITIVEINDEX=3|OBJECTID=Pad|HIDDENPRIMITIVESCOUNT=1\0

0000003f: 7c 03 00 00                  prim_len = 892
00000043: PRIMITIVEOBJECTID=11|SELECTION=FALSE|LAYER=TOPPASTE|LOCKED=FALSE|
          POLYGONOUTLINE=FALSE|USERROUTED=TRUE|KEEPOUT=FALSE|UNIONINDEX=0|
          SOLDERMASKEXPANSIONMODE=None|PASTEMASKEXPANSIONMODE=None|NAME= |
          KIND=0|SUBPOLYINDEX=-1|ARCRESOLUTION=0.1mil|ISSHAPEBASED=TRUE|
          MAINCONTOURVERTEXCOUNT=5|KIND0=0|VX0=-15.748mil|VY0=10.8268mil|
          ... (contour vertices) ...|HOLECOUNT=0\0
```

Pad at index 3 has 1 hidden Region (TOPPASTE layer) with a 5-vertex custom paste shape
stored inline. This region does NOT appear in the footprint's Data stream.

## Relationship to PcbDoc Unions

| PcbLib (per-footprint) | PcbDoc (board-level) | Stream name | Format |
|------------------------|----------------------|-------------|--------|
| `SharedUnion` stream | `Section_SharedUnions` | PcbLib: `/<fp>/SharedUnion`; PcbDoc: `/Section_SharedUnions/Data` | count + len-prefixed param strings |
| Parameters: `SMARTUNIONSSTORAGE` | `Section_UnionNames` | `/Section_UnionNames/Data` | u32 count + u32 index + UTF-16LE name |
| Parameters: `SMARTUNION_*` | `Section_SmartUnions` | `/Section_SmartUnions/Data` | Standard block-encoded params |
| — | `Section_UnionRelations` | `/Section_UnionRelations/Data` | Binary i32 pairs (see below) |

### PcbDoc Section_SharedUnions

**Delphi class**: `TSharedUnionSection` (VMT at 0x04897d28 in Advpcb.dll)

Uses the same format as PcbLib SharedUnion: count + length-prefixed parameter strings with
the same header keys (`PRIMITIVEINDEX`, `OBJECTID`, `HIDDENPRIMITIVESCOUNT`/`PRIMITIVESCOUNT`,
`REF{i}INDEX`, `REF{i}OBJID`) and optional inline hidden primitive blocks.

**Save** (FUN_04898ca0): For each entry, writes header params, then if hidden primitives
exist, writes `HIDDENPRIMITIVESCOUNT` and each hidden primitive as `PRIMITIVEOBJECTID` +
full primitive params in separate parameter blocks.

**Load** (FUN_04899210): Reads entry count, then for each entry reads header, checks
`HIDDENPRIMITIVESCOUNT`, reads that many hidden primitive blocks.

### PcbDoc Section_UnionRelations (binary format)

**Delphi class**: `TUnionRelationsSection` (VMT at 0x0490fc88 in Advpcb.dll)

This section stores **binary pairs of i32 values**, NOT parameter-based strings.

```
u32 LE: count              // from block header

For each entry:
  i32 LE: parent_id        // parent union index
  i32 LE: child_id         // child union index
```

Internal data structure is `TDictionary<Integer, Pointer>` mapping union indices to
relation data. The `SetRelation(parentID, childID)` method on `IPCB_BoardUnionManager2`
populates this.

## C# Interface Hierarchy

```
IPCB_Union
  - GetState_Name() -> string
  - GetState_Type() -> TUnionTypeID
  - GetState_UnionIndex() -> int
  - GetState_DeadCopper() -> bool

IPCB_SharedUnion
  - GetState_OwnerPrimitive() -> IPCB_Primitive
  - GetState_Primitives() -> IPCB_PrimitiveList
  - GetState_PrimitiveLockOnLayer(layer) -> bool
  - GetState_LayerIsVisible(layer) -> bool
  - GetState_ID() -> int
  - GetState_ActiveLayer() -> TV7_Layer
  - LinkPrimitive(prim) / UnLinkPrimitive(prim)
  - MoveByXY(x, y) / RotateAroundXY(x, y, angle)

IPCB_SmartUnionObject : IPCB_Primitive
  - GetState_UnionType() -> TSmartUnionObjectType
  - GetState_PrimitivesCount() -> int
  - GetState_Primitive(index) -> IPCB_Primitive
  - RebuildAfterLoad()
```

### TUnionTypeID (Delphi enum, u8)

| Value | Name | Description |
|-------|------|-------------|
| 0 | `eUnionTypeID_SimpleUnion` | Basic union |
| 1 | `eUnionTypeID_DrillTable` | Drill table |
| 2 | `eUnionTypeID_LayerStackTable` | Layer stack table |
| 3 | `eUnionTypeID_NetLengthTuning` | Net length tuning |
| 4 | `eUnionTypeID_ViaStitching` | Via stitching pattern |
| 5 | `eUnionTypeID_ViaShielding` | Via shielding |
| 6 | `eUnionTypeID_SmartPaste` | Smart paste union |
| 7 | `eUnionTypeID_StackedMicroVia` | Stacked micro-via |
| 8 | `eUnionTypeID_StaggeredMicroVia` | Staggered micro-via |
| 9 | `eUnionTypeID_DiffPairLengthTuning` | Diff pair length tuning |
| 10 | `eUnionTypeID_Rectangle` | Rectangle union |
| 11 | `eUnionTypeID_ReuseBlock` | Reuse block |

### TSmartUnionObjectType (Delphi enum, u8)

| Value | Name |
|-------|------|
| 0 | `eUnknown` |
| 1 | `eDrillTable` |
| 2 | `eViaStitching` |
| 3 | `eLayerStackTable` |
| 4 | `eAccordion` |
| 5 | `eMetaDataSmartUnion` |
| 6 | `eViaShielding` |
| 7 | `eMultilineText` |
| 8 | `eAccordionDiffPair` |
| 9 | `eRectangleSmartUnion` |
| 10 | `eReuseBlock` |

## Delphi Internals (from Ghidra decompilation)

### Key classes (RTTI names from Advpcb.dll)

| Class | Purpose |
|-------|---------|
| `TSharedUnion` | Runtime SharedUnion object |
| `TSharedUnionHolder` | Container for SharedUnion |
| `TSharedUnionInternal` | Internal implementation |
| `TSharedUnionSection` | PcbDoc section for SharedUnion data |
| `TSmartUnionsSection` | PcbDoc section for SmartUnion data |
| `TUnionNamesSection` | PcbDoc section for union names |
| `TUnionRelationsSection` | PcbDoc section for union relations |
| `TSmartUnionObject` | Board-level smart union primitive |
| `TSmartUnionObjectAdapter` | Adapter for save/load |
| `SmartUnionsStorageImplementation` | Storage impl for PcbLib level |
| `SharedUnionAggregateItem` | Aggregate item in BinaryLoader |

### SharedUnionInformation object layout (from FUN_025aa280 constructor)

| Offset | Type | Init value | Description |
|--------|------|------------|-------------|
| 0x08 | object | — | Reference/name object |
| 0x10 | i32 | -1 (0xFFFFFFFF) | PrimitiveIndex |
| 0x14 | u8 | 0 | ObjectID |
| 0x18 | list | — | List of child primitives |

### Serialization functions (Advpcb.dll)

- **Export** (FUN_025ab640): Writes `PrimitiveIndex` (i32), `ObjectID` (byte as string),
  then `PrimitivesCount` + `Ref{i}Index`/`Ref{i}ObjId` pairs for referenced children.
- **Import** (FUN_025ab990): Reads `PrimitiveIndex` → offset 0x10, `ObjectID` → offset 0x14,
  then reads primitive references.

### Primitive-level SharedUnion property

In BinaryLoader.dll, `SharedUnion` is a **published Delphi property** on primitive adapter
classes (RTTI at 0x01846e25). The getter (FUN_0184e710) returns a boolean indicating whether
the primitive participates in a SharedUnion, calling the primitive's virtual method at vftable
offset 0x1e0.

### Board ↔ footprint context transfer

Two methods handle moving SmartUnion state between board and footprint contexts:
- `SmartUnionsOntoBoard` (string at 0x03cf3b77): Transfers union data from footprint to board
- `SmartUnionsBackFromBoard` (string at 0x03cf3b17): Transfers union data from board back to footprint

## PcbLib Save/Load Interface

The `IPCB_LibComponent_SaveLoadParameters` interface provides:

```csharp
void SmartUnions_Export_ToParameter(IWideParameterList argParameters);
void SmartUnions_Import_FromParameter(IWideParameterList argParameters);
```

This confirms the SharedUnion stream uses parameter-list serialization (pipe-delimited
key=value pairs), NOT binary struct serialization.

## Current Implementation Status

- **PcbLib**: Hard error when SharedUnion stream is encountered (`footprint.rs:289-299`)
- **PcbDoc SmartUnions**: Enumerated in `ParamSectionKind` but not parsed
- **PcbDoc UnionNames**: Parsed (`parse_union_name_records` in `records.rs`)
- **PcbDoc UnionRelations**: Enumerated but not parsed

## Source References

- `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_SharedUnion.cs`
- `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_Union.cs`
- `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TUnionTypeID.cs`
- `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TSmartUnionObjectType.cs`
- `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_SmartUnionObject.cs`
- `AD26-dotnet/Altium.Edp.Interfaces/PCBInterfaces/IPCB_LibComponent_SaveLoadParameters.cs`
- `AD26-dotnet/Altium.SDK.Interfaces/PCB/IPCB_SharedUnionHelper.cs`
- `crates/altium-format/src/pcblib/footprint.rs` (current hard-error handler)
- `crates/altium-format/src/pcbdoc/records.rs` (UnionNames parser)

### Delphi (Ghidra decompilation, project: altium26)

- `Advpcb.dll` FUN_025ab640: SharedUnionInformation.Export_ToParameters
- `Advpcb.dll` FUN_025ab990: SharedUnionInformation.Import_FromParameters
- `Advpcb.dll` FUN_04898ca0: TSharedUnionSection.Save (PcbDoc)
- `Advpcb.dll` FUN_04899210: TSharedUnionSection.Load (PcbDoc)
- `Advpcb.dll` FUN_049101a0: TUnionRelationsSection.Save
- `Advpcb.dll` FUN_049100e0: TUnionRelationsSection.Load
- `Altium.PCB.BinaryLoader.dll` FUN_018cf410: SharedUnion section save (BinaryLoader)
- `Altium.PCB.BinaryLoader.dll` FUN_018cf980: SharedUnion section load (BinaryLoader)
- `Altium.PCB.BinaryLoader.dll` FUN_018464e5: NeedsToProcessSharedUnion
