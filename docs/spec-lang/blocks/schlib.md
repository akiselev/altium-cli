# Schematic Library (`.schlib-spec`)

A `.schlib-spec` file describes the symbols in an Altium schematic library
(`SchLib`). Its top-level blocks are `component` declarations (plus optional
file-level `swap_group` declarations, `import` directives, and `let` bindings).
Each `component` compiles to one `SchLib` component: a `lib_reference`, its pins,
parameters, footprint models, and symbol graphics.

**Related pages:** [Blocks overview](../language/blocks-overview.md) ·
[Types and values](../language/types-and-values.md) ·
[PcbLib blocks](pcblib.md) · [Annotations](../language/annotations.md) ·
[Altium mapping reference](../reference/altium-mapping.md)

---

## `component`

The top-level block of a schematic library. The block name becomes the
component's library reference.

```hcl
[binding =] component NAME {
    designator: "..."
    description: "..."
    # pins, parameters, aliases, footprint maps, graphics, parts, swap_groups
}
```

The block name (`NAME`) may be a bare identifier, a quoted string, or an
integer; it is stored as `lib_reference`. An optional `#[annotation(...)]`
attribute may precede the block for sync stability (see
[Annotations](../language/annotations.md)).

| Name | Type | Required | Description |
| --- | --- | --- | --- |
| `designator` | string | No | Default designator prefix (e.g. `"R"`, `"U?"`). Compiles to `ComponentSpec.designator`. |
| `description` | string | No | Component description / comment. |
| `component_kind` | string | No | Component kind keyword. Compiles via `parse_component_kind` to `ComponentKind` (e.g. `standard`). |
| `part_count` | integer | No | Number of parts (gates) in a multi-part component. If omitted, inferred from `part N { }` blocks. |
| `show_hidden_pins` | bool | No | Whether hidden pins are shown. |

All other content is supplied by the child blocks below (`pin`, `parameter`,
`alias`, `footprint`, `part`, `swap_group`, and graphic blocks).

**Maps to Altium:** one entry in the `SchLib` component list. `compile_component`
(in `src/compiler.rs`) builds a `ComponentSpec`; `component_from_spec` (in
`src/executor.rs`) turns it into an `api::Component` with `lib_reference`,
`designator`, `description`, `component_kind`, and `part_count`. When the
component already exists, fields are merged additively (`Some` overrides, `None`
preserves).

### Example

```hcl
component R_0603 {
    designator: "R"
    description: "SMD resistor 0603"
}
```

---

## `pin`

Declares a pin on the component (or on a `part`). The block name is the pin's
**designator** (the number/identifier printed at the pin); the human-readable
`name` is a property.

```hcl
[binding =] pin DESIGNATOR {
    name: "..."
    at: (x, y)            # or x:/y:
    orientation: "..."
    electrical: "..."
    length: <dim>
    # ... anchor keys, swap groups
}
```

| Name | Type | Required | Description |
| --- | --- | --- | --- |
| `name` | string | No | Pin name (logical signal name). Omitted ⇒ empty string. Compiles to `PinSpec.name`. |
| `at` | tuple `(x, y)` | No | Pin location. Alternative to `x:`/`y:`. Defaults to `(0, 0)`. |
| `x`, `y` | dim | No | Individual coordinate components if `at` is not used. |
| `orientation` | string | No | Pin rotation. Accepts `0`/`rotate0`/`right`/`east`, `90`/`up`/`north`, `180`/`left`/`west`, `270`/`down`/`south`. Defaults to `Rotate0`. |
| `electrical` | string | No | Electrical type. See accepted values below. Defaults to `Passive` on apply. |
| `length` | dim | No | Pin stub length. Defaults to 25 mil on apply. |
| `is_hidden` | bool | No | Hide the pin. |
| `hidden_net_name` | string | No | Net auto-connected when the pin is hidden. |
| `swap_group` | `$ref` or string | No | Pin-swap group ID (`swap_id_pin`). |
| `part_swap_group` | `$ref` or string | No | Part/gate-swap group ID (`swap_id_part`). |
| `pair_swap_group` | `$ref` or string | No | Differential-pair swap group ID (`swap_id_pair`). |

**Anchor keys** (used to position pins relative to a bound box graphic instead
of by absolute `at`): `on: $body.left`, `at: "start"|"center"|"end"`,
`after: $otherPin`, `before: $otherPin`, and `gap: <dim>`. `at`, `after`, and
`before` are mutually exclusive in anchor mode. See
[Placement](placement.md) and `resolve_anchor_pins` in `src/compiler.rs`.

Accepted `electrical` values (case-insensitive, from `parse_pin_electrical_type`):
`input`; `input_output`/`inputoutput`/`io`/`bidir`; `output`;
`open_collector`/`opencollector`/`oc`; `passive`; `hiz`/`hi_z`/`tristate`;
`open_emitter`/`openemitter`/`oe`; `power`.

If no explicit `binding =` is given, a pin gets an implicit binding derived from
its designator (`pin 1` → `$pin1`, `pin SDA` → `$pinSDA`) for use by anchor
references.

**Maps to Altium:** an `api::Pin` built by `pin_from_spec` in `src/executor.rs`.
`name` → pin name, `electrical` → `PinElectricalType`, `at` → `location`,
`length`, `orientation`, `is_hidden`, `hidden_net_name`, and the three swap
groups → `swap_id_pin`/`swap_id_part`/`swap_id_pair`. `owner_part_id` is set
from the enclosing `part` (or part 1 for single-part components).

### Example

```hcl
component R {
    pin 1 {
        at: (100mil, 0mil)
        orientation: "0"
    }
    pin 2 {
        at: (-100mil, 0mil)
        orientation: "180"
    }
}
```

> Coordinates use the dimension grammar described in
> [Types and values](../language/types-and-values.md). `100mil` compiles to the
> internal Altium unit `1_000_000` (10,000 internal units per mil).

---

## `part`

Splits a component into multiple parts (gates). Component-level pins and
graphics are shared (owner part 0); pins/graphics inside a `part N { }` block
belong to part `N`.

```hcl
[binding =] part N {
    pin "..." { ... }
    # graphics, properties, let bindings
}
```

| Item | Description |
| --- | --- |
| `pin` | A pin owned by this part (`owner_part_id = N`). |
| graphic blocks | Part-local symbol graphics (e.g. `body = rectangle { ... }`). |
| `swap_group` / `part_swap_group` property | Applied to every pin in the part that does not set its own `part_swap_group`. |
| `let` bindings | Part-scoped values. |

A component is treated as multi-part when `part_count > 1` **or** any `part N { }`
block is present. Graphic unique IDs become part-scoped
(`spec:{component}:part{N}:{name}`).

**Maps to Altium:** `compile_part_with_anchors` produces a `PartSpec`
(`part_number`, `pins`, `graphics`); `component_from_spec` flattens all parts'
pins into the component, each tagged with its `owner_part_id`. `part_count` is
inferred as the max part number when not stated.

### Example

```hcl
component LM358 {
    part 1 {
        pin "IN+" { at: (0mil, 0mil) }
        pin "IN-" { at: (0mil, -50mil) }
        pin "OUT" { at: (100mil, -25mil) }
    }
    part 2 {
        pin "IN+" { at: (0mil, 0mil) }
        pin "OUT" { at: (100mil, -25mil) }
    }
}
```

---

## `parameter`

A named string parameter (BOM/metadata field) on the component. The block name
is the parameter name.

```hcl
[binding =] parameter NAME {
    text: "..."     # alias: value:
    is_hidden: <bool>
}
```

| Name | Type | Required | Description |
| --- | --- | --- | --- |
| `text` | string | No | Parameter value. The key `value:` is accepted as a synonym; default is empty. |
| `is_hidden` | bool | No | Whether the parameter is hidden on the sheet. |

**Maps to Altium:** `compile_parameter` → `ParameterSpec`; `param_from_spec`
in `src/executor.rs` → `api::Parameter` (`name`, `text`, `is_hidden`).

### Example

```hcl
component R_0603 {
    parameter "Value" { text: "10k" }
    parameter "Tolerance" { text: "1%", is_hidden: false }
}
```

---

## `alias`

Declares an alternate library name (alias) for the component. It has no body.

```hcl
alias NAME
```

| Name | Type | Required | Description |
| --- | --- | --- | --- |
| `NAME` | identifier or string | Yes | The alias name. |

**Maps to Altium:** `compile_alias` returns the alias string; it is appended to
`ComponentSpec.aliases`, which becomes the component's alias list.

### Example

```hcl
component RES {
    alias "R"
    alias RESISTOR
}
```

---

## `footprint` (footprint map)

Associates a PCB footprint model with the component and, optionally, remaps pins
to pads. Two forms exist:

```hcl
# Implicit 1:1 mapping — pin N maps to pad N for all pads.
footprint MODEL_REF

# Explicit remapping and/or description.
footprint MODEL_REF {
    description: "..."
    pin "PIN": pad "PAD"
    # ...more pin:pad pairs
}
```

The `MODEL_REF` may be a bare name/string or a `$path` reference (e.g. a `let`
binding that resolves to a footprint from an imported `.pcblib-spec`).

| Name | Type | Required | Description |
| --- | --- | --- | --- |
| `description` | string | No | Footprint model description. |
| `pin X: pad Y` | pair | No | Explicit pin→pad mapping. Each side may be a literal (`pin "1"`) or a `$path` reference. When all pairs are absent, the mapping is implicit 1:1. |

**Maps to Altium:** `compile_footprint_map` → `FootprintMapSpec` (`model_name`,
`maps: Vec<PinPadMap>`, `description`). `footprint_from_spec` in `src/executor.rs`
produces an `api::FootprintMap`. An implicit footprint compiles to an empty
`maps` vec (the binary writer treats this as "all pins map to same-numbered
pads").

### Example

```hcl
component R_0603 {
    footprint "R_0603_SMD"
}

component U1 {
    footprint "SOIC-8" {
        description: "8-lead SOIC"
        pin "VCC": pad "8"
        pin "GND": pad "4"
    }
}
```

---

## `swap_group`

Declares a named swap group so pins can reference it by binding (`$name`)
instead of repeating the literal string. May appear at file level or inside a
`component`.

```hcl
[binding =] swap_group NAME { }
```

The declaration registers `NAME` (and any `binding`) in scope as a swap-group
reference value. It does **not** itself emit anything into the component model —
the binding is consumed where a pin references it via `swap_group: $NAME` (and
the dump emits `swap_group NAME {}` declarations for groups shared by 2+ pins).

| Name | Type | Required | Description |
| --- | --- | --- | --- |
| `NAME` | identifier / string | Yes | The swap-group identifier registered in scope. |

**Maps to Altium:** there is no standalone swap-group record. The group name
flows into the referencing pins' `swap_id_pin` / `swap_id_part` / `swap_id_pair`
fields (see `pin` above). `get_swap_group_opt` in `src/compiler.rs` accepts both
a `$ref` (`Value::SwapGroup`) and a plain string.

### Example

```hcl
component U_GATES {
    swap_group GATE_A {}
    part 1 {
        pin 1 { name: "A", swap_group: $GATE_A }
        pin 2 { name: "B", swap_group: $GATE_A }
    }
}
```

---

## Symbol graphics

Inside a `component` (or `part`) you can draw the symbol body using graphic
blocks. The recognized schematic graphic types (`SCH_GRAPHIC_TYPES` in
`src/ast.rs`) are: `line`, `rectangle`, `arc`, `elliptical_arc`, `ellipse`,
`polyline`, `polygon`, `bezier`, `pie`, `round_rectangle`, `label`,
`text_frame`, `image`.

```hcl
[binding =] GRAPHIC_TYPE {
    from: (x, y)
    to: (x, y)
    # type-specific properties (center, radius, points, text, color, ...)
}
```

A bound box-type graphic (`rectangle`, `round_rectangle`, `text_frame`, `image`)
exposes named edges (`$body.left`, `$body.right`, `$body.top`, `$body.bottom`)
that anchored pins can reference. Box graphics with omitted `from`/`to` are
auto-sized to enclose their pins. Each graphic receives a unique ID of the form
`spec:{component}[:part{N}]:{binding-or-type_index}`.

The full property set is defined by `GraphicProperties` in `src/model.rs`
(coordinates, `center`/`radius`/`start_angle`/`end_angle`, `points`, `color`,
`area_color`, `line_width`, `text`, `font_id`, `file_name`, `image_data`, etc.).

**Maps to Altium:** `compile_sch_graphic` → `GraphicSpec`, applied as the
corresponding schematic primitive (rectangle, line, arc, label, …).

### Example

```hcl
component R_0603 {
    body = rectangle {
        from: (0mil, 0mil)
        to: (100mil, 50mil)
    }
}
```

---

## Complete worked example

```hcl
#[annotation(id = "CMP00000A")]
component LM358 {
    designator: "U"
    description: "Dual operational amplifier"
    part_count: 2

    swap_group OPAMP {}

    part 1 {
        body = rectangle { from: (0mil, 0mil), to: (300mil, -200mil) }
        pin "IN+"  { on: $body.left,  at: "start",  electrical: "input"  }
        pin "IN-"  { on: $body.left,  after: $pinIN+, electrical: "input" }
        pin "OUT"  { on: $body.right, at: "center", electrical: "output", swap_group: $OPAMP }
    }
    part 2 {
        body = rectangle { from: (0mil, 0mil), to: (300mil, -200mil) }
        pin "IN+"  { on: $body.left,  at: "start",  electrical: "input"  }
        pin "OUT"  { on: $body.right, at: "center", electrical: "output", swap_group: $OPAMP }
    }

    parameter "Value"        { text: "LM358" }
    parameter "Manufacturer" { text: "TI", is_hidden: true }

    alias "LM358D"

    footprint "SOIC-8" {
        description: "8-lead SOIC"
        pin "OUT":  pad "1"
        pin "IN-":  pad "2"
        pin "IN+":  pad "3"
    }
}
```

This compiles (`compile_component`) to one `ComponentSpec` with two parts, three
parameters/aliases, and one footprint map, and applies (`apply_spec_schlib`) as
a single `SchLib` component.
