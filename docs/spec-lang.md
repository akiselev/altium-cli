# Altium Spec Language Specification

Version: 0.2 (draft)
File extensions: `.schlib-spec`, `.pcblib-spec`

## 1. Overview

The Altium Spec Language is a declarative DSL for describing the **desired state** of Altium
Designer library files (SchLib, PcbLib). It is the complement to the imperative Ops Language
(`docs/ops-lang-spec.md`): where ops says "add this, edit that", specs say "the document
should look like this."

The CLI reads the spec, diffs it against the current document, produces a plan of minimal
changes (an engineering change order), and applies them idempotently. Running the same spec
file twice is a no-op.

### Relationship to the Ops Language

The spec language **reuses the ops lexer, expression language, and type system** (§5–§9 of
`ops-lang-spec.md`). The differences are:

| | Ops Language | Spec Language |
|---|---|---|
| Paradigm | Imperative (do X, then Y) | Declarative (look like this) |
| Idempotent | No (add twice = two copies) | Yes (apply twice = no-op) |
| Verbs | `add_*`, `edit`, `remove`, `query` | None — entities declared, not commanded |
| Identity | Implicit (by execution order) | Explicit (by natural key: lib_ref, designator, name) |
| Placement | `place $pin { on: $rect.top }` | Same anchor syntax inside entity blocks |
| Output | `ApplyReport` (what happened) | ECO (engineering change order) + `ApplyReport` |

### Relationship to other docs

- `docs/ops-lang-spec.md` — imperative ops language (shared expression/type system)
- `docs/ops-design.md` — lowering pipeline, field mapping tables
- `docs/query-lang.md` — AQL reference (not used in spec lang directly)


## 2. Design Goals

1. **Declarative.** Describe what, not how. No mutation verbs.
2. **Idempotent.** Applying the same spec to the same document is always a no-op.
3. **Additive by default.** Entities in the document but NOT in the spec are untouched.
   The spec is a subset assertion, not a complete truth.
4. **Token-minimal.** Reuses the ops expression language. Natural keys are positional
   (the identifier/string after the entity keyword). Quotes optional for simple names.
5. **Placement-aware.** Anchor-based relative placement for both SchLib pins and PcbLib
   pads. Row/column/grid layout primitives for footprint pad patterns.
6. **ECO-grade output.** The plan command generates a full engineering change order,
   suitable for real-world hardware development review processes.
7. **Agent-friendly.** Same expression language, same error reporting, same schema
   introspection as the ops language.
8. **Composable.** Import other spec files to link footprints and components, and to
   split large libraries across files.


## 3. File Structure & CLI

### 3.1 File Extensions

| Extension | Domain | Output file |
|-----------|--------|-------------|
| `.schlib-spec` | SchLib | `.schlib` (same base name) |
| `.pcblib-spec` | PcbLib | `.pcblib` (same base name) |

Default output: `foo.schlib-spec` → `foo.SchLib`. Override with `--output`.

### 3.2 CLI Commands

```bash
# Plan: show ECO without mutating
altium plan my-parts.schlib-spec
altium plan my-footprints.pcblib-spec

# Apply: generate ECO + execute
altium apply my-parts.schlib-spec                     # creates/updates my-parts.SchLib
altium apply my-parts.schlib-spec --output custom.SchLib

# Plan/apply with an existing document (update mode)
altium plan my-parts.schlib-spec --target existing.SchLib
altium apply my-parts.schlib-spec --target existing.SchLib

# Dump: reverse-generate a spec from an existing library
altium dump my-parts.SchLib                            # outputs my-parts.schlib-spec
altium dump my-parts.PcbLib --output footprints.pcblib-spec

# JSON output
altium plan my-parts.schlib-spec --json
altium apply my-parts.schlib-spec --report-json
```

When no `--target` is given, the tool looks for the output file (e.g., `my-parts.SchLib`).
If it exists, it's opened and updated (reconciliation). If it doesn't exist, a new empty
document is created and the full spec is applied.

### 3.3 File Layout

A spec file is a sequence of **let bindings**, **import declarations**, and **entity
declarations**:

```
// Imports
import "standard-footprints.pcblib-spec" as footprints

// Shared templates
let passive_pin = { electrical: passive, length: 25, side: outside }

// Entity declarations
component R_0603 {
    designator: "R?"
    // ...
}

component C_0805 {
    // ...
}
```


## 4. Entity Declarations

### 4.1 Entity Names: Quoted and Unquoted

Entity names (the identifier after `component`, `pin`, `pad`, etc.) can be **unquoted**
when they are valid identifiers or integers, or **quoted** when they contain spaces or
special characters:

```
component R_0603 { ... }              // unquoted identifier
component "My Special Part" { ... }   // quoted (has spaces)
pin 1 { ... }                         // unquoted integer
pin VCC { ... }                       // unquoted identifier
pad A1 { ... }                        // unquoted identifier
pad "EP" { ... }                      // quoted (works too)
```

**Grammar for entity names:**
```
entity_name = STRING | IDENT | INTEGER ;
```

Unquoted names are treated as strings (not as expressions). `pin 1` is equivalent to
`pin "1"` — both declare a pin with designator `"1"`.

### 4.2 Component Declaration (SchLib)

```
component NAME { properties_and_children }
```

The name is the **lib_reference** — the identity key for matching.

```
component R_0603 {
    designator: "R?"
    description: "0603 Resistor"

    body = rectangle { from: (-20mil, -10mil), to: (20mil, 10mil), is_solid: true }

    pin 1 { on: $body.left, at: center, side: outside, electrical: passive, length: 25 }
    pin 2 { on: $body.right, at: center, side: outside, electrical: passive, length: 25 }

    parameter Value { text: "{VALUE}" }
    parameter MFG { text: "ACME" }

    alias R0603
    alias RES_0603

    footprint "0603" {
        map { pin: 1, pad: 1 }
        map { pin: 2, pad: 2 }
    }
}
```

**Component properties:**

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `designator` | String | required | Designator pattern (e.g., "R?", "U?") |
| `description` | String | `""` | Human-readable description |
| `component_kind` | Enum | `standard` | `standard` / `mechanical` / `graphical_std` / `graphical_mech` |
| `show_hidden_pins` | Bool | `false` | Display hidden (power) pins |

### 4.3 Multi-Part Components and the `part` Block

Multi-part components (e.g., dual op-amp, quad NAND gate) use `part` blocks to group
per-part primitives. Shared primitives (power pins, common graphics) live outside any
`part` block.

Altium stores parts as a flat list with `owner_part_id` properties. The `part` block
is syntactic sugar: during compilation, child entities inside `part N { ... }` get
`owner_part_id: N` set automatically.

```
component LM358 {
    designator: "U?"
    description: "Dual Operational Amplifier"

    part 1 {
        body = rectangle { from: (-100mil, -100mil), to: (100mil, 100mil) }
        pin 1 { name: "OUT",  on: $body.right, at: center, electrical: output }
        pin 2 { name: "IN-",  on: $body.left, at: start, gap: 30mil, electrical: input }
        pin 3 { name: "IN+",  on: $body.left, after: $pin2, gap: 60mil, electrical: input }
    }

    part 2 {
        body = rectangle { from: (-100mil, -100mil), to: (100mil, 100mil) }
        pin 5 { name: "IN+",  on: $body.left, at: start, gap: 30mil, electrical: input }
        pin 6 { name: "IN-",  on: $body.left, after: $pin5, gap: 60mil, electrical: input }
        pin 7 { name: "OUT",  on: $body.right, at: center, electrical: output }
    }

    // Shared (all parts) — owner_part_id = 0
    pin 4 { electrical: power, is_hidden: true, hidden_net_name: "GND" }
    pin 8 { electrical: power, is_hidden: true, hidden_net_name: "VCC" }

    footprint DIP8 {
        map { pin: 1, pad: 1 }
        map { pin: 2, pad: 2 }
        map { pin: 3, pad: 3 }
        map { pin: 4, pad: 4 }
        map { pin: 5, pad: 5 }
        map { pin: 6, pad: 6 }
        map { pin: 7, pad: 7 }
        map { pin: 8, pad: 8 }
    }
}
```

`part_count` is inferred from the highest `part N` block number. Explicit
`part_count: 3` on the component overrides if needed (e.g., for parts without
dedicated blocks).

### 4.4 Pin Declaration (SchLib)

```
pin NAME { properties }
```

The name is the **designator** — identity key within the parent component.

Pins support two placement modes:

**Absolute:**
```
pin 1 { at: (-30mil, 0), orientation: 0, electrical: passive, length: 25 }
```

**Anchor-based (preferred):**
```
pin 1 { on: $body.left, at: center, side: outside, electrical: passive, length: 25 }
```

**Pin properties:**

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `at` | Coord | — | Absolute position (mutually exclusive with `on`) |
| `orientation` | Enum/Int | `auto` | `0`/`90`/`180`/`270` or `auto` (inferred from anchor) |
| `electrical` | Enum | `passive` | `passive`, `input`, `output`, `io`, `open_collector`, `open_emitter`, `power`, `hi_z`, `tristate` |
| `length` | Dim | `25mil` | Pin stub length |
| `name` | String | `""` | Pin function name |
| `is_hidden` | Bool | `false` | Hidden pin |
| `hidden_net_name` | String | `""` | Implicit net for hidden pins |

**Anchor placement fields:**

| Field | Type | Description |
|-------|------|-------------|
| `on` | Anchor ref | Edge/point to place on (`$body.left`, `$body.top`, etc.) |
| `at` | Enum | Position along edge: `start`, `center`, `end` |
| `after` | Entity ref | Place after another entity on the same edge |
| `before` | Entity ref | Place before another entity |
| `gap` | Dim | Spacing (default: `100mil`) |
| `offset` | Coord | Post-placement translation |
| `side` | Enum | `inside`, `outside`, `center` |

### 4.5 Other SchLib Child Declarations

**Parameter:**
```
parameter NAME { properties }
```
Identity key: `name`. Fields: `text` (String), `is_hidden` (Bool).

**Alias:**
```
alias NAME
```
Identity key: the alias name. No body.

**Footprint map:**
```
footprint NAME { map_entries }
```
Identity key: `model_name`. Can also reference an imported footprint (§8).

```
footprint "0603" {
    map { pin: 1, pad: 1 }
    map { pin: 2, pad: 2 }
}
```

### 4.6 Anchor-Based Placement

Adopted from the ops language `place` op (ops-lang-spec.md §4.1.1). Avoids hardcoded
coordinates by specifying position relative to named anchor entities.

**Anchor references** use member access on bound graphics:

```
body = rectangle { from: (-200mil, -100mil), to: (200mil, 100mil) }

// Edge anchors
$body.top       $body.bottom       $body.left       $body.right

// Corner anchors
$body.top_left       $body.top_right
$body.bottom_left    $body.bottom_right

// Center
$body.center
```

**Geometry class anchor table:**

| Geometry class | Examples | Anchors |
|---------------|----------|---------|
| Box | rectangle, round_rectangle, text_frame, image | top, bottom, left, right, corners, center |
| Center+radius | arc, ellipse, pie | center, start_point, end_point |
| Segment | line, track | start, end, midpoint |
| Vertex-list | polyline, polygon, bezier | vertex[N], centroid |
| Point | pin, label, via, pad | location |

**Orientation `auto`** infers from anchor edge:
- `$body.left` → `0` (pin points right, connects left)
- `$body.right` → `180`
- `$body.top` → `270`
- `$body.bottom` → `90`

### 4.7 Graphics Declarations (SchLib)

Graphics are declared with an optional **binding name** that becomes the entity's
`unique_id` for reconciliation and enables anchor references.

```
body = rectangle { from: (-20mil, -10mil), to: (20mil, 10mil), is_solid: true }
pin1_stub = line { from: (-30mil, 0), to: (-20mil, 0) }
line { from: (20mil, 0), to: (30mil, 0) }   // unnamed → auto-generated ID
```

**Available graphic types:**

| Keyword | Key fields |
|---------|-----------|
| `line` | `from`, `to`, `color`, `line_width` |
| `rectangle` | `from`, `to`, `is_solid`, `color`, `area_color` |
| `arc` | `center`, `radius`, `start_angle`, `end_angle` |
| `elliptical_arc` | `center`, `radius`, `secondary_radius`, `start_angle`, `end_angle` |
| `ellipse` | `center`, `radius`, `secondary_radius`, `is_solid` |
| `polyline` | `points`, `color`, `line_width` |
| `polygon` | `points`, `is_solid`, `color`, `area_color` |
| `bezier` | `points` (4 control points) |
| `pie` | `center`, `radius`, `start_angle`, `end_angle`, `is_solid` |
| `round_rectangle` | `from`, `to`, `corner_x_radius`, `corner_y_radius` |
| `label` | `at`, `text`, `font_id`, `color` |
| `text_frame` | `from`, `to`, `text`, `is_solid`, `show_border` |
| `image` | `from`, `to`, `file_name`, `image_data` |


## 5. Footprint Declaration (PcbLib)

```
footprint NAME { properties_and_children }
```

The name is the **display_name** — the identity key.

```
let smd = { layer: "TopLayer", pad_mode: simple, is_plated: false, hole_size: 0 }

footprint SOT23 {
    description: "SOT-23 Package"
    height: 1mm

    pad 1 { ...smd, at: (-0.95mm, -1mm), shape: rectangular, x_size: 0.6mm, y_size: 0.7mm }
    pad 2 { ...smd, at: (0.95mm, -1mm), shape: rectangular, x_size: 0.6mm, y_size: 0.7mm }
    pad 3 { ...smd, at: (0, 1mm), shape: rectangular, x_size: 0.6mm, y_size: 0.7mm }

    courtyard = polyline {
        points: [(-1.5mm, -1.6mm), (1.5mm, -1.6mm), (1.5mm, 1.6mm), (-1.5mm, 1.6mm)]
        width: 5mil, layer: "TopOverlay", closed: true
    }
}
```

**Footprint properties:**

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `description` | String | `""` | Human-readable description |
| `height` | Dim | `0` | Assembly height |
| `pattern` | String | display_name | Pattern identifier |

### 5.1 Pad Declaration (PcbLib)

```
pad NAME { properties }
```

The name is the **pad_name/designator** — identity key within the footprint.

Pads support both absolute and anchor-based placement:

**Absolute (from datasheet dimensions):**
```
pad 1 { at: (-0.95mm, -1mm), shape: rectangular, x_size: 0.6mm, y_size: 0.7mm }
```

**Anchor-based (relative to other entities):**
```
pad 1 { on: $body.left, at: start, offset: (0, -0.5mm), shape: rectangular, x_size: 0.6mm, y_size: 0.7mm }
```

**Pad properties:**

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `at` | Coord | — | Absolute position (mutually exclusive with `on`) |
| `shape` | Enum | `round` | `round`, `rectangular`, `octagonal` |
| `x_size` | Dim | `60mil` | Pad width |
| `y_size` | Dim | `60mil` | Pad height |
| `rotation` | Number | `0` | Degrees |
| `hole_size` | Dim | `0` | `0`=SMD, `>0`=through-hole |
| `is_plated` | Bool | `true` | Plated through-hole vs NPTH |
| `layer` | String | `"MultiLayer"` | `"MultiLayer"` for TH, `"TopLayer"` for SMD |
| `pad_mode` | Enum | `simple` | `simple`, `top_middle_bottom` |
| `solder_mask_expansion` | Dim | — | Override mask expansion |
| `paste_mask_expansion` | Dim | — | Override paste expansion |
| `plane_connection` | Enum | — | `no_connect`, `direct_connect`, `thermal_relief` |
| `relief_conductor_width` | Dim | — | Thermal relief spoke width |
| `relief_entries` | Int | — | Number of thermal spokes |
| `relief_air_gap` | Dim | — | Thermal relief gap |

Anchor placement fields are the same as for SchLib pins (§4.4).

### 5.2 Pad Layout: Rows, Columns, and Grids

For IC packages with regular pad patterns, the spec supports `row`, `column`, and `grid`
layout blocks that generate multiple pads from a template. These are syntactic sugar —
they expand to individual `pad` declarations during compilation.

**Row** — linear sequence of pads:

```
footprint QFP32 {
    let body = rectangle { from: (-3.5mm, -3.5mm), to: (3.5mm, 3.5mm) }

    let qfp_pad = { shape: rectangular, x_size: 1.2mm, y_size: 0.3mm }

    row {
        on: $body.left, at: center       // anchor to left edge
        pitch: 0.5mm                      // center-to-center spacing
        count: 8                          // number of pads
        start: 1                          // first pad name (auto-increments: 1, 2, 3, ...)
        side: outside                     // pads extend outward from body
        pad: { ...qfp_pad }              // template for each pad
    }

    row {
        on: $body.bottom, at: center
        pitch: 0.5mm, count: 8, start: 9
        side: outside
        pad: { ...qfp_pad, rotation: 90 }
    }

    row {
        on: $body.right, at: center
        pitch: 0.5mm, count: 8, start: 17
        side: outside, direction: reverse // bottom-to-top instead of top-to-bottom
        pad: { ...qfp_pad }
    }

    row {
        on: $body.top, at: center
        pitch: 0.5mm, count: 8, start: 25
        side: outside, direction: reverse
        pad: { ...qfp_pad, rotation: 90 }
    }
}
```

**Row properties:**

| Field | Type | Description |
|-------|------|-------------|
| `on` | Anchor ref | Edge to place along |
| `at` | Enum | `start`, `center`, `end` — where first pad goes |
| `pitch` | Dim | Center-to-center spacing |
| `count` | Int | Number of pads |
| `start` | Int/String | First pad name (auto-increments) |
| `direction` | Enum | `forward` (default) or `reverse` |
| `side` | Enum | `inside`, `outside`, `center` |
| `pad` | Object | Template properties applied to each pad |
| `skip` | Array | Pad indices to skip (for irregular packages) |

**Column** — alias for `row` with vertical default. Identical semantics.

**Grid** — 2D array of pads (for BGA, LGA, QFN exposed pads):

```
footprint BGA256 {
    description: "256-ball BGA, 1mm pitch, 16x16"
    height: 1.2mm

    grid {
        origin: (0, 0)                   // center of grid
        rows: 16, cols: 16
        pitch: 1mm                        // uniform x and y pitch
        naming: alphanumeric              // A1, A2, ..., B1, B2, ..., P16
        pad: { shape: round, x_size: 0.4mm, y_size: 0.4mm }
        skip: [H8, H9, J8, J9]          // omit pads in thermal area
    }

    // Thermal/exposed pad
    pad EP { at: (0, 0), shape: rectangular, x_size: 4mm, y_size: 4mm }
}
```

**Grid properties:**

| Field | Type | Description |
|-------|------|-------------|
| `origin` | Coord | Center of grid |
| `rows` | Int | Number of rows |
| `cols` | Int | Number of columns |
| `pitch` | Dim | Uniform pitch (shorthand for `pitch_x` + `pitch_y`) |
| `pitch_x` | Dim | Column-to-column spacing |
| `pitch_y` | Dim | Row-to-row spacing |
| `naming` | Enum | `numeric` (1,2,...), `alphanumeric` (A1,A2,...,B1,...) |
| `pad` | Object | Template for each pad |
| `skip` | Array | Pad names to omit |
| `perimeter_only` | Bool | Only place pads on outer ring (default `false`) |

**DIP / SIP with rows:**

```
footprint DIP8 {
    description: "8-pin DIP, 300mil row spacing"
    height: 3.3mm

    let th = { shape: round, x_size: 60mil, y_size: 60mil, hole_size: 40mil, is_plated: true, layer: "MultiLayer" }

    row {
        at: (-150mil, 150mil)            // absolute start position
        pitch: 100mil, count: 4, start: 1
        direction: down                   // top to bottom (negative Y)
        pad: { ...th }
    }

    row {
        at: (150mil, -150mil)
        pitch: 100mil, count: 4, start: 5
        direction: up                     // bottom to top (positive Y)
        pad: { ...th }
    }

    // Pin 1 is rectangular (convention)
    pad 1 { shape: rectangular }         // override just the shape, keeps other props from row
}
```

### 5.3 PCB Graphics

Same binding-name-as-identity pattern as SchLib. Identity stored in the
`UniqueIDPrimitiveInformation` sidecar stream.

| Keyword | Key fields |
|---------|-----------|
| `track` | `start`, `end`, `width`, `layer` |
| `arc` | `center`, `radius`, `start_angle`, `end_angle`, `width`, `layer` |
| `fill` | `corner1`, `corner2`, `rotation`, `layer` |
| `region` | `outline`, `holes`, `kind`, `layer` |
| `text` | `at`, `text`, `height`, `rotation`, `layer`, `font` |


## 6. Import System

Spec files can import other spec files to:
- Link components to footprints defined in separate files
- Split large libraries across multiple files
- Reuse shared templates and constants

### 6.1 Named Import (Namespace)

```
import "standard-footprints.pcblib-spec" as footprints
```

Brings the imported file's declarations under a namespace. Access via `$footprints.NAME`
or `$footprints["Name With Spaces"]`:

```
import "standard-footprints.pcblib-spec" as footprints

component MCU32 {
    designator: "U?"
    // ...

    // Reference imported footprint by . access
    footprint $footprints.QFP32 {
        map { pin: 1, pad: 1 }
        // ...
    }

    // Reference with bracket access (for names with special chars)
    footprint $footprints["SOT-23"] {
        map { pin: 1, pad: 1 }
        map { pin: 2, pad: 2 }
        map { pin: 3, pad: 3 }
    }
}
```

### 6.2 Bare Import (Merge)

```
import "common-passives.schlib-spec"
```

Merges the imported file's declarations into the current file's scope. Component and
footprint declarations from the imported file are treated as if written inline.

This enables splitting a library into multiple source files:

```
// all-parts.schlib-spec (the main spec)
import "passives.schlib-spec"
import "connectors.schlib-spec"
import "ics.schlib-spec"
```

```bash
altium apply all-parts.schlib-spec    # creates/updates all-parts.SchLib
```

### 6.3 Import Semantics

- Imports are resolved relative to the importing file's directory
- Circular imports are an error
- Let bindings from imported files are NOT merged into the importing scope
  (only entity declarations are merged). Use named import for templates.
- `.schlib-spec` files can import other `.schlib-spec` files (merge) or
  `.pcblib-spec` files (namespace, for footprint references)
- `.pcblib-spec` files can import other `.pcblib-spec` files (merge)

### 6.4 Footprint Linking via Import

When a component references a footprint from an imported pcblib-spec, the system
validates that the footprint exists and that the pin-to-pad map is consistent:

```
import "footprints.pcblib-spec" as fp

component R {
    pin 1 { ... }
    pin 2 { ... }

    // Links to footprint "0603" defined in footprints.pcblib-spec
    footprint $fp.R0603 {
        map { pin: 1, pad: 1 }
        map { pin: 2, pad: 2 }
    }

    // Can have multiple footprint mappings
    footprint $fp.R0805 {
        map { pin: 1, pad: 1 }
        map { pin: 2, pad: 2 }
    }
}
```

When applying, the tool validates that:
1. The referenced footprint exists in the pcblib-spec
2. All mapped pads exist in the footprint definition
3. All mapped pins exist in the component


## 7. Expression Language & Type System

**Identical to the Ops Language** (ops-lang-spec.md §5, §7). All expression features are
shared:

- Value literals: strings, numbers, dims, colors, booleans, null
- Operators: `+`, `-`, `*`, `/`, `.`, `[expr]`
- Coords: `(x, y)` tuples
- Arrays: `[expr, ...]`
- Objects: `{ key: value, ... }` with spread (`...expr`) and block-scoped bindings
- Template strings: `` `text {expr}` ``
- Path expressions: `$ref.field`, `$ref[index]`
- Dimensional scalars: `10mil`, `5mm`, `0.5in`, `100dxp`, `50raw`
- Enum resolution: case-insensitive, underscore-insensitive bare identifiers

### 7.1 Intra-Spec References

Bound entities can be referenced within their scope:

```
component R_0603 {
    body = rectangle { from: (-20mil, -10mil), to: (20mil, 10mil) }
    pin 1 { on: $body.left, at: center, side: outside }  // $body in scope
}
// $body is NOT visible outside the component block
```

### 7.2 Cross-Entity References

Within a component, bound pins can be referenced by later pins (for `after`/`before`):

```
component LM358 {
    part 1 {
        body = rectangle { ... }
        p2 = pin 2 { on: $body.left, at: start, gap: 30mil }
        pin 3 { on: $body.left, after: $p2, gap: 60mil }  // references $p2
    }
}
```


## 8. Identity Key Strategy

| Entity | Document | Identity Key | Source |
|--------|----------|-------------|--------|
| Component | SchLib | `lib_reference` | Name after `component` |
| Pin | SchLib | `designator` (scoped) | Name after `pin` |
| Parameter | SchLib | `name` (scoped) | Name after `parameter` |
| Alias | SchLib | `alias_name` (scoped) | Name after `alias` |
| Graphic | SchLib | `unique_id` on record | Binding name → `spec:{component}:{name}` |
| Footprint | PcbLib | `display_name` | Name after `footprint` |
| Pad | PcbLib | `pad_name` (scoped) | Name after `pad` |
| PCB graphic | PcbLib | unique_id via sidecar | Binding name → `spec:{footprint}:{name}` |

**Unnamed graphics** get auto-generated IDs: `spec:R_0603:rectangle_0`, etc.

**Uniqueness constraints** (enforced at parse time):
- No duplicate identity keys within their scope
- No duplicate binding names within a scope


## 9. Merge Semantics: Ensure (Additive)

- Entities **in the spec** are added (if missing) or updated (if different)
- Entities **in the document but not in the spec** are **left untouched**
- **No deletions** — the spec is a subset assertion
- Future: `purge` modifier for full convergence

This means hand-crafted components can coexist with spec-managed components.


## 10. Engineering Change Order (ECO)

The `altium plan` command generates a full ECO suitable for hardware development review.

### 10.1 ECO Text Format

```
╔══════════════════════════════════════════════════════════════════════╗
║  ENGINEERING CHANGE ORDER                                          ║
║  Library: my-parts.SchLib                                          ║
║  Spec:    my-parts.schlib-spec                                     ║
║  Date:    2026-02-26 14:30:00 UTC                                  ║
╚══════════════════════════════════════════════════════════════════════╝

SUMMARY
  Components:  2 add, 1 update, 15 unchanged
  Pins:        8 add, 3 update, 42 unchanged
  Parameters:  4 add, 1 update, 30 unchanged
  Graphics:    6 add, 0 update, 45 unchanged

CHANGES

  + ADD component "R_0603_NEW"
    │ designator: "R?"
    │ description: "New 0603 resistor variant"
    ├── + pin "1" at (-30mil, 0) electrical=passive
    ├── + pin "2" at (30mil, 0) electrical=passive
    ├── + parameter "MFG" text="ACME"
    ├── + rectangle "body" (-20mil,-10mil)–(20mil,10mil) solid
    └── + footprint "0603" [2 pin-pad maps]

  ~ UPDATE component "R_0805"
    │ ~ description: "0805 Resistor" → "0805 Resistor (updated)"
    ├── + pin "3" at (0, 50mil) electrical=passive  [NEW]
    ├── = pin "1" (unchanged)
    ├── = pin "2" (unchanged)
    └── ~ parameter "MFG": text "ACME" → "ACME Inc."

  = 15 components unchanged (not shown)

END OF ECO
```

### 10.2 ECO JSON Format

With `--json`, the plan is output as structured JSON for machine consumption.
Includes all fields for each change, before/after values, and summary statistics.

### 10.3 ECO Data Structure

```rust
struct EngineeringChangeOrder {
    library_path: PathBuf,
    spec_path: PathBuf,
    timestamp: DateTime<Utc>,
    summary: EcoSummary,
    changes: Vec<EntityChange>,
}

struct EcoSummary {
    by_kind: IndexMap<String, KindSummary>,  // "component", "pin", etc.
}

struct KindSummary {
    adds: usize,
    updates: usize,
    unchanged: usize,
}

enum EntityChange {
    Add { kind: String, identity: String, details: String, children: Vec<EntityChange> },
    Update { kind: String, identity: String, prop_changes: Vec<PropChange>, children: Vec<EntityChange> },
    Unchanged { kind: String, identity: String },
}

struct PropChange {
    field: String,
    old_value: String,
    new_value: String,
}
```


## 11. Spec Dump (Reverse Generation)

`altium dump` reads an existing library file and generates a spec file, enabling:
- Bootstrapping spec-based management of existing libraries
- Inspecting library contents in a human-readable format
- Creating a baseline for version-controlled library specs

```bash
altium dump my-parts.SchLib                    # → my-parts.schlib-spec
altium dump my-parts.PcbLib --output fp.pcblib-spec
```

The generated spec includes:
- All components/footprints with their full property set
- Pins with absolute placement (not anchor-based — reverse inference of anchors is complex)
- All parameters, aliases, graphics
- Let-binding extraction for repeated patterns (optional, with `--extract-templates`)

The dump output is a valid spec file that, when applied to an empty document,
recreates the original library (modulo serialization ordering).


## 12. Edit Operations Required for Reconciliation

The reconciler needs to UPDATE existing entities, not just add new ones. This requires
new low-level edit operations beyond what the ops DSL currently supports.

### 12.1 Missing Operations (Implementation Required)

**SchLib:**

| Operation | Description |
|-----------|-------------|
| `EditPin` | Change position, orientation, length, electrical type, name, hidden state |
| `EditGraphic` | Change position, dimensions, colors, line width for any graphic type |
| `EditAlias` | Rename an alias |
| `EditFootprintMap` | Update pin-to-pad mapping |

**PcbLib:**

| Operation | Description |
|-----------|-------------|
| `AddPad` | Create new pad with full properties |
| `EditPad` | Change position, shape, size, hole, rotation, masks, thermal relief |
| `EditTrack` | Change start, end, width, layer |
| `EditVia` | Change position, diameter, hole size, layers |
| `EditFootprint` | Change description, height, pattern |
| `DeletePad` | Remove a pad |
| `DeleteTrack` | Remove a track |
| `DeleteVia` | Remove a via |

### 12.2 Reconciler Strategy

For entities where a targeted edit op exists, the reconciler emits an edit.
For entities where only add/delete exists, the reconciler uses **delete + re-add**
as a fallback (preserving the identity key).


## 13. Formal Grammar

```ebnf
(* ================================================================ *)
(* File structure                                                    *)
(* ================================================================ *)

spec_file       = { spec_item [";"] } ;

spec_item       = import_decl
                | let_binding
                | component_decl        (* SchLib *)
                | footprint_decl ;      (* PcbLib *)

import_decl     = "import" STRING [ "as" IDENT ] ;
let_binding     = ["let"] IDENT "=" expr ;

(* ================================================================ *)
(* Entity names (quoted or unquoted)                                 *)
(* ================================================================ *)

entity_name     = STRING | IDENT | INTEGER ;

(* ================================================================ *)
(* SchLib declarations                                               *)
(* ================================================================ *)

component_decl  = "component" entity_name "{" { component_item [sep] } "}" ;

component_item  = let_binding
                | property
                | part_block
                | pin_decl
                | parameter_decl
                | alias_decl
                | footprint_map_decl
                | graphic_decl ;

part_block      = "part" INTEGER "{" { part_item [sep] } "}" ;
part_item       = let_binding | pin_decl | graphic_decl ;

pin_decl        = "pin" entity_name object ;
parameter_decl  = "parameter" entity_name object ;
alias_decl      = "alias" entity_name ;
footprint_map_decl = "footprint" ( entity_name | "$" path_expr ) "{" { map_entry [sep] } "}" ;
map_entry       = "map" object ;

graphic_decl    = [ IDENT "=" ] GRAPHIC_TYPE object ;

GRAPHIC_TYPE    = "line" | "rectangle" | "arc" | "elliptical_arc" | "ellipse"
                | "polyline" | "polygon" | "bezier" | "pie"
                | "round_rectangle" | "label" | "text_frame" | "image" ;

(* ================================================================ *)
(* PcbLib declarations                                               *)
(* ================================================================ *)

footprint_decl  = "footprint" entity_name "{" { footprint_item [sep] } "}" ;

footprint_item  = let_binding
                | property
                | pad_decl
                | row_decl
                | grid_decl
                | pcb_graphic_decl ;

pad_decl        = "pad" entity_name object ;

row_decl        = ("row" | "column") object ;
grid_decl       = "grid" object ;

pcb_graphic_decl = [ IDENT "=" ] PCB_GRAPHIC_TYPE object ;
PCB_GRAPHIC_TYPE = "track" | "arc" | "fill" | "region" | "text" ;

(* ================================================================ *)
(* Shared productions                                                *)
(* ================================================================ *)

property        = IDENT ":" expr ;
object          = "{" [ object_body ] "}" ;
object_body     = object_item { sep object_item } ;
object_item     = let_binding | spread | property ;
spread          = "..." expr ;

(* Expression — reuses ops-lang-spec.md §10 expression grammar *)
expr            = (* see ops-lang-spec.md §10 *) ;
path_expr       = (* see ops-lang-spec.md §5.3 *) ;

sep             = "," | NEWLINE ;
```


## 14. Complete Examples

### Example 1: Basic Passive Library

```
// passives.schlib-spec
let passive_pin = { electrical: passive, length: 25, side: outside }
let two_pin_body = { from: (-20mil, -10mil), to: (20mil, 10mil), is_solid: true }

component R {
    designator: "R?"
    description: "Resistor"
    body = rectangle { ...two_pin_body }
    pin 1 { ...passive_pin, on: $body.left, at: center }
    pin 2 { ...passive_pin, on: $body.right, at: center }
    parameter Value { text: "{VALUE}" }
    footprint R0805 { map { pin: 1, pad: 1 }, map { pin: 2, pad: 2 } }
}

component C {
    designator: "C?"
    description: "Capacitor"
    body = rectangle { ...two_pin_body }
    pin 1 { ...passive_pin, on: $body.left, at: center }
    pin 2 { ...passive_pin, on: $body.right, at: center }
    parameter Value { text: "{VALUE}" }
    footprint C0805 { map { pin: 1, pad: 1 }, map { pin: 2, pad: 2 } }
}

component L {
    designator: "L?"
    description: "Inductor"
    body = rectangle { ...two_pin_body }
    pin 1 { ...passive_pin, on: $body.left, at: center }
    pin 2 { ...passive_pin, on: $body.right, at: center }
    parameter Value { text: "{VALUE}" }
    footprint L0805 { map { pin: 1, pad: 1 }, map { pin: 2, pad: 2 } }
}
```

### Example 2: QFP with Row Placement

```
// qfp.pcblib-spec
let qfp_pad = { shape: rectangular, x_size: 1.5mm, y_size: 0.3mm, layer: "TopLayer", hole_size: 0 }

footprint QFP32 {
    description: "32-pin QFP, 0.8mm pitch, 7x7mm body"
    height: 1.2mm

    let body = rectangle { from: (-3.5mm, -3.5mm), to: (3.5mm, 3.5mm) }

    row { on: $body.left, at: center, pitch: 0.8mm, count: 8, start: 1, side: outside, pad: { ...qfp_pad } }
    row { on: $body.bottom, at: center, pitch: 0.8mm, count: 8, start: 9, side: outside, pad: { ...qfp_pad, rotation: 90 } }
    row { on: $body.right, at: center, pitch: 0.8mm, count: 8, start: 17, side: outside, direction: reverse, pad: { ...qfp_pad } }
    row { on: $body.top, at: center, pitch: 0.8mm, count: 8, start: 25, side: outside, direction: reverse, pad: { ...qfp_pad, rotation: 90 } }

    outline = polyline {
        points: [(-4mm, -4mm), (4mm, -4mm), (4mm, 4mm), (-4mm, 4mm)]
        width: 10mil, layer: "TopOverlay", closed: true
    }

    pin1_mark = arc { center: (-4mm, 3.5mm), radius: 0.3mm, start_angle: 0, end_angle: 360, width: 10mil, layer: "TopOverlay" }
}
```

### Example 3: BGA with Grid

```
// bga.pcblib-spec
footprint BGA256 {
    description: "256-ball BGA, 1mm pitch"
    height: 1.5mm

    grid {
        origin: (0, 0)
        rows: 16, cols: 16
        pitch: 1mm
        naming: alphanumeric
        pad: { shape: round, x_size: 0.4mm, y_size: 0.4mm, layer: "TopLayer", hole_size: 0 }
        skip: [H8, H9, J8, J9]
    }

    pad EP { at: (0, 0), shape: rectangular, x_size: 5mm, y_size: 5mm, layer: "TopLayer" }
}
```

### Example 4: Multi-Part IC with Import

```
// ics.schlib-spec
import "standard-footprints.pcblib-spec" as fp

let input_pin = { electrical: input, length: 25, side: outside }
let output_pin = { electrical: output, length: 25, side: outside }
let power_pin = { electrical: power, length: 25, is_hidden: true }

component LM358 {
    designator: "U?"
    description: "Dual Operational Amplifier"

    part 1 {
        body = rectangle { from: (-100mil, -100mil), to: (100mil, 100mil) }
        pin 1 { name: "OUT",  ...output_pin, on: $body.right, at: center }
        pin 2 { name: "IN-",  ...input_pin, on: $body.left, at: start, gap: 30mil }
        pin 3 { name: "IN+",  ...input_pin, on: $body.left, after: $pin2, gap: 60mil }
    }

    part 2 {
        body = rectangle { from: (-100mil, -100mil), to: (100mil, 100mil) }
        pin 5 { name: "IN+",  ...input_pin, on: $body.left, at: start, gap: 30mil }
        pin 6 { name: "IN-",  ...input_pin, on: $body.left, after: $pin5, gap: 60mil }
        pin 7 { name: "OUT",  ...output_pin, on: $body.right, at: center }
    }

    pin 4 { ...power_pin, hidden_net_name: "GND" }
    pin 8 { ...power_pin, hidden_net_name: "VCC" }

    parameter MFG { text: "Texas Instruments" }
    alias LM358N
    alias LM358P

    footprint $fp.DIP8 {
        map { pin: 1, pad: 1 }
        map { pin: 2, pad: 2 }
        map { pin: 3, pad: 3 }
        map { pin: 4, pad: 4 }
        map { pin: 5, pad: 5 }
        map { pin: 6, pad: 6 }
        map { pin: 7, pad: 7 }
        map { pin: 8, pad: 8 }
    }
}
```

### Example 5: Composable Library Files

```
// my-library.schlib-spec (main entry point)
import "passives.schlib-spec"
import "connectors.schlib-spec"
import "ics.schlib-spec"
```

```bash
altium apply my-library.schlib-spec  # creates/updates my-library.SchLib with all components
```


## 15. Scope Boundaries

### What We Build

- Declarative entity blocks with unquoted/quoted names
- `part` blocks for multi-part components
- Anchor-based relative placement for pins and pads
- `row`/`column`/`grid` layout primitives for regular pad patterns
- Graphics with binding-name identity
- Import system for file composition and footprint linking
- ECO-grade plan output for hardware review
- `altium dump` for reverse-generating specs from existing files
- Identity-key-based reconciliation (additive/ensure semantics)

### What We Don't Build (Initially)

- **No imperative verbs.** No `add_*`, `edit`, `remove`, `query`.
- **No selectors.** Specs don't query — they declare.
- **No SchDoc/PcbDoc support.** Placed instances have harder identity problems.
- **No purge/delete semantics.** Additive only.
- **No control flow.** No if/else, no loops.
- **No anchor inference in dump.** Dump generates absolute coordinates.

### Open Questions

1. **SchPie missing `unique_id`**: Needs verification, may need field added.
2. **PcbLib sidecar write**: Confirm arbitrary `unique_id` writes to UniqueIDPrimitiveInformation.
3. **Coordinate tolerance**: Reconciler may need ±1 internal unit tolerance for float rounding.
4. **Row/grid pad override**: When a `row` generates pad "1" and an explicit `pad 1 { ... }`
   also exists, the explicit declaration should win (override specific properties).
5. **Import cycle detection**: Need topological sort for import resolution.
