# ExtendedPrimitiveIndices & Testpoint Options

Research findings for two PcbDoc CFB sections: `ExtendedPrimitiveIndices` (stream index 38)
and `Testpoint Options` (stream index 36).

---

## 1. ExtendedPrimitiveIndices

### Overview

CFB stream name: `ExtendedPrimitiveIndices`
Section enum name: `Section_ExtendedPrimitiveIndices` (from Delphi section registry)
Stream index: 38 (in the PCB stream name table)

This is a **sidecar stream** that provides a fast lookup index into the
`ExtendedPrimitiveInformation` section. It maps each primitive to its corresponding
group entry in ExtendedPrimitiveInformation, avoiding linear scans when looking up a
specific primitive's extended properties (mask expansion mode, etc.).

### Relationship to ExtendedPrimitiveInformation

The `ExtendedPrimitiveInformation` section (stream index 37) stores mask expansion
settings per primitive using parameter blocks. Each block has a `PRIMITIVEINDEX` key
that identifies the primitive. The `ExtendedPrimitiveIndices` section duplicates this
mapping in a binary lookup table for O(1) access. The `Data` stream itself is
self-contained -- the indices are purely an optimization.

### Binary Format

The section uses the standard PcbDoc section layout: `Header` + `Data` streams.

**Header**: 4 bytes `u32 LE` -- entry count (number of `TReferenceToGroup` entries in Data).

**Data**: Array of `TReferenceToGroup` entries, each 16 bytes.

From `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TReferenceToGroup.cs`:

```csharp
[StructLayout(LayoutKind.Sequential, Pack = 8)]
public struct TReferenceToGroup
{
    public TPrimitiveKey Prim;       // 8 bytes
    public TPrimitiveKey PrimGroup;  // 8 bytes
}
```

From `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TPrimitiveKey.cs`:

```csharp
[StructLayout(LayoutKind.Sequential, Pack = 1)]
public struct TPrimitiveKey
{
    public int ObjectId;       // 4 bytes, i32 LE -- TObjectId enum value
    public int IndexForSave;   // 4 bytes, i32 LE -- primitive index within its section
}
```

Binary layout per entry (16 bytes total):

```
Offset  Size  Field                  Description
------  ----  ---------------------  -------------------------------------------
0x00    4     Prim.ObjectId          i32 LE: TObjectId of the primitive
0x04    4     Prim.IndexForSave      i32 LE: index in that primitive's section
0x08    4     PrimGroup.ObjectId     i32 LE: TObjectId of the group entry
0x0C    4     PrimGroup.IndexForSave i32 LE: index in ExtendedPrimitiveInformation
```

### API on IPCB_BinarySection

The `IPCB_BinarySection` interface (used by all primitive sections) includes methods
for managing extended indices:

```csharp
int ExtendedIndexCount();
TReferenceToGroup GetExtendedIndex(int argIndex);
void AddExtendedIndex(TReferenceToGroup argData);
void ApplyExtendedIndices();
```

These are called during Import/Export to read/write the sidecar alongside the
primitive data.

### TStorageFeature Flag

`TStorageFeature.eHasExtendedGroupIndicesAreUsed` (value 23) controls whether this
section is present. When the flag is set in `FileVersionInfo`, the loader reads
`ExtendedPrimitiveIndices`; otherwise the section is absent.

### Occurrence in Test Files

Not found in any of the existing test PcbDoc files in `data/pcbdoc/`. This section
is likely only present in boards that have explicit mask expansion overrides on
individual primitives (as opposed to using design rules). It is an optimization
sidecar and can be regenerated from `ExtendedPrimitiveInformation/Data` alone.

### Implementation Notes

For implementation:
- The section can be treated as a simple Header/Data binary section.
- Header: `u32 count`
- Data: `count` x 16-byte `TReferenceToGroup` entries
- On save: regenerate from `ExtendedPrimitiveInformation` entries by building
  (PrimitiveKey -> GroupKey) mappings.
- The section is optional and purely an optimization. If absent, lookups into
  ExtendedPrimitiveInformation fall back to linear scanning via `PRIMITIVEINDEX`.

---

## 2. Testpoint Options

### Overview

CFB stream name: `Testpoint Options` (note: has a space in the name)
Section enum name: `Section_Testpoint` (from Delphi section registry)
Stream index: 36 (in the PCB stream name table)
Options Object ID: `eTestpointOptions` = 14 (byte) in `TOptionsObjectId` enum

This section stores the board-level testpoint assignment configuration. It belongs to
the `IPCB_OptionsList` system alongside other option sets (printer, gerber, DRC, etc.).

### Data Format

Testpoint options use the standard PCB options serialization mechanism:
- `Import_FromParameters(displayUnit, parameterString)` / `Export_ToParameters(parameterString)`
- The data is stored as a single **parameter block** (pipe-delimited `|KEY=VALUE|` string)
  in the standard section layout (`Header` + `Data` streams).

The `Header` is a 4-byte `u32 LE` with value 1 (single record).
The `Data` stream contains one `u32 LE` length prefix + parameter payload string.

### Interface: IPCB_TestPointOptions

From `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_TestPointOptions.cs`:

```csharp
public interface IPCB_TestPointOptions : IPCB_AbstractOptions
{
    TOptionsObjectId GetState_ObjectID();
    void SetState_ObjectID(TOptionsObjectId argOId);
    void Import_FromParameters(TUnit argDisplayUnit, StringBuilder argParameters);
    void Export_ToParameters(StringBuilder argParameters);
    // Also has Version3 and Version4 import/export variants
    nint I_ObjectAddress();
}
```

The interface extends `IPCB_AbstractOptions` which defines the generic parameter
serialization contract. The `IPCB_Board_SaveLoadParameters` interface provides
access via `GetState_TestPointOptions()`.

### Known Testpoint Parameters (from Related Interfaces)

While the exact parameter keys for the options block are in the Delphi implementation
(not decompiled), the test point RULES provide strong clues about what's configured:

**From IPCB_TestPointStyleRule** (rule-level, but likely mirrored in options):
- `TestpointUnderComponent` (bool) -- allow testpoints under components
- `MinSize` / `MaxSize` / `PreferedSize` (Coord) -- pad/via size constraints
- `MinHoleSize` / `MaxHoleSize` / `PreferedHoleSize` (Coord) -- hole size constraints
- `UseGrid` (bool) -- snap testpoints to grid
- `GridOrigin` (TCoordPoint) -- grid origin X,Y
- `TestpointGrid` (Coord) -- grid spacing
- `GridTolerance` (Coord) -- snap tolerance
- `MinSpacing` (Coord) -- minimum spacing between testpoints
- `CompBodyClearance` (Coord) -- clearance to component bodies
- `BoardEdgeClearance` (Coord) -- clearance to board edge
- `AllowedSide` (TTestpointAllowedSideSet) -- which board sides are allowed
- `DistanceToViaHoleCenter` (Coord) -- min distance to nearest via hole center
- `DistanceToPadHoleCenter` (Coord) -- min distance to nearest pad hole center

**From IPCB_TestPointUsage** (rule-level):
- `Valid` (TTestpointValid) -- require/invalid/ignore/requireAtLeafs
- `AllowMultipleOnNet` (bool) -- allow multiple testpoints per net

### Related Enums

**TTestpointValid** (from `Pcbtypes/TTestpointValid.cs`):
```
eRequire = 0        -- net must have a testpoint
eInvalid = 1        -- testpoint not valid here
eIgnore = 2         -- don't care
eRequireAtLeafs = 3 -- only in SDK.Interfaces version (AD26 extension?)
```

**TTestpointAllowedSide** (from `Pcbtypes/TTestpointAllowedSide.cs`):
```
eAllowTopSide = 0
eAllowBottomSide = 1
```

**TTestPoint_PadViaType** (from `xPCBTypes/TTestPoint_PadViaType.cs`):
```
ePadType_ThruHole = 0
ePadType_SMD = 1
ePadType_Via = 2
```

### TOptionsObjectId Enum

Full enum from `RT_PCB/TOptionsObjectId.cs`:
```
eAbstractOptions = 0
eOutputOptions = 1
ePrinterOptions = 2
eGerberOptions = 3
eAdvancedPlacerOptions = 4
eDesignRuleCheckerOptions = 5
eSpecctraRouterOptions = 6
eAdvancedRouterOptions = 7
eEngineeringChangeOrderOptions = 8
eInteractiveRoutingOptions = 9
eSystemOptions = 10
ePinSwapOptions = 11
eEdgeLayoutOptions = 12
eSplitpPlaneDRCOptions = 13
eTestpointOptions = 14         <-- this one
```

### Primitive Testpoint Attributes

Testpoint status is stored as boolean flags on individual primitives (pads and vias):

**From TPrimitiveAttribute enum** (short values):
- `ePrimitiveAttribute_TestpointTop` (53) -- "Testpoint - Top"
- `ePrimitiveAttribute_TestpointBottom` (54) -- "Testpoint - Bottom"
- `ePrimitiveAttribute_FabTestpointTop` (55) -- "Fabrication Testpoint - Top"
- `ePrimitiveAttribute_FabTestpointBottom` (56) -- "Fabrication Testpoint - Bottom"
- `ePrimitiveAttribute_AssyTestpointTop` (57) -- "Assembly Testpoint - Top"
- `ePrimitiveAttribute_AssyTestpointBottom` (58) -- "Assembly Testpoint - Bottom"

These are already implemented in the codebase as `is_testpoint_top`, `is_testpoint_bottom`,
`is_assy_testpoint_top`, `is_assy_testpoint_bottom` fields on via and pad primitives.

The serialization parameter keys are:
- `"TestpointTop"` / `"TestpointBottom"` (fabrication testpoints)
- `"TestpointFabTop"` / `"TestpointFabBottom"` (alt names for fabrication)
- `"TestpointAssyTop"` / `"TestpointAssyBottom"` (assembly testpoints)

### Testpoint Rule Categories

From `TRuleKind` enum:
- `eRule_TestPointStyle` = 48 (fabrication testpoint style)
- `eRule_TestPointUsage` = 49 (fabrication testpoint usage)
- `eRule_AssyTestPointStyle` = 62 (assembly testpoint style)
- `eRule_AssyTestPointUsage` = 63 (assembly testpoint usage)

These are grouped under `TRuleCategory.eRuleCategory_Testpoint`.

### Occurrence in Test Files

Not found in any existing test PcbDoc files in `data/pcbdoc/`. The section is likely
only present in boards where testpoint options have been explicitly configured (most
community-sourced boards don't use testpoint management).

### Implementation Notes

For implementation:
- Standard single-record parameter section: Header (u32 count=1) + Data (u32 len + params).
- The parameters follow the `IPCB_AbstractOptions.Export_ToParameters()` contract:
  pipe-delimited `|KEY=VALUE|` in Windows-1252 encoding.
- The exact parameter key names are in the Delphi implementation; they likely mirror
  the style/usage rule parameters with appropriate prefixes.
- This section is optional. If absent, testpoint options use defaults.
- Register as `ParamSectionKind::TestpointOptions` with stream name `"Testpoint Options"`.

---

## 3. Related: Testpoint Rules (Already Implemented)

The testpoint RULES (as opposed to OPTIONS) are already implemented in the codebase:

**In `crates/altium-format/src/pcbdoc/drc.rs`:**
- `TestpointStyleRuleData` -- struct with fields: `testpoint_under_component`, `min_size`,
  `max_size`, `prefered_size`, `min_hole_size`, `max_hole_size`, `prefered_hole_size`,
  `use_grid`, `grid_origin`, `testpoint_grid`, `grid_tolerance`, `min_spacing`,
  `comp_body_clearance`, `board_edge_clearance`, `allowed_side`,
  `distance_to_via_hole_center`, `distance_to_pad_hole_center`
- `TestpointUsageRuleData` -- struct with fields: `valid` (TestpointValid),
  `allow_multiple_on_net` (bool)

**PcbRuleKindData variants:**
- `FabricationTestpointStyle(TestpointStyleRuleData)`
- `FabricationTestpointUsage(TestpointUsageRuleData)`
- `AssyTestPointStyle(TestpointStyleRuleData)`
- `AssyTestPointUsage(TestpointUsageRuleData)`

---

## 4. Summary

| Aspect | ExtendedPrimitiveIndices | Testpoint Options |
|--------|------------------------|-------------------|
| Stream name | `ExtendedPrimitiveIndices` | `Testpoint Options` |
| Stream index | 38 | 36 |
| Section enum | `Section_ExtendedPrimitiveIndices` | `Section_Testpoint` |
| Format | Binary: array of 16-byte `TReferenceToGroup` | Param: single `\|KEY=VALUE\|` block |
| Header | u32 count | u32 count (always 1) |
| Data | `count * sizeof(TReferenceToGroup)` raw bytes | u32 len + param string |
| Purpose | Fast index into ExtendedPrimitiveInformation | Board-level testpoint config |
| Required? | Optional (optimization sidecar) | Optional (defaults used if absent) |
| Feature flag | `eHasExtendedGroupIndicesAreUsed` (TStorageFeature 23) | None found |
| In test files? | No | No |
| Already impl? | No (but ExtendedPrimitiveInformation is) | No |
