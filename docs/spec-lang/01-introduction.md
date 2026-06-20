# Introduction

The Altium Spec Language is a declarative DSL for describing the desired state of
Altium Designer documents as plain, version-controllable text. Instead of editing
binary `.SchLib`/`.PcbLib`/`.SchDoc`/`.PcbDoc`/`.PrjPcb` files directly, you write
a `*-spec` file that states *what the document should contain*, then let the
tooling reconcile that intent against an existing document and apply the diff.

**Related pages:** [README](README.md) · [Getting started](02-getting-started.md) ·
[Design rationale](explanation/design-rationale.md)

## What it is for

The spec language exists so that external tools and humans never need to touch
Altium binaries directly. The intended workflow is:

> write or generate a spec → inspect / tweak → reconcile → apply to the document

This keeps design decisions — component definitions, footprint maps, net
connectivity, board rules, component placement — human-readable and diffable in
source control. It is the *intermediate representation* between higher-level
tooling and the opaque Altium file formats (see
[design rationale](explanation/design-rationale.md) on spec-as-IR). The crate is
described in `crates/altium-format-spec/README.md`.

## Design philosophy

**Declarative.** A spec describes desired state, not a sequence of edit commands.
The reconciler computes the difference between the spec and a document and emits
an Engineering Change Order (ECO); the executor applies it. This is fundamentally
different from an imperative "add this, edit that" interface.

**Fail-fast.** Mirroring the wider `altium-cli` project philosophy, the compiler
and validator reject the first thing they do not understand rather than silently
dropping it. Unknown annotation keys are rejected at parse time, undefined
`$`-bindings are hard errors, and type mismatches in coordinate contexts abort
compilation (`SpecErrorCode::TypeMismatch`, `UndefinedBinding` in `src/eval.rs`).

**Spec-as-intermediate-representation.** The spec sits between source-controlled
intent and the binary document. Placement intent, for example, is expressed in a
readable `placement { }` sub-language and reconciled into board geometry, rather
than being authored as raw coordinates inside a PcbDoc.

**Text version-controllable.** Specs are ordinary UTF-8 text. Dump output is
sorted deterministically by designator/name so that re-dumping a document
produces stable diffs (invariant in `crates/altium-format-spec/README.md`).
Rewrite operations (`format`, `sync`) edit the source text in place using
byte-offset spans rather than round-tripping through the AST, so user formatting
is largely preserved.

## The five-phase pipeline

Compilation and reconciliation run as a five-phase pipeline. The phases and the
modules that implement them are documented in the crate README:

```
Phase 1: PARSE      lexer → parser → AST (with BlockAnnotation nodes)
Phase 2: COMPILE    AST → SpecModel (CompiledAnnotation, sane defaults, seen_ids)
Phase 3: VALIDATE   validator.rs: duplicate designators, dangling net refs, duplicate IDs
Phase 4: RESOLVE    resolver.rs: SchLib lookups → FootprintResolvedSpec
Phase 5a: PROJECT   sync.rs: SpecModel → SyncSnapshot → diff → apply
Phase 5b: PROJECT   reconciler.rs: SpecModel vs document → EngineeringChangeOrder
```

1. **PARSE** — `src/lexer.rs` tokenizes the source, attaching a
   `Span { start, end }` byte offset to every token; `src/parser.rs` builds a
   typed AST (`src/ast.rs`). Every AST node carries a span, and
   `parse_annotation()` runs before each block declaration so `#[annotation(...)]`
   attributes attach to the block that follows.

2. **COMPILE** — `src/compiler.rs` lowers the AST to a `SpecModel`
   (`src/model.rs`). This is where unit strings (`mm`, `mil`, `inch`) are
   resolved to internal Altium coordinates, layers are resolved, references are
   evaluated, and each `BlockAnnotation` becomes a `CompiledAnnotation`. The
   compiler also performs a within-file duplicate-ID check via a per-compile
   `seen_ids` set.

3. **VALIDATE** — `src/validator.rs` runs the authoritative structural checks:
   duplicate designators, dangling net references, and cross-file duplicate
   annotation IDs. `validate_*_spec()` returns `Ok(warnings)` when the spec is
   structurally valid (non-fatal warnings carried in the `Ok` value) or
   `Err(errors)` when a hard error means projection must not proceed.

4. **RESOLVE** — `src/resolver.rs` performs library lookups, e.g. resolving a
   SchDoc component's footprint from its SchLib symbol to produce a
   `FootprintResolvedSpec`.

5. **PROJECT** — the terminal phase has two consumers of the `SpecModel`:
   - **5a (sync)** — `src/sync.rs` projects a spec into a `SyncSnapshot`, diffs
     two snapshots, and applies the resulting changes back to a spec file.
   - **5b (reconcile)** — `src/reconciler.rs` diffs the `SpecModel` against a
     *binary* Altium document and produces an `EngineeringChangeOrder`, which the
     executor (`src/executor.rs`) applies to mutate the document.

## How specs relate to Altium binary documents

A spec is never the file of record on its own; it is a textual *projection* of a
document's desired state. The relationship is bidirectional:

- **Forward** (`apply`, `plan`) — the compiler turns the spec into a `SpecModel`,
  the reconciler diffs it against an existing document (or an empty one for a new
  file), and the executor applies the ECO to produce or update the binary.
- **Reverse** (`dump`) — an existing `.SchLib`, `.PcbLib`, `.SchDoc`, `.PcbDoc`,
  or `.PrjPcb` is read and emitted as spec text, with an `#[annotation(...)]`
  emitted before each block to anchor its identity for future sync.

Because reconciliation is a diff, applying an unchanged spec is a no-op, and
applying an edited spec produces only the minimal ECO needed to bring the
document into agreement with the spec.
