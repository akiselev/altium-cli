# SchDoc High-Level API — Implementation Plan

## Context

The SchDoc parser (open/save/roundtrip/validate/render) is complete. The missing piece is
a **high-level API** analogous to what SchLib and PcbLib already have — public types that
encapsulate the raw `SchRecord` list and expose query/mutation methods through natural keys.

This plan covers: high-level API types, read path, write path, query methods, mutation
methods, dump command, and the prep work for spec language integration.

### User decisions

- **Scope**: Full read + write API, plus dump command. Spec language deferred to follow-up.
- **Spec syntax direction**: `component $lib.REF { ... }` (reference-based, no inline defs).
  Placement deferred to solver (see `docs/notes/solverang/`).
- **Type hierarchy**: Single ordered `Vec<SheetObject>` enum, NOT separate vecs per entity
  type. Preserves ordering semantics and reflects the true tree structure.

---

## 1. Design Rationale

### 1.1 Why a single ordered enum?

The internal SchDoc format is a flat `Vec<SchRecord>` with OWNERINDEX integers encoding a
tree. The save pipeline writes records in **depth-first order** with children sorted by
insertion order (RECORD <= 225). This ordering:

1. **Determines OWNERINDEX values** (position-dependent)
2. **Is deterministic and canonical** (Altium always writes this exact order)
3. **Matters for version control** (reordering records = noisy diffs)
4. **Reflects logical grouping** (components with their children, sheet symbols with entries)

Splitting into separate `Vec<Wire>`, `Vec<NetLabel>`, etc. **destroys ordering** — you
can't reconstruct the interleaved order of wires, labels, and components on the sheet.
A single `Vec<SheetObject>` preserves it naturally.

### 1.2 The tree structure

```
SchDoc
└── Sheet (singleton)
    ├── Template (singleton, always first child)
    │   └── children: Vec<SheetObject>  (template-owned graphics/images)
    └── objects: Vec<SheetObject>       (ordered list of all sheet content)
        ├── Component "U1"
        │   └── children: Vec<ComponentChild>
        │       ├── Pin "1"
        │       ├── Pin "2"
        │       ├── Designator (implicit, extracted to field)
        │       ├── Parameter "Value"
        │       ├── Parameter "Comment"
        │       ├── Graphic::Rectangle { ... }
        │       └── FootprintMap "DIP-8"        (collapsed from Impl chain)
        ├── Wire { vertices, ... }
        ├── NetLabel "VCC3P3"
        ├── Component "R1"
        │   └── children: Vec<ComponentChild>
        │       └── ...
        ├── Junction { location }
        ├── PowerObject "GND"
        ├── SheetSymbol "Power"
        │   └── children: Vec<SheetSymbolChild>
        │       ├── SheetEntry "VCC"
        │       ├── SheetEntry "GND"
        │       └── Parameter "..."
        ├── ParameterSet "..."
        │   └── parameters: Vec<Parameter>
        ├── Note { text, author, ... }
        ├── NoConnect { location }
        ├── Graphic::Label { text }     (sheet-level annotation)
        └── Parameter "CurrentTime"     (sheet-level parameter)
```

### 1.3 Reusing SchLib child types

The types `Pin`, `Parameter`, `Graphic`, `FootprintMap` from the SchLib API represent the
same domain concepts. A pin is a pin whether in a library or a document. Only the
serialization format differs (text vs binary), which is handled below the API layer.

We reuse these types directly rather than duplicating them.

---

## 2. Public Types (`api/schdoc_types.rs`)

### 2.1 Top-level document

```rust
/// A schematic document with sheet properties and an ordered list of objects.
pub struct SchDocSheet {
    // ── Sheet properties ─────────────────────────────────────
    pub fonts: Vec<Font>,
    pub snap_grid_size: Coord,
    pub visible_grid_size: Coord,
    pub hot_spot_grid_size: Coord,
    pub snap_grid_on: bool,
    pub visible_grid_on: bool,
    pub hot_spot_grid_on: bool,
    pub sheet_style: SheetStyle,
    pub use_custom_sheet: bool,
    pub custom_width: Coord,
    pub custom_height: Coord,
    pub area_color: Color,
    pub border_on: bool,
    pub title_block_on: bool,
    pub show_template_graphics: bool,
    pub template_file_name: String,
    pub display_unit: DisplayUnit,
    pub workspace_orientation: i32,
    pub show_hidden_pins: bool,

    // ── Template ─────────────────────────────────────────────
    pub template: Template,

    // ── Ordered content ──────────────────────────────────────
    /// All sheet-level objects in document order. This ordering is preserved
    /// across save/load cycles and determines the serialized record positions.
    pub objects: Vec<SheetObject>,
}

pub struct Font {
    pub name: String,
    pub size: i32,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikeout: bool,
    pub rotation: i32,
}

pub struct Template {
    pub file_name: String,
    /// Template-owned graphics (labels, images from the .SchDot template).
    pub children: Vec<Graphic>,
}
```

### 2.2 SheetObject — the ordered top-level enum

Every entity directly owned by the Sheet is a variant of this enum. The `Vec<SheetObject>`
preserves document ordering.

```rust
/// A top-level object on the schematic sheet.
///
/// Variants are ordered to match the SchDoc ownership tree. Each container
/// variant bundles its children inline rather than via separate collections.
pub enum SheetObject {
    // ── Placed components ────────────────────────────────────
    Component(SchDocComponent),

    // ── Connectivity ─────────────────────────────────────────
    Wire(Wire),
    Bus(Bus),
    NetLabel(NetLabel),
    PowerObject(PowerObject),
    Port(Port),
    Junction(Junction),
    NoConnect(NoConnect),
    BusEntry(BusEntry),

    // ── Hierarchical ─────────────────────────────────────────
    SheetSymbol(SheetSymbol),

    // ── Annotations ──────────────────────────────────────────
    ParameterSet(ParameterSet),
    Note(Note),
    Probe(Probe),
    CompileMask(CompileMask),
    Blanket(Blanket),

    // ── Graphics (sheet-level, not owned by a component) ─────
    Graphic(Graphic),

    // ── Sheet-level parameters (CurrentTime, etc.) ───────────
    Parameter(Parameter),

    // ── Harness (future — currently errors on load) ──────────
    HarnessConnector(HarnessConnector),
    SignalHarness(SignalHarness),
}
```

### 2.3 SchDocComponent — placed component with children

```rust
/// A placed component instance on a schematic sheet.
///
/// Identity: `designator` (e.g. "R1", "U3"). Unique within the document.
///
/// Unlike SchLib's `Component` (identified by `lib_reference`), a SchDocComponent
/// represents a *placed instance* with position, orientation, and library back-references.
/// Its children are bundled in an ordered `Vec<ComponentChild>`.
pub struct SchDocComponent {
    // ── Identity ─────────────────────────────────────────────
    pub designator: String,
    pub unique_id: String,

    // ── Library reference ────────────────────────────────────
    pub lib_reference: String,
    pub source_library_name: String,
    pub design_item_id: String,
    pub library_path: String,

    // ── Placement ────────────────────────────────────────────
    pub location: CoordPoint,
    pub orientation: RotationBy90,
    pub is_mirrored: bool,

    // ── Properties ───────────────────────────────────────────
    pub description: Option<String>,
    pub component_kind: ComponentKind,
    pub part_count: i32,
    pub current_part_id: i32,
    pub display_mode_count: i32,
    pub show_hidden_pins: bool,

    // ── Children (ordered) ───────────────────────────────────
    /// All component children in document order: pins, parameters,
    /// graphics, and footprint maps interleaved as they appear in the file.
    pub children: Vec<ComponentChild>,
}
```

### 2.4 ComponentChild — ordered children of a component

```rust
/// A child object of a placed component.
///
/// This enum preserves the ordering of children within a component, which
/// mirrors the depth-first record order in the SchDoc file. The Designator
/// record (RECORD=34) is NOT included here — it is extracted to
/// `SchDocComponent.designator`.
pub enum ComponentChild {
    Pin(Pin),
    Parameter(Parameter),
    Graphic(Graphic),
    FootprintMap(FootprintMap),
}
```

`Pin`, `Parameter`, `Graphic`, and `FootprintMap` are **reused from the SchLib API types**
(`crate::api::schlib_types`).

The internal implementation chain (ImplementationList → Implementation → ImplementationMap
→ MapDefiner) is collapsed into `FootprintMap` exactly as in the SchLib API. The
`SchRecord::Designator` child is extracted into `SchDocComponent.designator` and does not
appear in the `children` vec.

### 2.5 Connectivity types

```rust
pub struct Wire {
    pub unique_id: String,
    pub vertices: Vec<CoordPoint>,
    pub color: Color,
    pub line_width: PenWidth,
    pub line_style: LineStyle,
}

pub struct Bus {
    pub unique_id: String,
    pub vertices: Vec<CoordPoint>,
    pub color: Color,
    pub line_width: PenWidth,
}

pub struct NetLabel {
    pub unique_id: String,
    pub text: String,
    pub location: CoordPoint,
    pub orientation: RotationBy90,
    pub justification: TextJustification,
    pub font_id: i32,
    pub color: Color,
    pub is_mirrored: bool,
    pub is_hidden: bool,
}

pub struct PowerObject {
    pub unique_id: String,
    pub text: String,
    pub location: CoordPoint,
    pub orientation: RotationBy90,
    pub style: PowerObjectStyle,
    pub show_net_name: bool,
    pub font_id: i32,
    pub color: Color,
    pub is_cross_sheet_connector: bool,
}

pub struct Port {
    pub unique_id: String,
    pub name: String,
    pub location: CoordPoint,
    pub io_type: PortIoType,
    pub style: PortArrowStyle,
    pub width: Coord,
    pub height: Coord,
    pub color: Color,
    pub area_color: Color,
    pub text_color: Color,
    pub font_id: i32,
    pub alignment: HorizontalAlign,
    pub harness_type: String,
    pub border_width: PenWidth,
    pub auto_size: bool,
    pub port_name_is_hidden: bool,
}

pub struct Junction {
    pub unique_id: String,
    pub location: CoordPoint,
    pub color: Color,
}

pub struct NoConnect {
    pub unique_id: String,
    pub location: CoordPoint,
    pub color: Color,
    pub orientation: RotationBy90,
    pub symbol: String,
    pub is_active: bool,
    pub suppress_all: bool,
}

pub struct BusEntry {
    pub unique_id: String,
    pub location: CoordPoint,
    pub corner: CoordPoint,
    pub color: Color,
    pub line_width: PenWidth,
}
```

### 2.6 Hierarchical sheet types

```rust
/// Hierarchical sheet symbol with ordered children.
///
/// Children include SheetEntry ports, Parameters, and the field objects
/// SheetName and SheetFileName (extracted to `sheet_name` and `file_name`).
pub struct SheetSymbol {
    pub unique_id: String,
    pub location: CoordPoint,
    pub x_size: Coord,
    pub y_size: Coord,
    pub color: Color,
    pub area_color: Color,
    pub line_width: PenWidth,
    pub is_solid: bool,
    pub symbol_type: SheetSymbolType,

    // Field objects (extracted from children, always present):
    pub sheet_name: String,
    pub file_name: String,

    // Children (ordered):
    pub children: Vec<SheetSymbolChild>,
}

pub enum SheetSymbolChild {
    Entry(SheetEntry),
    Parameter(Parameter),
}

pub struct SheetEntry {
    pub unique_id: String,
    pub name: String,
    pub io_type: PortIoType,
    pub side: LeftRightSide,
    pub distance_from_top: Coord,
    pub style: PortArrowStyle,
    pub color: Color,
    pub area_color: Color,
    pub text_color: Color,
    pub text_font_id: i32,
}
```

### 2.7 Annotation types

```rust
/// Parameter set attached to a net, with ordered child parameters.
pub struct ParameterSet {
    pub unique_id: String,
    pub location: CoordPoint,
    pub color: Color,
    pub orientation: RotationBy90,
    pub name: String,
    pub style: i32,
    pub parameters: Vec<Parameter>,
}

pub struct Note {
    pub unique_id: String,
    pub location: CoordPoint,
    pub corner: CoordPoint,
    pub text: String,
    pub author: String,
    pub font_id: i32,
    pub color: Color,
    pub area_color: Color,
    pub text_color: Color,
    pub is_solid: bool,
    pub show_border: bool,
    pub alignment: HorizontalAlign,
    pub word_wrap: bool,
    pub clip_to_rect: bool,
    pub text_margin: Coord,
    pub collapsed: bool,
}

pub struct Probe {
    pub unique_id: String,
    pub location: CoordPoint,
    pub color: Color,
    pub orientation: RotationBy90,
    pub name: String,
}

pub struct CompileMask {
    pub unique_id: String,
    pub location: CoordPoint,
    pub corner: CoordPoint,
    pub color: Color,
    pub area_color: Color,
    pub line_width: PenWidth,
    pub collapsed: bool,
}

pub struct Blanket {
    pub unique_id: String,
    pub location: CoordPoint,
    pub corner: CoordPoint,
    pub color: Color,
    pub area_color: Color,
    pub line_style: LineStyle,
    pub line_width: PenWidth,
    pub vertices: Vec<CoordPoint>,
    pub collapsed: bool,
}
```

### 2.8 Harness types (stub — currently errors on load)

```rust
pub struct HarnessConnector {
    pub unique_id: String,
    pub location: CoordPoint,
    pub x_size: Coord,
    pub y_size: Coord,
    pub color: Color,
    pub area_color: Color,
    pub line_width: PenWidth,
    pub children: Vec<HarnessChild>,
}

pub enum HarnessChild {
    Entry(SheetEntry),      // reuses SheetEntry
    ConnectorType(String),  // from SchSheetName.text
    Parameter(Parameter),
}

pub struct SignalHarness {
    pub unique_id: String,
    pub vertices: Vec<CoordPoint>,
    pub color: Color,
    pub line_width: PenWidth,
}
```

---

## 3. Natural Key Table

| Entity | Natural Key | Unique Within |
|--------|------------|---------------|
| SchDocComponent | `designator` | SchDocSheet |
| Wire | `unique_id` | SchDocSheet |
| Bus | `unique_id` | SchDocSheet |
| NetLabel | `unique_id` | SchDocSheet |
| PowerObject | `unique_id` | SchDocSheet |
| Port | `unique_id` | SchDocSheet |
| Junction | `unique_id` | SchDocSheet |
| NoConnect | `unique_id` | SchDocSheet |
| BusEntry | `unique_id` | SchDocSheet |
| SheetSymbol | `unique_id` | SchDocSheet |
| SheetEntry | `unique_id` | SheetSymbol |
| ParameterSet | `unique_id` | SchDocSheet |
| Note | `unique_id` | SchDocSheet |
| Pin | `designator` | SchDocComponent |
| Parameter | `name` | parent (Component/SheetSymbol/ParameterSet/Sheet) |
| Graphic | `unique_id` | parent |
| FootprintMap | `model_name` | SchDocComponent |

---

## 4. Query Methods on `SchDoc`

The main read method returns the entire tree:

```rust
impl SchDoc {
    /// Parse the flat record list into a structured sheet with all objects.
    pub fn sheet(&self) -> Result<SchDocSheet>;
}
```

Convenience accessors filter the `SheetObject` vec:

```rust
impl SchDocSheet {
    pub fn components(&self) -> Vec<&SchDocComponent>;
    pub fn component(&self, designator: &str) -> Option<&SchDocComponent>;
    pub fn wires(&self) -> Vec<&Wire>;
    pub fn buses(&self) -> Vec<&Bus>;
    pub fn net_labels(&self) -> Vec<&NetLabel>;
    pub fn power_objects(&self) -> Vec<&PowerObject>;
    pub fn ports(&self) -> Vec<&Port>;
    pub fn sheet_symbols(&self) -> Vec<&SheetSymbol>;
    pub fn junctions(&self) -> Vec<&Junction>;
    pub fn no_connects(&self) -> Vec<&NoConnect>;
    pub fn bus_entries(&self) -> Vec<&BusEntry>;
    pub fn parameter_sets(&self) -> Vec<&ParameterSet>;
    pub fn notes(&self) -> Vec<&Note>;
    pub fn graphics(&self) -> Vec<&Graphic>;
    pub fn parameters(&self) -> Vec<&Parameter>;
}
```

Similarly for child queries:

```rust
impl SchDocComponent {
    pub fn pins(&self) -> Vec<&Pin>;
    pub fn pin(&self, designator: &str) -> Option<&Pin>;
    pub fn parameters(&self) -> Vec<&Parameter>;
    pub fn parameter(&self, name: &str) -> Option<&Parameter>;
    pub fn graphics(&self) -> Vec<&Graphic>;
    pub fn footprints(&self) -> Vec<&FootprintMap>;
}

impl SheetSymbol {
    pub fn entries(&self) -> Vec<&SheetEntry>;
    pub fn entry(&self, name: &str) -> Option<&SheetEntry>;
    pub fn parameters(&self) -> Vec<&Parameter>;
}
```

### 4.1 Read path strategy

The read path resolves the flat OWNERINDEX tree into the nested type hierarchy:

1. **Build ownership map**: Walk all records, compute `parent_index → Vec<child_index>`.
2. **Parse Sheet** (record 0): Extract fonts, display settings → `SchDocSheet`.
3. **Parse Template** (record 1): Extract file_name, collect owned graphics.
4. **Walk top-level children** (owned by record 0, excluding Template) in order:
   - For each `SchRecord::Component`: collect children by OWNERINDEX, convert via
     reused `pin_from_internal`, `parameter_from_internal`, `graphic_from_record`,
     `build_footprint_maps`. Extract Designator child to `designator` field.
   - For each connectivity/leaf record: convert directly.
   - For each `SchRecord::SheetSymbol`: collect children, extract SheetName/SheetFileName
     field objects, convert entries and parameters.
   - For each `SchRecord::ParameterSet`: collect child parameters.
   - For each graphic/parameter at sheet level: wrap in `SheetObject::Graphic`/`Parameter`.
5. **Preserve ordering**: Insert into `Vec<SheetObject>` in the same order they appear
   in the flat list. Container variants (Component, SheetSymbol, ParameterSet) consume
   their children from the flat list — children are NOT also added as top-level objects.

### 4.2 Reused conversion functions from SchLib

| Function in `schlib_read.rs` | Used for |
|------------------------------|----------|
| `pin_from_internal(&SchPin)` | Convert component pins |
| `parameter_from_internal(&SchParameter)` | Convert parameters everywhere |
| All graphic converters (line, rect, arc, etc.) | Convert component and sheet-level graphics |
| `build_footprint_maps(&[SchRecord])` | Collapse implementation chain |

These functions need to be made accessible from `schdoc_read.rs`. Currently they're in
`schlib_read.rs` which is `pub(crate)`. Options:
- Move shared converters to a `api/sch_common.rs` module (recommended)
- Or import directly from `schlib_read` (works but couples the modules)

---

## 5. Mutation Methods on `SchDoc`

### 5.1 Whole-document update

The primary mutation path: read → modify → write back:

```rust
impl SchDoc {
    /// Replace the document contents with a new sheet structure.
    /// Converts the tree back to flat records and validates invariants.
    pub fn update_sheet(&mut self, sheet: &SchDocSheet) -> Result<()>;
}
```

### 5.2 Targeted mutations

For convenience, targeted methods that operate on the `objects` vec:

```rust
impl SchDoc {
    // Component CRUD
    pub fn add_component(&mut self, comp: SchDocComponent) -> Result<()>;
    pub fn update_component(&mut self, comp: &SchDocComponent) -> Result<()>;
    pub fn remove_component(&mut self, designator: &str) -> Result<()>;

    // Wire CRUD
    pub fn add_wire(&mut self, wire: Wire) -> Result<()>;
    pub fn remove_wire(&mut self, unique_id: &str) -> Result<()>;

    // NetLabel CRUD
    pub fn add_net_label(&mut self, label: NetLabel) -> Result<()>;
    pub fn remove_net_label(&mut self, unique_id: &str) -> Result<()>;

    // PowerObject CRUD
    pub fn add_power_object(&mut self, po: PowerObject) -> Result<()>;
    pub fn remove_power_object(&mut self, unique_id: &str) -> Result<()>;

    // Port CRUD
    pub fn add_port(&mut self, port: Port) -> Result<()>;
    pub fn remove_port(&mut self, unique_id: &str) -> Result<()>;

    // SheetSymbol CRUD
    pub fn add_sheet_symbol(&mut self, sym: SheetSymbol) -> Result<()>;
    pub fn update_sheet_symbol(&mut self, sym: &SheetSymbol) -> Result<()>;
    pub fn remove_sheet_symbol(&mut self, unique_id: &str) -> Result<()>;

    // Simple adds (leaf entities)
    pub fn add_junction(&mut self, j: Junction) -> Result<()>;
    pub fn add_no_connect(&mut self, nc: NoConnect) -> Result<()>;
    pub fn add_bus_entry(&mut self, be: BusEntry) -> Result<()>;
    pub fn remove_no_connect(&mut self, unique_id: &str) -> Result<()>;
}
```

### 5.3 Write path strategy

The write path flattens the tree back to `Vec<SchRecord>`:

1. **Emit Sheet** (RECORD=31) at index 0.
2. **Emit Template** (RECORD=39) at index 1, then template children.
3. **Walk `objects` in order**, for each `SheetObject`:
   - `Component`: emit SchComponent (RECORD=1), then children in order.
     For each `ComponentChild::Pin` → SchRecord::Pin, etc. Re-synthesize
     the Designator record (RECORD=34) from `comp.designator`.
     Re-synthesize the implementation chain from `FootprintMap`.
   - `Wire/Bus/NetLabel/...`: emit the corresponding SchRecord.
   - `SheetSymbol`: emit SchSheetSymbol (RECORD=15), then re-synthesize
     SheetName (RECORD=32) and SheetFileName (RECORD=33) field objects,
     then children in order.
   - `ParameterSet`: emit RECORD=43, then child parameters.
   - `Graphic/Parameter`: emit directly.
4. **Assign OWNERINDEX values** during emission: each record gets the index of its
   parent in the flat list. Sheet children get 0.
5. **Compute weight** = total record count.
6. **Validate invariants**.

### 5.4 Format-internal field preservation

Like SchLib's `update_component_internal`, the write path must preserve format-internal
fields that aren't exposed in the public API. When updating an existing component:

- `vault_guid`, `item_guid`, `revision_guid`, `symbol_vault_guid`, etc.
- `all_pin_count` (recomputed from children)
- `display_mode_count`
- `override_colors`, `pin_color`, color fields
- `has_only_current_part_info`

Strategy: when `update_component` is called, the write path finds the existing internal
`SchComponent` and copies format-internal fields onto the new record.

---

## 6. Dump Command (`dump_schdoc`)

### 6.1 Output format

```
sheet {
    style: custom
    width: 1500mil
    height: 950mil
    snap_grid: 10mil
}

component "U1" {
    lib_reference: "LM358"
    source_library: "opamps.SchLib"
    at: (100mil, 200mil)
    orientation: right

    pin 1 { at: (75mil, 225mil), electrical: input, name: "IN+" }
    pin 2 { at: (75mil, 175mil), electrical: input, name: "IN-" }

    rectangle { location: (85mil, 165mil), corner: (115mil, 235mil) }

    parameter Value = "LM358"
    parameter Comment = "Dual Op-Amp" { is_hidden: true }

    footprint "DIP-8" {
        map 1 -> 1
        map 2 -> 2
    }
}

wire { vertices: [(100mil, 200mil), (300mil, 200mil)] }

net_label "VCC3P3" { at: (300mil, 200mil) }

power "GND" { at: (100mil, 100mil), style: gnd_power, orientation: down }

junction { at: (200mil, 200mil) }

no_connect { at: (400mil, 300mil) }

sheet_symbol "Power" {
    at: (500mil, 500mil)
    size: (200mil, 300mil)
    file: "power.SchDoc"

    entry "VCC" { side: left, io_type: input }
}
```

### 6.2 Implementation

Extend `crates/altium-format-spec/src/dump.rs`:

```rust
pub fn dump_schdoc(doc: &SchDoc) -> Result<String, altium_format::AltiumFormatError> {
    let sheet = doc.sheet()?;
    let mut out = String::new();
    dump_sheet_properties(&mut out, &sheet);
    for obj in &sheet.objects {
        dump_sheet_object(&mut out, obj);
    }
    Ok(out)
}
```

The dump walks `sheet.objects` in order. Each `SheetObject` variant has a dump function.
Component children are dumped via the existing `dump_pin`, `dump_parameter`,
`dump_graphic`, `dump_footprint_map` helpers from SchLib.

### 6.3 CLI wiring

| Location | Change |
|----------|--------|
| `model.rs` | Add `SpecDomain::SchDoc` |
| `main.rs: detect_document_domain` | Add `"schdoc"` arm |
| `main.rs: detect_spec_domain` | Add `"schdoc-spec"` arm |
| `main.rs: default_output_for_spec` | Add `SchDoc` → `"SchDoc"` |
| `main.rs: default_spec_for_document` | Add `SchDoc` → `"schdoc-spec"` |
| `main.rs: run_dump` | Add `SchDoc` arm: `SchDoc::open` → `dump_schdoc` |

---

## 7. Implementation Order

### Phase 1: Types and Read Path

1. Create `api/sch_common.rs` — extract shared conversion functions from `schlib_read.rs`
   (`pin_from_internal`, `parameter_from_internal`, graphic converters,
   `build_footprint_maps`)
2. Create `api/schdoc_types.rs` — all public types from section 2 above
3. Create `api/schdoc_read.rs` — OWNERINDEX tree resolution → nested types
4. Add `SchDoc::sheet()` query method
5. Add convenience accessors on `SchDocSheet` and `SchDocComponent`
6. Wire up in `api/mod.rs`
7. Test against fixture files

### Phase 2: Write Path and Mutation

1. Create `api/schdoc_write.rs` — tree flattening back to `Vec<SchRecord>`
2. Add `SchDoc::update_sheet()` for whole-document writes
3. Add targeted CRUD methods (add/update/remove component, wire, etc.)
4. Format-internal field preservation on update
5. Test: create blank → add entities → save → reopen → verify
6. Test: open fixture → query → re-save → semantic CFB diff

### Phase 3: Dump Command

1. Add `dump_schdoc()` to dump.rs
2. Add `SpecDomain::SchDoc` variant
3. Wire CLI dispatch
4. Test: `cargo run -- dump <fixture.SchDoc>`

### Phase 4: Polish

1. Property tests for SchDoc API roundtrip
2. Idempotency tests
3. Cross-validate against real Altium files

---

## 8. Files to Create / Modify

### New files

| File | Purpose |
|------|---------|
| `crates/altium-format/src/api/schdoc_types.rs` | Public API types |
| `crates/altium-format/src/api/schdoc_read.rs` | Internal → public conversion |
| `crates/altium-format/src/api/schdoc_write.rs` | Public → internal conversion |
| `crates/altium-format/src/api/sch_common.rs` | Shared converters (Pin, Parameter, Graphic, FootprintMap) |

### Modified files

| File | Changes |
|------|---------|
| `crates/altium-format/src/api/mod.rs` | Add schdoc + sch_common modules, re-export types |
| `crates/altium-format/src/api/schlib_read.rs` | Move shared converters to sch_common, re-import |
| `crates/altium-format/src/api/schlib_write.rs` | Move shared converters to sch_common, re-import |
| `crates/altium-format/src/schdoc/mod.rs` | Add sheet() + CRUD methods |
| `crates/altium-format/src/lib.rs` | Re-export new SchDoc API types |
| `crates/altium-format-spec/src/model.rs` | Add `SpecDomain::SchDoc` |
| `crates/altium-format-spec/src/dump.rs` | Add `dump_schdoc()` |
| `crates/altium-format-spec/src/lib.rs` | Re-export `dump_schdoc` |
| `crates/altium-cli/src/main.rs` | Add SchDoc to dump/plan/apply dispatch |

---

## 9. Testing Strategy

### Unit tests (no fixtures)

- `new_blank_ad26()` → `sheet()` → verify default fonts, grid, empty objects
- Add component via CRUD → `sheet()` → verify component in objects
- Add wire/netlabel/junction → save → reopen → `sheet()` → verify ordering preserved
- Duplicate designator rejection
- OWNERINDEX fixup correctness (add/remove entities)

### Fixture tests (`#[cfg(feature = "test-fixtures")]`)

- Open each LimeSDR SchDoc → `sheet()` → verify no errors
- Verify `sheet.objects` count matches expected record census
- Verify `sheet.components().len()` matches RECORD=1 count
- Save-as → semantic CFB diff against original

### Property tests (`#[cfg(feature = "proptest")]`)

- Random SchDocComponent → add → sheet() → verify fields
- Random Wire/NetLabel/PowerObject → add → sheet() → verify
- Add/remove cycles → validate_invariants passes

---

## 10. UniqueId Identity Architecture

### 10.1 The Problem (Topological Naming)

The spec reconciler needs **stable identity** to associate spec entities with SchDoc records
across runs. Without embedded identity, the reconciler must match by coordinates — which
breaks whenever components move (the same problem as FreeCAD's topological naming problem).

```
Run 1: spec says `no_connect { on: $U1.pin3 }` → solver places at (400, 300) → UNIQUEID=XYZABCDE
Run 2: U1 moved → pin3 is now at (600, 500)
       Reconciler finds UNIQUEID=XYZABCDE → moves it to (600, 500)
       (Without identity: deletes old no-connect, creates new one — loses Altium-side edits)
```

### 10.2 UniqueId Coverage

Nearly every spec-relevant SchDoc record type has a `UNIQUEID` field (8-char uppercase A-Z):

| Record Type | Has UniqueId | Spec-relevant |
|-------------|:---:|:---:|
| Component (1) | Yes | Yes |
| Wire (27) | Yes | Yes |
| Bus (26) | Yes | Yes |
| NetLabel (25) | Yes | Yes |
| PowerObject (17) | Yes | Yes |
| Port (18) | Yes | Yes |
| NoConnect (22) | Yes | Yes |
| Junction (29) | Yes | Yes |
| BusEntry (37) | Yes | Yes |
| SheetSymbol (15) | Yes | Yes |
| SheetEntry (16) | Yes | Yes |
| ParameterSet (43) | Yes | Yes |
| Blanket (225) | Yes | Yes |
| Decorative graphics (3-12) | **No** | Rarely (component children, addressed via OWNERINDEX) |

The only records WITHOUT UniqueId are pure decorative graphics (Bezier, Polyline, Polygon,
Ellipse, Arc, etc.) which are component children addressed via OWNERINDEX, not independently
referenced.

### 10.3 Altium's Deterministic UniqueId Algorithm

Altium has two UniqueId generation paths:

1. **Random** (`SchDataUtils.GenerateNewUniqueId`): GUID-seeded `Random` → 8 random A-Z chars.
   Used during normal UI operations.

2. **Deterministic** (`UniqueIdUtils.GenerateUniqueId(seed)`): MD5-based hash → 8-char base-26.
   Used for migration/remapping. **This is what we use.**

Source: `AD26-dotnet/Altium.Sch.Base/Altium.Sch.Base.Utils/UniqueIdUtils.cs`

**Algorithm (exact C# translation):**

```
Input:  seed string (e.g., "spec:psu:inst:R1")
Step 1: Encode seed as Windows-1252 bytes (= ASCII for our seeds)
Step 2: Compute MD5 digest → 16 bytes → format as 32-char uppercase hex string
Step 3: Process hex string in 8 chunks of 4 characters each:
        For each chunk [c0, c1, c2, c3]:
            h = 19
            h = h * 31 + hex_value(c0)    // hex_value: '0'-'9'→0-9, 'A'-'F'→10-15
            h = h * 31 + hex_value(c1)
            h = h * 31 + hex_value(c2)
            h = h * 31 + hex_value(c3)
            output_char = "ABCDEFGHIJKLMNOPQRSTUVWXYZ"[h % 26]
Result: 8 uppercase ASCII letters (A-Z)
```

**Rust implementation:**

```rust
use md5;

const ALPHABET: &[u8; 26] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";

/// Generate a deterministic UniqueId from a seed string.
/// Replicates Altium's `UniqueIdUtils.GenerateUniqueId(seed)` exactly.
pub fn unique_id_from_seed(seed: &str) -> UniqueId {
    // Step 1-2: MD5 of ASCII/Windows-1252 bytes → uppercase hex
    let digest = md5::compute(seed.as_bytes());
    let hex = format!("{:X}", digest); // 32-char uppercase hex

    // Step 3: fold 4-hex-char chunks into base-26 letters
    let hex_bytes = hex.as_bytes();
    let mut result = [0u8; 8];
    for i in 0..8 {
        let mut h: i64 = 19;
        for j in 0..4 {
            let c = hex_bytes[i * 4 + j];
            let v = match c {
                b'0'..=b'9' => (c - b'0') as i64,
                b'A'..=b'F' => (c - b'A') as i64 + 10,
                _ => 0,
            };
            h = h * 31 + v;
        }
        result[i] = ALPHABET[(h.rem_euclid(26)) as usize];
    }
    UniqueId::from_str(std::str::from_utf8(&result).unwrap()).unwrap()
}

/// Increment a UniqueId in base-26 (A=0, Z=25). Used for collision resolution.
/// Replicates Altium's `UniqueIdUtils.GetNextUniqueId()`.
pub fn next_unique_id(id: &UniqueId) -> UniqueId {
    let bytes = id.as_str().as_bytes();
    let mut result = [0u8; 8];
    result.copy_from_slice(bytes);
    let mut carry = true;
    for i in (0..8).rev() {
        if carry {
            let val = result[i] - b'A';
            if val == 25 { // Z → wrap to A, carry
                result[i] = b'A';
            } else {
                result[i] = b'A' + val + 1;
                carry = false;
            }
        }
    }
    UniqueId::from_str(std::str::from_utf8(&result).unwrap()).unwrap()
}
```

### 10.4 Seed Format Convention

Seeds follow a hierarchical path that uniquely identifies each spec entity:

```
spec:{spec_file_stem}:{entity_type}:{identity_key}
```

| Spec Entity | Seed Format | Example |
|------------|------------|---------|
| Component instance | `spec:{file}:inst:{designator}` | `spec:psu:inst:R1` |
| Wire (net stub) | `spec:{file}:wire:{net}:{pin_ref}` | `spec:psu:wire:VCC:U1.8` |
| Wire (explicit) | `spec:{file}:wire:{binding_name}` | `spec:psu:wire:clk_route` |
| NetLabel | `spec:{file}:netlabel:{net}:{index}` | `spec:psu:netlabel:VCC:0` |
| PowerObject | `spec:{file}:power:{net}:{index}` | `spec:psu:power:GND:0` |
| NoConnect | `spec:{file}:nc:{comp}.{pin}` | `spec:psu:nc:U1.3` |
| Junction | `spec:{file}:junc:{net}:{index}` | `spec:psu:junc:SDA:0` |
| Port | `spec:{file}:port:{name}` | `spec:psu:port:DATA_BUS` |
| SheetSymbol | `spec:{file}:sheetsym:{name}` | `spec:psu:sheetsym:Regulators` |
| SheetEntry | `spec:{file}:sheetentry:{sym}:{name}` | `spec:psu:sheetentry:Regulators:VIN` |
| Graphic (binding) | `spec:{file}:gfx:{binding_name}` | `spec:psu:gfx:border_rect` |
| Graphic (unnamed) | `spec:{file}:gfx:anon:{type}:{index}` | `spec:psu:gfx:anon:line:3` |

**Collision resolution**: After computing the hash, check against all existing UniqueIds in the
document. If collision, call `next_unique_id()` repeatedly until unique. Store the actual used
ID in the reconciler state so subsequent runs use the same value.

### 10.5 Reconciler Identity Matching

On each spec run, the reconciler:

1. **Computes expected UniqueIds** from spec entity seeds (deterministic)
2. **Scans existing SchDoc records** building a `UniqueId → record_index` map
3. **Matches spec entities to records by UniqueId**:
   - Found → compare fields, emit `Update` or `Unchanged`
   - Not found → emit `Add` (with the deterministic UniqueId)
4. **Records NOT matched by any spec entity** → left untouched (additive semantics)

Records created manually in Altium will have random UniqueIds (no `spec:` prefix in the seed),
so they never collide with spec-generated IDs and are always preserved.

### 10.6 Semantic Placement via Spec References

With UniqueId identity, the spec language can use **semantic references** instead of raw
coordinates. The solver resolves references to absolute positions at apply time:

```
// Spec file — semantic, no coordinates
no_connect { on: $U1.pin3 }
wire { from: $U1.pin8, label: "VCC" }
wire { from: $R1.pin2, to: $C1.pin1 }

// Solver resolves at apply time:
//   $U1.pin3 → looks up U1's position + orientation + pin3 offset → absolute coords
//   UniqueId = unique_id_from_seed("spec:psu:nc:U1.3")
```

This means LLM agents never need to track coordinates — they declare intent ("no-connect on
pin 3 of U1") and the solver does the spatial math.

### 10.7 Metadata via UniqueId (No Custom Keys Needed)

The deterministic UniqueId scheme makes custom metadata fields unnecessary for the reconciler's
core needs. The seed itself encodes the semantic meaning:

- `UNIQUEID=XYZABCDE` on a wire → reverse-hash isn't needed; the reconciler holds the mapping
  `seed → UniqueId` in memory during the run
- Between runs, the same seed produces the same UniqueId → stable matching
- The spec file IS the metadata store (what entity, what rule, what constraint)

If richer per-record metadata is ever needed (e.g., solver iteration state), the
`ParameterSet` mechanism described in the README can overlay it. But for the core
reconciliation loop, UniqueId alone is sufficient.

---

## 11. Open Questions

1. **Shared child types**: Reuse `api::Pin`, `api::Parameter`, `api::Graphic`,
   `api::FootprintMap` from SchLib. Decision: **yes, reuse**. Same domain concepts.

2. **Template handling**: Template is always present with exactly one SchTemplate record.
   Decision: extract as `Template { file_name, children }` field on `SchDocSheet`,
   not as a `SheetObject` variant. It's a structural element, not user content.

3. **Harness types**: HarnessConnector, SignalHarness, HighLevelCode* records reuse
   existing internal structs. Decision: define API types but mark as future — the
   parser currently errors if these streams exist, so the API types will only be
   exercised when we implement those parsers.

4. **SchDocComponent format-internal fields**: `vault_guid`, `all_pin_count`,
   `has_only_current_part_info`, etc. Decision: NOT exposed in public API. Preserved
   by write path when updating existing components, computed for new ones.

5. **SchSymbol (RECORD=3)**: IEEE symbol graphics inside components. Currently skipped
   in SchLib read path. Decision: skip for now, add as `Graphic::Symbol` variant later.
