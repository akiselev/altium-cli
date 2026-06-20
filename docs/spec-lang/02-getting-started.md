# Getting started

A hands-on walkthrough: write a minimal schematic-symbol spec for a resistor,
apply it to produce a `.SchLib`, dump an existing document back to a spec, and
preview an ECO with `plan`. Every command shown is a real `altium` subcommand
defined in `crates/altium-cli/src/main.rs`.

**Related pages:** [Introduction](01-introduction.md) ·
[CLI reference](operations/cli.md) · [`schlib` blocks](blocks/schlib.md)

## 1. Write a minimal `.schlib-spec`

Create a file called `resistor.schlib-spec`. A schematic library spec is a list
of `component` blocks; each component has properties, pins, and parameters. The
following resistor symbol is adapted from the inline compiler tests in
`crates/altium-format-spec/src/compiler.rs`:

```
component R_0603 {
    designator: "R"
    description: "SMD resistor 0603"

    pin 1 {
        at: (100mil, 0mil)
        orientation: "0"
        electrical: passive
    }
    pin 2 {
        at: (-100mil, 0mil)
        orientation: "180"
        electrical: passive
    }

    parameter "Value" { text: "10k" }
    parameter "Tolerance" { text: "1%", is_hidden: false }
}
```

What each piece does (see [`schlib` blocks](blocks/schlib.md) for the full
reference):

- `component R_0603 { ... }` declares a symbol whose `lib_reference` is `R_0603`.
- `designator:` and `description:` are component properties.
- `pin 1 { at: (100mil, 0mil) ... }` places a pin at an absolute coordinate.
  Dimension literals such as `100mil` are converted to internal Altium
  coordinates by the compiler (`100mil` → `1_000_000` internal units).
- `orientation: "0"` / `"180"` set the pin rotation (`RotationBy90`).
- `electrical: passive` sets the pin's electrical type; other values include
  `input`, `output`, `io`, and `power` (`parse_pin_electrical_type` in
  `src/compiler.rs`).
- `parameter "Value" { text: "10k" }` attaches a named parameter.

## 2. Apply the spec to produce a `.SchLib`

`altium apply` compiles the spec, reconciles it (against an empty document when
no `--target` is given), and writes the resulting binary:

```bash
altium apply resistor.schlib-spec
```

With no `--output`, the output path defaults to the spec's stem with the
domain's binary extension — here `resistor.SchLib` (see
`default_output_for_spec` in `crates/altium-cli/src/main.rs`). To update an
existing library instead of creating a new one, point `--target` at it:

```bash
altium apply resistor.schlib-spec --target existing.SchLib --output updated.SchLib
```

Other flags: `--report-json` prints the apply report as JSON, and `--all`
processes a spec plus all its imports (valid only for `.prjpcb-spec`).

## 3. Dump an existing document back to a spec

`altium dump` reverse-generates a spec from a binary document. This is how you
bring an existing library under spec control:

```bash
altium dump existing.SchLib
```

The output path defaults to the document stem with the matching spec extension —
here `existing.schlib-spec` (`default_spec_for_document` in
`crates/altium-cli/src/main.rs`). Pass `--output` to choose a different path:

```bash
altium dump existing.SchLib --output recovered.schlib-spec
```

The dumper emits an `#[annotation(...)]` line before each block to anchor its
identity, and sorts output deterministically so re-dumps produce stable diffs.
See [Dump](operations/dump.md) and [Annotations](language/annotations.md).

## 4. Preview an ECO with `plan`

Before applying changes, use `altium plan` to see the Engineering Change Order
without mutating anything:

```bash
altium plan resistor.schlib-spec --target existing.SchLib
```

`plan` reconciles the spec against the target and prints the ECO — the set of
adds and updates that `apply` would perform. With no `--target`, it plans
against an empty document (everything in the spec is an addition). Useful flags:

- `--json` — emit the ECO as JSON instead of a text report.
- `--all` — include imported specs (`.prjpcb-spec` only).

`plan` exits non-zero behavior is driven by whether changes exist; see
[Apply and plan](operations/apply-and-plan.md) for the full semantics.

## Where to go next

- [`schlib` blocks](blocks/schlib.md) — the full schematic-library grammar.
- [Apply and plan](operations/apply-and-plan.md) — the reconcile/ECO workflow.
- [Sync](operations/sync.md) — synchronizing a `.schdoc-spec` into a `.pcbdoc-spec`.
- [Syntax](language/syntax.md) — lexical structure and block forms.
