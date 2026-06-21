# Dump

How to reverse-generate a spec file from an existing Altium document with
`altium dump`: annotation-ID auto-generation, deterministic sort ordering, the
merge-on-write behavior, and round-trip caveats.

## Related pages

- [CLI Reference](cli.md#altium-dump) — flags, domain detection, output paths
- [Apply and Plan](apply-and-plan.md) — apply a spec back to a document
- [Annotations](../language/annotations.md) — the `#[annotation(...)]` syntax
- [Operations overview](../README.md)

## What dump does

`altium dump <document>` opens a binary Altium document and emits equivalent
spec source (`dump_*` in `src/dump.rs`). The output uses **absolute placement
only** — `at: (x, y)` with explicit `orientation:`. No anchors, rows, grids, or
template bindings are emitted; what you get is a literal, fully resolved
description of the document.

```bash
altium dump my-parts.SchLib
# Dumped: my-parts.SchLib -> my-parts.schlib-spec

altium dump board.PcbDoc --output board.pcbdoc-spec
```

Domain is detected from the document extension; see
[CLI Reference](cli.md#domain-handling). `.intlib` is special — it can emit both
a `.schlib-spec` and a `.pcblib-spec`, and treats `--output` as a directory.

## Annotation-ID auto-generation

Every dumped block (component, footprint, net, polygon, rule) is preceded by an
`#[annotation(...)]` line. When the block has no pre-existing annotation, dump
generates a fresh 8-character short ID via `generate_short_id()`
(`emit_annotation_line`):

```
#[annotation(id = "Qx7Kp2mZ")]
component R_0603 { ... }
```

When a source ID is available (e.g. a schematic `UNIQUE_ID`), it is emitted too:

```
#[annotation(id = "Qx7Kp2mZ", source_id = "ABCD1234-...")]
```

### Stability of generated IDs

Auto-generated IDs are a zero-effort convenience, not a stable anchor. Per the
crate README:

- A **manually set** `id` is preserved verbatim through all spec rewrite
  operations and is never overwritten by the dumper.
- An **auto-generated** ID may differ between dump runs if a block's identity is
  ambiguous (e.g. two footprints with the same name): the value depends on the
  order the dumper visits blocks, which can change after document edits.

If you need stable IDs — for three-way merge or to anchor external tooling — set
them by hand:

```
#[annotation(id = "MYID1234")]
component R1 { ... }
```

The [merge-on-write](#merge-on-write) flow exists precisely to preserve these
manual IDs across re-dumps.

## Deterministic sort ordering

Dump always emits in a stable order so diffs stay small (crate invariant:
"`dump_*` always sorts output by designator/name for stable diffs").

- Components and footprints follow the document's name ordering
  (`component_names()` / `footprint_names()`).
- Designator-keyed collections sort with a **numeric-aware** comparator
  (`designator_key`), so `U2` sorts before `U10`, not after. This is used for
  placement blocks (`dump_placement_block_from_parts`) and PcbDoc component
  ordering.
- Auxiliary lists (aliases, unique-ID sets) are sorted before emission
  (`sorted.sort()`, `ids.sort_unstable()`).

The practical effect: dumping the same document twice yields byte-identical
output (modulo freshly generated annotation IDs for un-annotated blocks), and
small edits to a document produce small, reviewable spec diffs.

## Merge-on-write

Dump does not blindly clobber an existing spec. `write_spec_merged` (main.rs):

| Existing output state      | Action |
| -------------------------- | ------ |
| Does not exist             | Write fresh. Prints `Dumped: <doc> -> <out>`. |
| Exists and parses cleanly  | Apply typed structured CST edits, preserving unchanged bytes, comments, ordering, and manual annotation IDs. Prints `Merged: <doc> -> <out>`. |
| Exists but fails to parse  | Return a hard error. The existing source is never overwritten. |

This is what lets you hand-edit a dumped spec (add comments, pin manual IDs) and
re-dump after a document change without losing your edits.

When a fresh block has a source ID, that ID is authoritative and must match.
Blocks without source IDs use a natural key when available, then a guarded
ordinal fallback for identityless records whose collection cardinality is
unchanged. Property and annotation values are edited at their exact CST ranges;
unchanged formatting and comments remain byte-identical. A source-ID-backed
rename changes only the name token. If a different authored header construct
(for example, adding/removing a binding) cannot be updated without destroying
intent, dump fails closed and leaves the file untouched.

## Round-trip caveats

Dump is not a perfect inverse of apply. Key gaps to be aware of:

- **Pin connections become resolved low-level objects.** The high-level
  `pin X -> #NET` syntax is *not* reconstructed from a SchDoc. As the crate
  README states, "Round-trip dump of `pin X -> #NET` from an existing SchDoc is
  not implemented; dump emits the resolved low-level wire/label objects
  instead." So a dumped SchDoc spec shows `wire`, `net_label`, and
  `power_object` blocks rather than the connection shorthand you may have
  authored.

- **Absolute placement only.** Anchors, rows, grids, and template bindings are
  collapsed to concrete `at:` coordinates. Re-applying reproduces the geometry
  but not the original layout intent.

- **Auto-generated annotation IDs are not document-stable** for ambiguous blocks
  (see above). Use manual IDs plus merge-on-write to keep them fixed.

- **PcbLib footprint errors abort dump.** A failed footprint load cannot produce
  a partial `.pcblib-spec`.

For a faithful round-trip workflow, dump once, hand-pin the annotation IDs you
care about, and rely on merge-on-write for subsequent dumps.
