> **Related docs**: [ops-design.md](ops-design.md) | [ops-lang-spec.md](ops-lang-spec.md) | [schlib-ops.md](schlib-ops.md) | [schdoc-ops.md](schdoc-ops.md) | [ops-e2e-gaps.md](ops-e2e-gaps.md) | [ops-lang-checklist.md](ops-lang-checklist.md)

# SchLib Low-Level Operations

Low-level operations that `altium-format` exposes to `autopcb-spec` for SchLib
manipulation. Each op is a flat enum variant in `SchLibLowOp`, executed sequentially
by `apply_schlib_low_ops()`. No state machine — components are created atomically,
children appended one at a time, and `save()` handles all serialization complexity.

## Architecture

```
autopcb-spec                    altium-format
─────────────────                    ─────────────
HighOp (YAML/JSON)                   SchLibLowOp enum
  ↓ lower                              ↓ apply_schlib_low_ops()
ComposedOp                           ops_* methods on SchLib
  ↓ lower                              ↓ mutate in-memory structs
SchLibLowOp ──────────────────────→  save() handles:
                                       • OWNERINDEX assignment
                                       • record ordering
                                       • pin sidecar generation
                                       • SectionKeys / CFB keys
                                       • block encoding
                                       • PinConglomerate packing
                                       • weight recomputation
```

The ops crate never sees internal record types (`SchRecord`, `SchComponent`, etc.).
It constructs `SchLibLowOp` values and calls into `sch_ops_core`. The format crate
owns all legacy details.

## Current Ops (implemented)

| SchLibLowOp variant | SchLib method | What it does |
|---------------------|---------------|--------------|
| `CreateComponentRoot(ComponentRootOp)` | `ops_append_component_root()` | Creates SchComponent (RECORD=1) + header index entry |
| `CreateComponentDesignator(ComponentTextOp)` | `ops_append_designator()` | Appends SchDesignator (RECORD=34), NAME="Designator" |
| `CreateComponentComment(ComponentTextOp)` | `ops_append_comment()` | Appends SchParameter (RECORD=41), NAME="Comment" |
| `AddPin(PinOp)` | `ops_append_pin()` | Appends SchPin (RECORD=2), updates all_pin_count + weight |

## Execution Context

Each op targets a component by `component_ref: Option<RefExpr>`. Resolution order:

1. Explicit ref → look up in `ctx.refs` (batch-placed components) or `lib.ops_find_component_index_by_ref()` (existing components)
2. No ref → use `ctx.last_component` (most recently created component)

The executor maintains `SchLibExecCtx { refs: HashMap<String, usize>, last_component: Option<usize> }`.

---

## New Ops Needed

### Phase 1: Complete the Component Chain

#### `AddParameter`

Appends an arbitrary RECORD=41 `SchParameter` with user-specified NAME and TEXT.

```rust
pub struct ParameterOp {
    pub component_ref: Option<RefExpr>,
    pub name: String,
    pub text: String,
    pub is_hidden: Option<bool>,
}
```

**Domain logic hidden by `ops_append_parameter()`:**

| Concern | Detail |
|---------|--------|
| NAME/TEXT convention | Comment and Designator have special NAME values; arbitrary parameters use the user's name |
| Default fields | `FONTID=1`, `READONLYSTATE=None`, `PARAMTYPE=String`, `COLOR=BLACK` |
| UniqueID | Generated automatically (8-char hex from UUID v4) |
| OWNERINDEX | Set to 0 (component root) — resolved to absolute at save time |
| `%UTF8%` prefix | Applied automatically for non-ASCII parameter names at serialization |
| Tier 1/2 serialization | Per-field default-skipping rules applied at save time |

#### `AddImplementationList`

Appends a RECORD=44 `SchImplementationList` container.

```rust
pub struct ImplementationListOp {
    pub component_ref: Option<RefExpr>,
}
```

**Domain logic hidden:**

| Concern | Detail |
|---------|--------|
| OWNERINDEX | Points to component root (index 0 relative) |
| Record is a container | No user-visible fields; exists solely to own Implementation children |
| Depth in tree | Level 1 child of component |

#### `AddImplementation`

Appends a RECORD=45 `SchImplementation` under the ImplementationList.

```rust
pub struct ImplementationOp {
    pub component_ref: Option<RefExpr>,
    pub model_name: String,
    pub model_type: Option<String>,  // default: "PCBLIB"
    pub is_current: Option<bool>,    // default: true
}
```

**Domain logic hidden:**

| Concern | Detail |
|---------|--------|
| OWNERINDEX | Must point to the ImplementationList record's index |
| MODELTYPE default | "PCBLIB" for footprints, "SIM" for simulation, "SI" for signal integrity |
| ISCURRENT | First implementation defaults to true |
| DATAFILECOUNT | Auto-set based on model data entries |
| MODELDATAFILEENTITY0 / MODELDATAFILEKIND0 | Template defaults for PCBLIB type |
| DATALINKSLOCKED, INTEGRATEDMODEL, DATABASEMODEL | Boolean defaults (false) |
| UniqueID | Generated automatically |
| Parent lookup | Must find the ImplementationList in this component's records |

#### `AddImplementationMap`

Appends a RECORD=46 `SchImplementationMap` container under the Implementation.

```rust
pub struct ImplementationMapOp {
    pub component_ref: Option<RefExpr>,
}
```

**Domain logic hidden:**

| Concern | Detail |
|---------|--------|
| OWNERINDEX | Must point to the Implementation record's index |
| Parent lookup | Must find the most recent Implementation in this component's records |

#### `AddMapDefiner`

Appends a RECORD=47 `SchMapDefiner` — one pin-to-pad mapping entry.

```rust
pub struct MapDefinerOp {
    pub component_ref: Option<RefExpr>,
    pub pin_name: String,   // schematic pin designator
    pub pad_name: String,   // footprint pad designator
}
```

**Domain logic hidden:**

| Concern | Detail |
|---------|--------|
| OWNERINDEX | Must point to the ImplementationMap record's index |
| Field naming | Internal keys are `PINNAME` and `PADNAME` (legacy naming) |
| Parent lookup | Must find the most recent ImplementationMap in this component's records |

#### `AddParameterList`

Appends a RECORD=48 `SchParameterList` container under the Implementation.

```rust
pub struct ParameterListOp {
    pub component_ref: Option<RefExpr>,
}
```

**Domain logic hidden:**

| Concern | Detail |
|---------|--------|
| OWNERINDEX | Must point to the Implementation record's index |
| Parent lookup | Must find the most recent Implementation in this component's records |

#### Composed: Full Implementation Chain

The ops crate composes the 5 implementation ops from a single high-level spec:

```yaml
footprint:
  model_name: "0805"
  map:
    - { pin: "1", pad: "1" }
    - { pin: "2", pad: "2" }
```

Lowers to:

```
AddImplementationList
AddImplementation { model_name: "0805" }
AddImplementationMap
AddMapDefiner { pin: "1", pad: "1" }
AddMapDefiner { pin: "2", pad: "2" }
AddParameterList
```

The OWNERINDEX chain (ImplementationList→Implementation→Map→MapDefiner×N→ParameterList)
is resolved by each `ops_append_*` method finding the correct parent in the component's
existing record list.

---

### Phase 2: Graphics Primitives

All graphics ops append to a component's record list with OWNERINDEX=0 (component root).
The ops crate provides coordinates, colors, and style enums. The format crate handles
DXP fractional encoding, COLORREF BGR packing, and per-field serialization rules.

#### Common Fields

Every graphics op includes these optional fields (with defaults):

```rust
pub struct GraphicsCommon {
    pub component_ref: Option<RefExpr>,
    pub owner_part_id: Option<i32>,             // default: 0 (all parts)
    pub owner_part_display_mode: Option<i32>,    // default: 0 (all modes)
}
```

Multi-part symbols use `owner_part_id` (1-based, 0 = common to all parts) to assign
graphics to specific parts. `owner_part_display_mode` assigns to display modes
(0 = common to all modes).

#### `AddLine`

Appends a RECORD=13 `SchLine`.

```rust
pub struct LineOp {
    pub common: GraphicsCommon,
    pub start: CoordPoint,    // LOCATION.X/Y
    pub end: CoordPoint,      // CORNER.X/Y
    pub color: Option<Color>, // default: BLACK
    pub line_width: Option<i32>,  // 0=Smallest, 1=Small(default), 2=Medium, 3=Large
    pub line_style: Option<i32>,  // 0=Solid(default), 1=Dashed, 2=Dotted, 3=DashDotted
}
```

**Domain logic hidden:**

| Concern | Detail |
|---------|--------|
| DXP fractional coords | Each coordinate split into integer + `_FRAC` at save time |
| COLORREF encoding | RGB → `0x00BBGGRR` (BGR byte order) at save time |
| LineWidth enum | 0=eSmallest, 1=eSmall, 2=eMedium, 3=eLarge |
| LineStyle enum | 0=Solid, 1=Dashed, 2=Dotted, 3=DashDotted |
| Parameter ordering | Hardcoded per FileFormatV5.cs at save time |

#### `AddRectangle`

Appends a RECORD=14 `SchRectangle`.

```rust
pub struct RectangleOp {
    pub common: GraphicsCommon,
    pub location: CoordPoint,    // bottom-left (LOCATION.X/Y)
    pub corner: CoordPoint,      // top-right (CORNER.X/Y)
    pub color: Option<Color>,    // border color, default: BLACK
    pub area_color: Option<Color>, // fill color, default: WHITE
    pub is_solid: Option<bool>,  // default: true
    pub transparent: Option<bool>, // default: false
    pub line_width: Option<i32>, // default: 1 (Small)
}
```

**Domain logic hidden:**

| Concern | Detail |
|---------|--------|
| Two-corner encoding | LOCATION = bottom-left, CORNER = top-right, each with `_FRAC` |
| COLOR vs AREACOLOR | Border vs fill — both COLORREF |
| ISSOLID + TRANSPARENT | Interact: solid=true + transparent=true = translucent fill |

#### `AddArc`

Appends a RECORD=12 `SchArc`.

```rust
pub struct ArcOp {
    pub common: GraphicsCommon,
    pub center: CoordPoint,
    pub radius: Coord,
    pub start_angle: f64,       // degrees, default: 0.0
    pub end_angle: f64,         // degrees, default: 360.0 (full circle)
    pub color: Option<Color>,
    pub line_width: Option<i32>,
}
```

**Domain logic hidden:**

| Concern | Detail |
|---------|--------|
| RADIUS + RADIUS_FRAC | DXP fractional encoding |
| Angle format | Delphi-compatible float formatting at save time |
| Full circle | STARTANGLE=0.0, ENDANGLE=360.0 |

#### `AddEllipticalArc`

Appends a RECORD=11 `SchEllipticalArc`.

```rust
pub struct EllipticalArcOp {
    pub common: GraphicsCommon,
    pub center: CoordPoint,
    pub radius: Coord,              // primary radius
    pub secondary_radius: Coord,    // secondary radius
    pub start_angle: f64,
    pub end_angle: f64,
    pub color: Option<Color>,
    pub line_width: Option<i32>,
}
```

#### `AddEllipse`

Appends a RECORD=8 `SchEllipse`.

```rust
pub struct EllipseOp {
    pub common: GraphicsCommon,
    pub center: CoordPoint,
    pub radius: Coord,
    pub secondary_radius: Coord,
    pub color: Option<Color>,
    pub area_color: Option<Color>,
    pub is_solid: Option<bool>,
    pub line_width: Option<i32>,
}
```

#### `AddPolyline`

Appends a RECORD=6 `SchPolyline`.

```rust
pub struct PolylineOp {
    pub common: GraphicsCommon,
    pub points: Vec<CoordPoint>,  // 2+ points
    pub color: Option<Color>,
    pub line_width: Option<i32>,
    pub line_style: Option<i32>,
}
```

**Domain logic hidden:**

| Concern | Detail |
|---------|--------|
| 1-based indexed coords | `LOCATIONCOUNT=N`, `X1/Y1`..`XN/YN`, each with `_FRAC` |
| Minimum points | Must have at least 2 points |

#### `AddPolygon`

Appends a RECORD=7 `SchPolygon`.

```rust
pub struct PolygonOp {
    pub common: GraphicsCommon,
    pub points: Vec<CoordPoint>,  // 3+ points (closed shape)
    pub color: Option<Color>,
    pub area_color: Option<Color>,
    pub is_solid: Option<bool>,
    pub line_width: Option<i32>,
}
```

#### `AddBezier`

Appends a RECORD=5 `SchBezier` (4 control points).

```rust
pub struct BezierOp {
    pub common: GraphicsCommon,
    pub points: [CoordPoint; 4],  // exactly 4 control points
    pub color: Option<Color>,
    pub line_width: Option<i32>,
}
```

#### `AddPie`

Appends a RECORD=9 `SchPie`.

```rust
pub struct PieOp {
    pub common: GraphicsCommon,
    pub center: CoordPoint,
    pub radius: Coord,
    pub start_angle: f64,
    pub end_angle: f64,
    pub color: Option<Color>,
    pub area_color: Option<Color>,
    pub is_solid: Option<bool>,
    pub line_width: Option<i32>,
}
```

#### `AddRoundRectangle`

Appends a RECORD=10 `SchRoundRectangle`.

```rust
pub struct RoundRectangleOp {
    pub common: GraphicsCommon,
    pub location: CoordPoint,
    pub corner: CoordPoint,
    pub corner_x_radius: Coord,
    pub corner_y_radius: Coord,
    pub color: Option<Color>,
    pub area_color: Option<Color>,
    pub is_solid: Option<bool>,
    pub transparent: Option<bool>,
    pub line_width: Option<i32>,
}
```

#### `AddLabel`

Appends a RECORD=4 `SchLabel`.

```rust
pub struct LabelOp {
    pub common: GraphicsCommon,
    pub location: CoordPoint,
    pub text: String,
    pub color: Option<Color>,
    pub font_id: Option<i32>,         // default: 1
    pub orientation: Option<i32>,     // 0, 90, 180, 270
    pub justification: Option<i32>,   // 0-8 (BottomLeft..TopRight)
    pub is_hidden: Option<bool>,
    pub is_mirrored: Option<bool>,
}
```

**Domain logic hidden:**

| Concern | Detail |
|---------|--------|
| Orientation bitmask | bit 0=ROTATED (90deg), bit 1=FLIPPED — packed from 0/90/180/270 |
| Justification enum | 3x3 grid: 0=BottomLeft, 1=BottomCenter, ... 8=TopRight |

#### `AddTextFrame`

Appends a RECORD=28 `SchTextFrame`.

```rust
pub struct TextFrameOp {
    pub common: GraphicsCommon,
    pub location: CoordPoint,     // bottom-left
    pub corner: CoordPoint,       // top-right
    pub text: String,
    pub color: Option<Color>,
    pub area_color: Option<Color>,
    pub font_id: Option<i32>,
    pub alignment: Option<i32>,
    pub word_wrap: Option<bool>,
    pub show_border: Option<bool>,
    pub is_solid: Option<bool>,
    pub clip_to_rect: Option<bool>,
}
```

#### `AddImage`

Appends a RECORD=30 `SchImage` and its embedded binary data.

```rust
pub struct ImageOp {
    pub common: GraphicsCommon,
    pub location: CoordPoint,      // bottom-left
    pub corner: CoordPoint,        // top-right
    pub file_name: String,
    pub image_data: Vec<u8>,       // raw image bytes
    pub keep_aspect: Option<bool>,
}
```

**Domain logic hidden:**

| Concern | Detail |
|---------|--------|
| /Storage stream | Image data stored in global /Storage as zlib-compressed 0xD0 envelope |
| Cross-reference | SchImage.FILENAME must match the embedded object's Name in /Storage |
| EMBEDIMAGE flag | Set to true when data is provided inline |

---

### Phase 3: Mutation Ops

#### `EditComponent`

Modifies fields on a component's SchComponent record.

```rust
pub struct EditComponentOp {
    pub component_ref: RefExpr,
    pub description: Option<String>,
    pub part_count: Option<i32>,
    pub display_mode_count: Option<i32>,
    pub component_kind: Option<i32>,
    pub show_hidden_pins: Option<bool>,
}
```

**Domain logic hidden:**

| Concern | Detail |
|---------|--------|
| PARTCOUNT encoding | Stored as value+1 in file |
| ComponentKind triple | `ComponentKind`/`ComponentKindVersion2`/`ComponentKindVersion3` encoding |
| FileHeader sync | Must update LibRef/CompDescr/PartCount in header index if changed |

#### `EditRecord`

Modifies fields on child records matching a selector.

```rust
pub struct EditRecordOp {
    pub component_ref: Option<RefExpr>,
    pub selector: RecordSelector,
    pub patch: RecordPatch,  // field-level updates
}

pub enum RecordSelector {
    ByDesignator(String),    // pin or designator text
    ByRecordType(i32),       // RECORD number
    ByIndex(usize),          // position in component's records
    ByName(String),          // parameter NAME field
}
```

**Domain logic hidden:**

| Concern | Detail |
|---------|--------|
| Selector resolution | Match against component's child records |
| Field patching | Update only specified fields, preserve unmodified |
| Pin sidecar impact | Editing pin text fields may require sidecar regeneration at save time |
| OWNERINDEX stability | Editing does not change record positions |

#### `RemoveRecords`

Removes records matching a selector, cascading to children.

```rust
pub struct RemoveRecordsOp {
    pub component_ref: Option<RefExpr>,
    pub selector: RecordSelector,
}
```

**Domain logic hidden:**

| Concern | Detail |
|---------|--------|
| Cascade delete | Removing a parent removes all children (by OWNERINDEX chain) |
| Index renumbering | All OWNERINDEX values after removed records shift down |
| Pin count update | `all_pin_count` decremented for each removed pin |
| Weight update | Header weight recomputed after removal |
| Sidecar regeneration | Pin sidecar indices are sequential; removal requires regeneration at save time |

#### `RemoveComponent`

Removes an entire component from the library.

```rust
pub struct RemoveComponentOp {
    pub component_ref: RefExpr,
}
```

**Domain logic hidden:**

| Concern | Detail |
|---------|--------|
| In-memory removal | Remove from `components` vec and `header.components` index |
| Alias cleanup | Remove all associated aliases from `aliases` vec |
| SectionKeys cleanup | Handled at save time (rebuilt from current components) |
| CFB storage | Handled at save time (only existing components are written) |
| Weight recomputation | Recomputed after removal |

---

### Phase 4: Alias & Query Ops

#### `AddAlias`

Creates an alternative name for a component.

```rust
pub struct AddAliasOp {
    pub component_ref: RefExpr,
    pub alias_name: String,
}
```

**Domain logic hidden:**

| Concern | Detail |
|---------|--------|
| Dual tracking | Alias added to both `aliases` vec and `header.components[i].aliases` |
| ALIASLIST field | Updated on the SchComponent record |
| CFB key sanitization | Applied to alias name for Redirection storage path |
| SectionKeys | Entry added if alias name > 31 chars (handled at save time) |
| Redirection stream | `/<AliasKey>/Redirection` written at save time |
| Weight | Aliases count toward weight |

#### `RemoveAlias`

Removes an alias.

```rust
pub struct RemoveAliasOp {
    pub component_ref: RefExpr,
    pub alias_name: String,
}
```

**Domain logic hidden:** Reverse of AddAlias.

#### `QueryComponents` (read-only)

Lists components matching a selector pattern.

```rust
pub struct QueryComponentsOp {
    pub pattern: Option<String>,  // glob or substring match
}

pub struct ComponentInfo {
    pub index: usize,
    pub lib_reference: String,
    pub description: String,
    pub part_count: i32,
    pub pin_count: i32,
    pub aliases: Vec<String>,
    pub has_footprint: bool,
}
```

**Domain logic hidden:**

| Concern | Detail |
|---------|--------|
| Record traversal | Iterate internal SchLibComponent structs |
| Pin counting | Count SchRecord::Pin variants in records list |
| Footprint detection | Check for ImplementationList/Implementation in records |
| Alias resolution | Cross-reference with header component index |

#### `QueryPins` (read-only)

Returns pin information for a component.

```rust
pub struct QueryPinsOp {
    pub component_ref: RefExpr,
}

pub struct PinInfo {
    pub designator: String,
    pub name: String,
    pub electrical: String,        // human-readable enum name
    pub location: CoordPoint,
    pub length: Coord,
    pub orientation: i32,          // degrees: 0, 90, 180, 270
    pub is_hidden: bool,
    pub owner_part_id: i32,
}
```

**Domain logic hidden:**

| Concern | Detail |
|---------|--------|
| Sidecar merge | All 9 sidecar streams already merged at load time |
| Binary pin decoding | Already decoded from binary format at load time |
| PinConglomerate | Unpacked into separate fields at load time |
| Coordinate scaling | Internal units → human-readable conversion |

#### `QueryRecords` (read-only)

General record query.

```rust
pub struct QueryRecordsOp {
    pub component_ref: RefExpr,
    pub record_type: Option<i32>,
}

pub struct RecordInfo {
    pub index: usize,
    pub record_type: i32,
    pub owner_index: i32,
    pub summary: String,  // human-readable one-liner
}
```

---

## What `save()` Handles (not in ops)

These format-level concerns are handled entirely by the serialization pipeline:

| Concern | Where |
|---------|-------|
| OWNERINDEX: relative indices | `serialize_component_data()` adjusts to component-relative |
| Record ordering | `SchDataObjectComparator`: RECORD ≤ 225 stable, > 225 sort by type |
| Binary pin encoding | `serialize_binary_pin()`: 0x02 tag, packed struct, pascal strings |
| PinConglomerate packing | Orientation + visibility flags packed into bitmask |
| Pin sidecar streams | 9 streams conditionally written per pin write conditions |
| DXP fractional coords | Integer + `_FRAC` split at serialize time |
| COLORREF encoding | RGB → `0x00BBGGRR` at serialize time |
| Block headers | 4-byte LE: flags(8b) \| size(24b) |
| Windows-1252 encoding | Applied to all parameter text at serialize time |
| `%UTF8%` prefix | Applied to Unicode parameter keys at serialize time |
| NUL terminator | Appended to each parameter block |
| CFB key sanitization | `/\:*?"<>|!` → `_`, truncate to 31 chars |
| SectionKeys stream | Written only if any key was truncated |
| /Storage stream | Embedded images with zlib + 0xD0 envelope |
| /LibAdditional | Per-component Additional streams |
| Alias Redirection | `/<AliasKey>/Redirection` streams |
| FileHeader | Full header with component index, font table, display settings |
| Tier 1/2 serialization | Per-field default-skipping rules |
| Parameter ordering | Hardcoded per record type |
| `PARTCOUNT+1` encoding | Component's part_count stored as value+1 |
| Delphi float formatting | Angle and other float values |

## Implementation Notes

### Parent Lookup for Implementation Chain

The implementation chain ops need to find their parent record within the component.
Each `ops_append_*` method scans the component's existing `records` vec in reverse
to find the correct parent:

- `AddImplementation` → finds last `SchRecord::ImplementationList`
- `AddImplementationMap` → finds last `SchRecord::Implementation`
- `AddMapDefiner` → finds last `SchRecord::ImplementationMap`
- `AddParameterList` → finds last `SchRecord::Implementation`

The OWNERINDEX stored is the record's position within the component's child list
(not the global warehouse position). This matches the convention used by existing
ops (`ops_append_designator` sets `owner_index: 0`).

### Multi-Part Symbol Support

Graphics primitives support `owner_part_id` for multi-part symbols:

- `0` = common to all parts (drawn in every part view)
- `1..N` = specific part (1-based)

The component's `part_count` defines how many parts exist. When adding graphics
to a specific part, the ops crate sets `owner_part_id` on the record.

### Weight Invariant

After every mutation op, `ops_recompute_header_weight()` must be called.
Weight = sum over all components of (child record count + alias count).
The SchComponent root record is NOT counted. The end marker is NOT counted
(SchLib Data streams have no end marker; reading terminates at EOF).
