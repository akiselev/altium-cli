# Schematic Records

Schematic files (`.SchLib`, `.SchDoc`) store data as **parameter-based records**
— pipe-delimited key=value strings inside size-prefixed blocks within CFB
streams.

## Record Format

Each record is a block of Windows-1252 text:

```
|RECORD=2|OWNERINDEX=0|LOCATION.X=100|LOCATION.Y=200|COLOR=128|NAME=VCC|DESIGNATOR=1|ELECTRICAL=7|
```

The `RECORD` parameter identifies the record type. The library reads this value,
dispatches to the matching Rust struct, and calls `FromParams` to populate the
fields.

### RECORDEX — Extended record types (RECORD=254)

When `RECORD=254`, the actual record type is stored in a second parameter `RECORDEX`:

```
|RECORD=254|RECORDEX=209|...|
```

This mechanism exists for record types with IDs ≥ 256 that do not fit in the legacy
`RECORD` byte. The parser reads `RECORD` first; if the value is 254, it reads `RECORDEX`
to obtain the true record type ID. Example: `SchTextFrameVariant` (type 209) may appear
via this mechanism in newer file versions. See `schlib.rs` for the dispatch implementation.

## Record Type Table

| RECORD | Rust Type | Purpose |
|--------|-----------|---------|
| 1 | `SchComponent` | Component instance (container for child primitives) |
| 2 | `SchPin` | Electrical connection pin |
| 3 | `SchSymbol` | Graphical shape primitive |
| 4 | `SchLabel` | Text annotation |
| 5 | `SchBezier` | Bezier curve |
| 6 | `SchPolyline` | Multi-segment line |
| 7 | `SchPolygon` | Closed filled polygon |
| 8 | `SchEllipse` | Ellipse |
| 9 | `SchPie` | Pie/wedge shape |
| 11 | `SchEllipticalArc` | Elliptical arc |
| 12 | `SchArc` | Circular arc |
| 13 | `SchLine` | Single line segment |
| 14 | `SchRectangle` | Rectangle |
| 17 | `SchPowerObject` | Power/ground symbol |
| 18 | `SchPort` | Sheet port connector |
| 22 | `SchNoErc` | "No ERC" marker |
| 25 | `SchNetLabel` | Net name label |
| 26 | `SchBus` | Bus (multi-wire) connection |
| 27 | `SchWire` | Electrical wire |
| 28 | `SchTextFrame` | Rich text box |
| 29 | `SchJunction` | Wire junction dot |
| 30 | `SchImage` | Embedded image |
| 31 | `SchSheetHeader` | Sheet properties (page size, grid, fonts) |
| 34 | `SchDesignator` | Component reference designator text |
| 37 | `SchBusEntry` | Bus tap point |
| 41 | `SchParameter` | Named parameter (user-defined attribute) |
| 43 | `SchWarningSign` | Warning annotation |
| 44 | `SchImplementationList` | Container for footprint assignments |
| 45 | `SchImplementation` | Single footprint assignment |
| 46 | `SchMapDefinerList` | Container for pin mappings |
| 47 | `SchMapDefiner` | Pin-to-pad mapping entry |
| 48 | `SchImplementationParameters` | Footprint parameters |
| 209 | `SchTextFrameVariant` | Variant-aware text frame |

Unknown record IDs are captured as `SchRecord::Unknown { record_id, params }`.

## Dispatch Enum

```rust
// crates/altium-format/src/sch_records.rs
pub enum SchRecord {
    Component(SchComponent),
    Pin(SchPin),
    Wire(SchWire),
    Label(SchLabel),
    // … 33 variants total …
    Unknown { record_id: i32, params: ParameterCollection },
}
```

## Base Types (Composition)

Schematic records share common fields through composition rather than
inheritance. Two base structs are flattened into concrete types:

### SchPrimitiveBase

Common to all schematic primitives:

```rust
// crates/altium-format/src/sch_records.rs
pub struct SchPrimitiveBase {
    pub owner_index: i32,                   // OWNERINDEX — parent record index (-1 = root)
    pub is_not_accessible: bool,            // ISNOTACCESIBLE
    pub owner_part_id: Option<i32>,         // OWNERPARTID — multi-part symbol part number
    pub owner_part_display_mode: Option<i32>, // OWNERPARTDISPLAYMODE
    pub graphically_locked: bool,           // GRAPHICALLYLOCKED
}
```

### SchGraphicalBase

Extends `SchPrimitiveBase` with position and color:

```rust
// crates/altium-format/src/sch_records.rs
pub struct SchGraphicalBase {
    pub base: SchPrimitiveBase,  // flattened — all base fields become top-level params
    pub location_x: i32,        // LOCATION.X + LOCATION.X_FRAC (DXP fractional)
    pub location_y: i32,        // LOCATION.Y + LOCATION.Y_FRAC
    pub color: i32,             // COLOR (Win32 COLORREF)
    pub area_color: i32,        // AREACOLOR (fill color)
}
```

Most record types embed `SchGraphicalBase` via a `#[altium(flatten)]` field,
which causes all the base parameters to be read/written as top-level keys in
the parameter string.

## Ownership Model

Records form parent-child trees via `OWNERINDEX`:

```
SchComponent (OWNERINDEX=-1, index 0 in primitives list)
├── SchPin        (OWNERINDEX=0)  — owned by component at index 0
├── SchPin        (OWNERINDEX=0)
├── SchRectangle  (OWNERINDEX=0)
├── SchDesignator (OWNERINDEX=0)
├── SchParameter  (OWNERINDEX=0)
├── SchImplementationList (OWNERINDEX=0)
│   └── SchImplementation (OWNERINDEX=6)  — owned by impl list at index 6
│       └── SchMapDefiner (OWNERINDEX=7)  — owned by implementation at index 7
└── …
```

- `OWNERINDEX = -1` (or absent): top-level record, not owned by another.
- `OWNERINDEX = N`: owned by the record at position N in the current
  component's primitive list.

### Multi-Part Symbols

Components with multiple parts (e.g., a dual op-amp) use:
- `part_count` on `SchComponent`: total number of parts.
- `owner_part_id` on child primitives: which part this primitive belongs to
  (1-based). Part ID 0 means "common to all parts".

### Display Modes

Components can have alternate display representations:
- `display_mode_count` on `SchComponent`: number of display modes.
- `owner_part_display_mode` on child primitives: which display mode this
  primitive appears in. Mode 0 means "common to all modes".

---

## Key Record Types

### SchComponent (Record 1)

The root container for a schematic symbol. Every other primitive in the symbol
is a child of this record.

```rust
// crates/altium-format/src/records/sch/component.rs
pub struct SchComponent {
    pub graphical: SchGraphicalBase,       // position, color (flattened)
    pub lib_reference: String,             // LIBREFERENCE — component name in library
    pub component_description: String,     // COMPONENTDESCRIPTION
    pub unique_id: String,                 // UNIQUEID
    pub current_part_id: i32,              // CURRENTPARTID
    pub part_count: i32,                   // PARTCOUNT (stored as value+1 in file)
    pub display_mode_count: i32,           // DISPLAYMODECOUNT
    pub display_mode: i32,                 // DISPLAYMODE
    pub show_hidden_pins: bool,            // SHOWHIDDENPINS
    pub library_path: String,              // LIBRARYPATH (default: "*")
    pub source_library_name: String,       // SOURCELIBRARYNAME
    pub sheet_part_filename: String,       // SHEETPARTFILENAME
    pub target_filename: String,           // TARGETFILENAME
    pub override_colors: bool,             // OVERRIDECOLORS
    pub designator_locked: bool,           // DESIGNATORLOCKED
    pub part_id_locked: bool,              // PARTIDLOCKED
    pub component_kind: i32,               // COMPONENTKIND
    pub alias_list: String,                // ALIASLIST
    pub orientation: TextOrientations,     // ORIENTATION (bitmask: ROTATED | FLIPPED)
    pub unknown_params: UnknownFields,     // preserves unrecognized parameters
}
```

Example parameter string:
```
|RECORD=1|LIBREFERENCE=Resistor|COMPONENTDESCRIPTION=1k Ohm|
UNIQUEID=ABCDEF12|CURRENTPARTID=1|PARTCOUNT=2|DISPLAYMODECOUNT=1|
LOCATION.X=500|LOCATION.Y=300|COLOR=128|
```

### SchPin (Record 2)

An electrical connection point on a component symbol. This is one of the most
important record types — it defines how a symbol connects to nets.

```rust
// crates/altium-format/src/records/sch/pin.rs
pub struct SchPin {
    pub graphical: SchGraphicalBase,           // position, color (flattened)
    pub symbol_inner_edge: PinSymbol,          // SYMBOL_INNEREDGE
    pub symbol_outer_edge: PinSymbol,          // SYMBOL_OUTEREDGE
    pub symbol_inside: PinSymbol,              // SYMBOL_INSIDE
    pub symbol_outside: PinSymbol,             // SYMBOL_OUTSIDE
    pub symbol_line_width: LineWidth,          // SYMBOL_LINEWIDTH
    pub description: String,                   // DESCRIPTION
    pub formal_type: i32,                      // FORMALTYPE
    pub electrical: PinElectricalType,         // ELECTRICAL (0-7)
    pub pin_conglomerate: PinConglomerateFlags, // PINCONGLOMERATE (bitmask)
    pub pin_length: i32,                       // PINLENGTH + PINLENGTH_FRAC (DXP fractional)
    pub name: String,                          // NAME (pin function name, e.g., "VCC")
    pub designator: String,                    // DESIGNATOR (pin number, e.g., "1")
    pub swap_id_group: String,                 // SWAPIDGROUP (for pin swapping)
    pub swap_id_part: i32,                     // SWAPIDPART
    pub swap_id_sequence: String,              // SWAPIDSEQUENCE
    pub hidden_net_name: String,               // HIDDENNETNAME
    pub default_value: String,                 // DEFAULTVALUE
    pub pin_propagation_delay: f64,            // PINPROPAGATIONDELAY
    pub unique_id: String,                     // UNIQUEID
}
```

#### Pin Electrical Types

The `ELECTRICAL` parameter maps to:

| Value | Variant | Meaning |
|-------|---------|---------|
| 0 | `Input` | Signal input |
| 1 | `InputOutput` | Bidirectional |
| 2 | `Output` | Signal output |
| 3 | `OpenCollector` | Open-collector output |
| 4 | `Passive` | Passive (default) — resistors, capacitors |
| 5 | `HiZ` | High-impedance |
| 6 | `OpenEmitter` | Open-emitter output |
| 7 | `Power` | Power supply pin |

#### Pin Symbols

Pins have four symbol positions (inner edge, outer edge, inside, outside) that
control graphical indicators:

| Value | Variant | Visual |
|-------|---------|--------|
| 0 | `None` | No symbol (default) |
| 1 | `Dot` | Inversion dot |
| 3 | `Clock` | Clock edge indicator |
| 4 | `ActiveLowInput` | Active-low input bar |
| 9 | `OpenCollector` | Open-collector symbol |
| 10 | `HiZ` | High-impedance symbol |
| 13 | `Schmitt` | Schmitt trigger symbol |
| 33 | `LeftRightSignalFlow` | Signal flow arrow |
| … | … | (see `PinSymbol` enum for full list) |

#### Pin Conglomerate Flags

The `PINCONGLOMERATE` parameter is a bitmask:

| Bit | Meaning |
|-----|---------|
| HIDE | Pin is hidden |
| DISPLAY_NAME_VISIBLE | Show pin name text |
| DESIGNATOR_VISIBLE | Show pin number text |
| ROTATED | Pin orientation rotated |
| FLIPPED | Pin orientation flipped |

#### Example Parameter String

```
|RECORD=2|OWNERINDEX=0|LOCATION.X=100|LOCATION.Y=200|COLOR=128|
SYMBOL_INNEREDGE=0|SYMBOL_OUTEREDGE=0|SYMBOL_INSIDE=0|SYMBOL_OUTSIDE=0|
SYMBOL_LINEWIDTH=0|DESCRIPTION=|FORMALTYPE=0|ELECTRICAL=7|PINCONGLOMERATE=0|
PINLENGTH=100|PINLENGTH_FRAC=0|NAME=VCC|DESIGNATOR=1|SWAPIDPART=0|
PINPROPAGATIONDELAY=0.000000|UNIQUEID=ABCD1234|
```

### SchWire (Record 27)

An electrical wire connecting two or more points. Uses indexed vertex
coordinates for multi-segment wires.

```rust
// crates/altium-format/src/records/sch/wire.rs
pub struct SchWire {
    pub graphical: SchGraphicalBase,     // position, color (flattened)
    pub line_width: LineWidth,           // LINEWIDTH
    pub line_style: LineStyle,           // LINESTYLE
    pub vertices: Vec<(i32, i32)>,       // X1/Y1, X2/Y2, … (indexed coordinates)
    pub unknown_params: UnknownFields,
}
```

Example:
```
|RECORD=27|OWNERINDEX=0|LINEWIDTH=1|LOCATIONCOUNT=2|
X1=100|Y1=200|X2=300|Y2=200|
```

### SchNetLabel (Record 25)

Names a net. Extends `SchLabel` by composition:

```rust
// crates/altium-format/src/records/sch/netlabel.rs
pub struct SchNetLabel {
    pub label: SchLabel,               // flattened — inherits all label fields
    pub unknown_params: UnknownFields,
}
```

The `SchLabel` (Record 4) provides the text, font, orientation, and
justification fields:

```rust
// crates/altium-format/src/records/sch/label.rs
pub struct SchLabel {
    pub graphical: SchGraphicalBase,
    pub orientation: TextOrientations,    // ORIENTATION
    pub justification: TextJustification, // JUSTIFICATION
    pub font_id: i32,                     // FONTID
    pub text: String,                     // TEXT
    pub is_mirrored: bool,                // ISMIRRORED
    pub url: String,                      // URL
    pub unknown_params: UnknownFields,
}
```

### SchPowerObject (Record 17)

Power and ground symbols. These connect to power nets.

```rust
// crates/altium-format/src/records/sch/power.rs
pub struct SchPowerObject {
    pub graphical: SchGraphicalBase,
    pub style: PowerObjectStyle,          // STYLE (bar, circle, wave, arrow, etc.)
    pub orientation: TextOrientations,    // ORIENTATION
    pub text: String,                     // TEXT (net name, e.g., "VCC", "GND")
    pub show_net_name: bool,              // SHOWNETNAME
    pub font_id: i32,                     // FONTID
    pub unknown_params: UnknownFields,
}
```

### SchRectangle (Record 14)

A filled or outlined rectangle, commonly used as the body of a component symbol.

Parameters include `LOCATION.X`, `LOCATION.Y` (bottom-left corner),
`CORNER.X`, `CORNER.Y` (top-right corner), `COLOR`, `AREACOLOR`,
`ISSOLID`, `LINEWIDTH`, and `TRANSPARENT`.

### SchDesignator (Record 34)

The reference designator text (e.g., "U1", "R3") shown on a component. Inherits
label-like text properties and is always a child of a `SchComponent`.

### SchParameter (Record 41)

User-defined key-value parameters attached to components (e.g., "Value=10k",
"Tolerance=5%"). Stored as children of `SchComponent` with their own
`OWNERINDEX`.

### SchImplementation (Record 45) and Related

Records 44-48 handle footprint assignments:

- **Record 44** (`SchImplementationList`): Container that groups implementation
  records under a component.
- **Record 45** (`SchImplementation`): A single footprint assignment (e.g.,
  "0402" package).
- **Record 46** (`SchMapDefinerList`): Container for pin-to-pad mappings.
- **Record 47** (`SchMapDefiner`): Maps a schematic pin to a PCB pad.
- **Record 48** (`SchImplementationParameters`): Additional footprint
  parameters.

These form a hierarchy:
```
SchComponent
└── SchImplementationList (OWNERINDEX → component)
    └── SchImplementation (OWNERINDEX → impl list)
        ├── SchMapDefiner (OWNERINDEX → implementation)
        └── SchImplementationParameters (OWNERINDEX → implementation)
```
