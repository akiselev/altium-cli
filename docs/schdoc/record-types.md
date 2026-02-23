# Record Types

Field definitions for all parameter text records found in SchDoc files. Record type is
identified by the `RECORD` key. All records use flags=0x00 (parameter text format).

## Base compositions

All records build on shared base types via composition. These are identical to the SchLib
base types and should be shared.

### SchPrimitiveBase (common to all primitives)

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `OWNERINDEX` | i32 | -1 | Parent record index (0-based absolute in flat list) |
| `ISNOTACCESIBLE` | bool | F | Inverse accessibility flag (note: Altium typo, single 's') |
| `OWNERPARTID` | i32 | -1 | Multi-part symbol part number (1-based; -1 = sheet-level) |
| `OWNERPARTDISPLAYMODE` | i32 | 0 | Display mode (0 = common to all modes) |
| `GRAPHICALLYLOCKED` | bool | F | Whether the primitive is locked |
| `INDEXINSHEET` | i32 | -1 | Sequential index within sheet (SchDoc-specific, not in SchLib) |
| `OWNERINDEXADDITIONALLIST` | bool | F | If T, OWNERINDEX refers to AdditionalWarehouse; if F, refers to BaseWarehouse |
| `IGNOREONLOAD` | bool | F | Skip this record during loading (COND: only if true) |
| `WIRINGDIAGRAMORIGINUNIQUEID` | string | | Wiring diagram origin ID (COND: only for containers, if non-empty) |
| `ISSCHEMATICBLOCKOBJECT` | bool | F | Record belongs to a schematic block |
| `UNIQUEIDINREUSEBLOCK` | string | | UniqueID within a reuse block (COND: only if non-empty) |

### SchGraphicalBase (extends SchPrimitiveBase)

Adds position and color to `SchPrimitiveBase`.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (all SchPrimitiveBase fields) | | | |
| `LOCATION.X` + `LOCATION.X_FRAC` | i32 | 0 | X position (DXP fractional pair) |
| `LOCATION.Y` + `LOCATION.Y_FRAC` | i32 | 0 | Y position (DXP fractional pair) |
| `COLOR` | i32 | 0 | Foreground color (COLORREF) |
| `AREACOLOR` | i32 | 0 | Fill/area color (COLORREF) |
| `SELECTIONMEMORY` | i32 | 0 | Selection state memory |
| `UNIONINDEX` | i32 | 0 | Union group index |

---

## SchDoc-only record types

These record types appear ONLY in SchDoc files, never in SchLib.

### RECORD=31: SchSheet

Sheet properties. Always the first content record (index 0 in warehouse). Every SchDoc
has exactly one.

See [fileheader-stream.md](fileheader-stream.md) for the full field list including font
table. Core fields:

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `RECORD` | i32 | 31 | Always 31 |
| `FontIdCount` | i32 | | Number of fonts in the font table |
| `Size{N}` | i32 | | Font N size in points |
| `FontName{N}` | string | | Font N face name |
| `Rotation{N}` | i32 | 0 | Font N rotation (optional) |
| `Bold{N}` | bool | F | Font N bold (optional) |
| `Italic{N}` | bool | F | Font N italic (optional) |
| `Underline{N}` | bool | F | Font N underline (optional) |
| `StrikeOut{N}` | bool | F | Font N strikethrough (optional) |
| `SheetStyle` | i32 | | Sheet size preset (optional) |
| `SystemFont` | i32 | 1 | System font ID |
| `BorderOn` | bool | T | Show border |
| `SheetNumberSpaceSize` | i32 | 12 | Sheet number space |
| `AreaColor` | i32 | 16317695 | Background color |
| `SnapGridOn` | bool | T | Snap grid enabled |
| `SnapGridSize` + `_Frac` | i32 | | Snap grid size |
| `VisibleGridOn` | bool | T | Visible grid enabled |
| `VisibleGridSize` + `_Frac` | i32 | | Visible grid size |
| `HotSpotGridOn` | bool | T | Hotspot grid enabled |
| `HotSpotGridSize` + `_Frac` | i32 | | Hotspot grid size |
| `CustomX` | i32 | 1000 | Custom sheet width (DXP units) |
| `CustomY` | i32 | 800 | Custom sheet height (DXP units) |
| `ShowTemplateGraphics` | bool | T | Show template graphics |
| `TemplateFileName` | string | | Template file path |
| `Display_Unit` | i32 | 1 | Display unit (0=mils, 1=mm) |
| `UseMBCS` | bool | T | Multi-byte character set |
| `IsBOC` | bool | T | (always T) |

### RECORD=39: SchTemplate

Template file reference. Always the second content record. Exactly one per SchDoc.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `RECORD` | i32 | 39 | Always 39 |
| `ISNOTACCESIBLE` | bool | T | Always T |
| `OWNERPARTID` | i32 | -1 | Always -1 |
| `FILENAME` | string | | Full path to .SchDot template file |

### RECORD=27: SchWire

Electrical wire connection. Uses indexed vertex coordinates like SchPolyline.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (SchPrimitiveBase fields) | | | |
| `LOCATIONCOUNT` | i32 | | Number of vertices (typically 2, observed up to 14) |
| `X{N}` + `X{N}_FRAC` | i32 | | Vertex N X coordinate (1-based) |
| `Y{N}` + `Y{N}_FRAC` | i32 | | Vertex N Y coordinate (1-based) |
| `COLOR` | i32 | 8388608 | Wire color (default: dark red) |
| `LINEWIDTH` | i32 | 1 | Line width |
| `LINESTYLE` | i32 | 0 | Line style (see `LineStyle` in enumerations.md; 0=Solid) |
| `UNIQUEID` | string | | 8-character unique identifier |

Note: Wires are electrical connections. They create nets when they connect pins or other
wires. They are NOT the same as SchLine (RECORD=13) which is purely graphical.

### RECORD=26: SchBus

Bus line. Same structure as SchWire. Not observed in our test files but documented in the
.NET model.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (SchPrimitiveBase fields) | | | |
| `LOCATIONCOUNT` | i32 | | Number of vertices |
| `X{N}` + `X{N}_FRAC` | i32 | | Vertex N X coordinate (1-based) |
| `Y{N}` + `Y{N}_FRAC` | i32 | | Vertex N Y coordinate (1-based) |
| `COLOR` | i32 | | Bus color |
| `LINEWIDTH` | i32 | 1 | Line width |
| `UNIQUEID` | string | | 8-character unique identifier |

### RECORD=25: SchNetLabel

Net label that names a wire/net. Composes SchLabel (RECORD=4) and inherits its text fields.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (SchGraphicalBase fields) | | | |
| `TEXT` | string | | Net name (e.g., "VCC3P3", "FPGA_GPIO4") |
| `FONTID` | i32 | 1 | Font ID (1-based index into font table) |
| `ORIENTATION` | i32 | 0 | Text orientation bitmask (bit 0=ROTATED 90deg, bit 1=FLIPPED) |
| `JUSTIFICATION` | i32 | 0 | Text justification (see `TextJustification` in enumerations.md) |
| `ISMIRRORED` | bool | F | Mirror flag |
| `ISHIDDEN` | bool | F | Hidden flag |
| `UNIQUEID` | string | | 8-character unique identifier |

### RECORD=17: SchPowerObject

Power port symbol (VCC, GND, etc.). Inherits from ILabel in the .NET model.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (SchGraphicalBase fields) | | | |
| `TEXT` | string | | Power net name (e.g., "VCC3P3", "GND") |
| `STYLE` | i32 | | Power symbol style (see `PowerObjectStyle` in enumerations.md) |
| `SHOWNETNAME` | bool | T | Show net name text |
| `ORIENTATION` | i32 | | Direction (0=right, 1=up, 2=left, 3=down) |
| `FONTID` | i32 | 1 | Font ID |
| `UNIQUEID` | string | | 8-character unique identifier |

Observed Style values: 2 (bar/VCC style), 4 (ground symbol).
Observed Orientation values: 1 (up, for VCC), 3 (down, for GND).

### RECORD=18: SchPort

Port connection point. Not observed in our test files but documented in .NET model.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (SchGraphicalBase fields) | | | |
| `NAME` | string | | Port name |
| `IOTYPE` | i32 | | Port direction (see `PortIoType` in enumerations.md) |
| `STYLE` | i32 | | Arrow style (see `PortArrowStyle` in enumerations.md) |
| `ALIGNMENT` | i32 | | Text alignment |
| `WIDTH` | i32 | | Port width |
| `HEIGHT` | i32 | | Port height |
| `TEXTCOLOR` | i32 | | Text color (COLORREF) |
| `FONTID` | i32 | 1 | Font ID |
| `UNIQUEID` | string | | 8-character unique identifier |

### RECORD=22: SchNoConnect

No-connect (no ERC) marker placed on unconnected pins.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (SchGraphicalBase fields) | | | |
| `ORIENTATION` | i32 | | Orientation |
| `SYMBOL` | string | | Symbol type ("Thin Cross" or "Checkbox") |
| `ISACTIVE` | bool | T | Whether the marker is active |
| `SUPPRESSALL` | bool | T | Suppress all ERC violations |
| `CONNECTIONPAIRSTOSUPPRESS` | string | | Specific pairs to suppress (only when SuppressAll=F) |
| `UNIQUEID` | string | | 8-character unique identifier |

### RECORD=29: SchJunction

Junction dot where wires meet.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (SchGraphicalBase fields) | | | |
| `COLOR` | i32 | 128 | Junction color |

Note: Junctions have NO UniqueID and always have `IndexInSheet=-1`.

### RECORD=15: SchSheetSymbol

Hierarchical sheet symbol. Not observed in our test files but documented in .NET model.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (SchGraphicalBase fields) | | | |
| `CORNER.X` + `CORNER.X_FRAC` | i32 | | Bottom-right corner X |
| `CORNER.Y` + `CORNER.Y_FRAC` | i32 | | Bottom-right corner Y |
| `ISSOLID` | bool | F | Solid fill |
| `UNIQUEID` | string | | 8-character unique identifier |
| `SYMBOLTYPE` | string | | Sheet symbol type |
| `SHEETNAME` | string | | Display name for the sheet |
| `FILENAME` | string | | Referenced SchDoc file path |

### RECORD=16: SchSheetEntry

Entry point on a sheet symbol. Child of a SchSheetSymbol. Not observed in our test files
but documented in .NET model.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (SchGraphicalBase fields) | | | |
| `NAME` | string | | Entry/port name |
| `IOTYPE` | i32 | | Direction (see `PortIoType` in enumerations.md) |
| `SIDE` | i32 | | Which side of the sheet symbol (0=left, 1=right, 2=top, 3=bottom) |
| `STYLE` | i32 | | Arrow style |
| `UNIQUEID` | string | | 8-character unique identifier |

### RECORD=43: SchParameterSet

Parameter set marker. Attaches named parameters to a wire or other net object. Named `eParameterSet`/`SchDataParameterSet` in the .NET source (`BinaryFileCode.cs:89: CParameterSet = 43`).

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (SchGraphicalBase fields) | | | |
| `LOCATION.X` + `LOCATION.X_FRAC` | i32 | | X position (DXP fractional) |
| `LOCATION.Y` + `LOCATION.Y_FRAC` | i32 | | Y position (DXP fractional) |
| `COLOR` | i32 | | Color (COLORREF) |
| `ORIENTATION` | i32 | 0 | Orientation (RotationBy90) |
| `NAME` | string | | Parameter set name |
| `STYLE` | i32 | 0 | Visual style (see `ParameterSetStyle` in enumerations.md) |
| `UNIQUEID` | string | | 8-character unique identifier |

### RECORD=209: SchNote

Annotation text frame with author and rich formatting. Named `eNote`/`SchDataNote` in the
.NET source (docs/dxp/sch-files.md code 209). Despite being called "Hyperlink" or
"TextFrameVariant" in some earlier references, it functions as an annotation note box. The
actual `eHyperlink` type has binary code 226 (not observed in our test files).

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (SchGraphicalBase fields) | | | |
| `CORNER.X` + `CORNER.X_FRAC` | i32 | | Bottom-right corner X |
| `CORNER.Y` + `CORNER.Y_FRAC` | i32 | | Bottom-right corner Y |
| `TEXT` | string | | Annotation text |
| `AUTHOR` | string | | Author initials/name |
| `FONTID` | i32 | | Font ID |
| `TEXTCOLOR` | i32 | | Text color (COLORREF) |
| `ISSOLID` | bool | T | Solid fill |
| `SHOWBORDER` | bool | T | Show border |
| `WORDWRAP` | bool | T | Word wrap enabled |
| `CLIPTORECT` | bool | T | Clip text to rectangle |
| `TEXTMARGIN` | i32 | 5 | Text margin |

### RECORD=210: SchProbe

Simulation probe marker.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (SchGraphicalBase fields) | | | |
| `LOCATION.X` + `LOCATION.X_FRAC` | i32 | | X position |
| `LOCATION.Y` + `LOCATION.Y_FRAC` | i32 | | Y position |
| `COLOR` | i32 | | Color |
| `ORIENTATION` | i32 | 0 | Orientation (RotationBy90) |
| `NAME` | string | | Probe name |
| `UNIQUEID` | string | | 8-character unique identifier |

### RECORD=211: SchCompileMask

Compile mask / blanket region that suppresses compile warnings. Named `eCompileMask`/`SchDataCompileMask` in the .NET source (`BinaryFileCode.cs:241: CCompileMask = 211`).

**Note:** RECORD=211 is the actual CompileMask. RECORD=43 is ParameterSet (see above).

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (SchGraphicalBase fields) | | | |
| `UNIQUEID` | string | | 8-character unique identifier (exported FIRST, before Location) |
| `LOCATION.X` + `LOCATION.X_FRAC` | i32 | | Bottom-left corner X |
| `LOCATION.Y` + `LOCATION.Y_FRAC` | i32 | | Bottom-left corner Y |
| `CORNER.X` + `CORNER.X_FRAC` | i32 | | Top-right corner X |
| `CORNER.Y` + `CORNER.Y_FRAC` | i32 | | Top-right corner Y |
| `COLOR` | i32 | | Line color (COLORREF) |
| `AREACOLOR` | i32 | | Fill color (COLORREF) |
| `COLLAPSED` | bool | F | Whether the mask is collapsed |
| `LINEWIDTH` | i32 | 0 | Line width (TSize enum) |

UniqueID ordering anomaly: CompileMask exports UniqueID BEFORE Location/Corner, unlike most records.

---

## Shared record types (same in SchDoc and SchLib)

These record types have the same field definitions in both SchDoc and SchLib. They should
share a single implementation. Note: in SchDoc they always have `INDEXINSHEET` and
frequently have `ISNOTACCESIBLE`, which are typically absent in SchLib.

### RECORD=3: SchSymbol

IEEE symbol graphical primitive. Named `eSymbol`/`SchDataSymbol` in .NET.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (SchGraphicalBase fields) | | | |
| `SYMBOL` | i32 | 0 | IEEE symbol type (IeeeSymbol enum) |
| `LOCATION.X` + `LOCATION.X_FRAC` | i32 | | X position |
| `LOCATION.Y` + `LOCATION.Y_FRAC` | i32 | | Y position |
| `SCALEFACTOR` | i32 | | Scale factor (coord) |
| `ORIENTATION` | i32 | 0 | Orientation (RotationBy90) |
| `LINEWIDTH` | i32 | 0 | Line width (TSize enum) |
| `COLOR` | i32 | | Color (COLORREF) |
| `MIRROR` | bool | F | Mirror flag (note: key is "Mirror", not "IsMirrored") |

### RECORD=1: SchComponent

Component symbol instance. Present in both SchDoc and SchLib but with slightly different
field sets.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (SchGraphicalBase fields) | | | |
| `LIBREFERENCE` | string | | Component name in library |
| `COMPONENTDESCRIPTION` | string | | Human-readable description |
| `DESIGNITEMID` | string | | Design item identifier (SchDoc-specific) |
| `UNIQUEID` | string | | 8-character unique identifier |
| `CURRENTPARTID` | i32 | 1 | Currently active part number |
| `PARTCOUNT` | i32 | 2 | Total parts (stored as actual_count + 1 in file) |
| `DISPLAYMODECOUNT` | i32 | 1 | Number of display modes |
| `DISPLAYMODE` | i32 | 0 | Current display mode |
| `LIBRARYPATH` | string | * | Library path (default literal `*`) |
| `SOURCELIBRARYNAME` | string | * | Source library name |
| `TARGETFILENAME` | string | * | Target filename (default literal `*`) |
| `ORIENTATION` | i32 | 0 | Rotation (0=0, 1=90, 2=180, 3=270 degrees) |
| `PARTIDLOCKED` | bool | F | Lock part ID |
| `ALLPINCOUNT` | i32 | | Total pin count (SchDoc-specific) |
| `NOTUSEDBTABLENAME` | bool | | Database table flag (SchDoc-specific) |
| `SHOWHIDDENPINS` | bool | F | Show hidden pins |
| `OVERRIDECOLORS` | bool | F | Override primitive colors |
| `DESIGNATORLOCKED` | bool | F | Lock designator text |
| `COMPONENTKIND` | i32 | 0 | Component kind (see enumerations.md) |
| `ALIASLIST` | string | | Pipe-separated alias list |
| `SHEETPARTFILENAME` | string | * | Sheet part filename |

### RECORD=2: SchPin (text format in SchDoc)

In SchDoc, pins are parameter text records (flags=0x00). In SchLib, pins are binary
records (flags=0x01). The field set is the same, but the encoding differs.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `RECORD` | i32 | 2 | Always 2 |
| `OWNERINDEX` | i32 | | Parent component index |
| `OWNERPARTID` | i32 | | Part number (1-based) |
| `SYMBOL_INNEREDGE` | i32 | 0 | Inner edge IEEE symbol (see `PinSymbol` in enumerations.md) |
| `SYMBOL_OUTEREDGE` | i32 | 0 | Outer edge IEEE symbol |
| `SYMBOL_INSIDE` | i32 | 0 | Inside IEEE symbol |
| `SYMBOL_OUTSIDE` | i32 | 0 | Outside IEEE symbol |
| `SYMBOL_LINEWIDTH` | i32 | 0 | Symbol line width |
| `DESCRIPTION` | string | | Pin description text |
| `FORMALTYPE` | i32 | | Formal type |
| `ELECTRICAL` | i32 | | Electrical type (see `PinElectricalType` in enumerations.md) |
| `PINCONGLOMERATE` | i32 | | Bitmask: orientation, visibility (see enumerations.md) |
| `PINLENGTH` + `PINLENGTH_FRAC` | i32 | | Pin length (DXP fractional pair) |
| `LOCATION.X` + `LOCATION.X_FRAC` | i32 | | Pin endpoint X (DXP fractional pair) |
| `LOCATION.Y` + `LOCATION.Y_FRAC` | i32 | | Pin endpoint Y (DXP fractional pair) |
| `COLOR` | i32 | 128 | Pin color (COLORREF) |
| `NAME` | string | | Pin name |
| `DESIGNATOR` | string | | Pin designator (number) |
| `SWAPIDPIN` | string | | Pin swap group ID |
| `SPICEPINNAME` | string | | SPICE pin name |
| `SWAPIDPART` | string | | Part swap group identifier |
| `HIDDENNETNAME` | string | | Net name for hidden pins (e.g., "VCC") |
| `PINPROPAGATIONDELAY` | f64 | 0.0 | Propagation delay |
| `DEFAULTVALUE` | string | | Default logic value |
| `UNIQUEID` | string | | 8-character unique identifier |

Note: In SchDoc, pin coordinates use full DXP fractional encoding (integer + _FRAC).
In SchLib binary format, coordinates are truncated to i16 and the PinFrac sidecar
provides the remainder.

Note: `SWAPIDPART` can appear as a duplicate key (multiple values). This is a known
encoding artifact from Altium's serialization.

### RECORD=4: SchLabel

Text label annotation.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (SchGraphicalBase fields) | | | |
| `TEXT` | string | | Label text (can be expression like `=sheetnumber`) |
| `ORIENTATION` | i32 | 0 | Text orientation (see enumerations.md) |
| `JUSTIFICATION` | i32 | 0 | Text justification (see enumerations.md) |
| `FONTID` | i32 | 1 | Font ID (1-based) |
| `ISMIRRORED` | bool | F | Mirror flag |
| `ISHIDDEN` | bool | F | Hidden flag |

### RECORD=5: SchBezier

Cubic bezier curve. Same indexed coordinate structure as SchPolyline with `LOCATIONCOUNT=4`.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (SchGraphicalBase fields) | | | |
| `LINEWIDTH` | i32 | 1 | Line width |
| `LOCATIONCOUNT` | i32 | 4 | Always 4 (control points) |
| `X{N}` + `X{N}_FRAC` | i32 | | Control point N X (1-based) |
| `Y{N}` + `Y{N}_FRAC` | i32 | | Control point N Y (1-based) |

### RECORD=6: SchPolyline

Multi-segment polyline with N vertices.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (SchGraphicalBase fields) | | | |
| `LINEWIDTH` | i32 | 1 | Line width |
| `LINESTYLE` | i32 | 0 | Line style (see enumerations.md) |
| `LOCATIONCOUNT` | i32 | | Number of vertices |
| `X{N}` + `X{N}_FRAC` | i32 | | Vertex N X coordinate (1-based) |
| `Y{N}` + `Y{N}_FRAC` | i32 | | Vertex N Y coordinate (1-based) |

### RECORD=7: SchPolygon

Closed filled polygon. Same structure as SchPolyline plus fill fields.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (all SchPolyline fields) | | | |
| `ISSOLID` | bool | F | Solid fill |

### RECORD=8: SchEllipse

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (SchGraphicalBase fields) | | | |
| `RADIUS` + `RADIUS_FRAC` | i32 | | Primary (major) radius |
| `SECONDARYRADIUS` + `SECONDARYRADIUS_FRAC` | i32 | | Secondary (minor) radius |
| `ISSOLID` | bool | F | Solid fill |
| `LINEWIDTH` | i32 | 1 | Line width |

### RECORD=9: SchPie

Pie/wedge shape (filled arc sector).

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (SchGraphicalBase fields) | | | |
| `RADIUS` + `RADIUS_FRAC` | i32 | | Radius |
| `STARTANGLE` | f64 | 0.0 | Start angle in degrees |
| `ENDANGLE` | f64 | 360.0 | End angle in degrees |
| `ISSOLID` | bool | F | Solid fill |
| `LINEWIDTH` | i32 | 1 | Line width |

### RECORD=10: SchRoundRectangle

Rounded-corner rectangle.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (SchGraphicalBase fields) | | | |
| `LOCATION.X` + `LOCATION.X_FRAC` | i32 | | Bottom-left corner X |
| `LOCATION.Y` + `LOCATION.Y_FRAC` | i32 | | Bottom-left corner Y |
| `CORNER.X` + `CORNER.X_FRAC` | i32 | | Top-right corner X |
| `CORNER.Y` + `CORNER.Y_FRAC` | i32 | | Top-right corner Y |
| `CORNERXRADIUS` + `CORNERXRADIUS_FRAC` | i32 | | Corner X radius |
| `CORNERYRADIUS` + `CORNERYRADIUS_FRAC` | i32 | | Corner Y radius |
| `LINEWIDTH` | i32 | 0 | Line width |
| `COLOR` | i32 | | Color |
| `AREACOLOR` | i32 | | Fill color |
| `ISSOLID` | bool | F | Solid fill |
| `UNIQUEID` | string | | 8-character unique identifier |

### RECORD=11: SchEllipticalArc

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (SchGraphicalBase fields) | | | |
| `RADIUS` + `RADIUS_FRAC` | i32 | | Primary radius |
| `SECONDARYRADIUS` + `SECONDARYRADIUS_FRAC` | i32 | | Secondary radius |
| `STARTANGLE` | f64 | 0.0 | Start angle in degrees |
| `ENDANGLE` | f64 | 360.0 | End angle in degrees |
| `LINEWIDTH` | i32 | 1 | Line width |

### RECORD=12: SchArc

Circular arc. `LOCATION` is the center point.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (SchGraphicalBase fields) | | | |
| `RADIUS` + `RADIUS_FRAC` | i32 | | Arc radius (DXP fractional pair) |
| `STARTANGLE` | f64 | 0.0 | Start angle in degrees |
| `ENDANGLE` | f64 | 360.0 | End angle in degrees (360 = full circle) |
| `LINEWIDTH` | i32 | 1 | Line width |

### RECORD=13: SchLine

Single line segment from `LOCATION` to `CORNER`.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (SchGraphicalBase fields) | | | |
| `CORNER.X` + `CORNER.X_FRAC` | i32 | | End point X |
| `CORNER.Y` + `CORNER.Y_FRAC` | i32 | | End point Y |
| `LINEWIDTH` | i32 | 1 | Line width |
| `LINESTYLE` | i32 | 0 | Line style |

### RECORD=14: SchRectangle

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (SchGraphicalBase fields) | | | |
| `CORNER.X` + `CORNER.X_FRAC` | i32 | | Top-right corner X |
| `CORNER.Y` + `CORNER.Y_FRAC` | i32 | | Top-right corner Y |
| `ISSOLID` | bool | F | Solid fill |
| `LINEWIDTH` | i32 | 1 | Line width |
| `TRANSPARENT` | bool | F | Transparent fill |

### RECORD=28: SchTextFrame

Text frame with optional border and background.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (SchGraphicalBase fields) | | | |
| `CORNER.X` + `CORNER.X_FRAC` | i32 | | Bottom-right corner X |
| `CORNER.Y` + `CORNER.Y_FRAC` | i32 | | Bottom-right corner Y |
| `TEXT` | string | | Frame text content |
| `FONTID` | i32 | 1 | Font ID |
| `TEXTCOLOR` | i32 | | Text color (COLORREF) |
| `TEXTMARGIN` + `TEXTMARGIN_FRAC` | i32 | | Text margin |
| `WORDWRAP` | bool | F | Word wrap enabled |
| `CLIPTORECT` | bool | F | Clip text to rectangle |
| `SHOWBORDER` | bool | T | Show frame border |
| `ISSOLID` | bool | F | Solid fill background |
| `ALIGNMENT` | i32 | 0 | Text alignment |

### RECORD=30: SchImage

Embedded image. The `FILENAME` key matches an embedded object in the `Storage` stream.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (SchGraphicalBase fields) | | | |
| `CORNER.X` + `CORNER.X_FRAC` | i32 | | Bottom-right corner X |
| `CORNER.Y` + `CORNER.Y_FRAC` | i32 | | Bottom-right corner Y |
| `FILENAME` | string | | Image filename (matches Storage stream entry id) |
| `EMBEDIMAGE` | bool | T | Whether image is embedded |
| `KEEPASPECT` | bool | T | Maintain aspect ratio |

Note: `OWNERINDEX` is present when the image is owned by a template (RECORD=39) but
absent for standalone images.

### RECORD=32: SchSheetName

Sheet name label on a sheet symbol. Same parameter order as SchLabel with additional fields.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (SchGraphicalBase fields) | | | |
| `LOCATION.X` + `LOCATION.X_FRAC` | i32 | | X position |
| `LOCATION.Y` + `LOCATION.Y_FRAC` | i32 | | Y position |
| `ORIENTATION` | i32 | 0 | Orientation (RotationBy90) |
| `JUSTIFICATION` | i32 | 0 | Text justification |
| `COLOR` | i32 | | Color |
| `FONTID` | i32 | 0 | Font ID |
| `ISHIDDEN` | bool | F | Hidden flag |
| `TEXT` | string | | Sheet name text |
| `ISMIRRORED` | bool | F | Mirror flag |
| `NOTAUTOPOSITION` | bool | F | Inverted from AutoPosition |
| `TEXTHORZANCHOR` | i32 | 0 | Horizontal anchor |
| `TEXTVERTANCHOR` | i32 | 0 | Vertical anchor |
| `UNIQUEID` | string | | 8-character unique identifier |

### RECORD=33: SchSheetFileName

Sheet filename label. Identical parameter order to RECORD=32 (SchSheetName).

### RECORD=34: SchDesignator

Reference designator text (e.g., "U1", "R5", "C12"). One per component.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (SchGraphicalBase fields) | | | |
| `TEXT` | string | * | Designator text |
| `NAME` | string | Designator | Parameter name (always "Designator") |
| `READONLYSTATE` | i32 | 1 | Read-only flag |
| `FONTID` | i32 | 1 | Font ID (1-based) |
| `UNIQUEID` | string | | 8-character unique identifier |

### RECORD=37: SchBusEntry

Bus tap connector.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (SchGraphicalBase fields) | | | |
| `UNIQUEID` | string | | 8-character unique identifier (exported FIRST, before Location) |
| `LOCATION.X` + `LOCATION.X_FRAC` | i32 | | Start point X |
| `LOCATION.Y` + `LOCATION.Y_FRAC` | i32 | | Start point Y |
| `CORNER.X` + `CORNER.X_FRAC` | i32 | | End point X |
| `CORNER.Y` + `CORNER.Y_FRAC` | i32 | | End point Y |
| `LINEWIDTH` | i32 | 0 | Line width |
| `COLOR` | i32 | | Color |

UniqueID ordering anomaly: same as CompileMask -- UniqueID is exported first.

### RECORD=41: SchParameter

Named parameter on a component or sheet. Most common record type.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| (SchGraphicalBase fields) | | | |
| `TEXT` | string | * | Parameter value (`*` = dynamic/unset) |
| `NAME` | string | Comment | Parameter name (e.g., "Comment", "Value", "CurrentTime") |
| `FONTID` | i32 | 1 | Font ID (1-based) |
| `UNIQUEID` | string | | 8-character unique identifier |
| `READONLYSTATE` | i32 | | Read-only flag |
| `ISHIDDEN` | bool | F | Hidden flag |
| `SHOWNAME` | bool | | Show parameter name alongside value |
| `ORIENTATION` | i32 | | Text orientation |

Note: Parameters are heavily used. Each component generates 20-30 parameters. Sheet-level
parameters (OWNERINDEX referencing RECORD=31) include system parameters like "CurrentTime",
"CurrentDate", "DocumentFullPathAndName", etc.

### RECORD=44: SchImplementationList

Container for footprint implementations. No fields beyond SchPrimitiveBase.

| Key | Type | Description |
|-----|------|-------------|
| `RECORD` | i32 | Always 44 |
| `OWNERINDEX` | i32 | Parent component index |

### RECORD=45: SchImplementation

Single footprint assignment. OWNERINDEX points to a SchImplementationList.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `RECORD` | i32 | 45 | |
| `OWNERINDEX` | i32 | | Index of parent SchImplementationList |
| `INDEXINSHEET` | i32 | -1 | Always -1 |
| `MODELNAME` | string | | Footprint name (e.g., "10M16SAU169C8G", "0402") |
| `MODELTYPE` | string | PCBLIB | Model type string |
| `DESCRIPTION` | string | | Human-readable footprint description |
| `DATAFILECOUNT` | i32 | | Number of data file entries |
| `MODELDATAFILEENTITY0` | string | | Data file entity |
| `MODELDATAFILEKIND0` | string | | Data file kind (e.g., "PCBLib") |
| `ISCURRENT` | bool | F | Whether this is the active implementation |
| `DATALINKSLOCKED` | bool | F | Data links locked (may be absent in SchDoc) |
| `DATABASEDATALINKSLOCKED` | bool | F | Database data links locked (may be absent) |
| `INTEGRATEDMODEL` | bool | F | Integrated model flag (may be absent) |
| `DATABASEMODEL` | bool | F | Database model flag (may be absent) |
| `UNIQUEID` | string | | 8-character unique identifier |

### RECORD=46: SchImplementationMap

Container for pin-to-pad mappings. OWNERINDEX points to a SchImplementation. No
additional fields beyond `OWNERINDEX` and `RECORD`.

### RECORD=47: SchMapDefiner

Pin-to-pad mapping entry. OWNERINDEX points to a SchImplementation.

| Key | Type | Description |
|-----|------|-------------|
| `RECORD` | i32 | Always 47 |
| `OWNERINDEX` | i32 | Index of parent SchImplementation |
| `PINNAME` | string | Schematic pin name |
| `PADNAME` | string | PCB pad name |

### RECORD=48: SchParameterList

Container for parameter list entries. Named `CParameterList` in `BinaryFileCode.cs:99`. OWNERINDEX points to a SchImplementation. No additional fields beyond `OWNERINDEX` and `RECORD`.

---

## Record type census from real files

Observed across 9 LimeSDR SchDoc files:

| RECORD | Type | Count | Files present |
|--------|------|-------|--------------|
| 1 | SchComponent | 815 | 04-09 |
| 2 | SchPin | 3,777 | 04-09 |
| 4 | SchLabel | 528 | All 9 |
| 5 | SchBezier | 2 | Rare |
| 6 | SchPolyline | 2,652 | All 9 |
| 7 | SchPolygon | 136 | 05, 09 |
| 8 | SchEllipse | 31 | 05, 09 |
| 9 | SchPie | 5 | 08, 09 |
| 11 | SchEllipticalArc | 10 | 05, 09 |
| 12 | SchArc | 334 | 05, 06, 07, 09 |
| 13 | SchLine | 229 | 04 (and others) |
| 14 | SchRectangle | 949 | 04-09 |
| 17 | SchPowerObject | 256 | 04-09 |
| 22 | SchNoConnect | 71 | 04-09 |
| 25 | SchNetLabel | 336 | 04, 05, 07-09 |
| 27 | SchWire | 891 | 04-09 |
| 28 | SchTextFrame | 479 | 04-09 |
| 29 | SchJunction | 492 | 04-09 |
| 30 | SchImage | 30 | 01-03, 05, 08, 09 |
| 31 | SchSheet | 9 | All 9 (exactly 1 each) |
| 34 | SchDesignator | 815 | 04-09 (1:1 with components) |
| 39 | SchTemplate | 9 | All 9 (exactly 1 each) |
| 41 | SchParameter | 17,713 | All 9 (most common) |
| 43 | SchParameterSet | 78 | 04-09 |
| 44 | SchImplementationList | 441 | 04-09 |
| 45 | SchImplementation | 911 | 04-09 |
| 46 | SchImplementationMap | 911 | 04-09 |
| 47 | SchMapDefiner | 25 | 05, 08, 09 |
| 48 | SchParameterList | 911 | 04-09 |
| 209 | SchNote | 36 | All 9 |
| 225 | SchDashedRectangle | 23 | 04, 05, 07-09 (in Additional stream) |

Documented above but not observed in LimeSDR test files:

- RECORD=3: SchSymbol (documented above)
- RECORD=10: SchRoundRectangle (documented above)
- RECORD=15: SchSheetSymbol (documented above)
- RECORD=16: SchSheetEntry (documented above)
- RECORD=18: SchPort (documented above)
- RECORD=26: SchBus (documented above)
- RECORD=32: SchSheetName (documented above)
- RECORD=33: SchSheetFileName (documented above)
- RECORD=37: SchBusEntry (documented above)
- RECORD=210: SchProbe (documented above)
- RECORD=211: SchCompileMask (documented above)

Not observed in LimeSDR test files and not yet documented:

- RECORD=23: SchErrorMarker (appears in files saved with compilation errors present)
- RECORD=106-124: Harness records (appear in wire harness schematics)
- RECORD=226: SchHyperlink (URL hyperlink -- distinct from RECORD=209 SchNote)
