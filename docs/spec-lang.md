# Altium Spec Language Specification

Version: 0.3
File extensions: `.schlib-spec`, `.pcblib-spec`

## 1. Overview

The Altium Spec Language is a declarative DSL for describing the **desired state** of Altium
Designer library files (SchLib, PcbLib). Instead of issuing mutation commands, a spec says
"the document should look like this."

The CLI reads the spec, diffs it against the current document, produces a plan of minimal
changes (an engineering change order), and applies them idempotently. Running the same spec
file twice is a no-op.


## 2. Design Goals

1. **Declarative.** Describe what, not how. No mutation verbs.
2. **Idempotent.** Applying the same spec to the same document is always a no-op.
3. **Additive by default.** Entities in the document but NOT in the spec are untouched.
   The spec is a subset assertion, not a complete truth.
4. **Token-minimal.** Natural keys are positional (the identifier/string after the
   entity keyword). Quotes optional for simple names. Every value position is an expression.
5. **Placement-aware.** Anchor-based relative placement for both SchLib pins and PcbLib
   pads. Row/column/grid layout primitives for footprint pad patterns.
6. **ECO-grade output.** The plan command generates a full engineering change order,
   suitable for real-world hardware development review processes.
7. **Agent-friendly.** Expression language with structured error messages, source spans,
   and schema introspection.
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
| `component_kind` | Enum | `standard` | `standard`, `mechanical`, `graphical`, `net_tie_bom`, `net_tie_no_bom`, `standard_no_bom`, `jumper` |
| `part_count` | Int | (inferred) | Override inferred part count from highest `part N` block number |
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
        p2 = pin 2 { name: "IN-",  on: $body.left, at: start, gap: 30mil, electrical: input }
        pin 3 { name: "IN+",  on: $body.left, after: $p2, gap: 60mil, electrical: input }
    }

    part 2 {
        body = rectangle { from: (-100mil, -100mil), to: (100mil, 100mil) }
        p5 = pin 5 { name: "IN+",  on: $body.left, at: start, gap: 30mil, electrical: input }
        pin 6 { name: "IN-",  on: $body.left, after: $p5, gap: 60mil, electrical: input }
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
| `at` | Coord \| Enum | — | **Absolute mode** (no `on:`): Coord position. **Anchor mode** (with `on:`): `start`, `center`, `end` position along edge. |
| `orientation` | Enum/Int | `auto` | `0`/`90`/`180`/`270` or `auto` (inferred from anchor) |
| `electrical` | Enum | `passive` | `input`, `input_output` (alias: `io`), `output`, `open_collector`, `open_emitter`, `passive`, `hi_z`, `power` |
| `length` | Dim | `25mil` | Pin stub length |
| `name` | String | `""` | Pin function name |
| `is_hidden` | Bool | `false` | Hidden pin |
| `hidden_net_name` | String | `""` | Implicit net for hidden pins |

**Anchor placement fields:**

| Field | Type | Description |
|-------|------|-------------|
| `on` | Anchor ref | Edge to place on (`$body.left`, `$body.top`, etc.) |
| `after` | Entity ref | Place after another entity on the same edge |
| `before` | Entity ref | Place before another entity |
| `gap` | Dim | Spacing (default: `100mil`) |
| `offset` | Coord | Post-placement translation |
| `side` | Enum | `inside`, `outside`, `center` |

**Placement mode constraints:**

Placement modes are mutually exclusive:
- **Absolute**: `at: (x, y)` — position and `orientation` are explicit.
- **Anchor-based**: `on:` + `at: start|center|end` — position and orientation
  are computed from the anchor.

If `on:` is present and `at:` is a Coord, this is an error.

The fields `at` (enum), `after`, and `before` are mutually exclusive in anchor
mode. Specifying more than one is an error.

The entity referenced by `after:` or `before:` must be on the **same anchor
edge** as the current entity's `on:`. If the referenced entity is on a
different edge or uses absolute placement, this is an error:

```
error[E_CROSS_EDGE_REFERENCE]: pin '$p2' (on $body.right) is not on
  the same edge as pin '3' (on $body.left)
```

```
error[E_RELATIVE_TO_ABSOLUTE]: cannot use 'after' with
  absolutely-placed pin '$p1' (pin '1' uses 'at: (-30mil, 0)')
```

### 4.5 Other SchLib Child Declarations

**Parameter:**
```
parameter NAME { properties }
```
Identity key: `name`. Fields: `text` (String), `is_hidden` (Bool).

The `text` property is a plain string value. Altium uses `{PARAM_NAME}` syntax
within parameter text for dynamic substitution at schematic placement time.
This is a literal string in the spec — no spec-level interpolation occurs.
For spec-level interpolation, use a template string:
`` text: `prefix {$some_expr}` ``.

**Alias:**
```
alias NAME
```
Identity key: the alias name. No body.

**Footprint map:**
```
footprint NAME { map_entries }
```
Identity key: `model_name`. Can also reference an imported footprint (§6).

```
footprint "0603" {
    map { pin: 1, pad: 1 }
    map { pin: 2, pad: 2 }
}
```

### 4.6 Anchor-Based Placement

Avoids hardcoded coordinates by specifying position relative to named anchor entities.

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

Corner anchors (`$body.top_left`, `$body.top_right`, `$body.bottom_left`,
`$body.bottom_right`) are valid as coordinate references in expressions (e.g.,
`from: $body.top_left`) but **cannot be used with `on:` for pin or pad
placement**. Corners are points, not edges — `at: start|center|end` and
`after:`/`before:` sequencing are undefined on a point. Using a corner anchor
with `on:` is an error.

**Orientation `auto`** infers from anchor edge (edge anchors only):
- `$body.left` → `0` (pin points right, connects left)
- `$body.right` → `180`
- `$body.top` → `270`
- `$body.bottom` → `90`

### 4.7 Graphics Declarations (SchLib)

Graphics are declared with an optional **binding name** that becomes the entity's
`unique_id` for reconciliation and enables anchor references. The optional
binding name (before `=`) makes the entity available as `$name` within its
scope for anchor references and `after:`/`before:` placement.

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
| `at` | Coord \| Enum | — | **Absolute mode** (no `on:`): Coord position. **Anchor mode** (with `on:`): `start`, `center`, `end` position along edge. |
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

Anchor placement fields and placement mode constraints (mutual exclusivity,
cross-edge errors) are the same as for SchLib pins (§4.4).

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
| `at` | Coord \| Enum | **With `on:`**: `start`, `center`, `end` — where the row starts along the anchor edge. **Without `on:`**: Coord — absolute start position of the first pad. |
| `pitch` | Dim | Center-to-center spacing |
| `count` | Int | Number of pads |
| `start` | Int | First pad name (auto-increments: `start`, `start+1`, ...) |
| `direction` | Enum | Pad ordering direction: `forward` (default), `reverse`, `up`, `down`, `left`, `right` |
| `side` | Enum | `inside`, `outside`, `center` |
| `pad` | Object | Template properties applied to each pad |
| `skip` | Array | Pad names to skip (for irregular packages) |

**Per-edge `forward` direction:**

| Anchor edge | `forward` direction | `reverse` direction |
|-------------|---------------------|---------------------|
| `$body.left` | Top-to-bottom (decreasing Y) | Bottom-to-top |
| `$body.right` | Bottom-to-top (increasing Y) | Top-to-bottom |
| `$body.top` | Left-to-right (increasing X) | Right-to-left |
| `$body.bottom` | Right-to-left (decreasing X) | Left-to-right |

`up`, `down`, `left`, `right` are only valid for absolute-positioned rows
(using `at: Coord` without `on:`). Using `up`/`down`/`left`/`right` with
anchor-based `on:` is an error.

**Column** — syntactically identical to `row` with the same semantics. No
separate example is provided.

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

**Override semantics**: When a `row`, `column`, or `grid` generates a pad with
name N, and an explicit `pad N { ... }` also exists in the same footprint, the
explicit declaration is a **field-level override**:

- Fields specified in the explicit `pad N` take precedence over the row/grid
  template for that pad.
- Fields NOT specified in the explicit `pad N` inherit from the row/grid
  template.
- The position computed by the layout algorithm is overridable via explicit
  `at:` (though this breaks the geometric pattern).
- This is NOT a duplicate identity key error.
- Evaluation order is irrelevant — the merge is declarative, not sequential.

**Skip semantics**: `skip` values are matched against generated pad names as
strings. Unquoted identifiers and integers are treated as their string
equivalents: `skip: [1, 2]` is equivalent to `skip: ["1", "2"]`.

For rows: `skip` references pad names after `start` numbering, not positional
indices. In a row with `start: 5`, `skip: [6, 8]` skips the pads named "6"
and "8".

Skip entries that don't match any generated pad name are a warning (not an
error).

### 5.3 PCB Graphics

Same binding-name-as-identity pattern as SchLib. Identity stored in the
`UniqueIDPrimitiveInformation` sidecar stream. The optional binding name
(before `=`) makes the entity available as `$name` within its scope for
anchor references and `after:`/`before:` placement.

| Keyword | Key fields |
|---------|-----------|
| `track` | `start`, `end`, `width`, `layer` |
| `arc` | `center`, `radius`, `start_angle`, `end_angle`, `width`, `layer` |
| `fill` | `corner1`, `corner2`, `rotation`, `layer` |
| `region` | `outline`, `holes`, `kind`, `layer` |
| `text` | `at`, `text`, `height`, `rotation`, `layer`, `font` |
| `via` | `at`, `diameter`, `hole_size`, `start_layer`, `end_layer` |
| `component_body` | `model_name`, `standoff_height`, `overall_height`, `body_opacity` |
| `line` | `from`, `to`, `width`, `layer` |
| `polyline` | `points`, `width`, `layer`, `closed` *(lowers to tracks or region)* |

Notes:
- `via` — a via primitive placed in the footprint (via-in-pad, test points)
- `component_body` — 3D body definition for DRC height checks
- `line` — single line segment (same semantics as `track` but using SchLib naming)
- `polyline` — spec-level sugar that lowers to multiple `track` primitives or a
  `region` (not a native PCB primitive type)


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
- Circular imports are an error (see below)
- Let bindings from imported files are NOT merged into the importing scope
  (only entity declarations are merged). Use named import for templates.

**Cross-domain import rules:**
- `.schlib-spec` can import `.schlib-spec` (bare or named)
- `.schlib-spec` can import `.pcblib-spec` (named only — for footprint refs)
- `.pcblib-spec` can import `.pcblib-spec` (bare or named)
- `.pcblib-spec` **cannot** import `.schlib-spec` (error)

**Bare import collision**: If two bare imports define entities with the same
identity key, this is a **hard error at plan/typecheck time** (not at parse
time, since identity keys are only known after all imports are resolved):

```
error[E_DUPLICATE_ENTITY]: component 'R' defined in both
  'passives.schlib-spec' (line 12) and 'connectors.schlib-spec' (line 8)
```

There is no last-wins or first-wins behavior. Resolve by renaming the
conflicting entity or using named imports instead of bare imports.

**Named import alias uniqueness**: Import aliases must be unique within a file.
Two imports with the same alias is a parse-time error:

```
error[E_DUPLICATE_IMPORT_ALIAS]: import alias 'fp' already defined
  --> my-parts.schlib-spec:2:1
  |
  1 | import "footprints-a.pcblib-spec" as fp
  2 | import "footprints-b.pcblib-spec" as fp
  |                                        ^^ duplicate alias
```

**Import cycle detection**: Imports are resolved using topological sort. A
cycle is detected during import resolution:

```
error[E_CIRCULAR_IMPORT]: circular import detected
  a.schlib-spec → b.schlib-spec → a.schlib-spec
```

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

**Footprint validation**: When a component references a footprint from an
imported pcblib-spec (`footprint $fp.DIP8 { ... }`), validation is against the
spec definition — the referenced `.pcblib-spec` file must be importable and
must define the named footprint. The referenced PcbLib file does NOT need to
exist on disk (validation is spec-to-spec, not spec-to-binary).

When applying, the tool validates that:
1. The referenced footprint exists in the pcblib-spec
2. All mapped pads exist in the footprint definition
3. All mapped pins exist in the component
4. No pad is mapped more than once (error: `E_DUPLICATE_MAP`)
5. Unmapped pads in the footprint are allowed (thermal pads, mounting holes,
   fiducials) — an informational note is emitted


## 7. Expression Language

**Every value position is an expression.** There is no special prefix or delimiter to mark
expressions — the Pratt parser runs on every value.

Literals (strings, numbers, booleans) are trivial expressions. References, arithmetic, and
dimensional values compose into compound expressions.

### 7.1 Value Literals

| Syntax | Type | Examples |
|--------|------|---------|
| `"..."` | String | `"R1"`, `"10K"`, `"0805"` |
| `` `...` `` | Template string | `` `expected {$body.width}` `` |
| `42`, `-5` | Integer | Bare digits, optional sign |
| `3.14`, `-0.5` | Float | Has decimal point |
| `20mm`, `100mil` | Dim (scalar with units) | Number immediately followed by unit suffix |
| `true` / `false` | Bool | |
| `null` | Null | |
| `#FF0000` | Color | `#` + exactly 6 hex digits |

**Strings** are always quoted. Escape sequences: `\"`, `\\`, `\n`, `\t`, `\r`.
`{` and `}` have no special meaning inside regular `"..."` strings.

**Template strings** use backticks (`` ` ``). They support `{expr}` interpolation
where `{expr}` is replaced by the evaluated expression value at runtime. Literal
`{` and `}` inside template strings are escaped as `{{` and `}}`. Escape sequences:
`` \` ``, `\\`, `\n`, `\t`, `\r`, `\{`, `\}`. Template strings evaluate to `String`
at runtime. Template strings are forbidden as entity names (entity names must be static).

**Integers and floats** are plain numbers without unit suffixes. In a field that
expects a dimensional value, a bare number defaults to mils. The type checker
handles this, not the parser.

**Dimensional scalars** ("dims") are numbers with a unit suffix. The suffix is lexed
as part of the token — `20mm` is one token, not `20` followed by identifier `mm`.

**Colors** start with `#` followed by exactly 6 hex digits. Named colors (`red`,
`blue`, etc.) are bare identifiers resolved by the enum registry when the field
type is `Color`.

### 7.2 Operators & Precedence

The Pratt parser handles expression parsing with these binding powers:

| Precedence | Operators | Associativity | Description |
|------------|-----------|---------------|-------------|
| 90 | `.` `[expr]` | left | Field access, index |
| 70 | unary `-` | prefix | Negation |
| 60 | `*` `/` | left | Multiply, divide |
| 50 | `+` `-` | left | Add, subtract |

**Type rules for arithmetic:**

| Left | Op | Right | Result |
|------|----|-------|--------|
| dim | `+` `-` | dim | dim |
| dim | `*` | number | dim |
| dim | `/` | number | dim |
| number | `*` | dim | dim |
| number | `+` `-` `*` `/` | number | number |

Dim + dim works even with mixed units: `100mil + 2.54mm` evaluates correctly
because both convert to internal units before arithmetic.

Coord (point) arithmetic is NOT supported at the expression level. Coords are
constructed via tuple syntax `(x_expr, y_expr)` where each component is a scalar
expression.

### 7.3 Path Expressions (References)

Path expressions navigate bindings, entity declarations, and imported namespaces:

```
// Binding references ($ prefix)
$body                       // bound graphic entity
$body.left                  // edge anchor
$body.top_left              // corner anchor (coordinate only, not for on:)
$p2                         // bound pin entity
$fp.DIP8                    // import namespace access
$fp["SOT-23"]               // bracket access for special chars

// Let bindings (no $ prefix)
spacing                     // file-level let value
passive_pin                 // let-bound object (for spread)

// Anchor access on bound graphics
$body.left                  // left edge of rectangle
$body.right                 // right edge
$body.top                   // top edge
$body.bottom                // bottom edge
$body.center                // center point
```

**Path syntax:**

```
path       = root { step }
root       = '$' IDENT          // bound entity or import alias
           | IDENT              // let binding or enum
step       = '.' IDENT          // field access
           | '[' key ']'        // index access
key        = INTEGER            // numeric index
           | IDENT              // named index
           | STRING             // quoted index
```

**Resolution order** (when evaluating a bare identifier):

1. Built-in keywords: `true`, `false`, `null`
2. Bindings: innermost scope first, then outer scopes
3. Enum registry: if field expects an enum, check for match

`$`-prefixed identifiers resolve against bound entity declarations (within the
enclosing component/footprint/part scope) and import namespaces.

### 7.4 Coords (Tuples)

Coords are 2D points constructed with tuple syntax:

```
(x_expr, y_expr)
```

Each component is a scalar expression (dim, number, or reference to a dim).

```
at: (1000, 800)                                    // literal (mils)
at: (20mm, 0mm)                                    // with units
from: (-20mil, -10mil)                             // explicit mil
to: ($body.top_left.x + 100mil, $body.top_left.y)  // expressions
offset: (0, -0.5mm)                                // mixed units OK
```

**Single-element tuples:** `(expr)` is parenthesized grouping, not a 1-tuple.
Coords always have exactly 2 elements.

Mixed units in coord tuples are valid: `(1mm, 100mil)` evaluates each dimension
independently.

### 7.5 Arrays

Arrays use bracket syntax:

```
[expr, expr, ...]
```

```
points: [(-1.5mm, -1.6mm), (1.5mm, -1.6mm), (1.5mm, 1.6mm), (-1.5mm, 1.6mm)]
skip: [H8, H9, J8, J9]
```

Elements can be any expression. Element types should be homogeneous (enforced
by the type checker, not the parser).

### 7.6 Objects and Spread

Objects use brace syntax with optional bindings and spread:

```
{ [bindings...] [spread...] key: expr, key: expr, ... }
```

Objects appear as entity bodies, array elements, and template values for let bindings.

#### Basic objects

```
// Entity body
pad 1 { at: (-0.95mm, -1mm), shape: rectangular, x_size: 0.6mm, y_size: 0.7mm }

// Let-bound template
let passive_pin = { electrical: passive, length: 25, side: outside }
```

#### Spread operator (`...`)

The spread operator `...expr` expands an object expression's fields into the
enclosing object. The expression must evaluate to an object.

```
let smd = { layer: "TopLayer", pad_mode: simple, is_plated: false, hole_size: 0 }

pad 1 { ...smd, at: (-0.95mm, -1mm), shape: rectangular, x_size: 0.6mm, y_size: 0.7mm }
```

**Last-wins rule:** Explicit fields override spread fields. Later spreads override
earlier spreads:

```
let defaults = { shape: round, x_size: 60mil, y_size: 60mil }

pad 1 { ...defaults, shape: rectangular }  // rectangular overrides round
```

**Multiple spreads:**

```
let physical = { layer: "TopLayer", hole_size: 0 }
let sizing = { x_size: 0.6mm, y_size: 0.7mm }

pad 1 { ...physical, ...sizing, at: (-0.95mm, -1mm), shape: rectangular }
```

**Spread sources:** The expression after `...` can be:
- A let-bound object: `...passive_pin`
- Any expression that evaluates to an object

**Spread does NOT work in arrays.** `[...arr1, ...arr2]` is not supported.

## 8. Type System

### 8.1 Scalar Types

| AST type | Syntax | Maps to (Rust) |
|----------|--------|----------------|
| String | `"..."` | `String` |
| TemplateString | `` `...{expr}...` `` | `String` (after interpolation) |
| Integer | `42` | `i32` |
| Float | `3.14` | `f64` |
| Dim | `20mm` | `Coord` (single-axis) |
| Bool | `true` | `bool` |
| Null | `null` | `Option::None` |
| Color | `#FF0000` | `Color` |
| Ident | `passive` | Resolved by type checker |

### 8.2 Coord Type

| AST type | Syntax | Maps to (Rust) |
|----------|--------|----------------|
| Coord | `(x, y)` | `CoordPoint` |

Coords are always 2-element tuples. Each element is a scalar expression that
resolves to a dimensional value.

### 8.3 Unit Suffixes

Dimensional scalars carry a unit suffix that determines conversion to internal units:

| Suffix | Meaning | Internal units per 1 |
|--------|---------|---------------------|
| *(none)* | Mils (default) | 10,000 |
| `mil` | Mils (explicit) | 10,000 |
| `mm` | Millimeters | 393,701 |
| `in` | Inches | 10,000,000 |
| `dxp` | DXP units (10 mils) | 100,000 |
| `raw` | Raw internal units | 1 |

Bare numbers in dimensional fields default to mils. This is resolved at type-check
time, not parse time.

### 8.4 Enum Resolution

Bare identifiers in typed fields are resolved against the field's expected enum type.
Resolution is **case-insensitive** and **underscore-insensitive**.

```
electrical: passive          // PinElectricalType::Passive
electrical: open_collector   // PinElectricalType::OpenCollector
shape: round                 // PadShape::Round
shape: rectangular           // PadShape::Rectangular
layer: Top                   // V6Layer::TopLayer
color: red                   // Color::from_name("red")
component_kind: standard     // ComponentKind::Standard
```

If the field does not expect an enum, a bare identifier is resolved as a
binding first. If it's neither, the type checker reports an error.

### 8.5 Type Coercion Rules

The type checker applies these coercions at field boundaries:

| Field expects | Expression produces | Coercion |
|---------------|-------------------|----------|
| Dim | Integer | Apply default unit (mils) |
| Dim | Float | Apply default unit (mils) |
| Coord | 2-tuple of dims | Construct CoordPoint |
| String | *(no coercion)* | Must be quoted string |
| Enum | Ident | Look up in enum registry |
| Color | `#RRGGBB` | Parse hex |
| Color | Ident | Named color lookup |


## 9. Scoping & References

### 9.1 Intra-Spec References

All entity declarations (component, footprint, part, pin, pad, parameter,
graphics) support an optional binding prefix: `name = entity ...` or
`let name = entity ...`. The `let` keyword is optional and has no semantic
effect — it exists for readability.

Bindings are visible within their enclosing scope (component, footprint, or
part block). Bindings in a `part` block are NOT visible in other `part` blocks
or at component level.

Within a component, footprint, or part block, all bindings (including entity
declarations with binding names) are visible throughout the block **regardless
of source order**. Forward references are valid:

```
component R {
    pin 1 { on: $body.left, at: center }      // $body used before declaration
    body = rectangle { from: (-20mil, -10mil), to: (20mil, 10mil) }
}
```

This works because binding resolution is lazy — the expression AST is stored at
definition and evaluated at each use site. At evaluation time, all bindings in
the enclosing scope are available.

```
component R_0603 {
    body = rectangle { from: (-20mil, -10mil), to: (20mil, 10mil) }
    pin 1 { on: $body.left, at: center, side: outside }  // $body in scope
}
// $body is NOT visible outside the component block
```

**Part-to-part scope isolation:**
- Each `part` block has its own scope, isolated from other `part` blocks.
- Bindings declared at component level (outside any `part` block) are visible
  inside all `part` blocks via lexical scoping (read-only).
- Bindings inside `part 1 { ... }` are NOT visible inside `part 2 { ... }`.
- Two `part` blocks MAY declare bindings with the same name (e.g., both can
  have `body = rectangle { ... }`). These are distinct entities.

### 9.2 Cross-Entity References

Within a component, bound pins can be referenced by later pins (for `after`/`before`).
Pins without a binding prefix cannot be referenced by `after:`/`before:`:

```
component LM358 {
    part 1 {
        body = rectangle { ... }
        p2 = pin 2 { on: $body.left, at: start, gap: 30mil }
        pin 3 { on: $body.left, after: $p2, gap: 60mil }  // references $p2
    }
}
```

### 9.3 Circular Reference Detection

During expression evaluation, if a binding references itself directly or
transitively through spread or field access, this is an error:

```
error[E_CIRCULAR_BINDING]: binding 'a' has circular reference
  through 'b' → 'a'
  --> my-parts.schlib-spec:3:1
  |
  3 | let a = { ...b, x: 1 }
  |     ^ cycle starts here
  4 | let b = { ...a, y: 2 }
  |              ^ back-reference to 'a'
```

Cycle detection occurs at evaluation time (not parse time) since bindings
are lazy. The full cycle path is reported.


## 10. Identity Key Strategy

| Entity | Document | Identity Key | Source |
|--------|----------|-------------|--------|
| Component | SchLib | `lib_reference` | Name after `component` |
| Pin (single-part) | SchLib | `designator` (scoped to component) | Name after `pin` |
| Pin (multi-part) | SchLib | `(owner_part_id, designator)` | Part block number + name after `pin` |
| Parameter | SchLib | `name` (scoped) | Name after `parameter` |
| Alias | SchLib | `alias_name` (scoped) | Name after `alias` |
| Graphic | SchLib | `unique_id` on record | Binding name → see unique_id table below |
| Footprint | PcbLib | `display_name` | Name after `footprint` |
| Pad | PcbLib | `pad_name` (scoped) | Name after `pad` |
| PCB graphic | PcbLib | unique_id via sidecar | Binding name → see unique_id table below |

In a multi-part component, each `part N { ... }` block defines a separate
scope. Pins in different parts MAY have the same designator — they are
distinct entities with different `owner_part_id` values. Pins declared at
component level (outside any `part` block) have `owner_part_id = 0` (shared
across all parts).

**unique_id scheme:**

| Context | unique_id format |
|---------|-----------------|
| Component-level graphic | `spec:{component}:{name}` |
| Part-scoped graphic | `spec:{component}:part{N}:{name}` |
| Footprint graphic | `spec:{footprint}:{name}` |
| Unnamed graphic | `spec:{context}:{type}_{counter}` (not stable across edits) |

**Warning**: Unnamed graphics have no stable identity. The reconciler will
delete-and-re-add them on any spec change. Use binding names for all graphics
that need stable identity across edits.

**Uniqueness constraints** (enforced at parse time):
- No duplicate identity keys within their scope
- No duplicate binding names within a scope

### 10.1 Equality and Normalization Rules

Defines what "different" means for the reconciler when comparing spec values
to document values:

**(a) Dimensions** — All dimensional values normalize to i32 internal units
(10,000 per mil) before comparison. Conversion: `1mm = 393,701 internal units`
(rounded to nearest). `1in = 10,000,000 internal units`. `1dxp = 1 internal
unit`. `1raw = 1 internal unit`.

**(b) Coordinates** — Exact equality after normalization to internal units.
Tolerance: ±1 internal unit to absorb float-to-integer rounding. This means
`(0.5mm, 1mm)` and `(196,850raw, 393,701raw)` compare as equal (within ±1).

**(c) Colors** — Compare as normalized Win32 COLORREF `0x00BBGGRR` u32 values.
Case-insensitive hex input (`#ff0000` = `#FF0000`).

**(d) Strings** — Exact byte equality (case-sensitive). The following Altium
fields are case-insensitive at the format level and must be compared
case-insensitively: `lib_reference`, `designator` (pin), `pad_name`,
`display_name` (footprint). All other string fields: case-sensitive.

**(e) Enums** — Compare by canonical value after case-insensitive,
underscore-insensitive normalization. `open_collector` = `OpenCollector` =
`OPENCOLLECTOR`.

**(f) Booleans** — `true`/`false` only. Compare by value.


## 11. Merge Semantics: Ensure (Additive)

- Entities **in the spec** are added (if missing) or updated (if different)
- Entities **in the document but not in the spec** are **left untouched**
- **No deletions** — the spec is a subset assertion
- Future: `purge` modifier for full convergence

This means hand-crafted components can coexist with spec-managed components.

Empty components and footprints are valid (graphical symbols, mechanical parts).

### 11.1 Additive Semantics Limitations

- Entities removed from the spec are NOT removed from the document. The spec
  is a subset assertion.
- To remove entities, manually edit the document.
- Identity key changes (renames) are NOT renames — they result in the old entity
  persisting and a new entity being added. To rename, first remove the old
  entity manually.
- Future: `purge` modifier for full convergence semantics.


## 12. Engineering Change Order (ECO)

The `altium plan` command generates a full ECO suitable for hardware development review.

### 12.1 ECO Text Format

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

### 12.2 ECO JSON Format

With `--json`, the plan is output as structured JSON for machine consumption.
Includes all fields for each change, before/after values, and summary statistics.

### 12.3 ECO Data Structure

```rust
struct EngineeringChangeOrder {
    library_path: PathBuf,
    spec_path: PathBuf,
    timestamp: DateTime<Utc>,  // or std::time::SystemTime (implementation choice)
    summary: EcoSummary,
    changes: Vec<EntityChange>,
}

enum EntityKind {
    Component, Pin, Parameter, Alias, Graphic,
    Footprint, Pad, Track, Via, Arc, Text, Fill, Region,
}

struct EcoSummary {
    by_kind: IndexMap<EntityKind, KindSummary>,
}

struct KindSummary {
    adds: usize,
    updates: usize,
    unchanged: usize,
}

enum EntityChange {
    Add { kind: EntityKind, identity: String, props: Vec<PropChange>, children: Vec<EntityChange> },
    Update { kind: EntityKind, identity: String, prop_changes: Vec<PropChange>, children: Vec<EntityChange> },
    Unchanged { kind: EntityKind, identity: String },
}

struct PropChange {
    field: String,
    old_value: String,
    new_value: String,
}
```


## 13. Spec Dump (Reverse Generation)

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


## 14. Reconciliation Architecture

The spec system targets **`altium-format` LowOps directly** — it does NOT go through
the HighOp or ComposedOp layers used by the imperative ops pipeline. The imperative
pipeline (HighOp → ComposedOp → LowOp) exists for its own use case; the spec system
has its own entry point to the shared LowOp execution layer.

### 14.1 Execution Path

```
Spec file (.schlib-spec / .pcblib-spec)
    ↓ parse_spec()
SpecModel (semantic representation)
    ↓ reconcile(doc, spec_model)
EngineeringChangeOrder (EntityChange list)
    ↓ spec_to_low_ops(eco)                    ← direct mapping, no HighOp/ComposedOp
Vec<SchLibLowOp> or Vec<PcbLibLowOp>
    ↓ apply_schlib_low_ops(doc, ops)          ← existing executor
Mutated document + Vec<OpResult>
```

**Why LowOps directly (not HighOps)?**

The imperative ops pipeline decomposes coarse-grained user commands: a single
`AddComponent` HighOp expands into 5–11 ComposedOps (component root, designator,
comment, pins, implementation list, map definers, etc.). The spec reconciler has
already done this decomposition — it knows exactly which entities to add and which
fields to edit. Going through HighOps would reconstruct complexity the reconciler
already resolved.

The spec executor is also simpler than the imperative executor: it does not need
the `SchDocExecCtx` reference-resolution machinery (opid tracking, "last" component,
chain state). Each ECO `EntityChange` carries its own identity key from the
reconciler — the mapping to LowOps is self-contained:

| ECO entry | LowOp(s) |
|-----------|----------|
| `Add { kind: Component, ... }` | `CreateComponentRoot` + child `AddPin`/`AddParameter`/etc. |
| `Add { kind: Pin, ... }` | `AddPin` |
| `Update { kind: Component, prop_changes }` | `EditComponent` |
| `Update { kind: Pin, prop_changes }` | `EditPin` |
| `Unchanged { ... }` | *(no-op)* |

LowOps are the **public inter-crate contract** between `altium-format` and
`altium-format-ops` — the existing `composed_to_schlib_low.rs` files already
construct LowOp structs directly. The spec executor follows the same pattern.

### 14.2 Low-Level Ops Required for Reconciliation

The reconciler needs add and edit operations. These are implemented as LowOps in
`altium-format`'s `sch_ops_core` / `pcb_ops_core` modules. Each Edit LowOp follows
the `EditComponentOp` pattern: take a reference + optional fields, apply only
non-None fields.

**SchLib LowOps:**

| LowOp | Status | Description |
|--------|--------|-------------|
| `CreateComponentRoot` | Exists | Create component with properties |
| `AddPin` | Exists | Create pin with full properties |
| `AddParameter` | Exists | Create parameter |
| `AddAlias` | Exists | Create alias |
| `AddGraphic` | Exists | Create graphic primitive (all 13 types) |
| `EditComponent` | Exists | Change component-level properties (description, etc.) |
| `EditPin` | Needed | Change position, orientation, length, electrical type, name |
| `EditParameter` | Needed | Change text, visibility |
| `EditGraphic` | Needed | Change position, dimensions, colors, line width |

**PcbLib LowOps:**

| LowOp | Status | Description |
|--------|--------|-------------|
| `AddFootprint` | Exists | Create footprint with properties |
| `AddPad` | Exists | Create pad with full properties |
| `AddTrack` | Exists | Create track primitive |
| `EditPad` | Needed | Change position, shape, size, hole, rotation, masks |
| `EditTrack` | Needed | Change start, end, width, layer |
| `EditFootprint` | Needed | Change description, height, pattern |

Edit LowOps are dedicated types (not overloaded via `EditRecord`/`RecordPatch`)
to maintain type safety and match the project's domain-type discipline.

### 14.3 Reconciler Strategy

For entities where a targeted edit LowOp exists, the reconciler emits an edit.
For entities where only add exists, the reconciler uses **delete + re-add**
as a fallback (preserving the identity key).

The LowOp layer is the single source of truth for document mutation — shared by
both the imperative ops pipeline and the spec reconciliation system.


## 15. Lexical Rules

### 15.1 Tokens

| Token | Pattern | Examples |
|-------|---------|---------|
| `IDENT` | `[a-zA-Z_][a-zA-Z0-9_]*` | `component`, `passive`, `R1` |
| `STRING` | `"` (escape \| [^"\\])* `"` | `"R1"`, `"10K"` |
| `TEMPLATE` | `` ` `` { char \| `{` expr `}` } `` ` `` | `` `got {$body.width}` `` |
| `INTEGER` | `-`? `[0-9]+` | `42`, `-5`, `0` |
| `FLOAT` | `-`? `[0-9]+` `.` `[0-9]+` | `3.14`, `-0.5` |
| `DIM` | (`INTEGER` \| `FLOAT`) `UNIT` | `20mm`, `2.54mm`, `100mil` |
| `COLOR` | `#` `[0-9a-fA-F]{6}` | `#FF0000`, `#00ff00` |
| `DOLLAR_IDENT` | `$` `IDENT` | `$body`, `$fp`, `$p2` |
| `UNIT` | `mm` \| `mil` \| `in` \| `dxp` \| `raw` | |
| `DOTDOTDOT` | `...` | Spread operator |
| Punctuation | `: , . + - * / ( ) [ ] { }` | |
| Keywords | `true` `false` `null` `import` `as` `component` `footprint` `pin` `pad` `part` `parameter` `alias` `map` `row` `column` `grid` | |
| Noise | `let` `;` | Optional, ignored (§15.5) |
| Line comment | `//` ... newline | |
| Block comment | `/*` ... `*/` | Nesting allowed |

**Lexer disambiguation:**

- `#` followed by 6 hex digits → `COLOR`. `#` is only used for colors.
- Number immediately followed by a unit suffix (no whitespace) → `DIM`.
  `20 mm` is `INTEGER` `IDENT`, not `DIM`.
- `$` followed by identifier → `DOLLAR_IDENT`. Always.
- `-` is unary negation in prefix position, subtraction in infix position.
- `...` (three dots) is always the spread operator.

### 15.2 Separators

Values, fields, and array elements are separated by:

- **Comma** (`,`) — on the same line or across lines
- **Newline** — implicit separator when not inside `()`/`[]`

Commas are required between values on the same line. Newlines are sufficient
across lines. Trailing commas are always allowed.

```
// All valid:
{ a: 1, b: 2, c: 3 }         // inline, commas
{                              // block, newlines
    a: 1
    b: 2
    c: 3
}
{ a: 1, b: 2                  // mixed
  c: 3 }
{ a: 1, b: 2, c: 3, }        // trailing comma
```

**Newline suppression:** Newlines inside `()`, `[]`, and `{}` do NOT act as
separators for the *enclosing* context.

### 15.3 Comments

Line comments start with `//` and extend to end of line.
Block comments use `/* ... */` and may span multiple lines. Block comments nest.

```
// Line comment
component R {    // inline comment
    designator: "R?"
    /* This field is temporarily disabled:
    description: "Resistor"
    */
}
```

### 15.4 Whitespace

Spaces and tabs are insignificant (not indentation-sensitive). Newlines are
significant only as separators (§15.2).

### 15.5 Optional Noise Tokens (LLM Tolerance)

Since this language is primarily generated by LLMs, the parser accepts certain
tokens that have no semantic meaning:

| Token | Where accepted | Why LLMs emit it |
|-------|---------------|-------------------|
| `;` | After any statement or field | C/Rust/JS muscle memory |
| `let` | Before a binding (`let x = ...`) | Rust/JS/Python habit |
| Trailing `,` | After last element in `[]`, `{}` | Already valid, but worth noting |

**All of the following are equivalent:**

```
// Minimal (canonical)
passive_pin = { electrical: passive, length: 25 }

// With noise tokens (also valid)
let passive_pin = { electrical: passive, length: 25, };
```


## 16. Formal Grammar

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
(* Binding prefix (uniform across all entity declarations)           *)
(* The "let" keyword is optional and has no semantic effect.         *)
(* ================================================================ *)

binding_prefix  = ["let"] IDENT "=" ;

(* ================================================================ *)
(* SchLib declarations                                               *)
(* ================================================================ *)

component_decl  = [binding_prefix] "component" entity_name "{" { component_item [sep] } "}" ;

component_item  = let_binding
                | property
                | part_block
                | pin_decl
                | parameter_decl
                | alias_decl
                | footprint_map_decl
                | graphic_decl ;

part_block      = [binding_prefix] "part" INTEGER "{" { part_item [sep] } "}" ;
part_item       = let_binding | pin_decl | graphic_decl ;

pin_decl        = [binding_prefix] "pin" entity_name object ;
parameter_decl  = [binding_prefix] "parameter" entity_name object ;
alias_decl      = "alias" entity_name ;
footprint_map_decl = "footprint" ( entity_name | dollar_path ) "{" { map_entry [sep] } "}" ;
map_entry       = "map" object ;

graphic_decl    = [binding_prefix] GRAPHIC_TYPE object ;

GRAPHIC_TYPE    = "line" | "rectangle" | "arc" | "elliptical_arc" | "ellipse"
                | "polyline" | "polygon" | "bezier" | "pie"
                | "round_rectangle" | "label" | "text_frame" | "image" ;

(* ================================================================ *)
(* PcbLib declarations                                               *)
(* ================================================================ *)

footprint_decl  = [binding_prefix] "footprint" entity_name "{" { footprint_item [sep] } "}" ;

footprint_item  = let_binding
                | property
                | pad_decl
                | row_decl
                | grid_decl
                | pcb_graphic_decl ;

pad_decl        = [binding_prefix] "pad" entity_name object ;

row_decl        = ("row" | "column") object ;
grid_decl       = "grid" object ;

pcb_graphic_decl = [binding_prefix] PCB_GRAPHIC_TYPE object ;
PCB_GRAPHIC_TYPE = "track" | "arc" | "fill" | "region" | "text"
                 | "via" | "component_body"
                 | "line" | "polyline" ;

(* ================================================================ *)
(* Dollar paths (lexer produces DOLLAR_IDENT as a single token)      *)
(* ================================================================ *)

dollar_path     = DOLLAR_IDENT { "." IDENT | "[" expr "]" } ;

(* ================================================================ *)
(* Shared productions                                                *)
(* ================================================================ *)

property        = IDENT ":" expr ;
object          = "{" [ object_body ] "}" ;
object_body     = object_item { sep object_item } ;
object_item     = let_binding | spread | property ;
spread          = "..." expr ;

(* ================================================================ *)
(* Expression (Pratt parser, every value position)                  *)
(* ================================================================ *)

expr            = pratt_expr ;

(* Pratt with binding powers — see §7.2 for precedence table       *)
pratt_expr      = prefix_expr { infix_op pratt_expr } ;

prefix_expr     = STRING
                | template                              (* `text {expr}` *)
                | INTEGER
                | FLOAT
                | DIM
                | COLOR
                | BOOL
                | "null"
                | DOLLAR_IDENT path_tail            (* $ref.field *)
                | IDENT path_tail                   (* binding or enum *)
                | "-" pratt_expr                    (* unary negate *)
                | "(" expr "," expr ")"             (* coord tuple *)
                | "(" expr ")"                      (* grouping *)
                | "[" [ expr_list ] "]"             (* array *)
                | object ;                          (* nested object *)

infix_op        = "+" | "-" | "*" | "/"             (* arithmetic *)
                | "." IDENT                          (* field access *)
                | "[" expr "]" ;                     (* index access *)

path_tail       = { "." IDENT | "[" expr "]" } ;

expr_list       = expr { sep expr } ;

(* Template string — backtick-delimited with {expr} interpolation *)
template        = '`' { char | '{' expr '}' | '{{' | '}}' } '`' ;

sep             = "," | NEWLINE ;
```

**Note**: The `object` production (`{ property | spread | let_binding }`) is
a plain property map. Entity container bodies (`component_decl`, `footprint_decl`,
`part_block`) use separate productions that additionally allow child entity
declarations (pins, pads, graphics, etc.). Entity-specific declarations like
`pin_decl` or `graphic_decl` are NOT valid inside a plain `object`.


## 17. Complete Examples

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

    body = rectangle { from: (-3.5mm, -3.5mm), to: (3.5mm, 3.5mm) }

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
        p2 = pin 2 { name: "IN-",  ...input_pin, on: $body.left, at: start, gap: 30mil }
        pin 3 { name: "IN+",  ...input_pin, on: $body.left, after: $p2, gap: 60mil }
    }

    part 2 {
        body = rectangle { from: (-100mil, -100mil), to: (100mil, 100mil) }
        p5 = pin 5 { name: "IN+",  ...input_pin, on: $body.left, at: start, gap: 30mil }
        pin 6 { name: "IN-",  ...input_pin, on: $body.left, after: $p5, gap: 60mil }
        pin 7 { name: "OUT",  ...output_pin, on: $body.right, at: center }
    }

    // Shared pins — owner_part_id = 0
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


## 18. Scope Boundaries

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
- Expression language with dimensional scalars, coords, arithmetic, spread
- Full type system with unit coercion and enum resolution

### What We Don't Build (Initially)

- **No imperative verbs.** Specs declare, they don't command.
- **No selectors.** Specs don't query — they declare.
- **No SchDoc/PcbDoc support.** Placed instances have harder identity problems.
- **No purge/delete semantics.** Additive only.
- **No control flow.** No if/else, no loops.
- **No functions.** No sin(), sqrt(), min(). Complex geometry pre-computed by agent.
- **No anchor inference in dump.** Dump generates absolute coordinates.
- **No Coord arithmetic.** No `point + point`. Compose via `(x_expr, y_expr)`.
- **No array spread.** `[...a, ...b]` is not supported.

### Resolved Questions

1. **SchPie `unique_id`**: SchPie records support `UNIQUEID` parameter via
   `SchPrimitiveBase`. Verified in codebase — no field addition needed.
2. **PcbLib sidecar write**: Confirmed: `serialize_unique_id_primitive_information()`
   writes arbitrary string unique_ids to `UniqueIDPrimitiveInformation` sidecar.
3. **Coordinate tolerance**: Resolved: ±1 internal unit tolerance. See §10.1(b).
4. **Row/grid pad override**: Resolved in §5.2. See field-level override
   semantics.
5. **Import cycle detection**: Resolved. Topological sort with cycle
   detection error. See §6.3.
