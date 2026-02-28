 Completed:
  - SchLib high-level API (read + write) — types, read path, write path, query/mutation methods, tests
  - PcbLib placeholder types

  Next steps from the plan (in priority order):

  1. Wire up the spec executor/reconciler — The altium-format-spec crate should use the new component()/update_component() API instead
  of low-level record access. This is a key consumer.
  2. Replace DumpView usage in CLI — The dump command currently uses SchLibComponentDumpView. It should switch to api::Component and
  format into spec syntax, eventually deprecating the DumpView types.
  3. PcbLib read/write paths — The types are defined but footprint()/add_footprint() etc. aren't implemented yet. The plan explicitly
  deferred this.
  4. SchDoc / PcbDoc APIs — Different structure (flat records with cross-references), sketched in the design doc but not planned yet.


Deferred: PcbLib write path, SchDoc, PcbDoc, spec executor/reconciler rewiring, CLI
 CRUD commands, DumpView deprecation.

# High-Level API Design

Public API for querying and mutating Altium documents from `altium-format`.

## Goals

1. **Unified surface** — the same types serve crate consumers, altium-cli commands,
   and the spec language executor/reconciler.
2. **Encapsulate format complexity** — consumers never touch `SchRecord`, `PcbPrimitive`,
   OWNERINDEX linking, sidecar streams, or CFB structure.
3. **Fail-fast on corruption** — all mutations go through document methods that
   validate invariants before and after changes.

## Design Principles

- **Natural keys as identity.** No opaque `RefId` handles. Components are identified
  by `lib_reference`, pins by `designator`, parameters by `name`, etc.
- **Component-level update granularity.** To change a pin, load the component, modify
  the pin, update the component. No standalone `update_pin()` on the document.
- **Every record type must be modeled.** The public types must cover every
  `SchRecord` / `PcbPrimitive` variant. No passthrough, no opaque carry-through.
  If a record type exists in the file, it has a field in the public struct or
  a variant in the public enum. This follows the cardinal rule: unknown data is
  a bug, not something to silently preserve.
- **Domain types everywhere.** `Coord` not `f64`, `PinElectricalType` not `String`,
  `CoordPoint` not `(f64, f64)`.
- **Rename = remove + add.** Changing a natural key (e.g. `lib_reference`) is not
  supported through update. Remove the old entity and add a new one.

## Natural Key Table

| Entity       | Key             | Unique within |
| ------------ | --------------- | ------------- |
| Component    | `lib_reference` | SchLib        |
| Pin          | `designator`    | Component     |
| Parameter    | `name`          | Component     |
| Graphic      | `unique_id`     | Component     |
| FootprintMap | `model_name`    | Component     |
| Alias        | alias name      | SchLib        |
| Footprint    | `display_name`  | PcbLib        |
| Pad          | `pad_name`      | Footprint     |

## Public Types

### SchLib

```rust
impl SchLib {
    // ── Query ────────────────────────────────────────────────
    pub fn component_names(&self) -> Vec<&str>;
    pub fn component(&self, lib_ref: &str) -> Result<Component>;
    pub fn components(&self) -> Result<Vec<Component>>;

    // ── Mutate ───────────────────────────────────────────────
    pub fn add_component(&mut self, comp: Component) -> Result<()>;
    pub fn update_component(&mut self, comp: &Component) -> Result<()>;
    pub fn remove_component(&mut self, lib_ref: &str) -> Result<()>;
}
```

### Component (SchLib)

```rust
pub struct Component {
    pub lib_reference: String,
    pub designator: Option<String>,
    pub description: Option<String>,
    pub component_kind: Option<ComponentKind>,
    pub part_count: i32,
    pub show_hidden_pins: bool,

    pub pins: Vec<Pin>,
    pub parameters: Vec<Parameter>,
    pub footprints: Vec<FootprintMap>,
    pub graphics: Vec<Graphic>,
    pub aliases: Vec<String>,
}
```

### Pin

```rust
pub struct Pin {
    pub designator: String,
    pub name: String,
    pub electrical: PinElectricalType,
    pub location: CoordPoint,
    pub length: Coord,
    pub orientation: RotationBy90,
    pub is_hidden: bool,
    pub hidden_net_name: String,
    pub owner_part_id: i32,
    // Full sidecar fields merged in:
    pub swap_id: String,
    pub swap_id_part: String,
    pub default_value: String,
    pub pin_package_length: String,
    pub propagation_delay: String,
    pub symbol_line_width: i32,
    // ... remaining pin fields as needed
}
```

### Parameter

```rust
pub struct Parameter {
    pub name: String,
    pub text: String,
    pub is_hidden: bool,
    pub read_only: ParameterReadOnlyState,
    pub location: CoordPoint,
    pub orientation: RotationBy90,
    pub color: Color,
    pub font_id: i32,
}
```

### FootprintMap

```rust
pub struct FootprintMap {
    pub model_name: String,
    pub description: String,
    pub pin_pad_maps: Vec<PinPadMap>,
}

pub struct PinPadMap {
    pub pin: String,
    pub pad: String,
}
```

### Graphic (enum)

Each variant carries type-specific geometry plus common fields from
`SchPrimitiveBase` / `SchGraphicalBase`.

```rust
pub enum Graphic {
    Line(LineGraphic),
    Rectangle(RectangleGraphic),
    RoundRectangle(RoundRectangleGraphic),
    Arc(ArcGraphic),
    EllipticalArc(EllipticalArcGraphic),
    Ellipse(EllipseGraphic),
    Pie(PieGraphic),
    Polyline(PolylineGraphic),
    Polygon(PolygonGraphic),
    Bezier(BezierGraphic),
    Image(ImageGraphic),
    Label(LabelGraphic),
    TextFrame(TextFrameGraphic),
}

// Common fields present on every variant:
//   unique_id: String
//   owner_part_id: i32
//   display_mode: i32
//   location: CoordPoint
//   color: Color
//   area_color: Color

// Example variant:
pub struct ArcGraphic {
    // common
    pub unique_id: String,
    pub owner_part_id: i32,
    pub display_mode: i32,
    pub location: CoordPoint,
    pub color: Color,
    pub area_color: Color,
    // type-specific
    pub radius: Coord,
    pub start_angle: f64,
    pub end_angle: f64,
    pub line_width: PenWidth,
}
```

### PcbLib

```rust
impl PcbLib {
    // ── Query ────────────────────────────────────────────────
    pub fn footprint_names(&self) -> Vec<&str>;
    pub fn footprint(&self, display_name: &str) -> Result<Footprint>;
    pub fn footprints(&self) -> Result<Vec<Footprint>>;

    // ── Mutate ───────────────────────────────────────────────
    pub fn add_footprint(&mut self, fp: Footprint) -> Result<()>;
    pub fn update_footprint(&mut self, fp: &Footprint) -> Result<()>;
    pub fn remove_footprint(&mut self, display_name: &str) -> Result<()>;
}
```

### Footprint (PcbLib)

```rust
pub struct Footprint {
    pub display_name: String,
    pub description: String,
    pub pattern: String,
    pub height: Coord,

    pub pads: Vec<Pad>,
    pub graphics: Vec<PcbGraphic>,
}
```

### Pad

```rust
pub struct Pad {
    pub pad_name: String,
    pub location: CoordPoint,
    pub shape: PadShape,
    pub x_size: Coord,
    pub y_size: Coord,
    pub rotation: f64,
    pub hole_size: Coord,
    pub is_plated: bool,
    pub layer: V6Layer,
    pub pad_mode: PadStackMode,
    pub solder_mask_expansion: Coord,
    pub paste_mask_expansion: Coord,
    pub plane_connection: PlaneConnectionStyle,
    pub relief_conductor_width: Coord,
    pub relief_entries: i32,
    pub relief_air_gap: Coord,
    // ... remaining pad fields
}
```

### PcbGraphic (enum)

```rust
pub enum PcbGraphic {
    Track(TrackGraphic),
    Arc(PcbArcGraphic),
    Fill(FillGraphic),
    Region(RegionGraphic),
    Text(TextGraphic),
    Via(ViaGraphic),
    ComponentBody(ComponentBodyGraphic),
}

// Common fields on every variant:
//   layer: V6Layer
//   flags: PcbFlags
//   net_index: u16
```

## Update Semantics

When `update_component(&comp)` is called:

1. Find existing `SchLibComponent` by `comp.lib_reference`. Error if not found.
2. Build the full internal record list from the `Component` fields (pins,
   parameters, graphics, implementations — every record type is modeled).
3. Rebuild sidecar data from the new pin list.
4. Update the header index entry (description, part_count, aliases).
5. Run `validate_invariants()`.

The same pattern applies to `update_footprint` on PcbLib.

### Children Diffing

Within an update, children are matched by natural key:

- **Pins** matched by `designator`. Present in new but not old → add. Present
  in old but not new → remove. Both → update in place.
- **Parameters** matched by `name`. Same add/remove/update logic.
- **Graphics** matched by `unique_id`. Same logic.
- **FootprintMaps** matched by `model_name`. Same logic.

This means removing a pin is done by omitting it from the component's pin list
before calling `update_component`.

## SchDoc / PcbDoc (Future)

These document types have different top-level structure (flat record lists
with cross-references rather than component containers), so their APIs will
differ. Sketched here for completeness:

```rust
impl SchDoc {
    pub fn sheet(&self) -> Result<Sheet>;
    pub fn components(&self) -> Result<Vec<SchDocComponent>>;
    pub fn wires(&self) -> Result<Vec<Wire>>;
    pub fn net_labels(&self) -> Result<Vec<NetLabel>>;
    // ... other placed objects

    pub fn add_component(&mut self, comp: SchDocComponent) -> Result<()>;
    pub fn update_component(&mut self, comp: &SchDocComponent) -> Result<()>;
    pub fn remove_component(&mut self, designator: &str) -> Result<()>;
    // ... other mutation methods
}

impl PcbDoc {
    pub fn board(&self) -> Result<Board>;
    pub fn components(&self) -> Result<Vec<PcbDocComponent>>;
    pub fn nets(&self) -> Result<Vec<Net>>;
    // ...
}
```

## Spec Executor Usage

The executor uses the public API to apply specs:

```rust
pub fn apply_spec_schlib(spec: &SchLibSpec, doc: &mut SchLib) -> Result<()> {
    for comp_spec in &spec.components {
        match doc.component(&comp_spec.lib_reference) {
            Ok(mut existing) => {
                // Merge spec fields into existing component
                apply_component_spec(&mut existing, comp_spec);
                doc.update_component(&existing)?;
            }
            Err(_) => {
                // Build new component from spec
                let comp = component_from_spec(comp_spec);
                doc.add_component(comp)?;
            }
        }
    }
    Ok(())
}
```

The reconciler queries existing state to produce Add/Update/Unchanged ECOs:

```rust
pub fn reconcile_schlib(spec: &SchLibSpec, doc: &SchLib) -> Result<ECO> {
    for comp_spec in &spec.components {
        match doc.component(&comp_spec.lib_reference) {
            Ok(existing) => {
                // Diff spec vs existing → Update or Unchanged entries
            }
            Err(_) => {
                // Not found → Add entry
            }
        }
    }
}
```

## DumpView Deprecation

The existing `SchLibComponentDumpView`, `PcbLibFootprintDumpView`, etc. will
be replaced by the public types above. The `dump` CLI command will use
`Component` / `Footprint` and format them into spec syntax.

## Implementation Order

1. SchLib `Component` / `Pin` / `Parameter` / `FootprintMap` / `Graphic` types
2. `SchLib::component()` read path (project internal types into public types)
3. `SchLib::add_component()` / `update_component()` / `remove_component()` write path
4. Wire up spec executor and reconciler
5. Replace DumpView usage in CLI dump command
6. PcbLib `Footprint` / `Pad` / `PcbGraphic` types + read/write paths
7. SchDoc / PcbDoc (future)
