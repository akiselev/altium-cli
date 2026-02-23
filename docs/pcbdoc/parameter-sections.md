# Parameter Sections

PcbDoc files store many object types as pipe-delimited key-value parameter strings
rather than packed binary records. This document covers the two parameter block formats
and the parameter keys used by each section.

---

## 1. Parameter Block Formats

### 1.1 Standard Parameter Block

Used by the majority of parameter sections. Each record is:

```
+----------+----------------------------------+
| Length   | Parameter String                  |
| 4 bytes  | N bytes (Win1252, NUL-terminated) |
+----------+----------------------------------+
```

- **Length**: u32 LE. Byte count of the parameter string including its NUL terminator.
- **Parameter String**: Pipe-delimited key-value pairs encoded in Windows-1252:
  `|KEY1=VALUE1|KEY2=VALUE2|...\0`

Records are concatenated back-to-back in the `Data` stream. The `Header` stream
contains a u32 LE record count.

### 1.2 Prefixed Parameter Block

Used by Rules6, Dimensions6, Coordinates6, and NewRules6. Each record has a 2-byte
prefix before the standard length+string:

```
+--------+----------+----------------------------------+
| Prefix | Length   | Parameter String                  |
| 2 bytes| 4 bytes  | N bytes (Win1252, NUL-terminated) |
+--------+----------+----------------------------------+
```

- **Prefix**: u16 LE. Meaning is section-specific:
  - **Rules6 / NewRules6**: `TRuleKind` enum value (see section 3.5)
  - **Dimensions6**: `TDimensionKind` enum value (see section 3.7)
  - **Coordinates6**: purpose unclear (no records observed in test files; likely
    analogous to Dimensions6)

---

## 2. Common Base Parameters

Most parameter sections share a set of common primitive base parameters inherited from
`IPCB_Primitive`. These appear at the start of every record:

| Key | Type | Description |
|-----|------|-------------|
| `SELECTION` | bool | Whether the primitive is selected |
| `LAYER` | string | Layer name (e.g., `TOP`, `BOTTOM`, `MID1`, `MULTILAYER`) |
| `LOCKED` | bool | Whether the primitive is position-locked |
| `POLYGONOUTLINE` | bool | Whether this is a polygon outline primitive |
| `USERROUTED` | bool | Whether the primitive was manually routed |
| `KEEPOUT` | bool | Whether this is a keepout primitive |
| `UNIONINDEX` | int | Index of the union this primitive belongs to (0 = none) |
| `PRIMITIVELOCK` | bool | Whether the primitive is individually locked |
| `UNIQUEID` | string | 8-character unique identifier (e.g., `HJTCPLIN`) |

Not all sections include every base parameter. The options sections (Advanced Placer
Options6, Design Rule Checker Options6, Pin Swap Options6) use a `RECORD` key instead
of the standard base parameters.

---

## 3. Section-by-Section Reference

### 3.1 Nets6

**Format**: Standard parameter block
**Content**: Net definitions -- one record per net in the design.

| Key | Type | Description |
|-----|------|-------------|
| `NAME` | string | Net name (e.g., `GND`, `VCC3P3`) |
| `VISIBLE` | bool | Whether the net is visible |
| `COLOR` | int | Win32 COLORREF color (0x00BBGGRR) |
| `LOOPREMOVAL` | bool | Whether loop removal is enabled for this net |
| `OVERRIDECOLORFORDRAW` | bool | Override the net color for drawing |
| `TOPLAYER_MRWIDTH` | coord | Maximum routed width on top layer |
| `MIDLAYER{1..30}_MRWIDTH` | coord | Maximum routed width on mid layers 1-30 |
| `BOTTOMLAYER_MRWIDTH` | coord | Maximum routed width on bottom layer |
| `JUMPERSVISIBLE` | bool | Whether jumpers are visible on this net |

Shorter net records may omit the per-layer `MRWIDTH` parameters and have only the
core parameters (NAME, VISIBLE, COLOR, etc.).

Source: `IPCB_Net_SaveLoadParameters` in
`AD26-dotnet/Altium.Edp.Interfaces/PCBInterfaces/IPCB_Net_SaveLoadParameters.cs`

### 3.2 Components6

**Format**: Standard parameter block
**Content**: Component (footprint instance) definitions -- one record per placed component.

| Key | Type | Description |
|-----|------|-------------|
| `X` | coord | Component origin X position |
| `Y` | coord | Component origin Y position |
| `PATTERN` | string | Footprint pattern name (e.g., `RES0402`, `QFN48`) |
| `NAMEON` | bool | Whether the designator text is visible |
| `COMMENTON` | bool | Whether the comment text is visible |
| `GROUPNUM` | int | Group number |
| `COUNT` | int | Primitive count (number of child primitives) |
| `ROTATION` | float | Rotation angle in Delphi scientific notation |
| `HEIGHT` | coord | Component height (3D) |
| `CHANNELOFFSET` | int | Multi-channel offset |
| `SOURCEDESIGNATOR` | string | Original schematic designator (e.g., `R96`) |
| `SOURCEUNIQUEID` | string | Schematic unique ID (prefixed with `\`) |
| `SOURCEHIERARCHICALPATH` | string | Schematic hierarchical path |
| `SOURCEFOOTPRINTLIBRARY` | string | Source footprint library file name |
| `SOURCECOMPONENTLIBRARY` | string | Source schematic library file name |
| `SOURCELIBREFERENCE` | string | Source library reference name |
| `FOOTPRINTDESCRIPTION` | string | Footprint description text |
| `SOURCECOMPLIBIDENTIFIERKIND` | int | `TLibIdentifierKind` enum value |
| `SOURCECOMPLIBRARYIDENTIFIER` | string | Source component library identifier |
| `JUMPERSVISIBLE` | bool | Whether jumpers are visible |

Source: `IPCB_Component_SaveLoadParameters` in
`AD26-dotnet/Altium.Edp.Interfaces/PCBInterfaces/IPCB_Component_SaveLoadParameters.cs`

### 3.3 Polygons6

**Format**: Standard parameter block
**Content**: Copper pour polygon definitions -- one record per polygon.

| Key | Type | Description |
|-----|------|-------------|
| `POLYGONTYPE` | string | Polygon type (`Polygon`, `SplitPlane`, etc.) |
| `POUROVER` | bool | Whether to pour over same-net objects |
| `REMOVEDEAD` | bool | Remove dead copper islands |
| `GRIDSIZE` | coord | Pour grid size |
| `TRACKWIDTH` | coord | Hatch line track width |
| `HATCHSTYLE` | string | Hatch style (`Solid`, `45Degree`, `90Degree`, etc.) |
| `USEOCTAGONS` | bool | Use octagonal pad connections |
| `MINPRIMLENGTH` | coord | Minimum primitive length threshold |
| `KIND{N}` | int | Vertex N kind: 0=line, 1=arc |
| `VX{N}` | coord | Vertex N X coordinate |
| `VY{N}` | coord | Vertex N Y coordinate |
| `CX{N}` | coord | Vertex N arc center X (0 for lines) |
| `CY{N}` | coord | Vertex N arc center Y (0 for lines) |
| `SA{N}` | float | Vertex N start angle (arcs only) |
| `EA{N}` | float | Vertex N end angle (arcs only) |
| `R{N}` | coord | Vertex N arc radius (0 for lines) |
| `SHELVED` | bool | Whether the polygon is shelved (not poured) |
| `RESTORELAYER` | string | Layer before shelving (or `UNKNOWN`) |
| `RESTORENET` | string | Net name for the polygon |
| `REMOVEISLANDSBYAREA` | bool | Remove small copper islands |
| `REMOVENECKS` | bool | Remove narrow necks |
| `AREATHRESHOLD` | float | Area threshold for island removal |
| `ARCRESOLUTION` | coord | Arc approximation resolution |
| `NECKWIDTHTHRESHOLD` | coord | Neck width threshold for removal |
| `POUROVERSTYLE` | int | Pour-over style mode |
| `NAME` | string | Polygon name (comma-separated ASCII codes when AUTONAME=TRUE) |
| `POURINDEX` | int | Pour order index |
| `IGNOREVIOLATIONS` | bool | Ignore DRC violations during pour |
| `AUTONAME` | bool | Whether name is auto-generated |
| `NET` | int | Net index (into Nets6 section) |

The vertex parameters (`KIND{N}`, `VX{N}`, `VY{N}`, etc.) define the polygon outline.
Each vertex is either a line segment endpoint (KIND=0) or an arc (KIND=1) with center
point (CX/CY), start/end angles (SA/EA), and radius (R).

Source: `IPCB_PolygonsBinarySection` in
`AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_PolygonsBinarySection.cs`

### 3.4 Classes6

**Format**: Standard parameter block
**Content**: Object class definitions -- one record per class (net classes, component
classes, layer classes, etc.).

| Key | Type | Description |
|-----|------|-------------|
| `NAME` | string | Class name (e.g., `All Nets`, `Top Side Components`) |
| `KIND` | int | Class kind (see table below) |
| `SUPERCLASS` | bool | Whether this is a superclass (contains all objects) |
| `M{N}` | string | Member N name (net name, component designator, etc.) |
| `SELECTED` | bool | Whether the class is selected |
| `SCHAUTOGENERATEDCLUSTER` | bool | Whether auto-generated from schematic |

**KIND values** (observed from real files):

| Value | Meaning |
|-------|---------|
| 0 | Net Class |
| 1 | Component Class |
| 2 | From-To Class |
| 3 | Pad Class |
| 4 | Layer Class |
| 6 | Differential Pair Class |
| 7 | Polygon Class |
| 10 | xSignal Class |

The `M{N}` parameters enumerate class members. For net classes, each `M{N}` is a net
name. For component classes, each `M{N}` is a component designator.

### 3.5 Rules6

**Format**: Prefixed parameter block (u16 prefix = `TRuleKind`)
**Content**: Design rule definitions -- one record per rule.

The u16 prefix encodes the `TRuleKind` enum value, identifying the rule type:

| Prefix | TRuleKind | RULEKIND String |
|--------|-----------|-----------------|
| 0 | `eRule_Clearance` | `Clearance` |
| 2 | `eRule_MaxMinWidth` | `Width` |
| 4 | `eRule_MatchedLengths` | `MatchedLengths` |
| 6 | `eRule_PowerPlaneConnectStyle` | `PlaneConnect` |
| 7 | `eRule_RoutingTopology` | `RoutingTopology` |
| 8 | `eRule_RoutingPriority` | `RoutingPriority` |
| 9 | `eRule_RoutingLayers` | `RoutingLayers` |
| 10 | `eRule_RoutingCornerStyle` | `RoutingCorners` |
| 11 | `eRule_RoutingViaStyle` | `RoutingVias` |
| 12 | `eRule_PowerPlaneClearance` | `PlaneClearance` |
| 13 | `eRule_SolderMaskExpansion` | `SolderMaskExpansion` |
| 14 | `eRule_PasteMaskExpansion` | `PasteMaskExpansion` |
| 15 | `eRule_ShortCircuit` | `ShortCircuit` |
| 16 | `eRule_BrokenNets` | `UnRoutedNet` |
| 20 | `eRule_PolygonConnectStyle` | `PolygonConnect` |
| 24 | `eRule_ComponentClearance` | `ComponentClearance` |
| 42 | `eRule_MaxMinHoleSize` | `HoleSize` |
| 43 | `eRule_TestPointStyle` | `FabricationTestpoint` |
| 44 | `eRule_TestPointUsage` | `FabricationTestPointUsage` |
| 48 | `eRule_LayerPair` | `LayerPairs` |
| 49 | `eRule_FanoutControl` | `FanoutControl` |
| 50 | `eRule_MaxMinHeight` | `Height` |
| 51 | `eRule_DifferentialPairsRouting` | `DiffPairsRouting` |
| 52 | `eRule_HoleToHoleClearance` | `HoleToHoleClearance` |
| 53 | `eRule_MinimumSolderMaskSliver` | `MinimumSolderMaskSliver` |
| 54 | `eRule_SilkToSolderMaskClearance` | `SilkToSolderMaskClearance` |
| 55 | `eRule_SilkToSilkClearance` | `SilkToSilkClearance` |
| 56 | `eRule_NetAntennae` | `NetAntennae` |
| 57 | `eRule_AssyTestPointStyle` | `AssemblyTestpoint` |
| 58 | `eRule_AssyTestPointUsage` | `AssemblyTestPointUsage` |
| 59 | `eRule_SilkToBoardRegion` | `SilkToBoardRegionClearance` |
| 62 | `eRule_ModifiedPolygon` | `UnpouredPolygon` |
| 63 | `eRule_BoardOutlineClearance` | `BoardOutlineClearance` |

**Common rule parameters** (present in all rules):

| Key | Type | Description |
|-----|------|-------------|
| `RULEKIND` | string | Rule kind string (see table above) |
| `NETSCOPE` | string | Net scope (`AnyNet`, `DifferentNets`, etc.) |
| `LAYERKIND` | string | Layer scope (`SameLayer`, `AnyLayer`, etc.) |
| `SCOPE1EXPRESSION` | string | First scope query expression |
| `SCOPE2EXPRESSION` | string | Second scope query expression |
| `NAME` | string | Rule name |
| `ENABLED` | bool | Whether the rule is enabled |
| `PRIORITY` | int | Rule priority (lower = higher priority) |
| `COMMENT` | string | Rule comment |
| `DEFINEDBYLOGICALDOCUMENT` | bool | Whether defined by schematic |

**Rule-specific parameters vary by RULEKIND.** Examples:

- **Clearance**: `GAP`, `COLLISIONCHECKMODE`, `VERTICALGAP`, `SHOWDISTANCES`
- **Width**: `MINLIMIT`, `MAXLIMIT`, `PREFEREDWIDTH`
- **PolygonConnect**: `CONNECTSTYLE`, `RELIEFCONDUCTORWIDTH`, `RELIEFENTRIES`,
  `POLYGONRELIEFANGLE`, `AIRGAPWIDTH`
- **RoutingVias**: `HOLEWIDTH`, `WIDTH`, `VIASTYLE`, `MINHOLEWIDTH`, `MINWIDTH`,
  `MAXHOLEWIDTH`, `MAXWIDTH`
- **ComponentClearance**: `GAP`, `COLLISIONCHECKMODE`, `VERTICALGAP`, `SHOWDISTANCES`
- **DiffPairsRouting**: extensive parameters for differential pair routing constraints
- **RoutingLayers**: per-layer enable flags (e.g., `TOPLAYER=TRUE`, `MIDLAYER1=FALSE`)

Source: `TRuleKind` in `AD26-dotnet/Altium.SDK.Interfaces/PCB/TRuleKind.cs`

### 3.6 NewRules6

**Format**: Prefixed parameter block (u16 prefix = `TRuleKind`)
**Content**: Extended design rules using the same format as Rules6.

This section stores rules added in newer format versions. The prefix is the same
`TRuleKind` enum value. No records were observed in the test files; the section
exists as a placeholder in the CFB structure.

### 3.7 Dimensions6

**Format**: Prefixed parameter block (u16 prefix = `TDimensionKind`)
**Content**: Dimension annotation objects.

The u16 prefix encodes the `TDimensionKind` enum value:

| Prefix | TDimensionKind | Description |
|--------|----------------|-------------|
| 0 | `eNoDimension` | No dimension type |
| 1 | `eLinearDimension` | Linear (distance) dimension |
| 2 | `eAngularDimension` | Angular dimension |
| 3 | `eRadialDimension` | Radial (radius) dimension |
| 4 | `eLeaderDimension` | Leader line dimension |
| 5 | `eDatumDimension` | Datum dimension |
| 6 | `eBaselineDimension` | Baseline dimension |
| 7 | `eCenterDimension` | Center mark dimension |
| 8 | `eOriginalDimension` | Original dimension |
| 9 | `eLinearDiameterDimension` | Linear diameter dimension |
| 10 | `eRadialDiameterDimension` | Radial diameter dimension |

**Key parameters:**

| Key | Type | Description |
|-----|------|-------------|
| `DIMENSIONLAYER` | string | Layer for the dimension annotation |
| `DIMENSIONLOCKED` | bool | Whether the dimension is locked |
| `OBJECTID` | int | TObjectId (always 13 for dimensions) |
| `DIMENSIONKIND` | int | `TDimensionKind` value (matches prefix) |
| `DRCERROR` | bool | Whether there is a DRC error |
| `VINDEXFORSAVE` | int | Save index for violation tracking |
| `LX`, `LY` | coord | Bounding box lower-left |
| `HX`, `HY` | coord | Bounding box upper-right |
| `X1`, `Y1` | coord | First reference point |
| `X2`, `Y2` | coord | Second reference point |
| `TEXTX`, `TEXTY` | coord | Dimension text position |
| `HEIGHT` | coord | Dimension line height/offset |
| `LINEWIDTH` | coord | Dimension line width |
| `TEXTHEIGHT` | coord | Text character height |
| `TEXTWIDTH` | coord | Text stroke width |
| `FONT` | string | Font name or `DEFAULT` |
| `STYLE` | string | Dimension style (`None`, etc.) |
| `TEXTLINEWIDTH` | coord | Text line stroke width |
| `TEXTPOSITION` | string | Text position mode (`Auto`, `Manual`) |
| `TEXTGAP` | coord | Gap between text and dimension line |
| `TEXTFORMAT` | int | Text format code |
| `TEXTDIMENSIONUNIT` | string | Display unit (`Millimeters`, `Centimeters`, `Mils`, etc.) |
| `TEXTPRECISION` | int | Number of decimal places |
| `TEXTPREFIX` | string | Text prefix string |
| `TEXTSUFFIX` | string | Text suffix string (e.g., `cm`, `mm`) |
| `ARROWSIZE` | coord | Arrow head size |
| `ARROWLINEWIDTH` | coord | Arrow line width |
| `ARROWLENGTH` | coord | Arrow length |
| `ARROWPOSITION` | string | Arrow position (`Inside`, `Outside`) |
| `EXTENSIONOFFSET` | coord | Extension line offset |
| `EXTENSIONLINEWIDTH` | coord | Extension line width |
| `EXTENSIONPICKGAP` | coord | Extension line pick gap |
| `REFERENCES_COUNT` | int | Number of reference primitives |
| `REFERENCE{N}PRIM` | int | Reference N primitive index |
| `REFERENCE{N}OBJECTID` | int | Reference N object type |
| `REFERENCE{N}OBJECTSTRING` | string | Reference N object type name |
| `REFERENCE{N}POINTX` | coord | Reference N anchor point X |
| `REFERENCE{N}POINTY` | coord | Reference N anchor point Y |
| `REFERENCE{N}ANCHOR` | int | Reference N anchor type |
| `TEXT1X`, `TEXT1Y` | coord | Secondary text position |
| `TEXT1ANGLE` | float | Secondary text rotation angle |
| `TEXT1MIRROR` | bool | Secondary text mirrored |
| `USETTFONTS` | bool | Use TrueType fonts |
| `BOLD` | bool | Bold text |
| `ITALIC` | bool | Italic text |
| `FONTNAME` | string | TrueType font name |
| `ANGLE` | float | Overall dimension angle |

Source: `TDimensionKind` in `AD26-dotnet/Altium.SDK.Interfaces/PCB/TDimensionKind.cs`,
`IPCB_DimensionsSection` in `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_DimensionsSection.cs`

### 3.8 Coordinates6

**Format**: Prefixed parameter block (u16 prefix)
**Content**: Coordinate annotation objects.

Similar to Dimensions6 but for coordinate-style annotations. No records were
observed in the test files. The section shares the prefixed parameter block format.

Source: `IPCB_Coordinate_SaveLoadParameters` in
`AD26-dotnet/Altium.Edp.Interfaces/PCBInterfaces/IPCB_Coordinate_SaveLoadParameters.cs`

### 3.9 DifferentialPairs6

**Format**: Standard parameter block
**Content**: Differential pair definitions -- one record per differential pair.

| Key | Type | Description |
|-----|------|-------------|
| `POSITIVENETNAME` | string | Positive net name (e.g., `TX2_1_B_P`) |
| `NEGATIVENETNAME` | string | Negative net name (e.g., `TX2_1_B_N`) |
| `NAME` | string | Differential pair name (e.g., `TX2_1_B`) |
| `GATHERCONTROL` | bool | Whether gather control is enabled |

Source: `IPCB_DifferentialPair_SaveLoadParameters` in
`AD26-dotnet/Altium.Edp.Interfaces/PCBInterfaces/IPCB_DifferentialPair_SaveLoadParameters.cs`

### 3.10 FromTos6

**Format**: Standard parameter block
**Content**: FromTo (ratsnest endpoint) definitions.

No records observed in the test files. FromTos represent explicit ratsnest connections.

### 3.11 EmbeddedBoards6

**Format**: Standard parameter block
**Content**: Embedded board array definitions -- one record per embedded board.

| Key | Type | Description |
|-----|------|-------------|
| `X1`, `Y1` | coord | Bounding box lower-left |
| `X2`, `Y2` | coord | Bounding box upper-right |
| `ROTATION` | float | Rotation angle |
| `ISVIEWPORT` | bool | Whether this is a viewport |
| `VIEWPORTX1`, `VIEWPORTY1` | coord | Viewport lower-left |
| `VIEWPORTX2`, `VIEWPORTY2` | coord | Viewport upper-right |
| `VIEWPORTSCALE` | float | Viewport zoom scale |
| `VIEWPORTVISIBLE` | bool | Viewport visibility |
| `VIEWPORTTITLE` | string | Viewport title |
| `FONTNAME` | string | Font name for title |
| `FONTSIZE` | int | Font size |
| `FONTCOLOR` | int | Font color (Win32 COLORREF) |
| `VISIBLELAYERS` | string | Layer visibility hash (complex serialized format) |
| `DOCUMENTPATH` | string | Path to the embedded PcbDoc file |
| `X`, `Y` | coord | Array origin position |
| `ROWSPACING` | coord | Row spacing |
| `COLSPACING` | coord | Column spacing |
| `ROWCOUNT` | int | Number of rows |
| `COLCOUNT` | int | Number of columns |
| `MIRROR` | bool | Whether the array is mirrored |
| `ORIGINMODE` | int | Origin mode |

Source: `IPCB_EmbeddedBoard_SaveLoadParameters` in
`AD26-dotnet/Altium.Edp.Interfaces/PCBInterfaces/IPCB_EmbeddedBoard_SaveLoadParameters.cs`

### 3.12 Embeddeds6

**Format**: Standard parameter block
**Content**: Generic embedded object definitions.

No records observed in the test files.

### 3.13 SmartUnions

**Format**: Standard parameter block
**Content**: Smart union (accordion/meander) definitions.

| Key | Type | Description |
|-----|------|-------------|
| `UNIONTYPE` | int | Union type code |
| `FUNIONINDEX` | int | Union index |
| `PRIMITIVESLOCKED` | bool | Whether primitives are locked |
| `ALIGNMENT` | int | Alignment mode |
| `MIRROR` | bool | Mirrored |
| `LINE{N}.X1`, `LINE{N}.Y1` | coord | Line segment N start |
| `LINE{N}.X2`, `LINE{N}.Y2` | coord | Line segment N end |
| `LINE{N}.WIDTH` | coord | Line segment N width |
| `ACCLIST{N}X`, `ACCLIST{N}Y` | coord | Accordion element N position |
| `ACCLIST{N}R` | coord | Accordion element N radius |
| `ACCLIST{N}A1`, `ACCLIST{N}A2` | float | Accordion element N angles |
| `ACCLIST{N}WIDTH` | coord | Accordion element N width |
| `ACCLIST{N}X1`, `ACCLIST{N}Y1` | coord | Accordion line element N start |
| `ACCLIST{N}X2`, `ACCLIST{N}Y2` | coord | Accordion line element N end |
| `SX`, `SY` | coord | Start point |
| `EX`, `EY` | coord | End point |
| `ACCORDIONLENGTH` | coord | Total accordion length |
| `NETNAME` | string | Net name being tuned |
| `GAP` | coord | Accordion gap |
| `STYLE` | int | Accordion style |
| `AMPLITUDE` | coord | Accordion amplitude |
| `TOLERANCE` | coord | Length tolerance |
| `TARGETLENGTHMODE` | int | Target length mode |
| `AMPLITUDEINCREMENT` | coord | Amplitude increment |
| `GAPINCREMENT` | coord | Gap increment |
| `CLIPTOTARGETLENGTH` | bool | Clip to target length |
| `MITTERRADIUSRATIO` | float | Miter radius ratio |
| `TUNEDOBJECT` | string | Name of the object being tuned |
| `TARGETLENGTH` | int | Target length in internal units |
| `MATCHEDLENGTHRULE` | string | Matched length rule unique ID |
| `ISSHAPEBASED` | bool | Whether shape-based |
| `MAINCONTOURVERTEXCOUNT` | int | Vertex count of bounding contour |
| `KIND{N}`, `VX{N}`, `VY{N}`, etc. | various | Contour vertices (same format as Polygons6) |

### 3.14 PrimitiveParameters

**Format**: Standard parameter block
**Content**: Per-primitive parameter overrides (component parameters, BOM data, etc.).

Records come in two flavors -- a header record followed by value records:

**Header record:**

| Key | Type | Description |
|-----|------|-------------|
| `PRIMITIVEID` | string | Unique ID of the primitive |
| `VARIANTGUID` | string | Variant GUID (empty for base design) |
| `COUNT` | int | Number of parameter value records following |

**Value record:**

| Key | Type | Description |
|-----|------|-------------|
| `NAME` | string | Parameter name (e.g., `Assembly Info`, `Category`) |
| `VALUE` | string | Parameter value |
| `ISIMPORTED` | bool | Whether imported from schematic |

### 3.15 UniqueIDPrimitiveInformation

**Format**: Standard parameter block (sidecar)
**Content**: Per-primitive unique ID assignments.

| Key | Type | Description |
|-----|------|-------------|
| `PRIMITIVEINDEX` | int | Index of the primitive in its section |
| `PRIMITIVEOBJECTID` | string | Object type name (e.g., `Pad`, `Track`) |
| `UNIQUEID` | string | 8-character unique identifier |

### 3.16 ExtendedPrimitiveInformation

**Format**: Standard parameter block (sidecar)
**Content**: Per-primitive extended property overrides added in later format versions.

| Key | Type | Description |
|-----|------|-------------|
| `PRIMITIVEINDEX` | int | Index of the primitive in its section |
| `PRIMITIVEOBJECTID` | string | Object type name (e.g., `Region`) |
| `TYPE` | string | Extended property type (e.g., `Mask`) |
| `SOLDERMASKEXPANSIONMODE` | string | Solder mask mode (`Manual`, `None`, `FromRule`) |
| `SOLDERMASKEXPANSION_MANUAL` | coord | Manual solder mask expansion |
| `PASTEMASKEXPANSIONMODE` | string | Paste mask mode (`Manual`, `None`, `FromRule`) |

### 3.17 PadViaLibrary

**Format**: Standard parameter block
**Content**: Pad/via template library definitions.

| Key | Type | Description |
|-----|------|-------------|
| `PADVIALIBRARY.LIBRARYID` | string | Library GUID |
| `PADVIALIBRARY.LIBRARYNAME` | string | Library name (e.g., `<Local>`) |
| `PADVIALIBRARY.DISPLAYUNITS` | int | Display units code |

### 3.18 PadViaLibraryCache

**Format**: Standard parameter block
**Content**: Cached pad/via template data.

Same parameter structure as PadViaLibrary.

### 3.19 PadViaLibraryLinks

**Format**: Standard parameter block
**Content**: Links from primitives to pad/via library templates.

No records observed in the test files.

### 3.20 SignalClasses

**Format**: Standard parameter block
**Content**: Signal class definitions.

Uses the same parameter structure as Classes6, with KIND=10 for xSignal classes.

### 3.21 PinPairsSection

**Format**: Standard parameter block
**Content**: Pin pair definitions for swap operations.

No records observed in the test files.

### 3.22 UnionRelations

**Format**: Standard parameter block
**Content**: Union relation mappings between primitives and their unions.

No records observed in the test files (empty Data stream).

### 3.23 WaivedViolations

**Format**: Standard parameter block
**Content**: DRC violations that have been manually waived.

No records observed in the test files.

---

## 4. Options Sections

These sections each contain a single parameter block record with a `RECORD` key
identifying the options type.

### 4.1 Advanced Placer Options6

| Key | Type | Description |
|-----|------|-------------|
| `RECORD` | string | Always `AdvancedPlacerOptions` |
| `PLACELARGECLEAR` | coord | Large component clearance |
| `PLACESMALLCLEAR` | coord | Small component clearance |
| `PLACEUSEROTATION` | bool | Allow rotation during placement |
| `PLACEUSELAYERSWAP` | bool | Allow layer swap during placement |
| `PLACEBYPASSNET1` | string | First bypass net name |
| `PLACEBYPASSNET2` | string | Second bypass net name |
| `PLACEUSEADVANCEDPLACE` | bool | Use advanced placement algorithm |
| `PLACEUSEGROUPING` | bool | Use component grouping |

### 4.2 Advanced Router Options6

Single parameter block record for auto-router settings.

### 4.3 Design Rule Checker Options6

| Key | Type | Description |
|-----|------|-------------|
| `RECORD` | string | Always `DesignRuleCheckerOptions` |
| `DOMAKEDRCFILE` | bool | Generate DRC report file |
| `DOMAKEDRCERRORLIST` | bool | Generate error list |
| `DOSUBNETDETAILS` | bool | Include subnet details |
| `REPORTFILENAME` | string | DRC report file path |
| `EXTERNALNETLISTFILENAME` | string | External netlist file path |
| `CHECKEXTERNALNETLIST` | bool | Check against external netlist |
| `MAXVIOLATIONCOUNT` | int | Maximum violations to report |
| `REPORTDRILLEDSMTPADS` | bool | Report drilled SMT pads |
| `REPORTINVALIDMULTILAYERPADS` | bool | Report invalid multilayer pads |
| `RULESSETTOCHECK` | string | Comma-separated rule set indices |
| `ONLINERULESETTOENFORCE` | string | Online rule enforcement set |

### 4.4 Pin Swap Options6

| Key | Type | Description |
|-----|------|-------------|
| `RECORD` | string | Always `PinSwapOptions` |
| `QUIET` | bool | Suppress prompts |
| `APPROXIMATEPINPOSITIONS` | bool | Use approximate pin positions |
| `ALLOWPARTIALLYROUTEDCONNECTIONS` | bool | Allow partial routes |
| `VIAPENALTYSTATE` | bool | Enable via penalty |
| `CROSSOVERRATIO` | int | Crossover ratio percentage |
| `VIAPENALTYVALUE` | int | Via penalty value |
| `IGNORENETS` | string | Comma-separated nets to ignore |
| `IGNORENETCLASSES` | string | Comma-separated net classes to ignore |
| `IGNORECOMPONENTS` | string | Components to ignore |
| `IGNOREDIFFERENTIALPAIRS` | string | Differential pairs to ignore |
| `HEURISTICNAME` | string | Heuristic algorithm name |
| `HEURISTICONOFFSTATE` | string | Heuristic on/off state |
| `HEURISTICWEIGHTVALUE` | string | Heuristic weight value |

---

## 5. Value Types Reference

Parameter values use these type conventions:

| Type | Format | Example |
|------|--------|---------|
| `bool` | `TRUE` or `FALSE` | `LOCKED=FALSE` |
| `int` | Decimal integer | `PRIORITY=2` |
| `float` | Delphi scientific notation | `ROTATION= 1.80000000000000E+0002` |
| `coord` | Decimal with unit suffix | `X=4686.0219mil` |
| `string` | Plain text (Win1252) | `NAME=GND` |
| `color` | Win32 COLORREF decimal | `COLOR=7709086` |

Coordinates use `mil` suffix (1 mil = 0.001 inch). Internal storage is in units of
10,000 per mil (Altium Coord type). The string representation includes the unit suffix.

Float values use Delphi's 20-digit scientific notation with explicit sign and 4-digit
exponent (e.g., ` 1.80000000000000E+0002` = 180.0). Note the leading space for
positive values.

---

## 6. Source Files

| File | Purpose |
|------|---------|
| `AD26-dotnet/Altium.SDK.Interfaces/PCB/TRuleKind.cs` | Rule kind enum (70+ values) |
| `AD26-dotnet/Altium.SDK.Interfaces/PCB/TDimensionKind.cs` | Dimension kind enum |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_PolygonsBinarySection.cs` | Polygon section interface |
| `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/IPCB_DimensionsSection.cs` | Dimension section interface |
| `AD26-dotnet/Altium.Edp.Interfaces/PCBInterfaces/IPCB_Net_SaveLoadParameters.cs` | Net save/load |
| `AD26-dotnet/Altium.Edp.Interfaces/PCBInterfaces/IPCB_Component_SaveLoadParameters.cs` | Component save/load |
| `AD26-dotnet/Altium.Edp.Interfaces/PCBInterfaces/IPCB_DifferentialPair_SaveLoadParameters.cs` | Diff pair save/load |
| `AD26-dotnet/Altium.Edp.Interfaces/PCBInterfaces/IPCB_EmbeddedBoard_SaveLoadParameters.cs` | Embedded board save/load |
| `AD26-dotnet/Altium.Edp.Interfaces/PCBInterfaces/IPCB_Dimension_SaveLoadParameters.cs` | Dimension save/load |
| `AD26-dotnet/Altium.Edp.Interfaces/PCBInterfaces/IPCB_Coordinate_SaveLoadParameters.cs` | Coordinate save/load |
| `AD26-dotnet/Altium.Edp.Interfaces/PCBInterfaces/IPCB_Primitive_SaveLoadParameters.cs` | Base primitive save/load |
