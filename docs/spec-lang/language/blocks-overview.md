# Blocks overview

A spec file is a sequence of declarations, most of which are *blocks*: a keyword, a name,
and a brace-delimited body of properties and nested blocks. This page explains the general
block and declaration model that all document-specific block types share.

The declaration AST lives in
[`src/ast.rs`](../../../crates/altium-format-spec/src/ast.rs); parsing is in
[`src/parser.rs`](../../../crates/altium-format-spec/src/parser.rs).

**Related pages**

- [Syntax](syntax.md) and [Expressions](expressions.md) — the tokens and values inside blocks
- [Annotations](annotations.md) — the `#[...]` prefix that may precede a block
- Block reference by document type:
  [schlib](../blocks/schlib.md), [pcblib](../blocks/pcblib.md),
  [schdoc](../blocks/schdoc.md), [pcbdoc](../blocks/pcbdoc.md),
  [prjpcb](../blocks/prjpcb.md), [placement](../blocks/placement.md)
- [Grammar reference](../reference/grammar.md)

## A spec file

A spec file (`SpecFile`) is a list of top-level items (`SpecItem`). Each item is one of:
an `import` directive, a top-level `let` binding, or a block declaration. The set of legal
top-level blocks depends on the document domain (SchLib, PcbLib, SchDoc, PcbDoc, PrjPcb).

```
import "standard-footprints.sym" as fp

let passive_pin = { electrical: passive, length: 25, side: outside }

component R {
    designator: "R?"
    description: "Resistor"
}
```

## The block shape

The universal block form is:

```
[#[annotation(...)]] [binding =] KEYWORD NAME {
    property: value
    property: value
    nested_block NAME { ... }
}
```

- **keyword** — a reserved word selecting the block type (`component`, `footprint`, `pin`,
  `pad`, `net`, `board`, `project`, …). See [Keyword reference](../reference/keywords.md).
- **name** — an `EntityName`: an identifier, a quoted string, or an integer. All three are
  legal and reduce to a string identity key:

  ```
  component R          { }   // Ident
  component "My Part"  { }   // String
  pin 1                { }   // Integer
  ```

- **body** — a brace-delimited list of items. Items are separated by newlines, commas, or
  semicolons (whitespace-tolerant). Body items are properties, nested blocks, `let`
  bindings, and spreads, depending on the block type.

Some blocks carry no body (e.g. `alias R0603`).

## Properties

The core body item is a property: `key: value`.

```
designator: "R?"
height: 1.2mm
shape: rectangular
```

The key is an identifier (parameter keys are case-insensitive and normalized to lowercase
by the compiler). The value is any [expression](expressions.md). A block body is generally
lowered to an `Object` value, so properties behave like object entries — including support
for [spread](expressions.md#spread):

```
pad: { ...qfp_pad, layer: "TopLayer" }
```

## Bindings

A block may be assigned to a binding with the `name =` prefix so later expressions can
reference it via `$name`:

```
component R {
    body = rectangle { from: (-20mil, -10mil), to: (20mil, 10mil) }
    pin 1 { on: $body.left, at: center }   // $body refers to the bound graphic block
}
```

Bindings and the `let` keyword are described in
[Expressions § let bindings and scopes](expressions.md#let-bindings-and-scopes).

## Nesting

Blocks nest to mirror Altium's containment hierarchy. The parser uses a typed item enum per
container, so only the legal child blocks are accepted in each context (a typo or
out-of-place block is a parse error rather than a silent no-op). For example, a `component`
body (`ComponentItem`) may contain properties, `part`, `pin`, `parameter`, `alias`,
`footprint`-map, graphic, `swap_group`, and pin-connection items:

```
component LM358 {
    designator: "U?"
    part 1 {
        body = rectangle { from: (-100mil, -100mil), to: (100mil, 100mil) }
        pin 1 { electrical: output }
        pin 2 { electrical: input }
    }
    pin 4 { electrical: power, is_hidden: true }
    alias LM358N
    footprint $fp.DIP8
}
```

A `footprint` body (`FootprintItem`) instead accepts `pad`, `row`, `column`, `grid`, and
graphic blocks:

```
footprint QFP32 {
    description: "32-pin QFP, 0.8mm pitch"
    body = rectangle { from: (-3.5mm, -3.5mm), to: (3.5mm, 3.5mm) }
    row { on: $body.left, at: center, pitch: 0.8mm, count: 8, start: 1, pad: { ...qfp_pad } }
}
```

## Document-specific blocks

The concrete keywords and their legal children differ by document type. They are documented
on the per-domain block pages:

| Domain | Top-level blocks (examples) | Page |
| ------ | --------------------------- | ---- |
| SchLib | `component`, `part`, `pin`, `parameter`, `alias`, graphics | [blocks/schlib](../blocks/schlib.md) |
| PcbLib | `footprint`, `pad`, `row`, `column`, `grid`, graphics | [blocks/pcblib](../blocks/pcblib.md) |
| SchDoc | `sheet`, `net`, `power`, `wire`/`bus`/`port`/… objects | [blocks/schdoc](../blocks/schdoc.md) |
| PcbDoc | `board`, `polygon`, `rule`, `class`, `differential_pair`, primitives | [blocks/pcbdoc](../blocks/pcbdoc.md) |
| PrjPcb | `project` with `document`, `annotation`, `variant`, … | [blocks/prjpcb](../blocks/prjpcb.md) |
| Placement | `placement` with `place`, `group`, `separate`, constraints | [blocks/placement](../blocks/placement.md) |

## The annotation prefix

Any block declaration may be preceded by a single `#[annotation(...)]` attribute, which
attaches sync metadata (a stable ID, a stability flag, a group). It is parsed into the
`annotation` field on the block's `*Decl`:

```
#[annotation(id = "AB12CD34", stable = true, group = "power")]
net VCC { }
```

Annotations are documented in full on the [Annotations](annotations.md) page.
