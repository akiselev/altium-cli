> **AD26 source snapshot; not current implementation status.** Validate against current Rust code before use.

# Gerber Export in Altium Designer

Reverse-engineered findings on how Altium Designer generates Gerber (RS-274X / Gerber X2)
and NC Drill (Excellon) files, and what we need to replicate this in altium-cli.

## Architecture Overview

Altium's Gerber export is split across **three layers**:

1. **C# (.NET)** -- configuration interfaces, type definitions, orchestration, file validation
2. **Advpcb.dll (Delphi)** -- PCB data model, object iteration, painter dispatch, polygon pre-computation
3. **OUT_Gerber.dll (Delphi)** -- the actual Gerber file writer (`TGerberFile` class)

The C# layer defines the full configuration surface and wires it into the OutputJob system.
Advpcb.dll manages the PCB object model and iterates primitives per layer.
OUT_Gerber.dll is a plugin DLL that implements the `TGerberFile` class containing all
actual Gerber format generation: header writing, aperture definitions, coordinate formatting,
drawing commands, region fills, and polarity management.

### Data Flow

```
OutputJob Document (.OutJob)
    |
    v
IOutputGenerator (registered generator, type = eProtelGerber)
    |
    v
OUT_Gerber.dll: GetOutputGenerator() --> TGerberFile created
    |
    v
Advpcb.dll entry points:
    PcbApi_QueryBoardGerberOptions    (0x03d40330)  -- read settings from board
    PcbApi_GetLayerPolygonShapesForOutput (0x03d5a1c0)  -- precompute polygon fills
    PcbApi_CreatePainter              (0x03d58940)  -- create renderer (COM factory)
    |
    v
For each enabled layer:
    For each PCB primitive on that layer:
        PcbApi_Export_ToPainter       (0x03d589d0)  -- push primitive through TGerberFile
            --> TGerberFile methods write: aperture selection, coordinates, D-codes
    |
    v
Per-layer Gerber files (.GTL/.GBL/.G1/.GTS/etc.) + drill files (.DRL) + reports (.REP)
```

The output driver type is `TOutputDriverType::eProtelGerber` (byte value 1).

---

## C# Type Definitions

All in namespace `RT_GerberOutputs` under `Altium.Edp.Interfaces`.

### Enumerations

#### TGerberVersion
```csharp
enum TGerberVersion {
    eX2,    // Gerber X2 (with TF/TO/TD attributes)
    e724X,  // RS-274X (classic extended Gerber)
}
```

#### TGerberFileKind
Maps each output file to a fabrication purpose:
```csharp
enum TGerberFileKind {
    gfkUndefined,       // 0
    gfkCopper,          // 1  -- signal/plane copper layers
    gfkSoldermask,      // 2
    gfkKeepOut,         // 3
    gfkPasteMask,       // 4
    gfkMechanical,      // 5
    gfkDrillDrawing,    // 6
    gfkDrillGuide,      // 7
    gfkProfile,         // 8  -- board outline
    gfkNPTH,            // 9  -- non-plated through holes
    gfkPTH,             // 10 -- plated through holes
    gfkBlindViaHoles,   // 11
    gfkOverlay,         // 12 -- silkscreen
    gfkPadMaster,       // 13
    gfkBackdrills,      // 14
    gfkMicroVias,       // 15
}
```

#### TApertureShape
```csharp
enum TApertureShape {
    eApertureRound,             // 0
    eApertureRectangle,         // 1
    eApertureOctagon,           // 2
    eApertureRoundedRectangle,  // 3
    eApertureRoundRelief,       // 4  -- thermal relief (round)
    eApertureOvalRelief,        // 5  -- thermal relief (oval)
    eApertureSquareRelief,      // 6  -- thermal relief (square)
    eApertureRectRelief,        // 7  -- thermal relief (rect)
    eApertureCustomShape,       // 8  -- AM macro
}
```

#### TApertureUsage
```csharp
enum TApertureUsage {
    eApertureFlashOnly,     // 0 -- D03 flash only
    eApertureStrokeOnly,    // 1 -- D01 draw only
    eApertureFlashOrStroke, // 2 -- either
}
```

#### TZeroesMode
```csharp
enum TZeroesMode {
    eKeepLeadingAndTrailingZeroes,  // 0
    eSuppressLeadingZeroes,         // 1
    eSuppressTrailingZeroes,        // 2
}
```

#### TOriginPosition
```csharp
enum TOriginPosition {
    eAbsolute,  // 0 -- board origin (0,0)
    eRelative,  // 1 -- user-defined reference point
    eCenter,    // 2 -- board center
}
```

#### TOutputFormat
```csharp
enum TOutputFormat {
    eSingle,    // 0 -- all layers in one file
    eDifferent, // 1 -- separate file per layer
}
```

#### TFileSubject
Used for Gerber X2 `%TF.Part` attribute:
```csharp
enum TFileSubject {
    fsNone,             // 0 -- "None"
    fsAutoDetect,       // 1 -- "Autodetect"
    fsSinglePCB,        // 2 -- "Single"
    fsArray,            // 3 -- "CustomerPanel"
    fsProductionPanel,  // 4 -- "ProductionPanel"
    fsCoupon,           // 5 -- "Coupon"
    fsOther,            // 6 -- "Other"
}
```

#### TLayerGroupType
Controls mechanical layer grouping for combined output:
```csharp
enum TLayerGroupType {
    lgtNone,              // 0
    lgtCopperLayers,      // 1
    lgtSilkscreen,        // 2
    lgtSolderMask,        // 3
    lgtPasteMask,         // 4
    lgtMechanicalLayers,  // 5
    lgtDrills,            // 6
    lgtOtherLayers,       // 7
    lgtCustomLayers,      // 8
    lgtViaStructures,     // 9
}
```

### D-Code Constants

```csharp
const int kMinDCode = 10;
const int kMaxDCode = 9999;
const int kMaxDCodeGerberSpecification = 999;  // per RS-274X spec
```

### IApertureInfo Interface

Each aperture (Gerber tool) is represented as:

```csharp
interface IApertureInfo {
    int   GetState_DCode();        // D-code number (10..9999)
    TApertureShape GetState_Shape();
    int   GetState_XSize();        // width  (internal coords)
    int   GetState_YSize();        // height (internal coords)
    int   GetState_HoleSize();     // drill hole (internal coords)
    int   GetState_XOffSet();      // X offset from center
    int   GetState_YOffSet();      // Y offset from center
    double GetState_Angle();       // rotation angle (degrees)
    TExtendedHoleType GetState_HoleType();
    TApertureUsage GetState_Usage();
}
```

### IGerberSettings Interface

The master configuration surface (236 methods). Key groups:

**Format:**
- `GetGerberVersion()` / `SetGerberVersion()` -- X2 or RS-274X
- `GetUnits()` / `SetUnits()` -- mm or inch
- `GetNumberOfDecimals()` / `SetNumberOfDecimals()` -- coordinate precision
- `GetLeadingAndTrailingZeroesMode()` / `SetLeadingAndTrailingZeroesMode()`
- `GetOutputFormat()` / `SetOutputFormat()` -- single file or per-layer

**Film:**
- `GetFilmXSize()` / `GetFilmYSize()` / `GetFilmBorderSize()`
- `GetOriginPosition()` / `SetOriginPosition()` -- absolute/relative/center

**Apertures:**
- `GetApertureInfo(index)` / `SetApertureInfo(index, value)`
- `GetApertureInfoCount()`
- `SetPopulateApertureInfoFromPCB()` -- auto-generate aperture table from board
- `SetSaveApertureInfo()` / `SetLoadApertureInfo()` -- aperture library I/O
- `GetApertureTolerancePlus()` / `GetApertureToleranceMinus()`
- `GetMaximumApertureSize()` / `SetMaximumApertureSize()`
- `GetEmbeddedApertures()` -- embed %AD definitions in file (vs external .apt)

**Rendering behavior:**
- `GetG54OnApertureChange()` -- insert G54 command on tool change
- `GetSoftwareArcs()` -- linearize arcs (true) vs hardware G02/G03 (false)
- `GetFlashPadShapes()` -- use D03 flash for pads
- `GetFlashAndFills()` -- use D03 flash for fills
- `GetGenerateReliefShapes()` -- output thermal relief geometries
- `GetUsePolygonFormOctagonalParts()` -- polygon decomposition for octagons
- `GetMergeRegionAndPadsInFootprint()` -- boolean union overlapping copper
- `GetOptimizeChangelocationCommands()` -- remove redundant move commands
- `GetSorted()` -- optimize draw order to minimize head travel
- `GetPlotPositivePlaneLayers()` -- positive polarity for plane layers

**Layers:**
- `GetLayerPlot(layerID)` / `SetLayerPlot(layerID, bool)` -- per-layer enable
- `GetLayerMirror(layerID)` / `SetLayerMirror(layerID, bool)` -- per-layer mirror
- `GetLayerDisplayName(layerID)` / `SetLayerDisplayName(layerID, name)`
- `GetAddMechanicalLayerToAll(layerID)` -- add mech layer to every output file
- `GetMechanicalLayersToGroup(groupType)` -- assign mech layers to groups
- `GetClassPlot(className)` / `GetClassMirror(className)` -- per-class filtering

**Drill files:**
- `GetPlotDrill(kind, layer1, layer2, pairType)` -- enable drill output per layer pair
- `GetPlotDrillMirror(kind)` -- mirror drill output
- `GetPlotDrillFileName(kind, layer1, layer2, pairType, counterHoleParams)`
- `GetPlotDrillPair(kind, drillLayerPair)` -- enable by drill pair object
- `GetPlotBoardProfile()` / `GetPlotBoardProfileFileName()` -- board outline file

**Misc:**
- `GetPanelize()` -- panelization support
- `GetIncludeUnconnectedMidLayerPads()` -- include unconnected mid-layer pads
- `GetFileSubject()` / `GetFileComment()` -- X2 metadata
- `GetGenerateGRCRules()` -- generate Gerber rule check output
- `GetGenerateReports()` -- generate .REP report files
- `GetBoard()` -- reference to source PCB board
- `PrepareLayerNames()` / `GetLayerNamesCount()` / `GetLayerNamesLayerForItem()`

### IPCB_GerberOptions Interface

Lower-level options stored in the PCB board record itself (in `RT_PCB` namespace):

```csharp
interface IPCB_GerberOptions : IPCB_AbstractOptions {
    bool   GetState_SortOutput();
    bool   GetState_UseSoftwareArcs();
    bool   GetState_CenterPhotoPlots();
    bool   GetState_EmbedApertures();
    bool   GetState_Panelize();
    bool   GetState_G54();               // G54 on aperture change
    int    GetState_PlusTol();
    int    GetState_MinusTol();
    int    GetState_FilmSizeX();
    int    GetState_FilmSizeY();
    int    GetState_BorderSize();
    string GetState_AptTable();           // aperture table file path
    int    GetState_MaxAperSize();
    bool   GetState_ReliefShapesAllowed();
    bool   GetState_PadsFlashOnly();
    int    GetState_GerberUnits();        // 0=inch, 1=mm (likely)
    int    GetState_GerberDecs();         // number of decimal places
    bool   GetState_FlashAllFills();

    // Serialization to/from parameter strings:
    void Import_FromParameters(TUnit displayUnit, StringBuilder parameters);
    void Export_ToParameters(StringBuilder parameters);
    void Import_FromParameters_Version3(...);
    void Import_FromParameters_Version4(...);
}
```

These options are serialized into the PCB file's board record and loaded via
`PcbApi_QueryBoardGerberOptions`.

---

## Delphi Layer -- Advpcb.dll (PCB Data Model & Painter Dispatch)

### Entry Points in Advpcb.dll

From `pcb-api-functions.md` and ghidra reverse-engineering:

| Address | Function | Purpose |
|---------|----------|---------|
| 0x03d40330 | `PcbApi_QueryBoardGerberOptions` | Read/write Gerber settings from board |
| 0x03d3ee10 | `PcbApi_QueryBoardOutputOptions` | Read general output options |
| 0x03d3fee0 | `PcbApi_QueryBoardOutputOptionsPlotLayers` | Get plot layer config |
| 0x03d3ffe0 | `PcbApi_QueryBoardOutputOptionsFlipLayers` | Get flip layer config |
| 0x03d40860 | `PcbApi_QueryBoardPrinterOptions` | Read printer options |
| 0x03d58940 | `PcbApi_CreatePainter` | Factory: create output renderer via COM |
| 0x03d589d0 | `PcbApi_Export_ToPainter` | Export PCB data through painter |
| 0x03d58af0 | `PcbApi_Export_ToPainter_ByHandle` | Export by handle through painter |
| 0x03d5a1c0 | `PcbApi_GetLayerPolygonShapesForOutput` | Pre-computed polygon shapes |
| 0x03d5a210 | `PcbApi_GetLayerPolygonShapesForOutputEx` | Extended polygon shapes |

### Drill-Related Entry Points

| Address | Function | Purpose |
|---------|----------|---------|
| 0x03d3ebd0 | `PcbApi_QueryBoardLayerPairBackDrill` | Back drill info for layer pair |
| 0x03d3ec50 | `PcbApi_GetDrillTableLayerPairFromItsObject` | Drill table lookup |
| 0x03d27a80 | `PcbApi_QueryViaBackDrill` | Via back drill info |
| 0x03d41580 | `PcbApi_QueryBoardGetDrillSymbolsConfiguration` | Drill symbol config |
| 0x03d41b90 | `PcbApi_QueryBoardDrillSymbolIndex` | Drill symbol lookup |

### Decompiled: PcbApi_CreatePainter (0x03d58940)

```c
undefined8 PcbApi_CreatePainter(undefined8 param_1) {
    longlong *plStack_10;
    plStack_10 = (longlong *)0x0;
    FUN_012024b0(&plStack_10);               // Load global COM interface from 0x06376780
    FUN_0041c540(param_1);                   // Variant init
    uVar1 = (**(code **)(*plStack_10 + 0x118))(plStack_10, param_1);  // vtable +0x118
    FUN_0041c980(uVar1);                     // OleCheck
    FUN_0041c540(&plStack_10);               // Release
    return param_1;
}
```

The factory `FUN_012024b0` loads a COM interface reference from a global at `0x06376780`
(BSS segment). It calls vtable offset `+0x118` (method index ~35) to create the painter
object. The painter is then used by `PcbApi_Export_ToPainter`.

### Decompiled: PcbApi_Export_ToPainter (0x03d589d0)

```c
void PcbApi_Export_ToPainter(param_1, param_2, param_3) {
    FUN_0041c650(param_1);                   // AddRef on painter
    FUN_0041c5d0(aplStack_20, param_1, &UNK_03d58ad8);  // QueryInterface (GUID at 0x03d58ad8)
    FUN_00454390(&uStack_28, param_2);       // Wrap param_2 as OLE variant
    FUN_00454390(&uStack_30, param_3);       // Wrap param_3 as OLE variant
    (**(code **)(*aplStack_20[0] + 0x18))(aplStack_20[0], uStack_28, uStack_30);  // vtable +0x18
    FUN_004152f0(&uStack_30, 2);             // Release variants
    FUN_0041c540(aplStack_20);               // Release interface
    FUN_0041c540(param_1);                   // Release painter
}
```

QueryInterfaces the painter for a specific GUID, wraps two parameters (PCB primitive
handle and rendering context) as OLE variants, then dispatches through the painter's
vtable at offset `+0x18` (method index 3). The painter's virtual method renders the
PCB object into Gerber format.

### Decompiled: PcbApi_GetLayerPolygonShapesForOutput (0x03d5a1c0)

```c
undefined8 PcbApi_GetLayerPolygonShapesForOutput(longlong param_1, undefined4 param_2) {
    if (param_1 != 0) {
        return FUN_03d1eb80(param_1, param_2, param_2, 0, 0, 0, 0, 0);
    }
    return 0;
}
```

Thin wrapper delegating to `FUN_03d1eb80`, which:
1. Allocates a tracking object from class at `_UNK_03d1e8b0`
2. Stores the layer ID at offset `+0x18`
3. Creates two list/collection instances (class at `_UNK_0131ac90`)
4. Creates a computation worker via `FUN_03d0bf50` (class at `_UNK_03d0b6d0`)
5. Runs polygon computation via `FUN_03d0f100`
6. Returns pre-computed polygon shapes with clearances and thermal reliefs resolved

This pre-computation is critical for Gerber output -- polygon pours must have their
clearances, thermal reliefs, and dead copper removal resolved into final vertex lists
before they can be written as G36/G37 regions.

### Decompiled: PcbApi_QueryBoardGerberOptions (0x03d40330)

28-parameter bidirectional getter/setter. The internal Gerber options struct (obtained
via `FUN_0470ffe0` from the board object) has the following field layout:

| Offset | Type | Field |
|--------|------|-------|
| +0x28 | string | AptTable (aperture table file path) |
| +0x30 | string | Field 2 (unknown) |
| +0x38 | string | Field 3 (unknown) |
| +0x41 | bool | SortOutput |
| +0x42 | bool | UseSoftwareArcs |
| +0x43 | bool | CenterPhotoPlots |
| +0x44 | bool | EmbedApertures |
| +0x45 | bool | Panelize |
| +0x46 | bool | G54OnApertureChange |
| +0x48 | i32 | PlusTolerance |
| +0x50 | i64 | FilmSizeX |
| +0x58 | i64 | FilmSizeY |
| +0x60 | i64 | BorderSize |
| +0x68 | 27 bytes | MaxAperSize + related data |
| +0x83 | bool | ReliefShapesAllowed |
| +0x84 | bool | PadsFlashOnly |
| +0x88 | i64 | GerberUnits (0=imperial, 1=metric) |
| +0x90 | i64 | GerberDecimals |
| +0x98 | i64 | Field 21 (unknown) |
| +0xA0 | i32 | Field 22 (unknown) |
| +0xA4 | i32 | Field 23 (unknown) |
| +0xA8 | bool | FlashAllFills |
| +0xA9 | bool | Field 25 (unknown) |
| +0xAA | bool | Field 26 (unknown) |
| +0xAB | bool | Field 27 (unknown) |
| +0xAC | bool | Field 28 (read-only) |

Direction flag: `param_1 == 0` reads from board, `param_1 == 1` writes to board,
`param_1 == 2` treated as read. Validates board object type is `0x19` via `FUN_0469e1e0`.

---

## Delphi Layer -- OUT_Gerber.dll (TGerberFile: The Actual Gerber Writer)

### Plugin Architecture

OUT_Gerber.dll is a Delphi plugin DLL that exports a single factory function:

```c
GetOutputGenerator(undefined8 *param_1) {
    *param_1 = 0;
    lVar1 = FUN_00e52df0(
        _UNK_00e4d468,
        1,
        FUN_0156cee0,    // Settings/configuration callback
        0,
        FUN_0157c420,    // Setup callback
        FUN_01562170,    // Generate callback (main entry point)
        0,
        FUN_01594b70,    // Layer output callback
        0,
        &UNK_01594c94
    );
    // Returns interface pointer at object + 0xbd50
}
```

The main class is `TGerberFile`, identified by RTTI strings in the binary (e.g.,
`TGerberFile.ProcessLayerPolygons$90$0$IntfH/@`). Its vtable is at approximately
`0x01a04580`.

### TGerberFile Method Table

Key methods identified by string references and decompilation:

| Address | Purpose | Output |
|---------|---------|--------|
| 0x014e8610 | Write format specification | `%FSLAX36*%` |
| 0x014e8860 | Write unit mode | `%MOMM*%` or `%MOIN*%` |
| 0x013ff560 | Write circular aperture | `%ADD{d}C,{size}*%` |
| 0x013ffc00 | Write oblong aperture | `%ADD{d}O,{x}X{y}*%` |
| 0x013f77a0 | Coordinate formatter | Internal units --> formatted string |
| 0x014e4740 | Write X/Y coordinates | Optimized (only emits if changed) |
| 0x014e4ef0 | Write line draw | `X...Y...D01*` / `D02*` |
| 0x014e4fa0 | Write arc | `G02/G03 X...Y...I...J...D01*` |
| 0x014e4db0 | Write flash | `X...Y...D03*` |
| 0x014eba30 | Write region fill | `G36*` ... vertices ... `G37*` |
| 0x014ebed0 | Write layer polarity | `%LPD*%` / `%LPC*%` (stateful) |
| 0x014e2364 | Write X2 attributes | `TF.GenerationSoftware,...` |
| 0x014e46a0 | Write end of file | `M02*` |

### Gerber Format Strings (from OUT_Gerber.dll string table)

All Gerber output strings extracted from the binary, organized by function:

#### Header Commands
| Address | String | Description |
|---------|--------|-------------|
| 0x014e8820 | `%FS` | Format Specification prefix |
| 0x014e8800 | `L` | Leading zeroes suppression mode indicator |
| 0x014e8810 | `T` | Trailing zeroes suppression mode indicator |
| 0x014e8834 | `AX` | Absolute coordinates, X axis |
| 0x014e8848 | `Y` | Y axis format |
| 0x014e8858 | `*%` | Command block terminator |
| 0x014e88ee | `%MOMM*%` | Mode: Metric (millimeters) |
| 0x014e8908 | `G71*` | Legacy metric mode (deprecated) |
| 0x014e8922 | `%MOIN*%` | Mode: Imperial (inches) |
| 0x014e893c | `G70*` | Legacy imperial mode (deprecated) |

#### Aperture Definitions
| Address | String | Description |
|---------|--------|-------------|
| 0x013ff506 | `%%ADD%d` | Aperture Definition with D-code number |
| 0x013ff520 | `C,%s` | Circle shape with diameter |
| 0x013ff538 | `X%s` | X-size dimension separator |
| 0x013ff548 | `*%` | AD terminator |
| 0x013ff750 | `%%ADD%d` | (duplicate for oblong function) |
| 0x013ff76c | `O,%sX%s` | Oblong/Oval shape format |
| 0x013ffbd8 | `%%ADD%d%s*%%` | Full aperture definition format string |
| 0x013ffb50 | `CIRCLED%d` | Circle aperture macro name |
| 0x013ffb70 | `%%AM%s*` | Aperture Macro definition header |
| 0x013ffba0 | `1,1,%s,%s,%s*` | Circle primitive in aperture macro body |

#### Drawing Commands
| Address | String | Description |
|---------|--------|-------------|
| 0x014e85c8 | `G01*` | Linear interpolation mode |
| 0x014e85e0 | `G75*` | Multi-quadrant mode |
| 0x014e50c8 | `G02*` | Clockwise circular interpolation |
| 0x014e50e0 | `G03*` | Counter-clockwise circular interpolation |
| 0x014ebe5c | `D01*` | Draw (pen down, interpolate) |
| 0x014ebe44 | `D02*` | Move (pen up, no draw) |
| 0x014e4da4 | `D03*` | Flash (expose current aperture at position) |

#### Coordinate Prefixes
| Address | String | Description |
|---------|--------|-------------|
| 0x014e49a4 | `X` | X coordinate prefix |
| 0x014e49b4 | `Y` | Y coordinate prefix |
| 0x014e4bb4 | `I` | Arc center I offset prefix |
| 0x014e4bc4 | `J` | Arc center J offset prefix |

#### Region Fill
| Address | String | Description |
|---------|--------|-------------|
| 0x014ebe18 | `G36*` | Begin region fill |
| 0x014ebe74 | `G37*` | End region fill |

#### Polarity
| Address | String | Description |
|---------|--------|-------------|
| 0x014ebf38 | `%LPC*%` | Layer Polarity Clear (negative) |
| 0x014ebf50 | `%LPD*%` | Layer Polarity Dark (positive) |

#### End of File
| Address | String | Description |
|---------|--------|-------------|
| 0x014e46a0 | `M02*` | End of file |

#### Gerber X2 Attributes
| Address | String | Description |
|---------|--------|-------------|
| 0x014e2364 | `TF.GenerationSoftware,%s,%s,%s (%s)` | Generation software attribute |
| 0x014e23b8 | `Altium Limited` | Company name value |
| 0x014ec1b8 | `TF.SameCoordinates,%s` | Same coordinates attribute |
| 0x014ec3f0 | `TF.FilePolarity,%s` | File polarity attribute |
| 0x014e4ed0 | `G04 #@! %s*` | X2 attribute as structured comment |
| 0x014e7a60 | `G04 Layer_Physical_Order=%d*` | Layer ordering comment |
| 0x01409e8e | `%TA.AperFunction,` | Set aperture function attribute |
| 0x01409ed2 | `%TD.AperFunction*%` | Delete aperture function attribute |

### Decompiled Algorithm: Coordinate Formatting (FUN_013f77a0)

The coordinate formatter at `OUT_Gerber.dll:0x013f77a0` converts internal PCB units to
Gerber coordinate strings:

```
1. Unit conversion:
   - Imperial: call FUN_00e4af20 (divide by internal-to-inch constant)
   - Metric:   call FUN_013bff40 (divide by constant at 0x013f7930, converting to mm)

2. Scale to integer:
   value = round(physical_value * 10^decimals)
   Rounding via FUN_0040c870

3. Zero suppression (controlled by TZeroesMode):
   - Mode 0 (KeepAll):              pad to full width with leading zeros
   - Mode 1 (SuppressLeading):      strip leading '0' characters
   - Mode 2 (SuppressTrailing):     strip trailing '0' characters

4. Sign handling:
   - Negative coordinates get a '-' prefix

5. Bit-flag check at param_6 < 8 controls leading zero behavior
```

### Decompiled Algorithm: Coordinate Writing (FUN_014e4740)

The TGerberFile object tracks the last-written coordinates for optimization:

```
State offsets in TGerberFile:
  +0x84  origin offset X (subtracted from all X coordinates)
  +0x88  origin offset Y (subtracted from all Y coordinates)
  +0x8c  last written X value
  +0x90  last written Y value

Algorithm:
  1. Apply origin offset:  x = raw_x - origin_x
                           y = raw_y - origin_y
  2. Format x via FUN_013f77a0
  3. Format y via FUN_013f77a0
  4. If x != last_x: emit "X{formatted_x}",  update last_x
  5. If y != last_y: emit "Y{formatted_y}",  update last_y
  (if neither changed, nothing is emitted for coordinates)
```

This optimization means consecutive commands at the same X or Y skip that axis entirely.

### Decompiled Algorithm: Arc Writing (FUN_014e4fa0)

```
1. Move to start position:
   X{start_x}Y{start_y}D02*

2. Set interpolation mode:
   G02*    (clockwise)
   G03*    (counter-clockwise)

3. Write endpoint and center offset:
   X{end_x}Y{end_y}I{center_x - start_x}J{center_y - start_y}D01*

4. Return to linear mode:
   G01*
```

Note: I/J offsets are computed as `center - start_point`, not absolute coordinates.

### Decompiled Algorithm: Region Fill (FUN_014eba30)

```
1. Set polarity via FUN_014ebed0:
   %LPD*%   (dark/positive)  or  %LPC*%   (clear/negative)

2. Begin region:
   G36*

3. Move to first vertex:
   X{v0_x}Y{v0_y}D02*

4. Draw to each subsequent vertex:
   X{v1_x}Y{v1_y}D01*
   X{v2_x}Y{v2_y}D01*
   ...

5. Close contour (draw back to first vertex):
   X{v0_x}Y{v0_y}D01*

6. End region:
   G37*
```

### Decompiled Algorithm: Layer Polarity Toggle (FUN_014ebed0)

```
State offset in TGerberFile:
  +0x20  current polarity state (bool)

Algorithm:
  1. Check if requested polarity differs from stored state at +0x20
  2. If same: do nothing (skip write)
  3. If different:
     - Write %LPC*%  (if switching to clear/negative)
     - Write %LPD*%  (if switching to dark/positive)
     - Update stored state at +0x20
```

This is a key optimization: polarity commands are only emitted when the polarity
actually changes, not on every object.

### Unit Scale (FUN_013f3690)

Returns `4` for imperial (internal units to mils), `2` for metric (internal units to mm).
This determines the scale factor used in coordinate conversion.

---

## RS-274X File Format Details

Extracted from `CAMtasticFileChecker.cs` (Gerber file validation code).

### File Recognition Tokens

**Required for RS-274X identification:**
- Must contain: `FS` (format specification) AND `%ADD` (aperture definition)
- Must contain at least one mass parameter
- Must contain D-codes, G-codes, and M-codes

**Mass parameters (RS-274X extended commands):**
```
FSA     -- format specification (absolute)
FSLA    -- format specification (leading zero suppressed, absolute)
%MOIN*% -- unit = inch
%MOMM*% -- unit = millimeter
%AM     -- aperture macro definition
%LN     -- layer name
%LPD    -- layer polarity dark
%LPD*%  -- (variant)
%LPC    -- layer polarity clear
%LPC*%  -- (variant)
%IN     -- image name
%IPPOS*% -- image polarity positive
%IPNEG*% -- image polarity negative
```

**G-codes used by Altium:**
```
G00  -- rapid move (legacy)
G01  -- linear interpolation
G02  -- clockwise circular interpolation
G03  -- counterclockwise circular interpolation
G04  -- comment
G10  -- linear interpolation (10x scale, legacy)
G11  -- linear interpolation (0.1x scale, legacy)
G12  -- linear interpolation (0.01x scale, legacy)
G36  -- begin region (area fill)
G37  -- end region
G54  -- tool (aperture) change
G70  -- set units to inches (deprecated, use %MO)
G71  -- set units to mm (deprecated, use %MO)
G72  -- set units to inches (variant)
G74  -- single quadrant mode (arcs < 90 degrees)
G75  -- multi quadrant mode (arcs any size)
G90  -- absolute coordinate mode
G91  -- incremental coordinate mode
```

**D-codes:**
```
D01 / D1  -- draw (interpolate to coordinate, pen down)
D02 / D2  -- move (go to coordinate, pen up)
D03 / D3  -- flash (stamp aperture at coordinate)
D10+      -- select aperture (tool change)
```

**M-codes:**
```
M00*  -- program stop
M01*  -- optional stop
M02*  -- end of file
```

### Recognized File Extensions

```
.cam  .gbr  .gtl  .g    .gg   .ggg  .gbl  .gto  .gbo
.gtp  .gbp  .gts  .gbs  .gp   .gdg  .gko  .gm   .gd
.gdd  .gpt  .gpb  .apr  .apt  .apr_lib  .drl
```

Extension matching allows numeric suffixes (regex: `^.{ext}[\d]*$`), so `.g1`, `.g2`,
`.gm1`, `.gm13` etc. are all valid.

### Standard Altium File Naming Convention

| Extension | Layer |
|-----------|-------|
| `.GTL` | Top Copper |
| `.GBL` | Bottom Copper |
| `.G1`..`.G30` | Inner Copper 1-30 |
| `.GTO` | Top Overlay (Silkscreen) |
| `.GBO` | Bottom Overlay |
| `.GTS` | Top Solder Mask |
| `.GBS` | Bottom Solder Mask |
| `.GTP` | Top Paste Mask |
| `.GBP` | Bottom Paste Mask |
| `.GPT` | Top Pad Master |
| `.GPB` | Bottom Pad Master |
| `.GKO` | Keep-Out |
| `.GM1`..`.GM16` | Mechanical 1-16 |
| `.GD1` | Drill Drawing |
| `.GG1` | Drill Guide |
| `.DRL` | NC Drill (Excellon) |

---

## NC Drill (Excellon) Format

From `CAMtasticFileChecker.cs`:

### File Recognition

**Required header tokens (all must be present):**
```
M48                -- header start
;Layer_Color=      -- Altium layer color comment
;FILE_FORMAT=      -- format specification comment
```

**Header commands (at least one must be present):**
```
DETECT,ON    DETECT,OFF
FMAT         METRIC       METRIC,LZ    METRIC,TZ
INCH         INCH,LZ      INCH,TZ      M95
```

**Body commands (at least one must be present):**
```
G00X  G00   G05   G01   G02   G41   G42   G85
G90   G91   M15   M16   M17   M71   M72   T00
T01   M30
```

### Drill Report (.DRR) Recognition

Files with `.DRR` extension containing:
```
NCDrill File Report For
Layer Pair :
Total Processing Time (hh:mm:ss) :
```

---

## Coordinate Conversion

Altium internal coordinates use a fixed-point system:

```
1 mil     = 10,000 internal units
1 inch    = 10,000,000 internal units
1 mm      = 393,701 internal units (exact: 10,000,000 / 25.4)
1 unit    = 0.0001 mil = 0.00000254 mm = 2.54 nm
```

### For Gerber Output

**Metric format (e.g., %FSLAX36*% -- 3 integer, 6 decimal places):**
```
gerber_value = internal_units * 0.00000254 * 1,000,000
             = internal_units * 0.00254

Example: 100 mil track width
  internal = 1,000,000
  gerber   = 1,000,000 * 0.00254 = 2540.000000 (2.540000 mm)
  output   = "2540000" (with 6 decimal digits, no decimal point)
```

**Imperial format (e.g., %FSLAX25*% -- 2 integer, 5 decimal places):**
```
gerber_value = internal_units * 0.0000001 * 100,000
             = internal_units * 0.01

Example: 100 mil track width
  internal = 1,000,000
  gerber   = 1,000,000 * 0.01 = 10000.00 (0.10000 inch)
  output   = "10000" (with 5 decimal digits, no decimal point)
```

### Coordinate Formatter Implementation (from OUT_Gerber.dll FUN_013f77a0)

The reverse-engineered coordinate formatting pipeline:

```
Input:  internal_coord (i32), decimals (int), zeroes_mode (TZeroesMode), unit (TUnit)

Step 1 -- Unit conversion:
  if unit == Imperial:
    physical = FUN_00e4af20(internal_coord)       // internal -> inches
  if unit == Metric:
    physical = FUN_013bff40(internal_coord)       // internal -> mm (constant at 0x013f7930)

Step 2 -- Scale to integer representation:
  scaled = round(physical * 10^decimals)          // rounding via FUN_0040c870

Step 3 -- Format to string:
  raw_string = integer_to_string(abs(scaled))

Step 4 -- Zero suppression:
  Mode 0 (KeepAll):           left-pad with '0' to full width
  Mode 1 (SuppressLeading):   strip leading '0' characters
  Mode 2 (SuppressTrailing):  strip trailing '0' characters

Step 5 -- Sign:
  if scaled < 0: prepend '-'

Output: formatted coordinate string (no decimal point)
```

### Zero Suppression

Per `TZeroesMode`:
- **KeepLeadingAndTrailingZeroes**: `"002540000"` (full width, fixed)
- **SuppressLeadingZeroes**: `"2540000"` (drop leading zeros)
- **SuppressTrailingZeroes**: `"00254"` (drop trailing zeros)

### Origin Offset

Per `TOriginPosition`:
- **Absolute**: coordinates relative to (0, 0) in PCB space
- **Relative**: coordinates relative to a user-defined reference point on the board
- **Center**: coordinates relative to the center of the board bounding box

The origin offset is subtracted from every coordinate before formatting.
In TGerberFile, the origin is stored at offsets `+0x84` (X) and `+0x88` (Y).

---

## Implementation Strategy for altium-cli

### Prerequisites (PCB Record Parsing)

Gerber export requires parsed PCB data. Minimum record types needed:

| Record | Key Fields | Gerber Use |
|--------|-----------|------------|
| **Track** | start, end, width, layer, net | D01 line draw |
| **Pad** | position, shape, size, hole, layer, rotation, stack | D03 flash or drawn outline |
| **Via** | position, diameter, hole, from/to layers | D03 flash per layer |
| **Arc** | center, radius, start/end angle, width, layer | G02/G03 or linearized |
| **Fill** | corner coords, layer | D03 flash or region fill |
| **Region** | vertex list, layer, kind | G36/G37 contour fill |
| **Polygon** | vertex list, layer, hatch, pour | G36/G37 with thermal reliefs |
| **BoardOutline** | vertex list | Profile file |

### Module Structure

```
gerber/
  mod.rs
  writer.rs           -- GerberWriter: orchestrates file generation
  aperture_table.rs   -- scan PCB objects, build aperture table, assign D-codes
  coord_formatter.rs  -- PCB Coord -> Gerber coordinate string (matching FUN_013f77a0)
  layer_mapper.rs     -- V6Layer -> TGerberFileKind -> file extension
  header.rs           -- %FS, %MO, %AD, %AM, %TF (X2) blocks
  commands.rs         -- G01/G02/G03, D01/D02/D03 command sequences
  drill.rs            -- Excellon NC drill file writer
  settings.rs         -- GerberSettings config struct matching IGerberSettings
```

### Aperture Generation Algorithm

```
1. For each enabled layer:
   a. Collect all PCB objects on that layer
   b. For each object, determine its aperture signature:
      - Track: Round aperture, XSize = width
      - Pad (round): Round aperture, XSize = diameter
      - Pad (rect): Rectangle aperture, XSize = width, YSize = height
      - Pad (octagon): Octagon aperture, XSize = width
      - Pad (rounded rect): RoundedRectangle aperture with corner radius
      - Pad (custom): AM macro aperture
      - Fill: Rectangle aperture if flashable, else region
   c. Deduplicate: group objects by identical aperture signature
   d. Assign D-codes sequentially from D10
   e. Record in aperture table

2. Output aperture definitions (from OUT_Gerber.dll string format):
   - Round:       %%ADD{d}C,{diameter}*%      (FUN_013ff560)
   - Rectangle:   %%ADD{d}R,{x}X{y}*%
   - Octagon:     %%ADD{d}P,{od}X8X{rotation}*%  (regular polygon, 8 sides)
   - Oblong:      %%ADD{d}O,{x}X{y}*%        (FUN_013ffc00)
   - Custom:      %%AM{name}*...% then %%ADD{d}{name}*%  (FUN_013ffbd8)
   - Circle macro: %%AM CIRCLED{d}* 1,1,{s},{s},{s}*  (FUN_013ffb50/013ffba0)
```

### Per-Layer File Generation

Matching the exact sequence from TGerberFile decompilation:

```
1. Write header:
   G04 altium-cli generated Gerber*
   %FSLAX36*%          (FUN_014e8610: %FS + L/T + AX + {int}{dec} + Y + {int}{dec} + *%)
   %MOMM*%             (FUN_014e8860: or %MOIN*% for imperial)
   G75*                (FUN_014e85e0: multi-quadrant mode set early)
   G01*                (FUN_014e85c8: default to linear interpolation)

   If X2:
     G04 #@! TF.GenerationSoftware,altium-cli,{version}*    (FUN_014e4ed0 + 014e2364)
     G04 #@! TF.SameCoordinates,{id}*                       (FUN_014ec1b8)
     G04 #@! TF.FilePolarity,Positive*                       (FUN_014ec3f0)
     G04 Layer_Physical_Order={n}*                            (FUN_014e7a60)

2. Write aperture definitions:
   %ADD10C,0.254000*%
   %ADD11R,1.600000X1.600000*%
   ...
   If X2 aperture attributes:
     %TA.AperFunction,{function}*%                            (0x01409e8e)

3. Set initial polarity:
   %LPD*%             (dark polarity -- or %LPC*% for plane layers)

4. Write draw commands per object (matching TGerberFile methods):

   For each track:
     D{aperture}*                       (select aperture matching track width)
     X{x1}Y{y1}D02*                    (move to start -- FUN_014e4ef0)
     X{x2}Y{y2}D01*                    (draw to end)
     Note: X/Y only emitted if changed from last write (FUN_014e4740 optimization)

   For each pad (flashable):
     D{aperture}*                       (select aperture matching pad shape)
     X{x}Y{y}D03*                      (flash at position -- FUN_014e4db0)

   For each pad (complex / custom shape):
     G36*                                (begin region -- 0x014ebe18)
     X{v0x}Y{v0y}D02*                   (move to first vertex)
     X{v1x}Y{v1y}D01*                   (draw to next vertex)
     ...
     X{v0x}Y{v0y}D01*                   (close contour)
     G37*                                (end region -- 0x014ebe74)

   For each arc (FUN_014e4fa0):
     D{aperture}*
     X{start_x}Y{start_y}D02*           (move to start)
     G02* or G03*                        (set CW/CCW -- 0x014e50c8 / 014e50e0)
     X{end_x}Y{end_y}I{cx-sx}J{cy-sy}D01*  (arc with center offset)
     G01*                                (return to linear mode)

   For each region/polygon pour (FUN_014eba30):
     %LPD*% or %LPC*%                   (set polarity if changed -- FUN_014ebed0)
     G36*
     X{v0x}Y{v0y}D02*
     X{v1x}Y{v1y}D01*
     ...
     X{v0x}Y{v0y}D01*                   (close contour)
     G37*

   If X2 object attributes:
     G04 #@! TO.N,{net_name}*           (before copper objects)
     G04 #@! TO.C,{component_refdes}*   (before component objects)
     G04 #@! TO.P,{refdes},{pin}*       (before pad objects)
     G04 #@! TD*                        (reset attributes after)

5. Write footer:
   M02*                                  (FUN_014e46a0)
```

### Altium-Specific Behaviors to Match

From decompilation and C# interface analysis:

1. **Coordinate optimization** (FUN_014e4740): Only emit X or Y if the value changed
   from the last write. TGerberFile tracks last-written X at `+0x8c` and Y at `+0x90`.
2. **Stateful polarity tracking** (FUN_014ebed0): Only emit `%LPD*%`/`%LPC*%` when
   polarity actually changes. Stored at TGerberFile offset `+0x20`.
3. **G54 on aperture change**: When enabled, emit `G54D{nn}*` instead of just `D{nn}*`
4. **Flash pads**: Use D03 for standard shapes (round, rect, octagon). Fall back to
   drawn outline (G36/G37) for complex/custom shapes.
5. **Flash fills**: Use D03 for rectangular fills that match an aperture. Otherwise draw
   as region.
6. **Software arcs**: When enabled, approximate arcs as sequences of short line segments
   using chord tolerance. When disabled, use native G02/G03 with I/J center offsets
   (I/J = center minus start point, per FUN_014e4fa0).
7. **Relief shapes**: Generate thermal relief apertures (the `eApertureXxxRelief` shapes)
   for pad-to-plane connections. These are special apertures with spoke patterns.
8. **Octagonal decomposition**: When `UsePolygonFormOctagonalParts` is true, render
   octagonal pads as polygon outlines (G36/G37) instead of using the P aperture.
9. **Merge regions and pads**: When enabled, boolean-union overlapping copper shapes
   within the same footprint before output.
10. **Positive plane layers**: When `PlotPositivePlaneLayers` is true, output internal
    plane layers with dark polarity (%LPD) showing copper. When false, output with
    clear polarity (%LPC) showing the isolation/clearance pattern.
11. **Command sorting**: When `Sorted` is true, reorder draw commands to minimize
    pen travel distance (nearest-neighbor or similar heuristic).
12. **Optimize move commands**: When `OptimizeChangelocationCommands` is true, omit
    redundant D02 moves (e.g., when the next draw starts where the last one ended).
13. **Unconnected mid-layer pads**: When `IncludeUnconnectedMidLayerPads` is true,
    include pads on inner layers even if they have no net connection.
14. **Arc mode reset**: After each arc command, Altium explicitly returns to linear
    mode with `G01*` (confirmed in FUN_014e4fa0 decompilation).
15. **X2 attributes as structured comments**: Gerber X2 attributes are written using
    the `G04 #@! {attribute}*` format (FUN_014e4ed0), maintaining backward compatibility
    with RS-274X readers that treat G04 as comments.

### NC Drill File Generation

```
M48                           ; header start
;Layer_Color=9109504
;FILE_FORMAT=4:4              ; coordinate format
METRIC,LZ                    ; metric, leading zeros kept (or INCH,TZ etc)
;Gerber info (layer pair, etc.)
T01C0.300                     ; tool 1, diameter 0.3mm
T02C0.800                     ; tool 2, diameter 0.8mm
%                             ; end of header
T01                           ; select tool 1
X001500Y002500                ; drill at (1.5, 2.5)
X003200Y004100                ; drill at (3.2, 4.1)
T02                           ; select tool 2
X010000Y010000                ; drill at (10.0, 10.0)
M30                           ; end of file
```

Separate drill files are generated for each drill layer pair:
- PTH (plated through holes): top to bottom
- NPTH (non-plated through holes): top to bottom
- Blind vias: per layer pair
- Back drills: per layer pair
- Micro vias: per layer pair

### Gerber X2 Extensions

When `TGerberVersion::eX2` is selected, add file-level and object-level attributes:

**File attributes (in header):**
```
%TF.GenerationSoftware,altium-cli,{version}*%
%TF.CreationDate,{ISO8601}*%
%TF.FileFunction,{function}*%     -- derived from TGerberFileKind
%TF.FilePolarity,Positive*%       -- or Negative for plane layers
%TF.Part,{part}*%                 -- derived from TFileSubject
%TF.SameCoordinates,Original*%
```

Altium writes these as structured comments for backward compatibility:
```
G04 #@! TF.GenerationSoftware,Altium Limited,Altium Designer,{version} ({build})*
G04 #@! TF.SameCoordinates,{identifier}*
G04 #@! TF.FilePolarity,Positive*
G04 Layer_Physical_Order={layer_number}*
```

**FileFunction mapping from TGerberFileKind:**
```
gfkCopper (top)      -> Copper,L1,Top
gfkCopper (bottom)   -> Copper,L{n},Bot
gfkCopper (inner)    -> Copper,L{n},Inr
gfkSoldermask (top)  -> Soldermask,Top
gfkSoldermask (bot)  -> Soldermask,Bot
gfkPasteMask (top)   -> Paste,Top
gfkPasteMask (bot)   -> Paste,Bot
gfkOverlay (top)     -> Legend,Top
gfkOverlay (bot)     -> Legend,Bot
gfkProfile           -> Profile,NP
gfkDrillDrawing      -> FabricationDrawing
gfkDrillGuide        -> AssemblyDrawing
gfkPTH               -> Plated,1,{n},PTH
gfkNPTH              -> NonPlated,1,{n},NPTH
gfkBlindViaHoles     -> Plated,{from},{to},Blind
gfkBackdrills        -> Plated,{from},{to},Backdrill
gfkMicroVias         -> Plated,{from},{to},Micro
```

**Object attributes (inline, as structured comments):**
```
G04 #@! TO.N,{net_name}*            -- net name for copper objects
G04 #@! TO.C,{component_refdes}*    -- component reference designator
G04 #@! TO.P,{refdes},{pin}*        -- pad with component and pin info
G04 #@! TD*                          -- delete all object attributes (reset)
```

**Aperture attributes:**
```
%TA.AperFunction,{function}*%       -- set aperture function (before %ADD)
%TD.AperFunction*%                   -- delete aperture function (after use)
```

---

## Source Files Reference

### C# Interfaces (AD26-dotnet/)

```
Altium.Edp.Interfaces/RT_GerberOutputs/
  IGerberSettings.cs          -- master Gerber settings (236 methods)
  IGerberSettingsInfo.cs      -- settings persistence variant (GetState_* pattern)
  IApertureInfo.cs            -- aperture definition
  TGerberVersion.cs           -- eX2, e724X
  TGerberFileKind.cs          -- 16 file kind values
  TApertureShape.cs           -- 9 aperture shapes
  TApertureUsage.cs           -- flash/stroke/both
  TZeroesMode.cs              -- zero suppression modes
  TOriginPosition.cs          -- origin modes
  TOutputFormat.cs            -- single/per-layer
  TFileSubject.cs             -- X2 part type
  TLayerGroupType.cs          -- mechanical layer groups
  Consts.cs                   -- D-code min/max, FileSubject display names
  TGerberVersionConsts.cs     -- version constant helpers
  TApertureShapeConsts.cs     -- aperture shape constant helpers
  TDrillLayerPairType.cs      -- Regular, MicroViaDrill, Backdrill, CounterHole
  TExtendedHoleType.cs        -- eRoundHole, eSquareHole, eSlotHole

Altium.Edp.Interfaces/RT_PCB/
  IPCB_GerberOptions.cs       -- board-stored Gerber options
  TOutputDriverType.cs        -- eProtelGerber (byte 1) and others
  IPCB_ServerInterface.cs     -- PcbApi_Export_ToPainter signature (line ~137)

Altium.Edp.Interfaces/RT_Outputs/
  IGerberSettingsInfoBase.cs   -- empty base interface (marker)

Altium.SDK/Altium.Edp.Classes/
  OutputGenerator.cs           -- base class for all output generators
                                  RunGenerator() -> InternalRunGenerator() (abstract)
                                  ParameterTransferBegin() / ParameterTransferEnd()

Altium.SDK.Interfaces/PCB/
  IPCB_GerberOptions.cs       -- SDK-side interface duplicate
  IPCB_ServerInterface.cs     -- PcbApi_Export_ToPainter (line 198)

DXPServerSDK/Altium.Sdk.DxpAppServer.Common/
  CAMtasticFileChecker.cs      -- Gerber/drill/ODB++ file recognition
                                  CheckGerber() at line 136
                                  IsRS274XGerberFile() at line 501
                                  IsRS274DGerberFile() at line 492
```

### Delphi Entry Points -- Advpcb.dll (via ghidra, project: altium26)

```
PcbApi_QueryBoardGerberOptions          @ 0x03d40330  (28 params, bidirectional)
PcbApi_QueryBoardOutputOptions          @ 0x03d3ee10
PcbApi_QueryBoardOutputOptionsPlotLayers @ 0x03d3fee0
PcbApi_QueryBoardOutputOptionsFlipLayers @ 0x03d3ffe0
PcbApi_QueryBoardPrinterOptions         @ 0x03d40860
PcbApi_CreatePainter                    @ 0x03d58940  (COM factory, vtable +0x118)
PcbApi_Export_ToPainter                 @ 0x03d589d0  (QI + vtable +0x18)
PcbApi_Export_ToPainter_ByHandle        @ 0x03d58af0
PcbApi_GetLayerPolygonShapesForOutput   @ 0x03d5a1c0  (delegates to FUN_03d1eb80)
PcbApi_GetLayerPolygonShapesForOutputEx @ 0x03d5a210
```

### Delphi Gerber Writer -- OUT_Gerber.dll (via ghidra, project: altium26)

```
GetOutputGenerator                      @ export      (factory, creates TGerberFile)
TGerberFile vtable                      @ ~0x01a04580
TGerberFile.WriteFormatSpec             @ 0x014e8610  (%FSLAX...*%)
TGerberFile.WriteUnitMode               @ 0x014e8860  (%MOMM*% / %MOIN*%)
TGerberFile.WriteCircleAperture         @ 0x013ff560  (%%ADD{d}C,{s}*%)
TGerberFile.WriteOblongAperture         @ 0x013ffc00  (%%ADD{d}O,{x}X{y}*%)
TGerberFile.FormatCoordinate            @ 0x013f77a0  (internal -> formatted string)
TGerberFile.WriteCoordinates            @ 0x014e4740  (optimized X/Y output)
TGerberFile.WriteLineDraw               @ 0x014e4ef0  (D01/D02)
TGerberFile.WriteArc                    @ 0x014e4fa0  (G02/G03 + I/J)
TGerberFile.WriteFlash                  @ 0x014e4db0  (D03)
TGerberFile.WriteRegionFill             @ 0x014eba30  (G36/G37)
TGerberFile.WritePolarity               @ 0x014ebed0  (stateful %LPD/%LPC)
TGerberFile.WriteX2Attribute            @ 0x014e4ed0  (G04 #@! ...)
TGerberFile.WriteLayerOrder             @ 0x014e7a60  (G04 Layer_Physical_Order=)
TGerberFile.WriteEndOfFile              @ 0x014e46a0  (M02*)
TGerberFile.UnitScale                   @ 0x013f3690  (returns 4=imperial, 2=metric)
TGerberFile.GenerateCallback            @ 0x01562170  (main generation entry point)
TGerberFile.LayerOutputCallback         @ 0x01594b70  (per-layer output)
TGerberFile.SettingsCallback            @ 0x0156cee0  (configuration)
TGerberFile.SetupCallback               @ 0x0157c420  (initialization)

Key internal state offsets in TGerberFile:
  +0x20  current polarity state (bool, for stateful LPD/LPC toggle)
  +0x84  origin offset X (subtracted from all coordinates)
  +0x88  origin offset Y (subtracted from all coordinates)
  +0x8c  last written X value (for coordinate optimization)
  +0x90  last written Y value (for coordinate optimization)
```
