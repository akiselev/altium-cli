# Phase 3: Record Types

**Agents: 3 parallel tracks (3A, 3B, 3C)**
**Blocked by: Phase 2 (macro system)**

Each track implements record types using the `#[altium_record]` and `#[altium_enum]` macros from Phase 2. Agents reference the current v2 field definitions in `_v2_reference/fields/` and `_v2_reference/types.rs` for param keys and field mappings.

**IMPORTANT:** Record types do NOT need document I/O to work. They can be created from `RecordOrigin` directly and tested standalone.

---

## Track 3A: Core Schematic Records + Enums

**Files:**
- `crates/altium-format/src/v2/records/mod.rs`
- `crates/altium-format/src/v2/records/enums.rs`
- `crates/altium-format/src/v2/records/sch_pin.rs`
- `crates/altium-format/src/v2/records/sch_component.rs`
- `crates/altium-format/src/v2/records/sch_arc.rs`
- `crates/altium-format/src/v2/records/sch_line.rs`
- `crates/altium-format/src/v2/records/sch_rectangle.rs`
- `crates/altium-format/src/v2/records/sch_ellipse.rs`
- `crates/altium-format/src/v2/records/sch_polygon.rs`
- `crates/altium-format/src/v2/records/sch_polyline.rs`
- `crates/altium-format/src/v2/records/sch_bezier.rs`
- `crates/altium-format/src/v2/records/sch_image.rs`
- `crates/altium-format/src/v2/records/sch_round_rectangle.rs`
- `crates/altium-format/src/v2/records/sch_elliptical_arc.rs`
- `crates/altium-format/src/v2/records/sch_pie.rs`
- `crates/altium-format/src/v2/records/sch_parameter.rs`
- `crates/altium-format/src/v2/records/sch_designator.rs`

**Reference:**
- `_v2_reference/fields/pin.rs` — PinData fields and param keys
- `_v2_reference/fields/component.rs` — ComponentData fields
- `_v2_reference/fields/primitives.rs` — Arc, Line, Rectangle, etc.
- `_v2_reference/fields/parameter.rs` — ParameterData
- `_v2_reference/types.rs` — All enum definitions with values
- `_v2_reference/consts.rs` — Parameter name constants

### Enum Types (enums.rs)

Recreate ALL existing enums with `#[altium_enum]`:

```rust
// Each enum gets #[altium_enum] + standard derives
#[altium_enum]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PinElectricalType {
    Input = 0,
    IO = 1,
    Output = 2,
    OpenCollector = 3,
    Passive = 4,
    HiZ = 5,
    OpenEmitter = 6,
    Power = 7,
}
```

**Full enum list** (from `_v2_reference/types.rs`):
- `PinElectricalType` (0-7)
- `RotationBy90` (0-3)
- `LineStyle` (0-3)
- `IeeeSymbol` (0-34, sparse)
- `PortArrowStyle` (0-7)
- `PortIO` (0-3)
- `PowerObjectStyle` (0-10)
- `TextJustification` (0-8)
- `SheetStyle` (0-17)
- `PinItemMode` (0-1)
- `PinTextRotationAnchor` (0-1)
- `ComponentKind` (0, 1, 2, 5, 6 — sparse)
- `Size` (0-3)
- `NoERCSymbol` (0-4)
- `ParameterType` (0-1)
- `ParameterReadOnlyState` (0-1)
- `StdLogicState` (0-8)
- `HorizontalAlign` (0-2)
- `LineShape` (0-6)
- `LeftRightSide` (0-1)
- `ParameterSetStyle` (0-1)
- `TextHorzAnchor` (0-3)
- `TextVertAnchor` (0-3)
- `ObjectId` (0-115 — PCB object type IDs)

Also create **bitflags** types:
```rust
bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct PinConglomerateFlags: u32 {
        // Extract specific flags from _v2_reference sources
    }
}
```

Bitflags need manual `ParamCodec` impl using `.bits()` / `from_bits_truncate()`.

### Record Types

For each record type, create a file using `#[altium_record]`. Example for `SchPinRecord`:

```rust
// sch_pin.rs
use crate::v2::backing_store::RecordOrigin;
use crate::v2::coord::SchCoord;
use crate::v2::newtypes::*;
use crate::v2::records::enums::*;

#[altium_record(kind = "sch", record_id = 2, codec = "params")]
pub struct SchPinRecord {
    #[altium(key = "DESIGNATOR")]
    designator: Designator,

    #[altium(key = "NAME")]
    name: PinName,

    #[altium(key = "PINLENGTH")]
    pin_length: SchCoord,

    #[altium(key = "ELECTRICAL")]
    electrical: PinElectricalType,

    #[altium(key = "ORIENTATION")]
    orientation: RotationBy90,

    #[altium(key = "LOCATION.X")]
    location_x: SchCoord,

    #[altium(key = "LOCATION.Y")]
    location_y: SchCoord,

    #[altium(key = "ISHIDDEN")]
    is_hidden: bool,

    #[altium(key = "SHOWNAME")]
    show_name: bool,

    #[altium(key = "SHOWDESIGNATOR")]
    show_designator: bool,

    #[altium(key = "DESCRIPTION")]
    description: Description,

    #[altium(key = "PINCONGLOMERATE")]
    pin_conglomerate: PinConglomerateFlags,

    #[altium(key = "SYMBOLINNEREDGE")]
    symbol_inner_edge: IeeeSymbol,

    #[altium(key = "SYMBOLOUTEREDGE")]
    symbol_outer_edge: IeeeSymbol,

    #[altium(key = "SYMBOLINNER")]
    symbol_inner: IeeeSymbol,

    #[altium(key = "SYMBOLOUTER")]
    symbol_outer: IeeeSymbol,

    #[altium(key = "FORMALTYPE")]
    formal_type: StdLogicState,

    #[altium(key = "OWNERINDEX")]
    owner_index: i32,

    #[altium(key = "OWNERPARTID")]
    owner_part_id: i16,

    #[altium(key = "UNIQUEID")]
    unique_id: UniqueId,

    // ... extract ALL fields from _v2_reference/fields/pin.rs
}
```

**Record types to implement in this track** (with RECORD IDs from `_v2_reference/types.rs`):

| Record | ID | Reference File |
|---|---|---|
| `SchPinRecord` | 2 | `fields/pin.rs` |
| `SchComponentRecord` | 1 | `fields/component.rs` |
| `SchArcRecord` | 12 | `fields/primitives.rs` |
| `SchLineRecord` | 13 | `fields/primitives.rs` |
| `SchRectangleRecord` | 14 | `fields/primitives.rs` |
| `SchEllipseRecord` | 8 | `fields/primitives.rs` |
| `SchPolygonRecord` | 7 | `fields/primitives.rs` |
| `SchPolylineRecord` | 6 | `fields/primitives.rs` |
| `SchBezierRecord` | 5 | `fields/primitives.rs` |
| `SchImageRecord` | 30 | `fields/primitives.rs` |
| `SchRoundRectangleRecord` | 10 | `fields/primitives.rs` |
| `SchEllipticalArcRecord` | 11 | `fields/primitives.rs` |
| `SchPieRecord` | 9 | `fields/primitives.rs` |
| `SchParameterRecord` | 41 | `fields/parameter.rs` |
| `SchDesignatorRecord` | 34 | `fields/parameter.rs` |

### records/mod.rs

```rust
pub mod enums;
pub mod sch_pin;
pub mod sch_component;
pub mod sch_arc;
pub mod sch_line;
pub mod sch_rectangle;
// ... etc

pub use enums::*;
pub use sch_pin::SchPinRecord;
pub use sch_component::SchComponentRecord;
// ... etc

/// Dispatch function: given a record ID and origin, return boxed record
/// (used by document parsers in Phase 4)
pub fn record_id_for_params(params: &ParameterCollection) -> Option<u8> {
    params.get("RECORD").map(|v| v.as_int_or(0) as u8)
}
```

### Tests per Record

Each record file should have inline tests:
- Create from `ParamOrigin` with known params
- Verify getter returns correct typed value
- Verify setter writes to backing store
- Verify update closure works
- Verify roundtrip: create → set fields → read back → values match

### Acceptance Criteria

- [ ] All 24+ enum types recreated with `#[altium_enum]`
- [ ] All 15 core schematic record types implemented with `#[altium_record]`
- [ ] Each record has getter/setter/updater for all fields
- [ ] Each record has inline roundtrip tests
- [ ] `records/mod.rs` re-exports all types
- [ ] `cargo check` passes

---

## Track 3B: Additional Schematic Records

**Files:**
- `crates/altium-format/src/v2/records/sch_wire.rs`
- `crates/altium-format/src/v2/records/sch_bus.rs`
- `crates/altium-format/src/v2/records/sch_bus_entry.rs`
- `crates/altium-format/src/v2/records/sch_junction.rs`
- `crates/altium-format/src/v2/records/sch_net_label.rs`
- `crates/altium-format/src/v2/records/sch_power.rs`
- `crates/altium-format/src/v2/records/sch_port.rs`
- `crates/altium-format/src/v2/records/sch_no_erc.rs`
- `crates/altium-format/src/v2/records/sch_label.rs`
- `crates/altium-format/src/v2/records/sch_text_frame.rs`
- `crates/altium-format/src/v2/records/sch_symbol.rs`
- `crates/altium-format/src/v2/records/sch_note.rs`
- `crates/altium-format/src/v2/records/sch_sheet.rs`
- `crates/altium-format/src/v2/records/sch_sheet_symbol.rs`
- `crates/altium-format/src/v2/records/sch_sheet_entry.rs`
- `crates/altium-format/src/v2/records/sch_sheet_name.rs`
- `crates/altium-format/src/v2/records/sch_sheet_filename.rs`
- `crates/altium-format/src/v2/records/sch_implementation.rs`
- `crates/altium-format/src/v2/records/sch_implementation_list.rs`
- `crates/altium-format/src/v2/records/sch_blanket.rs`
- `crates/altium-format/src/v2/records/sch_hyperlink.rs`

**Reference:**
- `_v2_reference/fields/schematic.rs` — Wire, Bus, Port, Power, etc.
- `_v2_reference/fields/sheet.rs` — Sheet, SheetSymbol, SheetEntry
- `_v2_reference/fields/block.rs`, `implementation.rs`, `harness.rs`, `misc.rs`

### Record Types

| Record | ID | Reference |
|---|---|---|
| `SchWireRecord` | 27 | `fields/schematic.rs` |
| `SchBusRecord` | 26 | `fields/schematic.rs` |
| `SchBusEntryRecord` | 37 | `fields/schematic.rs` |
| `SchJunctionRecord` | 29 | `fields/schematic.rs` |
| `SchNetLabelRecord` | 25 | `fields/schematic.rs` |
| `SchPowerRecord` | 17 | `fields/schematic.rs` |
| `SchPortRecord` | 18 | `fields/schematic.rs` |
| `SchNoERCRecord` | 22 | `fields/schematic.rs` |
| `SchLabelRecord` | 4 | `fields/schematic.rs` |
| `SchTextFrameRecord` | 28 | `fields/schematic.rs` |
| `SchSymbolRecord` | 3 | `fields/schematic.rs` |
| `SchNoteRecord` | 209 | `fields/schematic.rs` |
| `SchSheetRecord` | 31 | `fields/sheet.rs` |
| `SchSheetSymbolRecord` | 39 | `fields/sheet.rs` |
| `SchSheetEntryRecord` | 40 | `fields/sheet.rs` |
| `SchSheetNameRecord` | 32 | `fields/sheet.rs` |
| `SchSheetFileNameRecord` | 33 | `fields/sheet.rs` |
| `SchImplementationRecord` | 45 | `fields/implementation.rs` |
| `SchImplementationListRecord` | 44 | `fields/implementation.rs` |
| `SchBlanketRecord` | 215 | `fields/misc.rs` |
| `SchHyperlinkRecord` | — | `fields/misc.rs` |

Same pattern as Track 3A: use `#[altium_record]`, create from `_v2_reference` field definitions, add inline tests.

### Acceptance Criteria

- [ ] All 21 additional schematic record types implemented
- [ ] Each has getter/setter/updater for all fields
- [ ] Each has inline roundtrip tests
- [ ] Added to `records/mod.rs` re-exports
- [ ] `cargo check` passes

---

## Track 3C: PCB Record Types

**Files:**
- `crates/altium-format/src/v2/records/pcb_pad.rs`
- `crates/altium-format/src/v2/records/pcb_track.rs`
- `crates/altium-format/src/v2/records/pcb_arc.rs`
- `crates/altium-format/src/v2/records/pcb_via.rs`
- `crates/altium-format/src/v2/records/pcb_fill.rs`
- `crates/altium-format/src/v2/records/pcb_text.rs`
- `crates/altium-format/src/v2/records/pcb_region.rs`
- `crates/altium-format/src/v2/records/pcb_component_body.rs`
- `crates/altium-format/src/v2/records/pcb_footprint.rs`
- `crates/altium-format/src/v2/records/pcb_enums.rs`

**Reference:**
- `_v2_reference/pcb/pad.rs` — PcbPad binary structure
- `_v2_reference/pcb/primitive.rs` — PcbCommonHeader, trailing fields
- `_v2_reference/pcb/enums.rs` — PCB-specific enums
- `_v2_reference/pcb/io/pcblib.rs` — Binary record dispatch table

### PCB Enum Types (pcb_enums.rs)

```rust
#[altium_enum]
pub enum PcbPadShape { Round = 0, Rectangular = 1, Octagonal = 2, RoundedRectangle = 3 }

#[altium_enum]
pub enum PcbObjectId { Arc = 1, Pad = 2, Via = 3, Track = 4, Text = 5, Fill = 6, ... }
```

### Simple Binary Records (sequential layout)

```rust
#[altium_record(kind = "pcb", object_id = Track, codec = "binary")]
pub struct PcbTrackRecord {
    #[altium(header)]
    header: PcbCommonHeader,
    start_x: PcbCoord,
    start_y: PcbCoord,
    end_x: PcbCoord,
    end_y: PcbCoord,
    width: PcbCoord,
    subpoly_index: u16,
}
```

Same for `PcbArcRecord`, `PcbFillRecord`.

### Complex Binary Records (custom parser)

```rust
#[altium_record(kind = "pcb", object_id = Pad, codec = "binary",
    parse_fn = "parse_pad", serialize_fn = "serialize_pad")]
pub struct PcbPadRecord {
    name: String,          // PadName
    position_x: PcbCoord,
    position_y: PcbCoord,
    top_size_x: PcbCoord,
    top_size_y: PcbCoord,
    // ... all fields from _v2_reference/pcb/pad.rs
    hole_size: PcbCoord,
    rotation: f64,
    is_plated: bool,
}

// Hand-written parser
fn parse_pad(data: &[u8]) -> crate::Result<RecordOrigin> {
    // Parse 6 subrecords, build BinaryOrigin with field_spans
    // Reference: _v2_reference/pcb/pad.rs PcbPad parsing code
    todo!()
}

fn serialize_pad(origin: &BinaryOrigin) -> crate::Result<Vec<u8>> {
    // Reconstruct 6 subrecords from raw_block
    todo!()
}
```

Same custom-parser pattern for `PcbViaRecord`, `PcbTextRecord`, `PcbRegionRecord`, `PcbComponentBodyRecord`.

### PcbFootprintRecord (Metadata)

The footprint metadata is **param-based** (from the Parameters stream):

```rust
#[altium_record(kind = "pcb", object_id = Footprint, codec = "params")]
pub struct PcbFootprintRecord {
    #[altium(key = "PATTERN")]
    pattern: String,
    #[altium(key = "DESCRIPTION")]
    description: Description,
    #[altium(key = "HEIGHT")]
    height: PcbCoord,
    // ... extract from _v2_reference/pcb/io/pcblib.rs Parameters stream parsing
}
```

### Acceptance Criteria

- [ ] PCB enum types created with `#[altium_enum]`
- [ ] Simple binary records (Track, Arc, Fill) use sequential layout
- [ ] Complex binary records (Pad, Via, Text, Region, ComponentBody) have custom parsers
- [ ] `PcbFootprintRecord` uses param-based codec
- [ ] All records have inline tests
- [ ] `cargo check` passes
