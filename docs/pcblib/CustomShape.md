# CustomShapes Sidecar Streams (PcbLib)

Three related sidecar streams store custom pad shape data that extends the core pad binary records. These exist because the custom shape feature was added after the original pad binary format was established.

## Overview

| Stream | Section Index | Feature Flag | Purpose |
|--------|--------------|--------------|---------|
| `CustomShapes` | 69 | `eHasCustomPadShapesAtWriteStage` (9) | Custom pad shapes (chamfered rect, rounded rect, donut) |
| `CustomReliefs` | 71 | `eHasCustomReliefInfosAtWriteStage` (12) | Custom thermal relief shapes |
| `CustomMaskShapes` | 75 | `eHasCustomMaskInfosAtWriteStage` (16) | Custom solder/paste mask shapes |

An additional flag `eHasCustomPadShapesDonutAtWriteStage` (20) indicates donut-shaped custom pads are present.

These streams are **per-footprint** in PcbLib files, stored at `<FootprintName>/CustomShapes`, `<FootprintName>/CustomMaskShapes`, etc.

## Binary Format

All three streams share the same framing — the standard PcbLib parameter-block format:

```
[4 bytes]  u32 LE: entry count
For each entry:
  [4 bytes]  u32 LE: parameter string length (including NUL terminator)
  [N bytes]  NUL-terminated parameter string (pipe-delimited |KEY=VALUE|)
```

This is the same framing as WideStrings, UniqueIDPrimitiveInformation, and ExtendedPrimitiveInformation sidecars.

## CustomShapes Parameters

Each entry describes the custom shape configuration for one pad on one or more layers.

### Common Parameters

| Key | Type | Description |
|-----|------|-------------|
| `PRIMITIVEINDEX` | integer | 0-based index into the footprint's primitive list (identifies the pad) |

### Per-Layer Shape Parameters (prefix `S{N}.`)

The `S{N}` prefix supports multiple per-layer shape definitions, indexed from 0. In practice, `S0` is the most common (single-layer or "simple" pad mode). For local-stack pads with per-layer custom shapes, additional entries `S1`, `S2`, etc. would appear.

| Key | Type | Description |
|-----|------|-------------|
| `S{N}.LAYER` | string | Layer name (`TOP`, `BOTTOM`, `MID1`, etc.) |
| `S{N}.XSIZE` | integer | X size in internal units (Coord, 10000 = 1mil) |
| `S{N}.YSIZE` | integer | Y size in internal units (Coord) |
| `S{N}.SHAPEKIND` | integer | `TShapeSubKind` enum value (see below) |

### Corner Parameters (prefix `S{N}.CPS.`)

Present when `SHAPEKIND` is `eChamferedRectangle` (4) or `eRoundedRectangle` (3). `CPS` = Custom Pad Shape.

| Key | Type | Description |
|-----|------|-------------|
| `S{N}.CPS.BLCE` | boolean | Bottom-Left Corner Enabled (`TRUE`/`FALSE`) |
| `S{N}.CPS.BRCE` | boolean | Bottom-Right Corner Enabled |
| `S{N}.CPS.TRCE` | boolean | Top-Right Corner Enabled |
| `S{N}.CPS.TLCE` | boolean | Top-Left Corner Enabled |
| `S{N}.CPS.CS` | integer | Corner Size in internal units (0 = use percentage-based default) |

### TShapeSubKind Enum

From `AD26-dotnet/Altium.SDK.Interfaces/PCB/TShapeSubKind.cs`:

| Value | Name | Description |
|-------|------|-------------|
| 0 | `eNoKind` | No custom shape sub-kind |
| 1 | `eOctagonalFinger` | Octagonal finger shape |
| 2 | `eRoundedFinger` | Rounded finger shape |
| 3 | `eRoundedRectangle` | Rounded rectangle (selective corners) |
| 4 | `eChamferedRectangle` | Chamfered rectangle (selective corners) |
| 5 | `eDonut` | Donut (annular ring) shape |

### Additional Parameters (from Delphi string analysis, not yet observed in test data)

The following parameter names were found as strings in `Advpcb.dll` at addresses `0x00e2709c`–`0x00e27139`, suggesting they exist for specific shape sub-kinds:

| String | Likely Usage |
|--------|-------------|
| `CustomShape_RectanglesCorners` | Alternative corner specification format |
| `CustomShape_CornerRadiusAbsolute` | Absolute corner radius (vs percentage-based) |
| `CustomShape_Donut` | Donut-specific parameters (inner width, outer diameter) |

These likely appear as additional parameters when `SHAPEKIND=5` (donut) or when absolute corner radii are specified instead of percentage-based.

## CustomMaskShapes Parameters

Custom mask shapes override the pad's mask layer expansion with a custom outline. Uses the `SPM{N}` prefix (Solder Paste Mask).

| Key | Type | Description |
|-----|------|-------------|
| `PRIMITIVEINDEX` | integer | 0-based pad primitive index |
| `SPM{N}.LAYER` | string | Mask layer name (`TOPPASTE`, `BOTTOMPASTE`, `TOPSOLDER`, `BOTTOMSOLDER`) |
| `SPM{N}.SHAPE` | string | Shape type (`CUSTOM` for custom mask shape) |
| `SPM{N}.XSIZE` | integer | X size override in internal units |
| `SPM{N}.YSIZE` | integer | Y size override in internal units |

Additional mask-specific parameters may exist for shape kind, corner configuration, etc., following the same pattern as CustomShapes but with the `SPM{N}` prefix.

## CustomReliefs Parameters

Custom thermal relief shapes. No test data observed yet — format likely follows the same pattern with a relief-specific prefix.

## Observed Data

### Example 1: WSON-10B (FragasLab-Footprint.PcbLib)

File: `data/pcblib/FragasLab-Footprint.PcbLib`, stream: `/WSON-10B/CustomShapes` (660 bytes)

4 entries, each defining a rounded rectangle with selective corners enabled:

```
Entry count: 4

Entry 0 (160 bytes):
|PRIMITIVEINDEX=3|S0.LAYER=TOP|S0.XSIZE=275592|S0.YSIZE=110236|
S0.SHAPEKIND=3|S0.CPS.BLCE=FALSE|S0.CPS.BRCE=TRUE|S0.CPS.TRCE=TRUE|
S0.CPS.TLCE=FALSE|S0.CPS.CS=0

Entry 1 (160 bytes):
|PRIMITIVEINDEX=4|S0.LAYER=TOP|S0.XSIZE=275592|S0.YSIZE=110236|
S0.SHAPEKIND=3|S0.CPS.BLCE=TRUE|S0.CPS.BRCE=FALSE|S0.CPS.TRCE=FALSE|
S0.CPS.TLCE=TRUE|S0.CPS.CS=0

Entry 2 (160 bytes):
|PRIMITIVEINDEX=2|S0.LAYER=TOP|S0.XSIZE=275592|S0.YSIZE=110236|
S0.SHAPEKIND=3|S0.CPS.BLCE=FALSE|S0.CPS.BRCE=TRUE|S0.CPS.TRCE=TRUE|
S0.CPS.TLCE=FALSE|S0.CPS.CS=0

Entry 3 (160 bytes):
|PRIMITIVEINDEX=5|S0.LAYER=TOP|S0.XSIZE=275592|S0.YSIZE=110236|
S0.SHAPEKIND=3|S0.CPS.BLCE=TRUE|S0.CPS.BRCE=FALSE|S0.CPS.TRCE=FALSE|
S0.CPS.TLCE=TRUE|S0.CPS.CS=0
```

Pads 2–5 have rounded rectangle shapes with opposite corners enabled (creating a "D" pad shape). Entry order does NOT match primitive index order.

### Example 2: QFN40-68 (SMotlaq-PCB_lib.PcbLib)

File: `data/pcblib/SMotlaq-PCB_lib.PcbLib`, stream: `/QFN40-68/CustomShapes` (172 bytes)

```
Entry count: 1

Entry 0 (164 bytes):
|PRIMITIVEINDEX=20|S0.LAYER=TOP|S0.XSIZE=2362206|S0.YSIZE=2362206|
S0.SHAPEKIND=3|S0.CPS.BLCE=FALSE|S0.CPS.BRCE=FALSE|S0.CPS.TRCE=FALSE|
S0.CPS.TLCE=TRUE|S0.CPS.CS=0
```

### Example 3: CustomMaskShapes — WSON-6 (SMotlaq-PCB_lib.PcbLib)

File: `data/pcblib/SMotlaq-PCB_lib.PcbLib`, stream: `/WSON-6/CustomMaskShapes` (90 bytes)

```
Entry count: 1

Entry 0 (82 bytes):
|PRIMITIVEINDEX=3|SPM0.LAYER=TOPPASTE|SPM0.SHAPE=CUSTOM|
SPM0.XSIZE=0|SPM0.YSIZE=0
```

## Relationship to Pad Records

Custom shapes apply to pads whose `TShape` is `eCustomShape` (10). The pad's core binary record in the Data stream stores the base shape, but the actual custom shape geometry is defined in this sidecar:

1. Parser reads pad from Data stream — sees `shape = eCustomShape`
2. Parser reads CustomShapes sidecar — finds entry with matching `PRIMITIVEINDEX`
3. The `S{N}.SHAPEKIND`, corner flags, and size define the actual pad outline
4. At runtime, Altium constructs a region primitive from the custom shape parameters

For pads that are NOT `eCustomShape`, the CustomShapes sidecar has no entry.

## C# Interface Hierarchy

From decompiled .NET code in `AD26-dotnet/`:

```
IPCB_Pad4
  ├── HasCustomShapes() → bool
  ├── HasCustomMaskShapes() → bool
  ├── HasCustomDonut() → bool
  ├── HasCustomMaskDonutShapes() → bool
  └── LinkCustomShape(IPCB_Primitive)

IPCB_CustomPadShape (per-layer access)
  ├── GetProperty_CustomShape(layer) → IPCB_Primitive
  ├── GetState_RegionShapeOnLayer(layer) → IPCB_Region
  ├── GetProperty_CustomShapeKind(layer) → TShapeSubKind
  ├── GetState_CustomShapeInfo(layer) → IPCB_CustomShapeInfo
  └── SetCustomShapeDefaultsOnLayer(layer)

IPCB_CustomShapeInfo
  ├── GetState_ShapeKind() → TShapeSubKind
  ├── GetState_CustomShapeParameters() → object
  ├── ExportToParameters(params, prefix)
  └── ImportFromParameters(params, prefix)

IPCB_CustomShapeRectParameters
  ├── GetState_CornerEnabled(TRectCorner) → bool
  └── SetState_CornerEnabled(TRectCorner, bool)

IPCB_CustomShapeDonutParameters
  ├── GetSatete_Width() → int       (note: typo "Satete" is in original)
  ├── GetSatete_OuterDiameter() → int
  └── SetSatete_Width/OuterDiameter()

IPCB_CustomShapeStorage
  ├── GetState_CustomShape(index) → nint
  ├── GetState_CustomShapeCount() → int
  └── CollectCustomShapes()

IPCB_CustomShapeSupports
  ├── GetState_PadOwner() → IPCB_Primitive
  ├── SetState_PadOwner(pad)
  ├── ApplyPadTransformation(applyExtendData) → IPCB_Primitive
  └── RevertPadTransformation()
```

Key source files:
- `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_CustomShapeInfo.cs`
- `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_CustomPadShape.cs`
- `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_CustomShapeStorage.cs`
- `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_CustomShapeRectParameters.cs`
- `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_CustomShapeDonutParameters.cs`
- `AD26-dotnet/Altium.Edp.Interfaces/PCBInterfaces/IPCB_CustomShapeSupports.cs`
- `AD26-dotnet/Altium.Edp.Interfaces/PCBInterfaces/TEnumCustomShapes.cs`

## Delphi Implementation (Ghidra Findings)

### BinaryLoader.dll — TCustomShapeSection

The `TCustomShapeSection` class in `Altium.PCB.BinaryLoader.dll` handles reading/writing the CustomShapes stream.

**RTTI location:** `0x019fdcb0` (class name at `0x019fdcb6`)

**Methods (from RTTI method table):**

| Method | Address | Description |
|--------|---------|-------------|
| `Create` | `0x01a04230` | Constructor — calls `TSection.Create` base at `0x0189fcf0`, initializes internal collections |
| `Destroy` | `0x01a04420` | Destructor |
| `DataWrite` | `0x01a04570` | Serializes custom shape entries to stream |
| `DataRead` | `0x01a04660` | Reads entries from stream (loops over count, calling per-entry reader) |
| `PrepareToSaveInLibrary` | `0x01a04730` | Pre-save hook for library context |
| `Apply` | `0x01a048e0` | Applies custom shapes to board primitives |
| `ApplyInLibrary` | `0x01a04ac0` | Applies custom shapes to library component primitives |
| `CollectExtraPrimitives` | `0x01a07780` | Collects primitives that are part of custom shapes |

**Per-entry read function:** `0x01a044b0` — Creates a `TCustomShapeInfo` object (class at `0x016c2988`), reads parameter block from stream buffer, and deserializes into the object.

**Per-entry write function:** `0x01a04370` — Serializes one `TCustomShapeInfo` to the stream.

### Advpcb.dll — TCustomShapeInfoImplementation

The runtime implementation of custom shape info lives in `Advpcb.dll`.

**RTTI location:** `0x05332702`

**Key string addresses (parameter names):**

| Address | String | Usage |
|---------|--------|-------|
| `0x00e2709c` | `CustomShape_RectanglesCorners` | Rectangle corner enable flags |
| `0x00e270e9` | `CustomShape_CornerRadiusAbsolute` | Absolute corner radius value |
| `0x00e27139` | `CustomShape_Donut` | Donut shape parameters |
| `0x00e27057` | `CustomShapeSnapPoints` | Snap point polygon vertices |

**Published properties (from RTTI at `0x0136d7ba`):**
- `ShapeKind` (at offset `0x0136d84b`)
- `CustomShapeBaseParameters` (at offset `0x0136d87a`)

### TCustomShapeInfo (BinaryLoader.dll)

**RTTI location:** `0x016c2988`

**Instance size:** `0x28` (40 bytes)

**Key fields:**
- Offset `+0x08`: `i32` pad index (initialized to `-1` / `0xFFFFFFFF` in constructor)
- Offset `+0x10`: Pointer to internal shape info list

**Constructor:** `0x016ca380` — allocates 40-byte object, sets pad index to -1, creates inner list at `0x016cc760`

**Note:** Much of the code in the `0x016ca000`–`0x016cb000` range is not properly analyzed by Ghidra (functions show as 1-byte stubs). Manual function creation at these addresses fails with "Function body must contain the entrypoint" — the addresses may need disassembly first. This is a known issue with Delphi binaries in Ghidra.

## Jumping-Off Points for Future Reverse Engineering

### High Priority

1. **Donut parameters format**: No test files with `SHAPEKIND=5` (donut) were found. Create a PcbLib with a donut pad in Altium Designer and dump the CustomShapes stream to discover the donut-specific parameters (likely `S{N}.DONUT.WIDTH`, `S{N}.DONUT.OUTERDIAMETER` or similar, corresponding to `IPCB_CustomShapeDonutParameters`).

2. **Absolute corner radius**: The `CustomShape_CornerRadiusAbsolute` string suggests pads can specify absolute radius instead of percentage. Create test footprints with explicit corner radius values to discover this parameter format.

3. **Multi-layer custom shapes**: All observed entries use `S0` only. Create a pad with `ePadMode_LocalStack` (per-layer mode) and different custom shapes per layer to discover if `S1`, `S2`, etc. entries appear, or if separate entries per PRIMITIVEINDEX+layer are used.

4. **CustomReliefs stream format**: No test data found. Create footprints with custom thermal relief patterns to dump and decode the `/CustomReliefs` stream.

### Medium Priority

5. **Snap point data**: The `CustomShapeSnapPoints` string in Advpcb.dll suggests polygon vertex data may be stored. Investigate whether this appears as additional parameters in the CustomShapes stream or as a separate mechanism.

6. **`TCustomShapeSection.CollectExtraPrimitives`** at `0x01a07780`: This function collects "extra primitives" associated with custom shapes — likely region primitives that define the actual outline. Decompile to understand how primitives are linked.

7. **Ghidra analysis gap**: The entire `0x016ca000`–`0x016cb000` range in `Altium.PCB.BinaryLoader.dll` needs manual disassembly. Run Ghidra's "Disassemble" on the following addresses to create proper functions:
   - `0x016ca4f0` (starts with `55 53 4883ec68` = `push rbp; push rbx; sub rsp, 0x68`)
   - Functions called from `FUN_01a044b0` and `FUN_01a04370`

### Low Priority

8. **`CustomShapeCompatibilityMode`** strings at `0x015b0e7d` and `0x015b5abf` in BinaryLoader.dll — may indicate backwards-compatibility behavior for older format versions.

9. **`TEnumCustomShapes`** enum (`eAllShapes=0, eShapeInformation=1`) used by `IPCB_CustomShapeStorage` — investigate how this enum controls which shape data is collected/enumerated.

10. **PcbDoc vs PcbLib**: The CustomShapes stream format appears identical between PcbDoc and PcbLib (both use parameter-block format), but this should be verified by examining PcbDoc files.

## Test Files

| File | Footprint | Stream | Size | Content |
|------|-----------|--------|------|---------|
| `FragasLab-Footprint.PcbLib` | WSON-10B | CustomShapes | 660 bytes | 4 rounded-rect pads with selective corners |
| `SMotlaq-PCB_lib.PcbLib` | QFN40-68 | CustomShapes | 172 bytes | 1 rounded-rect pad |
| `SMotlaq-PCB_lib.PcbLib` | WSON-6 | CustomMaskShapes | 90 bytes | 1 custom paste mask |
