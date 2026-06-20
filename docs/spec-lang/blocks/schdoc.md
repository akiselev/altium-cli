# Schematic document blocks (`.schdoc-spec`)

Blocks that describe a schematic *sheet*: its metadata and fonts, placed component
instances, declared nets and power rails, pin-to-net connections, and low-level
schematic objects.

**Related pages:** [Blocks overview](../language/blocks-overview.md) ·
[`.schlib-spec`](schlib.md) · [`.pcbdoc-spec`](pcbdoc.md) ·
[Annotations](../language/annotations.md) ·
[Apply and plan](../operations/apply-and-plan.md) ·
[Altium mapping](../reference/altium-mapping.md)

A `.schdoc-spec` file compiles to a `SchDocSpec` containing a single `SheetSpec`
(`src/model.rs`). The top-level items recognised in this domain are `sheet`,
`component`, `net`, `power`, and the identifier-dispatched SchDoc object blocks
(`wire`, `net_label`, `power_object`, …).

> **Fail-fast gap:** `compile_schdoc` currently skips top-level items from other
> domains (`board`, `placement`, `rule`, …). Do not mix document domains in one
> file or rely on that behavior; accepting and dropping a parsed declaration
> violates the repository's fail-fast rule and is tracked in `STATUS.md`.

---

## `sheet`

Sheet-level metadata, fonts, and placement constraints. A spec has at most one
`sheet` block; its properties populate the `SheetSpec` scalar fields.

```
sheet {
    style: "A4"
    border_on: true
    fonts { font 1 { name: "Times New Roman", size: 10 } }
    constraint edge_placement { designator: "U1", edge: "top" }
}
```

| Name | Type | Required | Description |
| --- | --- | --- | --- |
| `style` | string | No | Standard sheet size: `A4`, `A3`, `A2`, `A1`, `A0`, `A`, `B`, `C`, `D`, `E`, `Letter`, `Legal`, `Tabloid`, `OrcadA`–`OrcadE`. Mutually exclusive with `custom_width`/`custom_height`. |
| `custom_width` | dimension | No | Custom sheet width (implies a custom sheet size). |
| `custom_height` | dimension | No | Custom sheet height. |
| `snap_grid_on` | bool | No | Snap grid enabled. |
| `visible_grid_on` | bool | No | Visible grid enabled. |
| `hot_spot_grid_on` | bool | No | Hot-spot grid enabled. |
| `show_hidden_pins` | bool | No | Show hidden pins. |
| `border_on` | bool | No | Draw the sheet border. |
| `title_block_on` | bool | No | Draw the title block. |
| `fonts` | block | No | Font table — see [`fonts`](#fonts). |
| `constraint` | block | No | Placement constraint — see [`constraint`](#constraint). Repeatable. |

**Maps to Altium:** parsed by `compile_sheet_metadata` into `SheetSpec` fields
(`sheet_style`, `custom_width`, `custom_height`, the `*_on` booleans). `style` is
resolved by `parse_sheet_style`; an unknown name is a `TypeMismatch` error. These
map to the SchDoc `Sheet` record properties (sheet size, grid, border flags).

---

### `fonts`

A font table inside `sheet`. Each `font N { … }` defines one entry keyed by the
integer font id.

```
sheet {
    fonts {
        font 1 { name: "Times New Roman", size: 10 }
        font 2 { name: "Arial", size: 8, bold: true }
    }
}
```

| Name | Type | Required | Description |
| --- | --- | --- | --- |
| `name` | string | Yes | Font face name. |
| `size` | integer | No | Point size (defaults to `10`). |
| `bold` | bool | No | Bold. |
| `italic` | bool | No | Italic. |
| `underline` | bool | No | Underline. |
| `strikeout` | bool | No | Strikeout. |
| `rotation` | integer | No | Text rotation in degrees. |

**Maps to Altium:** `compile_font` produces a `FontSpec { id, name, size, bold,
italic, underline, strikeout, rotation }`. These feed the SchDoc `FontIDn…`
header parameters; `font_id` references on objects (net labels, ports, notes)
index into this table.

---

### `constraint`

A placement constraint declared inside `sheet`. The constraint *kind* is a typed
keyword (typos are rejected at parse time); the body is a free key/value object
stored verbatim.

```
sheet {
    constraint edge_placement  { designator: "U1", edge: "top" }
    constraint directional     { a: "U1", b: "U2", direction: "left_of", gap: 5mm }
    constraint near            { a: "U1", b: "U2", max_distance: 10mm }
    constraint region          { designator: "U1", min_x: 0mm, min_y: 0mm, max_x: 50mm, max_y: 50mm }
    constraint fixed_position  { designator: "U1", x: 25mm, y: 30mm }
}
```

| Kind keyword | `ConstraintKind` | Typical body keys |
| --- | --- | --- |
| `edge_placement` | `EdgePlacement` | `designator`, `edge` |
| `directional` | `Directional` | `a`, `b`, `direction`, `gap` |
| `near` | `Near` | `a`, `b`, `max_distance` |
| `region` | `Region` | `designator`, `min_x`, `min_y`, `max_x`, `max_y` |
| `fixed_position` | `FixedPosition` | `designator`, `x`, `y` |

**Maps to Altium:** `compile_constraint_decl` produces a `ConstraintSpec { kind,
properties }` where `properties` is an `IndexMap<String, String>` of the body
evaluated to string representations — the body keys are *not* further validated by
the compiler. A `constraint` block may carry an `#[annotation(...)]`. These are
solver/placement hints carried in the spec model; they are not direct Altium
record fields. An unrecognised kind keyword is a parse error.

---

## `component` (placed instance)

A placed schematic component instance. Unlike the `component` block in a
`.schlib-spec` (which *defines* a symbol), a `.schdoc-spec` `component` places an
instance of a library symbol at a location and may attach pin connections and
parameters.

```
component U1 {
    symbol: $mcu.ESP32_C6
    at: (100mil, 200mil)
    orientation: 90
    is_mirrored: false
    description: "Main MCU"
    pin GPIO4 -> #SDA
    pin VDD   -> #VDD3V3
    pin NC1   -> nc
}
```

| Name | Type | Required | Description |
| --- | --- | --- | --- |
| `symbol` | import ref / string | No | Library symbol. `$alias.Name` resolves against an imported `.schlib-spec` and is validated at compile time; a plain string is an unvalidated `lib_reference`. Defaults to the designator. |
| `at` | point | No | Placement location `(x, y)`. Defaults to `(0, 0)`. |
| `orientation` | rotation | No | `0`/`90`/`180`/`270`. |
| `is_mirrored` | bool | No | Horizontal mirror. |
| `description` | string | No | Component description. |
| `parameter NAME { … }` | block | No | BOM/visible parameter. Repeatable. |
| `pin NAME -> …` | statement | No | Pin connection — see [pin connections](#pin-connections). Repeatable. |

**Maps to Altium:** `compile_schdoc_component` produces a `SchDocComponentSpec`.
The `symbol:` value resolves to `SymbolRef::Import { alias, name }` (validated
against `imported_components`) or `SymbolRef::Literal(String)`; see the
`Value::ImportRef` rationale in the crate README. The component lowers to a SchDoc
`Component` record with `Location`, `Orientation`, and `IsMirrored`; `parameter`
blocks become child `Parameter` records.

---

## Pin connections (`pin X -> #NET` / `pin X -> nc`)

A pin connection is declared *inside* a placed `component`. It connects a named
pin of the component to a net (`#NET`) or marks it as a no-connect (`nc`).

```
component U1 {
    at: (0mil, 0mil)
    pin GPIO4 -> #SDA      // signal net  → NetLabel
    pin VDD   -> #VDD3V3   // power net    → PowerObject
    pin NC1   -> nc        // no-connect   → NoConnect marker
}
```

| Form | `PinConnectionTarget` (AST) | Compiled target |
| --- | --- | --- |
| `pin X -> #NAME` (signal) | `NetRef("NAME")` | `Signal("NAME")` |
| `pin X -> #NAME` (power) | `NetRef("NAME")` | `Power("NAME")` |
| `pin X -> nc` | `NoConnect` | `NoConnect` |

### Power-vs-NetLabel classification

The compiler runs a **pre-pass** over the whole file collecting every `power {}`
block name before compiling any component. During component compilation, a
`#NAME` target is classified as `Power` if `NAME` matches a key in
`power_declarations`; otherwise it is `Signal`
(`compile_schdoc_component` in `src/compiler.rs`). This makes classification
order-independent — a `pin -> #VCC` is treated as power even if the
`power VCC {}` block appears later in the file.

### Expansion at apply time

Pin connections are **not** collapsed to coordinates during compilation. The
executor's `generate_pin_connection_stubs` resolves them at apply time, because it
needs live access to the imported SchLib to look up pin positions
(`resolve_pin` matches pin *name* first, then *designator*). For each connection
it:

1. Computes the pin tip via `transform_pin_position` (component location +
   placement transform).
2. Transforms the pin orientation via `transform_pin_orientation` (mirror, then
   add component rotation mod 360 — see below).
3. Emits Altium objects by target:
   - **`Signal`** → a `Wire` stub (200 mil) from the pin tip plus a `NetLabel` at
     the stub end.
   - **`Power`** → a `Wire` stub plus a `PowerObject` (style from
     `power_declarations`, defaulting to `Bar`).
   - **`NoConnect`** → a `NoConnect` marker at the pin tip (no wire stub).

### Orientation conventions

- **Pin orientation transform is mirror-then-rotate.** Mirror flips 0°↔180° (90°
  and 270° unchanged), then the component rotation is added modulo 4 quarter-turns
  (`transform_pin_orientation`). Reversing the order produces wrong stub
  directions. Example: pin 0° (right) + mirror + rotation 90° → flips to 180°
  (left) → +90° = 270° (down); the stub extends downward.
- **NetLabel orientation is 0° or 90° only, never 180°/270°.**
  `remap_label_orient` collapses `Rotate180 → Rotate0` and `Rotate270 → Rotate90`
  so label text never reads backward or upside-down. Anchor direction for
  left/down stubs is handled by justification instead.
- **PowerObject orientation matches the stub direction directly.** A
  `PowerObject`'s `orientation` equals the transformed pin orientation with no
  remapping — the power symbol rotates to face wherever the stub points.

**Maps to Altium:** `PinConnectionSpec { pin_name, target }` on the component;
expansion in `src/executor.rs` produces SchDoc `Wire`, `NetLabel`, `PowerObject`,
and `NoConnect` records. Round-trip dump of `pin X -> #NET` from an existing
SchDoc is not implemented — `dump` emits the resolved low-level objects instead.

---

## `net`

A top-level signal net declaration listing the pins it connects.

```
net CLK {
    pins: [U1.14, U2.3]
}
```

| Name | Type | Required | Description |
| --- | --- | --- | --- |
| `pins` | array | No | Pin references `Designator.Pin` (e.g. `U1.14`). |

**Maps to Altium:** `compile_net` → `NetSpec { name, pins }` where each pin is a
`PinRef { component, pin }`. A `net` block may carry `#[annotation(...)]`.

---

## `power`

A power-rail declaration. Its name registers in `power_declarations`, which drives
the power-vs-signal classification of `pin X -> #NAME` connections (see above).

```
power VCC {
    style: bar
    show_net_name: true
    orientation: 90
    pins: [U1.1, C1.1]
}
```

| Name | Type | Required | Description |
| --- | --- | --- | --- |
| `style` | enum | No | Power object style (e.g. `bar`); defaults to `Bar`. |
| `show_net_name` | bool | No | Show the net name on the symbol. |
| `orientation` | rotation | No | Symbol orientation. |
| `pins` | array | No | Pin references attached to the rail. |

**Maps to Altium:** `compile_power` → `PowerSpec { name, style, pins,
show_net_name, orientation }`. The set of `power` names is collected in the
compiler pre-pass into `SheetSpec::power_declarations`
(`HashMap<String, PowerObjectStyle>`); style is filled in after all `power` blocks
compile. Lowers to SchDoc `PowerObject` records.

---

## Low-level SchDoc objects

Identifier-dispatched object blocks for full round-trip fidelity (mainly emitted
by `dump`). The recognised types (`SCHDOC_OBJECT_TYPES` in `src/ast.rs`) are:

`wire`, `bus`, `net_label`, `power_object`, `port`, `junction`, `no_connect`,
`bus_entry`, `sheet_symbol`, `parameter_set`, `note`, `probe`, `compile_mask`,
`blanket`, `harness_connector`, `signal_harness`.

```
wire {
    vertices: [(100mil, 100mil), (200mil, 100mil)]
}

net_label {
    text: "CLK"
    location: (200mil, 100mil)
    orientation: 0
}

sheet_symbol Sub1 {
    sheet_name: "Sub.SchDoc"
    location: (300mil, 300mil)
    entry CLK_IN { io_type: input, side: left }
}
```

Each block compiles to a `SchDocObjectSpec` variant (`WireSpec`, `NetLabelSpec`,
`PowerObjectSpec`, `SheetSymbolSpec`, …) — see `src/model.rs` for the per-type
fields. `sheet_symbol` may contain child `entry NAME { … }` blocks (`EntryDecl`)
that compile to `SheetEntrySpec { name, io_type, side, distance_from_top }`.

**Maps to Altium:** each variant lowers directly to the corresponding SchDoc
record (`Wire`, `NetLabel`, `PowerObject`, `Port`, `Junction`, `NoConnect`,
`BusEntry`, `SheetSymbol` + `SheetEntry`, `Parameter`, `Note`, `Probe`, …) via
`schdoc_object_from_spec` in `src/executor.rs`. These are the explicit form of the
objects that pin connections generate implicitly.
