# Design rationale

Why the Altium Spec Language is built the way it is. This page distills the
"Design Decisions", "Why ..." and invariant sections of
`crates/altium-format-spec/README.md` into a readable explanation, grouped by
theme. It is background reading, not a reference — cited modules are where you
go to verify each decision in source.

**Related pages:** [Introduction](../01-introduction.md) ·
[Annotations](../language/annotations.md) · [Sync](../operations/sync.md)

## Spec as an intermediate representation

External tools should never need to open a `.PcbDoc` binary directly. The spec is
the human-readable, version-controllable middle layer: *write or generate a spec
→ inspect / tweak → reconcile → apply*. Keeping placement and connectivity
decisions in text means they live in source control and diff cleanly, instead of
being buried in an opaque binary.

## Two authoritative modes: greenfield vs brownfield

A spec workflow is either *greenfield* (the `*-spec` files are authoritative and
Altium is a generated artifact + GUI escape hatch) or *brownfield* (existing Altium
files are authoritative and the spec is an agent-editable view). The distinction
decides what `dump` / `apply` should preserve — most visibly, whether a SchDoc
component's inline children are materialized verbatim (brownfield) or treated as
overrides on an imported symbol (greenfield), and whether the tooling may embed its
own identity metadata into the file. This is significant enough to live on its own
page: [Greenfield vs brownfield](greenfield-vs-brownfield.md).

## Text-based rewriting, not AST round-trip

Operations that *rewrite* a spec — `format` and `sync` — edit the source text in
place rather than re-serializing the AST. A full AST round-trip would require the
parser to preserve every whitespace and comment token, which is significant
infrastructure. Instead, the lexer already records a `Span { start, end }` byte
offset on every token, so the rewriter can locate and replace exactly the region
it needs.

The trade-off: user comments *inside* a block that gets rewritten (for example a
`place` block edited by sync) may not survive that block's replacement. This is
accepted as the cost of avoiding full-fidelity AST round-tripping.

### Spans on every AST node

Because rewriting is span-based, *every* AST node type carries
`Span { start: usize, end: usize }` (`src/diagnostic.rs`, applied throughout
`src/ast.rs`). Without spans the rewriter could not find a `place` block in the
source to perform targeted replacement. The cost is near-zero because the lexer
already tracks positions.

## Annotation IDs and identity stability

Sync and rewrite operations need a stable way to refer to "the same block" across
edits, which is what `#[annotation(id = "...")]` provides
(`src/annotation.rs`, `BlockAnnotation` in `src/ast.rs`).

**Auto-generated IDs may change between dump runs.** When `dump_*()` emits a
block that has no existing annotation, it generates a short ID from the block's
content. If two blocks are ambiguous — say two footprints with identical names —
the generated ID depends on the order the dumper visits them, which can differ
after document edits. This is intentional: auto-generation trades strict
stability for zero user effort on a first dump.

**Manual IDs are preserved verbatim.** A hand-set `#[annotation(id = "...")]` is
never overwritten by the dumper and survives all rewrite operations. Users who
need stable IDs (to anchor external tooling or a future three-way merge) should
set them by hand.

**Annotation keys are predefined, not free-form.** `#[annotation(...)]` accepts
only the keys in the `AnnotationKey` enum (`id`, `stable`, `group`, `source_id`).
Arbitrary key-value pairs are rejected at parse time. The reason is safety: if
free-form keys were allowed, a typo like `stabl = true` would be silently
accepted and silently do nothing. A predefined key set makes the parser reject
unknown keys immediately with an actionable error. New metadata is added by
introducing a new `AnnotationKey` variant, not a free-form escape hatch.

**Duplicate detection is two-layer.** The compiler catches within-file duplicate
IDs during incremental compilation using a per-call `seen_ids` set (fast-fail);
the validator performs the authoritative cross-file check. The compiler surfaces
errors early; the validator is authoritative for multi-file projects.

## Reconciler tolerances

The reconciler compares spec geometry against document geometry with deliberate
tolerances so that floating-point round-trip artifacts are not reported as real
changes:

- **Position:** 0.01 mm. Altium internal coordinates are 10,000 units/mil, so a
  `Coord → f64 → Coord` round-trip introduces at most ~0.003 mm of error. The
  0.01 mm threshold (3× the round-trip error) suppresses encoding noise while
  still catching genuine moves.
- **Rotation:** 0.1°, which equals Altium's minimum UI rotation granularity.

This keeps `plan` output focused on intentional edits rather than re-encoding
jitter.

## Designator-based matching, not UniqueID

Specs are plain text without persistent UUIDs, so the reconciler and sync system
match entities by *designator*. Altium itself falls back to designator matching
(`eMapByDesignator`) when UniqueIDs are missing or broken, so designator-only
matching is the correct starting point. Annotation IDs are supplementary, kept in
reserve for rename detection in a future three-way merge.

The connection back to Altium's own ECO is preserved through
`SyncComponent.source_unique_id`, which uses Altium's exact backslash-prefixed
format (`\UNIQUEID` for single-sheet, `Sheet1\UNIQUEID` for hierarchical). Altium
uses this field to match PCB components to schematic components; an empty or wrong
value makes Altium treat every component as new on each ECO cycle.

## Why `SyncSnapshot` is separate and ephemeral

The reconciler diffs a spec against a *binary document* and emits changes to
apply to the document. The sync system diffs *two specs* against each other via a
common projection and emits changes to apply to a spec. Both produce
`EntityChange`-style output but operate on different input types, so they are kept
as separate code paths.

The `SyncSnapshot` IR (`src/sync.rs`) is intentionally ephemeral — generated,
diffed, applied, and discarded, mirroring how Altium's own ECO objects are
transient. Recomputing it is cheap (milliseconds), and persisting it would
introduce staleness risk if the spec is edited between syncs. A persisted *base*
snapshot would be needed for three-way merge, but that design is deferred.

Two supporting choices reinforce determinism and safety:

- **`IndexMap` over `HashMap`** in the snapshot preserves spec declaration order,
  making diff output deterministic (a `HashMap` would break ECO-report test
  assertions).
- **`SyncPolicy` has no `Default`.** An all-`None` policy would silently skip all
  sync. Requiring explicit construction forces the caller to state the direction
  each property flows, preventing accidental no-ops; the CLI always constructs a
  `SyncPolicy` with a named `SyncDirection` per property.

## Pin connection and orientation conventions

`pin X -> #NET` connections are *not* collapsed to wire coordinates at compile
time. They stay as `PinConnectionSpec` in the model, and the executor resolves
them at apply time, when it has live access to the imported SchLib needed to look
up pin positions. This preserves a clean separation between "what the user
declared" and "what Altium objects result."

When the executor does resolve a connection, it applies the placement transform
**mirror first, then rotate** (matching `transform_pin_position`). Reversing the
order produces wrong stub directions. Two conventions then govern the generated
labels:

- **NetLabels use only 0° or 90°**, never 180°/270° — those would render text
  backward or upside-down. `remap_label_orient()` enforces this
  (Rotate180 → Rotate0, Rotate270 → Rotate90).
- **Power symbols match the stub direction directly**, with no secondary
  remapping, because a power symbol rotates to face wherever its stub points.

Whether a net reference becomes a `NetLabel` or a `PowerObject` is decided by a
compiler pre-pass: a net name matching any `power { }` declaration generates a
`PowerObject`; all others generate a `NetLabel`.

## Validator return type

`validate_*_spec()` returns `Result<Vec<SpecError>, Vec<SpecError>>`. `Ok(warnings)`
means the spec is structurally valid with non-fatal warnings (for example
unresolved pin refs that need a library to verify) carried in the `Ok` value;
`Err(errors)` means a hard error and projection must not proceed. The CLI prints
warnings to stderr and continues, but converts errors into a single
`anyhow::Error`.

## Known limitations

These are documented in the crate README as deliberate, scoped limitations rather
than bugs:

- **Library alias disambiguation.** `SchLibSpec` carries no library identity
  (filename or alias), so the resolver cannot disambiguate two libraries that both
  contain a component with the same `lib_reference`; it searches in order and
  takes the first match. When library identity is added, the lookup must filter by
  alias first.
- **No round-trip dump of `pin X -> #NET`.** Dumping an existing SchDoc emits the
  resolved low-level wire/label objects rather than reconstructing the high-level
  pin-connection syntax.
