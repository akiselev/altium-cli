# The `placement` block (`.pcbdoc-spec`)

A sub-language inside a `.pcbdoc-spec` file that describes **where components go**
on the board and how an automatic placer may move them. The spec crate parses,
compiles, and formats placement intent, but contains no placement solver itself —
the compiled `PlacementSpec` is handed to the external autopcb placer.

**Related pages:** [`.pcbdoc-spec` blocks](pcbdoc.md) ·
[Blocks overview](../language/blocks-overview.md) ·
[Types and values](../language/types-and-values.md) ·
[Annotations](../language/annotations.md) ·
[Apply and plan](../operations/apply-and-plan.md) ·
[Altium mapping](../reference/altium-mapping.md)

---

## Overview

The `placement { }` block appears at most once inside a `.pcbdoc-spec` file. It
compiles (`parse_placement`/`compile_placement` in `src/parser.rs` and
`src/compiler.rs`) to a `PlacementSpec` stored on `PcbDocSpec.placement`
(`src/model.rs`). Each `place` declaration becomes a `PlacementPlaceSpec`; the
block-level properties configure the placer as a whole.

```
placement {
    unplaced: autoplace            // policy for components not named below
    algorithm: full_pipeline       // (inside an autoplace { } block — see below)

    place U1 {
        at: (50mm, 40mm)           // fixed pin
    }

    place R1, R2, R3 {
        autoplace: true            // let the solver choose
        near: $U1
        max_distance: 5mm
    }

    group power { components: ["U1", "C1", "C2"] }
    separate $power, $analog { gap: 3mm }

    left_of $U1, $U2
    minimize wirelength
}
```

> **Maps to Altium:** Placement intent has **no direct Altium record**. It is
> consumed by the autopcb placer, which produces concrete component X/Y/rotation
> values that *then* flow through the normal `component` apply path
> (`apply_spec_pcbdoc`). A `place` with a fixed `at:` and no `autoplace` is
> equivalent to setting the component's location directly.

---

## Block-level items

The body of `placement { }` accepts these items (`PlacementItem` in
`src/ast.rs`, dispatched in `parse_placement`):

| Item | Form | Purpose |
|---|---|---|
| Property | `key: value` | Block-wide settings (e.g. `unplaced:`) |
| `let` binding | `let NAME = expr` | Local binding usable later in the block |
| `place` | `place D1, D2 { … }` | Place one or more components ([below](#the-place-declaration)) |
| `left_of` / `right_of` / `above` / `below` | `left_of $A, $B` | Directional relative constraint |
| `optimize` | `optimize { ratsnest: true … }` | Objective weighting |
| `clearance` | `clearance { all: 0.5mm, edge: 1mm }` | Minimum spacing hints |
| `minimize` | `minimize wirelength [subject_to { … }]` | Set optimization objective |
| `autoplace` | `autoplace { algorithm: … }` | Solver configuration block |
| `group` | `group NAME { components: [...] }` | Named component group |
| `separate` | `separate $A, $B { gap: Nmm }` | Keep two groups apart |

### `unplaced:` policy

Controls what happens to PcbDoc components **not** named in any `place` block
(`UnplacedStrategy` in `src/model.rs`, parsed in `compile_placement`):

| Value | Meaning |
|---|---|
| `autoplace` *(default)* | Unnamed components become free variables the solver may place |
| `ignore` | Unnamed components are pinned at their current PcbDoc position |
| `error` | Compilation/placement errors if any PcbDoc component is missing from the spec |

### `minimize` objective

`minimize <objective> [subject_to { … }]` (`MinimizeDecl`). The objective is an
identifier such as `wirelength`, `congestion`, or `area`. Currently only
`wirelength` is wired through — it sets `optimize.ratsnest = true` with a default
weight; other objectives parse but are reserved for future milestones. The
optional `subject_to { … }` block carries constraint-relaxation hints (parsed,
not yet consumed).

### `optimize` and `clearance`

```
optimize { ratsnest: true, ratsnest_weight: 0.02 }
clearance { all: 0.4mm, edge: 1mm }
```

`optimize` recognises `ratsnest` (bool) and `ratsnest_weight` (float).
`clearance` recognises `all` and `edge` (both coordinates). See
`compile_placement` in `src/compiler.rs`.

### `autoplace { }` configuration block

A bare `autoplace { … }` (distinct from the `autoplace:` property on a `place`)
configures the solver itself — `algorithm`, `sa_cooling`, `sa_moves_per_temp`,
and related tuning knobs (`compile_autoplace_config`, `AutoplaceConfig`). A
common value is `algorithm: full_pipeline`.

---

## The `place` declaration

```
place <designator>[, <designator>…] {
    <property>: <value>
    …
}
```

`place` names one or more component designators and sets placement properties for
all of them (`PlaceDecl` → `PlacementPlaceSpec`). A `place` block may be preceded
by an [`#[annotation(...)]`](../language/annotations.md) attribute.

### `place` properties

| Property | Type | Description |
|---|---|---|
| `at` | coord point `(x, y)` | Position. With no `autoplace`, pins the component (`FixedPosition`) |
| `rotation` | integer / float / array | Fixed rotation (float) **or**, when the component is a solver variable, the allowed rotation set (array of ints, e.g. `[0, 90, 180, 270]`) |
| `fixed` | bool | Treat the component as locked at `at` |
| `side` | string | Board side (e.g. `top` / `bottom`) |
| `autoplace` | bool / `solved` / `locked` | Placement mode — see [below](#autoplace-modes) |
| `region` | name **or** `{ from: (x,y), to: (x,y) }` | Restrict to a named region or an explicit rectangle |
| `edge` | string | Edge to hug when autoplacing (e.g. `top`, `left`) |
| `inset` | coordinate | Distance in from the `edge` |
| `near` | component ref | Place near another component (e.g. `$U1`) |
| `max_distance` | coordinate | Maximum distance for the `near` constraint |
| `no_pin_swap` | string / array | Pin names excluded from pin-swap optimization |
| `no_part_swap` | bool | Disable part swapping for this component |

Property names and types are verified in `compile_placement_place`
(`src/compiler.rs`).

### `autoplace` modes

The `autoplace:` value maps to `PlacementAutoplaceMode`
(`expr_to_autoplace_mode`):

| Spec value | Mode | Meaning |
|---|---|---|
| `false` *(default when omitted)* | `Disabled` | Component is not solver-managed |
| `true` | `Auto` | Solver may place the component freely |
| `solved` | `Solved` | Marks a position the solver produced |
| `locked` | `Locked` | Solver-managed but locked in place |

### Constraint semantics summary

How `place` properties translate to solver constraints (reproduced from the crate
README; the spec crate stores the intent, the solver applies the constraint):

| Spec property | Solver constraint |
|---|---|
| `at: (x,y)` with no `autoplace: true` | `FixedPosition` — component pinned |
| `autoplace: true` (no other hint) | Unconstrained placement variable |
| `autoplace: true, edge: top, inset: 2mm` | `EdgePlacement { edge: Top, inset: 2.0 }` |
| `autoplace: true, near: $REF, max_distance: 5mm` | `Near { max_distance: 5.0 }` |
| `autoplace: true, region: center` | `RegionContainment` covering that region |

---

## Regions

`region:` accepts either a **named region** (identifier or string) or an explicit
rectangle object `{ from: (x, y), to: (x, y) }`. Named regions are interpreted by
the placer, not validated by the spec crate. The standard names are:

`center`, `top_half`, `bottom_half`, `left_half`, `right_half`,
`quadrant_tl`, `quadrant_tr`, `quadrant_bl`, `quadrant_br`.

```
place U1 { autoplace: true, region: center }
place J1 { autoplace: true, region: { from: (0mm, 0mm), to: (20mm, 10mm) } }
```

---

## Groups and separation

### `group`

```
group power { components: ["U1", "C1", "C2"] }
```

Defines a named set of components (`PlacementGroupDecl` →
`compile_placement_group`). Groups are referenced elsewhere with the `$name`
syntax.

### `separate`

```
separate $power, $analog { gap: 3mm }
```

Requests a minimum `gap` between the centroids of two groups
(`PlacementSeparateDecl`). The `gap` is a coordinate distance.

> **Note:** `separate` is parsed and carried in the AST, but at the current
> milestone `compile_placement` does not yet lower it into a
> `PlacementConstraintSpec`. Treat it as forward-looking syntax until the solver
> side consumes it.

---

## Directional constraints

```
left_of  $U1, $U2
right_of $A,  $B
above    $A,  $B
below    $A,  $B
```

Each names two component (or group) references and asserts a relative ordering
(`PlacementConstraintDecl`: `LeftOf` / `RightOf` / `Above` / `Below`). An optional
trailing `{ … }` body can carry parameters. Directional constraints cannot be
preceded by an annotation — only `place` blocks can.

---

## Notes and limitations

- The `placement` block is **PcbDoc-only**. It is ignored in other spec domains.
- The spec crate has **no solver dependency**: it produces a typed `PlacementSpec`
  describing intent. Concrete coordinates come from the external placer and are
  then applied through the normal `component` path.
- Several items (`minimize` objectives beyond `wirelength`, `separate` lowering,
  `subject_to` hints) are accepted by the grammar but only partially consumed at
  the current milestone, by design — they reserve syntax for upcoming solver
  features rather than failing the parse.
