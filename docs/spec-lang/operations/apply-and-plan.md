# Apply and Plan

How to drive the compile → reconcile → ECO → execute workflow with `altium plan`
and `altium apply`: what an Engineering Change Order (ECO) is, how previewing
differs from mutating, and how reconciler tolerances suppress encoding noise.

## Related pages

- [CLI Reference](cli.md) — exact flags and exit codes for `plan` and `apply`
- [Dump](dump.md) — generate a spec from an existing document
- [Sync](sync.md) — spec-to-spec synchronization (a different diff engine)
- [Operations overview](../README.md)

## The pipeline

Both `plan` and `apply` start the same way (`compile_and_resolve` in main.rs):

```
spec text ──parse──▶ AST ──resolve imports──▶ compile──▶ SpecModel
```

From the `SpecModel`, the two commands diverge:

- **`plan`** feeds the model into the **reconciler**, producing an
  `EngineeringChangeOrder` and printing it. The document is never written.
- **`apply`** feeds the model into the **executor** (`apply_spec_*`), which
  mutates the in-memory document and saves it.

```
                ┌─ plan  → reconcile_* → EngineeringChangeOrder → print
SpecModel ──────┤
                └─ apply → apply_spec_* → mutate document → save
```

## What an ECO is

An `EngineeringChangeOrder` (`src/eco.rs`) is the structured diff between the
desired state (your spec) and the current state (an existing document, or an
empty one). It mirrors Altium's own ECO concept: a reviewable list of additions
and modifications. It has:

- `library_path` — the document the changes target.
- `spec_path` — the source spec.
- `timestamp` — when the ECO was computed (rendered as UTC).
- `summary` — an `EcoSummary`: per-`EntityKind` counts of `adds`, `updates`,
  `unchanged`.
- `changes` — a tree of `EntityChange` nodes.

### EntityChange

Each `EntityChange` (`src/eco.rs`) is one of three variants:

| Variant     | Meaning | Carries |
| ----------- | ------- | ------- |
| `Add`       | Entity is in the spec but not the document. | `props` (initial values) + `children` |
| `Update`    | Entity exists in both; one or more fields differ. | `prop_changes` (old → new) + `children` |
| `Unchanged` | Entity exists in both and matches. | identity only |

Changes nest: a component `Add` contains pin/parameter/graphic/footprint-map
`Add` children; a footprint `Update` contains pad and graphic children. The
summary counts every node in the tree recursively (`compute_summary`).

### EntityKind

`EntityKind` enumerates every entity the reconciler can diff, spanning all
domains: schematic library (`Component`, `Pin`, `Parameter`, `Alias`,
`Graphic`, `Footprint`), PCB library (`Pad`, `Track`, `Via`, `Arc`, `Text`,
`Fill`, `Region`), schematic sheets (`Sheet`, `Wire`, `Bus`, `NetLabel`,
`PowerObject`, `Port`, `Junction`, `NoConnect`, `BusEntry`, `SheetSymbol`,
`Net`, `Power`, ...), PCB boards (`Board`, `PcbDocNet`, `PcbDocComponent`,
`ComponentBody`, `Polygon`, `Rule`, `Class`, `DifferentialPair`, `Dimension`),
and projects (`Project`, `Document`, `OutputGroup`, `OutputJob`, `Variant`,
`Variation`, `ComparisonRule`, `ErcMatrixCell`, `ErcLevel`).

### Reading the text report

`render_text()` produces a boxed report:

```
╔══════════════════════════════════════════════════════════════════════╗
║  ENGINEERING CHANGE ORDER                                              ║
║  Library: my-parts.SchLib                                              ║
║  Spec:    my-parts.schlib-spec                                         ║
║  Date:    2026-06-18 12:00:00 UTC                                      ║
╚══════════════════════════════════════════════════════════════════════╝

SUMMARY
  Components:    1 add, 1 update, 1 unchanged

CHANGES

  + ADD component "R_0603"
  │ designator: "R?"
      ├── + pin "1"
      └── + pin "2"

  ~ UPDATE component "R_0805"
  │ ~ description: "0805 Resistor" → "0805 Resistor (updated)"

  = 1 component unchanged (not shown)

END OF ECO
```

`Add` is `+`, `Update` is `~`, `Unchanged` is `=`. Top-level `Unchanged` runs
are collapsed into a single `= N <kind> unchanged (not shown)` line; unchanged
children collapse to `= N <kind> unchanged`. The JSON form (`--json` /
`render_json()`) serializes the same structure with `change` as a discriminator
(`add` / `update` / `unchanged`).

## Plan: preview without mutating

The reconciler is **read-only** with respect to the document — only
`apply_spec_*` mutates (crate invariant). Use `plan` to:

- Review what an apply *would* do before committing.
- Drift-check in CI: `plan` exits `1` when changes exist, `0` when clean. (Only
  `adds`/`updates` count toward "changes"; an all-`Unchanged` ECO exits `0`.)

```bash
altium plan my-parts.schlib-spec
# exit 0 if no adds/updates, exit 1 if there are

altium plan board.pcbdoc-spec --target board.PcbDoc --json
```

## Empty document vs existing document

The target is resolved per domain (`plan_for_model` / `apply_for_model`):

1. `resolved_target` = `--target` if given, else the spec's default document path.
2. If that path **exists**, the document-aware reconciler (`reconcile_<domain>`)
   diffs the spec against real content. Matched entities become `Update` or
   `Unchanged`; new ones become `Add`.
3. If it **does not exist**, the empty reconciler (`reconcile_<domain>_empty`)
   treats *every* spec entity as an `Add`.

For `apply`, the same fork governs whether a blank AD26 document is created.
PcbDoc is the exception: `apply` **requires** an existing target and errors
otherwise (`PcbDoc apply requires an existing target file`). See
[CLI Reference](cli.md#create-vs-update-behavior-apply_for_model).

### Matching keys

Reconcilers match entities by stable identity, not position:

| Domain entity      | Match key |
| ------------------ | --------- |
| SchLib component   | `lib_reference` |
| Pin                | `designator` |
| Parameter          | `name` |
| Footprint map      | `model_name` |
| PcbLib footprint   | `display_name` |
| Pad                | `pad_name` |
| Graphic            | `unique_id` |
| SchDoc component   | `designator` |
| PcbDoc component   | `designator` |
| PcbDoc net         | `name` |

Spec-side optional fields are only diffed when present: if the spec omits a
field, the reconciler leaves it alone (`diff_opt_field` / `diff_opt_field_vs_str`
short-circuit on `None`). This keeps partial specs from clearing document data.

## Reconciler tolerances

Coordinates round-trip through `Coord → f64 → Coord`, which introduces tiny
encoding artifacts. To avoid reporting non-moves, the reconciler applies
tolerances (`src/reconciler.rs`):

- **Position:** `POSITION_TOLERANCE_MM = 0.01` mm. A component is "moved" only if
  `|Δx|` or `|Δy|` exceeds 0.01 mm. This is ~3× the worst-case round-trip error
  (≤0.003 mm), enough to swallow artifacts while catching real moves. Applied in
  `diff_placement_positions` for `placement { place ... }` blocks that carry an
  explicit `at:`.
- **Rotation:** `ROTATION_TOLERANCE_DEG = 0.1`° matches Altium's minimum UI
  rotation granularity. It is currently reserved — `PlacementPlaceSpec` has no
  rotation field yet, so rotation moves are not compared in placement diffs.
- Pad rotation diffs (footprint reconcile) use `f64::EPSILON`; PcbDoc component
  rotation diffs use a `0.001`° threshold.

Components named in a placement spec but absent from the PcbDoc are warned about
on stderr and skipped, not errored.

## Notes and current limitations

- `--report-json` on `apply` is accepted but **not consumed** today — `apply`
  prints `Saved: <path>`, not a JSON report. Use `plan --json` for a structured
  diff.
- PCB graphic diffs report `Unchanged` when a matching `unique_id` exists rather
  than doing a full field-by-field comparison (the comment in `diff_pcb_graphics`
  notes this would be "very verbose").
- `--all` (PrjPcb only) runs the full pipeline once for the root spec and once
  per imported spec.
