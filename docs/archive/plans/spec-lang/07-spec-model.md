# 07 - SpecModel (Typed Intermediate Representation)

## Location

`crates/altium-format-ops/src/spec/model.rs`

## Purpose

The SpecModel is the fully resolved, typed representation of a spec file after:
- All let bindings evaluated and spreads expanded
- All imports resolved and namespaces merged
- All anchor references resolved to absolute coordinates
- All row/column/grid blocks expanded to individual pads
- All types checked and coerced (dims to Coord, enums resolved)

The reconciler (§08) compares the SpecModel against a loaded document.

## Design Principle

The SpecModel describes **desired state**, not mutations. Each type represents
what an entity SHOULD look like. Fields are `Option<T>` where the spec does not
constrain a value (the reconciler leaves unspecified fields unchanged in existing
entities).

## Types

### SchLib SpecModel

```rust
pub struct SchLibSpec {
    /// Components declared in the spec (identity key: lib_reference).
    pub components: Vec<ComponentSpec>,
}

pub struct ComponentSpec {
    pub lib_reference: String,             // identity key
    pub designator: Option<String>,
    pub description: Option<String>,
    pub component_kind: Option<ComponentKind>,
    pub part_count: Option<i32>,
    pub show_hidden_pins: Option<bool>,

    /// Pins at component level (owner_part_id = 0 / shared).
    pub pins: Vec<PinSpec>,
    pub parameters: Vec<ParameterSpec>,
    pub aliases: Vec<String>,
    pub footprints: Vec<FootprintMapSpec>,
    pub graphics: Vec<GraphicSpec>,

    /// Per-part pins and graphics.
    pub parts: Vec<PartSpec>,
}

pub struct PartSpec {
    pub part_number: i32,                  // 1, 2, 3, ...
    pub pins: Vec<PinSpec>,
    pub graphics: Vec<GraphicSpec>,
}

pub struct PinSpec {
    pub designator: String,                // identity key
    pub name: Option<String>,
    pub electrical: Option<PinElectricalType>,
    pub length: Option<Coord>,
    pub location: CoordPoint,              // absolute (resolved from anchors)
    pub orientation: RotationBy90,         // absolute (resolved from auto)
    pub is_hidden: Option<bool>,
    pub hidden_net_name: Option<String>,
    pub owner_part_id: i32,                // 0 = shared, N = part N
}

pub struct ParameterSpec {
    pub name: String,                      // identity key
    pub text: String,
    pub is_hidden: Option<bool>,
}

pub struct FootprintMapSpec {
    pub model_name: String,                // identity key
    pub maps: Vec<PinPadMap>,
    /// If from an import, the source file path for validation.
    pub source: Option<PathBuf>,
}

pub struct PinPadMap {
    pub pin: String,
    pub pad: String,
}
```

### PcbLib SpecModel

```rust
pub struct PcbLibSpec {
    /// Footprints declared in the spec (identity key: display_name).
    pub footprints: Vec<FootprintSpec>,
}

pub struct FootprintSpec {
    pub display_name: String,              // identity key
    pub description: Option<String>,
    pub height: Option<Coord>,
    pub pattern: Option<String>,

    pub pads: Vec<PadSpec>,
    pub graphics: Vec<PcbGraphicSpec>,
}

pub struct PadSpec {
    pub pad_name: String,                  // identity key
    pub at: CoordPoint,                    // absolute position
    pub shape: Option<PadShape>,
    pub x_size: Option<Coord>,
    pub y_size: Option<Coord>,
    pub rotation: Option<f64>,
    pub hole_size: Option<Coord>,
    pub is_plated: Option<bool>,
    pub layer: Option<V6Layer>,
    pub pad_mode: Option<PadMode>,
    pub solder_mask_expansion: Option<Coord>,
    pub paste_mask_expansion: Option<Coord>,
    pub plane_connection: Option<PlaneConnectionStyle>,
    pub relief_conductor_width: Option<Coord>,
    pub relief_entries: Option<i32>,
    pub relief_air_gap: Option<Coord>,
}
```

### Graphics

```rust
pub struct GraphicSpec {
    pub unique_id: String,                 // from binding name (spec:{context}:{name})
    pub graphic_type: GraphicType,
    pub properties: GraphicProperties,
}

pub enum GraphicType {
    Line, Rectangle, Arc, EllipticalArc, Ellipse,
    Polyline, Polygon, Bezier, Pie, RoundRectangle,
    Label, TextFrame, Image,
}

/// Union of all graphic property fields.
/// Only fields relevant to the graphic_type are Some.
pub struct GraphicProperties {
    // Box types (rectangle, round_rectangle, text_frame, image)
    pub from: Option<CoordPoint>,
    pub to: Option<CoordPoint>,
    pub is_solid: Option<bool>,
    pub corner_x_radius: Option<Coord>,
    pub corner_y_radius: Option<Coord>,

    // Center+radius types (arc, ellipse, pie)
    pub center: Option<CoordPoint>,
    pub radius: Option<Coord>,
    pub secondary_radius: Option<Coord>,
    pub start_angle: Option<f64>,
    pub end_angle: Option<f64>,

    // Segment types (line)
    // (uses from/to)

    // Vertex-list types (polyline, polygon, bezier)
    pub points: Option<Vec<CoordPoint>>,

    // Common visual
    pub color: Option<Color>,
    pub area_color: Option<Color>,
    pub line_width: Option<Coord>,

    // Text (label, text_frame)
    pub text: Option<String>,
    pub font_id: Option<i32>,
    pub at: Option<CoordPoint>,

    // Image
    pub file_name: Option<String>,
    pub image_data: Option<Vec<u8>>,

    // PCB-specific
    pub layer: Option<V6Layer>,
    pub width: Option<Coord>,
    pub closed: Option<bool>,
    pub show_border: Option<bool>,
}

/// PCB graphic (track, arc, fill, region, text, via, component_body, polyline)
pub struct PcbGraphicSpec {
    pub unique_id: String,
    pub graphic_type: PcbGraphicType,
    pub properties: PcbGraphicProperties,
}
```

## unique_id Generation

When the compiler encounters a binding prefix (`body = rectangle { ... }`),
it generates a unique_id following the scheme from spec-lang.md §10:

| Context | unique_id format |
|---------|-----------------|
| Component-level graphic | `spec:{component}:{name}` |
| Part-scoped graphic | `spec:{component}:part{N}:{name}` |
| Footprint graphic | `spec:{footprint}:{name}` |
| Unnamed graphic | `spec:{context}:{type}_{counter}` |

Examples:
- `body = rectangle { ... }` in component `R_0603` → `spec:R_0603:body`
- `body = rectangle { ... }` in part 1 of `LM358` → `spec:LM358:part1:body`
- Unnamed `line { ... }` → `spec:R_0603:line_0`, `spec:R_0603:line_1`, ...

## Compilation Pipeline

```rust
/// Compile a resolved spec into a typed model.
pub fn compile_spec(
    resolved: &ResolvedSpec,
    domain: SpecDomain,          // SchLib or PcbLib
) -> Result<SpecModel, SpecError>

pub enum SpecDomain { SchLib, PcbLib }

pub enum SpecModel {
    SchLib(SchLibSpec),
    PcbLib(PcbLibSpec),
}
```

### Compiler State

```rust
struct SpecCompiler<'a> {
    domain: SpecDomain,
    /// Scope stack for let bindings and entity bindings.
    scopes: Vec<Scope>,
    /// Named imports for namespace resolution.
    imports: &'a IndexMap<String, (PathBuf, SpecFile)>,
    /// Counter for unnamed graphic unique_ids.
    unnamed_counter: usize,
}

struct Scope {
    /// Let bindings: name -> evaluated value
    lets: IndexMap<String, Value>,
    /// Entity bindings: name -> entity reference (for anchors)
    entities: IndexMap<String, EntityBinding>,
}

enum EntityBinding {
    Graphic(GraphicSpec),          // for anchor resolution
    Pin(PinSpec),                  // for after:/before: refs
    Pad(PadSpec),                  // for after:/before: refs
}
```

### Compilation Steps (per component/footprint)

1. **Enter scope**: Push new scope for the entity
2. **Collect bindings**: Scan all items to register binding names (forward refs)
3. **Evaluate let bindings**: Evaluate in dependency order (detect cycles)
4. **Compile graphics**: Evaluate graphic declarations, register anchors
5. **Compile pins/pads**: Resolve anchor placement, expand layouts
6. **Compile parameters/aliases**: Simple evaluation
7. **Compile footprint maps**: Validate against imports
8. **Exit scope**: Pop scope

Step 2 (forward reference support) requires a two-pass approach: first
register all binding names, then evaluate. Within a scope, all bindings are
visible regardless of source order (spec-lang.md §9.1).

## Test Strategy

- Compile each example from spec-lang.md §17
- Verify absolute coordinates after anchor resolution
- Verify unique_id generation
- Verify spread evaluation
- Verify forward reference resolution
- Verify part-scoped binding isolation
- Verify type coercion (bare integers → mils, enum resolution)
- Error: circular binding
- Error: undefined reference
- Error: type mismatch
