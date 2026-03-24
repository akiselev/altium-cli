> **Related docs**: [ops-design.md](ops-design.md) | [ops-lang-spec.md](ops-lang-spec.md) | [schlib-ops.md](schlib-ops.md) | [schdoc-ops.md](schdoc-ops.md) | [ops-e2e-gaps.md](ops-e2e-gaps.md) | [ops-lang-checklist.md](ops-lang-checklist.md)

# SchDoc Low-Level Operations

Low-level operations that `altium-format` exposes to `autopcb-spec` for SchDoc
manipulation. Each op is a flat enum variant in `SchDocLowOp`, executed sequentially
by `apply_schdoc_low_ops()`. Same flat-op architecture as SchLib — no state machine.

## Architecture

```
autopcb-spec                    altium-format
─────────────────                    ─────────────
HighOp (YAML/JSON)                   SchDocLowOp enum
  ↓ lower                              ↓ apply_schdoc_low_ops()
ComposedOp                           free functions (schdoc_append_*)
  ↓ lower                              ↓ push onto doc.records
SchDocLowOp ──────────────────────→  save() handles:
                                       • record ordering
                                       • OWNERINDEX (absolute, already set by ops)
                                       • text pin serialization
                                       • block encoding
                                       • Weight update
```

The key structural difference from SchLib: SchDoc uses a **flat global record list**
with **absolute OWNERINDEX** values. Records are pushed directly onto `doc.records`.
There are no per-component sub-storages, no pin sidecar streams, and no SectionKeys.

## SchDoc vs SchLib — Key Differences

| Aspect | SchDoc | SchLib |
|--------|--------|--------|
| CFB layout | Flat: `/FileHeader` + `/Storage` + `/Additional` | Per-component storages |
| Content stream | Single `/FileHeader` holds ALL records | Per-component `/<key>/Data` |
| OWNERINDEX | Global absolute into flat record list | Relative within component section |
| Pin format | Text parameters (flags=0x00) | Binary (flags=0x01) + 9 sidecar streams |
| Root record | Sheet (RECORD=31) at index 0 | No sheet; component is root |
| Font table | In Sheet record (RECORD=31) | In `/FileHeader` header block |
| Weight | `doc.records.len()` (all content records) | Sum of (child records + aliases) per component |
| Sheet-level objects | Wire, Bus, NetLabel, PowerPort, Junction, Port, etc. | Not present |
| OWNERINDEX for sheet objects | 0 (owned by Sheet) | N/A |
| OWNERINDEX for component children | Absolute index of parent component | 0 (component root, relative) |

## Execution Context

The SchDoc executor uses `SchDocExecCtx`:

```rust
struct SchDocExecCtx {
    refs: HashMap<String, usize>,           // designator → component index
    last_component: Option<usize>,          // implicit target for child ops
    chain_state: HashMap<usize, ImplChainState>,  // impl chain tracking per component
}
```

On construction, `refs` is pre-populated from existing `SchDesignator` records in the
document, so new ops can reference existing components by designator.

Component resolution: explicit `component_ref` → look up in `ctx.refs`. No ref →
use `ctx.last_component`.

Implementation chain tracking: `ImplChainState` tracks the record indices of the most
recent ImplementationList, Implementation, and ImplementationMap for each component,
ensuring correct OWNERINDEX chaining.

## Current Ops (implemented)

### Component Creation

| SchDocLowOp variant | Function | What it does |
|---------------------|----------|--------------|
| `CreateComponentRoot(ComponentRootOp)` | `schdoc_create_component_root()` | Creates SchComponent (RECORD=1), OWNERINDEX=0 (sheet-owned) |
| `CreateComponentDesignator(ComponentTextOp)` | `schdoc_append_designator()` | Appends SchDesignator (RECORD=34), NAME="Designator" |
| `CreateComponentComment(ComponentTextOp)` | `schdoc_append_comment()` | Appends SchParameter (RECORD=41), NAME="Comment" |
| `AddPin(PinOp)` | `schdoc_append_pin()` | Appends SchPin (RECORD=2) via text format |

### Implementation Chain

| SchDocLowOp variant | Function | What it does |
|---------------------|----------|--------------|
| `CreateImplementationList(ComponentRefOp)` | `schdoc_append_implementation_list()` | Appends SchImplementationList (RECORD=44), OWNERINDEX→component |
| `CreateImplementation(ImplementationOp)` | `schdoc_append_implementation()` | Appends SchImplementation (RECORD=45), OWNERINDEX→impl list |
| `CreateImplementationMap(ComponentRefOp)` | `schdoc_append_implementation_map()` | Appends SchImplementationMap (RECORD=46), OWNERINDEX→implementation |
| `CreateMapDefiner(MapDefinerOp)` | `schdoc_append_map_definer()` | Appends SchMapDefiner (RECORD=47), OWNERINDEX→impl map |
| `CreateParameterList(ComponentRefOp)` | `schdoc_append_parameter_list()` | Appends SchParameterList (RECORD=48), OWNERINDEX→implementation |

The implementation chain requires strict ordering: `CreateImplementationList` before
`CreateImplementation` before `CreateImplementationMap`/`CreateParameterList` before
`CreateMapDefiner`. Each step records its index in `chain_state` so the next step
can set the correct OWNERINDEX.

---

## New Ops Needed

### Phase 1: Connectivity Primitives

These are the core electrical objects that make SchDoc different from SchLib.
All are sheet-level objects with `OWNERINDEX=0`.

#### `AddWire`

Appends a RECORD=27 `SchWire` — an electrical connection between points.

```rust
pub struct WireOp {
    pub points: Vec<CoordPoint>,   // 2+ vertices
    pub color: Option<Color>,      // default: dark blue (Altium default)
    pub line_width: Option<i32>,   // default: 1 (Small)
}
```

**Domain logic hidden:**

| Concern | Detail |
|---------|--------|
| OWNERINDEX | Always 0 (sheet-owned) |
| 1-based indexed coords | `LOCATIONCOUNT=N`, `X1/Y1`..`XN/YN`, each with `_FRAC` |
| UniqueID | Generated automatically |
| Minimum points | Must have at least 2 points |
| LineWidth enum | 0=Smallest, 1=Small, 2=Medium, 3=Large |
| COLORREF | RGB → `0x00BBGGRR` at save time |
| Ordering anomaly | UniqueID exported BEFORE vertices (unlike most records) |

#### `AddBus`

Appends a RECORD=26 `SchBus` — a multi-signal connection.

```rust
pub struct BusOp {
    pub points: Vec<CoordPoint>,
    pub color: Option<Color>,
    pub line_width: Option<i32>,   // default: 1
}
```

**Domain logic hidden:**

| Concern | Detail |
|---------|--------|
| OWNERINDEX | Always 0 |
| Indexed coords | Same 1-based format as Wire |
| UniqueID | Generated automatically |
| Ordering anomaly | UniqueID exported AFTER vertices (opposite of Wire!) |

#### `AddNetLabel`

Appends a RECORD=25 `SchNetLabel` — names a net at a location.

```rust
pub struct NetLabelOp {
    pub location: CoordPoint,
    pub text: String,              // net name
    pub color: Option<Color>,
    pub font_id: Option<i32>,      // default: 1
    pub orientation: Option<i32>,  // 0, 90, 180, 270
    pub justification: Option<i32>, // 0-8
}
```

**Domain logic hidden:**

| Concern | Detail |
|---------|--------|
| OWNERINDEX | Always 0 |
| DXP fractional coords | LOCATION.X/Y + _FRAC |
| UniqueID | Generated automatically |
| Orientation bitmask | Packed from degrees |
| Justification enum | 3x3 grid: 0=BottomLeft..8=TopRight |

#### `AddPowerPort`

Appends a RECORD=17 `SchPowerObject` — a power/ground symbol (VCC, GND, etc.).

```rust
pub struct PowerPortOp {
    pub location: CoordPoint,
    pub text: String,                // net name ("VCC", "GND")
    pub style: String,               // enum: "bar", "arrow", "gnd_power", "gnd_signal", etc.
    pub orientation: Option<i32>,    // 0, 90, 180, 270
    pub color: Option<Color>,
    pub show_net_name: Option<bool>, // default: true
    pub is_cross_sheet_connector: Option<bool>, // default: false
    pub font_id: Option<i32>,
}
```

**Domain logic hidden:**

| Concern | Detail |
|---------|--------|
| OWNERINDEX | Always 0 |
| PowerObjectStyle enum | 0=Circle, 1=Arrow, 2=Bar, 3=Wave, 4=GndPower, 5=GndSignal, 6=GndEarth, 7-10=others |
| SYMBOLTYPE | Internal field, set from style |
| UniqueID | Generated automatically |
| Enum resolution | Case-insensitive: "gnd_power" → 4 |

#### `AddJunction`

Appends a RECORD=29 `SchJunction` — a wire junction dot.

```rust
pub struct JunctionOp {
    pub location: CoordPoint,
    pub color: Option<Color>,      // default: Altium junction color
}
```

**Domain logic hidden:**

| Concern | Detail |
|---------|--------|
| OWNERINDEX | Always 0 |
| No UniqueID | Junctions have NO UniqueID (unique among record types) |
| SIZE field | Default junction size |

#### `AddPort`

Appends a RECORD=18 `SchPort` — a sheet port connector.

```rust
pub struct PortOp {
    pub location: CoordPoint,
    pub name: String,
    pub io_type: String,           // enum: "unspecified", "output", "input", "bidirectional"
    pub style: Option<String>,     // enum: arrow style
    pub width: Option<Coord>,
    pub height: Option<Coord>,
    pub color: Option<Color>,
    pub area_color: Option<Color>,
    pub text_color: Option<Color>,
    pub font_id: Option<i32>,
    pub alignment: Option<i32>,
}
```

**Domain logic hidden:**

| Concern | Detail |
|---------|--------|
| OWNERINDEX | Always 0 |
| PortIoType enum | 0=Unspecified, 1=Output, 2=Input, 3=Bidirectional |
| PortArrowStyle enum | Arrow shape variants |
| UniqueID | Generated automatically |

#### `AddNoConnect`

Appends a RECORD=22 `SchNoConnect` — a no-connect/no-ERC marker.

```rust
pub struct NoConnectOp {
    pub location: CoordPoint,
    pub color: Option<Color>,
    pub orientation: Option<i32>,
    pub symbol: Option<String>,    // "Thin Cross", "Checkbox", etc.
}
```

**Domain logic hidden:**

| Concern | Detail |
|---------|--------|
| OWNERINDEX | Always 0 |
| SYMBOL field | String identifier for visual style |
| ISACTIVE | Default: true |
| SUPPRESSALL | Default: false |
| UniqueID | Generated automatically |

#### `AddBusEntry`

Appends a RECORD=37 `SchBusEntry` — a bus tap connector.

```rust
pub struct BusEntryOp {
    pub location: CoordPoint,
    pub corner: CoordPoint,
    pub color: Option<Color>,
    pub line_width: Option<i32>,
}
```

**Domain logic hidden:**

| Concern | Detail |
|---------|--------|
| OWNERINDEX | Always 0 |
| UniqueID | Generated automatically |
| Ordering anomaly | UniqueID exported FIRST (before Location/Corner) |

---

### Phase 2: Hierarchy Primitives

#### `AddSheetSymbol`

Appends a RECORD=15 `SchSheetSymbol` — a hierarchical sheet reference.

```rust
pub struct SheetSymbolOp {
    pub location: CoordPoint,      // top-left corner
    pub x_size: Coord,
    pub y_size: Coord,
    pub sheet_name: String,
    pub file_name: String,         // path to child .SchDoc
    pub color: Option<Color>,
    pub area_color: Option<Color>,
    pub is_solid: Option<bool>,
    pub line_width: Option<i32>,
}
```

**Domain logic hidden:**

| Concern | Detail |
|---------|--------|
| OWNERINDEX | Always 0 |
| UniqueID | Uses `Export_String` (not `Export_DynamicString`), defaults to `"$$$"` when empty |
| SYMBOLTYPE | Internal classification field |
| CORNER.X/Y | Computed from location + size |

#### `AddSheetEntry`

Appends a RECORD=16 `SchSheetEntry` — a port on a sheet symbol.

```rust
pub struct SheetEntryOp {
    pub component_ref: Option<RefExpr>,  // targets the parent SheetSymbol
    pub name: String,
    pub io_type: String,           // enum: "unspecified", "output", "input", "bidirectional"
    pub side: Option<i32>,         // 0=Left, 1=Right, 2=Top, 3=Bottom
    pub style: Option<String>,     // arrow style
    pub distance_from_top: Option<Coord>,
    pub color: Option<Color>,
    pub text_color: Option<Color>,
    pub font_id: Option<i32>,
}
```

**Domain logic hidden:**

| Concern | Detail |
|---------|--------|
| OWNERINDEX | Points to parent SheetSymbol's absolute index |
| PortIoType enum | 0=Unspecified, 1=Output, 2=Input, 3=Bidirectional |
| LeftRightSide enum | 0=Left, 1=Right, 2=Top, 3=Bottom |
| DISTANCEFROMTOP | Coord positioning relative to parent |
| UniqueID | Generated automatically |

---

### Phase 3: Annotation Primitives

#### `AddParameterSet`

Appends a RECORD=43 `SchParameterSet` — a parameter marker on wires.

```rust
pub struct ParameterSetOp {
    pub location: CoordPoint,
    pub name: String,
    pub style: Option<i32>,
    pub orientation: Option<i32>,
    pub color: Option<Color>,
}
```

**Domain logic hidden:**

| Concern | Detail |
|---------|--------|
| OWNERINDEX | Typically 0 (sheet-owned) or points to a wire |
| UniqueID | Generated automatically |

#### `AddNote`

Appends a RECORD=209 `SchNote` — an annotation note box.

```rust
pub struct NoteOp {
    pub location: CoordPoint,
    pub corner: CoordPoint,
    pub text: String,
    pub author: Option<String>,
    pub color: Option<Color>,
    pub area_color: Option<Color>,
    pub text_color: Option<Color>,
    pub font_id: Option<i32>,
    pub is_solid: Option<bool>,
    pub show_border: Option<bool>,
    pub word_wrap: Option<bool>,
    pub text_margin: Option<i32>,
}
```

**Domain logic hidden:**

| Concern | Detail |
|---------|--------|
| OWNERINDEX | Always 0 |
| UniqueID | Generated automatically |
| RECORD encoding | RECORD=254, RECORDEX=209 (since 209 < 256, actually: RECORD=209 is < 256 so standard encoding) |
| Note: RECORD >= 256 encoding | `SchNote` uses RECORD=255 internally → written as `RECORD=254|RECORDEX=255` |

#### `AddProbe`

Appends a RECORD=210 `SchProbe` — a simulation probe.

```rust
pub struct ProbeOp {
    pub location: CoordPoint,
    pub name: String,
    pub orientation: Option<i32>,
    pub color: Option<Color>,
}
```

**Domain logic hidden:**

| Concern | Detail |
|---------|--------|
| OWNERINDEX | Always 0 |
| RECORD encoding | RECORD=254, RECORDEX=210 |
| UniqueID | Generated automatically |

---

### Phase 4: Graphics Primitives (shared with SchLib)

SchDoc uses the same graphics primitives as SchLib, but they are typically
sheet-owned (OWNERINDEX=0) rather than component-owned. The op structs are
identical to those defined in `docs/schlib-ops.md`:

- `AddLine` (RECORD=13)
- `AddRectangle` (RECORD=14)
- `AddArc` (RECORD=12)
- `AddEllipticalArc` (RECORD=11)
- `AddEllipse` (RECORD=8)
- `AddPolyline` (RECORD=6)
- `AddPolygon` (RECORD=7)
- `AddBezier` (RECORD=5)
- `AddPie` (RECORD=9)
- `AddRoundRectangle` (RECORD=10)
- `AddLabel` (RECORD=4)
- `AddTextFrame` (RECORD=28)
- `AddImage` (RECORD=30)

The difference is that SchDoc graphics can be either sheet-level (OWNERINDEX=0,
standalone annotations) or component-level (OWNERINDEX=component index, part of
a placed component). The op needs an optional `component_ref` to distinguish.

#### `AddParameter`

Appends a RECORD=41 `SchParameter` with arbitrary NAME/TEXT. Same as SchLib but
can be sheet-level or component-level.

```rust
pub struct ParameterOp {
    pub component_ref: Option<RefExpr>,  // None = sheet-level (OWNERINDEX=0)
    pub name: String,
    pub text: String,
    pub is_hidden: Option<bool>,
}
```

---

### Phase 5: Mutation Ops

#### `EditComponent`

Modifies fields on a component matching a selector.

```rust
pub struct EditComponentOp {
    pub selector: String,          // designator pattern, e.g. "R1"
    pub value: Option<String>,     // update Comment parameter
    pub designator: Option<String>, // update Designator text
    pub location: Option<CoordPoint>,
    pub orientation: Option<i32>,
}
```

**Domain logic hidden:**

| Concern | Detail |
|---------|--------|
| Selector resolution | Find component by designator in flat record list |
| Child record updates | Changing value requires finding the child Parameter with NAME="Comment" |
| OWNERINDEX stability | Editing doesn't change record indices |
| Weight stability | Editing doesn't change record count |

#### `EditRecord`

Modifies fields on records matching a selector.

```rust
pub struct EditRecordOp {
    pub selector: RecordSelector,
    pub patch: RecordPatch,
}
```

#### `RemoveRecords`

Removes records matching a selector, cascading to children.

```rust
pub struct RemoveRecordsOp {
    pub selector: RecordSelector,
}
```

**Domain logic hidden:**

| Concern | Detail |
|---------|--------|
| Cascade delete | Removing a component removes all children (pins, designator, params, impl chain) |
| OWNERINDEX renumbering | ALL OWNERINDEX values after removed records must shift (global absolute indices) |
| Weight update | `doc.header.weight = doc.records.len()` |
| This is more complex than SchLib | SchLib removes from a component's local record list; SchDoc must fix up the entire flat list |

---

### Phase 6: Query Ops (read-only)

#### `QueryComponents`

Lists components with semantic info.

```rust
pub struct ComponentInfo {
    pub index: usize,
    pub designator: String,
    pub value: String,
    pub lib_reference: String,
    pub location: CoordPoint,
    pub pin_count: i32,
    pub has_footprint: bool,
}
```

**Domain logic hidden:**

| Concern | Detail |
|---------|--------|
| Designator/Value lookup | Must find child Designator and Comment Parameter records by OWNERINDEX |
| Pin counting | Count SchRecord::Pin with matching OWNERINDEX |
| Footprint detection | Check for ImplementationList child |

#### `QueryNets`

Returns net connectivity information.

```rust
pub struct NetInfo {
    pub name: String,
    pub wire_count: usize,
    pub connected_pins: Vec<String>,  // "U1-1", "R1-2", etc.
}
```

#### `QueryRecords`

General record query.

```rust
pub struct RecordInfo {
    pub index: usize,
    pub record_type: i32,
    pub owner_index: i32,
    pub summary: String,
}
```

---

## What `save()` Handles (not in ops)

| Concern | Detail |
|---------|-------|
| Record ordering | Sheet (RECORD=31) at index 0, Template (RECORD=39) at index 1, then depth-first traversal with `SchDataObjectComparator` |
| Text pin serialization | Pins serialized as parameter text blocks (not binary like SchLib) |
| PinConglomerate packing | Orientation + visibility flags packed into bitmask field |
| DXP fractional coords | Integer + `_FRAC` split at save time |
| COLORREF encoding | RGB → `0x00BBGGRR` at save time |
| Block headers | 4-byte LE: flags(8b) \| size(24b), always flags=0x00 for SchDoc |
| Windows-1252 encoding | Applied to all parameter text at save time |
| `%UTF8%` prefix | Applied to Unicode parameter keys at save time |
| NUL terminator | Appended to each parameter block |
| RECORD >= 256 encoding | Written as `RECORD=254` + `RECORDEX=<actual_value>` |
| Tier 1/2 serialization | Per-field default-skipping rules hardcoded per record type |
| Parameter ordering | Hardcoded per record type per FileFormatV5.cs |
| /Storage stream | Embedded images with zlib + 0xD0 envelope |
| /Additional stream | RECORD=225 dashed rectangle records |
| FileHeader header block | HEADER + WEIGHT + MinorVersion + UniqueID |
| Font table | Serialized into Sheet record (RECORD=31) |
| Auto-junctions | `AddAutoJunctions()` appended at end of warehouse on save |
| Special object ordering | `MoveSpecialObjectsToTop()` for Sheet/Template |

## OWNERINDEX Model

All records live in a single flat list. OWNERINDEX values are absolute indices:

```
Index 0:  SchSheet           (OWNERINDEX absent, root)
Index 1:  SchTemplate        (OWNERINDEX=0, sheet-owned)
Index 2:  SchComponent       (OWNERINDEX=0, sheet-owned)
Index 3:  SchDesignator      (OWNERINDEX=2, component-owned)
Index 4:  SchParameter       (OWNERINDEX=2, NAME="Comment")
Index 5:  SchPin             (OWNERINDEX=2, pin 1)
Index 6:  SchPin             (OWNERINDEX=2, pin 2)
Index 7:  SchImplList        (OWNERINDEX=2)
Index 8:  SchImplementation  (OWNERINDEX=7)
Index 9:  SchImplMap         (OWNERINDEX=8)
Index 10: SchMapDefiner      (OWNERINDEX=9)
Index 11: SchMapDefiner      (OWNERINDEX=9)
Index 12: SchParameterList   (OWNERINDEX=8)
Index 13: SchWire            (OWNERINDEX=0, sheet-owned)
Index 14: SchNetLabel        (OWNERINDEX=0, sheet-owned)
Index 15: SchPowerObject     (OWNERINDEX=0, sheet-owned)
Index 16: SchJunction        (OWNERINDEX=0, sheet-owned)
```

When ops append records, they push to `doc.records` and set OWNERINDEX to the
absolute index of the parent. For sheet-level objects (wires, netlabels, etc.)
this is 0. For component children, it's the component's index. For implementation
chain records, it's tracked via `ImplChainState` in the execution context.

**Warning**: Removing records from the middle of the list requires renumbering ALL
subsequent OWNERINDEX values. This makes `RemoveRecords` significantly more complex
for SchDoc than for SchLib (where records are in per-component lists).

## Implementation Notes

### Pin Format Difference

SchDoc pins use **text parameter format** (same pipe-delimited `|KEY=VALUE|` as all
other records). This is fundamentally different from SchLib which uses binary pin
format (flags=0x01, packed struct with pascal strings).

The existing `schdoc_append_pin()` function uses `parse_text_pin()` to construct
the pin from a `ParameterCollection`, which is the correct approach for SchDoc.

### No Sidecar Streams

SchDoc has **no pin sidecar streams**. All pin data (coordinates, text, symbols) is
inline in the text parameters. This simplifies pin operations significantly compared
to SchLib — no PinFrac, PinDesc, PinWideText, etc. to worry about.

### Weight Calculation

SchDoc weight = `doc.records.len()` (total record count including Sheet and Template).
Updated at the end of `apply_schdoc_low_ops()`:

```rust
doc.header.weight = doc.records.len() as i32;
```

### Implementation Chain OWNERINDEX Tracking

The implementation chain ops use `ImplChainState` to track record indices:

```rust
struct ImplChainState {
    impl_list: Option<usize>,        // index of ImplementationList
    implementation: Option<usize>,   // index of Implementation
    implementation_map: Option<usize>, // index of ImplementationMap
}
```

Each step validates that its prerequisite was created:
- `CreateImplementation` requires `impl_list` to be set
- `CreateImplementationMap` requires `implementation` to be set
- `CreateMapDefiner` requires `implementation_map` to be set
- `CreateParameterList` requires `implementation` to be set

This ensures the OWNERINDEX chain is always valid.

## Recommended Implementation Priority

**Phase 1 — Connectivity (core SchDoc value)**:
1. `AddWire` + `AddNetLabel` (most common operations for circuit editing)
2. `AddPowerPort` + `AddJunction`
3. `AddBus` + `AddBusEntry` + `AddPort` + `AddNoConnect`

**Phase 2 — Hierarchy**:
4. `AddSheetSymbol` + `AddSheetEntry`

**Phase 3 — Annotations**:
5. `AddNote` + `AddParameterSet` + `AddProbe`

**Phase 4 — Shared graphics** (reuse SchLib implementations):
6. Graphics primitives with sheet-level OWNERINDEX support

**Phase 5 — Mutations**:
7. `EditComponent` / `EditRecord`
8. `RemoveRecords` (complex due to global OWNERINDEX renumbering)

**Phase 6 — Queries**:
9. `QueryComponents` + `QueryNets` + `QueryRecords`
