# altium-format-spec

Spec DSL for declaratively describing Altium Designer documents. Covers symbol
libraries (SchLib and PcbLib), schematic sheets, PCB boards, and PCB component
placement. Provides compile, execute (apply), reconcile (ECO diff), dump
(reverse-generate), and spec-to-spec sync operations for all document types.

## Architecture

### Core pipeline (all document types)

```
*-spec file text
     |
     v
  lexer.rs       tokenizes with byte-offset spans on every token
     |
     v
  parser.rs      builds typed AST (ast.rs); all AST nodes carry Span {start, end}
                 calls parse_annotation() before each block declaration
     |
     v
  compiler.rs    lowers AST to SpecModel (model.rs); resolves layers, units, refs
                 compiles BlockAnnotation → CompiledAnnotation; tracks seen_ids per file
     |
     v
  SpecModel      in-memory typed representation; all block types carry
                 annotation: Option<CompiledAnnotation>

  SpecModel is then consumed by:

  executor.rs    apply_spec_*(): SpecModel → mutate Altium document
  reconciler.rs  reconcile_*(): SpecModel diff document → EngineeringChangeOrder
  dump.rs        dump_*(): Altium document → *-spec text (emits #[annotation(...)] per block)
```

### Sync pipeline (spec-to-spec synchronization)

```
.schdoc-spec ──compile──▶ SchDocSpec ──project──▶ SyncSnapshot
                                               │
.pcbdoc-spec ──compile──▶ PcbDocSpec ──project──▶ SyncSnapshot
                                                       │
                                              diff_snapshots()
                                                       │
                                                Vec<SyncChange>
                                                       │
                                           filter_changes(policy, direction)
                                                       │
                                      apply_sync_changes_to_pcbdoc()
                                                       │
                                         write back .pcbdoc-spec file
```

The five-phase execution model:

```
Phase 1: PARSE      lexer → parser → AST (with BlockAnnotation nodes)
Phase 2: COMPILE    AST → SpecModel (CompiledAnnotation, sane defaults, seen_ids)
Phase 3: VALIDATE   validator.rs: duplicate designators, dangling net refs, duplicate IDs
Phase 4: RESOLVE    resolver.rs: SchLib lookups → FootprintResolvedSpec
Phase 5a: PROJECT   sync.rs: SpecModel → SyncSnapshot → diff → apply
Phase 5b: PROJECT   reconciler.rs: SpecModel vs document → EngineeringChangeOrder
```

## Placement Spec

The `placement { }` block is a sub-language within `.pcbdoc-spec` files. It is
parsed and formatted as board placement intent, but this crate has no placement
solver dependency.

### Constraint semantics

| Spec property | Solver constraint |
|---|---|
| `at: (x,y)` with no `autoplace: true` | `FixedPosition` — component pinned |
| `autoplace: true` (no other hint) | Unconstrained placement intent |
| `autoplace: true, edge: top, inset: 2mm` | `EdgePlacement { edge: Top, inset: 2.0 }` |
| `autoplace: true, near: $REF, max_distance: 5mm` | `Near { max_distance: 5.0 }` |
| `autoplace: true, region_name: center` | `RegionContainment` covering center quarter |
| `separate $a, $b { gap: Nmm }` | `Directional` between group centroids |
| `unplaced: autoplace` (default) | Components not in spec added as free variables |
| `unplaced: ignore` | Components not in spec pinned at current PcbDoc position |
| `unplaced: error` | Error if any PcbDoc component is missing from spec |

Named regions for `region_name:`: `center`, `top_half`, `bottom_half`, `left_half`,
`right_half`, `quadrant_tl`, `quadrant_tr`, `quadrant_bl`, `quadrant_br`.

## Sync System

### Why SyncSnapshot is separate from the reconciler's ECO

The reconciler diffs a spec against a *binary Altium document* and produces changes to
apply to the document. The sync system diffs two *specs* against each other via a common
projection and produces changes to apply to a spec. Both produce `EntityChange`-style
outputs but operate on different input types. They must remain separate.

### Why SyncSnapshot is ephemeral

Altium's ECO objects are transient in-memory — generated, reviewed, applied, discarded.
Recomputing is cheap (O(ms)). Persistence would add staleness risk if the spec is edited
between syncs. For three-way merge (Phase 3), a base snapshot would need persistence, but
that design is deferred.

### Why auto-generated annotation IDs may change between dump runs

When `dump_*()` emits a spec block that has no existing `#[annotation(id = "...")]`,
it generates a fresh short ID derived from the block's content. If a block's identity
is ambiguous — for example, two footprints with identical names — the generated ID
depends on the ordering the dumper happens to visit them in, which may differ between
runs after document edits. This is expected behaviour: auto-generation is a convenience
default that trades strict stability for zero user effort on first dump.

Users who require stable IDs — for example, to drive three-way merge or to anchor
external tooling — should set them manually:

```
#[annotation(id = "MYID1234")]
component R1 { ... }
```

A manually set ID is preserved verbatim through all spec rewrite operations and is
never overwritten by the dumper. Auto-generation applies only when no annotation is
present.

### Why annotation keys are predefined (no arbitrary key-value pairs)

`#[annotation(...)]` accepts only the keys declared in `AnnotationKey` (`id`, `stable`,
`group`, `source_id`). Arbitrary key-value pairs are intentionally rejected.

If free-form keys were allowed, a typo such as `stabl = true` would be silently
accepted by the parser and have no effect — the component would not be treated as
stable and there would be no indication of the mistake. With a predefined key set
the parser rejects unknown keys at parse time with an actionable error. Safety at
parse time outweighs the flexibility of open-ended metadata.

To add new metadata, add a new variant to `AnnotationKey` in `src/ast.rs` rather
than introducing a free-form escape hatch.

### Why designator-based matching, not UniqueID

Specs are plain text without persistent UUIDs. Altium itself falls back to designator
matching (`eMapByDesignator`) when UniqueIDs are missing or broken. Designator-only
matching is the correct starting point; annotation IDs are supplementary for future
rename detection in Phase 3 three-way merge.

### Why `IndexMap` over `HashMap` in SyncSnapshot

`IndexMap` preserves insertion order (spec declaration order), which makes diff output
deterministic. `HashMap` would produce non-deterministic ordering and break test
assertions on ECO reports.

### Why `SyncPolicy` has no `Default` impl

An all-`None` policy silently skips all sync. Requiring explicit construction forces the
caller to state which direction each property flows, preventing accidental no-ops.
The CLI always constructs `SyncPolicy` with named `SyncDirection` per property.

### Why annotation duplicate detection is two-layer

The compiler detects within-file duplicates during incremental compilation (fast-fail,
one `seen_ids: HashSet<String>` per spec file compile call). The validator performs the
authoritative cross-file duplicate check. The compiler check surfaces errors early;
the validator check is authoritative for multi-file projects.

### Phase 1 forward sync property exclusions

`footprint` is `None` for SchDoc projections (SchDoc specs do not assign footprints
directly; footprint comes from the library symbol). Syncing `None` forward would silently
clear all PcbDoc footprint assignments. Same reasoning applies to `net_color` and
`component_location`. These are excluded from the Phase 1 forward `SyncPolicy`.

### Why SyncPin.designator holds a pad designator, not a pin name

`SyncPin.designator` contains the **pad designator** (the string identifying a pad within
a footprint, e.g. `"10"`), not the schematic pin name (e.g. `"IO8"`). The resolution
happens in `build_pin_to_pad_map()` during `project_schdoc_spec()`:

1. Pin name → pin designator via `PinSpec.name`/`PinSpec.designator`
2. Pin designator → pad name via `FootprintMapSpec.maps`
3. If `maps` is empty: pin designator IS pad name (implicit 1:1)

Both forms (pin name and pin designator) are stored as keys so that callers can look up
either without knowing which form the SchDoc uses for a given component.

### Why source_unique_id uses backslash-prefix format

`SyncComponent.source_unique_id` produces `\UNIQUEID` (backslash-prefixed) for
single-sheet designs. Altium's own ECO populates this field from the schematic
`UNIQUE_ID` parameter with this exact prefix. Hierarchical paths use
`Sheet1\UNIQUEID` (sheet name + backslash + ID). Altium's "Update PCB from
Schematic" uses `SOURCEUNIQUEID` to match PCB components to schematic components; an
empty or wrong value causes Altium to treat every component as new on each ECO cycle.

### Resolver library alias limitation

`SchLibSpec` carries no library identity (filename or alias). The resolver cannot
disambiguate between two libraries that both contain a component with the same
`lib_reference`. It searches all provided libraries in order and picks the first match,
ignoring the alias declared in the spec. When library identity is added to `SchLibSpec`,
this lookup must be updated to filter by alias first.

### Validator return type

`validate_*_spec()` returns `Result<Vec<SpecError>, Vec<SpecError>>`:
- `Ok(warnings)` — spec is structurally valid; non-fatal warnings (e.g., unresolved pin
  refs that require library for verification) are carried in the `Ok` value
- `Err(errors)` — one or more hard errors; spec must not proceed to projection

The CLI prints `Ok(warnings)` to stderr before proceeding and converts `Err(errors)` to
a single `anyhow::Error`.

## Sync Invariants

1. Every block in a spec file has a unique annotation ID after dump (auto-generated if absent).
2. Annotation IDs are stable across spec rewrites — same block identity → same ID.
3. `diff_snapshots(a, a)` always produces an empty changeset.
4. `apply(diff(source, target), target)` produces a target where `diff(source, target')` is empty.
5. `project_schdoc_spec` and `project_pcbdoc_spec` are side-effect-free but fallible.
6. `annotation: stable = true` blocks are skipped by the executor during sync apply.
7. `filter_changes()` returns `Err` if it encounters any `AddPin`/`RemovePin`/`UpdatePin`
   in Phase 1 — these must not be silently stripped.

## Design Decisions

**Spec-as-intermediate-representation.** External tools should not need to touch
PcbDoc binaries directly. The workflow is: write or generate spec → inspect/tweak
→ reconcile → apply to `.PcbDoc`. This keeps placement decisions human-readable and
version-controllable.

**Text-based spec rewriting (not AST round-trip).** Full AST round-trip rewriting would
require preserving all whitespace and comment tokens in the parser — significant
infrastructure. Text-based rewriting using byte-offset spans from the lexer achieves the
same result with far less code. Cost: user comments inside `place` blocks may not survive
a rewrite of that block.

**Spans on all AST nodes.** Every AST node type carries `Span { start: usize, end: usize }`
byte offsets — required for targeted spec rewriting. Without span fields, the rewriter cannot
locate `place` blocks in source text to perform targeted replacement. Amortized cost is
near-zero because the lexer already tracks positions.

**Reconciler tolerance.** Position comparison uses 0.01 mm tolerance; rotation uses 0.1°.
Altium internal coordinates are 10,000 units/mil, so Coord→f64→Coord round-trips introduce
at most ~0.003 mm error. The 0.01 mm threshold (3× round-trip error) suppresses encoding
artifacts while catching real moves. 0.1° equals Altium's minimum UI rotation granularity.

**Pin orientation transform order: mirror then rotate.** When resolving `pin X -> #NET`
connections, a pin's orientation from the SchLib must be transformed by the component's
placement. Mirror is applied first (flips left↔right, i.e., 0°↔180°; 90° and 270°
unchanged), then the component rotation is added modulo 360°. Reversing the order produces
wrong stub directions. This matches the `transform_pin_position` function used elsewhere in
the executor.

Example: pin orientation 0° (right) + mirror=true + rotation=90° → mirror flips to 180°
(left) → add 90° = 270° (down). Stub extends downward.

**NetLabel orientation convention: 0° or 90° only, never 180°/270°.** Altium's NetLabel
`orientation` field accepts four values, but standard Altium schematics always use 0° for
horizontal stubs (both left- and right-pointing) and 90° for vertical stubs (both up- and
down-pointing). Using 180° or 270° produces text that reads backward or upside-down.
`remap_label_orient()` enforces this: Rotate180 → Rotate0, Rotate270 → Rotate90.

**Power symbol orientation matches stub direction directly.** A `PowerObject`'s
`orientation` field equals the stub direction (the direction the pin extends). There is no
secondary remapping — the power symbol rotates to face wherever the stub points. This
differs from NetLabel where the text-readability constraint requires collapsing 180°/270°.

**`PinConnectionSpec` stays in model; executor resolves at apply time.** Pin connection
declarations are not collapsed to wire coordinates during compilation. The executor needs
live access to `imported_components` (a `HashMap<String, ComponentSpec>` threaded from the
CLI) to look up pin positions from the imported SchLib. Keeping the spec-level intent in
`PinConnectionSpec` also preserves a clean separation between "what the user declared" and
"what Altium objects result." Round-trip dump of `pin X -> #NET` from an existing SchDoc
is not implemented; dump emits the resolved low-level wire/label objects instead.

**`Value::ImportRef` carries alias provenance for compile-time symbol validation.** When
`eval_field_access` resolves `$mcu.ESP32_C6` and the base `Value` is an
`ImportObject { alias, entries }`, it returns `Value::ImportRef { alias, name }` instead
of a plain `Value::String`. The compiler recognises `ImportRef` in the `symbol:` property
and emits `SymbolRef::Import { alias, name }`, then validates `name` against the
`imported_components` SchLib at compile time. All other field access paths (non-import
objects) continue to return `Value::String`. This change is contained entirely to
`eval_field_access`.

## Invariants

- `compiler.rs` resolves all unit conversions (mm, mil, inch) to internal Altium coords
  before storing in SpecModel. Downstream (executor, reconciler) never parse unit strings.
- Parameter keys in spec are case-insensitive at the compiler level; the compiler
  normalizes to lowercase before storing in SpecModel.
- The reconciler is read-only with respect to the document. Only `apply_spec_*` mutates.
- `dump_*` always sorts output by designator/name for stable diffs.
- A net name in `pin X -> #NET` that matches any key in `SheetSpec::power_declarations`
  generates a `PowerObject`; all other net names generate a `NetLabel`. The compiler
  pre-scans all `power {}` blocks before processing components to populate this map.
- `remap_label_orient()` must never return `Rotate180` or `Rotate270`. Any new caller
  that bypasses `remap_label_orient` must document why it can produce non-standard
  orientations.
- `SymbolRef::Import { alias, name }` is only produced by `compile_schdoc_component`
  when the `symbol:` value is `Value::ImportRef`. `SymbolRef::Literal` is the fallback for
  all other string-valued `symbol:` properties.
