# RECORD=18 (Port) - Missing Fields Research

## Problem

RECORD=18 (Port) is missing `HARNESSTYPE`, `AUTOSIZE`, `BorderWidth`, `PortNameIsHidden`,
and `ObjectDefinitionId` parameters. This blocks validation of 85 SchDoc files.

## Current Implementation

File: `crates/altium-format/src/sch_records.rs` lines 1569-1598

```rust
/// Port record (RECORD=18).
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SchPort {
    #[param(flatten)]
    pub base: SchPrimitiveBase,
    #[param(coord_point, x_key = LOCATION_X, x_frac = LOCATION_X_FRAC, y_key = LOCATION_Y, y_frac = LOCATION_Y_FRAC)]
    pub location: CoordPoint,
    #[param(key = COLOR, default = Color::BLACK)]
    pub color: Color,
    #[param(key = AREA_COLOR, default = Color::BLACK)]
    pub area_color: Color,
    #[param(key = NAME, default = String::new())]
    pub name: String,
    #[param(key = IO_TYPE, default = 0i32)]
    pub io_type: i32,
    #[param(key = STYLE, default = 0i32)]
    pub style: i32,
    #[param(coord, key = WIDTH, frac_key = "Width_Frac")]
    pub width: Coord,
    #[param(coord, key = HEIGHT, frac_key = "Height_Frac")]
    pub height: Coord,
    #[param(key = TEXT_COLOR, default = Color::BLACK)]
    pub text_color: Color,
    #[param(key = FONT_ID, default = 1i32)]
    pub font_id: i32,
    #[param(key = ALIGNMENT, default = TextJustification::BottomLeft)]
    pub alignment: TextJustification,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
}
```

### Issues in Current Implementation

1. **Missing fields**: `HarnessType`, `AutoSize`, `BorderWidth`, `PortNameIsHidden`, `ObjectDefinitionId`
2. **Wrong type for `io_type`**: Uses `i32` instead of a domain enum (should be `PortIO`)
3. **Wrong type for `style`**: Uses `i32` instead of `PortArrowStyle`
4. **Wrong type for `alignment`**: Uses `TextJustification` but should be `THorizontalAlign` (only 3 values: Left/Center/Right)
5. **Wrong type for `font_id`**: Uses `i32`, consistent with other records but should probably use `i32` (Altium font IDs are 1-based indices)

## C# Reference Analysis

### Complete Serialization Order (FileFormatV5.cs lines 731-821)

**Export (lines 731-756):**
```csharp
ExportGraphicalObject(argSerializer, argObject);  // base class fields
Export_PortArrowStyle(style, "Style");
Export_PortIO(ioType, "IOType");
Export_HorizontalAlign(alignment, "Alignment");
Export_Coord(width, "Width");
Export_Coord(location.X, "Location.X");
Export_Coord(location.Y, "Location.Y");
Export_Color(color, "Color");
Export_FontID(fontID, "FontID");
Export_Color(areaColor, "AreaColor");
Export_Color(textColor, "TextColor");
Export_DynamicString(name, "Name");
Export_DynamicString(harnessType, "HarnessType");
Export_DynamicString(uniqueId, "UniqueID");
Export_Coord(height, "Height");
Export_Size(borderWidth, "BorderWidth");
Export_Boolean(autoSize, "AutoSize");
Export_DynamicString(objectDefinitionId, "ObjectDefinitionId");
Export_Boolean(!showNetName, "PortNameIsHidden");  // NOTE: inverted!
ExportDefaultCrossSheetHidden(...)  // only for GlobalSchDataPort
```

**Import (lines 758-821) - with defaults:**
```csharp
Style          = TPortArrowStyle.ePortNone (0)     // then normalized!
IOType         = TPortIO.ePortUnspecified (3)
Alignment      = THorizontalAlign.eHorizontalCentreAlign (1)
Width          = 50 (coord)
Location.X     = 0
Location.Y     = 0
Color          = 0 (black)
AreaColor      = 0 (black)
TextColor      = 0 (black)
FontID         = 1
Name           = "Port"
HarnessType    = "" (empty string)
UniqueID       = "" (empty string)
Height         = 0 (then clamped: if <= 0, set to 1000000)
BorderWidth    = TSize.eZeroSize (0)
AutoSize       = false
ObjectDefinitionId = "" (empty string)
PortNameIsHidden = false (meaning ShowNetName = true)
```

### Style Import Normalization (line 765)

During import, the style value is normalized to a "base" style:
- Horizontal styles (0-3: None, Left, Right, LeftRight) -> `ePortNone` (0)
- Vertical styles (4-7: NoneVertical, Top, Bottom, TopBottom) -> `ePortNoneVertical` (4)

This appears to be legacy behavior where the full style is reconstructed at runtime.
The raw value from the file IS the full style; this normalization happens only during import.
For our parser, we should parse the raw value as-is (preserving the full enum value).

### SchDataPort Data Object (SchDataPort.cs)

Private fields:
```csharp
private THorizontalAlign alignment;        // 0=Left, 1=Center, 2=Right
private bool autoSize;                     // default: true
private TSize borderWidth;                 // TSize enum (0-3)
private int fontID;                        // 1-based font table index
private string harnessType;                // e.g. "USB", "LVDS" or ""
private int height;                        // coord, default: 1000000
private TPortIO portIOType;                // 0-3
private string name;                       // default: "Port"
private TPortArrowStyle style;             // 0-7
private uint textColor;                    // COLORREF
private int width;                         // coord
// Properties (not in private fields):
string ObjectDefinitionId;                 // custom port shape reference
bool ShowNetName;                          // default: true
```

### SetDefault values (SchDataPort.cs lines 242-260):
```csharp
name = "Port";
harnessType = string.Empty;
style = TPortArrowStyle.ePortNone;         // 0
portIOType = TPortIO.ePortUnspecified;     // 3
alignment = THorizontalAlign.eLeftAlign;   // 0
width = cDefaultPortWidth[unit];           // unit-dependent
height = 1000000;                          // 100mil
fontID = GetDefaultHorizontalSysFontId();
borderWidth = TSize.eZeroSize;            // 0
autoSize = true;
textColor = GetPortText();                // from prefs
ObjectDefinitionId = null;
ShowNetName = true;
```

## Missing Fields Detail

### 1. `HarnessType` (String)

- **Parameter key**: `"HarnessType"` (constant: `harness::HARNESS_TYPE`)
- **Wire type**: DynamicString
- **Default**: `""` (empty string)
- **Semantics**: Names the harness type this port belongs to (e.g., "USB", "LVDS"). When
  non-empty, the port is a harness port. When empty, it's a regular signal port.
- **Runtime behavior**: `isHarnessObject` is set to `true` when `harnessType.Length != 0`
  (see `HarnessPropertiesCalculator`). Harness ports always report IOType as `ePortUnspecified`.
  The harness color and inferred type are runtime-only (not persisted).

### 2. `AutoSize` (bool)

- **Parameter key**: `"AutoSize"` (constant: `text::AUTO_SIZE`)
- **Wire type**: bool
- **Default**: `false` (import default) / `true` (SetDefault)
- **Semantics**: When true, the port width auto-adjusts to fit the port name text.
  When false, the width is fixed at the user-specified value.

### 3. `BorderWidth` (PenWidth)

- **Parameter key**: `"BorderWidth"` (constant: `visual::BORDER_WIDTH`)
- **Wire type**: u8 (TSize enum: 0=Zero, 1=Small, 2=Medium, 3=Large)
- **Default**: `PenWidth::Zero` (0)
- **Semantics**: Width of the port outline/border. Maps to `Rt_Schematic.TSize` enum
  which is identical to our `PenWidth` enum.

### 4. `PortNameIsHidden` (bool, inverted semantics)

- **Parameter key**: `"PortNameIsHidden"` (constant: `electrical::PORT_NAME_IS_HIDDEN`)
- **Wire type**: bool
- **Default**: `false` (meaning net name IS shown)
- **Semantics**: When `true`, the net name label on the port is hidden. Stored with
  **inverted semantics** in SchDataPort as `ShowNetName` (so `PortNameIsHidden=T` means
  `ShowNetName=false`).
- **In our struct**: Store as `port_name_is_hidden: bool` with default `false`.

### 5. `ObjectDefinitionId` (String)

- **Parameter key**: `"ObjectDefinitionId"` (constant: `harness::OBJECT_DEFINITION_ID`)
- **Wire type**: DynamicString
- **Default**: `""` (empty string)
- **Semantics**: References a custom port shape definition (RECORD=129 ObjectDefinition).
  When non-empty, the port uses a custom visual style defined by the referenced object
  definition. `GetState_IsCustomStyle()` returns `true` when this is non-empty.

## Type Corrections for Existing Fields

### `io_type`: `i32` -> `PortIO` domain enum

The C# type is `TPortIO`:
```csharp
enum TPortIO {
    ePortUnspecified = 3,  // default for import
    ePortOutput = 0,
    ePortInput = 1,
    ePortBidirectional = 2,
}
```
We have `PortIO` defined... let me check. If not, it needs to be added. The current code
uses `i32` which violates the domain type rule.

**NOTE**: Looking at the existing code, `PortArrowStyle` already exists in
`altium-format-types/src/sch.rs` (line 817). Need to verify `PortIO` existence.

### `style`: `i32` -> `PortArrowStyle`

Already exists as `PortArrowStyle` enum (lines 813-847 of sch.rs):
```rust
pub enum PortArrowStyle {
    None = 0,
    Left = 1,
    Right = 2,
    LeftRight = 3,
    NoneVertical = 4,
    Top = 5,
    Bottom = 6,
    TopBottom = 7,
}
```

### `alignment`: `TextJustification` -> `THorizontalAlign`

The C# type is `THorizontalAlign`:
```csharp
enum THorizontalAlign {
    eLeftAlign = 0,
    eHorizontalCentreAlign = 1,
    eRightAlign = 2,
}
```
This is NOT the same as `TextJustification` (which has 9 values). We need a `HorizontalAlign`
enum if one doesn't exist.

## Harness Port Architecture

Harness ports are NOT a separate record type. They are regular Port records (RECORD=18)
with a non-empty `HarnessType` field. The harness-specific behavior is determined at runtime:

1. **File format**: Port record with `|HarnessType=USB|` (or similar non-empty value)
2. **On load**: `HarnessPropertiesCalculator.Calculate()` sets:
   - `isHarnessObject = true` (if harnessType non-empty)
   - `harnessTypeInferred` based on connected nets
   - `harnessColor` from connected harness net color (default: 0xE7DAD3 = `15187117`)
3. **Runtime**: `GetState_IOType()` always returns `ePortUnspecified` for harness ports
4. **Display**: `GetState_IdentifierString()` shows `"Name {HarnessType}"` instead of
   `"Name (IOType)"`
5. **None of the runtime fields are persisted** - only `HarnessType` is saved to file

## Recommended Fix

Add the 5 missing fields to `SchPort`:

```rust
/// Port record (RECORD=18).
#[derive(FromParams, ToParams, Debug)]
pub(crate) struct SchPort {
    #[param(flatten)]
    pub base: SchPrimitiveBase,
    #[param(coord_point, x_key = LOCATION_X, x_frac = LOCATION_X_FRAC, y_key = LOCATION_Y, y_frac = LOCATION_Y_FRAC)]
    pub location: CoordPoint,
    #[param(key = COLOR, default = Color::BLACK)]
    pub color: Color,
    #[param(key = AREA_COLOR, default = Color::BLACK)]
    pub area_color: Color,
    #[param(key = NAME, default = String::new())]
    pub name: String,
    #[param(key = IO_TYPE, default = 0i32)]        // TODO: use PortIO enum
    pub io_type: i32,
    #[param(key = STYLE, default = 0i32)]           // TODO: use PortArrowStyle enum
    pub style: i32,
    #[param(coord, key = WIDTH, frac_key = "Width_Frac")]
    pub width: Coord,
    #[param(coord, key = HEIGHT, frac_key = "Height_Frac")]
    pub height: Coord,
    #[param(key = TEXT_COLOR, default = Color::BLACK)]
    pub text_color: Color,
    #[param(key = FONT_ID, default = 1i32)]
    pub font_id: i32,
    #[param(key = ALIGNMENT, default = TextJustification::BottomLeft)]  // TODO: use HorizontalAlign
    pub alignment: TextJustification,
    #[param(key = UNIQUE_ID, default = String::new())]
    pub unique_id: String,
    // --- NEW FIELDS ---
    #[param(key = HARNESS_TYPE, default = String::new())]
    pub harness_type: String,
    #[param(key = BORDER_WIDTH, default = PenWidth::Zero)]
    pub border_width: PenWidth,
    #[param(key = AUTO_SIZE, default = false)]
    pub auto_size: bool,
    #[param(key = PORT_NAME_IS_HIDDEN, default = false)]
    pub port_name_is_hidden: bool,
    #[param(key = OBJECT_DEFINITION_ID, default = String::new())]
    pub object_definition_id: String,
}
```

### Constants already exist:
- `harness::HARNESS_TYPE` = `"HarnessType"`
- `visual::BORDER_WIDTH` = `"BorderWidth"`
- `text::AUTO_SIZE` = `"AutoSize"`
- `electrical::PORT_NAME_IS_HIDDEN` = `"PortNameIsHidden"`
- `harness::OBJECT_DEFINITION_ID` = `"ObjectDefinitionId"`

### Future type improvements (separate PR):
- `io_type: i32` -> `PortIO` enum (need to add if missing)
- `style: i32` -> `PortArrowStyle` (already exists)
- `alignment: TextJustification` -> `HorizontalAlign` (need to add)

### Default value notes:
- `HarnessType`: default `""` (empty string) - most ports are not harness ports
- `BorderWidth`: default `PenWidth::Zero` (0) - matches import default
- `AutoSize`: default `false` - matches import default (though SetDefault uses `true`)
- `PortNameIsHidden`: default `false` - matches import default (net name shown)
- `ObjectDefinitionId`: default `""` - most ports don't use custom shapes
