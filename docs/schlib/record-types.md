# Record Types

Field definitions for all parameter text records (flags=0x00 blocks) found in SchLib
`Data` streams. Record type is identified by the `RECORD` key.

## Base compositions

All records build on shared base types via composition.

### SchPrimitiveBase (common to all primitives)

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `OWNERINDEX` | i32 | -1 | Parent record index (relative within component section) |
| `ISNOTACCESIBLE` | bool | F | Inverse accessibility flag |
| `OWNERPARTID` | i32 | -1 | Multi-part symbol part number (1-based; 0 = common to all parts) |
| `OWNERPARTDISPLAYMODE` | i32 | 0 | Display mode (0 = common to all modes) |
| `GRAPHICALLYLOCKED` | bool | F | Whether the primitive is locked |
| `INDEXINSHEET` | i32 | -1 | Position index within parent container |

### SchGraphicalBase (extends SchPrimitiveBase)

Adds position and color to `SchPrimitiveBase`.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (all SchPrimitiveBase fields) | | | |
| `LOCATION.X` + `LOCATION.X_FRAC` | i32 | 0 | X position (DXP fractional pair) |
| `LOCATION.Y` + `LOCATION.Y_FRAC` | i32 | 0 | Y position (DXP fractional pair) |
| `COLOR` | i32 | 0 | Foreground color (COLORREF) |
| `AREACOLOR` | i32 | 0 | Fill/area color (COLORREF) |

For DXP fractional encoding see [coordinate-system.md](coordinate-system.md). The `_FRAC`
key is omitted when its value is zero.

## RECORD=1: SchComponent

The root container record. Always the first block in a component's `Data` stream. Every
component's section has exactly one `SchComponent`.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (all SchGraphicalBase fields) | | | |
| `LIBREFERENCE` | string | | Component name in the library |
| `COMPONENTDESCRIPTION` | string | | Human-readable description |
| `UNIQUEID` | string | | 8-character unique identifier |
| `CURRENTPARTID` | i32 | 1 | Currently active part number |
| `PARTCOUNT` | i32 | 2 | Total parts; stored as (actual_part_count + 1) in file |
| `DISPLAYMODECOUNT` | i32 | 1 | Number of display modes |
| `DISPLAYMODE` | i32 | 0 | Current display mode index |
| `SHOWHIDDENPINS` | bool | F | Show hidden pins |
| `LIBRARYPATH` | string | * | Library path (default literal `*`) |
| `SOURCELIBRARYNAME` | string | * | Source library name (default literal `*`) |
| `SHEETPARTFILENAME` | string | * | Sheet part filename (default literal `*`) |
| `TARGETFILENAME` | string | * | Target filename (default literal `*`) |
| `OVERRIDECOLORS` | bool | F | Override primitive colors |
| `DESIGNATORLOCKED` | bool | F | Lock designator text |
| `PARTIDLOCKED` | bool | F | Lock part ID |
| `COMPONENTKIND` | i32 | 0 | Component kind (see `ComponentKind` in enumerations.md) |
| `ALIASLIST` | string | | Pipe-separated list of aliases |
| `ORIENTATION` | i32 | 0 | Bitmask: bit 0 = ROTATED, bit 1 = FLIPPED |

## RECORD=34: SchDesignator

The reference designator text (e.g., "U?", "R?", "C?") shown on the schematic.
Every component has exactly one `SchDesignator`.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (all SchGraphicalBase fields) | | | |
| `TEXT` | string | * | Designator text (default literal `*`) |
| `NAME` | string | Designator | Parameter name |
| `READONLYSTATE` | i32 | 1 | Read-only flag |
| `FONTID` | i32 | 1 | Font ID (file-local, 1-based index into font table) |
| `UNIQUEID` | string | | 8-character unique identifier |

## RECORD=41: SchParameter

A named parameter (e.g., "Comment", "Value", "Footprint"). Each component typically
has at least one `SchParameter` for the comment/value.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (all SchGraphicalBase fields) | | | |
| `TEXT` | string | * | Parameter value (default literal `*`) |
| `NAME` | string | Comment | Parameter name |
| `FONTID` | i32 | 1 | Font ID (file-local, 1-based) |
| `UNIQUEID` | string | | 8-character unique identifier |
| `READONLYSTATE` | i32 | | Read-only flag |
| `ISHIDDEN` | bool | F | Hidden flag |

## RECORD=14: SchRectangle

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (all SchGraphicalBase fields) | | | |
| `CORNER.X` + `CORNER.X_FRAC` | i32 | | Top-right corner X (DXP fractional pair) |
| `CORNER.Y` + `CORNER.Y_FRAC` | i32 | | Top-right corner Y (DXP fractional pair) |
| `ISSOLID` | bool | F | Solid fill |
| `LINEWIDTH` | i32 | 1 | Line width (see `LineWidth` in enumerations.md) |
| `TRANSPARENT` | bool | F | Transparent fill |

## RECORD=13: SchLine

Single line segment from `LOCATION` to `CORNER`.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (all SchGraphicalBase fields) | | | |
| `CORNER.X` + `CORNER.X_FRAC` | i32 | | End point X (DXP fractional pair) |
| `CORNER.Y` + `CORNER.Y_FRAC` | i32 | | End point Y (DXP fractional pair) |
| `LINEWIDTH` | i32 | 1 | Line width |
| `LINESTYLE` | i32 | 0 | Line style (see `LineStyle` in enumerations.md) |

## RECORD=6: SchPolyline

Multi-segment polyline with N vertices. Vertex coordinates are 1-based indexed.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (all SchGraphicalBase fields) | | | |
| `LINEWIDTH` | i32 | 1 | Line width |
| `LINESTYLE` | i32 | 0 | Line style |
| `LOCATIONCOUNT` | i32 | | Number of vertices |
| `X{N}` + `X{N}_FRAC` | i32 | | Vertex N X coordinate (1-based) |
| `Y{N}` + `Y{N}_FRAC` | i32 | | Vertex N Y coordinate (1-based) |

## RECORD=7: SchPolygon

Closed filled polygon. Same structure as `SchPolyline` plus:

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (all SchPolyline fields) | | | |
| `ISSOLID` | bool | F | Solid fill |

## RECORD=12: SchArc

Circular arc. `LOCATION` is the center point. Angles are in degrees.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (all SchGraphicalBase fields) | | | |
| `RADIUS` + `RADIUS_FRAC` | i32 | | Arc radius (DXP fractional pair) |
| `STARTANGLE` | f64 | 0.0 | Start angle in degrees |
| `ENDANGLE` | f64 | 360.0 | End angle in degrees (360 = full circle) |
| `LINEWIDTH` | i32 | 1 | Line width |

## RECORD=11: SchEllipticalArc

Elliptical arc. Same as `SchArc` but with a second radius for the minor axis.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (all SchArc fields) | | | |
| `SECONDARYRADIUS` + `SECONDARYRADIUS_FRAC` | i32 | | Secondary (minor) radius |

## RECORD=8: SchEllipse

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (all SchGraphicalBase fields) | | | |
| `RADIUS` + `RADIUS_FRAC` | i32 | | Primary (major) radius |
| `SECONDARYRADIUS` + `SECONDARYRADIUS_FRAC` | i32 | | Secondary (minor) radius |
| `ISSOLID` | bool | F | Solid fill |
| `LINEWIDTH` | i32 | 1 | Line width |

## RECORD=5: SchBezier

Cubic bezier curve. Always has exactly 4 control points. Same indexed coordinate
structure as `SchPolyline` with `LOCATIONCOUNT=4`.

## RECORD=9: SchPie

Pie/wedge shape (filled arc sector). Same as `SchArc` plus:

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (all SchArc fields) | | | |
| `ISSOLID` | bool | F | Solid fill |

## RECORD=4: SchLabel

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (all SchGraphicalBase fields) | | | |
| `TEXT` | string | | Label text content |
| `ORIENTATION` | i32 | 0 | Text orientation bitmask (bit 0 = ROTATED 90deg, bit 1 = FLIPPED) |
| `JUSTIFICATION` | i32 | 0 | Text justification (see `TextJustification` in enumerations.md) |
| `FONTID` | i32 | 1 | Font ID (file-local, 1-based) |
| `ISMIRRORED` | bool | F | Mirror flag |
| `ISHIDDEN` | bool | F | Hidden flag |

## RECORD=28: SchTextFrame

Rich text box with optional border and background.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (all SchGraphicalBase fields) | | | |
| `CORNER.X` | i32 | | Bottom-right corner X |
| `CORNER.Y` | i32 | | Bottom-right corner Y |
| `TEXT` | string | | Frame text content |
| `FONTID` | i32 | 1 | Font ID |
| `ALIGNMENT` | i32 | 0 | Text alignment |
| `WORDWRAP` | bool | F | Word wrap enabled |
| `SHOWBORDER` | bool | T | Show frame border |
| `ISSOLID` | bool | F | Solid fill background |
| `CLIPTORECT` | bool | F | Clip text to rectangle |

## RECORD=30: SchImage

Embedded image. The `FILENAME` key matches the name of an embedded object in the
`/Storage` stream.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (all SchGraphicalBase fields) | | | |
| `CORNER.X` | i32 | | Bottom-right corner X |
| `CORNER.Y` | i32 | | Bottom-right corner Y |
| `FILENAME` | string | | Image filename (matches embedded object name in Storage) |
| `EMBEDIMAGE` | bool | T | Whether image is embedded (always T in SchLib) |
| `KEEPASPECT` | bool | T | Maintain aspect ratio |

## RECORD=44: SchImplementationList

Container record for footprint implementations. No fields beyond `SchPrimitiveBase`.

## RECORD=45: SchImplementation

Single footprint assignment. `OWNERINDEX` points to a `SchImplementationList`.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `OWNERINDEX` | i32 | | Index of parent SchImplementationList |
| `MODELNAME` | string | | Footprint name (e.g. "0402", "SOIC-8") |
| `MODELTYPE` | string | PCBLIB | Model type string |
| `DATAFILECOUNT` | i32 | | Number of data file entries |
| `MODELDATAFILEENTITY0` | string | | Data file entity |
| `MODELDATAFILEKIND0` | string | | Data file kind |
| `ISCURRENT` | bool | F | Whether this is the active implementation |
| `DATALINKSLOCKED` | bool | F | Data links locked |
| `DATABASEDATALINKSLOCKED` | bool | F | Database data links locked |
| `INTEGRATEDMODEL` | bool | F | Integrated model flag |
| `DATABASEMODEL` | bool | F | Database model flag |

## RECORD=46: SchImplementationMap

Container for pin-to-pad mappings. `OWNERINDEX` points to a `SchImplementation`.
No additional fields beyond `SchPrimitiveBase`.

## RECORD=47: SchMapDefiner

A single pin-to-pad mapping entry. `OWNERINDEX` points to a `SchImplementation`.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `OWNERINDEX` | i32 | | Index of parent SchImplementation |
| `PINNAME` | string | | Schematic pin name |
| `PADNAME` | string | | PCB pad name |

## RECORD=48: SchImplementationParameters

Container for footprint parameters. `OWNERINDEX` points to a `SchImplementation`.
No additional fields beyond `SchPrimitiveBase`.
