# Crate Structure and Module Layout

Architecture design for the `altium-cli` workspace. Four crates with a strict
dependency chain, clean public API boundaries, and private serialization
internals.

**Design philosophy: fail fast, fail hard.** These files go to PCB fabrication.
A silently dropped copper pour or missing pad could cost thousands of dollars.
We never skip unknown data -- we error. Our model must be complete.

---

## 1. Crate Dependency Graph

```
altium-format-derive          (proc-macro crate, no runtime deps)
         |
         v
    altium-format             (core library: types, parsing, querying, editing)
         |
         v
   altium-format-ops          (high-level operations: summaries, add, edit, diff)
         |
         v
      altium-cli              (binary: CLI interface, output formatting)
```

**Publishing order:** derive -> format -> ops -> cli.

**Versioning:** All four crates share the same version number for initial
releases.

**Key constraint:** `altium-format` implementation details (serialization
traits, binary layouts, parameter key names) are `pub(crate)` -- they MUST NOT
leak into `altium-format-ops` or `altium-cli`. The public API exposes typed
Rust structs and document-level operations only.

---

## 2. altium-format-derive

Proc-macro crate. Zero runtime dependencies (only `syn`, `quote`, `proc-macro2`
as build deps).

### Macros Provided

| Macro | Applies to | Generates |
|-------|-----------|-----------|
| `AltiumDeserialize` | Record structs | `FromParams` (SCH) or `FromBinary` (PCB) impl |
| `AltiumSerialize` | Record structs | `ToParams` (SCH) or `ToBinary` (PCB) impl |
| `AltiumEnum` | `#[repr(u8)]` / `#[repr(i32)]` enums | `TryFrom<integer>` (errors on unknown) |

A single struct can derive both `AltiumDeserialize` and `AltiumSerialize`.
The macro inspects field attributes to determine whether to generate
parameter-based (text) or binary-based code. A struct is either
parameter-serialized or binary-serialized, never both.

### Crate Layout

```
crates/altium-format-derive/
├── Cargo.toml
└── src/
    ├── lib.rs                   # proc-macro entry points
    ├── param_de.rs              # FromParams code generation
    ├── param_ser.rs             # ToParams code generation
    ├── binary_de.rs             # FromBinary code generation
    ├── binary_ser.rs            # ToBinary code generation
    ├── enum_derive.rs           # AltiumEnum code generation
    └── attrs.rs                 # Attribute parsing (#[altium(...)])
```

---

## 3. altium-format

The core library. Depends on `altium-format-derive` (for derive macros), `cfb`
(for OLE/CFB container access), `flate2` (for zlib decompression), and
`encoding_rs` (for Windows-1252).

### Module Layout

```
crates/altium-format/
├── Cargo.toml
└── src/
    ├── lib.rs                   # Crate root: re-exports public API
    │
    ├── error.rs                 # AltiumFormatError, Result<T> alias
    │
    ├── common/
    │   ├── mod.rs
    │   ├── coord.rs             # Coord(i32), CoordPoint, CoordRect
    │   ├── color.rs             # Color (Win32 COLORREF, BGR)
    │   ├── unique_id.rs         # UniqueId (8-char uppercase string)
    │   └── types.rs             # Shared small types (BoolShort, BoolLong)
    │
    ├── container/
    │   ├── mod.rs
    │   ├── cfb_reader.rs        # Read-only CFB wrapper
    │   ├── cfb_writer.rs        # Write CFB wrapper
    │   ├── block.rs             # Size-prefixed block framing (read/write)
    │   └── compressed.rs        # 0xD0 compressed block handling (zlib)
    │
    ├── params/
    │   ├── mod.rs
    │   ├── collection.rs        # ParameterCollection: parse/emit pipe-delimited kv
    │   ├── from_params.rs       # FromParams trait definition
    │   ├── to_params.rs         # ToParams trait definition
    │   └── value.rs             # FromParamValue / ToParamValue for primitives
    │
    ├── binary/
    │   ├── mod.rs
    │   ├── reader.rs            # BinaryReader: cursor with exact-byte-count tracking
    │   ├── writer.rs            # BinaryWriter: cursor with byte-count tracking
    │   ├── from_binary.rs       # FromBinary trait definition
    │   └── to_binary.rs         # ToBinary trait definition
    │
    ├── sch/
    │   ├── mod.rs               # Re-exports SchRecord, enums
    │   ├── record.rs            # SchRecord enum (dispatch by RECORD id)
    │   ├── enums.rs             # PinElectricalType, Rotation, LineStyle, etc.
    │   ├── base.rs              # SchPrimitiveBase, SchGraphicalBase
    │   ├── records/
    │   │   ├── mod.rs
    │   │   ├── component.rs     # SchComponent (RECORD=1)
    │   │   ├── pin.rs           # SchPin (RECORD=2)
    │   │   ├── symbol.rs        # SchSymbol (RECORD=3)
    │   │   ├── label.rs         # SchLabel (RECORD=4)
    │   │   ├── bezier.rs        # SchBezier (RECORD=5)
    │   │   ├── polyline.rs      # SchPolyline (RECORD=6)
    │   │   ├── polygon.rs       # SchPolygon (RECORD=7)
    │   │   ├── ellipse.rs       # SchEllipse (RECORD=8)
    │   │   ├── pie.rs           # SchPie (RECORD=9)
    │   │   ├── round_rect.rs    # SchRoundRectangle (RECORD=10)
    │   │   ├── elliptical_arc.rs # SchEllipticalArc (RECORD=11)
    │   │   ├── arc.rs           # SchArc (RECORD=12)
    │   │   ├── line.rs          # SchLine (RECORD=13)
    │   │   ├── rectangle.rs     # SchRectangle (RECORD=14)
    │   │   ├── sheet_symbol.rs  # SchSheetSymbol (RECORD=15)
    │   │   ├── sheet_entry.rs   # SchSheetEntry (RECORD=16)
    │   │   ├── power_object.rs  # SchPowerObject (RECORD=17)
    │   │   ├── port.rs          # SchPort (RECORD=18)
    │   │   ├── no_erc.rs        # SchNoErc (RECORD=22)
    │   │   ├── error_marker.rs  # SchErrorMarker (RECORD=23)
    │   │   ├── net_label.rs     # SchNetLabel (RECORD=25)
    │   │   ├── bus.rs           # SchBus (RECORD=26)
    │   │   ├── wire.rs          # SchWire (RECORD=27)
    │   │   ├── text_frame.rs    # SchTextFrame (RECORD=28)
    │   │   ├── junction.rs      # SchJunction (RECORD=29)
    │   │   ├── image.rs         # SchImage (RECORD=30)
    │   │   ├── sheet.rs         # SchSheet (RECORD=31, sheet properties + fonts)
    │   │   ├── sheet_name.rs    # SchSheetName (RECORD=32)
    │   │   ├── sheet_filename.rs # SchSheetFileName (RECORD=33)
    │   │   ├── designator.rs    # SchDesignator (RECORD=34)
    │   │   ├── bus_entry.rs     # SchBusEntry (RECORD=37)
    │   │   ├── template.rs      # SchTemplate (RECORD=39)
    │   │   ├── parameter.rs     # SchParameter (RECORD=41)
    │   │   ├── parameter_set.rs # SchParameterSet (RECORD=43)
    │   │   ├── impl_list.rs     # SchImplementationList (RECORD=44)
    │   │   ├── implementation.rs # SchImplementation (RECORD=45)
    │   │   ├── impl_map.rs      # SchImplementationMap (RECORD=46)
    │   │   ├── map_definer.rs   # SchMapDefiner (RECORD=47)
    │   │   ├── impl_params.rs   # SchImplementationParameters (RECORD=48)
    │   │   ├── note.rs          # SchNote (RECORD=209)
    │   │   ├── probe.rs         # SchProbe (RECORD=210)
    │   │   ├── compile_mask.rs  # SchCompileMask (RECORD=211/225)
    │   │   ├── harness.rs       # Harness records (RECORD=215-218, 104-138)
    │   │   ├── blanket.rs       # SchBlanket (RECORD=225)
    │   │   └── hyperlink.rs     # SchHyperlink (RECORD=226)
    │   └── sidecar/
    │       ├── mod.rs
    │       ├── pin_frac.rs      # PinFrac binary sidecar (12 bytes/pin)
    │       ├── pin_wide_text.rs # PinWideText UTF-16LE sidecar
    │       ├── pin_misc.rs      # PinMiscData, PinDesc, PinTextData
    │       ├── pin_symbol.rs    # PinSymbolLineWidth, PinFunctionData
    │       ├── pin_timing.rs    # PinPackageLength, PinPropagationDelay
    │       ├── wide_strings.rs  # WideStrings sidecar
    │       ├── unique_ids.rs    # UniqueIDs sidecar
    │       └── extended_info.rs # ExtendedPrimitiveInfo sidecar
    │
    ├── pcb/
    │   ├── mod.rs               # Re-exports PcbRecord, enums, layer types
    │   ├── record.rs            # PcbRecord enum (dispatch by object_id byte)
    │   ├── enums.rs             # PadShape, PadMode, HoleType, DrillType, etc.
    │   ├── layer.rs             # V6Layer(u8), V7Layer(u32), layer constants
    │   ├── base.rs              # PcbPrimitiveHeader (layer, flags, net, component)
    │   ├── records/
    │   │   ├── mod.rs
    │   │   ├── arc.rs           # PcbArc (ID=1)
    │   │   ├── pad.rs           # PcbPad (ID=2, multi-subrecord)
    │   │   ├── via.rs           # PcbVia (ID=3)
    │   │   ├── track.rs         # PcbTrack (ID=4)
    │   │   ├── text.rs          # PcbText (ID=5, multi-subrecord)
    │   │   ├── fill.rs          # PcbFill (ID=6)
    │   │   ├── polygon.rs       # PcbPolygon (ID=10)
    │   │   ├── region.rs        # PcbRegion (ID=11)
    │   │   ├── component_body.rs # PcbComponentBody (ID=12)
    │   │   ├── dimension.rs     # PcbDimension (ID=13)
    │   │   ├── coordinate.rs    # PcbCoordinate (ID=14)
    │   │   └── component.rs     # PcbComponent (parameter-based, from Components6)
    │   └── sidecar/
    │       ├── mod.rs
    │       ├── wide_strings.rs  # WideStrings6 binary TLV format
    │       ├── unique_ids.rs    # UniqueIDPrimitiveInformation parameter blocks
    │       ├── extended_info.rs # ExtendedPrimitiveInformation parameter blocks
    │       └── primitive_guids.rs # PrimitiveGuids 24-byte binary records
    │
    └── doc/
        ├── mod.rs               # Re-exports document types
        ├── sch_doc.rs           # SchDoc: load/save schematic document
        ├── sch_lib.rs           # SchLib: load/save schematic library
        ├── pcb_doc.rs           # PcbDoc: load/save PCB document
        ├── pcb_lib.rs           # PcbLib: load/save PCB library
        └── int_lib.rs           # IntLib: load/save integrated library
```

### Module Visibility Rules

| Module | Visibility | Rationale |
|--------|-----------|-----------|
| `common/` | `pub` | Coord, Color, UniqueId are part of the public API |
| `error` | `pub` | AltiumFormatError is public |
| `container/` | `pub(crate)` | CFB access is an implementation detail |
| `params/` | `pub(crate)` | Parameter serialization is an implementation detail |
| `binary/` | `pub(crate)` | Binary serialization is an implementation detail |
| `sch/` | `pub` (types only) | Record structs and enums are public |
| `sch/records/` | `pub` (types only) | Individual record structs are public |
| `sch/sidecar/` | `pub(crate)` | Sidecar merging is an implementation detail |
| `pcb/` | `pub` (types only) | Record structs, enums, layers are public |
| `pcb/records/` | `pub` (types only) | Individual record structs are public |
| `pcb/sidecar/` | `pub(crate)` | Sidecar merging is an implementation detail |
| `doc/` | `pub` | Document types are the primary public API |

**Struct fields** on all record types are `pub` -- users can read and write
typed fields directly. But the `FromParams`/`ToParams`/`FromBinary`/`ToBinary`
traits themselves are `pub(crate)` and never appear in the public API.

---

## 4. Public API Surface

### 4.1 Document Types (the primary user-facing API)

```rust
// crates/altium-format/src/doc/sch_doc.rs
pub struct SchDoc {
    header: SchSheet,              // Sheet properties (RECORD=31)
    records: Vec<SchRecord>,       // All records, parent-child via OwnerIndex
}

impl SchDoc {
    pub fn open(path: impl AsRef<Path>) -> Result<Self>;
    pub fn from_reader(reader: impl Read + Seek) -> Result<Self>;
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()>;
    pub fn to_writer(&self, writer: impl Write + Seek) -> Result<()>;

    pub fn header(&self) -> &SchSheet;
    pub fn header_mut(&mut self) -> &mut SchSheet;
    pub fn records(&self) -> &[SchRecord];
    pub fn records_mut(&mut self) -> &mut Vec<SchRecord>;
}
```

```rust
// crates/altium-format/src/doc/sch_lib.rs
pub struct SchLib {
    header: SchSheet,
    components: Vec<SchLibComponent>,
}

pub struct SchLibComponent {
    pub name: String,
    pub description: String,
    pub records: Vec<SchRecord>,   // RECORD=1 is first, children follow
    pub aliases: Vec<String>,
}

impl SchLib {
    pub fn open(path: impl AsRef<Path>) -> Result<Self>;
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()>;
    pub fn components(&self) -> &[SchLibComponent];
    pub fn components_mut(&mut self) -> &mut Vec<SchLibComponent>;
    pub fn find_component(&self, name: &str) -> Option<&SchLibComponent>;
}
```

```rust
// crates/altium-format/src/doc/pcb_doc.rs
pub struct PcbDoc {
    board: PcbBoard,                // Board6 properties
    components: Vec<PcbComponent>,  // Components6 records
    nets: Vec<PcbNet>,              // Nets6 records
    primitives: PcbPrimitives,      // All primitive records by type
    rules: Vec<PcbRule>,            // Rules6 records
    classes: Vec<PcbClass>,         // Classes6 records
    polygons: Vec<PcbPolygonDef>,   // Polygons6 definitions
}

pub struct PcbPrimitives {
    pub arcs: Vec<PcbArc>,
    pub pads: Vec<PcbPad>,
    pub vias: Vec<PcbVia>,
    pub tracks: Vec<PcbTrack>,
    pub texts: Vec<PcbText>,
    pub fills: Vec<PcbFill>,
    pub regions: Vec<PcbRegion>,
    pub component_bodies: Vec<PcbComponentBody>,
    pub dimensions: Vec<PcbDimension>,
}
```

```rust
// crates/altium-format/src/doc/pcb_lib.rs
pub struct PcbLib {
    board: PcbBoard,
    footprints: Vec<PcbLibFootprint>,
}

pub struct PcbLibFootprint {
    pub pattern: String,
    pub description: String,
    pub height: Coord,
    pub primitives: PcbPrimitives,
}
```

### 4.2 Record Types

The `SchRecord` enum dispatches to individual record structs:

```rust
pub enum SchRecord {
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
    Parameter(SchParameter),
    ParameterSet(SchParameterSet),
    ImplementationList(SchImplementationList),
    Implementation(SchImplementation),
    ImplementationMap(SchImplementationMap),
    MapDefiner(SchMapDefiner),
    ImplementationParameters(SchImplementationParameters),
    Note(SchNote),
    Probe(SchProbe),
    CompileMask(SchCompileMask),
    Blanket(SchBlanket),
    Hyperlink(SchHyperlink),
    // Harness records...
    // NO Unknown variant. Unknown record IDs are errors.
}
```

**No `Unknown` variant.** An unrecognized RECORD value means our model is
incomplete. This is a bug in our code that must be fixed by adding the missing
record type. We do not silently pass through data we don't understand -- that
data could contain critical electrical information.

Similarly for PCB:

```rust
pub enum PcbRecord {
    Arc(PcbArc),
    Pad(PcbPad),
    Via(PcbVia),
    Track(PcbTrack),
    Text(PcbText),
    Fill(PcbFill),
    Polygon(PcbPolygon),
    Region(PcbRegion),
    ComponentBody(PcbComponentBody),
    Dimension(PcbDimension),
    Coordinate(PcbCoordinate),
    // NO Unknown variant.
}
```

### 4.3 Example Record Struct

Each record struct is a plain Rust struct with public typed fields:

```rust
pub struct SchPin {
    pub base: SchGraphicalBase,
    pub name: String,
    pub designator: String,
    pub electrical: PinElectricalType,
    pub orientation: Rotation,
    pub pin_length: Coord,
    pub is_hidden: bool,
    pub show_name: bool,
    pub show_designator: bool,
    pub description: String,
    pub formal_type: StdLogicState,
    pub symbol_inner_edge: IeeeSymbol,
    pub symbol_outer_edge: IeeeSymbol,
    pub symbol_inside: IeeeSymbol,
    pub symbol_outside: IeeeSymbol,
    pub symbol_line_width: PenWidth,
    pub hidden_net_name: String,
    pub default_value: String,
    pub swap_id_group: String,
    pub swap_id_part: String,
    pub swap_id_sequence: String,
    pub pin_propagation_delay: f64,
    pub pin_package_length: Coord,
    pub unique_id: String,
}
```

No `unknown_fields` member. Every field in the file maps to a typed Rust field.
If a file contains a parameter key we don't recognize, parsing fails with
`AltiumFormatError::UnknownParameterKey`. This forces us to add the field.

### 4.4 Common Types

```rust
/// Coordinate in Altium internal units (10,000 units = 1 mil).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Coord(pub i32);

impl Coord {
    pub fn from_mils(mils: f64) -> Self;
    pub fn from_mm(mm: f64) -> Self;
    pub fn to_mils(self) -> f64;
    pub fn to_mm(self) -> f64;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoordPoint {
    pub x: Coord,
    pub y: Coord,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoordRect {
    pub min: CoordPoint,
    pub max: CoordPoint,
}

/// Win32 COLORREF in BGR format: 0x00BBGGRR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color(pub u32);

impl Color {
    pub fn rgb(r: u8, g: u8, b: u8) -> Self;
    pub fn r(self) -> u8;
    pub fn g(self) -> u8;
    pub fn b(self) -> u8;
}
```

### 4.5 Error Type

A single error type for the crate. Every error variant gives precise context
about what went wrong and where.

```rust
#[derive(Debug, thiserror::Error)]
pub enum AltiumFormatError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("CFB container error: {0}")]
    Cfb(String),

    #[error("unknown schematic record type: RECORD={record_id}")]
    UnknownSchRecord { record_id: i32 },

    #[error("unknown PCB object type: ID={object_id}")]
    UnknownPcbObjectType { object_id: u8 },

    #[error("unknown parameter key {key:?} in {record_type}")]
    UnknownParameterKey { key: String, record_type: String },

    #[error("missing required parameter {key:?} in {record_type}")]
    MissingRequiredParameter { key: String, record_type: String },

    #[error("invalid parameter value for {key:?}: {value:?} ({reason})")]
    InvalidParameterValue { key: String, value: String, reason: String },

    #[error("binary size mismatch in {record_type}: expected {expected} bytes, got {actual}")]
    BinaryLengthMismatch { record_type: String, expected: usize, actual: usize },

    #[error("unexpected trailing bytes in {record_type}: {count} bytes remaining")]
    UnexpectedTrailingBytes { record_type: String, count: usize },

    #[error("unknown enum variant: value {value} for {enum_type}")]
    UnknownEnumVariant { value: i64, enum_type: String },

    #[error("decompression error: {0}")]
    Decompression(String),

    #[error("encoding error: {0}")]
    Encoding(String),

    #[error("sidecar stream error in {stream}: {reason}")]
    SidecarError { stream: String, reason: String },

    #[error("stream not found: {path}")]
    StreamNotFound { path: String },

    #[error("invalid file format: {reason}")]
    InvalidFormat { reason: String },
}
```

Every function that can fail returns `Result<T, AltiumFormatError>`. No silent
drops. No panics on bad data. No `unwrap()` on user-controlled data.

---

## 5. Serialization Trait Hierarchy

All serialization traits are `pub(crate)`. They exist only for internal use by
derive macros and document loaders.

```rust
// Parameter-based (schematic text format)
pub(crate) trait FromParams: Sized {
    /// Parse from parameters. Errors on unknown keys, missing required keys,
    /// or invalid values. Consumes ALL keys -- nothing may be left over.
    fn from_params(params: &mut ParameterCollection) -> Result<Self>;
}

pub(crate) trait ToParams {
    /// Serialize to parameters. Writes every field.
    fn to_params(&self, params: &mut ParameterCollection) -> Result<()>;
}

// Binary-based (PCB format)
pub(crate) trait FromBinary: Sized {
    /// Parse from binary data. Errors if any bytes are unconsumed.
    fn from_binary(reader: &mut BinaryReader<'_>) -> Result<Self>;
}

pub(crate) trait ToBinary {
    /// Serialize to binary. Writes exact expected byte count.
    fn to_binary(&self, writer: &mut BinaryWriter) -> Result<()>;
}

// Value conversion (single parameter values)
pub(crate) trait FromParamValue: Sized {
    fn from_param_value(value: &str) -> Result<Self>;
}

pub(crate) trait ToParamValue {
    fn to_param_value(&self) -> String;
}
```

The key difference from a lenient design: `FromParams` takes a `&mut
ParameterCollection` and **removes** each key as it processes it. After all
fields are processed, if any keys remain, parsing fails with
`UnknownParameterKey`. Similarly, `FromBinary` tracks bytes consumed and
**errors if any bytes are left over**.

---

## 6. Derive Macro Strategy

### 6.1 Parameter-Based Records (Schematic)

```rust
#[derive(AltiumDeserialize, AltiumSerialize)]
#[altium(record_id = 12)]  // RECORD=12 = Arc
pub struct SchArc {
    #[altium(flatten)]
    pub base: SchGraphicalBase,

    #[altium(param = "RADIUS", with_frac = "RADIUS_FRAC")]
    pub radius: Coord,

    #[altium(param = "LINEWIDTH")]
    pub line_width: PenWidth,

    #[altium(param = "STARTANGLE")]
    pub start_angle: f64,

    #[altium(param = "ENDANGLE")]
    pub end_angle: f64,

    #[altium(param = "COLOR")]
    pub color: Color,

    #[altium(param = "UNIQUEID", default)]
    pub unique_id: String,
}
```

Generated `FromParams` behavior:
1. Remove `RECORD` key (already consumed by dispatch)
2. For `flatten` fields, delegate to the flattened struct's `FromParams`,
   which removes its keys from the collection
3. For each named field, remove the key and parse via `FromParamValue`
4. For `default` fields, use `Default::default()` if the key is absent
5. **After all fields are processed, check that the collection is empty.
   If any keys remain, return `UnknownParameterKey` error.**

Generated `ToParams` behavior:
1. Write `RECORD=12` first
2. For `flatten` fields, delegate
3. For each field, write the key=value pair
4. For `default` fields where value equals default, omit the key (skip_default)

### 6.2 Binary-Based Records (PCB)

```rust
#[derive(AltiumDeserialize, AltiumSerialize)]
#[altium(object_id = 4)]  // eTrackObject = 4
pub struct PcbTrack {
    #[altium(flatten)]
    pub header: PcbPrimitiveHeader,

    #[altium(binary = "coord_point")]
    pub start: CoordPoint,

    #[altium(binary = "coord_point")]
    pub end: CoordPoint,

    #[altium(binary = "i32le")]
    pub width: Coord,
}
```

Generated `FromBinary` behavior:
1. Create a `BinaryReader` wrapping the byte slice
2. Read fields in declaration order, each consuming exact bytes
3. **After all fields, assert `reader.remaining() == 0`. If not, return
   `UnexpectedTrailingBytes` error.**

Generated `ToBinary` behavior:
1. Create a `BinaryWriter`
2. Write fields in declaration order
3. Assert total bytes written matches expected record size

### 6.3 AltiumEnum

```rust
#[derive(AltiumEnum)]
#[repr(u8)]
pub enum PinElectricalType {
    Input = 0,
    InputOutput = 1,
    Output = 2,
    OpenCollector = 3,
    Passive = 4,
    HiZ = 5,
    OpenEmitter = 6,
    Power = 7,
}
```

Generates `TryFrom<u8>` that returns `Err(AltiumFormatError::UnknownEnumVariant)`
for any value outside the defined variants. **No default fallback.** An unknown
enum value means either Altium added a new variant or our enum is wrong. Both
require code changes to fix, not silent data loss.

---

## 7. altium-format-ops

High-level operations on Altium documents. Depends on `altium-format` (public
API only). Has zero access to serialization internals.

```
crates/altium-format-ops/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── summary.rs         # Document summary (component count, net count, etc.)
    ├── query.rs           # Find components/nets/pins by name, pattern, etc.
    ├── bom.rs             # Bill of Materials extraction
    ├── diff.rs            # Structural diff between two documents
    ├── validate.rs        # Validate document integrity (dangling OwnerIndex, etc.)
    └── edit.rs            # Higher-level editing operations
```

---

## 8. altium-cli

Binary crate. Depends on `altium-format-ops` (and transitively on
`altium-format`).

```
crates/altium-cli/
├── Cargo.toml
└── src/
    ├── main.rs
    ├── commands/
    │   ├── mod.rs
    │   ├── info.rs        # Show file summary
    │   ├── list.rs        # List components, nets, pins, etc.
    │   ├── inspect.rs     # Detailed record inspection
    │   ├── validate.rs    # Run validation checks
    │   ├── bom.rs         # Export BOM
    │   └── diff.rs        # Diff two files
    └── output/
        ├── mod.rs
        ├── table.rs       # Table formatting
        └── json.rs        # JSON output
```

---

## 9. Feature Flags

Minimal. The default builds everything.

### altium-format

| Feature | Default | Description |
|---------|---------|-------------|
| `sch` | yes | Include schematic types (sch/ module) |
| `pcb` | yes | Include PCB types (pcb/ module) |

Both are on by default. Disabling one cuts compile time and binary size for
applications that only need one domain.

The `common/`, `container/`, `params/`, `binary/`, and `error` modules are
always compiled (they are small and shared).

No `serde` feature. If JSON output is needed, it belongs in `altium-format-ops`
or `altium-cli`, not in the core parsing library.

### altium-format-ops and altium-cli

No feature flags.

---

## 10. Third-Party Dependencies

### altium-format-derive

| Crate | Purpose |
|-------|---------|
| `proc-macro2` | Token stream manipulation |
| `quote` | Quasi-quoting for code generation |
| `syn` | Rust syntax parsing |

### altium-format

| Crate | Purpose |
|-------|---------|
| `cfb` | OLE/CFB container reading/writing |
| `flate2` | Zlib compression/decompression |
| `encoding_rs` | Windows-1252 encoding |
| `thiserror` | Error derive macros |

Notably absent:
- No `indexmap` -- we don't preserve key order because we don't do round-trip
  preservation of unknown fields. We parse into typed structs and serialize
  back from typed structs.
- No `byteorder` -- Rust's built-in `from_le_bytes`/`to_le_bytes` is
  sufficient.
- No `bitflags` -- we model flag fields as individual bools extracted from
  the bitmask, not as opaque bitflag types.

### altium-format-ops

| Crate | Purpose |
|-------|---------|
| `altium-format` | Core library |
| `thiserror` | Error types |

### altium-cli

| Crate | Purpose |
|-------|---------|
| `altium-format-ops` | Operations |
| `clap` | CLI argument parsing |
| `serde` + `serde_json` | JSON output formatting |

---

## 11. Design Decisions and Trade-offs

### Why no Unknown variant in SchRecord / PcbRecord?

The central design decision. Traditional parsers capture unknown records and
pass them through for round-trip fidelity. We reject them instead.

**Why:** Every record type in an Altium file carries semantic meaning. A
schematic wire (RECORD=27) defines an electrical connection. A PCB pad (ID=2)
defines a solderable copper area. If we encounter a record type we don't
recognize, we have two choices:

1. **Silently pass it through.** The record might define a critical electrical
   connection, a copper pour, a design rule. If we pass it through blindly and
   the user modifies nearby records, the unknown record's indices, coordinates,
   or net references might become stale. We've now produced a subtly corrupted
   file that will cost money to debug.

2. **Error immediately.** The developer adds support for the missing record
   type. The parser becomes complete. Every record is fully understood.

We choose option 2. The cost is that adding support for new record types
requires code changes. The benefit is that every file we successfully parse
is *fully* understood. There are no opaque blobs, no data we're carrying
around without comprehension.

### Why no unknown_fields on record structs?

Same reasoning, applied at the field level. If a schematic pin has a parameter
key we don't recognize, it might be a new Altium feature that affects
electrical connectivity. Silently ignoring it could cause a missing connection
in the output. We error, the developer adds the field, and the parser becomes
more complete.

### Why pub(crate) for serialization?

The CLAUDE.md mandate: "altium-format implementation details MUST BE KEPT
PRIVATE TO THE CRATE." Downstream code works with typed Rust structs, never
with parameter key strings or byte offsets. This means we can freely refactor
serialization internals without breaking the public API.

### Why pub fields on record structs?

Users need to read and modify record data directly. Getters/setters add
boilerplate without adding safety (the types themselves enforce validity --
`Coord` is always valid, `PinElectricalType` is always a valid variant, etc.).
Public fields keep the API simple and Rust-idiomatic.

### Why Vec-based storage instead of arena/ECS?

For an initial implementation, `Vec<SchRecord>` with index-based ownership
(`OwnerIndex`) is the simplest correct approach. If performance requires it
later, the internal representation can change without affecting the public API
(since serialization details are private).
