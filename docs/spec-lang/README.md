# Altium Spec Language

The Altium Spec Language is a declarative text DSL for describing the desired
state of Altium Designer documents — symbol libraries, schematic sheets, PCB
boards, projects, and PCB placement intent — so they can be version-controlled,
diffed, and reconciled against binary `.SchLib`/`.PcbLib`/`.SchDoc`/`.PcbDoc`/`.PrjPcb`
files. You write a `*-spec` file, and the tooling compiles it, validates it,
plans an Engineering Change Order (ECO) against an existing document, and applies
it. The implementation lives in the crate `crates/altium-format-spec`.

## Navigation

### Tutorials

| Page | Description |
| --- | --- |
| [Introduction](01-introduction.md) | What the language is, its design philosophy, and the compile pipeline. |
| [Getting started](02-getting-started.md) | Hands-on: write a resistor `.schlib-spec`, apply it, dump a document, and plan an ECO. |

### Reference

| Page | Description |
| --- | --- |
| [Syntax](language/syntax.md) | Lexical structure: tokens, identifiers, comments, blocks, annotations. |
| [Types and values](language/types-and-values.md) | Literal kinds: strings, dimensions, colors, tuples, arrays, objects. |
| [Expressions](language/expressions.md) | Operators, references, paths, function calls, spreads. |
| [Blocks overview](language/blocks-overview.md) | Every top-level declaration and where it is valid. |
| [Annotations](language/annotations.md) | The `#[annotation(...)]` attribute and its keys. |
| [`schlib` blocks](blocks/schlib.md) | `component`, `pin`, `parameter`, `alias`, footprint maps, graphics. |
| [`pcblib` blocks](blocks/pcblib.md) | `footprint`, `pad`, `row`/`column`/`grid`, PCB graphics. |
| [`schdoc` blocks](blocks/schdoc.md) | `sheet`, `net`, `power`, placed components, schematic objects. |
| [`pcbdoc` blocks](blocks/pcbdoc.md) | `board`, primitives, `polygon`, `rule`, `class`, `differential_pair`, `routing`. |
| [`prjpcb` blocks](blocks/prjpcb.md) | `project`, documents, annotation, ERC, output groups, variants. |
| [`placement` blocks](blocks/placement.md) | The placement sub-language inside `.pcbdoc-spec`. |
| [Grammar](reference/grammar.md) | The formal grammar reference. |
| [Keywords](reference/keywords.md) | Reserved words and block keywords. |
| [Altium mapping](reference/altium-mapping.md) | How spec constructs map to Altium objects. |

### How-to guides

| Page | Description |
| --- | --- |
| [CLI reference](operations/cli.md) | Every `altium` subcommand that touches specs. |
| [Apply and plan](operations/apply-and-plan.md) | Producing documents and previewing ECOs. |
| [Dump](operations/dump.md) | Reverse-generating a spec from a binary document. |
| [Sync](operations/sync.md) | Spec-to-spec synchronization (SchDoc → PcbDoc). |

### Explanation

| Page | Description |
| --- | --- |
| [Design rationale](explanation/design-rationale.md) | Why the language is built the way it is. |
| [Greenfield vs brownfield](explanation/greenfield-vs-brownfield.md) | Who is authoritative (spec or Altium files), how that decides inline-children handling, identity tracking, and two-sided change sets. |

## The five spec domains

A spec file's domain is determined by its extension (`detect_spec_domain` in
`crates/altium-cli/src/main.rs`). Each maps to one `SpecDomain` variant
(`SpecDomain` in `src/model.rs`):

| Extension | Domain | Describes |
| --- | --- | --- |
| `.schlib-spec` | `SchLib` | Schematic symbol library (components, pins, parameters). |
| `.pcblib-spec` | `PcbLib` | PCB footprint library (footprints, pads, courtyards). |
| `.schdoc-spec` | `SchDoc` | Schematic sheet (placed components, nets, power, wires). |
| `.pcbdoc-spec` | `PcbDoc` | PCB board (board outline, primitives, rules, placement). |
| `.prjpcb-spec` | `PrjPcb` | PCB project (member documents, annotation, ERC, variants). |

## The five operations

Each spec domain supports the same set of operations, exposed through the
`altium` CLI and the crate's public API (`src/lib.rs`):

| Operation | CLI | Crate entry point |
| --- | --- | --- |
| **compile / apply** — turn a spec into (or onto) a binary document | `altium apply` | `apply_spec_*` (`src/executor.rs`) |
| **plan / reconcile** — show the ECO without mutating | `altium plan` | `reconcile_*` (`src/reconciler.rs`) |
| **dump** — reverse-generate a spec from a document | `altium dump` | `dump_*` (`src/dump.rs`) |
| **format** — reformat spec text | `altium format` | `format_spec` (`src/formatter.rs`) |
| **sync** — synchronize two specs | `altium spec sync` | `diff_snapshots` / `apply_sync_changes_to_pcbdoc` (`src/sync.rs`) |
