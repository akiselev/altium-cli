# Plan/Apply Redesign — Note 04: Incremental roadmap

**Session date:** 2026-06-18

Sequenced, shippable steps to move from today's split dump/apply/plan to the
Terraform-style unified model. Each step is independently mergeable, preserves
fail-fast, and leaves the corpus passing. Builds on notes `00`–`03`.

---

## Guiding constraints

- **Never regress fail-fast.** Every "not yet supported" gap becomes a loud error
  or a marked `Unmanaged`, never a silent drop (CARDINAL RULE + note 02).
- **One engine end-state.** The destination is: `reconcile → ECO → execute_eco`,
  with `plan` = reconcile+render and `apply` = reconcile+execute (or execute a
  saved ECO) — for **both** directions (`compile` spec→doc, `dump` doc→spec).
- **Bidirectional (note 05).** Both the document and the `-spec` are editable;
  `dump apply` merges into the existing spec (never overwrites). The old
  one-shot, overwrite-the-spec `dump` is gone; the root `apply` is renamed
  `compile`.
- **Per step:** keep the 955 workspace unit tests green; run a corpus sweep
  before/after (methodology in spec-problems §2); do the mandated independent
  completeness review.

---

## Phase 1 — Foundation (no user-visible behavior change)

**1.1 `ResourceAddress` abstraction** (note 01 §3). One module in
`altium-format-spec` producing an address per entity: annotation-id → unique-id →
natural key → structural/content key. Unit-tested in isolation. *Unblocks
everything else.*

**1.2 Route `compile apply` through the ECO.** Refactor `executor.rs` so
`compile apply` (the renamed root `apply`) = `reconcile()` then
`execute_eco(doc, &eco)`, where `execute_eco` dispatches per `EntityChange`.
Behavior identical for now (still Add/Update only). This is the plan/apply-parity
fix (note 00 §2); after it, fix-#19-class drift is impossible. *Highest
structural value; do early.*

**1.3 ECO `Delete` variant** (note 01 §5) in `eco.rs`: enum arm, summary count,
text/json renderers. No reconciler emission yet — just the type + rendering.

**1.4 Rename root `apply` → `compile`; add `compile {plan,apply}` /
`dump {plan,apply}` command surface** (note 03 §1). Thin wrappers over the
generic engine. Can land alongside 1.2 since they touch the same dispatch. At
this stage `dump {plan,apply}` may still be the from-scratch generator; the merge
behavior lands in Phase 3.

## Phase 2 — Full lifecycle (compile direction)

**2.1 Reconciler emits Deletes.** Walk doc entities unmatched by any spec entity
(via `ResourceAddress`), emit `Delete`. Behind the default authoritative policy
with `--no-delete`/`--append` guard (note 03 §4). Start with the cleanest domain
(SchLib).

**2.2 `execute_eco` honors Delete.** Per-domain delete appliers. Validate
invariants after (`validate_invariants`) and roundtrip via
`assert_cfb_files_semantic_eq`.

**2.3 PcbDoc identity via structural keys** (note 01 §3, spec-problems §7).
Content-hash address for nameless primitives so unchanged boards plan as
`Unchanged` instead of false ADDs. *This makes PcbDoc plan usable for the first
time.* Measure: the ~78 false-ADD PcbDoc files should drop to clean plans.

## Phase 3 — The `dump` direction (doc → spec), bidirectional core

**3.1 Stable annotation IDs in dump** (note 05 §2). Stop random regeneration;
bind annotation-id ↔ UniqueId so the same doc object keeps the same spec ID
across runs. Retires the spec-problems §2 "strip annotations from the diff" hack
— once IDs are stable they belong *in* the diff. Prerequisite for meaningful
`dump plan`.

**3.2 `dump plan` (spec-side ECO).** `reconcile(doc, existing-spec)` →
`EntityChange`s over **spec blocks**. Renderer labels the target as the `-spec`.

**3.3 `dump apply` merges into the existing spec** (note 05 §2). `execute_eco`
mutates spec blocks via `trivia.rs`-preserving rewrite: keep annotation IDs,
comments, ordering, and spec-only constructs (`symbol:`, `pin -> #NET`) for
unchanged blocks (byte-identical, no churn); minimal edits for changed blocks.

**3.4 Saved plans, both directions** (note 03 §2–§3). `plan --out <eco.json>`
serializes the ECO (already `Serialize`); `apply --plan <eco.json>` executes it
after a staleness re-check. Works for `compile` and `dump`.

**3.5 Exit-code change:** `plan` → `0` converged / `2` drift / `1` error
(note 03 §2). Update any scripts/tests asserting exit 1 on drift.

## Phase 4 — Schema symmetry (problem #1 and its mirrors)

Apply the note-02 invariant domain by domain. Each sub-step: make `dump` and
`compile` share one schema OR make the gap a loud error. With Phase 3 in place,
this is now a *correctness prerequisite* for the doc→spec→doc loop (note 05 §3),
not just quality.

**4.1 SchDoc fail-loud scaffold** (note 02 §4 step 2): `compile plan`/`apply` on
inline children that `compile` can't yet materialize must **error**, not silently
drop. Removes the silent-loss; makes the 881 a tracked backlog.
**4.2 SchDoc sheet `parameter` blocks** (cheap, do-regardless).
**4.3 SchDoc materialize inline children** — pins → labels/wires/power →
graphics → params (note 02 §4 step 3). Each removes a 4.1 error and a failure
class. Largest effort; sequence over multiple sessions.
**4.4 `symbol:` + inline merge semantics** (note 02 §3 authoritative-inline).
**4.5 PcbDoc layer stack / board geometry** in `dump`+`compile` (note 02 §5), or
mark `Unmanaged` + preserve (note 03 §4) until implemented.
**4.6 Capture/fail-loud the §10 silent normalizations** (pin show flags, graphic
colors, pad corner radius, …).

## Phase 5 — Ergonomics & concurrency

**5.1 Conflict detection / last-synced baseline** (note 05 §4): refuse an
`apply` when the *other* artifact has drifted since last sync; needed for safe
concurrent two-author (human + agent) editing. May introduce a small sidecar
baseline (note 05 §6 open decision).
**5.2 `destroy`** (note 03 §4 / note 01 §5) — remove everything the source
manages from the target; mostly falls out of Delete support.
**5.3 `-target <address>`** scoped plan/apply.

---

## Dependency graph

```
1.1 ResourceAddress ──┬─► 2.1 Delete emit ──► 2.2 Delete exec
                      ├─► 2.3 PcbDoc structural identity
                      ├─► 3.1 stable annotation IDs ─► 3.2 dump plan ─► 3.3 dump apply (merge)
                      └─► 4.3 inline-child diff/merge
1.2 compile-via-ECO ──► 2.2, 3.3
1.3 Delete type ──────► 2.1
1.4 compile/dump CLI ─► 3.2
3.4 plan --out ───────► (apply --plan, both directions)
4.1 fail-loud ────────► 4.3 (replaces errors one by one)
```

## Sequencing recommendation

Do **1.1 → 1.2 → 1.3 → 1.4** first: pure-internal (plus the CLI rename), they
unlock everything, and 1.2 alone retires the worst latent bug class (plan/apply
drift) at zero behavioral risk. Then **2.3** (PcbDoc identity) for the biggest
single corpus win, and **3.1 → 3.3** (stable IDs + `dump plan/apply` merge) to
make the doc→spec half of bidirectional editing real. **4.1** (SchDoc fail-loud)
stops the largest silent-loss and guards the round-trip loop. The inline-child
build-out (4.3) and concurrency (5.1) are the long tail.

## Open questions to settle before Phase 2

1. **Delete default:** authoritative-by-default with `--no-delete`, or
   safe-by-default (`--prune` to enable)? Terraform is authoritative; PCB safety
   culture may prefer opt-in deletion. *Recommend authoritative + loud summary +
   `--no-delete`, matching Terraform and the spec being the source of truth — but
   this is a genuine user decision (destructive on real fab files).*
2. **Exit-code break:** is changing `plan` drift from `1`→`2` acceptable, or keep
   `1` and lose error/drift disambiguation? (Affects any CI using current `plan`.)
3. **`ResourceAddress` content-hash stability:** which fields are "identity" vs
   "attributes" for nameless primitives? Wrong split → spurious Replace churn.
   Needs per-primitive review against the format docs.
