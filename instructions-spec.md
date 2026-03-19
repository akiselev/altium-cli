# Altium Spec Language Reference

This document describes the `altium-cli` spec language, a declarative DSL for creating
and modifying Altium Designer files. It is intended as a companion to `instructions.md`
(the PCB design workflow) and enables LLM agents to produce concrete Altium artifacts
at every stage of the design process.

---

## Overview

The spec language lets you describe Altium files declaratively. Instead of manually
clicking through Altium Designer's GUI, you write a `.spec` file that describes what
components, footprints, schematics, or board features you want, and `altium-cli`
creates or updates the corresponding Altium file.

**Key design principles:**
- **Declarative**: describe the desired state, not mutation commands
- **Idempotent**: applying the same spec twice produces no changes
- **Additive**: a spec is a subset assertion, not a complete replacement — unlisted
  entities in an existing file are preserved
- **ECO-grade**: the `plan` command shows exactly what would change before you commit

**File extensions and their target Altium formats:**

| Spec extension    | Altium output | Purpose                        |
| ----------------- | ------------- | ------------------------------ |
| `.schlib-spec`    | `.SchLib`     | Schematic symbol libraries     |
| `.pcblib-spec`    | `.PcbLib`     | PCB footprint libraries        |
| `.schdoc-spec`    | `.SchDoc`     | Schematic sheets               |
| `.pcbdoc-spec`    | `.PcbDoc`     | PCB board documents            |
| `.prjpcb-spec`    | `.PrjPcb`     | Altium project configuration   |

---

## CLI Commands

```bash
# Preview changes (ECO) without modifying anything
altium plan my-library.schlib-spec
altium plan my-library.schlib-spec --json        # machine-readable ECO

# Apply spec → create or update Altium file
altium apply my-library.schlib-spec                          # creates my-library.SchLib
altium apply my-library.schlib-spec --output parts.SchLib    # custom output name
altium apply my-library.schlib-spec --target existing.SchLib # update existing file

# Reverse-generate spec from existing Altium file
altium dump existing.SchLib                                  # prints to stdout
altium dump existing.PcbLib --output footprints.pcblib-spec  # writes to file
```

`plan` generates an Engineering Change Order (ECO) showing adds, updates, and
unchanged entities. `apply` executes the ECO. `dump` reverse-generates a spec from
an existing file (useful for bootstrapping specs from legacy libraries).

---

## Language Basics

### Comments

```
// Line comment

/* Block comment */

/* Block comments
   /* can be nested */
   safely */
```

### Literals

```
// Strings
"hello world"

// Integers
42
-100

// Floats
3.14

// Dimensional values (no space between number and unit)
100mil        // 100 mils (thousandths of an inch)
2.54mm        // 2.54 millimeters
1in           // 1 inch

// Colors (RGB hex, 6 digits)
#FF0000       // red
#00FF00       // green
#0000FF       // blue

// Booleans
true
false

// Null
null

// Coordinates (x, y tuple)
(100mil, 200mil)
(-2.54mm, 0)
(0, 0)

// Arrays
[1, 2, 3]
[(0, 0), (100mil, 0), (100mil, 100mil)]

// Objects
{ shape: rectangular, x_size: 0.8mm, y_size: 0.9mm }
```

### Units

All dimensional values are converted to Altium's internal coordinate system
(10,000 internal units = 1 mil). Supported units:

| Unit  | Meaning          | Internal units per 1       |
| ----- | ---------------- | -------------------------- |
| `mil` | Mils (thou)      | 10,000                     |
| `mm`  | Millimeters      | 393,701 (≈ 10000/0.0254)  |
| `in`  | Inches           | 10,000,000                 |
| `dxp` | DXP units        | 100,000                    |
| `raw` | Raw internal     | 1                          |

Units can be mixed freely in expressions:

```
let total = 100mil + 2.54mm    // both converted to internal coords
let half = 100mil / 2          // dimension / scalar = dimension
let double = 2 * 100mil        // scalar * dimension = dimension
```

### Let Bindings and Variables

Variables are defined with `let` (the keyword is optional at file level):

```
let pitch = 2.54mm
let pad_size = 0.8mm
silk = 5mil                    // 'let' keyword optional at file level

// Use in expressions
pad 1 { at: (-pitch, 0), x_size: pad_size }
```

Bindings are scoped: file-level bindings are visible everywhere, entity-level
bindings are visible within their block. Forward references within the same scope
are allowed (two-pass resolution).

### Spread Operator

The `...expr` operator merges an object's key-value pairs into the surrounding object.
This is the primary mechanism for pad templates and shared defaults:

```
let smd = { layer: "TopLayer", pad_mode: simple, is_plated: false, hole_size: 0 }
let th = { layer: "MultiLayer", pad_mode: simple, is_plated: true }

// Spread + override: inherit all smd defaults, set shape and size
let pad_0603 = { ...smd, shape: rectangular, x_size: 0.8mm, y_size: 0.9mm }

// Use in a pad definition
pad 1 { ...pad_0603, at: (-0.75mm, 0) }
```

Later keys override earlier ones, so `{ ...defaults, size: 20 }` uses the
spread's `size` only if you don't also specify `size: 20`.

### Arithmetic Expressions

```
let x = 100mil + 50mil         // 150mil
let y = 100mil * 2             // 200mil (dimension * scalar)
let z = 100mil / 2             // 50mil  (dimension / scalar)
let w = 2.54mm - 1mm           // 1.54mm (mixed units ok)
let half = 3.14 / 2.0          // 1.57 (float arithmetic)
```

Operator precedence: `*` `/` (60) > `+` `-` (50). Field access `.` and indexing `[]`
have the highest precedence (90).

### Template Strings

Backtick-delimited strings with `{expr}` interpolation:

```
let width = 100mil
let label = `Width: {width}`           // "Width: 100mil"
let desc = `Part {$name} rev {$rev}`   // interpolate variables
```

Escape sequences: `\`` `\\` `\n` `\r` `\t` `\{` `\}`. Double braces `{{` `}}` also
produce literal braces.

### Entity Bindings

Named entities create variables that can be referenced by later items:

```
// Naming a graphic creates a reference for pin anchoring
body = rectangle { from: (-200mil, -100mil), to: (200mil, 100mil) }

// $body is now a reference with anchor points
pin 1 { on: $body.left, at: center, side: outside }
```

### Dollar References

`$name` references a binding. Path access with `.` and `[]`:

```
$body.left           // left edge anchor of a rectangle
$body.center         // center point
$import.ComponentName // entity from a named import
$ref[0]              // array indexing
```

---

## Schematic Library Specs (`.schlib-spec`)

SchLib specs define schematic symbols — the components that appear on schematic sheets.
This corresponds to **Stage 4** of the design workflow (Symbol generation).

### Component Block

```
component RESISTOR {
    designator: "R?"
    description: "Standard resistor"
    component_kind: standard          // standard | mechanical | graphical | net_tie_bom
                                      //   | net_tie_no_bom | jumper
    show_hidden_pins: false

    // Graphics (body shape) — named binding for pin anchoring
    body = rectangle {
        from: (-20mil, -40mil)
        to: (20mil, 40mil)
        is_solid: true
        color: #000080
        area_color: #FFFFC8
    }

    // Pins — anchored to body edges (orientation: auto inferred from edge)
    pin 1 { on: $body.top,    at: center, side: outside, electrical: passive, length: 40mil }
    pin 2 { on: $body.bottom, at: center, side: outside, electrical: passive, length: 40mil }

    // Parameters
    parameter Value { text: "10K", is_hidden: false }
    parameter Footprint { text: "0603", is_hidden: true }

    // Aliases (alternate names for same symbol)
    alias R0603
    alias R0805

    // Footprint mapping (pin-to-pad)
    footprint "0603" {
        map { pin: 1, pad: 1 }
        map { pin: 2, pad: 2 }
    }
}
```

### Component Properties

| Property           | Type              | Required | Description                         |
| ------------------ | ----------------- | -------- | ----------------------------------- |
| `designator`       | string            | no       | Designator pattern, e.g. `"R?"`     |
| `description`      | string            | no       | Component description               |
| `component_kind`   | enum              | no       | See enum values above               |
| `part_count`       | integer           | no       | Number of parts in multi-part       |
| `show_hidden_pins` | boolean           | no       | Show hidden power pins              |

### Pin Properties

| Property          | Type              | Required | Description                              |
| ----------------- | ----------------- | -------- | ---------------------------------------- |
| `on`              | anchor            | preferred| Edge anchor (e.g. `$body.left`)          |
| `at`              | enum/coord        | yes      | `start`/`center`/`end` on anchor, or absolute coord |
| `side`            | enum              | no       | `outside` (default) or `inside`          |
| `orientation`     | 0/90/180/270/auto | no       | Pin direction (`auto` inferred from edge)|
| `electrical`      | enum              | no       | See electrical types below               |
| `length`          | dimension         | no       | Pin stub length                          |
| `name`            | string            | no       | Pin function name                        |
| `is_hidden`       | boolean           | no       | Hidden pin (power pins)                  |
| `hidden_net_name` | string            | no       | Net name for hidden power pins           |
| `after`           | reference         | no       | Chain after another pin on same edge     |
| `before`          | reference         | no       | Chain before another pin on same edge    |
| `gap`             | dimension         | no       | Spacing from anchor point or chained pin |
| `offset`          | dimension         | no       | Fine adjustment along the edge           |

**Pin electrical types:** `input`, `output`, `input_output`, `open_collector`,
`open_emitter`, `passive`, `hi_z`, `power`

**Prefer anchor-based placement over absolute coordinates.** Anchors make symbols
resilient to body size changes and easier to read. Absolute `at: (x, y)` coordinates
are supported but should only be used for hidden power pins or unusual layouts.

### Pin Anchoring System

Pins are placed relative to named graphics (rectangles, round rectangles, text frames,
images) using the `on:` property. This is the recommended approach for all visible pins.

```
// 1. Name a box-type graphic
body = rectangle { from: (-200mil, -100mil), to: (200mil, 100mil), is_solid: true }

// 2. Place pins relative to edges
pin 1 { on: $body.left,   at: center, side: outside, electrical: input,  length: 30mil }
pin 2 { on: $body.right,  at: center, side: outside, electrical: output, length: 30mil }

// 3. Chain pins with after/before (implicit $pin3 binding auto-created)
pin 3 { on: $body.left, at: start, gap: 30mil, side: outside, electrical: input }
pin 4 { on: $body.left, after: $pin3, gap: 60mil, side: outside, electrical: input }
```

**Edge anchors:** `$body.left`, `$body.right`, `$body.top`, `$body.bottom`
**Corner anchors:** `$body.top_left`, `$body.top_right`, `$body.bottom_left`, `$body.bottom_right`
**Center:** `$body.center`

**Position on edge:** `at: start`, `at: center`, `at: end`
**Chaining:** `after: $pin3`, `before: $pin5` with `gap: <dim>`
**Placement side:** `side: outside` (default) or `side: inside`
**Offset:** `offset: <dim>` for fine adjustment along the edge

**Implicit bindings:** Every `pin N` declaration automatically creates a `$pinN`
reference (e.g., `pin 7` creates `$pin7`, `pin SDA` creates `$pinSDA`). Explicit
bindings (`my_ref = pin 1 { ... }`) still work and take priority.

When using `orientation: auto`, the pin direction is inferred from the edge
(left edge = points right = 0 degrees, etc.).

### Multi-Part Components

ICs with multiple identical sections (e.g., dual op-amp LM358):

```
component LM358 {
    designator: "U?"
    description: "Dual operational amplifier"
    part_count: 2

    part 1 {
        body = rectangle { from: (-100mil, -100mil), to: (100mil, 100mil), is_solid: true }
        pin 2 { on: $body.left,  at: start,  gap: 30mil, side: outside, electrical: input,  name: "IN+" }
        pin 3 { on: $body.left,  at: end,    gap: 30mil, side: outside, electrical: input,  name: "IN-" }
        pin 1 { on: $body.right, at: center,             side: outside, electrical: output, name: "OUT" }
    }

    part 2 {
        body = rectangle { from: (-100mil, -100mil), to: (100mil, 100mil), is_solid: true }
        pin 6 { on: $body.left,  at: start,  gap: 30mil, side: outside, electrical: input,  name: "IN+" }
        pin 5 { on: $body.left,  at: end,    gap: 30mil, side: outside, electrical: input,  name: "IN-" }
        pin 7 { on: $body.right, at: center,             side: outside, electrical: output, name: "OUT" }
    }

    // Shared hidden power pins (absolute coords ok for hidden pins)
    pin 8 { at: (0, -200mil), orientation: 90,  electrical: power, is_hidden: true, hidden_net_name: "V+" }
    pin 4 { at: (0, 200mil),  orientation: 270, electrical: power, is_hidden: true, hidden_net_name: "GND" }
}
```

### Example: IC with Chained Pins

For ICs with many pins per side, use `after:` to chain pins sequentially along an
edge. Every pin automatically gets an implicit binding derived from its designator
(`pin 3` creates `$pin3`), so no explicit bindings are needed:

```
component ADS1115 {
    designator: "U?"
    description: "16-bit ADC, 4-channel"

    body = rectangle { from: (-200mil, -250mil), to: (200mil, 250mil), is_solid: true }

    // Left side: inputs, chained top to bottom
    pin 1 { on: $body.left, at: start, gap: 50mil, side: outside, electrical: input, name: "ADDR" }
    pin 2 { on: $body.left, after: $pin1, gap: 100mil, side: outside, electrical: hi_z,  name: "ALERT" }
    pin 3 { on: $body.left, after: $pin2, gap: 100mil, side: outside, electrical: input, name: "AIN0" }
    pin 4 { on: $body.left, after: $pin3, gap: 100mil, side: outside, electrical: input, name: "AIN1" }

    // Right side: power and I2C, chained top to bottom
    pin 5 { on: $body.right, at: start, gap: 50mil, side: outside, electrical: power,       name: "VDD" }
    pin 6 { on: $body.right, after: $pin5, gap: 100mil, side: outside, electrical: input_output, name: "SDA" }
    pin 7 { on: $body.right, after: $pin6, gap: 100mil, side: outside, electrical: input,       name: "SCL" }

    // Bottom: ground
    pin 8 { on: $body.bottom, at: center, side: outside, electrical: power, name: "GND" }

    parameter Value { text: "ADS1115", is_hidden: false }

    footprint "MSOP-10" {
        map { pin: 1, pad: 1 }
        map { pin: 2, pad: 2 }
        map { pin: 3, pad: 3 }
        map { pin: 4, pad: 4 }
        map { pin: 5, pad: 10 }
        map { pin: 6, pad: 9 }
        map { pin: 7, pad: 8 }
        map { pin: 8, pad: 5 }
    }
}
```

The `after: $pin1` syntax means "place this pin below (or after) pin 1 on the same
edge, with `gap` spacing between them." Every `pin N` declaration automatically creates
a `$pinN` reference. Explicit bindings (`my_ref = pin 1 { ... }`) still work and take
priority — use them when you want a more descriptive name.

### Schematic Graphics

Available graphic types for symbol drawing:

| Type               | Key properties                                                |
| ------------------ | ------------------------------------------------------------- |
| `line`             | `from`, `to`, `color`, `line_width`                           |
| `rectangle`        | `from`, `to`, `is_solid`, `color`, `area_color`               |
| `round_rectangle`  | `from`, `to`, `corner_x_radius`, `corner_y_radius`            |
| `arc`              | `center`, `radius`, `start_angle`, `end_angle`                |
| `elliptical_arc`   | `center`, `radius`, `secondary_radius`, angles                |
| `ellipse`          | `center`, `radius`, `secondary_radius`, `is_solid`            |
| `pie`              | `center`, `radius`, `start_angle`, `end_angle`, `is_solid`    |
| `polyline`         | `points`, `color`, `line_width`                               |
| `polygon`          | `points`, `is_solid`, `color`, `area_color`                   |
| `bezier`           | `points`, `color`, `line_width`                               |
| `label`            | `at`, `text`, `font_id`, `color`                              |
| `text_frame`       | `from`, `to`, `text`, `is_solid`, `show_border`               |
| `image`            | `from`, `to`, `file_name`, `image_data`                       |

---

## PCB Footprint Library Specs (`.pcblib-spec`)

PcbLib specs define PCB footprints (land patterns). This corresponds to **Stage 4**
of the design workflow (Footprint generation).

### Footprint Block

```
footprint "0603" {
    description: "0603 (1608 Metric) 2-pad SMD"
    height: 0.55mm

    pad 1 { ...pad_0603, at: (-0.75mm, 0) }
    pad 2 { ...pad_0603, at: (0.75mm, 0) }

    outline = polyline {
        points: [(-1.3mm, -0.65mm), (1.3mm, -0.65mm), (1.3mm, 0.65mm), (-1.3mm, 0.65mm)]
        width: 5mil, layer: "TopOverlay", closed: true
    }
}
```

### Footprint Properties

| Property      | Type      | Required | Description                  |
| ------------- | --------- | -------- | ---------------------------- |
| `description` | string    | no       | Footprint description        |
| `height`      | dimension | no       | Component height for 3D      |
| `pattern`     | string    | no       | Pattern identifier           |

### Pad Properties

| Property                  | Type      | Required | Description                                  |
| ------------------------- | --------- | -------- | -------------------------------------------- |
| `at`                      | coord     | yes      | Pad center position                          |
| `shape`                   | enum      | no       | `round`, `rectangular`, `octagonal`, `round_rect` |
| `x_size`                  | dimension | no       | Pad width                                    |
| `y_size`                  | dimension | no       | Pad height                                   |
| `rotation`                | float     | no       | Rotation in degrees                          |
| `hole_size`               | dimension | no       | Drill hole diameter (0 = SMD)                |
| `is_plated`               | boolean   | no       | Plated through-hole                          |
| `layer`                   | layer     | no       | `"TopLayer"`, `"MultiLayer"`, etc.           |
| `pad_mode`                | enum      | no       | `simple`, `local_stack`, `external_stack`     |
| `solder_mask_expansion`   | dimension | no       | Mask expansion override                      |
| `paste_mask_expansion`    | dimension | no       | Paste expansion override                     |
| `plane_connection`        | enum      | no       | `relief`, `direct`, `no_connect`             |
| `relief_conductor_width`  | dimension | no       | Thermal relief spoke width                   |
| `relief_entries`          | integer   | no       | Number of relief spokes (usually 4)          |
| `relief_air_gap`          | dimension | no       | Gap between pad and copper pour              |

### Pad Templates with Spread

The idiomatic way to define footprints is with shared pad templates:

```
// File-level templates
let smd = { layer: "TopLayer", pad_mode: simple, is_plated: false, hole_size: 0 }
let th = { layer: "MultiLayer", pad_mode: simple, is_plated: true }

// Size templates that inherit from base
let pad_0402 = { ...smd, shape: rectangular, x_size: 0.5mm, y_size: 0.5mm }
let pad_0603 = { ...smd, shape: rectangular, x_size: 0.8mm, y_size: 0.9mm }
let pad_0805 = { ...smd, shape: rectangular, x_size: 1.0mm, y_size: 1.3mm }

footprint "0603" {
    pad 1 { ...pad_0603, at: (-0.75mm, 0) }
    pad 2 { ...pad_0603, at: (0.75mm, 0) }
}
```

### Row Layout (Linear Pad Arrays)

For ICs with pins along edges, use `row` to generate pads automatically:

```
footprint "SOIC-8" {
    let sp = { ...smd, shape: rectangular, x_size: 0.6mm, y_size: 1.5mm }
    let body = { from: (-2.0mm, -2.5mm), to: (2.0mm, 2.5mm) }

    // Left side: pins 1-4, top to bottom
    row {
        on: $body.left, at: center       // anchor to left edge, centered
        pitch: 1.27mm, count: 4, start: 1
        side: outside                     // pads extend outward from body
        pad: { ...sp }                   // pad template
    }

    // Right side: pins 5-8, bottom to top
    row {
        on: $body.right, at: center
        pitch: 1.27mm, count: 4, start: 5
        side: outside, direction: reverse // reverse = bottom to top
        pad: { ...sp }
    }
}
```

**Row properties:**

| Property    | Type      | Required | Description                                     |
| ----------- | --------- | -------- | ----------------------------------------------- |
| `on`        | anchor    | no*      | Edge anchor (`$body.left`, etc.)                |
| `at`        | enum/coord| yes      | Position: `start`, `center`, `end`, or coord    |
| `pitch`     | dimension | yes      | Spacing between pads                            |
| `count`     | integer   | yes      | Number of pads to generate                      |
| `start`     | integer   | yes      | First pad name/number                           |
| `direction` | enum      | no       | `forward` (default), `reverse`, `up`, `down`, `left`, `right` |
| `side`      | enum      | no       | `outside` (default), `inside`, `center`         |
| `pad`       | object    | yes      | Pad template (shape, size, layer, etc.)         |
| `skip`      | array     | no       | Pad numbers to skip, e.g. `[3, 4]`             |

*Either `on` (anchor-based) or `at` with a coordinate (absolute) is needed.

`column` is an alias for `row` — they are functionally identical.

### Grid Layout (BGA / Matrix Arrays)

For BGA packages or matrix pad arrays:

```
footprint "BGA-256" {
    grid {
        origin: (0, 0)
        rows: 16, cols: 16
        pitch: 1mm                       // or pitch_x / pitch_y for non-square
        naming: alphanumeric             // A1, A2, ... B1, B2, ... (skip I, O, Q, S)
        pad: { ...smd, shape: round, x_size: 0.4mm, y_size: 0.4mm }
        skip: [H8, H9, J8, J9]          // thermal void in center
    }
}
```

**Grid properties:**

| Property         | Type      | Required | Description                                       |
| ---------------- | --------- | -------- | ------------------------------------------------- |
| `origin`         | coord     | yes      | Grid center point                                 |
| `rows`           | integer   | yes      | Number of rows                                    |
| `cols`           | integer   | yes      | Number of columns                                 |
| `pitch`          | dimension | yes*     | Uniform pitch (* or use `pitch_x`/`pitch_y`)      |
| `pitch_x`        | dimension | no       | Column spacing (overrides `pitch`)                |
| `pitch_y`        | dimension | no       | Row spacing (overrides `pitch`)                   |
| `naming`         | enum      | no       | `numeric` (1,2,3...) or `alphanumeric` (A1,A2...) |
| `pad`            | object    | yes      | Pad template                                      |
| `skip`           | array     | no       | Pad names to omit                                 |
| `perimeter_only` | boolean   | no       | Only place pads on grid edges                     |

For `alphanumeric` naming, letters I, O, Q, and S are skipped (per industry convention).

### PCB Graphics

Available graphic types for footprint drawing:

| Type             | Key properties                                              |
| ---------------- | ----------------------------------------------------------- |
| `line`           | `from`, `to`, `width`, `layer`                              |
| `polyline`       | `points`, `width`, `layer`, `closed`                        |
| `arc`            | `center`, `radius`, `start_angle`, `end_angle`, `width`, `layer` |
| `track`          | `from`, `to`, `width`, `layer`                              |
| `fill`           | `from`, `to`, `layer`                                       |
| `region`         | `points`, `layer`                                           |
| `text`           | `at`, `text`, `layer`, `rotation`                           |
| `via`            | `at`, `hole_size`, `diameter`                               |
| `component_body` | `layer`                                                     |

Common layers: `"TopOverlay"` (silkscreen), `"TopLayer"`, `"BottomLayer"`,
`"MultiLayer"`, `"Mechanical1"` through `"Mechanical16"`, `"TopSolderMask"`,
`"TopPasteMask"`.

### Pad Override After Row/Grid

You can override individual pad properties after a `row` or `grid` generates them.
For example, making pin 1 rectangular on a through-hole connector:

```
footprint "JST-XH-4" {
    row {
        at: (-3.75mm, 0)
        pitch: 2.5mm, count: 4, start: 1
        direction: right
        pad: { ...th, shape: round, x_size: 1.6mm, y_size: 1.6mm, hole_size: 1.0mm }
    }

    // Override pin 1 shape (convention: pin 1 is rectangular)
    pad 1 { shape: rectangular }
}
```

---

## Schematic Document Specs (`.schdoc-spec`)

SchDoc specs define schematic sheets — the wiring diagrams that connect components.
This corresponds to **Stage 5** of the design workflow (Schematic capture).

### Sheet Configuration

```
sheet {
    custom_width: 6200mil
    custom_height: 3600mil
    snap_grid_on: true
    visible_grid_on: true
    hot_spot_grid_on: true
    show_hidden_pins: false
    border_on: true
    title_block_on: true

    fonts {
        font 1 { name: "Times New Roman", size: 10 }
        font 2 { name: "Arial", size: 8, bold: true }
    }
}
```

### Placing Components

Components reference symbols from a SchLib by their `lib_reference` name. Each
component automatically creates an implicit binding from its designator (`component "U1"`
creates `$U1` as a coordinate), enabling relative placement of subsequent components:

```
component "U1" {
    lib_reference: "ESP32-C6-MINI-1"
    at: (3200mil, 2600mil)
    orientation: 0                 // 0, 90, 180, 270
    is_mirrored: false
    description: "Main MCU"
}

// Relative placement: 200mil to the right of U1
component "C1" {
    lib_reference: "CAP-0402"
    at: ($U1.x + 200mil, $U1.y)
    description: "Decoupling cap for U1"
}

component "R1" {
    lib_reference: "RES-0603"
    at: (2000mil, 2000mil)
    orientation: 90
}
```

The implicit `$U1` binding is a coordinate with `.x` and `.y` fields that return
dimensional values usable in arithmetic expressions.

### Power Objects and Net Labels

```
// Power symbols (connect to named power nets)
power_object "VCC_3V3" { at: (1700mil, 3350mil) }
power_object "GND" { at: (1700mil, 1700mil), orientation: 180 }
power_object "VBUS" {
    at: (500mil, 3350mil)
    style: power_bar              // bar, arrow, wave, ground, etc.
    show_net_name: true
}

// Net labels (name a wire)
net_label "SDA" { at: (2500mil, 2000mil) }
net_label "SCL" { at: (2500mil, 1800mil), orientation: 0 }
```

### Wires

Wires connect pins and create electrical nets. Specify vertices as coordinate arrays:

```
// Simple wire: two points
wire { vertices: [(500mil, 3000mil), (1700mil, 3000mil)] }

// Multi-segment wire: chain of points
wire {
    vertices: [(1700mil, 3350mil), (1700mil, 3000mil), (2600mil, 3000mil), (3200mil, 2900mil)]
}

// Wire with visual properties
wire {
    vertices: [(100mil, 100mil), (500mil, 100mil)]
    color: #0000FF
    line_width: small              // smallest, small, medium, large
}
```

### Other Schematic Objects

```
// Bus
bus { vertices: [(1000mil, 500mil), (3000mil, 500mil)] }

// Junction (wire crossing that connects)
junction { location: (2000mil, 2000mil) }

// No-connect marker
no_connect { location: (3000mil, 1500mil) }

// Bus entry
bus_entry { location: (1000mil, 600mil), corner: (1100mil, 700mil) }

// Port (hierarchical connection)
port "DATA_BUS" {
    location: (5000mil, 2000mil)
    io_type: bidirectional         // unspecified, output, input, bidirectional
    width: 200mil
    height: 100mil
}

// Sheet symbol (hierarchical sub-sheet)
sheet_symbol "Power Supply" {
    sheet_name: "Power Supply"
    file_name: "PSU.SchDoc"
    location: (1000mil, 1000mil)
    x_size: 800mil
    y_size: 600mil
    entries: [
        { name: "VIN", io_type: input, side: left, distance_from_top: 100mil },
        { name: "VOUT", io_type: output, side: right, distance_from_top: 100mil },
        { name: "GND", io_type: bidirectional, side: left, distance_from_top: 300mil }
    ]
}

// Note (text annotation)
note {
    at: (400mil, 3450mil)
    text: "This section handles power input and regulation"
}

// Probe (test point marker)
probe "TP1" { location: (2000mil, 1500mil) }
```

### High-Level Net and Power Blocks

For declarative connectivity (alternative to explicit wires):

```
// Define a net by listing the pins it connects
net SDA {
    pins: ["U1.14", "U2.5", "R1.1"]
}

// Define a power net with style
power VCC_3V3 {
    style: bar
    pins: ["U1.1", "U2.8", "C1.1", "C2.1"]
}
```

### Import System for Cross-Library References

SchDoc specs can import SchLib specs to reference symbols:

```
import "my-parts.schlib-spec" as parts

component "U1" {
    lib_reference: $parts.ESP32_C6     // references the ESP32_C6 component from the import
    at: (3000mil, 2000mil)
}
```

---

## PCB Document Specs (`.pcbdoc-spec`)

PcbDoc specs describe PCB board documents — the physical board layout. This covers
**Stages 7-8** of the design workflow (stackup, constraints, placement).

### Board Configuration

```
board "" {
    signal_layer_count: 2              // number of copper layers
    snap_grid_size: 0.127mm            // snap grid
    visible_grid_size: 1mm             // display grid
    display_unit: "metric"             // "metric" or "imperial"
}
```

### Board Outline

```
geometry {
    outline {
        line (25.4mm, 25.4mm)
        line (100mm, 25.4mm)
        line (100mm, 75mm)
        line (25.4mm, 75mm)
    }
}
```

### Nets

```
net VCC { color: #FF0000, visible: true }
net GND { color: #00FF00, visible: true }
net SDA { color: #9EA175, visible: true }
```

### Components (Placement)

```
component U1 { pattern: "QFP-48", at: (50mm, 40mm), layer: TopLayer, rotation: 0 }
component R1 { pattern: "0603", at: (55mm, 35mm), layer: TopLayer, rotation: 90 }
component C1 { pattern: "0402", at: (48mm, 42mm), layer: BottomLayer, rotation: 0 }
```

### Primitives

```
// Track (copper trace)
track { layer: TopLayer, net: SDA, from: (50mm, 40mm), to: (55mm, 40mm), width: 0.254mm }

// Arc
arc { layer: TopOverlay, center: (50mm, 50mm), radius: 5mm, start_angle: 0, end_angle: 360, width: 0.2mm }

// Via
via { net: GND, at: (60mm, 40mm), diameter: 1.27mm, hole_size: 0.7mm, from_layer: TopLayer, to_layer: BottomLayer }

// Pad (board-level, not in footprint)
pad 1 { net: VCC, component: U1, at: (50mm, 40mm), layer: MultiLayer, shape: round, x_size: 1.5mm, y_size: 1.5mm }

// Text (silkscreen)
text { layer: TopOverlay, at: (45mm, 30mm), text: "REV A", height: 1.524mm }

// Fill (copper rectangle)
fill { layer: TopLayer, from: (40mm, 35mm), to: (60mm, 45mm) }

// Region
region { layer: TopLayer, kind: "copper" }

// Component body
component_body { layer: Mechanical13, component: U1 }
```

### Polygons (Copper Pours)

```
polygon "GND_POUR" { net: GND, layer: TopLayer, connect_style: "relief", pour_order: 0 }
polygon "PWR_POUR" { net: VCC, layer: BottomLayer, connect_style: "direct", pour_order: 1 }
```

### Design Rules

```
// Clearance
rule Clearance { kind: "clearance", enabled: true, priority: 1, gap: 0.254mm }

// Track width
rule Width { kind: "width", enabled: true, priority: 1, min: 0.254mm, max: 0.254mm, preferred: 0.254mm }

// Routing via style
rule RoutingVias { kind: "routing_via_style", enabled: true, priority: 1,
    width: 1.27mm..1.27mm (pref 1.27mm), hole: 0.7mm..0.7mm (pref 0.7mm) }

// Hole size constraints
rule HoleSize { kind: "max_min_hole_size", enabled: true, priority: 1, min: 0.2mm, max: 6.35mm }

// Solder mask
rule SolderMaskExpansion { kind: "solder_mask_expansion", enabled: true, priority: 1, expansion: 4mil }
rule PasteMaskExpansion { kind: "paste_mask_expansion", enabled: true, priority: 1, expansion: 0mil }

// Polygon connect
rule PolygonConnect { kind: "polygon_connect_style", enabled: true, priority: 1,
    connect_style: "relief", relief_width: 0.254mm, relief_entries: 4, air_gap: 0.254mm }

// Component clearance
rule ComponentClearance { kind: "component_clearance", enabled: true, priority: 1, gap: 0.254mm }

// Height limits
rule Height { kind: "max_min_height", enabled: true, priority: 1, min_height: 0mil, max_height: 25.4mm }

// Short circuit check
rule ShortCircuit { kind: "short_circuit", enabled: true, priority: 1, allowed: false }

// Unrouted nets
rule UnRoutedNet { kind: "broken_nets", enabled: true, priority: 1 }

// Routing topology
rule RoutingTopology { kind: "routing_topology", enabled: true, priority: 1, topology: "Shortest" }

// Routing corners
rule RoutingCorners { kind: "routing_corner_style", enabled: true, priority: 1, corner_style: "Degree45" }

// Differential pairs
rule DiffPairs { kind: "differential_pairs_routing", enabled: true, priority: 1,
    gap: 0.254mm..0.254mm (pref 0.254mm), max_uncoupled: 12.7mm }

// Hole-to-hole
rule HoleToHole { kind: "hole_to_hole_clearance", enabled: true, priority: 1, gap: 0.254mm }

// Silk clearances
rule SilkToMask { kind: "silk_to_solder_mask_clearance", enabled: true, priority: 1, gap: 0.254mm }
rule SilkToSilk { kind: "silk_to_silk_clearance", enabled: true, priority: 1, gap: 0.254mm }
```

### Net Classes

```
class "All Nets" { kind: "net" }
class "Power Nets" { kind: "net" }
class "Signal Layers" { kind: "layer" }
class "All Components" { kind: "component" }
class "Top Side Components" { kind: "component" }
class "All Differential Pairs" { kind: "differential_pair" }
```

### Differential Pairs

```
differential_pair "USB_DP" { positive_net: "USB_D+", negative_net: "USB_D-" }
```

### Placement Solver

The spec language includes a placement solver for automated component placement:

```
placement {
    target: "board.PcbDoc"

    place U1 { at: (50mm, 40mm), fixed: true }          // fixed position
    place U2 { near: "U1", max_distance: 10mm }          // proximity constraint
    place C1 { region_rect: ((40mm, 35mm), (60mm, 45mm)) } // within region
    place R1, R2 { edge: "top", inset: 5mm }             // along board edge

    constraints {
        left_of { a: "C1", b: "U1", gap: 2mm }          // relative positioning
        above { a: "R1", b: "U1" }
    }

    clearance { all: 0.5mm, edge: 1mm }
    optimize { ratsnest: true, ratsnest_weight: 1.0 }
}
```

---

## Project Specs (`.prjpcb-spec`)

PrjPcb specs configure Altium projects — design settings, document lists, ERC matrix,
output jobs, and variants.

```
project "MyProject" {
    hierarchy_mode: flat
    output_path: "Project Outputs"
    allow_port_net_names: true
    allow_sheet_entry_net_names: true

    document "Main.SchDoc" {
        annotation_enabled: true
        annotate_start_value: 1
    }

    document "Board.PcbDoc" {}

    annotation {
        sort_order: down
        sort_location: global
    }

    erc_matrix {
        (output, output): error
        (input, open_collector): warning
    }

    erc_levels {
        "Duplicate Sheet Symbol Entries": fatal
        "Missing Obligatory Parameter": error
    }

    output_group "Fabrication" {
        output "Gerber" { output_type: "Gerber", document_path: "Board.PcbDoc" }
        output "Drill" { output_type: "NC Drill", document_path: "Board.PcbDoc" }
    }

    variant "Production" {
        description: "Production variant"
        variation "R5" { kind: not_fitted }
        variation "J3" { kind: alternate, alternate_part: "JST-XH-3P" }
        param_variation "R1" { parameter: "Value", value: "4.7K" }
    }
}
```

---

## Import System

Spec files can import other spec files for reuse and composition.

### Named Imports

```
// Import a footprint library and give it an alias
import "standard-footprints.pcblib-spec" as footprints

// Reference entities from the import
component MCU {
    footprint $footprints.QFP48 {
        map { pin: 1, pad: 1 }
        // ...
    }
}
```

### Bare Imports

```
// Merge all entities from another spec into this scope
import "common-passives.schlib-spec"
```

### Import Rules

- Paths are relative to the importing file
- Circular imports are detected and rejected
- Cross-domain imports are forbidden (SchLib cannot import PcbLib)
- Imports are deduplicated and cached

---

## Complete Example: Footprint Library

This example demonstrates pad templates, row layouts, and various package types in a
single `.pcblib-spec` file.

```
// my-footprints.pcblib-spec

// ============================================================================
// Shared pad templates
// ============================================================================

let smd = { layer: "TopLayer", pad_mode: simple, is_plated: false, hole_size: 0 }
let th = { layer: "MultiLayer", pad_mode: simple, is_plated: true }

let pad_0603 = { ...smd, shape: rectangular, x_size: 0.8mm, y_size: 0.9mm }
let pad_0805 = { ...smd, shape: rectangular, x_size: 1.0mm, y_size: 1.3mm }

let silk = 5mil

// ============================================================================
// 2-terminal passive (0603)
// ============================================================================

footprint "0603" {
    description: "0603 (1608 Metric) 2-pad SMD"
    height: 0.55mm

    pad 1 { ...pad_0603, at: (-0.75mm, 0) }
    pad 2 { ...pad_0603, at: (0.75mm, 0) }

    outline = polyline {
        points: [(-1.3mm, -0.65mm), (1.3mm, -0.65mm), (1.3mm, 0.65mm), (-1.3mm, 0.65mm)]
        width: silk, layer: "TopOverlay", closed: true
    }
}

// ============================================================================
// SOT-23-5
// ============================================================================

footprint "SOT-23-5" {
    description: "SOT-23, 5-lead, 0.95mm pitch"
    height: 1.45mm

    let sp = { ...smd, shape: rectangular, x_size: 0.6mm, y_size: 0.5mm }

    pad 1 { ...sp, at: (-0.95mm, -0.65mm) }
    pad 2 { ...sp, at: (-0.95mm, 0.65mm) }
    pad 3 { ...sp, at: (0.95mm, 0.65mm) }
    pad 4 { ...sp, at: (0.95mm, 0) }
    pad 5 { ...sp, at: (0.95mm, -0.65mm) }

    outline = polyline {
        points: [(-1.5mm, -1.15mm), (1.5mm, -1.15mm), (1.5mm, 1.15mm), (-1.5mm, 1.15mm)]
        width: silk, layer: "TopOverlay", closed: true
    }
    pin1_mark = arc {
        center: (-1.7mm, -0.65mm), radius: 0.1mm
        start_angle: 0, end_angle: 360
        width: silk, layer: "TopOverlay"
    }
}

// ============================================================================
// SOIC-8 with exposed pad (row layout)
// ============================================================================

footprint "SOIC-8-PAD" {
    description: "SOIC-8 with exposed PowerPAD, 1.27mm pitch"
    height: 1.75mm

    let sp = { ...smd, shape: rectangular, x_size: 0.6mm, y_size: 1.5mm }
    let body = { from: (-2.0mm, -2.5mm), to: (2.0mm, 2.5mm) }

    // Left side: pins 1-4
    row {
        on: $body.left, at: center
        pitch: 1.27mm, count: 4, start: 1
        side: outside
        pad: { ...sp }
    }

    // Right side: pins 5-8 (reversed = bottom to top)
    row {
        on: $body.right, at: center
        pitch: 1.27mm, count: 4, start: 5
        side: outside, direction: reverse
        pad: { ...sp }
    }

    // Exposed thermal pad
    pad 9 { ...smd, at: (0, 0), shape: rectangular, x_size: 3.0mm, y_size: 2.4mm }

    outline = polyline {
        points: [(-2.0mm, -2.5mm), (2.0mm, -2.5mm), (2.0mm, 2.5mm), (-2.0mm, 2.5mm)]
        width: silk, layer: "TopOverlay", closed: true
    }
    pin1_mark = arc {
        center: (-2.3mm, -2.0mm), radius: 0.15mm
        start_angle: 0, end_angle: 360
        width: silk, layer: "TopOverlay"
    }
}

// ============================================================================
// Through-hole connector (row + pad override)
// ============================================================================

footprint "CONN-4P" {
    description: "4-pin, 2.5mm pitch, through-hole connector"
    height: 7.0mm

    row {
        at: (-3.75mm, 0)
        pitch: 2.5mm, count: 4, start: 1
        direction: right
        pad: { ...th, shape: round, x_size: 1.6mm, y_size: 1.6mm, hole_size: 1.0mm }
    }

    pad 1 { shape: rectangular }   // override: pin 1 is rectangular

    outline = polyline {
        points: [(-6.25mm, -2.5mm), (6.25mm, -2.5mm), (6.25mm, 7.0mm), (-6.25mm, 7.0mm)]
        width: silk, layer: "TopOverlay", closed: true
    }
}
```

---

## Complete Example: Schematic Sheet

This example shows a simple MCU schematic with components, power rails, nets, and wiring.

```
// controller.schdoc-spec

sheet {
    custom_width: 6200mil
    custom_height: 3600mil
    snap_grid_on: true
    visible_grid_on: true
    border_on: true
    title_block_on: true
}

// Components
component "J1" { lib_reference: "USB-C", at: (500mil, 3000mil), description: "USB connector" }
component "U1" { lib_reference: "LDO-3V3", at: (1700mil, 3000mil), description: "3.3V LDO" }
component "U2" { lib_reference: "MCU", at: (3200mil, 2600mil), description: "Microcontroller" }
component "R1" { lib_reference: "R", at: (2500mil, 2000mil), orientation: 90 }
component "C1" { lib_reference: "C", at: (1700mil, 2200mil), orientation: 0 }

// Power rails
power_object "VBUS" { at: (500mil, 3350mil) }
power_object "VCC_3V3" { at: (1700mil, 3350mil) }
power_object "GND" { at: (1700mil, 1700mil) }

// Signal net labels
net_label "SDA" { at: (3800mil, 2800mil) }
net_label "SCL" { at: (3800mil, 2400mil) }

// Wiring
wire { vertices: [(500mil, 3350mil), (500mil, 3000mil), (1700mil, 3000mil)] }
wire { vertices: [(1700mil, 3350mil), (1700mil, 3000mil), (3200mil, 2900mil)] }
wire { vertices: [(3200mil, 2800mil), (3800mil, 2800mil)] }
wire { vertices: [(3200mil, 2400mil), (3800mil, 2400mil)] }

// Annotation
note {
    at: (400mil, 3450mil)
    text: "Simple MCU controller with USB, LDO, and I2C bus"
}
```

---

## Mapping to Design Workflow Stages

Here is how the spec language maps to the stages defined in `instructions.md`:

| Workflow Stage                              | Spec Type        | What to Generate                                                |
| ------------------------------------------- | ---------------- | --------------------------------------------------------------- |
| **Stage 4: Symbol generation**              | `.schlib-spec`   | Components with pins, graphics, parameters, footprint maps      |
| **Stage 4: Footprint generation**           | `.pcblib-spec`   | Footprints with pads, silk outlines, courtyard, pin 1 markers   |
| **Stage 5: Schematic capture**              | `.schdoc-spec`   | Components, power rails, nets, wires, notes                     |
| **Stage 7: Stackup & constraints**          | `.pcbdoc-spec`   | Board settings, layer count, design rules, net classes           |
| **Stage 8: Placement**                      | `.pcbdoc-spec`   | Component placement with solver constraints                     |
| **Stage 14: Iteration (ECOs)**              | `altium plan`    | Preview changes before applying                                 |
| **Project configuration**                   | `.prjpcb-spec`   | Documents, ERC matrix, output jobs, variants                    |

### Typical Agent Workflow

1. **Research parts** (LLM reads datasheets) and produce a `.schlib-spec` with symbols
2. **Create footprints** from datasheet package drawings → `.pcblib-spec`
3. **Verify libraries**: `altium plan parts.schlib-spec` and `altium plan footprints.pcblib-spec`
4. **Build libraries**: `altium apply parts.schlib-spec` and `altium apply footprints.pcblib-spec`
5. **Render for review**: `altium render parts.SchLib` → SVG/PNG of each symbol
6. **Capture schematic**: write `.schdoc-spec` placing components and wiring nets
7. **Build schematic**: `altium apply schematic.schdoc-spec`
8. **Render schematic**: `altium render schematic.SchDoc` → SVG of full sheet
9. **Configure board**: write `.pcbdoc-spec` with rules, constraints, nets
10. **Apply to board**: `altium apply board.pcbdoc-spec --target board.PcbDoc`
11. **Run placement solver**: `altium placement solve --target board.PcbDoc`
12. **Iterate**: modify specs and re-apply; `altium plan` shows diffs

### Reverse Engineering Existing Libraries

To bootstrap specs from existing Altium files:

```bash
# Generate spec from existing SchLib
altium dump vendor-parts.SchLib --output vendor-parts.schlib-spec

# Generate spec from existing PcbLib
altium dump vendor-footprints.PcbLib --output vendor-footprints.pcblib-spec

# Generate spec from existing PcbDoc (for board settings, rules, nets)
altium dump existing-board.PcbDoc --output board.pcbdoc-spec
```

The dumped specs use absolute coordinates and non-default properties only. They can
be used as-is or refactored to use let bindings, pad templates, and row/grid layouts.

---

## Appendix: Enum Reference

### Pin Electrical Types
`input`, `output`, `input_output`, `open_collector`, `open_emitter`, `passive`, `hi_z`, `power`

### Component Kinds
`standard`, `mechanical`, `graphical`, `net_tie_bom`, `net_tie_no_bom`, `jumper`

### Pad Shapes
`round`, `rectangular`, `octagonal`, `round_rect`, `rotated_rect`, `custom`, `arc`, `terminator`

### Pad Stack Modes
`simple`, `local_stack`, `external_stack`

### Plane Connection Styles
`relief`, `direct`, `no_connect`

### Power Object Styles
`bar`, `arrow`, `wave`, `ground`, `power_ground`, `earth`, `signal_ground`,
`gost_arrow`, `gost_bar`, `gost_earth`, `gost_power_ground`

### Port IO Types
`unspecified`, `output`, `input`, `bidirectional`

### Line Styles
`solid`, `dashed`, `dotted`

### Pen Widths
`smallest`, `small`, `medium`, `large`

### DRC Rule Kinds
`clearance`, `short_circuit`, `broken_nets`, `width`, `routing_via_style`,
`routing_corner_style`, `routing_topology`, `routing_layers`, `routing_priority`,
`power_plane_connect_style`, `power_plane_clearance`, `polygon_connect_style`,
`paste_mask_expansion`, `solder_mask_expansion`, `component_clearance`,
`max_min_height`, `max_min_hole_size`, `layer_pair`, `hole_to_hole_clearance`,
`minimum_solder_mask_sliver`, `silk_to_solder_mask_clearance`,
`silk_to_silk_clearance`, `silk_to_board_region_clearance`, `net_antennae`,
`unpoured_polygon`, `fabrication_testpoint_usage`, `fabrication_testpoint_style`,
`assy_test_point_usage`, `assy_test_point_style`, `differential_pairs_routing`,
`fanout_control`, `confinement_constraint`

### Layer Names
**Copper:** `TopLayer`, `MidLayer1`-`MidLayer30`, `BottomLayer`, `MultiLayer`
**Silk:** `TopOverlay`, `BottomOverlay`
**Mask:** `TopSolderMask`, `BottomSolderMask`, `TopPasteMask`, `BottomPasteMask`
**Mechanical:** `Mechanical1`-`Mechanical16`
**Other:** `DrillGuide`, `DrillDrawing`, `KeepOutLayer`
