# Gerber Export in Altium Designer

Reverse-engineered findings on how Altium Designer generates Gerber (RS-274X / Gerber X2)
and NC Drill (Excellon) files, and what we need to replicate this in altium-cli.

## Architecture Overview

Altium's Gerber export is split across two layers:

1. **C# (.NET)** -- configuration interfaces, type definitions, orchestration, file validation
2. **Delphi (native DLLs)** -- actual PCB-to-Gerber conversion, coordinate math, file writing

The C# layer defines the full configuration surface and wires it into the OutputJob system.
The Delphi layer (`Advpcb.dll` and friends) does the heavy lifting.

### Data Flow

```
OutputJob Document (.OutJob)
    |
    v
IOutputer (registered generator, type = eProtelGerber)
    |
    v
IOutputGenerator.RunGenerator()
    |
    v
Delphi DLL entry points:
    PcbApi_QueryBoardGerberOptions    (0x03d40330)  -- read settings from board
    PcbApi_CreatePainter              (0x03d58940)  -- create renderer
    PcbApi_Export_ToPainter           (0x03d589d0)  -- push PCB data through painter
    PcbApi_GetLayerPolygonShapesForOutput (0x03d5a1c0)  -- precomputed polygon geometry
    |
    v
Per-layer Gerber files + drill files + reports
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

## Delphi Layer (Native DLLs)

### Entry Points in Advpcb.dll

From the `pcb-api-functions.md` reverse-engineering docs:

| Address | Function | Purpose |
|---------|----------|---------|
| 0x03d40330 | `PcbApi_QueryBoardGerberOptions` | Read Gerber settings from board |
| 0x03d3ee10 | `PcbApi_QueryBoardOutputOptions` | Read general output options |
| 0x03d3fee0 | `PcbApi_QueryBoardOutputOptionsPlotLayers` | Get plot layer config |
| 0x03d3ffe0 | `PcbApi_QueryBoardOutputOptionsFlipLayers` | Get flip layer config |
| 0x03d40860 | `PcbApi_QueryBoardPrinterOptions` | Read printer options |
| 0x03d58940 | `PcbApi_CreatePainter` | Factory: create output renderer |
| 0x03d589d0 | `PcbApi_Export_ToPainter` | Export PCB data through painter |
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

### What the Delphi Code Does

The actual generation logic in the Delphi layer handles:

1. **Object iteration** -- walks all PCB primitives on a given layer
2. **Aperture table generation** -- scans pads/tracks/fills, creates unique aperture
   for each distinct geometry (shape + size combination)
3. **D-code assignment** -- sequential starting at D10
4. **Coordinate transformation** -- PCB internal coords → Gerber format string
5. **Arc rendering** -- hardware G02/G03 curves or software linearization
6. **Polygon/region rendering** -- G36/G37 contour fill
7. **Pad rendering** -- D03 flash for standard shapes, drawn outline for complex
8. **Track rendering** -- select aperture by width, D01 draw between endpoints
9. **Relief shapes** -- thermal relief patterns for plane connections
10. **Command optimization** -- sort commands, remove redundant moves
11. **File I/O** -- write header, aperture block, commands, footer

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
  coord_formatter.rs  -- PCB Coord -> Gerber coordinate string
  layer_filter.rs     -- filter PCB objects by target layer
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

2. Output aperture definitions:
   - Round:       %ADD{d}C,{diameter}*%
   - Rectangle:   %ADD{d}R,{x}X{y}*%
   - Octagon:     %ADD{d}P,{od}X8X{rotation}*%  (regular polygon, 8 sides)
   - Oblong:      %ADD{d}O,{x}X{y}*%
   - Custom:      %AM{name}*...% then %ADD{d}{name}*%
```

### Per-Layer File Generation

```
1. Write header:
   G04 Altium-cli generated Gerber*
   %FSLAX36*%          (or X25 for inches)
   %MOMM*%             (or %MOIN*%)
   %TF.GenerationSoftware,altium-cli,...*%   (X2 only)
   %TF.FileFunction,...*%                     (X2 only)

2. Write aperture definitions:
   %ADD10C,0.254000*%
   %ADD11R,1.600000X1.600000*%
   ...

3. Set defaults:
   G75*                (multi-quadrant mode)
   %LPD*%             (dark polarity, or %LPC*% for clear/plane layers)

4. Write draw commands per object:
   For each track:
     D{aperture}*      (select aperture matching track width)
     X{x1}Y{y1}D02*   (move to start)
     X{x2}Y{y2}D01*   (draw to end)

   For each pad (flashable):
     D{aperture}*      (select aperture matching pad shape)
     X{x}Y{y}D03*     (flash at position)

   For each pad (complex / custom shape):
     G36*              (begin region)
     X{v0x}Y{v0y}D02* (move to first vertex)
     X{v1x}Y{v1y}D01* (draw to next vertex)
     ...
     G37*              (end region)

   For each arc:
     If software arcs: linearize into line segments, output as D01 draws
     If hardware arcs:
       D{aperture}*
       G75*
       X{start_x}Y{start_y}D02*
       G02/G03 X{end_x}Y{end_y}I{cx}J{cy}D01*

   For each region/polygon pour:
     G36*
     X{v0x}Y{v0y}D02*
     X{v1x}Y{v1y}D01*
     ...
     X{v0x}Y{v0y}D01*  (close contour)
     G37*

5. Write footer:
   M02*
```

### Altium-Specific Behaviors to Match

1. **G54 on aperture change**: When enabled, emit `G54D{nn}*` instead of just `D{nn}*`
2. **Flash pads**: Use D03 for standard shapes (round, rect, octagon). Fall back to
   drawn outline (G36/G37) for complex/custom shapes.
3. **Flash fills**: Use D03 for rectangular fills that match an aperture. Otherwise draw
   as region.
4. **Software arcs**: When enabled, approximate arcs as sequences of short line segments
   using chord tolerance. When disabled, use native G02/G03 with I/J center offsets.
5. **Relief shapes**: Generate thermal relief apertures (the `eApertureXxxRelief` shapes)
   for pad-to-plane connections. These are special apertures with spoke patterns.
6. **Octagonal decomposition**: When `UsePolygonFormOctagonalParts` is true, render
   octagonal pads as polygon outlines (G36/G37) instead of using the P aperture.
7. **Merge regions and pads**: When enabled, boolean-union overlapping copper shapes
   within the same footprint before output.
8. **Positive plane layers**: When `PlotPositivePlaneLayers` is true, output internal
   plane layers with dark polarity (%LPD) showing copper. When false, output with
   clear polarity (%LPC) showing the isolation/clearance pattern.
9. **Command sorting**: When `Sorted` is true, reorder draw commands to minimize
   pen travel distance (nearest-neighbor or similar heuristic).
10. **Optimize move commands**: When `OptimizeChangelocationCommands` is true, omit
    redundant D02 moves (e.g., when the next draw starts where the last one ended).
11. **Unconnected mid-layer pads**: When `IncludeUnconnectedMidLayerPads` is true,
    include pads on inner layers even if they have no net connection.

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

**Object attributes (inline):**
```
%TO.N,{net_name}*%            -- net name for copper objects
%TO.C,{component_refdes}*%    -- component reference designator
%TO.P,{refdes},{pin}*%        -- pad with component and pin info
%TD*%                          -- delete all object attributes (reset)
```

---

## Source Files Reference

### C# Interfaces (AD26-dotnet/)

```
Altium.Edp.Interfaces/RT_GerberOutputs/
  IGerberSettings.cs          -- master Gerber settings (236 methods)
  IGerberSettingsInfo.cs      -- settings persistence variant
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

Altium.Edp.Interfaces/RT_PCB/
  IPCB_GerberOptions.cs       -- board-stored Gerber options
  TOutputDriverType.cs        -- eProtelGerber and others

Altium.Edp.Interfaces/RT_Outputs/
  IGerberSettingsInfoBase.cs   -- empty base interface (marker)

Altium.SDK/Altium.Edp.Classes/
  OutputGenerator.cs           -- base class for all output generators

DXPServerSDK/Altium.Sdk.DxpAppServer.Common/
  CAMtasticFileChecker.cs      -- Gerber/drill/ODB++ file recognition
```

### Delphi Entry Points (Advpcb.dll via ghidra)

```
PcbApi_QueryBoardGerberOptions          @ 0x03d40330
PcbApi_QueryBoardOutputOptions          @ 0x03d3ee10
PcbApi_QueryBoardOutputOptionsPlotLayers @ 0x03d3fee0
PcbApi_QueryBoardOutputOptionsFlipLayers @ 0x03d3ffe0
PcbApi_QueryBoardPrinterOptions         @ 0x03d40860
PcbApi_CreatePainter                    @ 0x03d58940
PcbApi_Export_ToPainter                 @ 0x03d589d0
PcbApi_GetLayerPolygonShapesForOutput   @ 0x03d5a1c0
PcbApi_GetLayerPolygonShapesForOutputEx @ 0x03d5a210
```
