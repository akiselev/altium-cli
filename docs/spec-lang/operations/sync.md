# Sync

How to synchronize a `.schdoc-spec` (source) into a `.pcbdoc-spec` (target) with
`altium spec sync`: the `SyncSnapshot` projection IR, the diff/filter/apply
pipeline, per-property `SyncDirection`, pin→pad resolution, and the sync
invariants.

## Related pages

- [CLI Reference](cli.md#altium-spec-sync) — flags, exit codes, the fixed policy table
- [Apply and Plan](apply-and-plan.md) — the *other* diff engine (spec vs binary document)
- [Annotations](../language/annotations.md) — `stable` and `id` keys
- [Operations overview](../README.md)

## Sync vs reconcile

Sync and the reconciler both produce change lists, but they operate on different
inputs and must stay separate (crate README):

- **Reconciler** diffs a spec against a *binary Altium document* → an
  `EngineeringChangeOrder` applied to the document. See [Apply and Plan](apply-and-plan.md).
- **Sync** diffs *two specs* against each other through a common projection →
  `SyncChange`s applied to a spec. That is this page.

```
.schdoc-spec ──compile──▶ SchDocSpec ──project──▶ SyncSnapshot ┐
                                                               ├─ diff_snapshots ─▶ Vec<SyncChange>
.pcbdoc-spec ──compile──▶ PcbDocSpec ──project──▶ SyncSnapshot ┘                          │
                                                  filter_changes(policy, direction) ◀─────┘
                                                              │
                                          apply_sync_changes_to_pcbdoc + rewrite text
```

## Running sync

```bash
# Preview only (no writes)
altium spec sync sheet.schdoc-spec board.pcbdoc-spec --diff

# Apply SchDoc → PcbDoc (writes board.pcbdoc-spec atomically)
altium spec sync sheet.schdoc-spec board.pcbdoc-spec --forward

# Multi-sheet: never remove components from earlier sheets
altium spec sync sheet2.schdoc-spec board.pcbdoc-spec --forward --append
```

You must pass `--forward` or `--diff`. `--dry-run` prints the ECO but skips the
write step. Full flag semantics: [CLI Reference](cli.md#altium-spec-sync).

## The SyncSnapshot IR

`SyncSnapshot` (`src/sync.rs`) is the common projection both domains map into. It
is **ephemeral** — recomputed fresh on each sync, never persisted (recomputing is
cheap and persistence would risk staleness).

```rust
struct SyncSnapshot {
    components: IndexMap<String, SyncComponent>, // keyed by designator
    nets:       IndexMap<String, SyncNet>,       // keyed by name
}
```

`IndexMap` (not `HashMap`) preserves spec declaration order, which makes diff
output deterministic.

- **`SyncComponent`**: `designator`, `comment`, `footprint`, `source_library`,
  `parameters`, `pins` (keyed by **pad** designator), `annotation_id`,
  `source_unique_id`.
- **`SyncPin`**: `designator` (the pad designator), `net`.
- **`SyncNet`**: `name`, `color`, `pins` (`(component, pad)` tuples),
  `annotation_id`.

### Projection differences

`project_schdoc_spec` and `project_pcbdoc_spec` are side-effect-free but
fallible. Both fail hard on duplicate component designators; PcbDoc also fails on
duplicate net names. SchDoc fails on net/power pin references to non-existent
components.

A key asymmetry: **PcbDoc snapshots have empty `pins`** — PcbDoc specs do not
carry pin-level connectivity. SchDoc snapshots populate `pins` from `net`,
`power`, and `pin X -> #NET` declarations.

### Phase-1 forward exclusions

Some `SyncComponent` fields are deliberately `None` for SchDoc projections, so
syncing them forward would silently *clear* existing PcbDoc data. The CLI
excludes them from the Phase-1 forward policy:

- **`footprint`** is `None` for SchDoc (footprint comes from the library symbol,
  not the SchDoc directly).
- **`net_color`** and **`component_location`** are likewise excluded.

These map to `SyncDirection::None` in the CLI policy (see below).

## Pin→pad resolution

`SyncPin.designator` holds a **pad** designator (e.g. `"10"`), not the schematic
pin name (e.g. `"IO8"`). `build_pin_to_pad_map()` resolves names to pads during
`project_schdoc_spec`, using imported SchLib data:

1. Pin name → pin designator (via `PinSpec.name` / `PinSpec.designator`).
2. Pin designator → pad name (via `FootprintMapSpec.maps`).
3. If `maps` is empty, the pin designator *is* the pad name (implicit 1:1).

Both the pin name and the pin designator are stored as keys, so a caller can look
up either form without knowing which the SchDoc used.

## Diffing

`diff_snapshots(source, target)` is direction-agnostic — it computes what the
target must change to match the source, leaving direction to `filter_changes`.

- Components matched by designator → `AddComponent` / `RemoveComponent` /
  `UpdateComponent`. `UpdateComponent` carries `FieldChange`s for `comment`,
  `footprint`, `source_library`, and `parameter:<key>` entries.
- Within matched components, pins (by pad designator) →
  `AddPin` / `RemovePin` / `UpdatePin` (net assignment).
- Nets matched by name → `AddNet` / `RemoveNet` / `UpdateNet` (`net_color`).

`diff_snapshots(a, a)` always yields an empty change list (invariant 3).

## Filtering with SyncPolicy and SyncDirection

`SyncPolicy` assigns a `SyncDirection` to each property:

```rust
enum SyncDirection { Forward, Back, Bidirectional, None }
```

`filter_changes(changes, policy, direction)` keeps a field change only when its
property's direction is `Bidirectional` or equals the requested `direction`;
`None` always excludes it. Add/Remove of whole components and nets pass through
**unconditionally** — policy governs only which *fields* of existing entities are
updated, not whether an entity exists.

`SyncPolicy` intentionally has **no `Default` impl**: an all-`None` policy would
silently skip all sync, a hard-to-diagnose no-op. The CLI always constructs it
explicitly. The Phase-1 forward policy the CLI uses:

| Property             | Direction  |
| -------------------- | ---------- |
| `comment`            | `Forward`  |
| `footprint`          | `Forward`  |
| `source_library`     | `Forward`  |
| `parameters`         | `Forward`  |
| `net_name`           | `Forward`  |
| `net_color`          | `None`     |
| `pin_net_assignment` | `None`     |
| `component_location` | `None`     |

### Pin changes are never silently stripped

If `filter_changes` encounters any `AddPin`/`RemovePin`/`UpdatePin`:

- with `policy.pin_net_assignment == None` → the change is dropped (expected when
  pin sync is excluded), **but**
- with any other direction → it returns a hard `NotSupported` error: "pin-level
  sync is not supported: PcbDoc specs do not carry pin-level connectivity"
  (invariant 7). This prevents connectivity changes from vanishing unnoticed.

## Applying and writing back

`apply_sync_changes_to_pcbdoc` mutates the in-memory `PcbDocSpec` in three
dependency-ordered phases — Removes, then Updates, then Adds. It guards:

- Empty board list → error.
- More than one board → error (single-board specs only).
- Any pin-level change → hard error.

`component_location`, `rotation`, and `layer` are **never** touched by sync.
Newly added components get a fresh annotation (`generate_short_id` +
`generate_source_id(designator)`).

The model mutation is used for validation; the actual file change is a
**text-level rewrite** (`rewrite_pcbdoc_spec_with_changes`) that preserves all
non-component/net content verbatim (geometry, tracks, polygons, rules, placement
blocks). The rewritten source is reformatted and written atomically via temp
file + rename. `--append` drops every `Remove*` change before this step so
multi-sheet syncs accumulate rather than clobber.

## `stable` blocks are skipped

A block annotated `#[annotation(stable = true)]` is skipped by the executor
during sync apply (invariant 6). Use it to pin a component or net you have
hand-tuned in the PcbDoc spec so forward sync from the schematic won't overwrite
it.

## Sync invariants

From the crate README:

1. Every block has a unique annotation ID after dump (auto-generated if absent).
2. Annotation IDs are stable across spec rewrites — same identity → same ID.
3. `diff_snapshots(a, a)` produces an empty changeset.
4. `apply(diff(source, target), target)` yields a `target'` where
   `diff(source, target')` is empty.
5. `project_schdoc_spec` / `project_pcbdoc_spec` are side-effect-free but fallible.
6. `stable = true` blocks are skipped by the executor during sync apply.
7. `filter_changes` returns `Err` on any `AddPin`/`RemovePin`/`UpdatePin` in
   Phase 1 — they must not be silently stripped.

## ECO report

`render_eco_report` prints a flat summary (`+` add, `-` remove, `~` update with
indented field changes) and a total count. It requires a **filtered** change list
— it treats pin changes as unreachable and will panic if any remain, so always
call `filter_changes` first.

```
ECO (3 changes):
  + component U2 (pattern: SOIC-8)
  ~ component R1
      comment : (none) -> 10k
  - net OLD_NET
```
