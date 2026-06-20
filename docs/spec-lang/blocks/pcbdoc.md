# PCB document blocks (`.pcbdoc-spec`)

Blocks that describe a PCB layout: the `board` (geometry, layers, settings), placed
`component`s, `net`s, design `rule`s, `class`es, `differential pair`s, primitives,
`polygon` pours, and `routing`. Component placement intent lives in a nested
[`placement` block](placement.md).

**Related pages:** [Blocks overview](../language/blocks-overview.md) ·
[`placement` block](placement.md) · [`.pcblib-spec`](pcblib.md) ·
[`.schdoc-spec`](schdoc.md) · [Annotations](../language/annotations.md) ·
[Apply and plan](../operations/apply-and-plan.md) ·
[Altium mapping](../reference/altium-mapping.md)

A `.pcbdoc-spec` file compiles to a `PcbDocSpec { boards, placement,
placement_rules, routing }` (`src/model.rs`). At most one `board` block produces
the single `BoardSpec`; all primitives, nets, rules, classes, polygons, and
differential pairs are collected into that board (`compile_pcbdoc` in
`src/compiler.rs`).

> **Fail-fast gap:** `compile_pcbdoc` currently skips top-level items from other
> domains. Do not mix document domains in one file or rely on that behavior;
> accepting and dropping a parsed declaration is an open defect tracked in
> `STATUS.md`.

The recognised top-level primitive types (`PCBDOC_PRIMITIVE_TYPES`) are `track`,
`arc`, `via`, `fill`, `text`, `region`, `component_body`, `dimension`; the named
block types (`PCBDOC_BLOCK_TYPES`) are `polygon`, `rule`, `class`,
`differential_pair`.

---

## `board`

Board geometry and global settings. Properties are collected into an evaluated
property map and lowered into `BoardSpec` scalar fields plus the board outline.

```
board Main {
    signal_layer_count: 4
    snap_grid_size: 0.5mm
    visible_grid_size: 1mm
    display_unit: "mm"
    outline: rect((0mm, 0mm), (50mm, 40mm))
}
```

| Name | Type | Required | Description |
| --- | --- | --- | --- |
| `signal_layer_count` | integer | No | Number of signal (copper) layers. |
| `snap_grid_size` | dimension | No | Snap grid spacing. |
| `visible_grid_size` | dimension | No | Visible grid spacing. |
| `display_unit` | string | No | UI display unit (e.g. `"mm"`). |
| `outline` | shape / point array | No | Board outline. A shape from `rect()`/`circle()` (`Value::Shape`) or a raw vertex array. |

**Maps to Altium:** `compile_board_settings` evaluates the body; the scalar fields
become `BoardSpec.signal_layer_count`, `snap_grid_size`, `visible_grid_size`,
`display_unit`, and `extract_outline_from_props` builds `BoardSpec.outline:
Option<Vec<CoordPoint>>`. These map to the PcbDoc `Board` record (layer stack
count, grid settings, display unit) and the board-outline primitive.

> **Note (current behaviour):** `BoardSpec` also carries `keepouts:
> Vec<KeepoutSpec>` and `layers: Vec<BoardLayerSpec>` fields, but `compile_pcbdoc`
> currently initialises both to empty (`keepouts: Vec::new()`, `layers:
> Vec::new()`). There is no `board`-body syntax wired up in the compiler today
> that fills them. The model types exist for executor/round-trip use:
>
> - `KeepoutSpec { vertices, restrict_copper, restrict_components, layer }`
> - `BoardLayerSpec { name, is_copper, copper_index }`

---

## `component` (placed instance)

A placed PCB component (footprint instance). Pad-to-net assignments use the
`pad_net` statement.

```
component U1 {
    footprint: $fp.LQFP100
    comment: "ESP32-C6"
    at: (25mm, 20mm)
    rotation: 90
    layer: top
    pad_net 1: "GND"
    pad_net 2: "VCC"
}
```

| Name | Type | Required | Description |
| --- | --- | --- | --- |
| `footprint` | import ref | No | Footprint from an imported `.pcblib-spec` (`$alias.Name`), stored as `FootprintRef { import_alias, name }`. |
| `comment` | string | No | Component comment. |
| `at` | point | No | Placement location. |
| `rotation` | number | No | Rotation in degrees (`f64`). |
| `layer` | layer | No | Placement layer (e.g. `top`, `bottom`). |
| `pad_net PAD: "NET"` | statement | No | Pad-to-net assignment. Repeatable; duplicate pad names are rejected. |

**Maps to Altium:** `compile_pcbdoc_component` → `PcbDocComponentSpec` with
`footprint`, `comment`, `location`, `rotation`, `layer`, `parameters`, and
`pad_nets: IndexMap<String, String>`. Lowers to a PcbDoc `Component` record plus
its pad-net wiring.

---

## `net`

A PCB net with display and routing metadata.

```
net VCC {
    color: #FF0000
    visible: true
    routing_style: "power"
}
```

| Name | Type | Required | Description |
| --- | --- | --- | --- |
| `color` | color | No | Net display colour. |
| `visible` | bool | No | Net visibility. |
| `routing_style` | string | No | Free-form routing-style hint (e.g. `"power"`). |

**Maps to Altium:** `compile_pcbdoc_net` → `PcbDocNetSpec { name, color, visible,
routing_style }`. `routing_style` is read as a plain string and carried in the IR
for downstream routing tooling; it is not a fixed enum at the compiler level. May
carry `#[annotation(...)]`.

---

## `rule`

A PCB design rule. Well-known scalar keys (`kind`, `enabled`, `priority`, `scope`)
are pulled into typed fields; everything else falls into a `properties` map.

```
rule r_clearance {
    kind: "clearance"
    gap: 5mil
    scope: "all_copper"
    enabled: true
    priority: 1
}
```

| Name | Type | Required | Description |
| --- | --- | --- | --- |
| `kind` | string | No | Rule kind (e.g. `"clearance"`). |
| `enabled` | bool | No | Whether the rule is active. |
| `priority` | integer | No | Rule priority. |
| `scope` | string | No | First scope expression (e.g. `"all_copper"`). |
| `properties { … }` | block | No | Extra rule parameters merged into `properties`. |
| *(any other key)* | — | No | Folded into `properties` (including `gap`). |

Any body key that is not one of `kind`/`enabled`/`priority`/`scope`/`properties`
is folded into the `properties` map (so `gap: 5mil` becomes
`properties["gap"]`). A nested `properties { … }` block is also merged in.
`gap` is intentionally *not* a reserved key — PCB rule parameters such as
`Clearance.gap` are read out of `properties` downstream; excluding it would make
those fields read as `0.0`.

### Two-object rules (`scope2`)

`PcbDocRuleSpec` carries a second scope field, `scope2: Option<String>`, for
two-object rules such as Clearance and ComponentClearance (the rule applies
*between* objects matched by `scope` and objects matched by `scope2`).

> **Note (current behaviour):** `compile_pcbdoc_rule` currently sets `scope2:
> None` — the model field exists and is consumed downstream, but no `board`-spec
> body key is wired to populate it in the compiler today. Use `scope` for
> single-object rules; `scope2` is reserved for the two-object case.

**Maps to Altium:** `PcbDocRuleSpec { name, kind, enabled, priority, properties,
scope, scope2 }` → a PcbDoc design-rule record. May carry `#[annotation(...)]`.

---

## `class`

A component or net class with a member list.

```
class HighSpeed {
    kind: "net"
    members: ["CLK", "DATA0", "DATA1"]
}
```

| Name | Type | Required | Description |
| --- | --- | --- | --- |
| `kind` | string | No | Class kind (e.g. `"net"`, `"component"`). |
| `members` | array of strings | No | Member names. Non-string array entries are ignored. |

**Maps to Altium:** `compile_pcbdoc_class` → `PcbDocClassSpec { name, kind,
members }` → a PcbDoc class record. May carry `#[annotation(...)]`.

---

## `differential_pair`

A differential pair binding two nets.

```
differential_pair USB {
    positive_net: "USB_DP"
    negative_net: "USB_DM"
}
```

| Name | Type | Required | Description |
| --- | --- | --- | --- |
| `positive_net` | string | No | Positive-leg net name. |
| `negative_net` | string | No | Negative-leg net name. |

**Maps to Altium:** `compile_pcbdoc_diff_pair` → `PcbDocDifferentialPairSpec {
name, positive_net, negative_net }`. The compiler always sets `annotation: None`
for differential pairs (unlike most other blocks).

---

## `polygon`

A copper polygon pour.

```
polygon GND_FILL {
    net: "GND"
    layer: top
    connect_style: "relief"
    pour_order: 1
}
```

| Name | Type | Required | Description |
| --- | --- | --- | --- |
| `net` | string | No | Net to pour. |
| `layer` | layer | No | Pour layer. |
| `connect_style` | string | No | Thermal-relief / connect style. |
| `pour_order` | integer | No | Pour order. |

**Maps to Altium:** `compile_pcbdoc_polygon` → `PcbDocPolygonSpec { name, net,
layer, connect_style, pour_order }` → a PcbDoc `Polygon` record. May carry
`#[annotation(...)]`.

---

## Primitives (`track`, `arc`, `via`, `fill`, `text`, `region`, `component_body`, `dimension`)

Generic PCB primitives. They share one syntax — an optional name plus a property
object whose evaluated key/values are stored untyped; the executor converts them
to typed API objects.

```
track { start: (0mm, 0mm), end: (10mm, 0mm), width: 0.25mm, layer: top }
via   { at: (5mm, 5mm), diameter: 0.6mm, hole: 0.3mm }
text  { at: (1mm, 1mm), text: "REV A", layer: top_overlay }
```

| Name | Type | Required | Description |
| --- | --- | --- | --- |
| *(name)* | identifier | No | Optional primitive name. Named primitives get a stable `spec:board:NAME` id; unnamed ones get an auto-incremented id per type. |
| *(body)* | object | Yes | Type-specific properties (geometry, layer, width, net, …) stored as a `Value` map. |

**Maps to Altium:** `compile_pcbdoc_primitive` → `PcbDocPrimitiveSpec { id,
position_index, primitive_type, properties }`, collected into the matching
`BoardSpec` vector (`tracks`, `arcs`, `vias`, `pads`, `fills`, `texts`, `regions`,
`component_bodies`, `dimensions`). `properties` is an evaluated `IndexMap<String,
Value>`; per-type typing happens in the executor. Each lowers to its respective
PcbDoc primitive record. (`pad` is also accepted as a primitive type and routed to
`BoardSpec.pads`.)

---

## `routing`

A top-level routing-configuration block referencing an external solution file and
arbitrary config overrides.

```
routing {
    solution: "main.routes"
    via_style: "default"
}
```

| Name | Type | Required | Description |
| --- | --- | --- | --- |
| `solution` | string | No | Path to a `.routes` solution file (relative to the spec dir). |
| *(any other key)* | — | No | Folded into `config` as `key → display-string`. |

**Maps to Altium:** `compile_routing_decl` → `RoutingSpec { solution, config }`
stored on `PcbDocSpec.routing`. Every key other than `solution` is captured in
`config: IndexMap<String, String>`. This is routing tooling metadata, not a direct
PcbDoc record.

---

## `placement`

Component-placement intent (fixed positions, autoplace, grouping, separation,
optimization). This is a nested sub-language documented separately.

```
placement {
    unplaced: autoplace
    place U1 { at: (10mm, 20mm) }
    place U2 { autoplace: true, region: center }
}
```

**Maps to Altium:** `compile_placement_decl` → `PlacementSpec` on
`PcbDocSpec.placement`. See **[the placement block reference](placement.md)** for
the full grammar, `place` arguments, groups, separation, minimize, and the
constraint-semantics table.
