# Spec Language Dump/Apply Roundtrip — Problems, Fixes, and Open Decisions

> **Worklog snapshot.** Results and file paths below describe the 2026-06-10 session,
> not current support. Use [`STATUS.md`](../../STATUS.md) and the current code for status.

**Session date:** 2026-06-10
**Scope:** Verified autopcb removal, then audited and fixed the `altium-format-spec`
dump → apply → validate → re-dump roundtrip across the full `data/` fixture corpora.
This document captures everything needed to reconstruct the session and continue the work.

---

## 1. Starting context

The session began with three questions:
1. What is the current state of the repo?
2. Where are we with autopcb removal?
3. Do the specs and everything work?

**autopcb removal: complete.** `grep -rn "autopcb"` over all `.rs`/`.toml` files returns
nothing. Commit `f9dead2` moved autopcb to its own repo; `c03242d`..`4ea0007` brought the
spec language back in-house as `crates/altium-format-spec` (v0.2.0, a normal workspace
member under `crates/*`). **Stale doc note:** `CLAUDE.md` still claims `altium-cli` depends
on `autopcb-*` crates via cross-repo path dependencies — it does not anymore. Worth
correcting.

**Initial breakage found:** `cargo test --workspace` failed to compile the
`altium-format-spec` test target (13 × E0308) due to a one-line return-type typo in a test
helper, surfaced only under workspace feature unification (the `altium-apply` feature is
pulled in by `altium-cli`). Everything else passed standalone.

---

## 2. The core problem

The spec language had **drifted apart at every layer**. The pipeline is:

```
dump (Altium doc → *-spec text)
  → parse (text → AST)
  → compile (AST → SpecModel, typed)
  → execute (SpecModel → mutate/create Altium doc)
```

Dump emitted property keys that the **compiler silently ignored**, and the **executor
papered over the holes with non-zero defaults**. Result: geometry was zeroed, dropped, or
normalized on apply, and nobody noticed because dump omitted the same fields on both sides
of the comparison. Several of these traced down into genuine **file-format serialization
bugs in `altium-format` itself** (silent data loss on save, independent of the spec
language).

### Test methodology (reproducible)

Per-corpus sweep, run against a **snapshot of the built binary** (`cp target/debug/altium
/tmp/spectest/altium-snap`) so an in-flight rebuild can't swap the binary mid-sweep:

```bash
A=/tmp/spectest/altium-snap; D=/mnt/c/Users/dev/git/altium-cli/data
for f in $D/pcblib/*.PcbLib; do
  b=$(basename "$f" .PcbLib)
  $A dump "$f" --output "$b.pcblib-spec" \
  && $A apply "$b.pcblib-spec" --output "$b.PcbLib" \
  && $A validate "$b.PcbLib" \
  && $A dump "$b.PcbLib" --output "$b.spec2" \
  && diff <(grep -v '#\[annotation' "$b.pcblib-spec") \
          <(grep -v '#\[annotation' "$b.spec2")
done
```

`#[annotation(...)]` lines are stripped from the diff because they contain **random 8-char
IDs** regenerated on each dump (`annotation::generate_short_id`) — they are expected to
differ and are not a roundtrip failure.

For PcbDoc, apply-to-new-doc is unsupported, so the sweep is `dump` + `plan --target
<original>` instead. Note `plan` **exits 1 when the ECO is non-empty** (same convention as
`format --check`), so a nonzero exit there means "drift detected," not "crash."

Test corpora are cloned from (see CLAUDE.md "Test files"):
- `data/schlib/` — github.com/akiselev/altium-cli-test-schlib
- `data/pcblib/` — github.com/akiselev/altium-cli-test-pcblib
- `data/schdoc/` — github.com/akiselev/altium-cli-test-schdoc
- `data/pcbdoc/` — github.com/akiselev/altium-cli-test-pcbdoc

---

## 3. Final results

Criterion: `dump → apply (new doc) → validate → re-dump` yields an identical spec (modulo
random annotation IDs).

| Corpus | Result | Remaining failures |
|--------|--------|--------------------|
| **SchLib** | **121/126** | 4 × `%UTF8%` whitespace-trim upgrade artifact; 1 duplicate component storage name on apply |
| **PcbLib** | **33/40, zero roundtrip diffs** | 5 parser gaps on originals; 2 long-footprint-name apply failures (SectionKeys not written) |
| **SchDoc** | **67/1226** | 278 parser gaps on originals; 881 blocked on the inline-children design gap (§6, decision #1) |
| **PcbDoc** | **dump+plan runs on 79/132** | 36 corrupt/non-CFB fixtures; 15 `Dimensions6 TEXTX` parse gaps; 2 V5 files; reconciler reports false ADDs (no spec↔doc identity matching) |

All **955 workspace unit tests pass** after every change (`cargo test --workspace`, default
features — fixture/proptest suites intentionally not run per CLAUDE.md).

Before the fixes: SchLib and PcbLib had ~73 and ~40 failures respectively; SchDoc was 0/1226.

---

## 4. Fixes made — spec language layer alignment (`crates/altium-format-spec`)

These align dump emission ↔ compiler parsing ↔ executor consumption, and add fail-fast.

1. **Test typo** (`reconciler.rs`): `make_pcblib_spec` returned `SchLibSpec`, built
   `PcbLibSpec`. One-char fix; unblocked the workspace test build.
2. **Dead code removed** (`compiler.rs`): `compile_part` and `compile_pin` were unused
   duplicates of the anchor-aware `compile_part_with_anchors` / `PendingPin` logic.
3. **`Coord` Display roundtrip invariant** (`altium-format-types/src/coord.rs`): display
   emitted mm strings that re-parsed **1 internal unit off** (e.g. 200 mil = 2,000,000
   units printed as `5.08mm`, which re-parses via `from_mms` ×393,701 to 2,000,001). Now mm
   is chosen **only when the printed string re-parses to the identical coordinate**; mil is
   always exact (4 decimals = 1 unit). Added a reparse-sweep test.
4. **Graphic key drift** (`dump.rs` ↔ `compiler.rs`): dump emitted `location:`/`corner:` but
   the grammar uses `from:`/`to:` for box graphics (rectangle, round_rectangle, text_frame,
   image); polyline vertices key was `points` in the compiler but `vertices` in the grammar.
   Rectangles/polylines/text frames/images were **zeroed on apply**. Aligned all keys.
5. **`is_solid` silent flip**: dump emitted `is_solid: true` only when true, but the
   executor defaulted `is_solid` to `true` — so hollow shapes silently became solid. Now
   dump emits `is_solid: false` (omit-when-default-true).
6. **Unknown graphic keys → compile error**: added `KNOWN_KEYS` allow-lists to both
   `compile_graphic_properties` (sch) and `compile_pcb_graphic_properties` (pcb), with a new
   `SpecErrorCode::UnknownProperty`. A silently-ignored key = silently-dropped geometry,
   which violates the project's fail-fast rule.
7. **`quote_entity_name`**: emitted bare integers via `parse::<i64>()`, but the lexer tokens
   are `i32` and a leading `-` lexes as a separate `Minus` token. `692121030100` (overflow),
   `-1`, and `007` (leading zero) produced unparseable or corrupting output. Now bare only
   when `parse::<i32>()` succeeds, is non-negative, and round-trips exactly; also quotes
   reserved keywords (see #17).
8. **PCB graphics apply** (`executor.rs` `pcb_graphic_from_spec`): `via`/`fill`/`text`/
   `region`/`component_body` were zeroed, mis-keyed, or returned `None` (silently dropped).
   Changed signature `Option → Result` (Polyline as PCB primitive is now a hard error
   pointing at `track`); via reads `at` (was `center`); fill reads `corner1`/`corner2`
   (was `from`/`to`); text height has its own key; component_body now typed instead of
   `None`.
9. **`arc()` contour builtin** (`eval.rs`): added `Value::ContourArc` and a `builtin_contour_arc`
   so `outline:` arrays can carry typed arc segments. Previously dump emitted
   `arc(.., 0-360)` where `0-360` parsed as subtraction. All `Value` match sites
   (`display`, `kind_name`, `to_dim`, binop, unary, `value_to_points`, `into_object`)
   handle or reject ContourArc — no panic, no silent coercion.
10. **Pad stack overrides** (`model.rs` / `compiler.rs` / `executor.rs`): added
    `mid_shape`/`mid_x_size`/`mid_y_size`/`bot_*`/`hole_shape`/`slot_size` to `PadSpec`,
    parsed in both `compile_pad` and `pad_from_template`, applied in both new-footprint and
    merge paths via `pad_stack_from_spec`.
11. **Sheet style** (`model.rs` / `compiler.rs` / `dump.rs` / `executor.rs`): added
    `sheet_style: Option<SheetStyle>` and a `style: "A4"` spec key. Dump now always emits
    either `style:` or `custom_width`/`custom_height` so apply never falls back to the
    new-document template default (a custom 1500×950 sheet). This single fix cleared the
    universal SchDoc sheet-size diff.
12. **Footprint pin-pad literal maps** (`ast.rs` / `parser.rs` / `compiler.rs` / `formatter.rs`):
    grammar accepted only `$name` dollar-path refs; dump emitted `pin "1": pad "3"`. Added
    `PinPadRef::{Dollar,Literal}` and `parse_pin_pad_ref`.
13. **Footprint description** (`ast.rs` / `parser.rs` / `model.rs` / `compiler.rs` /
    `executor.rs` / `dump.rs`): `FootprintMap.description` was dumped as a `// comment`
    (unparseable as data) and dropped on apply. Now a real `description:` property; dump uses
    block form for 1:1 maps that carry a description.
14. **`part_count`** (`dump.rs` / `compiler.rs` / `executor.rs`): multi-part components lost
    shared (owner-part-0) pins. Dump now emits `part_count`; compiler infers multi-part from
    presence of `part N { }` blocks; executor infers from `parts` and from `part_count`.
15. **Annotation `source_id` escaping** (`dump.rs` / `formatter.rs` / `sync.rs`): PcbDoc
    hierarchical `source_id`s contain `\` (e.g. `XINCUUYP\CSTXATXL`), emitted unescaped →
    `invalid escape sequence \C`. Now routed through `quote_string`.
16. **Deterministic swap-group emission** (`dump.rs`): swap-group declarations iterated a
    `HashSet` (nondeterministic order) → spurious reordering diffs. Now sorted.
17. **Keyword quoting** (`lexer.rs` / `dump.rs`): an entity named after a keyword (e.g.
    `rule power`) lexed `power` as `TokenKind::Power` → "expected entity name." Added
    `lexer::is_keyword` and made `quote_entity_name` quote keywords.
18. **Always emit sizes** (`dump.rs`): pad `x_size`/`y_size`, via `diameter`/`hole_size`,
    and text `height` were omitted-when-zero, but apply defaults are **nonzero** (60mil /
    50mil / 28mil / 60mil). A zero-size mounting-hole pad or zero-diameter via silently grew.
    Now emitted unconditionally.
19. **Reconciler stack fields** (`reconciler.rs` `diff_pcb_pads`): `plan` compared only
    location/shape/size/hole/plated/layer/rotation, so it reported `Unchanged` while `apply`
    mutated `pad_mode`/`mid_*`/`bot_*`/`hole_shape`/`slot_size`. Plan and apply now agree.

---

## 5. Fixes made — real file-format bugs in `altium-format`

These are **silent data loss on save**, independent of the spec language. They are the most
important findings: any `save-as` of an affected file lost data, not just spec roundtrips.

1. **PinMiscData / `swap_id_pair`** (`schlib.rs`): pair swap IDs were written to the
   `PinMiscData` sidecar only when `pin_field_needs_wide_text(&swap_id_pair)` (i.e. the value
   was long or non-ASCII). But the binary pin record has **no field** for pair swap IDs — the
   sidecar is the *only* storage. Short ASCII pair-swap groups (e.g. `"IOR"`) were **silently
   dropped on every save**. AD26 source analysis showed that Altium writes
   this whenever the value is non-empty. Fixed the gate to `!swap_id_pair.is_empty()`.
2. **`update_sheet` display settings** (`schdoc/mod.rs`): `update_sheet` copied fonts back to
   the Sheet record but **discarded every display setting** (sheet size, grids, borders,
   title block, area color, workspace orientation, …) — the API `SchDocSheet` fields were
   write no-ops. Added a write-back block mirroring `schdoc_read.rs` (every ds-sourced field
   the API models; `workspace_orientation` fails fast via `try_from`). Fields the API doesn't
   model (zones, margins, MBCS, …) retain existing values.
3. **Pad `stack_data` synthesis** (`api/pcblib_write.rs`): `pad_to_internal` set
   `stack_data: None`, so newly-created/edited pads **lost slot holes, inner-layer overrides,
   and corner radii** entirely. Added `stack_data_from_api` (synthesizes the subrecord when
   any modeled field is non-default) and a merge in the patch path that keeps API-modeled
   fields authoritative while preserving record-only fields (hole offsets, alt shapes,
   per-layer flags, extended CR). This regression was *exposed* by fix #18 above
   (unconditional `hole_shape` emission made the loss visible on both sides of the diff).
4. **`HoleType` vs `PadShape` conflation** (`pcblib/mod.rs`, `pcblib/primitives/pad.rs`,
   `api/pcb_common.rs`, `api/pcblib_write.rs`, plus spec crate `model`/`compiler`/`dump`/
   `reconciler`): the pad hole-shape byte (`TExtendedHoleType`: 0=Round, 1=Square, 2=Slot)
   was decoded as `PadShape` (where 0=NoShape, 1=Round, 2=Rectangular). A slotted hole read
   back as the wrong enum and dumped as `"noshape"`, which then failed to re-parse. Switched
   to the already-existing `HoleType` enum (`altium-format-types/src/pcb.rs`) everywhere;
   added `parse_hole_type` / `hole_type_to_spec_string` in the spec crate.

---

## 6. Open decisions

### Decision #1 — RESOLVED → see `docs/spec-lang/explanation/greenfield-vs-brownfield.md`

**Resolution (2026-06-20):** The A/B/C fork below was reframed and resolved. The
spec language serves two workflows — *greenfield* (spec authoritative, Altium is a
generated artifact + GUI escape hatch) and *brownfield* (Altium authoritative, spec
is an agent-editable view). Inline-children handling is selected per component by
whether it resolves to an imported symbol: brownfield → materialize verbatim
(lossless, "Option A"); greenfield → reconstruct from the imported symbol and apply
inline children as overrides (emit only divergence on dump). Identity uses a cascade
(native UniqueId → embedded typed spec params → structural match). `plan`/`apply`
must produce two-sided change sets (source spec gains linking annotations; document
gains the design delta). Open item: mode detection (file-level marker vs inferred)
gated on a reverse-engineering investigation into whether Altium preserves our
metadata across a GUI save. Immediate fail-fast fix regardless: unhandled inline
children currently drop silently at compile and must become a hard error.

The original analysis is retained below for context.

### Decision #1 (original) — SchDoc inline children (the real architectural fork)

**The asymmetry.** The SchDoc spec language has two layers built for opposite purposes:

- **`apply` (authoring path)** consumes `component "U1" { symbol: $lib.LM358, at: (...),
  pin VCC -> #PWR }`. It assumes the symbol *geometry* (pin positions, body graphics) lives
  in an imported SchLib referenced by `symbol: $lib.Name`. `apply_schdoc_component` creates
  the component with `children: Vec::new()`; pins/wires/net-labels/power-objects are
  *synthesized* from the `pin X -> #NET` connections plus the library symbol.
- **`dump` (reverse-engineering path)** has no library to point at — it's looking at a
  finished document — so it emits every pin with absolute coordinates, every graphic, every
  parameter *inline* in the component block.

Dump therefore produces a spec strictly **richer** than apply can consume. The 881 SchDoc
"failures" are almost entirely this: re-applied documents are missing the inline children
the dump faithfully recorded. **Not corruption — apply discards detail it was never taught
to place.**

**Options:**

- **A. Teach `apply` to materialize inline children.** Build pins/graphics/parameters
  directly from the inline spec when present; fall back to the library-symbol path when only
  `symbol:` is given.
  - *Pros:* lossless SchDoc roundtrip (~880 cases); makes SchDoc first-class editable like
    SchLib/PcbLib; dump stops being lossy-on-purpose.
  - *Cons:* large feature. Needs flat-inline → OWNERINDEX tree reconstruction, coordinate/
    orientation handling matching the synthesized path, and merge semantics when both inline
    children *and* a `symbol:` reference appear (who wins?). Also a conceptual concern: a
    SchDoc component is an *instance* of a library symbol; inline geometry creates a second
    source of truth that can drift.
  - *Effort:* large (executor work + test matrix).

- **B. Make `dump` authoring-only.** Stop emitting inline children; dump only `symbol:` +
  pin connections + sheet metadata, resolving geometry back to a library.
  - *Pros:* roundtrip honest by construction; small effort; keeps "component = library
    instance" clean.
  - *Cons:* dump becomes lossy *by design* — can't reconstruct a SchDoc without its
    libraries, which is exactly what inspection/debugging wants; no standalone-sheet authoring.

- **C. Keep rich dump, mark inline children advisory.** Dump emits everything; the roundtrip
  *criterion* compares only the apply-consumable subset; inline children documented as
  dump-only.
  - *Pros:* preserves dump's inspection value; makes the metric meaningful cheaply
    (test/doc change).
  - *Cons:* doesn't make SchDoc editable; institutionalizes the asymmetry; a user who edits
    inline children and re-applies will be surprised when edits vanish.

**Underlying question:** *is SchDoc an authored format or an inspection format?* The STATUS
table says "Read/Write" (leans A); the apply path's design says inspection (leans B/C). That
contradiction is what needs resolving first. Recommendation: **C now, A later** — make the
metric honest immediately; build A only when a concrete authoring use case drives the
inline-vs-library merge semantics (building speculatively = guessing at semantics that age
badly).

**Separable sub-piece:** sheet-level document `parameter` blocks (CurrentDate, DocumentName,
…) are flat key/values with no tree reconstruction — cheap to apply independently of the
component-children question. Worth doing regardless.

### Decision #2 — SectionKeys on apply (contained bug, not architecture)

**Mechanism.** CFB storage/stream names cap at 31 UTF-16 chars. Altium stores a long-named
footprint/component under a truncated/synthesized storage key and records the real name in a
`SectionKeys` parameter mapping. Our reader handles this (these files *load*); our **writer
doesn't generate the mapping on apply**, so a new long-named part either collides (truncated
key) or fails `cfb_key` length validation. That's the `truncated CFB key '...' collides` /
`pascal length` errors in 2 PcbLib + 1 SchLib cases.

**This is a missing serialization step, not a design fork** — the format is understood and
we already parse SectionKeys.

**Options:**
- **Implement it:** on apply, when a name exceeds the limit, allocate a stable short key,
  write the `SectionKeys` mapping, use the short key as the storage name. Medium-small,
  bounded, no open questions. The right fix; aligns with fail-fast / no-data-loss.
- **Hard-error with a clear message:** stopgap only; arguably the status quo already (current
  failure is an error, not silent corruption).
- **Truncate + dedupe silently:** *rejected* — loses the display name on roundtrip, violates
  no-silent-data-loss.

**Recommendation:** normal backlog item; decision is *when*, not *whether* or *how*.

### Decision #3 — Parser-gap backlog (not really a decision)

The 287 SchDoc + 5 PcbLib + 15 PcbDoc files that fail to **open** because the parser doesn't
yet handle some record/parameter/stream: `IGNOREONLOAD`, `OWNERINDEXADDITIONALLIST`,
`Dimensions6 TEXTX`, a binary misread in one `Library/Data`, the SectionKeys pascal-length
case, etc. Plus a different bucket: 36 corrupt/non-CFB PcbDoc fixtures and 2 V5-format files
(bad/unsupported input, not parser bugs).

These are unrelated to the spec language — they block *any* operation (dump, validate, query,
render). This is the normal red/green reverse-engineering loop (look at C#/Delphi
decompilation + fixture, implement typed parse + serialize).

**Options for approach:**
- **Frequency-triage:** the histogram has a long tail — `IGNOREONLOAD` (~94) and
  `OWNERINDEXADDITIONALLIST` (~74 across casings) dominate SchDoc. Two fixes reclaim a few
  hundred files. Best return per unit effort.
- **Quarantine the corrupt/V5 bucket:** mark the 36 non-CFB + 2 V5 fixtures known-bad so they
  stop polluting pass/fail counts. Cheap, makes metrics honest.
- **One-record-per-session discipline:** each gap wants a focused investigation against the
  decompiled source; they don't batch well.

**Recommendation:** quarantine first (cheap, honest metrics), then work frequency-sorted gaps
in separate sessions.

### How the three relate

**#2 and #3 are "when"; #1 is "whether."** #2 (SectionKeys) and #3 (parser gaps) are
understood, bounded work — they need priority, not a decision. #1 (SchDoc inline children) is
the only one with a genuine product question (editable vs inspection format) that changes
what "correct" even means for dump output — settle it before investing in the large executor
work.

---

## 7. Remaining failure detail (for continuation)

**SchLib (5):**
- 4 × parameter values with non-ASCII chars get whitespace-trimmed via the `%UTF8%` duplicate
  entry on save (`param_collection.rs` ~line 206: `let trimmed = safe_value.trim()`). E.g.
  `"1.35VÂ "` → `"1.35VÂ"`, `" 39Ω (39R0) ±1%"` → `"39Ω (39R0) ±1%"`. Matches documented
  Altium upgrade behavior — verify against decompiled reader precedence (is the trimmed UTF-8
  entry authoritative over the untrimmed Win-1252 entry on read?). If Altium itself trims,
  this is a non-bug and the fixtures are pre-trim.
- 1 × `dungvh03-ICs`: "Cannot create storage at /AT25XV041B because a storage already exists"
  — duplicate component storage name on apply (related to but distinct from the SectionKeys
  issue; here two components produce the same CFB key).

**PcbLib (7):**
- 5 parser gaps on originals: `amiryeg-IC-SMD-BGA` / `lucashudson-Communication` /
  `SMotlaq-PCB_lib` (Unknown parameters in `/<fp>/Parameters`), `miniFOC-foc_pcblib`
  (`/Library/Data` binary read past end), `Senior-Design-Custom` (SectionKeys pascal length).
- 2 long-name apply failures: `mobinbyn-Socket` / `TranDangKhoa-LIB` (`truncated CFB key`) —
  decision #2.

**SchDoc:** 278 parser gaps (decision #3), 881 inline-children (decision #1).

**PcbDoc:** 36 corrupt/non-CFB, 15 `Dimensions6 TEXTX` ("invalid digit found in string"),
2 V5, plus ~78 where `plan` exits 1 because the PcbDoc reconciler has no spec↔document
identity matching and reports all dumped board primitives (tracks, etc.) as ADDs. That last
one is a reconciler limitation, not a parse or dump failure — dump is parseable and `plan`
runs cleanly; it just over-reports changes.

---

## 8. Files touched

```
crates/altium-format-types/src/coord.rs            # Coord Display roundtrip invariant + tests
crates/altium-format/src/schdoc/mod.rs             # update_sheet display-settings write-back
crates/altium-format/src/schlib.rs                 # PinMiscData export condition
crates/altium-format/src/pcblib/mod.rs             # HoleType in stack struct
crates/altium-format/src/pcblib/primitives/pad.rs  # HoleType parse
crates/altium-format/src/api/pcb_common.rs         # HoleType in PadStack
crates/altium-format/src/api/pcblib_write.rs       # stack_data synthesis + merge, HoleType
crates/altium-format-spec/src/ast.rs               # PinPadRef, FootprintMapDecl.description
crates/altium-format-spec/src/lexer.rs             # is_keyword
crates/altium-format-spec/src/parser.rs            # pin-pad refs, footprint description
crates/altium-format-spec/src/eval.rs              # ContourArc value + arc() builtin
crates/altium-format-spec/src/model.rs             # sheet_style, pad stack fields, ContourSegmentSpec, descriptions
crates/altium-format-spec/src/compiler.rs          # key alignment, unknown-key rejection, contour, hole_type, part inference
crates/altium-format-spec/src/executor.rs          # pcb graphic apply (Result), pad stack, sheet style, pin name
crates/altium-format-spec/src/dump.rs              # key alignment, always-emit sizes, arc syntax, escaping, sorting, regression tests
crates/altium-format-spec/src/formatter.rs         # pin-pad ref formatting, source_id escaping
crates/altium-format-spec/src/reconciler.rs        # diff_pcb_pads stack fields
crates/altium-format-spec/src/sync.rs              # source_id escaping
crates/altium-format-spec/src/resolver.rs          # sheet_style field
crates/altium-format-spec/src/validator.rs         # sheet_style field
STATUS.md                                          # roundtrip audit table + known issues
```

Net ~1,600 lines across 21 files. Regression tests added: `dump::graphic_roundtrip_tests`,
`dump::pcb_graphic_roundtrip_tests`, and the `coord` display reparse sweep.

---

## 9. Invariants now enforced (carry forward)

- `Coord` Display emits mm **only** when the printed string re-parses (via the 393,701
  units/mm conversion) to the identical coordinate; mils always exact.
- Unknown **graphic** property keys are compile errors (sch + pcb). **Not yet** enforced for
  `compile_pad` / pin compilation — those still ignore unknown keys silently (gap to close).
- Dump emission keys match the grammar (rectangle `from`/`to`, polyline `vertices`, fill
  `corner1`/`corner2`, via `at`, component_body `height`/`model`/`outline` with typed
  `arc(...)`, pad `mid_*`/`bot_*`/`hole_shape`/`slot_size`, explicit pad sizes, `part_count`,
  sheet `style:`).
- Pad hole shape uses `HoleType`, not `PadShape`.

## 10. Known silent normalizations still present (documented, not fixed)

Spec language does not yet capture these — they are normalized on apply: pin
`show_name`/`show_designator`; schematic graphic colors/line widths; pad
`corner_radius_pct`/`inner_layers`/`slot_rotation`; component_body
`standoff_height`/3D-color/opacity; text_frame `word_wrap`/`clip_to_rect`; image
`embed_image`/`keep_aspect`; PCB region `holes`/`kind` (PcbLib path). PcbLib dump also skips
regions with empty outlines (graphic count changes silently). See STATUS.md "Minor" issues.
