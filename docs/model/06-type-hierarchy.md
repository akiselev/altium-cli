# Type Hierarchy Design

Complete type hierarchy for all Altium primitives/records in the `altium-format` crate.

**Design Philosophy**: Fail fast, fail hard. No round-trip preservation, no unknown field
capture, no opaque blobs. If our parser encounters data it doesn't understand, that is a
bug in our code that must be fixed -- never silently skipped. These files control PCB
fabrication; a silently dropped field could cost thousands of dollars.

---

## 1. Cross-Domain Overlap Analysis

### 1.1 Shared Concepts

| Concept | Schematic | PCB | Shareable? |
|---------|-----------|-----|------------|
| Coordinate unit | `Coord(i32)`, 10k units/mil | `Coord(i32)`, 10k units/mil | Yes -- identical semantics |
| Point | `CoordPoint { x, y }` | `CoordPoint { x, y }` | Yes |
| Bounding box | `BoundingBox { min, max }` | `BoundingBox { min, max }` | Yes |
| Color | Win32 COLORREF `0x00BBGGRR` as i32 | Win32 COLORREF `0x00BBGGRR` as i32 | Yes |
| UniqueID | 8-char `[A-Z]` string | 8-char `[A-Z]` string | Yes |
| Container format | CFB/OLE via `cfb` crate | CFB/OLE via `cfb` crate | Yes |
| Block framing | 24-bit size + 8-bit flags | 24-bit size + 8-bit flags | Yes |
| Component kind | `ComponentKind` (0-6) | `ComponentKind` (0-6) | Yes -- identical enum |
| Record dispatch | `RECORD=N` param (i32) | `object_id` byte (u8) | No -- different ID spaces |
| Serialization | Pipe-delimited key=value text | Packed binary little-endian | No |
| Ownership | OWNERINDEX tree (i32 index) | Flat component index (i16) | No |
| Layers | N/A (schematic is layerless) | V6 byte / V7 u32 structured | PCB-only |
| Nets | Implicit via wire connectivity | Explicit net index (i16) | Different models |

### 1.2 Overlap Diagram

```
                    SHARED                          DOMAIN-SPECIFIC
              +------------------+          +---------------------------+
              |   Coord(i32)     |          | SCH: OWNERINDEX tree      |
              |   CoordPoint     |          | SCH: RECORD dispatch      |
              |   BoundingBox    |          | SCH: Parameter encoding   |
              |   Color          |          | SCH: DXP fractional coords|
              |   UniqueId       |          | SCH: Font table           |
              |   ComponentKind  |          | SCH: Display modes/parts  |
              |   CFB container  |          +---------------------------+
              |   Block framing  |          | PCB: Binary encoding      |
              +------------------+          | PCB: Layer system (V6/V7) |
                                            | PCB: Net indices          |
                                            | PCB: Pad stack modes      |
                                            | PCB: Sidecar TLV streams  |
                                            | PCB: Section-per-type     |
                                            +---------------------------+
```

### 1.3 Decision: Separate Enums Per Domain

The schematic and PCB record type ID spaces are completely disjoint:
- SCH uses `RECORD=N` with values 1-241 (sparse, ~50 defined)
- PCB uses `object_id` byte with values 0-26

Zero overlap in record semantics. The correct design is **separate enums per domain**
with shared foundational types.

---

## 2. Foundational Shared Types

### 2.1 Coordinates

```rust
/// Fixed-point coordinate: 10,000 internal units = 1 mil (0.001 inch).
///
/// 1 mil = 10,000 units
/// 1 mm  = ~393,701 units
/// 1 inch = 10,000,000 units
/// Range: approximately +/- 214,748 mils = +/- 5,454 mm
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Coord(i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct CoordPoint {
    pub x: Coord,
    pub y: Coord,
}

/// Axis-aligned bounding box defined by min and max corners.
/// Invariant: min.x <= max.x && min.y <= max.y (enforced at construction).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BoundingBox {
    min: CoordPoint,
    max: CoordPoint,
}
```

**Design notes:**
- Newtype `Coord(i32)` prevents accidental mixing with raw integers.
- `BoundingBox` uses min/max with enforced invariant. Named `BoundingBox` instead of
  `CoordRect` because Altium uses "rect" to mean different things in different contexts
  (some rects use `location + corner`, others use `p1 + p2`).
- Full arithmetic support on `Coord`: `Add`, `Sub`, `Neg`, `Mul<i32>`, `Div<i32>`.

### 2.2 Color

```rust
/// Win32 COLORREF: 0x00BBGGRR format stored as i32.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Color(i32);

impl Color {
    pub const BLACK: Self = Self(0x00000000);
    pub const WHITE: Self = Self(0x00FFFFFF);

    pub fn new(colorref: i32) -> Self { Self(colorref) }
    pub fn raw(self) -> i32 { self.0 }
    pub fn from_rgb(r: u8, g: u8, b: u8) -> Self { ... }
    pub fn r(self) -> u8 { ... }
    pub fn g(self) -> u8 { ... }
    pub fn b(self) -> u8 { ... }
}
```

### 2.3 UniqueId

```rust
/// 8-character uppercase alphabetic identifier (e.g., "LVUUGVHQ").
/// Validated at parse time; construction from arbitrary strings is fallible.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UniqueId(String);
```

### 2.4 Component Kind (shared enum)

```rust
/// Component classification, shared between schematic and PCB.
/// All enums are #[non_exhaustive] -- unknown values are parse errors, but
/// future Altium versions may add new variants that we then add here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum ComponentKind {
    #[default]
    Standard = 0,
    Mechanical = 1,
    Graphical = 2,
    NetTieBom = 3,
    NetTieNoBom = 4,
    StandardNoBom = 5,
    Jumper = 6,
}
```

---

## 3. Schematic Type Hierarchy

### 3.1 SchRecordType Enum

Maps `RECORD=N` values to Rust types. The binary record codes come from
`SchDataUtils.GetBinaryCodeByObjectId` (see doc 04). All values are the RECORD=N
parameter in the pipe-delimited text format.

**Critical**: This enum is `#[non_exhaustive]` but has NO catch-all/unknown variant.
An unrecognized RECORD value is a parse error -- it means our code is incomplete.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[repr(i32)]
pub enum SchRecordType {
    Component = 1,
    Pin = 2,
    Symbol = 3,
    Label = 4,
    Bezier = 5,
    Polyline = 6,
    Polygon = 7,
    Ellipse = 8,
    Pie = 9,
    RoundRectangle = 10,
    EllipticalArc = 11,
    Arc = 12,
    Line = 13,
    Rectangle = 14,
    SheetSymbol = 15,
    SheetEntry = 16,
    PowerObject = 17,
    Port = 18,
    NoErc = 22,
    ErrorMarker = 23,
    NetLabel = 25,
    Bus = 26,
    Wire = 27,
    TextFrame = 28,
    Junction = 29,
    Image = 30,
    Sheet = 31,
    SheetName = 32,
    SheetFileName = 33,
    Designator = 34,
    BusEntry = 37,
    Template = 39,
    TaskHolder = 40,
    Parameter = 41,
    ParameterSet = 43,
    ImplementationList = 44,
    Implementation = 45,
    ImplementationMap = 46,
    MapDefiner = 47,
    ParameterList = 48,
    // --- Harness records (104-138) ---
    HarnessWiringDiagram = 104,
    HarnessLayoutDrawing = 105,
    HarnessComponent = 106,
    HarnessWire = 107,
    HarnessSplice = 108,
    HarnessLayoutLabel = 109,
    HarnessLayoutConnectionPoint = 110,
    HarnessBundle = 111,
    HarnessLogicalSignal = 112,
    HarnessPin = 113,
    HarnessWireLabel = 114,
    HarnessWireData = 115,
    HarnessSpliceData = 116,
    HarnessShield = 117,
    HarnessTwist = 118,
    HarnessNoConnect = 119,
    HarnessNoConnectData = 120,
    HarnessShieldData = 121,
    HarnessTwistData = 122,
    HarnessCable = 123,
    HarnessCableData = 124,
    HarnessAssociatedParts = 125,
    LineView = 126,
    HarnessLibrary = 127,
    HarnessCovering = 128,
    ObjectDefinition = 129,
    HarnessWireBreak = 130,
    AssociatedObjects = 131,
    ElectronicsSystemDesignDocument = 132,
    FunctionalBlock = 133,
    FunctionalConnectionLine = 134,
    FunctionalTextFrame = 135,
    SchematicBlock = 136,
    ReuseSheetSymbol = 137,
    ReuseBlockImplementationInfo = 138,
    // --- Extended records (200+) ---
    SchLib = 200,
    Note = 209,
    Probe = 210,
    CompileMask = 211,
    HarnessConnector = 215,
    HarnessEntry = 216,
    HarnessConnectorType = 217,
    SignalHarness = 218,
    HighLevelCodeSymbol = 220,
    HighLevelCodeEntry = 221,
    Blanket = 225,
    Hyperlink = 226,
    RichTextDocument = 240,
    RtfLink = 241,
}
```

### 3.2 SchRecord Enum (Polymorphic Container)

This is the primary dispatch type. **No `Unknown` variant** -- encountering an
unrecognized record type is an error.

```rust
/// A deserialized schematic record.
pub(crate) enum SchRecord {
    Component(SchComponent),
    Pin(SchPin),
    Symbol(SchSymbol),
    Label(SchLabel),
    Bezier(SchBezier),
    Polyline(SchPolyline),
    Polygon(SchPolygon),
    Ellipse(SchEllipse),
    Pie(SchPie),
    RoundRectangle(SchRoundRectangle),
    EllipticalArc(SchEllipticalArc),
    Arc(SchArc),
    Line(SchLine),
    Rectangle(SchRectangle),
    SheetSymbol(SchSheetSymbol),
    SheetEntry(SchSheetEntry),
    PowerObject(SchPowerObject),
    Port(SchPort),
    NoErc(SchNoErc),
    ErrorMarker(SchErrorMarker),
    NetLabel(SchNetLabel),
    Bus(SchBus),
    Wire(SchWire),
    TextFrame(SchTextFrame),
    Junction(SchJunction),
    Image(SchImage),
    Sheet(SchSheet),
    SheetName(SchSheetName),
    SheetFileName(SchSheetFileName),
    Designator(SchDesignator),
    BusEntry(SchBusEntry),
    Template(SchTemplate),
    TaskHolder(SchTaskHolder),
    Parameter(SchParameter),
    ParameterSet(SchParameterSet),
    ImplementationList(SchImplementationList),
    Implementation(SchImplementation),
    ImplementationMap(SchImplementationMap),
    MapDefiner(SchMapDefiner),
    ParameterList(SchParameterList),
    Note(SchNote),
    Probe(SchProbe),
    CompileMask(SchCompileMask),
    HarnessConnector(SchHarnessConnector),
    HarnessEntry(SchHarnessEntry),
    HarnessConnectorType(SchHarnessConnectorType),
    SignalHarness(SchSignalHarness),
    Blanket(SchBlanket),
    Hyperlink(SchHyperlink),
    // Harness/extended variants added as implemented...
}
```

### 3.3 Schematic Base Types (Composition via Flatten)

```rust
/// Common fields for all schematic primitives.
/// Every schematic record embeds this (flattened during serialization).
#[derive(Debug, Clone, Default)]
pub(crate) struct SchPrimitiveBase {
    pub owner_index: i32,             // OWNERINDEX (-1 = root)
    pub is_not_accessible: bool,      // ISNOTACCESIBLE [sic -- Altium's typo]
    pub owner_part_id: i32,           // OWNERPARTID (-1 = all parts)
    pub owner_part_display_mode: i32, // OWNERPARTDISPLAYMODE (0 = all modes)
    pub graphically_locked: bool,     // GRAPHICALLYLOCKED
    pub index_in_sheet: i32,          // INDEXINSHEET
    pub unique_id: String,            // UNIQUEID
}

/// Position + color fields shared by most graphical schematic primitives.
/// Extends SchPrimitiveBase via composition.
#[derive(Debug, Clone, Default)]
pub(crate) struct SchGraphicalBase {
    pub base: SchPrimitiveBase,  // flattened
    pub location: CoordPoint,    // LOCATION.X + _FRAC, LOCATION.Y + _FRAC
    pub color: Color,            // COLOR
    pub area_color: Color,       // AREACOLOR
}
```

### 3.4 Schematic Record Structs (All Types)

Every field is concrete (no `Option` unless the field is genuinely optional in Altium).
No `unknown_fields`, no `UnknownFields`, no opaque passthrough. If we encounter a
parameter we don't know about, that's a parse error.

#### SchComponent (RECORD=1)

```rust
#[derive(Debug, Clone)]
pub(crate) struct SchComponent {
    pub base: SchGraphicalBase,
    pub lib_reference: String,
    pub component_description: String,
    pub current_part_id: i32,
    pub part_count: i32,              // stored as value+1 in file
    pub display_mode_count: i32,
    pub display_mode: i32,
    pub show_hidden_pins: bool,
    pub library_path: String,         // default: "*"
    pub source_library_name: String,
    pub target_filename: String,
    pub sheet_part_filename: String,
    pub override_colors: bool,
    pub designator_locked: bool,
    pub part_id_locked: bool,
    pub pins_moveable: bool,
    pub component_kind: ComponentKind,
    pub orientation: RotationBy90,
    pub is_mirrored: bool,
    pub alias_list: String,
    pub all_pin_count: i32,
    pub design_item_id: String,
    pub database_table_name: String,
    pub footprint: String,
    pub show_hidden_fields: bool,
}
```

#### SchPin (RECORD=2)

```rust
#[derive(Debug, Clone)]
pub(crate) struct SchPin {
    pub base: SchGraphicalBase,
    pub name: String,
    pub designator: String,
    pub description: String,
    pub electrical: PinElectricalType,
    pub pin_length: Coord,
    pub orientation: RotationBy90,
    pub hidden: bool,
    pub show_name: bool,
    pub show_designator: bool,
    pub symbol_inner_edge: IeeeSymbol,
    pub symbol_outer_edge: IeeeSymbol,
    pub symbol_inside: IeeeSymbol,
    pub symbol_outside: IeeeSymbol,
    pub symbol_line_width: PenWidth,
    pub formal_type: StdLogicState,
    pub hidden_net_name: String,
    pub default_value: String,
    pub swap_id_group: String,
    pub swap_id_part: String,
    pub swap_id_sequence: String,
    pub pin_propagation_delay: f64,
    pub pin_package_length: i32,
    pub pin_conglomerate: i32,        // bitmask: hide, name_visible, desg_visible, etc.
}
```

#### SchWire (RECORD=27)

```rust
#[derive(Debug, Clone)]
pub(crate) struct SchWire {
    pub base: SchGraphicalBase,
    pub line_width: PenWidth,
    pub line_style: LineStyle,
    pub vertices: Vec<CoordPoint>,    // indexed: X1/Y1, X2/Y2, ..., count=LOCATIONCOUNT
}
```

#### SchBus (RECORD=26)

```rust
#[derive(Debug, Clone)]
pub(crate) struct SchBus {
    pub base: SchGraphicalBase,
    pub line_width: PenWidth,
    pub vertices: Vec<CoordPoint>,
}
```

#### SchRectangle (RECORD=14)

```rust
#[derive(Debug, Clone)]
pub(crate) struct SchRectangle {
    pub base: SchGraphicalBase,
    pub corner: CoordPoint,           // CORNER.X + _FRAC, CORNER.Y + _FRAC
    pub line_width: PenWidth,
    pub is_solid: bool,
    pub transparent: bool,
}
```

#### SchRoundRectangle (RECORD=10)

```rust
#[derive(Debug, Clone)]
pub(crate) struct SchRoundRectangle {
    pub base: SchGraphicalBase,
    pub corner: CoordPoint,
    pub corner_radius_x: Coord,
    pub corner_radius_y: Coord,
    pub line_width: PenWidth,
    pub is_solid: bool,
    pub transparent: bool,
}
```

#### SchLabel (RECORD=4)

```rust
#[derive(Debug, Clone)]
pub(crate) struct SchLabel {
    pub base: SchGraphicalBase,
    pub text: String,
    pub font_id: i32,
    pub orientation: RotationBy90,
    pub justification: TextJustification,
    pub is_mirrored: bool,
}
```

#### SchNetLabel (RECORD=25)

```rust
#[derive(Debug, Clone)]
pub(crate) struct SchNetLabel {
    pub base: SchGraphicalBase,
    pub text: String,
    pub font_id: i32,
    pub orientation: RotationBy90,
    pub justification: TextJustification,
}
```

#### SchPowerObject (RECORD=17)

```rust
#[derive(Debug, Clone)]
pub(crate) struct SchPowerObject {
    pub base: SchGraphicalBase,
    pub text: String,
    pub style: PowerObjectStyle,
    pub show_net_name: bool,
    pub orientation: RotationBy90,
    pub font_id: i32,
    pub is_mirrored: bool,
}
```

#### SchPort (RECORD=18)

```rust
#[derive(Debug, Clone)]
pub(crate) struct SchPort {
    pub base: SchGraphicalBase,
    pub name: String,
    pub style: PortArrowStyle,
    pub io_type: PortIoType,
    pub alignment: TextJustification,
    pub width: Coord,
    pub height: Coord,
    pub font_id: i32,
    pub text_color: Color,
    pub border_width: PenWidth,
    pub harness_type: String,
}
```

#### SchArc (RECORD=12)

```rust
#[derive(Debug, Clone)]
pub(crate) struct SchArc {
    pub base: SchGraphicalBase,
    pub radius: Coord,
    pub start_angle: f64,
    pub end_angle: f64,
    pub line_width: PenWidth,
}
```

#### SchEllipticalArc (RECORD=11)

```rust
#[derive(Debug, Clone)]
pub(crate) struct SchEllipticalArc {
    pub base: SchGraphicalBase,
    pub radius: Coord,
    pub secondary_radius: Coord,
    pub start_angle: f64,
    pub end_angle: f64,
    pub line_width: PenWidth,
}
```

#### SchEllipse (RECORD=8)

```rust
#[derive(Debug, Clone)]
pub(crate) struct SchEllipse {
    pub base: SchGraphicalBase,
    pub radius: Coord,
    pub secondary_radius: Coord,
    pub line_width: PenWidth,
    pub is_solid: bool,
}
```

#### SchPie (RECORD=9)

```rust
#[derive(Debug, Clone)]
pub(crate) struct SchPie {
    pub base: SchGraphicalBase,
    pub radius: Coord,
    pub start_angle: f64,
    pub end_angle: f64,
    pub line_width: PenWidth,
    pub is_solid: bool,
}
```

#### SchLine (RECORD=13)

```rust
#[derive(Debug, Clone)]
pub(crate) struct SchLine {
    pub base: SchGraphicalBase,
    pub corner: CoordPoint,           // end point
    pub line_width: PenWidth,
    pub line_style: LineStyle,
    pub start_shape: LineShape,
    pub end_shape: LineShape,
}
```

#### SchPolyline (RECORD=6)

```rust
#[derive(Debug, Clone)]
pub(crate) struct SchPolyline {
    pub base: SchGraphicalBase,
    pub line_width: PenWidth,
    pub line_style: LineStyle,
    pub start_shape: LineShape,
    pub end_shape: LineShape,
    pub shape_size: PenWidth,
    pub vertices: Vec<CoordPoint>,
}
```

#### SchPolygon (RECORD=7)

```rust
#[derive(Debug, Clone)]
pub(crate) struct SchPolygon {
    pub base: SchGraphicalBase,
    pub line_width: PenWidth,
    pub is_solid: bool,
    pub transparent: bool,
    pub vertices: Vec<CoordPoint>,
}
```

#### SchBezier (RECORD=5)

```rust
#[derive(Debug, Clone)]
pub(crate) struct SchBezier {
    pub base: SchGraphicalBase,
    pub line_width: PenWidth,
    pub vertices: Vec<CoordPoint>,
}
```

#### SchSymbol (RECORD=3)

```rust
#[derive(Debug, Clone)]
pub(crate) struct SchSymbol {
    pub base: SchGraphicalBase,
    pub symbol: i32,                  // IEEE symbol type ID
    pub scale_factor: i32,
    pub is_solid: bool,
}
```

#### SchJunction (RECORD=29)

```rust
#[derive(Debug, Clone)]
pub(crate) struct SchJunction {
    pub base: SchGraphicalBase,
    pub locked: bool,
}
```

#### SchNoErc (RECORD=22)

```rust
#[derive(Debug, Clone)]
pub(crate) struct SchNoErc {
    pub base: SchGraphicalBase,
    pub active: bool,
    pub suppress_all: bool,
}
```

#### SchErrorMarker (RECORD=23)

```rust
#[derive(Debug, Clone)]
pub(crate) struct SchErrorMarker {
    pub base: SchGraphicalBase,
    pub error_kind: i32,
}
```

#### SchImage (RECORD=30)

```rust
#[derive(Debug, Clone)]
pub(crate) struct SchImage {
    pub base: SchGraphicalBase,
    pub corner: CoordPoint,
    pub keep_aspect: bool,
    pub embedded: bool,
    pub filename: String,
}
```

#### SchTextFrame (RECORD=28)

```rust
#[derive(Debug, Clone)]
pub(crate) struct SchTextFrame {
    pub base: SchGraphicalBase,
    pub corner: CoordPoint,
    pub text: String,
    pub font_id: i32,
    pub word_wrap: bool,
    pub show_border: bool,
    pub alignment: TextJustification,
    pub line_width: PenWidth,
    pub is_solid: bool,
    pub transparent: bool,
}
```

#### SchSheet (RECORD=31)

```rust
#[derive(Debug, Clone)]
pub(crate) struct SchSheet {
    pub base: SchPrimitiveBase,
    pub font_id_count: i32,
    pub fonts: Vec<SchFont>,
    pub sheet_style: SheetStyle,
    pub sheet_orientation: SheetOrientation,
    pub border_on: bool,
    pub reference_zones_on: bool,
    pub title_block_on: bool,
    pub document_border_style: SheetBorderStyle,
    pub snap_grid_on: bool,
    pub snap_grid_size: Coord,
    pub visible_grid_on: bool,
    pub visible_grid_size: Coord,
    pub custom_x: Coord,
    pub custom_y: Coord,
    pub use_custom_sheet: bool,
    pub display_unit: i32,
    pub minor_version: i32,
    pub show_template_graphics: bool,
    pub template_filename: String,
    pub system_font: i32,
}
```

#### SchParameter (RECORD=41)

```rust
#[derive(Debug, Clone)]
pub(crate) struct SchParameter {
    pub base: SchGraphicalBase,
    pub name: String,
    pub text: String,
    pub font_id: i32,
    pub orientation: RotationBy90,
    pub justification: TextJustification,
    pub is_hidden: bool,
    pub show_name: bool,
    pub auto_position: i32,
    pub is_mirrored: bool,
    pub read_only_state: i32,
}
```

#### SchDesignator (RECORD=34)

```rust
#[derive(Debug, Clone)]
pub(crate) struct SchDesignator {
    pub base: SchGraphicalBase,
    pub name: String,
    pub text: String,
    pub font_id: i32,
    pub orientation: RotationBy90,
    pub justification: TextJustification,
    pub is_hidden: bool,
    pub show_name: bool,
    pub auto_position: i32,
    pub is_mirrored: bool,
}
```

#### SchBusEntry (RECORD=37)

```rust
#[derive(Debug, Clone)]
pub(crate) struct SchBusEntry {
    pub base: SchGraphicalBase,
    pub corner: CoordPoint,
}
```

#### SchSheetSymbol (RECORD=15)

```rust
#[derive(Debug, Clone)]
pub(crate) struct SchSheetSymbol {
    pub base: SchGraphicalBase,
    pub x_size: Coord,
    pub y_size: Coord,
    pub filename: String,
    pub is_solid: bool,
}
```

#### SchSheetEntry (RECORD=16)

```rust
#[derive(Debug, Clone)]
pub(crate) struct SchSheetEntry {
    pub base: SchGraphicalBase,
    pub name: String,
    pub io_type: PortIoType,
    pub side: i32,                    // which side of parent sheet symbol
    pub distance_from_top: Coord,
    pub font_id: i32,
    pub text_color: Color,
    pub arrow_kind: PortArrowStyle,
}
```

#### SchSheetName (RECORD=32) / SchSheetFileName (RECORD=33)

```rust
#[derive(Debug, Clone)]
pub(crate) struct SchSheetName {
    pub base: SchGraphicalBase,
    pub text: String,
    pub font_id: i32,
    pub orientation: RotationBy90,
}

#[derive(Debug, Clone)]
pub(crate) struct SchSheetFileName {
    pub base: SchGraphicalBase,
    pub text: String,
    pub font_id: i32,
    pub orientation: RotationBy90,
}
```

#### Implementation Records (44-48)

```rust
#[derive(Debug, Clone)]
pub(crate) struct SchImplementationList {
    pub base: SchPrimitiveBase,
}

#[derive(Debug, Clone)]
pub(crate) struct SchImplementation {
    pub base: SchPrimitiveBase,
    pub model_name: String,
    pub model_type: String,           // "PCBLIB", "SIM", "SI"
    pub is_current: bool,
    pub data_links_locked: bool,
    pub database_model: bool,
    pub interface_name: String,
}

#[derive(Debug, Clone)]
pub(crate) struct SchImplementationMap {
    pub base: SchPrimitiveBase,
}

#[derive(Debug, Clone)]
pub(crate) struct SchMapDefiner {
    pub base: SchPrimitiveBase,
    pub des_intf: String,
    pub des_imp_count: i32,
    pub des_imp0: String,
}

#[derive(Debug, Clone)]
pub(crate) struct SchParameterList {
    pub base: SchPrimitiveBase,
}
```

#### Additional Records

```rust
#[derive(Debug, Clone)]
pub(crate) struct SchTemplate {
    pub base: SchPrimitiveBase,
    pub filename: String,
}

#[derive(Debug, Clone)]
pub(crate) struct SchTaskHolder {
    pub base: SchPrimitiveBase,
    pub process: String,
    pub instance_name: String,
}

#[derive(Debug, Clone)]
pub(crate) struct SchParameterSet {
    pub base: SchGraphicalBase,
    pub style: i32,
}

#[derive(Debug, Clone)]
pub(crate) struct SchNote {
    pub base: SchGraphicalBase,
    pub corner: CoordPoint,
    pub text: String,
    pub font_id: i32,
    pub author: String,
    pub collapsed: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct SchProbe {
    pub base: SchGraphicalBase,
}

#[derive(Debug, Clone)]
pub(crate) struct SchCompileMask {
    pub base: SchGraphicalBase,
}

#[derive(Debug, Clone)]
pub(crate) struct SchHarnessConnector {
    pub base: SchGraphicalBase,
    pub x_size: Coord,
    pub y_size: Coord,
}

#[derive(Debug, Clone)]
pub(crate) struct SchHarnessEntry {
    pub base: SchGraphicalBase,
    pub name: String,
    pub side: i32,
    pub distance_from_top: Coord,
}

#[derive(Debug, Clone)]
pub(crate) struct SchHarnessConnectorType {
    pub base: SchGraphicalBase,
}

#[derive(Debug, Clone)]
pub(crate) struct SchSignalHarness {
    pub base: SchGraphicalBase,
    pub line_width: PenWidth,
    pub vertices: Vec<CoordPoint>,
}

#[derive(Debug, Clone)]
pub(crate) struct SchBlanket {
    pub base: SchGraphicalBase,
    pub name: String,
    pub vertices: Vec<CoordPoint>,
}

#[derive(Debug, Clone)]
pub(crate) struct SchHyperlink {
    pub base: SchGraphicalBase,
    pub text: String,
    pub url: String,
    pub font_id: i32,
}
```

### 3.5 Schematic Enums

All enums are `#[non_exhaustive]` but have NO catch-all/unknown variant.
An unknown discriminant value is always a parse error.

```rust
/// Pin electrical type (0-7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum PinElectricalType {
    Input = 0,
    InputOutput = 1,
    Output = 2,
    OpenCollector = 3,
    #[default]
    Passive = 4,
    HiZ = 5,
    OpenEmitter = 6,
    Power = 7,
}

/// Rotation in 90-degree increments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum RotationBy90 {
    #[default]
    Rotate0 = 0,
    Rotate90 = 1,
    Rotate180 = 2,
    Rotate270 = 3,
}

/// IEEE pin symbol types (0-36).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum IeeeSymbol {
    #[default]
    NoSymbol = 0,
    Dot = 1,
    RightLeftSignalFlow = 2,
    Clock = 3,
    ActiveLowInput = 4,
    AnalogSignalIn = 5,
    NotLogicConnection = 6,
    ShiftRight = 7,
    PostponedOutput = 8,
    OpenCollector = 9,
    HiZ = 10,
    HighCurrent = 11,
    Pulse = 12,
    Schmitt = 13,
    Delay = 14,
    GroupLine = 15,
    GroupBin = 16,
    ActiveLowOutput = 17,
    PiSymbol = 18,
    GreaterEqual = 19,
    LessEqual = 20,
    Sigma = 21,
    OpenCollectorPullUp = 22,
    OpenEmitter = 23,
    OpenEmitterPullUp = 24,
    DigitalSignalIn = 25,
    And = 26,
    Invertor = 27,
    Or = 28,
    Xor = 29,
    ShiftLeft = 30,
    InputOutput = 31,
    OpenCircuitOutput = 32,
    LeftRightSignalFlow = 33,
    BidirectionalSignalFlow = 34,
    InternalPullUp = 35,
    InternalPullDown = 36,
}

/// VHDL formal type / std_logic state (0-8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum StdLogicState {
    #[default]
    Uninitialized = 0,
    ForcingUnknown = 1,
    Forcing0 = 2,
    Forcing1 = 3,
    HighZ = 4,
    WeakUnknown = 5,
    Weak0 = 6,
    Weak1 = 7,
    DontCare = 8,
}

/// Pen/border width (0-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum PenWidth {
    #[default]
    Zero = 0,
    Small = 1,
    Medium = 2,
    Large = 3,
}

/// Line style (0-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum LineStyle {
    #[default]
    Solid = 0,
    Dashed = 1,
    Dotted = 2,
    DashDotted = 3,
}

/// Line endpoint shape (0-6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum LineShape {
    #[default]
    None = 0,
    Arrow = 1,
    SolidArrow = 2,
    Tail = 3,
    SolidTail = 4,
    Circle = 5,
    Square = 6,
}

/// Text justification (0-8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum TextJustification {
    #[default]
    BottomLeft = 0,
    BottomCenter = 1,
    BottomRight = 2,
    CenterLeft = 3,
    Center = 4,
    CenterRight = 5,
    TopLeft = 6,
    TopCenter = 7,
    TopRight = 8,
}

/// Power object visual style (0-10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum PowerObjectStyle {
    #[default]
    Circle = 0,
    Arrow = 1,
    Bar = 2,
    Wave = 3,
    GndPower = 4,
    GndSignal = 5,
    GndEarth = 6,
    GostArrow = 7,
    GostGndPower = 8,
    GostGndEarth = 9,
    GostBar = 10,
}

/// Port arrow direction style (0-7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum PortArrowStyle {
    #[default]
    None = 0,
    Left = 1,
    Right = 2,
    LeftRight = 3,
    NoneVertical = 4,
    Top = 5,
    Bottom = 6,
    TopBottom = 7,
}

/// Port I/O direction (0-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum PortIoType {
    #[default]
    Unspecified = 0,
    Output = 1,
    Input = 2,
    Bidirectional = 3,
}

/// Sheet size standard (0-17).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum SheetStyle {
    #[default]
    A4 = 0,
    A3 = 1,
    A2 = 2,
    A1 = 3,
    A0 = 4,
    A = 5,
    B = 6,
    C = 7,
    D = 8,
    E = 9,
    Letter = 10,
    Legal = 11,
    Tabloid = 12,
    OrcadA = 13,
    OrcadB = 14,
    OrcadC = 15,
    OrcadD = 16,
    OrcadE = 17,
}

/// Sheet orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum SheetOrientation {
    #[default]
    Landscape = 0,
    Portrait = 1,
}

/// Sheet border style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum SheetBorderStyle {
    #[default]
    Standard = 0,
    Ansi = 1,
}
```

---

## 4. PCB Type Hierarchy

### 4.1 PcbObjectId Enum

All 27 values from the Delphi/C# `TObjectId`. No catch-all variant -- unknown values
are parse errors.

```rust
/// PCB record type discriminant (byte in binary format).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[repr(u8)]
pub enum PcbObjectId {
    NoObject = 0,
    Arc = 1,
    Pad = 2,
    Via = 3,
    Track = 4,
    Text = 5,
    Fill = 6,
    Connection = 7,
    Net = 8,
    Component = 9,
    Polygon = 10,
    Region = 11,
    ComponentBody = 12,
    Dimension = 13,
    Coordinate = 14,
    Class = 15,
    Rule = 16,
    FromTo = 17,
    DifferentialPair = 18,
    Violation = 19,
    Embedded = 20,
    EmbeddedBoard = 21,
    SplitPlane = 22,
    Trace = 23,
    SpareVia = 24,
    Board = 25,
    BoardOutline = 26,
}
```

### 4.2 PcbRecord Enum (Polymorphic Container)

No `Unknown` variant. An unrecognized object ID is a parse error.

```rust
/// A deserialized PCB primitive record.
pub(crate) enum PcbRecord {
    Arc(PcbArc),
    Pad(PcbPad),
    Via(PcbVia),
    Track(PcbTrack),
    Text(PcbText),
    Fill(PcbFill),
    Component(PcbComponent),
    Polygon(PcbPolygon),
    Region(PcbRegion),
    ComponentBody(PcbComponentBody),
    Dimension(PcbDimension),
    Coordinate(PcbCoordinate),
    BoardOutline(PcbBoardOutline),
    // Non-geometric types stored differently:
    // Net, Class, Rule are in separate sections, not in this enum.
}
```

### 4.3 PCB Base Types

```rust
/// Common header for all PCB binary primitives (19 bytes in V4+ format).
/// Every PCB record embeds this.
#[derive(Debug, Clone, Default)]
pub(crate) struct PcbPrimitiveCommon {
    pub layer: V6Layer,
    pub flags: PcbFlags,
    pub net: i16,
    pub component: i16,
    pub unique_id: String,            // from sidecar or inline
}

/// PCB primitive flags bitmask (u16).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct PcbFlags(u16);

impl PcbFlags {
    pub fn selected(self) -> bool { self.0 & 0x01 != 0 }
    pub fn locked(self) -> bool { self.0 & 0x10 != 0 }
    pub fn union_member(self) -> bool { self.0 & 0x80 != 0 }
    pub fn raw(self) -> u16 { self.0 }
}
```

### 4.4 Layer Types

```rust
/// V6 layer ID (byte, 0-82). Used in binary file format.
/// No catch-all -- unknown layer bytes are parse errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum V6Layer {
    #[default]
    NoLayer = 0,
    TopLayer = 1,
    MidLayer1 = 2,
    MidLayer2 = 3,
    MidLayer3 = 4,
    MidLayer4 = 5,
    MidLayer5 = 6,
    MidLayer6 = 7,
    MidLayer7 = 8,
    MidLayer8 = 9,
    MidLayer9 = 10,
    MidLayer10 = 11,
    MidLayer11 = 12,
    MidLayer12 = 13,
    MidLayer13 = 14,
    MidLayer14 = 15,
    MidLayer15 = 16,
    MidLayer16 = 17,
    MidLayer17 = 18,
    MidLayer18 = 19,
    MidLayer19 = 20,
    MidLayer20 = 21,
    MidLayer21 = 22,
    MidLayer22 = 23,
    MidLayer23 = 24,
    MidLayer24 = 25,
    MidLayer25 = 26,
    MidLayer26 = 27,
    MidLayer27 = 28,
    MidLayer28 = 29,
    MidLayer29 = 30,
    MidLayer30 = 31,
    BottomLayer = 32,
    TopOverlay = 33,
    BottomOverlay = 34,
    TopPaste = 35,
    BottomPaste = 36,
    TopSolder = 37,
    BottomSolder = 38,
    Mechanical1 = 39,
    Mechanical2 = 40,
    Mechanical3 = 41,
    Mechanical4 = 42,
    Mechanical5 = 43,
    Mechanical6 = 44,
    Mechanical7 = 45,
    Mechanical8 = 46,
    Mechanical9 = 47,
    Mechanical10 = 48,
    Mechanical11 = 49,
    Mechanical12 = 50,
    Mechanical13 = 51,
    Mechanical14 = 52,
    Mechanical15 = 53,
    Mechanical16 = 54,
    DrillGuide = 55,
    DrillDrawing = 56,
    InternalPlane1 = 57,
    InternalPlane2 = 58,
    InternalPlane3 = 59,
    InternalPlane4 = 60,
    InternalPlane5 = 61,
    InternalPlane6 = 62,
    InternalPlane7 = 63,
    InternalPlane8 = 64,
    InternalPlane9 = 65,
    InternalPlane10 = 66,
    InternalPlane11 = 67,
    InternalPlane12 = 68,
    InternalPlane13 = 69,
    InternalPlane14 = 70,
    InternalPlane15 = 71,
    InternalPlane16 = 72,
    KeepOutLayer = 73,
    MultiLayer = 74,
    ConnectLayer = 75,
    BackGroundLayer = 76,
    DrcErrorLayer = 77,
    HighlightLayer = 78,
    GridColor1 = 79,
    GridColor10 = 80,
    PadHoleLayer = 81,
    ViaHoleLayer = 82,
}

impl V6Layer {
    /// Signal layers: 1 (top) through 32 (bottom), with 2-31 as mid layers.
    pub fn is_signal(self) -> bool { ... }

    /// Mechanical layers 1-16.
    pub fn is_mechanical(self) -> bool { ... }

    /// Internal plane layers 1-16.
    pub fn is_internal_plane(self) -> bool { ... }

    /// Copper-carrying layers (signal + multi-layer).
    pub fn is_copper(self) -> bool { ... }

    /// Mechanical layer number (1-16) if this is a mechanical layer.
    pub fn mechanical_number(self) -> Option<u8> { ... }

    /// Internal plane number (1-16) if this is an internal plane layer.
    pub fn internal_plane_number(self) -> Option<u8> { ... }
}

/// V7 extended layer ID (32-bit structured).
///
/// Layout (from Delphi/C# struct with explicit field offsets):
/// ```text
/// Byte 0-1 (u16): Species (layer-specific index)
/// Byte 2   (u8):  Genus (layer category)
/// Byte 3   (u8):  Family (copper/dielectric/etc.)
/// ```
///
/// When genus=0 and family=0, the species low byte matches V6 layer IDs
/// (backward-compatible).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct V7Layer(u32);

impl V7Layer {
    pub fn new(raw: u32) -> Self { Self(raw) }
    pub fn raw(self) -> u32 { self.0 }
    pub fn species(self) -> u16 { (self.0 & 0xFFFF) as u16 }
    pub fn genus(self) -> u8 { ((self.0 >> 16) & 0xFF) as u8 }
    pub fn family(self) -> u8 { ((self.0 >> 24) & 0xFF) as u8 }

    /// Convert to V6 layer if this is a legacy-compatible layer.
    pub fn to_v6(self) -> Result<V6Layer, AltiumFormatError> { ... }
}
```

**Note on V6 layer numbering**: The Delphi `PcbApi_QueryBoardLayerInfo` byte mapping
shows 39-54 = Mechanical, 55-56 = DrillGuide/DrillDrawing, 57-72 = InternalPlane.
This matches the .NET TV6_Layer enum for the mechanical range but has a discrepancy
for KeepOut and Drill layers. We follow the Delphi byte mapping (ground truth for
binary format) and validate against real file data.

### 4.5 PCB Record Structs (All Types)

Every field is concrete. No `unknown_bytes`, no trailing byte capture.
If binary data has bytes we don't understand, that's a parse error.

#### PcbArc (Object ID 1)

```rust
#[derive(Debug, Clone)]
pub(crate) struct PcbArc {
    pub common: PcbPrimitiveCommon,
    pub center: CoordPoint,
    pub radius: Coord,
    pub start_angle: f64,
    pub end_angle: f64,
    pub width: Coord,
}
```

#### PcbPad (Object ID 2)

The most complex PCB primitive. 6 binary subrecords.

```rust
#[derive(Debug, Clone)]
pub(crate) struct PcbPad {
    pub common: PcbPrimitiveCommon,
    pub position: CoordPoint,
    pub top_size: CoordPoint,         // x_size, y_size for top layer
    pub mid_size: CoordPoint,         // x_size, y_size for mid layers
    pub bot_size: CoordPoint,         // x_size, y_size for bottom layer
    pub top_shape: PadShape,
    pub mid_shape: PadShape,
    pub bot_shape: PadShape,
    pub rotation: f64,
    pub plated: bool,
    pub pad_mode: PadStackMode,
    pub paste_mask_expansion: Coord,
    pub solder_mask_expansion: Coord,
    pub hole_size: Coord,
    pub hole_type: HoleType,
    pub hole_rotation: f64,
    pub hole_width: Coord,            // for slot holes
    pub name: String,
    pub jumper_id: i32,
    // Power plane properties
    pub plane_connection_style: PlaneConnectionStyle,
    pub relief_conductor_width: Coord,
    pub relief_entries: i16,
    pub relief_air_gap: Coord,
    pub power_plane_clearance: Coord,
    pub power_plane_relief_expansion: Coord,
    // Tenting
    pub tenting_top: bool,
    pub tenting_bottom: bool,
    // Test point
    pub assembly_testpoint_top: bool,
    pub assembly_testpoint_bottom: bool,
    // Full-stack data (used when pad_mode == LocalStack)
    pub per_layer_sizes: [CoordPoint; 32],
    pub per_layer_shapes: [PadShape; 32],
    // Paste/solder mask expansion modes (from ExtendedPrimitiveInformation sidecar)
    pub paste_mask_expansion_mode: MaskExpansionMode,
    pub solder_mask_expansion_mode: MaskExpansionMode,
}
```

#### PcbVia (Object ID 3)

```rust
#[derive(Debug, Clone)]
pub(crate) struct PcbVia {
    pub common: PcbPrimitiveCommon,
    pub position: CoordPoint,
    pub diameter: Coord,
    pub hole_size: Coord,
    pub from_layer: V6Layer,
    pub to_layer: V6Layer,
    pub tenting_top: bool,
    pub tenting_bottom: bool,
    pub plane_connection_style: PlaneConnectionStyle,
    pub relief_conductor_width: Coord,
    pub relief_entries: i16,
    pub relief_air_gap: Coord,
    pub power_plane_clearance: Coord,
    pub power_plane_relief_expansion: Coord,
    pub solder_mask_expansion: Coord,
    pub paste_mask_expansion: Coord,
}
```

#### PcbTrack (Object ID 4)

```rust
#[derive(Debug, Clone)]
pub(crate) struct PcbTrack {
    pub common: PcbPrimitiveCommon,
    pub start: CoordPoint,
    pub end: CoordPoint,
    pub width: Coord,
}
```

#### PcbText (Object ID 5)

Two binary subrecords.

```rust
#[derive(Debug, Clone)]
pub(crate) struct PcbText {
    pub common: PcbPrimitiveCommon,
    pub position: CoordPoint,
    pub height: Coord,
    pub stroke_width: Coord,
    pub rotation: f64,
    pub mirror: bool,
    pub text: String,
    pub font_name: String,
    pub text_kind: TextKind,
    pub bold: bool,
    pub italic: bool,
    pub inverted: bool,
    pub inverted_border: Coord,
    pub barcode_kind: BarcodeKind,
    pub barcode_render_mode: BarcodeRenderMode,
    pub auto_position: TextAutoPosition,
}
```

#### PcbFill (Object ID 6)

```rust
#[derive(Debug, Clone)]
pub(crate) struct PcbFill {
    pub common: PcbPrimitiveCommon,
    pub p1: CoordPoint,
    pub p2: CoordPoint,
    pub rotation: f64,
}
```

#### PcbComponent (Object ID 9)

Component records use text parameter format (Components6 section), not binary.

```rust
#[derive(Debug, Clone)]
pub(crate) struct PcbComponent {
    pub layer: V6Layer,
    pub position: CoordPoint,
    pub rotation: f64,
    pub pattern: String,
    pub designator: String,
    pub comment: String,
    pub description: String,
    pub source_footprint_library: String,
    pub source_component_library: String,
    pub source_lib_reference: String,
    pub height: Coord,
    pub name_on: bool,
    pub comment_on: bool,
    pub locked: bool,
    pub component_kind: ComponentKind,
    pub unique_id: String,
    pub name_auto_position: TextAutoPosition,
    pub comment_auto_position: TextAutoPosition,
    pub channel_offset: CoordPoint,
}
```

#### PcbPolygon (Object ID 10)

Polygon definitions use text parameter format (Polygons6 section).

```rust
#[derive(Debug, Clone)]
pub(crate) struct PcbPolygon {
    pub layer: V6Layer,
    pub net: i16,
    pub hatch_style: PolyHatchStyle,
    pub polygon_type: PolygonType,
    pub grid_size: Coord,
    pub track_width: Coord,
    pub min_primitive_length: Coord,
    pub pour_over_same_net: bool,
    pub remove_dead_copper: bool,
    pub remove_narrow_necks: bool,
    pub arc_approximation: Coord,
    pub vertices: Vec<CoordPoint>,
    pub unique_id: String,
}
```

#### PcbRegion (Object ID 11)

```rust
#[derive(Debug, Clone)]
pub(crate) struct PcbRegion {
    pub common: PcbPrimitiveCommon,
    pub kind: RegionKind,
    pub name: String,
    pub vertices: Vec<CoordPoint>,
    pub holes: Vec<Vec<CoordPoint>>,  // inner contours (cutouts)
}
```

#### PcbComponentBody (Object ID 12)

```rust
#[derive(Debug, Clone)]
pub(crate) struct PcbComponentBody {
    pub common: PcbPrimitiveCommon,
    pub model_id: String,
    pub standoff_height: Coord,
    pub overall_height: Coord,
    pub rotation: f64,
    pub body_projection: i32,
    pub vertices: Vec<CoordPoint>,
}
```

#### PcbDimension (Object ID 13)

```rust
#[derive(Debug, Clone)]
pub(crate) struct PcbDimension {
    pub common: PcbPrimitiveCommon,
    pub dimension_kind: DimensionKind,
    pub text_height: Coord,
    pub text_width: Coord,
    pub text_position: CoordPoint,
    pub text: String,
    pub references: Vec<CoordPoint>,  // reference/measurement points
    pub arrow_size: Coord,
    pub line_width: Coord,
    pub units: i32,
    pub font_name: String,
}
```

#### PcbCoordinate (Object ID 14)

```rust
#[derive(Debug, Clone)]
pub(crate) struct PcbCoordinate {
    pub common: PcbPrimitiveCommon,
    pub position: CoordPoint,
}
```

#### PcbBoardOutline (Object ID 26)

```rust
#[derive(Debug, Clone)]
pub(crate) struct PcbBoardOutline {
    pub vertices: Vec<CoordPoint>,
}
```

### 4.6 PCB Non-Primitive Types

These are stored in separate text-format sections, not as binary primitives.

```rust
/// Net definition (from Nets6 section).
pub(crate) struct PcbNet {
    pub name: String,
    pub color: Color,
    pub visible: bool,
}

/// Design rule (from Rules6 section).
pub(crate) struct PcbRule {
    pub name: String,
    pub rule_kind: String,
    pub enabled: bool,
    pub priority: i32,
    pub scope1_expression: String,
    pub scope2_expression: String,
    // Rule-kind-specific fields parsed based on rule_kind
}

/// Net/component class (from Classes6 section).
pub(crate) struct PcbClass {
    pub name: String,
    pub kind: String,             // "Net Class", "Component Class", etc.
    pub members: Vec<String>,
}

/// 3D model reference (from Models section).
pub(crate) struct PcbModel {
    pub id: String,
    pub filename: String,
    pub rotation_x: f64,
    pub rotation_y: f64,
    pub rotation_z: f64,
    pub standoff: Coord,
    pub body_opacity: f64,
}
```

### 4.7 PCB Enums

```rust
/// Pad/via shape (0-10). No catch-all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum PadShape {
    NoShape = 0,
    #[default]
    Round = 1,
    Rectangular = 2,
    Octagonal = 3,
    Circle = 4,
    Arc = 5,
    Terminator = 6,
    RoundRect = 7,
    RotatedRect = 8,
    RoundedRectangular = 9,
    Custom = 10,
}

/// Pad shape sub-kind (0-5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum PadShapeSubKind {
    #[default]
    NoKind = 0,
    OctagonalFinger = 1,
    RoundedFinger = 2,
    RoundedRectangle = 3,
    ChamferedRectangle = 4,
    Donut = 5,
}

/// Pad stack mode (0-2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum PadStackMode {
    #[default]
    Simple = 0,
    LocalStack = 1,
    ExternalStack = 2,
}

/// Hole type (0-2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum HoleType {
    #[default]
    Round = 0,
    Square = 1,
    Slot = 2,
}

/// Drill type (0-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum DrillType {
    #[default]
    Drilled = 0,
    Punched = 1,
    LaserDrilled = 2,
    PlasmaDrilled = 3,
}

/// Drill layer pair type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum DrillLayerPairType {
    #[default]
    Regular = 0,
    MicroViaDrill = 1,
    Backdrill = 2,
    CounterHole = 3,
}

/// PCB text kind (0-2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum TextKind {
    #[default]
    StrokeFont = 0,
    TrueTypeFont = 1,
    Barcode = 2,
}

/// Barcode kind (0-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum BarcodeKind {
    #[default]
    Code39 = 0,
    Code128 = 1,
    QrCode = 2,
    DataMatrix = 3,
}

/// Barcode render mode (0-1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum BarcodeRenderMode {
    #[default]
    ByMinWidth = 0,
    ByFullWidth = 1,
}

/// Polygon hatch style (0-5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum PolyHatchStyle {
    Hatch90 = 0,
    Hatch45 = 1,
    VerticalHatch = 2,
    HorizontalHatch = 3,
    NoHatch = 4,
    #[default]
    Solid = 5,
}

/// Polygon type (0-2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum PolygonType {
    #[default]
    SignalLayer = 0,
    SplitPlane = 1,
    CoverlayOutline = 2,
}

/// Region kind (0-4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum RegionKind {
    #[default]
    Copper = 0,
    Cutout = 1,
    Named = 2,
    BoardCutout = 3,
    Cavity = 4,
}

/// Plane connection style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum PlaneConnectionStyle {
    #[default]
    NoConnect = 0,
    Relief = 1,
    Direct = 2,
}

/// Text auto-position (0-9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum TextAutoPosition {
    #[default]
    Manual = 0,
    TopLeft = 1,
    CenterLeft = 2,
    BottomLeft = 3,
    TopCenter = 4,
    CenterCenter = 5,
    BottomCenter = 6,
    TopRight = 7,
    CenterRight = 8,
    BottomRight = 9,
}

/// Mask expansion mode (from ExtendedPrimitiveInformation sidecar).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum MaskExpansionMode {
    #[default]
    NoOverride = 0,
    Override = 1,
    TentingTop = 2,
    TentingBottom = 3,
    TentingBoth = 4,
}

/// Dimension kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum DimensionKind {
    #[default]
    Linear = 0,
    Angular = 1,
    Radial = 2,
    Leader = 3,
    Datum = 4,
    Baseline = 5,
    Center = 6,
    LinearDiameter = 7,
    RadialDiameter = 8,
}

/// PCB file format version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[repr(u8)]
pub enum PcbFileFormatVersion {
    None = 0,
    BinaryV3 = 1,
    LibraryV3 = 2,
    AsciiV3 = 3,
    BinaryV4 = 4,
    LibraryV4 = 5,
    AsciiV4 = 6,
    BinaryV5 = 7,
    LibraryV5 = 8,
    AsciiV5 = 9,
    BinaryV6 = 10,
    LibraryV6 = 11,
    AsciiV6 = 12,
    BinaryV6CS = 13,
    BinaryV6CM = 14,
    BinaryV6PCBWorks = 15,
    PadViaLibraryV6 = 16,
}

/// Board side (top/bottom).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
#[repr(u8)]
pub enum BoardSide {
    #[default]
    Top = 0,
    Bottom = 1,
}
```

---

## 5. Container (Document) Types

### 5.1 Schematic Containers

```rust
/// Parsed schematic document (.SchDoc).
/// Public API -- ops crate accesses data through methods, not fields.
pub struct SchDoc {
    // Private fields
    header: SchSheet,
    records: Vec<SchRecord>,
    // Sidecar data is merged into records during loading -- no separate storage.
}

/// Parsed schematic library (.SchLib).
pub struct SchLib {
    components: Vec<SchLibComponent>,
}

/// A single component definition within a SchLib.
pub(crate) struct SchLibComponent {
    pub name: String,
    pub description: String,
    pub part_count: i32,
    pub aliases: Vec<String>,
    pub records: Vec<SchRecord>,
}
```

### 5.2 PCB Containers

```rust
/// Parsed PCB document (.PcbDoc).
pub struct PcbDoc {
    board: PcbBoard,
    components: Vec<PcbComponent>,
    nets: Vec<PcbNet>,
    classes: Vec<PcbClass>,
    rules: Vec<PcbRule>,
    models: Vec<PcbModel>,
    // Per-type primitive storage (mirrors section-per-type file structure)
    arcs: Vec<PcbArc>,
    pads: Vec<PcbPad>,
    vias: Vec<PcbVia>,
    tracks: Vec<PcbTrack>,
    texts: Vec<PcbText>,
    fills: Vec<PcbFill>,
    regions: Vec<PcbRegion>,
    component_bodies: Vec<PcbComponentBody>,
    polygons: Vec<PcbPolygon>,
    dimensions: Vec<PcbDimension>,
    board_outline: PcbBoardOutline,
}

/// Parsed PCB library (.PcbLib).
pub struct PcbLib {
    footprints: Vec<PcbLibFootprint>,
}

/// A single footprint definition within a PcbLib.
pub(crate) struct PcbLibFootprint {
    pub pattern: String,
    pub height: Coord,
    pub description: String,
    pub primitives: Vec<PcbRecord>,
}

/// Board-level metadata (from Board6/Data section).
pub(crate) struct PcbBoard {
    pub origin: CoordPoint,
    pub snap_grid_size: Coord,
    pub visible_grid_size: Coord,
    // Layer stack definition
    pub layer_count: i32,
    // Additional board properties
}

/// Parsed integrated library (.IntLib).
/// Contains an embedded SchLib and PcbLib.
pub struct IntLib {
    pub sch_lib: SchLib,
    pub pcb_lib: PcbLib,
}
```

---

## 6. Design Decision Analysis

### 6.1 Strict Parsing: No Unknown Variants

**Chosen: No `Unknown` variant, no `unknown_fields`, no `unknown_bytes`.**

| Approach | Pros | Cons |
|----------|------|------|
| Unknown variants + field capture (old design) | Round-trips unknown data; handles any file | Silently hides bugs in our parser; risks corrupting data we don't understand |
| **Strict parsing (chosen)** | Forces us to fully understand every byte; any gap is a clear bug; safe for fabrication | Must implement every record type before loading those files; new Altium versions may break us |

The strict approach is correct for PCB data. A silently dropped copper pour or wrong
pad shape causes physical defects. We accept the cost of implementing every record type
and treating unknown data as errors that must be fixed in our code.

`#[non_exhaustive]` on enums allows adding new variants in future releases without
breaking downstream code, while still refusing to silently handle unknown values at
parse time.

### 6.2 Separate Enums Per Domain

**Chosen: Separate enums** (`SchRecordType` + `PcbObjectId`)

The domains have zero overlap in record IDs or semantics. A mega-enum would be misleading
and would create nonsensical cross-domain type states.

### 6.3 Enum Dispatch vs Trait Objects

**Chosen: Enum dispatch** (`SchRecord` / `PcbRecord` enums)

Record types are fixed by the file format (not extensible at runtime). Enum dispatch gives
us exhaustive matching at compile time, zero heap allocation, and natural serialization.
Without an `Unknown` variant, every match arm must handle a concrete type -- the compiler
enforces completeness.

### 6.4 Composition (Flatten) vs Inheritance

**Chosen: Composition with `#[altium(flatten)]`**

Rust has no inheritance. Composition via embedded base structs with a `flatten` derive
attribute expands base fields into the derived struct's serialization:

```rust
#[derive(AltiumRecord)]
pub(crate) struct SchWire {
    #[altium(flatten)]
    pub base: SchGraphicalBase,  // includes SchPrimitiveBase fields
    // ... type-specific fields
}
```

### 6.5 PCB Container: Per-Type Vecs vs Single Vec

**Chosen: Per-type Vecs** in `PcbDoc`

PcbDoc files store primitives in separate OLE sections per type. Mirroring this in our
data model makes serialization natural and enables typed iteration without downcasting.

### 6.6 Version-Gated Fields

**Decision: Always-present with defaults, not version-gated.**

Some fields exist in AD26 but not AD20. Rather than conditional compilation or runtime
version checking, all fields are always present in our structs. Fields absent in older
files get their Altium default values during parsing. This simplifies the API at the cost
of potentially writing newer fields to older-format files (acceptable since we target
modern AD formats).

### 6.7 Visibility Strategy

Per CLAUDE.md: implementation details must be private to the crate.

- Container types (`SchDoc`, `SchLib`, `PcbDoc`, `PcbLib`, `IntLib`): `pub`
- Shared foundational types (`Coord`, `Color`, `CoordPoint`, etc.): `pub`
- Record type enums (`SchRecordType`, `PcbObjectId`): `pub`
- Domain enums (`PadShape`, `PinElectricalType`, etc.): `pub`
- Record structs (`SchPin`, `PcbTrack`, etc.): `pub(crate)`
- Base types (`SchPrimitiveBase`, `PcbPrimitiveCommon`): `pub(crate)`
- Fields on record structs: `pub(crate)`

The ops crate accesses data through public methods on the container types.

---

## 7. Font Table

```rust
/// Font definition from SchSheet RECORD=31.
#[derive(Debug, Clone)]
pub(crate) struct SchFont {
    pub id: i32,            // 1-based index
    pub name: String,       // e.g., "Times New Roman"
    pub size: i32,          // point size
    pub rotation: i32,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikeout: bool,
}
```

---

## 8. Complete Type Count Summary

| Category | Count |
|----------|-------|
| Shared foundational types | 6 (`Coord`, `CoordPoint`, `BoundingBox`, `Color`, `UniqueId`, `ComponentKind`) |
| Schematic record structs | ~40 (all observed RECORD IDs + harness/extended types) |
| Schematic base types | 2 (`SchPrimitiveBase`, `SchGraphicalBase`) |
| Schematic enums | 15 (`SchRecordType`, `PinElectricalType`, `RotationBy90`, `IeeeSymbol`, `StdLogicState`, `PenWidth`, `LineStyle`, `LineShape`, `TextJustification`, `PowerObjectStyle`, `PortArrowStyle`, `PortIoType`, `SheetStyle`, `SheetOrientation`, `SheetBorderStyle`) |
| PCB record structs | ~15 (all object IDs + non-primitive types) |
| PCB base types | 2 (`PcbPrimitiveCommon`, `PcbFlags`) |
| PCB enums | 20 (`PcbObjectId`, `V6Layer`, `PadShape`, `PadShapeSubKind`, `PadStackMode`, `HoleType`, `DrillType`, `DrillLayerPairType`, `TextKind`, `BarcodeKind`, `BarcodeRenderMode`, `PolyHatchStyle`, `PolygonType`, `RegionKind`, `PlaneConnectionStyle`, `TextAutoPosition`, `MaskExpansionMode`, `DimensionKind`, `PcbFileFormatVersion`, `BoardSide`) |
| Container types | 5 (`SchDoc`, `SchLib`, `PcbDoc`, `PcbLib`, `IntLib`) |
| **Total** | **~105 types** |
