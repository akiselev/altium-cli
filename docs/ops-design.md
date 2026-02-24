# altium-format-ops High-Level Operations Design

## Architecture Overview

```
altium-format          Low-level crate. Exposes records, fields, document
(expose internals)     structure. Parsing, serialization, validation.
                       "Here are the raw records, have at it."
       │
       ▼
altium-format-ops      High-level operations. Composes records into
(domain operations)    meaningful actions. Owns specs, query lang,
                       record-chain orchestration.
                       "add_component(doc, spec) handles the 12-record chain."
       │
       ▼
altium-cli             CLI interface. Maps commands to operations.
                       "altium edit foo.SchDoc add-component --spec comp.yaml"
```

The key insight: **operations are the API, not objects**. Agents don't want
`handle.set_value("20K")` — they want
`altium edit design.SchDoc edit "component[designator=R1]" --set value=20K`.

## The Altium Document Universe

There are two domains with two roles each:

|  | **Schematic** | **PCB** |
|---|---|---|
| **Library (definition)** | SchLib — symbol definitions (pins, graphics) | PcbLib — footprint definitions (pads, tracks, 3D) |
| **Document (instance)** | SchDoc — placed instances + connectivity | PcbDoc — placed instances + routing |
| **Bundle** | IntLib — container bundling SchLib + PcbLib |

Libraries define *what things look like*. Documents place *instances of those things*
with connectivity/routing.

### SchLib vs SchDoc Component Differences

| Aspect | SchLib (Symbol) | SchDoc (Instance) |
|--------|----------------|-------------------|
| What it is | Symbol *definition* | Placed *instance* |
| Location | No placement (origin-relative) | Absolute sheet position |
| Designator | None (library symbol has name) | "R1", "U1" |
| Value | None | "10K", "LM358" |
| Footprint | None | "0805" (implementation chain) |
| Pins | Binary + 9 sidecar streams | Text format, inline |
| Graphics | Lines/rects/arcs define the symbol | Same, positioned by placement |
| Storage | Per-component CFB storage | Flat list, OWNERINDEX linking |

### PcbLib vs PcbDoc Differences

| Aspect | PcbLib (Footprint) | PcbDoc (Board) |
|--------|-------------------|----------------|
| What it is | Footprint *definition* | Full board layout |
| Components | Template footprints | Placed footprint instances |
| Nets | None | Real electrical connectivity |
| Structure | Per-footprint CFB storage | Section-per-primitive-type |
| Primitives | Pads, tracks, arcs, regions, bodies | Same + vias, fills, design rules |

## Layer 1: Exposing altium-format Internals

Currently everything is `pub(crate)`. We make records and document fields `pub`:

```rust
// Before (private)
pub(crate) struct SchComponent { pub lib_reference: String, ... }
pub(crate) enum SchRecord { Component(SchComponent), ... }
pub struct SchDoc { pub(crate) records: Vec<SchRecord>, ... }

// After (public)
pub struct SchComponent { pub lib_reference: String, ... }
pub enum SchRecord { Component(SchComponent), ... }
pub struct SchDoc { pub records: Vec<SchRecord>, ... }
```

Plus helper methods on document types for common low-level operations:

```rust
impl SchDoc {
    // Existing
    pub fn open(path) -> Result<Self>
    pub fn save(path) -> Result<()>
    pub fn validate_invariants() -> Result<()>

    // New helpers for ops crate
    pub fn append_record(&mut self, record: SchRecord) -> usize  // returns index
    pub fn record_count(&self) -> usize
    pub fn children_of(&self, index: usize) -> Vec<usize>
    pub fn update_weight(&mut self)  // recompute header.weight
    pub fn next_unique_id(&mut self) -> String
    pub fn next_index_in_sheet(&self) -> i32
}
```

Similarly for PcbDoc, SchLib, PcbLib — expose sections, footprints, components, primitives.

The low-level crate does NOT know about "components" as a high-level concept. It knows
about records, indices, and serialization. The ops crate provides the domain intelligence.

### What Cruft Each Layer Hides

| Domain | What ops exposes | What altium-format handles internally |
|--------|-----------------|---------------------------------------|
| **SchDoc** | `add_component(spec)` | OWNERINDEX flat indexing, 12-record chain (Component→Designator→Parameter→Pin×N→ImplList→Impl→ImplMap→MapDefiner×N→ParamList), PinConglomerate bitmask, DXP fractional coordinates, block stream headers, Windows-1252 encoding, Weight counter, IsNotAccesible typo, IndexInSheet, vault/GUID fields |
| **PcbDoc** | `add_track(spec)` | Section-per-type storages (Tracks6/Pads6/etc.), dual FileHeader+FileHeaderSix, net_index/component_index/polygon_index cross-references, WideStrings6 TLV encoding, UnionNames, binary primitive encoding, 20+ unknown fields |
| **SchLib** | `add_symbol(spec)` | Per-component CFB storages, SectionKeys name sanitization, binary pin format, 9 sidecar streams (PinFrac/PinDesc/PinWideText/etc.), font table internals |
| **PcbLib** | `add_footprint(spec)` | Per-footprint CFB storages, 13-byte PcbPrimitiveCommon headers, PcbPadCache (38 bytes), PcbPadStackData (596 bytes), 6 pad subrecords, UniqueID/WideStrings sidecars, V6→V7 layer mapping |
| **IntLib** | `.symbols()` / `.footprints()` | CFB container extraction, embedded SchLib/PcbLib byte streams, LibCrossRef.txt |

## Layer 2: Operations (altium-format-ops)

Operations are **functions**, not methods on objects. Each takes a document reference + a spec/selector.

### Schematic Operations (SchDoc)

```rust
/// Place a component with all child records (designator, pins,
/// parameters, implementation chain). Handles the 12-record chain.
pub fn add_component(doc: &mut SchDoc, spec: &ComponentSpec) -> Result<usize>;

/// Modify fields on components matching the selector.
/// Returns count of modified components.
pub fn edit_component(doc: &mut SchDoc, selector: &str, patch: &ComponentPatch) -> Result<usize>;

/// Remove components (and all children) matching the selector.
pub fn remove_records(doc: &mut SchDoc, selector: &str) -> Result<usize>;

/// Add a wire.
pub fn add_wire(doc: &mut SchDoc, spec: &WireSpec) -> Result<usize>;

/// Add a net label.
pub fn add_net_label(doc: &mut SchDoc, spec: &NetLabelSpec) -> Result<usize>;

/// Add a power port.
pub fn add_power_port(doc: &mut SchDoc, spec: &PowerPortSpec) -> Result<usize>;

/// Add a junction.
pub fn add_junction(doc: &mut SchDoc, spec: &JunctionSpec) -> Result<usize>;

/// Query records matching a selector. Returns structured results.
pub fn query(doc: &SchDoc, selector: &str) -> Result<Vec<QueryResult>>;

/// Dump component info in a structured format.
pub fn describe_component(doc: &SchDoc, selector: &str) -> Result<Vec<ComponentInfo>>;
```

### PCB Operations (PcbDoc)

```rust
pub fn add_track(doc: &mut PcbDoc, spec: &TrackSpec) -> Result<()>;
pub fn add_via(doc: &mut PcbDoc, spec: &ViaSpec) -> Result<()>;
pub fn edit_track(doc: &mut PcbDoc, selector: &str, patch: &TrackPatch) -> Result<usize>;
pub fn query(doc: &PcbDoc, selector: &str) -> Result<Vec<QueryResult>>;
```

### Library Operations

```rust
// SchLib
pub fn add_component(lib: &mut SchLib, spec: &SchLibAddComponentSpec) -> Result<()>;
pub fn edit_symbol(lib: &mut SchLib, selector: &str, patch: &SymbolPatch) -> Result<usize>;
pub fn query_symbols(lib: &SchLib, selector: &str) -> Result<Vec<SymbolInfo>>;

// PcbLib
pub fn add_footprint(lib: &mut PcbLib, spec: &FootprintSpec) -> Result<()>;
pub fn query_footprints(lib: &PcbLib, selector: &str) -> Result<Vec<FootprintInfo>>;
```

### SchLib `add_component` — Flat Op Architecture

SchLib operations use a **flat op list**, not a state machine or layered IR. Each op
is a standalone enum variant that targets a component by reference and appends a record.
No `BeginComponent`/`EndComponent` lifecycle — the component is created atomically by
`CreateComponentRoot`, then children are appended one at a time.

The serialization pipeline (`save()`) handles all format-level complexity:
OWNERINDEX assignment, record ordering, pin sidecar generation, SectionKeys, block
encoding, and parameter ordering. The ops just construct correct in-memory records.

#### Three-tier lowering pipeline

```
HighOp (YAML/JSON)           altium-format-ops    User-facing spec
  ↓ lower_high_ops()
ComposedOp                   altium-format-ops    Flat sequence, one op per record
  ↓ lower_composed_to_schlib_low()
SchLibLowOp                  altium-format        Crate-boundary types (no internal records)
  ↓ apply_schlib_low_ops()
ops_* methods on SchLib      altium-format        Mutate internal SchLibComponent structs
```

Each lowering pass is a pure 1:1 or 1:N mapping. No graph resolution, no symbolic handles.

#### Low-level op types (in `sch_ops_core.rs`)

```rust
pub enum SchLibLowOp {
    CreateComponentRoot(ComponentRootOp),
    CreateComponentDesignator(ComponentTextOp),
    CreateComponentComment(ComponentTextOp),
    AddPin(PinOp),
    // ... additional ops added as needed (see docs/schlib-ops.md)
}
```

Each variant maps to a `pub(crate)` method on `SchLib` (e.g., `ops_append_component_root`,
`ops_append_designator`, `ops_append_comment`, `ops_append_pin`). Internal record types
never cross the crate boundary.

#### Execution context

```rust
struct SchLibExecCtx {
    refs: HashMap<String, usize>,    // batch-placed component refs
    last_component: Option<usize>,   // implicit target for child ops
}
```

Component resolution: explicit `component_ref` → look up in `ctx.refs` or
`lib.ops_find_component_index_by_ref()`. No ref → use `ctx.last_component`.

#### Concrete op sequence for one SchLib component

For a spec like resistor `R` with `R?`, `10K`, two pins, and footprint `0805`,
the high-level `AddComponent` op lowers into this flat sequence:

```
CreateComponentRoot { lib_reference: "R" }
CreateComponentDesignator { text: "R?" }
CreateComponentComment { text: "10K" }
AddPin { designator: "1", electrical: "passive", ... }
AddPin { designator: "2", electrical: "passive", ... }
AddImplementationList { }
AddImplementation { model_name: "0805" }
AddImplementationMap { }
AddMapDefiner { pin: "1", pad: "1" }
AddMapDefiner { pin: "2", pad: "2" }
AddParameterList { }
```

All OWNERINDEX values, record ordering, pin sidecar streams, and serialization
details are handled by `save()` — not by the ops.

#### What `save()` handles (not the ops layer)

- OWNERINDEX: relative indices computed at serialize time
- Record ordering: RECORD ≤ 225 stable, > 225 sorted by type
- Binary pin encoding: 0x02 tag, packed struct, pascal strings
- PinConglomerate packing: orientation + visibility flags as bitmask
- Pin sidecar streams: 9 streams conditionally written per pin
- DXP fractional coordinates: integer + `_FRAC` split
- COLORREF encoding: RGB → `0x00BBGGRR`
- CFB key sanitization: `/\:*?"<>|!` → `_`, truncate to 31 chars
- SectionKeys stream: written only if any key was truncated
- Weight recomputation: done by `ops_recompute_header_weight()` after each mutation
- Block headers, Windows-1252 encoding, parameter ordering, tier 1/2 sparse serialization

See `docs/schlib-ops.md` for the complete list of low-level ops and their domain logic.

### SchLib AddComponent Field Mapping Table

This table defines how `AddComponentOp` spec fields map through lowering to internal
record fields. The ops crate emits flat `SchLibLowOp` values; `altium-format` fills
defaults and constructs records.

| Spec field | Low-level op | Internal record field(s) | Notes / defaults |
|-----------|-------------|--------------------------|------------------|
| `id` | `ComponentRootOp.id` | Batch ref key only | Never serialized; used for `ctx.refs` lookup |
| `lib_reference` | `ComponentRootOp.lib_reference` | `SchComponent.lib_reference` | Required |
| `designator` | `ComponentTextOp.text` | `SchDesignator.text` | `SchDesignator.name` forced to `"Designator"` |
| `value` | `ComponentTextOp.text` | `SchParameter.text` | `SchParameter.name` forced to `"Comment"` |
| `pins[].designator` | `PinOp.designator` | `SchPin.designator` | Required |
| `pins[].name` | `PinOp.name` | `SchPin.name` | Default: `""` |
| `pins[].electrical` | `PinOp.electrical` | `SchPin.electrical` | Parsed from human-readable name ("passive", "power", etc.) |
| `pins[].length_mils` | `PinOp.length_mils` | `SchPin.pin_length` | Converted to internal Coord units |
| `footprint.model_name` | `ImplementationOp.model_name` | `SchImplementation.model_name` | If absent, skip impl chain |
| `footprint.map[]` | `MapDefinerOp` | `SchMapDefiner.pin_name`, `SchMapDefiner.pad_name` | One op per pair |

#### Hidden legacy/internal fields (not in public spec)

These are set by `ops_*` method defaults and `save()` — never leak into the spec:

- `OWNERINDEX`/parent linkage mechanics (relative indices assigned at save time)
- `OWNER_PART_ID`, `OWNER_PART_DISPLAY_MODE`, `UNION_INDEX`, lock flags
- binary-pin packing/conglomerate bits (packed at save time)
- section key sanitization / CFB storage key generation (at save time)
- `Weight` and component index recomputation (`ops_recompute_header_weight()`)
- pin sidecar stream generation (9 streams, conditionally written at save time)
- `UniqueID` generation (8-char hex from UUID v4, set by ops_* methods)
- `PARTCOUNT+1` encoding, `ComponentKind` triple encoding (at save time)
- DXP fractional coordinate split, COLORREF BGR packing (at save time)

### Executor Implementation

The SchLib executor is `apply_schlib_low_ops()` in `sch_ops_core.rs`. It lives in
`altium-format` so internal record types remain private.

```rust
pub fn apply_schlib_low_ops(lib: &mut SchLib, ops: &[SchLibLowOp]) -> Result<()> {
    let mut ctx = SchLibExecCtx::new(lib);
    for op in ops {
        apply_schlib_low_op(lib, op, &mut ctx)?;
    }
    Ok(())
}
```

Each op variant dispatches to a `pub(crate)` method on `SchLib`:
- `CreateComponentRoot` → `ops_append_component_root()` — creates `SchLibComponent` + header index entry
- `CreateComponentDesignator` → `ops_append_designator()` — appends `SchDesignator` record
- `CreateComponentComment` → `ops_append_comment()` — appends `SchParameter` with NAME="Comment"
- `AddPin` → `ops_append_pin()` — appends `SchPin`, updates `all_pin_count` + weight

No state machine, no symbolic handles, no deferred resolution. Each method directly
constructs the internal record struct with sane defaults, appends to the component's
record list, and (for mutations) calls `ops_recompute_header_weight()`.

#### Execution boundaries

- `altium-format-ops` lowers spec into `Vec<SchLibLowOp>` via pure mapping functions.
- `altium-format` owns all record construction, default values, and legacy field handling.
- No internal record types (`SchRecord`, `SchComponent`, etc.) cross the crate boundary.

### What `add_component` Actually Does

The ops crate knows that a "component" is really N records. It lowers `AddComponent`
into a flat sequence of `SchLibLowOp` values, each appending one record:

```
CreateComponentRoot { lib_reference: "R" }           → SchComponent (block 0)
CreateComponentDesignator { text: "R?" }             → SchDesignator (NAME="Designator")
CreateComponentComment { text: "10K" }               → SchParameter (NAME="Comment")
AddPin { designator: "1", electrical: "passive" }    → SchPin
AddPin { designator: "2", electrical: "passive" }    → SchPin
AddImplementationList { }                            → SchImplementationList
AddImplementation { model_name: "0805" }             → SchImplementation
AddImplementationMap { }                             → SchImplementationMap
AddMapDefiner { pin: "1", pad: "1" }                 → SchMapDefiner
AddMapDefiner { pin: "2", pad: "2" }                 → SchMapDefiner
AddParameterList { }                                 → SchParameterList
```

The OWNERINDEX chain (which record owns which) is resolved at save time based on
record positions within the component. The user never sees OWNERINDEX values.

## Unified Refs and Results (Query + Ops)

To support agent feedback loops, **every operation returns a structured result** and
references are unified between query and mutation flows.

### `opid` and Result Table

Each op may define an optional `opid`:

```yaml
ops:
  - opid: create_comp
    op: add_component
    lib_reference: R
    designator: R1
    value: 10K

  - op: add_pin
    component_ref: $create_comp.ref
    designator: "1"
    electrical: passive
```

Execution builds a `result_table: HashMap<OpId, OpResult>`.
If `opid` is omitted, the evaluator assigns deterministic IDs (`op_0001`, `op_0002`, ...).

#### `opid` propagation across all lowering tiers

`opid` is not only a high-level concept. It is carried through every tier:

1. `HighOp` has `opid`.
2. Every `ComposedOp` emitted from that `HighOp` gets a derived `opid`
   (e.g. `create_comp#0`, `create_comp#1`, ...).
3. Every low-level op (`SchDocLowOp` / `SchLibLowOp`) also stores `opid`.
4. Executor returns `OpResult` keyed by low-level `opid`, while preserving parent/child lineage.

This guarantees that lowering itself can reference prior low-level results without stringly
guessing (important for recursive lowering and chain construction).

### OpResult contract

All ops return:

```rust
pub struct OpResult {
    pub kind: String,                          // e.g. "add_component"
    pub ref_: Option<EntityRef>,               // primary entity created/targeted
    pub refs: Vec<EntityRef>,                  // secondary entities (children, matches)
    pub fields: IndexMap<String, ResolvedValue>, // typed outputs, e.g. created_count
    pub warnings: Vec<String>,
}
```

This makes mutation ops and query ops symmetrical: both emit structured data that can feed
later expressions/references.

### Canonical reference model

Both query and ops use the same `EntityRef`:

```rust
pub struct EntityRef {
    pub domain: Domain,            // SchDoc, SchLib, ...
    pub entity_type: EntityType,   // component, pin, implementation, ...
    pub id: String,                // canonical internal id token
    pub display_path: String,      // human-readable path, e.g. "R1.pin[1]"
}
```

### Typed ref helper API (no stringly refs in lowering code)

Lowering and executor code should use typed builders/accessors instead of raw `"$id.field"` strings.

```rust
// Constructors
Ref::op("create_comp")              // -> result of opid=create_comp
Ref::last()                         // -> most recent op result in current scope
Ref::self_()                        // -> current op context
Ref::sheet()                        // -> sheet context

// Accessors
Ref::op("create_comp").member("ref")
Ref::op("q1").member("refs").index(0).member("display_path")
Ref::last().member("ref")
```

Recommended core types:

```rust
pub enum RefRoot {
    OpId(String),
    Last,
    Self_,
    Sheet,
}

pub enum RefStep {
    Member(String),
    Index(usize),
}

pub struct RefExpr {
    pub root: RefRoot,
    pub steps: Vec<RefStep>,
}

impl RefExpr {
    pub fn member(self, name: impl Into<String>) -> Self;
    pub fn index(self, idx: usize) -> Self;
}
```

Parser-facing syntax (`$create_comp.ref`) is converted once into this typed form. All internal
passes operate on typed `RefExpr`.

Reference rules:

1. Query output includes `EntityRef`.
2. Ops accept `EntityRef` or expressions resolving to it.
3. `$<opid>.<field>` resolves from the same result table used by both query and ops.
4. Cardinality is explicit (`expect one` vs `many`) and ambiguity is fail-fast.

Additionally:

5. Low-level op fields that accept refs use typed wrappers (e.g. `RefField<EntityRef>`),
   not raw strings.
6. Access to op results inside lowering uses `RefExpr` helper API (`Ref::op(...).member(...)`).

### Expression/ref integration

Expression evaluator resolution order becomes:

1. `$<opid>.<field>` from prior op results
2. batch-created aliases (`$last`, optional user `id`)
3. query against current mutated document/library state
4. `self.*` for current op-local context
5. `$sheet.*` metadata

### Clean crate split

- `altium-format-ops`: high-level ops, query language, expression parsing/evaluation,
  lowering, `opid` result-table orchestration.
- `altium-format`: low-level execution and record mutation over crate-private types;
  returns typed `OpResult` payloads for the orchestrator.

This preserves a single source of truth for domain behavior while still allowing agent-visible
feedback from every operation.

## Layer 3: Smart Specs

Specs are YAML/JSON input that support units, enum names, document references, and arithmetic.

### Three Layers of Smartness

#### 3a. Units

Any numeric value can have a unit suffix. No suffix = mils (schematic and PCB default).

| Suffix | Meaning | Conversion to internal units |
|--------|---------|------------------------------|
| (none) | mils | × 10,000 |
| `mil` | mils (explicit) | × 10,000 |
| `mm` | millimeters | × (10,000 / 0.0254) |
| `in` | inches | × 10,000,000 |
| `dxp` | raw DXP units | × 100,000 |
| `raw` | raw internal units | passthrough |

```yaml
width: 10           # 10 mils
width: 10mil        # same, explicit
width: 0.254mm      # same in mm
width: 0.01in       # same in inches
hole_size: 0.3mm    # common for PCB
```

#### 3b. Enum Names

String values in typed fields are resolved against the field's expected enum type.
Case-insensitive, underscore-insensitive (`gnd_power` = `GndPower` = `gndpower`).

| Field context | Input | Resolves to |
|---------------|-------|-------------|
| `electrical` | `passive` | `PinElectricalType::Passive` |
| `electrical` | `power` | `PinElectricalType::Power` |
| `electrical` | `open_collector` | `PinElectricalType::OpenCollector` |
| `style` (power port) | `bar` | `PowerObjectStyle::Bar` |
| `style` (power port) | `gnd_power` | `PowerObjectStyle::GndPower` |
| `layer` | `Top` | `V6Layer::TopLayer` |
| `layer` | `Bottom` | `V6Layer::BottomLayer` |
| `layer` | `Mid1` | `V6Layer::MidLayer1` |
| `shape` (pad) | `round` | `PadShape::Round` |
| `shape` (pad) | `rectangular` | `PadShape::Rectangular` |
| `orientation` | `0` / `90` / `180` / `270` | `RotationBy90::Rotate{N}` |
| `line_style` | `dashed` | `LineStyle::Dashed` |
| `width` (pen) | `small` / `medium` / `large` | `PenWidth::Small` etc. |
| `color` | `red` / `#FF0000` | `Color` value |

Resolution is context-dependent: the field's expected type determines which enum to search.

#### 3c. Expressions (the `=` prefix)

Any string value starting with `=` is an expression evaluated against the document context.

**Grammar:**

```
expr           = additive
additive       = multiplicative (('+' | '-') multiplicative)*
multiplicative = unary (('*' | '/') unary)*
unary          = '-' unary | atom
atom           = number_with_unit
               | reference
               | '(' expr ')'

number_unit    = NUMBER UNIT?
reference      = path ('.' path)*
path           = IDENT ('[' key ']')?
key            = STRING | NUMBER
```

**Reference resolution chain:**

When evaluating `U1.pin[14].location.x`:

```
1. "U1"           → query document for component with designator "U1"
2. ".pin[14]"     → find child pin with designator "14"
3. ".location"    → get the pin's location (CoordPoint)
4. ".x"           → extract x coordinate (Coord)
```

**Resolution order:**

1. **Previously placed records in this batch** (so R2 can reference R1 placed earlier)
2. **Existing document records** (so new wires can reference existing components)
3. **`self`** refers to the current operation's record (for edit operations)
4. **`$sheet`** refers to sheet-level properties (size, grid, etc.)

**Navigable references:**

```
// Component references
U1                          → component record
U1.location                 → CoordPoint
U1.location.x               → Coord
U1.location.y               → Coord
U1.orientation              → degrees (0, 90, 180, 270)
U1.designator               → String
U1.value                    → String
U1.lib_reference            → String

// Pin references (by designator)
U1.pin[1]                   → pin record
U1.pin[1].location          → CoordPoint (absolute position)
U1.pin[1].location.x        → Coord
U1.pin[1].name              → String
U1.pin[1].electrical        → PinElectricalType

// Pin by name (alternative syntax)
U1.pin[VCC]                 → pin with name "VCC"

// PCB pad references
U1.pad[1]                   → pad record
U1.pad[A1].location         → CoordPoint

// Self reference (in edit ops)
self.location.x             → current record's x
self.value                  → current record's value

// Sheet properties
$sheet.width                → sheet width
$sheet.height               → sheet height
```

**Arithmetic on coordinates:**

```
=U1.location.x + 400        → Coord + 400mil = Coord
=U1.location.x + 2.54mm     → Coord + Coord(from mm) = Coord
=U1.location                 → CoordPoint (when field expects a point)
```

### Syntax Summary

| Feature | Syntax | Example |
|---------|--------|---------|
| Plain number | `N` | `1000` (mils) |
| With unit | `Nunit` | `2.54mm`, `100mil`, `0.1in` |
| Enum name | `name` | `passive`, `Top`, `bar`, `round` |
| Color | `#RRGGBB` | `#FF0000` |
| Expression | `=expr` | `=U1.location.x + 400` |
| Op result ref | `$<opid>.<field>` | `$create_comp.ref`, `$q1.refs[0]` |
| Point ref | `=path.location` | `=U1.pin[1].location` |
| Self ref | `=self.field` | `=self.location.x + 100` |
| Sheet ref | `=$sheet.field` | `=$sheet.width` |
| Batch ref | `=DESIG.field` | `=R1.location.y` (placed earlier) |
| Arithmetic | `+ - * /` | `=U1.pin[1].x + 2.54mm` |

## Complete Spec Examples

### Example 1: Place a resistor next to an existing IC

```yaml
- op: add_component
  designator: R1
  lib_reference: R
  value: 10K
  footprint: "0805"
  location: [=U1.location.x + 400, =U1.location.y]
  orientation: 0
  pins:
    - designator: "1"
      electrical: passive
      offset: [-50, 0]
      length: 25
    - designator: "2"
      electrical: passive
      offset: [50, 0]
      length: 25
```

### Example 2: Wire from one pin to another

```yaml
- op: add_wire
  points:
    - =U1.pin[14].location
    - =R1.pin[1].location
```

### Example 3: PCB track with mm units

```yaml
- op: add_track
  start: [=U1.pad[1].location.x, =U1.pad[1].location.y]
  end: [=U1.pad[1].location.x + 2.54mm, =U1.pad[1].location.y]
  width: 0.254mm
  layer: Top
  net: VCC
```

### Example 4: Self-referencing batch (two resistors + wire)

```yaml
- op: add_component
  designator: R1
  lib_reference: R
  value: 10K
  location: [1000, 800]
  pins:
    - { designator: "1", electrical: passive, offset: [-50, 0], length: 25 }
    - { designator: "2", electrical: passive, offset: [50, 0], length: 25 }

- op: add_component
  designator: R2
  lib_reference: R
  value: 10K
  location: [=R1.location.x + 300, =R1.location.y]
  pins:
    - { designator: "1", electrical: passive, offset: [-50, 0], length: 25 }
    - { designator: "2", electrical: passive, offset: [50, 0], length: 25 }

- op: add_wire
  points:
    - =R1.pin[2].location
    - =R2.pin[1].location
```

### Example 5: Edit existing component

```yaml
- op: edit
  select: "component[designator=R1]"
  set:
    value: 20K
    location.x: =self.location.x + 100
```

### Example 6: Power port and net label

```yaml
- op: add_power_port
  name: VCC
  style: bar
  location: [=U1.pin[14].location.x, =U1.pin[14].location.y + 100]
  orientation: 90

- op: add_net_label
  name: DATA_BUS
  location: [500, 1200]
  orientation: 0
```

## CLI Mapping

```bash
# Spec-driven apply (canonical)
altium ops apply library.SchLib --spec-file add-r1.yaml

# Feedback-loop modes
altium ops apply library.SchLib --spec-file add-r1.yaml --dry-run
altium ops apply library.SchLib --spec-file add-r1.yaml --report-json

# Schema introspection for agents
altium schema add_component          # what fields, types, enums
altium schema add_component --json   # JSON Schema output
altium schema --list                 # all available operations
```

Notes:

- `--report-json` prints the full op result table (`opid -> OpResult`).
- `--dry-run` performs lowering + ref/query resolution without saving.
- No inline operation-field arguments (`key=value`) for `ops apply`.

## Implementation Architecture

### Evaluation Pipeline

```
YAML input
    │
    ▼
┌─────────────────────┐
│  YAML parser (serde) │  Deserialize into SpecValue tree
└─────────┬───────────┘  (strings, numbers, arrays, maps)
          │
          ▼
┌─────────────────────┐
│  Value classifier    │  For each value:
│                      │    "passive"  → EnumName("passive")
│                      │    "100"      → Number(100, None)
│                      │    "2.54mm"   → Number(2.54, Mm)
│                      │    "=U1..."   → parse into Expr AST
│                      │    "#FF0000"  → Color
└─────────┬───────────┘
          │
          ▼
┌─────────────────────┐
│  Topological sort    │  Order operations so references resolve
│                      │  (R2 depends on R1 → R1 goes first)
└─────────┬───────────┘
          │
          ▼
┌─────────────────────┐
│  Evaluator           │  For each operation, in order:
│                      │    1. Resolve `$opid.*` refs from prior OpResults
│                      │    2. Classify values as eager vs deferred
│                      │    3. Convert eager units/enums immediately
│                      │    4. Lower to low-level ops and execute in altium-format
│                      │    5. Capture OpResult and store in result table
│                      │    6. Expose result fields to subsequent expressions
└─────────┬───────────┘
          │
          ▼
    Modified SchDoc / PcbDoc
```

### Core Types

```rust
/// A parsed but not-yet-evaluated spec value.
pub enum SpecValue {
    Null,
    Bool(bool),
    String(String),
    Number(f64, Option<LengthUnit>),
    Color(u8, u8, u8),
    Expr(Expr),
    Array(Vec<SpecValue>),
    Map(IndexMap<String, SpecValue>),
}

pub enum LengthUnit { Mil, Mm, Inch }

/// Parsed expression AST.
pub enum Expr {
    Literal(f64, Option<LengthUnit>),
    Ref(Vec<PathSegment>),
    BinOp(Box<Expr>, Op, Box<Expr>),
    Neg(Box<Expr>),
}

pub enum PathSegment {
    Field(String),
    Index(String),      // pin[1] or pin[VCC] — always string key
}

pub enum Op { Add, Sub, Mul, Div }
```

### Evaluation Context

```rust
pub struct EvalContext<'a> {
    /// The document being modified (for resolving existing records).
    doc: &'a SchDoc,

    /// Results of previously executed operations, keyed by opid.
    results: IndexMap<String, OpResult>,

    /// The current operation's partially-evaluated fields (for self refs).
    current: Option<&'a IndexMap<String, ResolvedValue>>,
}

/// A fully resolved value, ready for record construction.
pub enum ResolvedValue {
    Coord(Coord),
    CoordPoint(CoordPoint),
    String(String),
    I32(i32),
    F64(f64),
    Bool(bool),
    Color(Color),
}
```

### Enum Resolution Registry

```rust
pub struct EnumRegistry {
    tables: HashMap<&'static str, Vec<(&'static str, i32)>>,
}

impl EnumRegistry {
    pub fn new() -> Self {
        let mut reg = Self { tables: HashMap::new() };

        reg.register("electrical", &[
            ("input", 0), ("io", 1), ("inputoutput", 1),
            ("output", 2), ("opencollector", 3), ("open_collector", 3),
            ("passive", 4), ("hiz", 5), ("highz", 5),
            ("openemitter", 6), ("open_emitter", 6), ("power", 7),
        ]);

        reg.register("layer", &[
            ("top", 1), ("toplayer", 1),
            ("bottom", 32), ("bottomlayer", 32), ("bot", 32),
            ("mid1", 2), ("midlayer1", 2),
            // ...
        ]);

        reg.register("style:power_port", &[
            ("circle", 0), ("arrow", 1), ("bar", 2), ("wave", 3),
            ("gndpower", 4), ("gnd_power", 4), ("gndsignal", 5),
            ("gndearth", 6), ("gnd_earth", 6),
        ]);

        reg.register("shape", &[
            ("round", 1), ("rectangular", 2), ("rect", 2),
            ("octagonal", 3), ("roundrect", 7), ("round_rect", 7),
        ]);

        reg
    }

    pub fn resolve(&self, field: &str, value: &str) -> Option<i32> {
        let table = self.tables.get(field)?;
        let normalized = value.to_ascii_lowercase().replace('_', "");
        table.iter()
            .find(|(name, _)| *name == normalized)
            .map(|(_, v)| *v)
    }
}
```

### Schema Introspection Output

The `altium schema` command outputs structured information for LLM agents:

```json
{
  "op": "add_component",
  "description": "Place a component instance on a schematic sheet",
  "fields": {
    "designator": { "type": "string", "required": true, "example": "R1" },
    "lib_reference": { "type": "string", "required": true, "example": "R" },
    "value": { "type": "string", "default": "" },
    "location": {
      "type": "coord_point", "required": true, "example": "[1000, 800]",
      "note": "Supports units: 100mil, 2.54mm, 0.1in. Default: mils. Supports expressions: =U1.location.x + 400"
    },
    "orientation": { "type": "enum", "values": ["0", "90", "180", "270"], "default": "0" },
    "footprint": { "type": "string", "example": "0805" },
    "pins": {
      "type": "array",
      "items": {
        "designator": { "type": "string", "required": true },
        "electrical": { "type": "enum", "values": ["input", "io", "output", "open_collector", "passive", "hiz", "open_emitter", "power"] },
        "offset": { "type": "coord_point", "example": "[-50, 0]" },
        "length": { "type": "coord", "default": "25" }
      }
    }
  },
  "expression_syntax": {
    "references": "U1.location.x, U1.pin[1].location, self.value, $sheet.width",
    "arithmetic": "+, -, *, /",
    "units": "100mil, 2.54mm, 0.1in"
  }
}
```

## Scope Boundaries

### What We Build

- Units on numeric values (mil, mm, in)
- Enum name resolution (context-dependent, case-insensitive)
- Expression evaluation with document/batch/self references
- Simple arithmetic on coordinates (+ - * /)
- Topological ordering of batch operations
- Schema introspection for LLM agent discovery

### What We Don't Build

- **No control flow** — no if/else, no loops. Generate N operations for N placements.
- **No functions** — no sin(), sqrt(), min(). Complex geometry pre-computed by the agent.
- **No string interpolation** — no `${{ }}` inside strings. Values are coordinates and enums.
- **No variables/bindings** — no `let x = ...`. Designator references serve this purpose.
- **No nested specs** — each operation is independent but can reference previous ops.

The expression language is deliberately minimal: **references + arithmetic + units**.
That covers 95% of "place things relative to other things." Anything more complex,
the agent computes before generating the spec.

## File Layout

### Current (implemented)

```
altium-format-ops/src/
  ops/
    mod.rs               apply_schdoc(), apply_schlib(), spec parsing
    model.rs             HighOp, ComposedOp, AddComponentOp, AddPinOp, ApplyReport
    lower/
      mod.rs
      high_to_composed.rs      HighOp → ComposedOp (1:N expansion)
      composed_to_schlib_low.rs  ComposedOp → SchLibLowOp (1:1 mapping)
      composed_to_schdoc_low.rs  ComposedOp → SchDocLowOp (1:1 mapping)
  schlib_ops.rs          SchLibOps trait (validate, save_as, version)
  schdoc_ops.rs          SchDocOps trait
  lib.rs                 Re-exports, AltiumOperationError

altium-format/src/
  sch_ops_core.rs        SchLibLowOp enum, apply_schlib_low_ops(), exec context
  schlib.rs              SchLib struct + ops_* methods (pub(crate))
```

### Planned additions

```
altium-format-ops/src/
  spec/                  (future: smart spec evaluation)
    mod.rs               SpecValue, deserialization, value classification
    expr.rs              Expression parser (grammar → AST)
    eval.rs              Expression evaluator, EvalContext
    enums.rs             EnumRegistry, enum name resolution
    units.rs             Unit parsing and Coord conversion
    schema.rs            JSON Schema generation for `altium schema`
  query/                 (future: record query language)
    mod.rs               AQL parser and evaluator
    selector.rs          Pattern/attribute/pseudo-class selectors
    filter.rs            Record filtering against parsed selectors
```

See `docs/schlib-ops.md` for the complete low-level ops inventory and implementation plan.
