# Board6 Section

The `Board6` section stores the board-level object (`eBoardObject`, TObjectId=25): file
metadata, grid settings, board outline geometry, layer stack definitions, view
configuration, and editor state. It is the first section processed during load and
contains a single parameter block record.

## Stream layout

```
/Board6/
  Header    4 bytes: u32 LE record count (always 1)
  Data      variable: single text-mode parameter block
```

### Header

A single `u32` little-endian integer, always `0x01000000` (value 1). The Board6 section
always contains exactly one record -- the board object itself.

### Data

A single text-mode block (flag byte = 0x00) containing a pipe-delimited parameter string.
Block format:

```
[4 bytes]  Block header: flags(8b) | size(24b), flag=0x00 (text mode)
[N bytes]  NUL-terminated pipe-delimited parameter string: |KEY1=VALUE1|KEY2=VALUE2|...\0
```

The 24-bit size field gives the byte count of the payload (the parameter string including
the NUL terminator). Observed sizes: ~100KB for a real-world 8-layer board
(LimeSDR_Mini_1v3).

**Note:** The parameter string appears twice in the parsed structure -- first a set of
"common primitive" parameters (SELECTION, LAYER, LOCKED, etc.), then a second set
beginning at the board outline definition (which repeats SELECTION, LAYER, LOCKED etc.).
The Board6 block uses the standard pipe-delimited format but with the backtick (`` ` ``)
character as an escape delimiter inside embedded configuration strings (2DCONFIGURATION,
3DCONFIGURATION).

## Parameter categories

The ~2700 unique parameter keys observed in a real PcbDoc file organize into the following
categories.

### 1. Common primitive parameters

These appear at the start of the block and mirror the common prefix found on all PCB
primitives:

| Key | Type | Example | Description |
|-----|------|---------|-------------|
| `SELECTION` | bool | `FALSE` | Object is selected |
| `LAYER` | string | `UNKNOWN` / `TOP` | Layer assignment (UNKNOWN for board root) |
| `LOCKED` | bool | `FALSE` | Object is locked |
| `POLYGONOUTLINE` | bool | `FALSE` | Is polygon outline |
| `USERROUTED` | bool | `TRUE` | User-routed flag |
| `KEEPOUT` | bool | `FALSE` | Is keepout |
| `UNIONINDEX` | i32 | `0` | Union index |

### 2. File identification

| Key | Type | Example | Description |
|-----|------|---------|-------------|
| `FILENAME` | string | `d:\...\LimeSDR_Mini_1v3_Rounded.$$$` | Source file path |
| `KIND` | string | `Protel_Advanced_PCB` | File type identifier |
| `VERSION` | string | `5,01` | Board format version (comma-separated major,minor) |
| `DATE` | string | `2019-06-04` | Last save date (YYYY-MM-DD) |
| `TIME` | string | `17:36:32` | Last save time (HH:MM:SS) |
| `RECORD` | string | `Board` | Record type identifier (always "Board") |
| `UNIQUEID` | string | (8-char hex) | Board-level unique identifier |
| `SHELVED` | bool | `FALSE` | Board is shelved |
| `NAME` | string | | Board name |

The `VERSION` field is a floating-point value encoded with a comma as decimal separator
(e.g., `5,01` = 5.01). This is passed to `SetState_BoardVersion()` during load.

### 3. Origin and display units

| Key | Type | Example | Description |
|-----|------|---------|-------------|
| `ORIGINX` | coord | `3937.0079mil` | Board origin X (mil string with unit suffix) |
| `ORIGINY` | coord | `3937.0079mil` | Board origin Y |
| `DISPLAYUNIT` | i32 | `0` | Display unit: 0=mils, 1=mm |
| `DESIGNATORDISPLAYMODE` | i32 | `0` | Designator display mode |

### 4. Grid settings (legacy)

Flat grid parameters from older format versions:

| Key | Type | Example | Description |
|-----|------|---------|-------------|
| `BIGVISIBLEGRIDSIZE` | float | `10000000.000` | Large visible grid size (internal units) |
| `BIGVISIBLEGRIDMULTFACTOR` | float | | Large visible grid multiplier |
| `VISIBLEGRIDSIZE` | float | `1000000.000` | Visible grid size |
| `VISIBLEGRIDMULTFACTOR` | float | | Visible grid multiplier |
| `SNAPGRIDSIZE` | float | `9842.519685` | Snap grid size |
| `SNAPGRIDSIZEX` | float | `9842.519685` | Snap grid X (may differ from Y) |
| `SNAPGRIDSIZEY` | float | `9842.519685` | Snap grid Y |
| `TRACKGRIDSIZE` | float | `200000.000000` | Track routing grid size |
| `VIAGRIDSIZE` | float | `200000.000000` | Via placement grid size |
| `COMPONENTGRIDSIZE` | float | `9842.519685` | Component placement grid size |
| `COMPONENTGRIDSIZEX` | float | `9842.519685` | Component grid X |
| `COMPONENTGRIDSIZEY` | float | `9842.519685` | Component grid Y |
| `DOTGRID` | bool | `TRUE` | Show dot grid |
| `ELECTRICALGRIDRANGE` | coord | `8mil` | Electrical grid snap range |
| `ELECTRICALGRIDENABLED` | bool | `TRUE` | Electrical grid enabled |
| `ELECTRICALGRIDMULTFACT` | float | `0.000` | Electrical grid multiplier |
| `ELECTRICALGRIDSNAPTOBO` | bool | `FALSE` | Snap to board outline |
| `ELECTRICALGRIDUSEALLLAYERS` | bool | `FALSE` | Use all layers for e-grid |
| `GRIDSNAPENABLED` | bool | | Grid snap enabled |
| `OGSNAPENABLED` | bool | | Object grid snap enabled |
| `MGSNAPENABLED` | bool | | Manual grid snap enabled |

### 5. Named grid objects (GR{N}_*)

Modern Altium uses named grid objects. Each grid has an index `N` (0-based):

| Suffix | Type | Example | Description |
|--------|------|---------|-------------|
| `TYPE` | string | `CartesianGrid` | Grid type |
| `NAME` | string | `Global Board Snap Grid` | Grid display name |
| `COLOR` | i32 | `6049101` | Grid color (COLORREF) |
| `COLORLGE` | i32 | `9473425` | Large grid color |
| `PRIO` | i32 | `50` | Priority |
| `OX` | coord | `3937.0079mil` | Grid origin X |
| `OY` | coord | `3937.0079mil` | Grid origin Y |
| `DRAWMODE` | i32 | `1` | Draw mode |
| `DRAWMODELARGE` | i32 | `0` | Large grid draw mode |
| `ENABLED` | bool | `TRUE` | Grid enabled |
| `MULT` | i32 | `1` | Multiplier |
| `MULTLARGE` | i32 | `5` | Large multiplier |
| `DISPLAYUNIT` | i32 | `0` | Display unit |
| `COMP` | bool | `TRUE` | Component grid |
| `GSX` | float | `9842.519685` | Grid step X |
| `GSY` | float | `9842.519685` | Grid step Y |
| `QSX` | coord | `99999mil` | Quad step X |
| `QSY` | coord | `99999mil` | Quad step Y |
| `ROT` | float | `0.000000` | Rotation angle |
| `FLAGS` | i32 | `15` | Grid flags bitmask |

### 6. Guide lines (GU{N}_*)

Guide lines for board alignment. Each guide has an index `N` (0-based):

| Suffix | Type | Example | Description |
|--------|------|---------|-------------|
| `TYPE` | string | `Line` | Guide type |
| `COLOR` | i32 | `8776954` | Guide color (COLORREF) |
| `X1` | coord | `-99999mil` | Start X |
| `Y1` | coord | `3937.0079mil` | Start Y |
| `X2` | coord | `99999mil` | End X |
| `Y2` | coord | `3937.0079mil` | End Y |
| `OX` | coord | `1000mil` | Origin X |
| `OY` | coord | `3937.0079mil` | Origin Y |
| `ENABLED` | bool | `TRUE` | Guide enabled |

### 7. Board outline geometry

The board outline is encoded as a sequence of vertices and arcs, using indexed
parameters `KIND{N}`, `VX{N}`, `VY{N}`, `CX{N}`, `CY{N}`, `SA{N}`, `EA{N}`, `R{N}`.

| Key | Type | Description |
|-----|------|-------------|
| `KIND{N}` | i32 | Vertex kind: 0=line segment, 1=arc segment |
| `VX{N}` | coord | Vertex X position |
| `VY{N}` | coord | Vertex Y position |
| `CX{N}` | coord | Arc center X (0 for line segments) |
| `CY{N}` | coord | Arc center Y (0 for line segments) |
| `SA{N}` | float | Arc start angle (Delphi Extended format) |
| `EA{N}` | float | Arc end angle (Delphi Extended format) |
| `R{N}` | coord | Arc radius (0 for line segments) |

The outline is a closed polygon. Line segments (`KIND=0`) connect the current vertex to
the next. Arc segments (`KIND=1`) define a curved transition using the center point
(CX,CY), radius (R), and angular sweep from SA to EA.

Example (rounded rectangle board outline):
```
KIND0=0  VX0=3937.0079mil VY0=4015.748mil     # straight segment
KIND1=0  VX1=3937.0079mil VY1=5093.3072mil     # straight segment
KIND2=1  VX2=3937.0078mil VY2=5093.3071mil ... R2=78.7402mil  # arc (corner rounding)
KIND3=0  VX3=4015.748mil  VY3=5172.0473mil     # straight segment
...
```

Additional outline parameters:

| Key | Type | Example | Description |
|-----|------|---------|-------------|
| `PRIMITIVELOCK` | bool | `TRUE` | Outline primitives locked |
| `POLYGONTYPE` | string | `Polygon` | Polygon type |
| `POUROVER` | bool | `FALSE` | Pour over outline |
| `POUROVERSTYLE` | string | | Pour over style |
| `REMOVEISLANDSBYAREA` | bool | | Remove copper islands by area |
| `REMOVENECKS` | bool | | Remove thin necks |
| `REMOVEDEAD` | bool | `FALSE` | Remove dead copper |
| `GRIDSIZE` | coord | `10mil` | Polygon grid size |
| `TRACKWIDTH` | coord | `10mil` | Pour track width |
| `HATCHSTYLE` | string | `None` | Hatch pattern style |
| `USEOCTAGONS` | bool | `FALSE` | Use octagonal pads in pour |
| `MINPRIMLENGTH` | coord | `3mil` | Minimum primitive length |
| `POURINDEX` | i32 | `-1` | Pour index (-1 = none) |
| `SPLITLINECOUNT` | i32 | `0` | Number of split plane lines |
| `OUTLINEMODELCRC` | i32 | `0` | CRC of 3D model used for outline |
| `OUTLINEMODELNAME` | string | | Name of 3D model used for outline |

### 8. Legacy layer stack (LAYER{N}*)

Pre-V7 layer stack definition. Layers are 1-indexed (LAYER1 through LAYER82):

| Suffix | Type | Example | Description |
|--------|------|---------|-------------|
| `NAME` | string | `Top` | Layer name |
| `PREV` | i32 | `0` | Previous layer index (0=none) |
| `NEXT` | i32 | `2` | Next layer index |
| `MECHENABLED` | bool | `FALSE` | Mechanical layer enabled |
| `COPTHICK` | coord | `0.7mil` | Copper thickness |
| `DIELTYPE` | i32 | `1` | Dielectric type |
| `DIELCONST` | float | `4.800` | Dielectric constant (Er) |
| `DIELHEIGHT` | coord | `6.6929mil` | Dielectric height |
| `DIELMATERIAL` | string | `FR-4` | Dielectric material name |

Layers 1-82 are always written (even if unused), representing the full 82-layer
fixed-size legacy layer table.

Additional layer parameters:

| Key | Type | Example | Description |
|-----|------|---------|-------------|
| `LAYERCOUNT` | i32 | | Number of signal layers |
| `LAYERSTACKSTYLE` | i32 | `0` | Stack style |
| `TOPTYPE` | i32 | `3` | Top surface type |
| `TOPCONST` | float | `3.500` | Top surface dielectric constant |
| `TOPHEIGHT` | coord | `0.4mil` | Top surface height |
| `TOPMATERIAL` | string | `Solder Resist` | Top surface material |
| `BOTTOMTYPE` | i32 | `3` | Bottom surface type |
| `BOTTOMCONST` | float | `3.500` | Bottom surface dielectric constant |
| `BOTTOMHEIGHT` | coord | `0.4mil` | Bottom surface height |
| `BOTTOMMATERIAL` | string | `Solder Resist` | Bottom surface material |
| `SHOWTOPDIELECTRIC` | bool | `FALSE` | Show top dielectric in stack |
| `SHOWBOTTOMDIELECTRIC` | bool | `FALSE` | Show bottom dielectric |
| `SHOWSIGNALLAYERSONLY` | bool | | Show signal layers only |

### 9. V7 layer stack (LAYERV7_{N}*)

V7 format layer definitions, 0-indexed. Adds a `LAYERID` field (integer layer identifier):

| Suffix | Type | Example | Description |
|--------|------|---------|-------------|
| `LAYERID` | i32 | `16908305` | Layer ID (TV7_Layer integer value) |
| `NAME` | string | `Mechanical 17` | Layer name |
| `PREV` | i32 | | Previous layer index |
| `NEXT` | i32 | | Next layer index |
| `MECHENABLED` | bool | | Mechanical layer enabled |
| `COPTHICK` | coord | | Copper thickness |
| `DIELTYPE` | i32 | | Dielectric type |
| `DIELCONST` | float | | Dielectric constant |
| `DIELHEIGHT` | coord | | Dielectric height |
| `DIELMATERIAL` | string | | Dielectric material |

### 10. V8 layer stack (LAYERMASTERSTACK_V8*, LAYERSUBSTACK_V8_{N}*, LAYER_V8_{N}*)

The V8 layer stack introduces GUIDs and substacks for rigid-flex support.

#### Master stack (LAYERMASTERSTACK_V8*)

| Suffix | Type | Example | Description |
|--------|------|---------|-------------|
| `STYLE` | i32 | `0` | Stack style |
| `ID` | GUID | `{414905F5-...}` | Master stack GUID |
| `NAME` | string | `Master layer stack` | Display name |
| `SHOWTOPDIELECTRIC` | bool | `FALSE` | Show top dielectric |
| `SHOWBOTTOMDIELECTRIC` | bool | `FALSE` | Show bottom dielectric |
| `ISFLEX` | bool | `FALSE` | Is flex stack |

#### Substacks (LAYERSUBSTACK_V8_{N}*)

| Suffix | Type | Example | Description |
|--------|------|---------|-------------|
| `ID` | GUID | `{C648DDCB-...}` | Substack GUID |
| `NAME` | string | `Board Layer Stack` | Substack name |
| `SHOWTOPDIELECTRIC` | bool | `FALSE` | Show top dielectric |
| `SHOWBOTTOMDIELECTRIC` | bool | `FALSE` | Show bottom dielectric |
| `ISFLEX` | bool | `FALSE` | Is flex substack |
| `SERVICE` | bool | `FALSE` | Is service substack |
| `USEDBYPRIMS` | bool | `FALSE` | Referenced by primitives |
| `TYPE` | i32 | `1` | Substack type |

#### Layers (LAYER_V8_{N}*)

Each layer entry has:

| Suffix | Type | Example | Description |
|--------|------|---------|-------------|
| `ID` | GUID | `{F0B75EAA-...}` | Layer GUID |
| `NAME` | string | `TopPaste` | Layer name |
| `LAYERID` | i32 | `16973832` | Integer layer ID (TV7_Layer) |
| `USEDBYPRIMS` | bool | `TRUE` | Layer has primitives |
| `MECHENABLED` | bool | | Mechanical layer enabled |
| `COPTHICK` | coord | | Copper thickness |
| `COMPONENTPLACEMENT` | i32 | | Component placement side |
| `DIELTYPE` | i32 | | Dielectric type |
| `DIELCONST` | float | | Dielectric constant |
| `DIELHEIGHT` | coord | | Dielectric height |
| `DIELMATERIAL` | string | | Dielectric material |
| `COVERLAY_EXPANSION` | coord | | Coverlay expansion |

Layers also carry per-substack context parameters using the substack GUID:
```
LAYER_V8_{N}_{substack-GUID}CONTEXT=0
LAYER_V8_{N}_{substack-GUID}USEDBYPRIMS=FALSE
```

### 11. V9 layer stack (V9_MASTERSTACK_*, V9_SUBSTACK{N}_*, V9_STACK_LAYER{N}_*, V9_CACHE_LAYER{N}_*)

The latest layer stack format. Structure mirrors V8 but uses `V9_` prefix.

#### V9 Master stack

| Key | Type | Example | Description |
|-----|------|---------|-------------|
| `V9_MASTERSTACK_STYLE` | i32 | `0` | Stack style |
| `V9_MASTERSTACK_ID` | GUID | `{414905F5-...}` | Master stack GUID |
| `V9_MASTERSTACK_NAME` | string | `Master layer stack` | Display name |
| `V9_MASTERSTACK_SHOWTOPDIELECTRIC` | bool | `FALSE` | Show top dielectric |
| `V9_MASTERSTACK_SHOWBOTTOMDIELECTRIC` | bool | `FALSE` | Show bottom dielectric |
| `V9_MASTERSTACK_ISFLEX` | bool | `FALSE` | Is flex stack |

#### V9 Substacks (V9_SUBSTACK{N}_*)

Same structure as V8 substacks but with `V9_SUBSTACK{N}_` prefix.

#### V9 Stack layers (V9_STACK_LAYER{N}_*)

Per-layer properties within the active stack. Same fields as LAYER_V8 but with
`V9_STACK_LAYER{N}_` prefix.

#### V9 Cache layers (V9_CACHE_LAYER{N}_*)

A flattened cache of the resolved layer stack. Same fields as stack layers but with
`V9_CACHE_LAYER{N}_` prefix. Includes additional fields:

| Suffix | Type | Description |
|--------|------|-------------|
| `PULLBACKDISTANCE` | coord | Pullback distance for solder mask/paste layers |

The cache layer count can exceed 100 entries (observed: 0-111 in test files), covering
all signal, dielectric, coverlay, solder mask, paste, and mechanical layers.

### 12. Internal plane net assignments

| Key | Type | Example | Description |
|-----|------|---------|-------------|
| `PLANE{N}NETNAME` | string | `(No Net)` | Net assigned to internal plane N (1-16) |

### 13. Layer pairs and mechanical pairs

| Key | Type | Example | Description |
|-----|------|---------|-------------|
| `LAYERPAIR{N}LOW` | string | `TOP` | Low (top) layer of drill pair |
| `LAYERPAIR{N}HIGH` | string | `BOTTOM` | High (bottom) layer of drill pair |
| `LAYERPAIR{N}DRILLGUIDE` | bool | `FALSE` | Show drill guide |
| `LAYERPAIR{N}DRILLDRAWING` | bool | `FALSE` | Show drill drawing |
| `LAYERPAIR{N}SUBSTACK_0` | GUID | `{C648DDCB-...}` | Associated substack |
| `MECHPAIR{N}L1` | string | `MECHANICAL15` | Mechanical pair layer 1 |
| `MECHPAIR{N}L2` | string | `MECHANICAL14` | Mechanical pair layer 2 |

### 14. Layer sets (LAYERSET{N}*)

Saved layer visibility presets. `LAYERSETSCOUNT` gives the total.

| Key | Type | Example | Description |
|-----|------|---------|-------------|
| `LAYERSETSCOUNT` | i32 | `15` | Number of layer sets |
| `LAYERSET{N}NAME` | string | `&All Layers` | Layer set name (`&` prefix = built-in) |
| `LAYERSET{N}LAYERS` | string | `TopLayer,MidLayer1,...` | Comma-separated layer names |
| `LAYERSET{N}ACTIVELAYER.7` | string | `TOP` | Active layer for this set |
| `LAYERSET{N}ISCURRENT` | bool | `FALSE` | Is current active set |
| `LAYERSET{N}ISLOCKED` | bool | `TRUE` | Set is locked (non-editable) |
| `LAYERSET{N}FLIPBOARD` | bool | `FALSE` | Board is flipped in this set |

### 15. Routing directions

Per-layer routing direction preferences:

| Key | Type | Example | Description |
|-----|------|---------|-------------|
| `ROUTINGDIRECTIONTOP LAYER` | string | `Automatic` | Top layer routing direction |
| `ROUTINGDIRECTIONBOTTOM LAYER` | string | `Automatic` | Bottom layer routing direction |
| `ROUTINGDIRECTIONMID LAYER {N}` | string | `Automatic` | Mid layer N routing direction |

**Note:** These keys contain spaces in the layer name portion -- this is intentional and
matches the Altium serialization.

### 16. Manual route last-used values

| Key | Type | Example | Description |
|-----|------|---------|-------------|
| `MRLASTVIASIZE` | coord | `50mil` | Last via outer diameter |
| `MRLASTVIAHOLE` | coord | `28mil` | Last via hole size |
| `LASTTARGETLENGTH` | coord | `99999mil` | Last target length for tuning |
| `TOPLAYER_MRLASTWIDTH` | coord | | Last track width on top layer |
| `BOTTOMLAYER_MRLASTWIDTH` | coord | | Last track width on bottom layer |
| `MIDLAYER{N}_MRLASTWIDTH` | coord | | Last track width on mid layer N |

### 17. Impedance formulas

| Key | Type | Example | Description |
|-----|------|---------|-------------|
| `SURFACEMICROSTRIP_I` | string | `(87/SQRT(...))*LN(...)` | Surface microstrip impedance formula |
| `SURFACEMICROSTRIP_W` | string | `((5.98*...)...)` | Surface microstrip width formula |
| `SYMMETRICSTRIPLINE_I` | string | `(60/SQRT(...))*LN(...)` | Symmetric stripline impedance formula |
| `SYMMETRICSTRIPLINE_W` | string | `((1.9*...)...)` | Symmetric stripline width formula |

### 18. Sheet and print area

| Key | Type | Example | Description |
|-----|------|---------|-------------|
| `SHEETWIDTH` | coord | `16535.4331mil` | Print sheet width |
| `SHEETHEIGHT` | coord | `11692.9134mil` | Print sheet height |
| `SHEETX` | coord | | Sheet X position |
| `SHEETY` | coord | | Sheet Y position |
| `SHOWSHEET` | bool | `FALSE` | Show sheet border |
| `LOCKSHEET` | bool | `TRUE` | Lock sheet position |

### 19. Viewport and 3D view state

| Key | Type | Example | Description |
|-----|------|---------|-------------|
| `VP.LX` | i32 | `37572970` | Viewport left X (internal units) |
| `VP.HX` | i32 | `71603706` | Viewport right X |
| `VP.LY` | i32 | `33145179` | Viewport bottom Y |
| `VP.HY` | i32 | `59307099` | Viewport top Y |
| `VIEWSIZE.X` | i32 | `45220198` | View size X |
| `VIEWSIZE.Y` | i32 | `28542241` | View size Y |
| `LOOKAT.X` | float | `53777428.000000` | 3D look-at X |
| `LOOKAT.Y` | float | `46002244.000000` | 3D look-at Y |
| `LOOKAT.Z` | float | `735809.875000` | 3D look-at Z |
| `EYEROTATION.X` | float | `0.000000` | 3D eye rotation X |
| `EYEROTATION.Y` | float | `-1.000000` | 3D eye rotation Y |
| `EYEROTATION.Z` | float | `0.000000` | 3D eye rotation Z |
| `ZOOMMULT` | float | `0.000035` | Zoom multiplier |
| `CURRENT2D3DVIEWSTATE` | string | `2D` | Current view state: `2D` or `3D` |
| `VIEWPORTSAREVISIBLE` | bool | `TRUE` | Viewports visible |

### 20. 2D/3D view configuration

Embedded configuration strings using backtick (`` ` ``) as a nested delimiter:

| Key | Type | Description |
|-----|------|-------------|
| `2DCONFIGTYPE` | string | Config type identifier (e.g., `.config_2dsimple`) |
| `2DCONFIGURATION` | string | Full 2D config (backtick-delimited sub-parameters) |
| `2DCONFIGFULLFILENAME` | string | Config file path (`(Not Saved)` if embedded) |
| `3DCONFIGTYPE` | string | Config type identifier (e.g., `.config_3d`) |
| `3DCONFIGURATION` | string | Full 3D config (backtick-delimited sub-parameters) |
| `3DCONFIGFULLFILENAME` | string | Config file path |
| `BOARDINSIGHTVIEWCONFIGURATIONNAME` | string | Board insight view config name |

The configuration strings use backtick `` ` `` as a delimiter instead of pipe `|` to
avoid conflicts with the outer parameter format. They contain nested key=value pairs
including layer opacity arrays (using `?` as array separator), color settings, and
display mode flags.

### 21. Polygon pour settings

| Key | Type | Example | Description |
|-----|------|---------|-------------|
| `ARCRESOLUTION` | coord | `0.5mil` | Arc approximation resolution |
| `AREATHRESHOLD` | float | `250000000000.000000` | Area threshold for island removal |
| `NECKWIDTHTHRESHOLD` | coord | `5mil` | Neck width threshold |

### 22. Near/far object visibility

| Key | Type | Example | Description |
|-----|------|---------|-------------|
| `NEAROBJECTSENABLED` | bool | `FALSE` | Near objects highlighting enabled |
| `FAROBJECTSENABLED` | bool | `FALSE` | Far objects highlighting enabled |
| `NEAROBJECTSET` | string | `011111100011...` | Bit mask of near object types |
| `FAROBJECTSET` | string | `001100000000...` | Bit mask of far object types |
| `NEARDISTANCE` | coord | `1000mil` | Near distance threshold |

### 23. Drill symbol and hole shape

| Key | Type | Example | Description |
|-----|------|---------|-------------|
| `DRILLSYMBOLASENUM` | i32 | `0` | Drill symbol as enum flag |
| `DRILLSYMBOLSIZE` | i32 | `200000` | Drill symbol size |
| `HOLESHAPEHASHSIZE` | i32 | `7` | Number of hole shape hash entries |
| `HASHKEY#{N}` | string | `[393700][0][0][1][2147483647 2147483647]` | Hole shape hash key |
| `HASHVALUE#{N}` | i32 | `9` | Hole shape hash value (drill symbol index) |
| `PINPAIRCOUNT` | i32 | | Pin pair count |

### 24. Place markers

10 place marker positions (1-indexed):

| Key | Type | Description |
|-----|------|-------------|
| `PLACEMARKERX{N}` | coord | Place marker N X position |
| `PLACEMARKERY{N}` | coord | Place marker N Y position |

### 25. Selection memory locks

| Key | Type | Example | Description |
|-----|------|---------|-------------|
| `SELECTIONMEMORYLOCK{N}` | bool | `TRUE`/`FALSE` | Lock state for selection memory slot N (1-8) |

### 26. Toggle layers and restore state

| Key | Type | Example | Description |
|-----|------|---------|-------------|
| `TOGGLELAYERS` | string | `1111100101...` | Bit string of toggled layer visibility |
| `RESTORELAYER` | string | `UNKNOWN` | Layer to restore on toggle |
| `RESTORENET` | string | (empty) | Net to restore on toggle |
| `IGNOREVIOLATIONS` | bool | `FALSE` | Ignore DRC violations |

### 27. Snapping configuration

| Key | Type | Description |
|-----|------|-------------|
| `SNAPPINGENTITYSET` | string | Snapping entity set bitmask |
| `POINTGUIDEENABLED` | bool | Point guide enabled |
| `SHOWDEFAULTSETS` | bool | Show default layer sets |
| `EGENABLED` | bool | Electrical grid enabled (compact form) |
| `EGRANGE` | coord | Electrical grid range (compact form) |
| `EGMULT` | float | Electrical grid multiplier (compact form) |
| `EGSNAPTOARCCENTERS` | bool | Snap to arc centers |
| `EGSNAPTOBOARDOUTLINE` | bool | Snap to board outline |
| `EGUSEALLLAYERS` | bool | Use all layers for electrical grid |

### 28. Font/value arrays (FN#, FV#)

| Key | Type | Description |
|-----|------|-------------|
| `FN#{N}` | string | Font name N |
| `FV#{N}` | i32 | Font value N |
| `VALUECOUNT` | i32 | Number of value entries |

## Comparison with PcbLib Library/Data

In PcbLib files, the equivalent section is `/Library/Data` (not `/Board6/Data`). Key
differences:

| Aspect | PcbDoc Board6/Data | PcbLib Library/Data |
|--------|--------------------|--------------------|
| Section path | `/Board6/Data` | `/Library/Data` |
| Block count | 1 block (single board object) | 2 blocks (block 0: library params, block 1: component list) |
| Board outline | Present (vertex arrays) | Not present |
| Layer stack | Full V7/V8/V9 layer stacks | Typically a subset |
| View config | Full 2D/3D configs | Typically present |
| Component list | Not present | Block 1: tab-separated component names |

## Related sections

### BoardRegions (BoardRegions/)

The `BoardRegions` section stores board region definitions for rigid-flex designs.

- Header: `u32` count (typically 1)
- Data: Binary records (not text-mode parameter blocks), each containing:
  - Binary header (object ID, flags, net/layer info)
  - Embedded parameter string (pipe-delimited)
  - Board region-specific fields: `V7_LAYER`, `NAME`, `KIND`, `SUBPOLYINDEX`,
    `ISBOARDCUTOUT`, `ISSHAPEBASED`, `CAVITYHEIGHT`, `OBJECTKIND=BoardRegion`,
    `LAYERSTACKID`, `BENDINGLINECOUNT`, `LOCKED3D`

### EmbeddedBoards6 (EmbeddedBoards6/)

Empty in typical single-board designs. Used for multi-board panel assemblies.

## Loading pipeline interaction

During the PcbDoc loading pipeline (see `docs/dxp/pcb-files.md` section 5), the Board6
section is processed first. The loader:

1. Reads the single parameter block from Board6/Data
2. Calls `SetState_BoardVersion()` with the `VERSION` value
3. Reconstructs the board outline from vertex arrays (`KIND{N}`, `VX{N}`, etc.)
4. Calls `SetState_BoardOutline()` with the reconstructed outline
5. Calls `UpdateLayerStackTables()` to rebuild the layer stack from the V7/V8/V9 data
6. Calls `AssignLayerStackToLayerPairs()` to set up drill pairs
7. Calls `CreateDefaultRules()` to create default DRC rules if missing

Source: `AD26-dotnet/Altium.Edp.Interfaces/PCBInterfaces/IPCB_Board_SaveLoadParameters.cs`

## TStorageFeature interaction

The `TStorageFeature` enum (source: `AD26-dotnet/Altium.Edp.Interfaces/RT_PCB/TStorageFeature.cs`)
defines feature flags that affect how Board6 data is interpreted:

| Flag | Description |
|------|-------------|
| `eHasImpedanceProfileCount` | Impedance profiles are present |
| `eHasPrintedElectronicLayers` | Printed electronic layers are present |
| `eHasMicroVias` | Micro-via support is active |
| `eHasShapeBasedRegions` | Shape-based regions (vs. primitive-based) |
| `eHasShapeBasedCompBodies` | Shape-based component bodies |
| `eHasIPC4761ViaTypesAtWriteStage` | IPC-4761 via type support |
| `eHasIncreasedSignalLayers` | Supports > 32 signal layers |
| `eHasSingleLayerModeAtWriteStage` | Single layer mode data saved |

These flags are stored elsewhere (FileHeader/FileVersionInfo) and affect how the loader
interprets Board6 parameters -- for example, `eHasIncreasedSignalLayers` determines
whether the V9 layer stack with > 32 layers is used.
